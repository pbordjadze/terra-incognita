import Foundation
import ImageIO
import Metal
import QuartzCore
import UIKit
import os

/// Clouds over the globe: where they usually are this month, as satellites have seen them (ISCCP-H,
/// 2000–2016: CloudsSky.png, made by tools/playground/clouds/build.py), drawn as weather that could
/// be (`Clouds.metal`). A Metal layer of its own between the globe's drawing and its names, drawn each
/// frame the globe ticks (`GlobeFollower`), so that the globe and its code are untouched.
///
/// Soft things need few pixels: it draws at about one and a quarter pixels a point and lets Core
/// Animation scale that up, then follows the GPU's own timings, drawing finer while a frame costs
/// little and coarser when it costs more, within a budget the globe can spare. When the GPU is two
/// frames behind it skips one rather than wait. The drift is a simulated day a minute; Reduce
/// Motion holds it still, and so does nothing being on screen (the globe's ticks stop).
final class CloudsLayer: GlobeFollower {
    var layer: CALayer { metal }

    private static let daySeconds = 60.0
    /// The GPU time a frame may take (ms), and what it aims below: the globe and its names share
    /// the frame, at up to 120 a second.
    private static let budget = (over: 3.5, under: 1.8)

    private let metal = CAMetalLayer()
    private let device = MTLCreateSystemDefaultDevice()
    private let gpu = CloudsGPU()
    private var queue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?
    private var sky: MTLTexture?
    private var land: MTLTexture?
    private var tile: MTLTexture?
    private var stopped = false

    /// Pixels a point it draws at, and the most it will (no more than the screen has, nor 2).
    private var scale = 1.25
    private var finest = 2.0
    private var days = 0.0
    private var heldDays: Double?
    private var heldMonth: Double?
    private var lastTick: CFTimeInterval?
    private var lastAdjust: CFTimeInterval = 0
    /// (the first report once frames have been timed)
    private var lastReport = CACurrentMediaTime()
    private let look: CloudsLook

    init(look: CloudsLook = CloudsLook()) {
        self.look = look
        metal.device = device
        metal.pixelFormat = .bgra8Unorm
        metal.framebufferOnly = true
        metal.isOpaque = false
        metal.contentsScale = scale
        metal.needsDisplayOnBoundsChange = false
        #if DEBUG
        // `-clouds-days 0.4` holds the drift at that moment and `-clouds-month 10` shows October's
        // sky (CI's pictures)
        let args = ProcessInfo.processInfo.arguments
        func value(_ flag: String) -> Double? {
            guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
            return Double(args[i + 1])
        }
        heldDays = value("-clouds-days")
        heldMonth = value("-clouds-month").map { (($0 - 1).truncatingRemainder(dividingBy: 12) + 12).truncatingRemainder(dividingBy: 12) }
        #endif
        days = heldDays ?? 0
        setUp()
    }

    // MARK: GlobeFollower

    func frame(of globe: GlobeView) {
        guard !stopped else { return }
        let now = CACurrentMediaTime()
        let dt = lastTick.map { min(now - $0, 0.1) } ?? 0
        lastTick = now
        if heldDays == nil, !UIAccessibility.isReduceMotionEnabled { days += dt / Self.daySeconds }
        guard let pipeline, let queue, let sky, let land, let tile else { return }
        // (the GPU two frames behind: skip this one rather than wait for a drawable)
        guard gpu.inFlight < 2 else { return }
        let bounds = globe.bounds
        guard bounds.width > 0, bounds.height > 0 else { return }
        finest = min(Double(globe.traitCollection.displayScale), 2)
        adjust(now: now)
        let size = CGSize(width: (bounds.width * scale).rounded(), height: (bounds.height * scale).rounded())
        if metal.frame != bounds || metal.drawableSize != size {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            metal.frame = bounds
            metal.contentsScale = scale
            metal.drawableSize = size
            CATransaction.commit()
        }
        guard let drawable = metal.nextDrawable(), let commands = queue.makeCommandBuffer() else { return }
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .clear
        pass.colorAttachments[0].clearColor = MTLClearColor(red: 0, green: 0, blue: 0, alpha: 0)
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = commands.makeRenderCommandEncoder(descriptor: pass) else { return }
        var u = uniforms(globe: globe, size: size)
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        let turning = Self.cyclones(days: days)
        turning.withUnsafeBytes { encoder.setFragmentBytes($0.baseAddress!, length: $0.count, index: 1) }
        encoder.setFragmentTexture(sky, index: 0)
        encoder.setFragmentTexture(land, index: 1)
        encoder.setFragmentTexture(tile, index: 2)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        commands.present(drawable)
        gpu.began()
        commands.addCompletedHandler { @Sendable [gpu] done in
            gpu.ended(seconds: done.gpuEndTime - done.gpuStartTime)
        }
        commands.commit()
        report(now: now)
    }

    func stop() {
        stopped = true
        pipeline = nil
        sky = nil
        land = nil
        tile = nil
    }

    // MARK: drawing

    /// The shader's uniforms (`CloudsUniforms` in Clouds.metal: the same order).
    private struct Uniforms {
        var frame: SIMD4<Float>
        var disc: SIMD4<Float>
        var rot0: SIMD4<Float>
        var rot1: SIMD4<Float>
        var rot2: SIMD4<Float>
        var time: SIMD4<Float>
        var sun: SIMD4<Float>
        var seaShadow: SIMD4<Float>
        var landShadow: SIMD4<Float>
        var tone: SIMD4<Float>
    }

    private func uniforms(globe: GlobeView, size: CGSize) -> Uniforms {
        let s = globe.stageCamera
        // world from view, Ry(lon) . Rx(-lat), as the globe turns (GlobeView.rotation), by columns
        let a = s.camera.lon * .pi / 180, b = -s.camera.lat * .pi / 180
        let ca = Float(cos(a)), sa = Float(sin(a)), cb = Float(cos(b)), sb = Float(sin(b))
        let side = Double(min(globe.bounds.width, globe.bounds.height))
        let reading = GlobeType.shared?.readingScale(shortSide: side) ?? 1
        let month = heldMonth ?? Self.monthNow()
        let m0 = floor(month)
        return Uniforms(
            frame: SIMD4(Float(size.width), Float(size.height), Float(scale), Float(reading)),
            disc: SIMD4(Float(s.centre.x), Float(s.centre.y), Float(s.radius), Float(s.camera.zoom)),
            rot0: SIMD4(ca, 0, -sa, 0),
            rot1: SIMD4(sa * sb, cb, ca * sb, 0),
            rot2: SIMD4(sa * cb, -sb, ca * cb, 0),
            time: SIMD4(Float(days), Float(m0), Float((m0 + 1).truncatingRemainder(dividingBy: 12)), Float(month - m0)),
            sun: look.sun.map { SIMD4($0, 1) } ?? .zero,
            seaShadow: look.seaShadow,
            landShadow: look.landShadow,
            tone: SIMD4(look.opacity, look.night, 0, 0))
    }

    /// The cyclones (`cyclones` in Clouds.metal): where each is and how hard it turns are the same for
    /// every pixel, so worked out here once a frame. Two float4s each: its centre (the globe's world
    /// coordinates) and size (radians), then its turn (signed: anticlockwise in the north) and the
    /// cosine of its reach. Five a hemisphere, each six days long, born at a place the hash of its
    /// number and its life's gives (the shader's pcg3d, as Clouds.metal's noise uses).
    private static func cyclones(days: Double) -> [SIMD4<Float>] {
        var out = [SIMD4<Float>](repeating: .zero, count: 20)
        for k in 0..<10 {
            let hemi = k < 5 ? 1.0 : -1.0
            let t = days / 6 + Double(k) * 0.2 + (hemi < 0 ? 0.1 : 0)
            let n = t.rounded(.down), age = t - n
            let r = hash3(Int32(k), Int32(n), 977)
            let lat = hemi * (38 + 22 * r.y) * .pi / 180, lon = 2 * .pi * r.x
            let size = 0.09 + 0.07 * r.z
            let wind = 2.6 * smoothstep(0, 0.55, age) * (1 - smoothstep(0.78, 1, age))
            out[2 * k] = SIMD4(Float(cos(lat) * sin(lon)), Float(sin(lat)), Float(cos(lat) * cos(lon)), Float(size))
            out[2 * k + 1] = SIMD4(Float(-hemi * wind), Float(cos(3 * size)), 0, 0)
        }
        return out
    }

    /// pcg3d (Clouds.metal), as three numbers from 0 to 1.
    private static func hash3(_ x: Int32, _ y: Int32, _ z: Int32) -> SIMD3<Double> {
        var a = UInt32(bitPattern: x) &* 1664525 &+ 1013904223
        var b = UInt32(bitPattern: y) &* 1664525 &+ 1013904223
        var c = UInt32(bitPattern: z) &* 1664525 &+ 1013904223
        a = a &+ b &* c; b = b &+ c &* a; c = c &+ a &* b
        a ^= a >> 16; b ^= b >> 16; c ^= c >> 16
        a = a &+ b &* c; b = b &+ c &* a; c = c &+ a &* b
        return SIMD3(Double(a), Double(b), Double(c)) / 4294967295
    }

    private static func smoothstep(_ a: Double, _ b: Double, _ x: Double) -> Double {
        let t = min(max((x - a) / (b - a), 0), 1)
        return t * t * (3 - 2 * t)
    }

    /// Today as months from mid-January (0) round the year: the sky between the two nearest months'.
    private static func monthNow() -> Double {
        let calendar = Calendar.current, now = Date()
        let month = calendar.component(.month, from: now) - 1
        let day = Double(calendar.component(.day, from: now) - 1) + Double(calendar.component(.hour, from: now)) / 24
        let length = Double(calendar.range(of: .day, in: .month, for: now)?.count ?? 30)
        let m = Double(month) + (day - length / 2) / length
        return (m.truncatingRemainder(dividingBy: 12) + 12).truncatingRemainder(dividingBy: 12)
    }

    /// Finer while a frame costs little, coarser when it costs more; looked at twice a second.
    private func adjust(now: CFTimeInterval) {
        guard now - lastAdjust > 0.5 else { return }
        lastAdjust = now
        let ms = gpu.milliseconds
        guard ms > 0 else { return }           // (no timings, as in some simulators: stay as it is)
        if ms > Self.budget.over { scale = max(0.6, scale * 0.85) }
        else if ms < Self.budget.under, scale < finest { scale = min(finest, scale * 1.1) }
    }

    /// What it costs, in the workbench's log, every ten seconds while it runs.
    private func report(now: CFTimeInterval) {
        guard now - lastReport > 10 else { return }
        lastReport = now
        let ms = gpu.milliseconds
        let cost = ms > 0 ? String(format: "%.1f ms of GPU a frame", ms) : "no GPU timings"
        PackStore.shared.record("clouds: \(cost), at \(String(format: "%.2f", scale)) pixels a point")
    }

    // MARK: setting up

    private func setUp() {
        guard let device, let library = device.makeDefaultLibrary(),
              let vertex = library.makeFunction(name: "cloudsVertex"),
              let fragment = library.makeFunction(name: "cloudsFragment"),
              let queue = device.makeCommandQueue() else {
            Self.log.error("clouds: no Metal device or functions")
            return
        }
        self.queue = queue
        let d = MTLRenderPipelineDescriptor()
        d.vertexFunction = vertex
        d.fragmentFunction = fragment
        // (no blending: each frame starts clear, and the shader writes premultiplied colour)
        d.colorAttachments[0].pixelFormat = .bgra8Unorm
        pipeline = try? device.makeRenderPipelineState(descriptor: d)
        tile = Self.noiseTile(device: device, library: library, queue: queue)
        // the measured sky and the land, decoded off the main thread
        let skyURL = Bundle.main.url(forResource: "CloudsSky", withExtension: "png")
        let landURL = Bundle.main.url(forResource: "CloudsLand", withExtension: "png")
        Task.detached(priority: .userInitiated) { [weak self] in
            let sky = skyURL.flatMap(CloudsLayer.rgba)
            let land = landURL.flatMap(GlobeData.grey)
            await self?.loaded(sky: sky, land: land)
        }
    }

    private func loaded(sky skyBytes: (bytes: [UInt8], width: Int, height: Int)?, land landBytes: (bytes: [UInt8], width: Int, height: Int)?) {
        guard !stopped, let device else { return }
        guard let skyBytes, let landBytes else {
            Self.log.error("clouds: CloudsSky.png or CloudsLand.png missing or unreadable")
            return
        }
        sky = Self.texture(device: device, format: .rgba8Unorm, bytes: skyBytes.bytes, width: skyBytes.width, height: skyBytes.height, channels: 4)
        land = Self.texture(device: device, format: .r8Unorm, bytes: landBytes.bytes, width: landBytes.width, height: landBytes.height, channels: 1)
    }

    private static let log = Logger(subsystem: "com.pbordjadze.terraincognita", category: "clouds")

    private static func texture(device: MTLDevice, format: MTLPixelFormat, bytes: [UInt8], width: Int, height: Int, channels: Int) -> MTLTexture? {
        guard bytes.count >= width * height * channels else { return nil }
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: format, width: width, height: height, mipmapped: false)
        d.usage = .shaderRead
        let texture = device.makeTexture(descriptor: d)
        texture?.replace(region: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0, withBytes: bytes, bytesPerRow: width * channels)
        return texture
    }

    /// The noise's tile (`cloudsNoiseTile`): 96 samples a side, made on the GPU before the first frame.
    private static func noiseTile(device: MTLDevice, library: MTLLibrary, queue: MTLCommandQueue) -> MTLTexture? {
        let d = MTLTextureDescriptor()
        d.textureType = .type3D
        d.pixelFormat = .r8Unorm
        d.width = 96
        d.height = 96
        d.depth = 96
        d.usage = [.shaderRead, .shaderWrite]
        d.storageMode = .private
        guard let texture = device.makeTexture(descriptor: d),
              let kernel = library.makeFunction(name: "cloudsNoiseTile"),
              let state = try? device.makeComputePipelineState(function: kernel),
              let commands = queue.makeCommandBuffer(),
              let encoder = commands.makeComputeCommandEncoder() else { return nil }
        encoder.setComputePipelineState(state)
        encoder.setTexture(texture, index: 0)
        encoder.dispatchThreads(MTLSize(width: 96, height: 96, depth: 96), threadsPerThreadgroup: MTLSize(width: 4, height: 4, depth: 4))
        encoder.endEncoding()
        commands.commit()
        return texture
    }

    /// An RGB picture's decoded bytes as they are, as RGBA (colour management would change the
    /// values, which are data); bytes in any other order are refused rather than read wrong.
    nonisolated private static func rgba(_ url: URL) -> (bytes: [UInt8], width: Int, height: Int)? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil), image.bitsPerComponent == 8,
              let data = image.dataProvider?.data, let bytes = CFDataGetBytePtr(data) else { return nil }
        let w = image.width, h = image.height, row = image.bytesPerRow, channels = image.bitsPerPixel / 8
        let order = image.byteOrderInfo
        guard order == .orderDefault || order == .order32Big else { return nil }
        switch (channels, image.alphaInfo) {
        case (4, .last), (4, .premultipliedLast), (4, .noneSkipLast), (3, .none): break
        default: return nil
        }
        guard CFDataGetLength(data) >= row * h else { return nil }
        var out = [UInt8](repeating: 255, count: w * h * 4)
        out.withUnsafeMutableBufferPointer { dst in
            for y in 0..<h {
                for x in 0..<w {
                    let s = y * row + x * channels, t = (y * w + x) * 4
                    dst[t] = bytes[s]; dst[t + 1] = bytes[s + 1]; dst[t + 2] = bytes[s + 2]
                }
            }
        }
        return (out, w, h)
    }
}

/// How the clouds sit on a globe: their opacity, their shadows' colours and strengths on sea and
/// land, and the light. These are the Wikipedia map's: a cool blue-grey on its pale sea, its own
/// drop shadow's warm grey (6E6352) on its cream land, lit from the north-west as its relief is. A
/// globe drawn another way (a satellite view, say) would give its own: darker shadows on darker
/// ground, perhaps a sun, which also turns the clouds' night side down (`night`).
struct CloudsLook {
    var opacity: Float = 1
    var seaShadow = SIMD4<Float>(0.20, 0.26, 0.38, 1)
    var landShadow = SIMD4<Float>(0.43, 0.39, 0.32, 0.8)
    /// Towards the sun in the globe's world coordinates (y to the north pole, z to 0° 0°); nil lights
    /// from the map's north-west, everywhere.
    var sun: SIMD3<Float>?
    var night: Float = 0.75
}

/// The GPU's timings and the frames it has in hand, written by Metal's completion handlers (any
/// thread) and read by the main one.
nonisolated final class CloudsGPU: @unchecked Sendable {
    private let lock = NSLock()
    private var flying = 0
    private var ms = 0.0

    var inFlight: Int { lock.withLock { flying } }
    /// A running average of a frame's GPU time, ms (0: none yet).
    var milliseconds: Double { lock.withLock { ms } }

    func began() { lock.withLock { flying += 1 } }

    func ended(seconds: Double) {
        lock.withLock {
            flying = max(0, flying - 1)
            guard seconds > 0, seconds < 1 else { return }
            ms = ms == 0 ? seconds * 1000 : ms * 0.9 + seconds * 1000 * 0.1
        }
    }
}
