import Foundation
import ImageIO
import Metal
import QuartzCore
import UIKit
import os

/// The stage's sea: how its surface usually moves in the globe's month, measured by drifting buoys
/// (the NOAA Global Drifter Program's climatology: OceanCurrents.png and OceanStream.png, made by
/// tools/ocean/build.py), drawn as streamlines with light running along them, and close in its
/// glitter and its surf (`Ocean.metal`). A Metal layer of its own between the globe's drawing and the
/// clouds (`GlobeView.sea`), drawn as the globe ticks, so the globe's own drawing is untouched; it
/// reads the globe's coast and the month's ice from the globe itself (`GlobeView.seaMaps`). A touch
/// on the water rings it (`touch`).
///
/// Fine lines need pixels: it draws at up to the screen's own and follows the GPU's timings, finer
/// while a frame costs little and coarser when it costs more, within a budget the globe can spare.
/// When the GPU is two frames behind it skips one rather than wait. While the camera rests it draws
/// thirty times a second, enough for the light to run smoothly. Reduce Motion holds everything still
/// (a touch's ring fades where it is); so does nothing being on screen (the globe's ticks stop). It
/// fades out where it doesn't belong (the Persian page, the authored sequences, the parked still sea)
/// and thins while an experience runs.
final class OceanLayer {
    var layer: CALayer { metal }

    /// How much of it shows while an experience runs (`CloudsSwitch.veiled`).
    private static let veil = 0.55
    /// The GPU time a frame may take (ms), and what it aims below.
    private static let budget = (over: 4.0, under: 2.0)

    private let metal = CAMetalLayer()
    private let device = MTLCreateSystemDefaultDevice()
    private let gpu = CloudsGPU()
    private var queue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?
    private var currents: MTLTexture?
    private var stream: MTLTexture?
    private var tile: MTLTexture?
    private var stopped = false

    /// Pixels a point it draws at, and the most it will (no more than the screen has).
    private var scale = 1.5
    private var finest = 2.0
    /// The sea's own seconds (they pass only while the globe ticks), for the touches' rings; and the
    /// drawn motion's, which Reduce Motion holds, as `-ocean-time` does.
    private var seconds = 0.0
    private var motion = 0.0
    private var heldSeconds: Double?
    private var heldMonth: Double?
    private var lastTick: CFTimeInterval?
    private var lastAdjust: CFTimeInterval = 0
    /// How far it is shown (0 to 1: it fades in and out), and what the last frame drew from.
    private var shown = 0.0
    private var drawn: (camera: SIMD8<Double>, at: CFTimeInterval, shown: Double)?
    /// Four touches: centre (world) and when, then strength; the next to be replaced.
    private var ripples = [SIMD4<Float>](repeating: .zero, count: 8)
    private var nextRipple = 0
    private var lastReport = CACurrentMediaTime()

    init() {
        metal.device = device
        metal.pixelFormat = .bgra8Unorm
        metal.framebufferOnly = true
        metal.isOpaque = false
        metal.contentsScale = scale
        metal.needsDisplayOnBoundsChange = false
        #if DEBUG
        // `-ocean-time 12` holds the sea at that moment and `-ocean-month 10` shows October's
        // currents (CI's pictures)
        let args = ProcessInfo.processInfo.arguments
        func value(_ flag: String) -> Double? {
            guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
            return Double(args[i + 1])
        }
        heldSeconds = value("-ocean-time")
        heldMonth = value("-ocean-month").map { (($0 - 1).truncatingRemainder(dividingBy: 12) + 12).truncatingRemainder(dividingBy: 12) }
        #endif
        motion = heldSeconds ?? 0
        setUp()
    }

    // MARK: each frame

    /// Draws the sea for the globe's camera this frame (GlobeView's tick, before the globe draws).
    func frame(of globe: GlobeView) {
        guard !stopped else { return }
        let now = CACurrentMediaTime()
        let dt = lastTick.map { min(now - $0, 0.1) } ?? 0
        lastTick = now
        seconds += dt
        if heldSeconds == nil, !UIAccessibility.isReduceMotionEnabled { motion += dt }
        let belongs = OceanLook.on(globe.style) && !globe.authoredMode && !globe.seaHidden
        fade(to: belongs ? (CloudsSwitch.shared.veiled ? Self.veil : 1) : 0, dt: dt)
        guard shown > 0, let pipeline, let queue, let currents, let stream, let tile, let maps = globe.seaMaps else { return }
        // (the GPU two frames behind: skip this one rather than wait for a drawable)
        guard gpu.inFlight < 2 else { return }
        let bounds = globe.bounds
        guard bounds.width > 0, bounds.height > 0 else { return }
        // while nothing changes but the sea itself, thirty frames a second (one, held still)
        let s = globe.stageCamera
        let camera = SIMD8<Double>(s.camera.lon, s.camera.lat, s.camera.zoom, s.radius,
                                  Double(s.centre.x), Double(s.centre.y), Double(bounds.width), Double(bounds.height))
        let held = heldSeconds != nil || UIAccessibility.isReduceMotionEnabled
        let ringing = ripples.indices.contains { $0 % 2 == 0 && ripples[$0 + 1].x > 0 && seconds - Double(ripples[$0].w) < 5 }
        if let d = drawn, d.camera == camera, d.shown == shown, now - d.at < (held && !ringing ? 1 : 1.0 / 32) { return }
        drawn = (camera, now, shown)
        finest = Double(globe.traitCollection.displayScale)
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
        encoder.setFragmentTexture(currents, index: 0)
        encoder.setFragmentTexture(stream, index: 1)
        encoder.setFragmentTexture(maps.coast, index: 2)
        encoder.setFragmentTexture(maps.ice, index: 3)
        encoder.setFragmentTexture(tile, index: 4)
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

    /// Rings the water at a place (degrees), from now.
    func touch(lon: Double, lat: Double) {
        guard shown > 0 else { return }
        let a = lon * .pi / 180, b = lat * .pi / 180
        ripples[2 * nextRipple] = SIMD4(Float(cos(b) * sin(a)), Float(sin(b)), Float(cos(b) * cos(a)), Float(seconds))
        ripples[2 * nextRipple + 1] = SIMD4(1, 0, 0, 0)
        nextRipple = (nextRipple + 1) % 4
        drawn = nil
    }

    func stop() {
        stopped = true
        pipeline = nil
        currents = nil
        stream = nil
        tile = nil
    }

    /// Towards shown (1), a veil or gone (0) in about a third of a second; hidden once gone, so Core
    /// Animation composites nothing for it.
    private func fade(to target: Double, dt: Double) {
        if shown != target {
            shown = target > shown ? min(target, shown + dt * 3) : max(target, shown - dt * 3)
        }
        let hidden = shown == 0
        if metal.isHidden != hidden {
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            metal.isHidden = hidden
            CATransaction.commit()
        }
    }

    // MARK: drawing

    /// The shader's uniforms (`OceanUniforms` in Ocean.metal: the same order).
    private struct Uniforms {
        var frame: SIMD4<Float>
        var disc: SIMD4<Float>
        var rot0: SIMD4<Float>
        var rot1: SIMD4<Float>
        var rot2: SIMD4<Float>
        var time: SIMD4<Float>
        var tone: SIMD4<Float>
        var ripple: (SIMD4<Float>, SIMD4<Float>, SIMD4<Float>, SIMD4<Float>, SIMD4<Float>, SIMD4<Float>, SIMD4<Float>, SIMD4<Float>)
    }

    private func uniforms(globe: GlobeView, size: CGSize) -> Uniforms {
        let s = globe.stageCamera
        // world from view, Ry(lon) . Rx(-lat), as the globe turns (GlobeView.rotation), by columns
        let a = s.camera.lon * .pi / 180, b = -s.camera.lat * .pi / 180
        let ca = Float(cos(a)), sa = Float(sin(a)), cb = Float(cos(b)), sb = Float(sin(b))
        let side = Double(min(globe.bounds.width, globe.bounds.height))
        let reading = GlobeType.shared?.readingScale(shortSide: side) ?? 1
        // the globe's month, which the ground and the clouds show too
        let month = heldMonth ?? Double(min(max(globe.month, 1), 12) - 1)
        let m0 = floor(month)
        let moving: Float = UIAccessibility.isReduceMotionEnabled ? 0 : 1
        let r = ripples
        return Uniforms(
            frame: SIMD4(Float(size.width), Float(size.height), Float(scale), Float(reading)),
            disc: SIMD4(Float(s.centre.x), Float(s.centre.y), Float(s.radius), Float(s.camera.zoom)),
            rot0: SIMD4(ca, 0, -sa, 0),
            rot1: SIMD4(sa * sb, cb, ca * sb, 0),
            rot2: SIMD4(sa * cb, -sb, ca * cb, 0),
            time: SIMD4(Float(seconds), Float(m0), Float((m0 + 1).truncatingRemainder(dividingBy: 12)), Float(month - m0)),
            tone: SIMD4(Float(shown), moving, Float(motion), 0),
            ripple: (r[0], r[1], r[2], r[3], r[4], r[5], r[6], r[7]))
    }

    /// Finer while a frame costs little, coarser when it costs more; looked at twice a second.
    private func adjust(now: CFTimeInterval) {
        guard now - lastAdjust > 0.5 else { return }
        lastAdjust = now
        let ms = gpu.milliseconds
        guard ms > 0 else { return }           // (no timings, as in some simulators: stay as it is)
        if ms > Self.budget.over { scale = max(0.8, scale * 0.85) }
        else if ms < Self.budget.under, scale < finest { scale = min(finest, scale * 1.1) }
    }

    /// What it costs, in the workbench's log, every ten seconds while it runs.
    private func report(now: CFTimeInterval) {
        guard now - lastReport > 10 else { return }
        lastReport = now
        let ms = gpu.milliseconds
        let cost = ms > 0 ? String(format: "%.1f ms of GPU a frame", ms) : "no GPU timings"
        PackStore.shared.record("ocean: \(cost), at \(String(format: "%.2f", scale)) pixels a point")
    }

    // MARK: setting up

    private func setUp() {
        guard let device, let library = device.makeDefaultLibrary(),
              let vertex = library.makeFunction(name: "oceanVertex"),
              let fragment = library.makeFunction(name: "oceanFragment"),
              let queue = device.makeCommandQueue() else {
            Self.log.error("ocean: no Metal device or functions")
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
        // the measured sea, decoded off the main thread
        let currentsURL = Bundle.main.url(forResource: "OceanCurrents", withExtension: "png")
        let streamURL = Bundle.main.url(forResource: "OceanStream", withExtension: "png")
        Task.detached(priority: .userInitiated) { [weak self] in
            let currents = currentsURL.flatMap(OceanLayer.rgba)
            let stream = streamURL.flatMap(OceanLayer.rgba)
            await self?.loaded(currents: currents, stream: stream)
        }
    }

    private func loaded(currents c: (bytes: [UInt8], width: Int, height: Int)?, stream s: (bytes: [UInt8], width: Int, height: Int)?) {
        guard !stopped, let device else { return }
        guard let c, let s else {
            Self.log.error("ocean: OceanCurrents.png or OceanStream.png missing or unreadable")
            return
        }
        currents = Self.texture(device: device, bytes: c.bytes, width: c.width, height: c.height)
        stream = Self.texture(device: device, bytes: s.bytes, width: s.width, height: s.height)
    }

    private static let log = Logger(subsystem: "com.pbordjadze.terraincognita", category: "ocean")

    private static func texture(device: MTLDevice, bytes: [UInt8], width: Int, height: Int) -> MTLTexture? {
        guard bytes.count >= width * height * 4 else { return nil }
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: width, height: height, mipmapped: false)
        d.usage = .shaderRead
        let texture = device.makeTexture(descriptor: d)
        texture?.replace(region: MTLRegionMake2D(0, 0, width, height), mipmapLevel: 0, withBytes: bytes, bytesPerRow: width * 4)
        return texture
    }

    /// The clouds' noise tile (`cloudsNoiseTile`), a copy of its own: 96 samples a side, made on the GPU
    /// before the first frame.
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

/// Where the sea belongs: on the ground and on the Wikipedia map, not on the Persian page (a
/// manuscript's water lies still).
enum OceanLook {
    /// (not on the Persian page, nor by night, where its light lines would outshine the lights)
    static func on(_ style: GlobeStyle) -> Bool { style != .persian && style != .night }
}
