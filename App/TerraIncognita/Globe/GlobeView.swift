import CoreGraphics
import CoreText
import Foundation
import ImageIO
import Metal
import OSLog
import QuartzCore
import UIKit
import simd

/// The globe's two styles: Wikipedia's location-map colours, a little prettier, and the Persian, the
/// same map in colours studied from a Persian manuscript, inside its frame (Globe.metal's P_ palette).
enum GlobeStyle: String, CaseIterable, Identifiable {
    case persian, wikipedia

    var id: String { rawValue }
    var name: String { self == .persian ? String(localized: "Persian") : String(localized: "Wikipedia") }
}

/// The whole globe, drawn live by Metal (`Globe.metal`) in either style: drag to spin it, pinch to
/// zoom, double-tap to fly in, tap a country to light it (in the locator's red, or gilded). Left alone
/// it turns slowly, as the Earth does.
final class GlobeView: UIView, UIGestureRecognizerDelegate {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    /// A country was tapped (nil: the selection was cleared).
    var onSelect: ((GlobeData.Country?) -> Void)?
    /// The first frame is on screen (the veil over the switch to it can lift).
    var onDrawn: (() -> Void)?
    /// The country in the locator's red, if any.
    var selectedCountry: GlobeData.Country? { selected.flatMap { data?.countries[$0] } }
    /// Parts of the screen covered by something else (the caption, the buttons), in this view's
    /// points: no name is placed under them.
    var keepClear: [CGRect] = [] {
        didSet {
            let fresh = clearBounds != bounds.size
            clearBounds = bounds.size
            guard keepClear != oldValue || fresh else { return }
            if !roomSettled { measureRoom() }
            labelsFading = true
            needsDraw = true
        }
    }
    /// The part of the screen the globe and its frame fill: between the buttons and the caption.
    /// Measured when the screen's size, its safe area or the style changes, not when the caption's
    /// text does, so a tap never moves the globe.
    private var room: CGRect?
    private var roomMeasured = false
    /// When the room was first measured from rects laid out for this screen: for a moment after,
    /// the caption and buttons may still be settling, and each new place of theirs measures it again.
    private var roomSince = 0.0
    private var roomSettled: Bool { roomMeasured && CACurrentMediaTime() - roomSince > 0.6 }
    /// The size the view had when `keepClear` was last set: rects from another size are stale.
    private var clearBounds = CGSize.zero
    /// While the film shows instead, nothing moves or is drawn (the globe is kept, its pictures
    /// decoded once).
    var suspended = false {
        didSet {
            displayLink?.isPaused = suspended
            lastTick = nil
            needsDraw = true
        }
    }

    private let data = GlobeData.load()
    private let device = MTLCreateSystemDefaultDevice()
    private var queue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?
    private var geo: MTLTexture?
    private var terrain: MTLTexture?
    private var ids: MTLTexture?
    private var band: MTLTexture?
    private var fleuron: MTLTexture?
    private var braid: MTLTexture?
    /// The Persian capitals' marker (Globe/rosette.png).
    private var rosette: CGImage?
    /// The style, and how far the drawing has crossed to it: 0 Wikipedia's, 1 the Persian.
    private(set) var style = GlobeStyle.persian
    private var styleMix = 1.0
    private var displayLink: CADisplayLink?
    private var lastTick: CFTimeInterval?
    private var needsDraw = true
    private var busy = true
    private var labelsFading = true
    private var cachedRotation: (lon: Double, lat: Double, m: simd_double3x3)?

    // the camera: the point of the globe at the screen's centre, in degrees, and how far in
    private var lon0 = -40.0
    private var lat0 = 22.0
    private var zoom = 1.0
    private static let zooms = 0.8...8.0
    // a fling, in degrees a second; a double tap's flight
    private var spin = CGVector.zero
    private var touching = 0
    private var lastTouch: CFTimeInterval = -100
    private var flight: Flight?
    // the country in the locator's red, and how far it has faded up (it fades down still drawn)
    private var selected: Int?
    private var shown: Int?
    private var highlight = 0.0
    /// How far the chosen country's name has faded in where it is (times the highlight).
    private var selectedAlpha = 0.0
    private let labelLayer = CALayer()
    private var labels: [Label] = []
    /// The chosen country's name: in Wikipedia's style a thick red silhouette of it (the halo, for
    /// where it reaches past its country onto the sea), the cream letters on top as its sublayer; in
    /// the Persian a small text panel, ruled in gold and lapis (`selectedRule`), the letters ink.
    private let selectedLabel = CATextLayer()
    private let selectedLetters = CATextLayer()
    private let selectedRule = CALayer()
    /// The screen's pixels per point, for the names' pictures.
    private var pixels: CGFloat = 3
    #if DEBUG
    /// `-globe-at lon lat zoom` (CI's screenshots): the camera held there, no turning.
    private var held = false
    #endif

    private struct Flight {
        let from: (lon: Double, lat: Double, zoom: Double)
        let to: (lon: Double, lat: Double, zoom: Double)
        let start: CFTimeInterval
        let duration: Double
    }

    private enum Kind { case country, sea, capital }

    /// A name on the globe: where it goes, from what zoom, its picture once first shown.
    private struct Label {
        let text: NSAttributedString
        let size: CGSize
        let lon: Double
        let lat: Double
        let kind: Kind
        let index: Int
        /// The web-map zoom from which it shows.
        let from: Double
        /// A Persian capital's rosette, left of its name and on the city itself, and its size.
        let mark: Bool
        let markSize: CGFloat
        /// Its halo's tone, and the room it keeps from other names (points).
        let halo: String
        let gap: CGFloat
        var layer: CALayer?
        var alpha = 0.0
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        GlobeData.registerFonts()
        backgroundColor = UIColor(red: 0xF6 / 255, green: 0xF2 / 255, blue: 0xE7 / 255, alpha: 1)
        isMultipleTouchEnabled = true
        setUpMetal()
        setUpLabels()
        // the names follow the reader's text size
        registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) { (view: GlobeView, _: UITraitCollection) in
            view.relabel()
        }
        let pan = UIPanGestureRecognizer(target: self, action: #selector(pan(_:)))
        pan.maximumNumberOfTouches = 2
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(pinch(_:)))
        let double = UITapGestureRecognizer(target: self, action: #selector(doubleTap(_:)))
        double.numberOfTapsRequired = 2
        let tap = UITapGestureRecognizer(target: self, action: #selector(tap(_:)))
        tap.require(toFail: double)
        for g in [pan, pinch, double, tap] as [UIGestureRecognizer] { g.delegate = self; addGestureRecognizer(g) }
        isAccessibilityElement = true
        accessibilityLabel = String(localized: "The globe")
        accessibilityHint = String(localized: "Drag to spin it, pinch to zoom, tap a country to name it")
        accessibilityTraits = [.image, .allowsDirectInteraction]
        #if DEBUG
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-globe-at"), i + 3 < args.count,
           let lon = Double(args[i + 1]), let lat = Double(args[i + 2]), let z = Double(args[i + 3]) {
            lon0 = lon; lat0 = lat; zoom = z; held = true
        }
        if let i = args.firstIndex(of: "-globe-select"), i + 1 < args.count,
           let k = data?.countries.firstIndex(where: { $0.n == args[i + 1] }) {
            select(k, animated: false)
        }
        #endif
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    // MARK: Metal

    private var metalLayer: CAMetalLayer? { layer as? CAMetalLayer }

    private func setUpMetal() {
        guard let device, let metalLayer, let library = device.makeDefaultLibrary(),
              let vertex = library.makeFunction(name: "globe_vertex"),
              let fragment = library.makeFunction(name: "globe_fragment") else { return }
        metalLayer.device = device
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.framebufferOnly = true
        metalLayer.isOpaque = true
        let d = MTLRenderPipelineDescriptor()
        d.vertexFunction = vertex
        d.fragmentFunction = fragment
        d.colorAttachments[0].pixelFormat = .bgra8Unorm
        pipeline = try? device.makeRenderPipelineState(descriptor: d)
        queue = device.makeCommandQueue()
        guard let queue else { return }
        geo = Self.texture(GlobeData.root.appending(path: "geo.png"), device: device, queue: queue)
        terrain = Self.texture(GlobeData.root.appending(path: "terrain.png"), device: device, queue: queue)
        band = Self.texture(GlobeData.root.appending(path: "band.png"), device: device, queue: queue, premultiply: true)
        fleuron = Self.texture(GlobeData.root.appending(path: "fleuron.png"), device: device, queue: queue, premultiply: true)
        braid = Self.texture(GlobeData.root.appending(path: "braid.png"), device: device, queue: queue)
        if let source = CGImageSourceCreateWithURL(GlobeData.root.appending(path: "rosette.png") as CFURL, nil) {
            rosette = CGImageSourceCreateImageAtIndex(source, 0, nil)
        }
        // (an ornament that won't load leaves its place empty; the globe still draws)
        let blank = Self.blank(device: device)
        if band == nil || fleuron == nil || braid == nil {
            Logger(subsystem: "com.pbordjadze.terraincognita", category: "globe")
                .error("ornament missing or unreadable: band \(self.band != nil) fleuron \(self.fleuron != nil) braid \(self.braid != nil)")
        }
        band = band ?? blank
        fleuron = fleuron ?? blank
        braid = braid ?? blank
        if geo == nil || terrain == nil || data == nil {
            Logger(subsystem: "com.pbordjadze.terraincognita", category: "globe")
                .error("globe data missing or unreadable: geo \(self.geo != nil) terrain \(self.terrain != nil) data \(self.data != nil)")
        }
        // the countries' map, from the bytes the tap reads (decoded once)
        if let data, data.ids.count == data.width * data.height {
            let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .r8Unorm, width: data.width, height: data.height, mipmapped: false)
            d.usage = .shaderRead
            ids = device.makeTexture(descriptor: d)
            ids?.replace(region: MTLRegionMake2D(0, 0, data.width, data.height), mipmapLevel: 0, withBytes: data.ids, bytesPerRow: data.width)
        }
    }

    /// One transparent pixel, in place of an ornament that won't load.
    private static func blank(device: MTLDevice) -> MTLTexture? {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: 1, height: 1, mipmapped: false)
        d.usage = .shaderRead
        let texture = device.makeTexture(descriptor: d)
        let clear: [UInt8] = [0, 0, 0, 0]
        texture?.replace(region: MTLRegionMake2D(0, 0, 1, 1), mipmapLevel: 0, withBytes: clear, bytesPerRow: 4)
        return texture
    }

    /// An RGBA map's decoded bytes uploaded as they are (colour management would bend the distances
    /// it is made of), with mipmaps. Bytes in any order but R, G, B, A are refused rather than read
    /// wrong. A picture with transparency (`premultiply`) is premultiplied if it isn't, so that its
    /// mipmaps blend its edges right.
    private static func texture(_ url: URL, device: MTLDevice, queue: MTLCommandQueue, premultiply: Bool = false) -> MTLTexture? {
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
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: w, height: h, mipmapped: true)
        d.usage = .shaderRead
        guard let texture = device.makeTexture(descriptor: d) else { return nil }
        if channels == 3 {
            // (a decoder may drop an alpha that is opaque everywhere)
            var rgba = [UInt8](repeating: 255, count: w * h * 4)
            rgba.withUnsafeMutableBufferPointer { dst in
                for y in 0..<h {
                    for x in 0..<w {
                        let s = y * row + x * 3, t = (y * w + x) * 4
                        dst[t] = bytes[s]; dst[t + 1] = bytes[s + 1]; dst[t + 2] = bytes[s + 2]
                    }
                }
            }
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: rgba, bytesPerRow: w * 4)
        } else if premultiply && image.alphaInfo == .last {
            var rgba = [UInt8](repeating: 0, count: w * h * 4)
            rgba.withUnsafeMutableBufferPointer { dst in
                for y in 0..<h {
                    for x in 0..<w {
                        let s = y * row + x * 4, t = (y * w + x) * 4
                        let a = UInt16(bytes[s + 3])
                        dst[t] = UInt8((UInt16(bytes[s]) * a + 127) / 255)
                        dst[t + 1] = UInt8((UInt16(bytes[s + 1]) * a + 127) / 255)
                        dst[t + 2] = UInt8((UInt16(bytes[s + 2]) * a + 127) / 255)
                        dst[t + 3] = bytes[s + 3]
                    }
                }
            }
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: rgba, bytesPerRow: w * 4)
        } else {
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: bytes, bytesPerRow: row)
        }
        if let commands = queue.makeCommandBuffer(), let blit = commands.makeBlitCommandEncoder() {
            blit.generateMipmaps(for: texture)
            blit.endEncoding()
            commands.commit()
        }
        return texture
    }

    private nonisolated struct Uniforms {
        var u0: SIMD4<Float>
        var u1: SIMD4<Float>
        var u2: SIMD4<Float>
        var r0: SIMD4<Float>
        var r1: SIMD4<Float>
        var r2: SIMD4<Float>
        var u3: SIMD4<Float>
    }

    /// Draws a frame; false if it couldn't (no drawable yet), so the next tick tries again.
    private func draw() -> Bool {
        guard let pipeline, let queue, let geo, let terrain, let ids, let band, let fleuron, let braid, let metalLayer,
              bounds.width > 0, bounds.height > 0, let drawable = metalLayer.nextDrawable(),
              let commands = queue.makeCommandBuffer() else { return false }
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .dontCare
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = commands.makeRenderCommandEncoder(descriptor: pass) else { return false }
        let scale = Double(metalLayer.contentsScale)
        let (centre, radius) = place
        let m = rotation
        let ring = 1 - smoothstep(1.25, 1.7, zoom)
        var u = Uniforms(
            u0: SIMD4(Float(Double(bounds.width) * scale), Float(Double(bounds.height) * scale), Float(Double(centre.x) * scale), Float(Double(centre.y) * scale)),
            u1: SIMD4(Float(radius * scale), Float(scale * Double(reading)), Float(zoom), Float(ring)),
            u2: SIMD4(Float(shown.map { $0 + 1 } ?? 0), Float(highlight), Float(styleMix), Float(pageFrame)),
            r0: SIMD4(Float(m[0][0]), Float(m[1][0]), Float(m[2][0]), 0),
            r1: SIMD4(Float(m[0][1]), Float(m[1][1]), Float(m[2][1]), 0),
            r2: SIMD4(Float(m[0][2]), Float(m[1][2]), Float(m[2][2]), 0),
            u3: SIMD4(Float(Double(safeAreaInsets.top) * scale), Float(Double(safeAreaInsets.bottom) * scale),
                      Float(Double(safeAreaInsets.left) * scale), Float(Double(safeAreaInsets.right) * scale)))
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        encoder.setFragmentTexture(geo, index: 0)
        encoder.setFragmentTexture(terrain, index: 1)
        encoder.setFragmentTexture(ids, index: 2)
        encoder.setFragmentTexture(band, index: 3)
        encoder.setFragmentTexture(fleuron, index: 4)
        encoder.setFragmentTexture(braid, index: 5)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        commands.present(drawable)
        commands.commit()
        return true
    }

    // MARK: the camera

    /// The globe's centre and radius on screen, in points (tools/globe/preview.py's `layout`): as large
    /// as its room allows with its frame round it (Wikipedia's gold ring reaches 11 reading points
    /// past the edge; the Persian frame's finials 0.1833 of the radius), 14 pt from the room's edges.
    private var place: (centre: CGPoint, radius: Double) {
        let box = room ?? CGRect(x: 0, y: Double(bounds.height) * 0.08, width: Double(bounds.width), height: Double(bounds.height) * 0.7)
        let half = min(Double(box.width), Double(box.height)) / 2 - 14
        let ring = 11 * Double(reading)
        let r = (half - ring) + (half / 1.1833 - (half - ring)) * styleMix
        return (CGPoint(x: box.midX, y: box.midY), max(r, 40) * zoom)
    }

    /// How far the Persian page's frame has come in: as the globe's own frame leaves the screen, the
    /// screen becomes the manuscript's panel (Globe.metal's page frame).
    private var pageFrame: Double { styleMix * smoothstep(1.3, 2.0, zoom) }

    /// The page frame's four strips, in points, while it shows: no name goes under them.
    private var pageStrips: [CGRect] {
        guard pageFrame > 0.01 else { return [] }
        let d = (6 + 16.81) * reading, s = safeAreaInsets, w = bounds.width, h = bounds.height
        return [CGRect(x: 0, y: 0, width: w, height: s.top + d), CGRect(x: 0, y: h - s.bottom - d, width: w, height: s.bottom + d),
                CGRect(x: 0, y: 0, width: s.left + d, height: h), CGRect(x: w - s.right - d, y: 0, width: s.right + d, height: h)]
    }

    /// Measures the globe's room from the safe area and what covers the screen (`keepClear`: the
    /// buttons above, the caption below).
    private func measureRoom() {
        var top = Double(safeAreaInsets.top), bottom = Double(bounds.height - safeAreaInsets.bottom)
        for r in keepClear where !r.isEmpty {
            if r.midY < bounds.midY { top = max(top, Double(r.maxY)) } else { bottom = min(bottom, Double(r.minY)) }
        }
        room = CGRect(x: 0, y: top, width: Double(bounds.width), height: max(bottom - top, Double(bounds.height) * 0.4))
        // (rects laid out for another size, as just after a rotation, don't settle it: their
        // successors are measured again when they come)
        let measured = !keepClear.isEmpty && clearBounds == bounds.size
        if measured && !roomMeasured { roomSince = CACurrentMediaTime() }
        roomMeasured = measured
        needsDraw = true
    }

    /// The screen, its safe area or the style changed: the room is measured again, and again as
    /// what covers the screen settles into its new place.
    private func unsettleRoom() {
        roomMeasured = false
        measureRoom()
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        unsettleRoom()
    }

    /// World from view: Ry(lon0) . Rx(-lat0), built from its columns (simd matrices are indexed by
    /// column: m[column][row]), and kept while the camera stays put: every name asks for it each frame.
    private var rotation: simd_double3x3 {
        if let c = cachedRotation, c.lon == lon0, c.lat == lat0 { return c.m }
        let a = lon0 * .pi / 180, b = -lat0 * .pi / 180
        let m = simd_double3x3(SIMD3<Double>(cos(a), 0, -sin(a)),
                               SIMD3<Double>(sin(a) * sin(b), cos(b), cos(a) * sin(b)),
                               SIMD3<Double>(sin(a) * cos(b), -sin(b), cos(a) * cos(b)))
        cachedRotation = (lon: lon0, lat: lat0, m: m)
        return m
    }

    /// Where a point of the globe is on screen, and how far it faces the viewer (1 at the centre, 0 at
    /// the edge, negative behind).
    private func project(lon: Double, lat: Double) -> (point: CGPoint, facing: Double) {
        let la = lat * .pi / 180, lo = lon * .pi / 180
        let w = SIMD3<Double>(cos(la) * sin(lo), sin(la), cos(la) * cos(lo))
        let v = rotation.transpose * w
        let (centre, radius) = place
        return (CGPoint(x: Double(centre.x) + v.x * radius, y: Double(centre.y) - v.y * radius), v.z)
    }

    /// The point of the globe under a point of the screen, if it is on the globe.
    private func unproject(_ p: CGPoint) -> (lon: Double, lat: Double)? {
        let (centre, radius) = place
        let x = (Double(p.x) - Double(centre.x)) / radius, y = (Double(centre.y) - Double(p.y)) / radius
        let r2 = x * x + y * y
        guard r2 < 1 else { return nil }
        let w = rotation * SIMD3<Double>(x, y, sqrt(1 - r2))
        return (atan2(w.x, w.z) * 180 / .pi, asin(min(max(w.y, -1), 1)) * 180 / .pi)
    }

    /// Turns the globe as a finger moving `d` points would drag its centre.
    private func turn(by d: CGVector) {
        let radius = place.radius
        guard radius > 0 else { return }
        lon0 -= Double(d.dx) / radius * 180 / .pi
        lat0 = min(max(lat0 + Double(d.dy) / radius * 180 / .pi, -80), 80)
        lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
        needsDraw = true
    }

    // MARK: the reader's hands

    private func touched() {
        lastTouch = CACurrentMediaTime()
        flight = nil
        spin = .zero
        #if DEBUG
        held = false
        #endif
    }

    @objc private func pan(_ g: UIPanGestureRecognizer) {
        switch g.state {
        case .began: touching += 1; touched()
        case .changed:
            let d = g.translation(in: self)
            g.setTranslation(.zero, in: self)
            turn(by: CGVector(dx: d.x, dy: d.y))
        case .ended, .cancelled, .failed:
            touching = max(0, touching - 1)
            lastTouch = CACurrentMediaTime()
            // the fling, in points a second; Reduce Motion lets the globe stop where it's left
            let v = g.velocity(in: self)
            spin = g.state == .ended && !UIAccessibility.isReduceMotionEnabled ? CGVector(dx: v.x, dy: v.y) : .zero
        default: break
        }
    }

    @objc private func pinch(_ g: UIPinchGestureRecognizer) {
        switch g.state {
        case .began: touching += 1; touched()
        case .changed:
            let f = g.location(in: self)
            let under = unproject(f)
            zoom = min(max(zoom * Double(g.scale), Self.zooms.lowerBound), Self.zooms.upperBound)
            g.scale = 1
            // the point under the fingers stays under them (two passes: the turn is a little curved)
            if let under {
                for _ in 0..<2 {
                    let now = project(lon: under.lon, lat: under.lat).point
                    turn(by: CGVector(dx: f.x - now.x, dy: f.y - now.y))
                }
            }
            needsDraw = true
        case .ended, .cancelled, .failed: touching = max(0, touching - 1); lastTouch = CACurrentMediaTime()
        default: break
        }
    }

    @objc private func doubleTap(_ g: UITapGestureRecognizer) {
        touched()
        let target = unproject(g.location(in: self)) ?? (lon: lon0, lat: lat0)
        let z = zoom < 4 ? min(zoom * 2, Self.zooms.upperBound) : 1
        fly(to: (lon: zoom < 4 ? target.lon : lon0, lat: zoom < 4 ? target.lat : lat0, zoom: z))
    }

    @objc private func tap(_ g: UITapGestureRecognizer) {
        touched()
        guard let k = country(near: g.location(in: self)), k != selected else {
            select(nil, animated: true)
            return
        }
        select(k, animated: true)
    }

    /// The country under a point of the screen or, off its coast, the nearest within a fingertip
    /// (`GlobeType.touch` reading points): small islands and states are smaller than a finger.
    private func country(near p: CGPoint) -> Int? {
        let reach = CGFloat(GlobeType.shared?.touch ?? 10) * reading
        if let w = unproject(p), let k = data?.country(lon: w.lon, lat: w.lat) { return k }
        // rings half a point apart, each probed every half point round it: the first land met is
        // the nearest (to half a point), however small the island
        var r: CGFloat = 0.5
        while r <= reach {
            let n = Int((4 * .pi * r).rounded(.up))
            for i in 0..<n {
                let a = 2 * Double.pi * Double(i) / Double(n)
                let q = CGPoint(x: p.x + r * CGFloat(cos(a)), y: p.y + r * CGFloat(sin(a)))
                if let w = unproject(q), let k = data?.country(lon: w.lon, lat: w.lat) { return k }
            }
            r += 0.5
        }
        return nil
    }

    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        !(g is UITapGestureRecognizer) && !(other is UITapGestureRecognizer)
    }

    private func fly(to target: (lon: Double, lat: Double, zoom: Double)) {
        let duration = UIAccessibility.isReduceMotionEnabled ? 0 : 0.7
        flight = Flight(from: (lon: lon0, lat: lat0, zoom: zoom),
                        to: (lon: min(max(target.lon, -180), 180), lat: min(max(target.lat, -80), 80), zoom: target.zoom),
                        start: CACurrentMediaTime(), duration: duration)
    }

    private func select(_ k: Int?, animated: Bool) {
        let still = !animated || UIAccessibility.isReduceMotionEnabled
        // straight from one country to another, the new one fades up from nothing too
        if let k, let s = shown, s != k, !still { highlight = 0 }
        selected = k
        if k != nil { shown = k }
        if still { highlight = k == nil ? 0 : 1 }
        if let k, let data {
            letterSelected()
            accessibilityValue = data.countries[k].long ?? data.countries[k].n
        } else {
            accessibilityValue = nil
        }
        onSelect?(k.flatMap { data?.countries[$0] })
        needsDraw = true
    }

    /// Changes the style: crossfaded, the names lettered afresh (Reduce Motion: at once).
    func setStyle(_ new: GlobeStyle, animated: Bool) {
        guard new != style || !animated else { return }
        style = new
        if !animated || UIAccessibility.isReduceMotionEnabled { styleMix = new == .persian ? 1 : 0 }
        // (the other style's caption is another size: measured again once it has been laid out)
        unsettleRoom()
        backgroundColor = new == .persian ? UIColor(red: 0xF3 / 255, green: 0xE9 / 255, blue: 0xD2 / 255, alpha: 1)
                                          : UIColor(red: 0xF6 / 255, green: 0xF2 / 255, blue: 0xE7 / 255, alpha: 1)
        relabel()
    }

    /// Letters every name afresh (a new style, a new text size).
    private func relabel() {
        for l in labels { l.layer?.removeFromSuperlayer() }
        makeLabels()
        letterSelected()
        labelsFading = true
        needsDraw = true
    }

    /// The chosen country's name, in the style's dress.
    private func letterSelected() {
        guard let k = shown ?? selected, let data else { return }
        let name = Self.short(data.countries[k].n).uppercased()
        let persian = style == .persian
        guard let face = faces["selected"] else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        selectedLetters.string = NSAttributedString(string: name, attributes: Self.attributes(face, labelScale))
        selectedLabel.string = persian ? nil : NSAttributedString(string: name, attributes: Self.attributes(face, labelScale, silhouette: true))
        let size = selectedLetters.preferredFrameSize()
        let pad = persian ? CGSize(width: 14 * labelScale, height: 7 * labelScale) : CGSize(width: 4, height: 2)
        selectedLabel.bounds = CGRect(x: 0, y: 0, width: ceil(size.width) + pad.width, height: ceil(size.height) + pad.height)
        selectedLetters.frame = persian ? CGRect(x: pad.width / 2, y: pad.height / 2, width: ceil(size.width), height: ceil(size.height))
                                        : selectedLabel.bounds
        selectedLabel.backgroundColor = persian ? UIColor(hex: face.halo)?.cgColor : nil
        selectedLabel.borderColor = UIColor(red: 0xC5 / 255, green: 0xAA / 255, blue: 0x6E / 255, alpha: 1).cgColor
        selectedLabel.borderWidth = persian ? 1.2 : 0
        selectedLabel.shadowColor = UIColor(hex: persian ? face.ink : face.halo)?.cgColor
        selectedLabel.shadowOpacity = persian ? 0.22 : 0.8
        selectedLabel.shadowRadius = persian ? 1.5 : 2
        selectedLabel.shadowOffset = persian ? CGSize(width: 0, height: 0.5) : .zero
        selectedRule.frame = selectedLabel.bounds.insetBy(dx: 2.2, dy: 2.2)
        selectedRule.isHidden = !persian
        CATransaction.commit()
    }

    // MARK: each frame

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, displayLink == nil {
            let link = CADisplayLink(target: GlobeLinkTarget(self), selector: #selector(GlobeLinkTarget.tick(_:)))
            link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
            link.add(to: .main, forMode: .common)
            link.isPaused = suspended
            displayLink = link
            busy = true
        } else if window == nil {
            displayLink?.invalidate()
            displayLink = nil
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let scale = traitCollection.displayScale > 0 ? traitCollection.displayScale : pixels
        pixels = scale
        selectedLabel.contentsScale = scale
        selectedLetters.contentsScale = scale
        metalLayer?.contentsScale = scale
        metalLayer?.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        labelLayer.frame = bounds
        CATransaction.commit()
        if room.map({ $0.width != Double(bounds.width) }) ?? true || roomBounds != bounds.size {
            roomBounds = bounds.size
            unsettleRoom()
        }
        // (a screen of another size reads at another scale)
        if abs(labelScale - letteredScale) > 0.001 { relabel() }
        needsDraw = true
    }

    /// The size the room was measured for.
    private var roomBounds = CGSize.zero

    fileprivate func tick(_ link: CADisplayLink) {
        let now = CACurrentMediaTime()
        let dt = min(link.targetTimestamp - (lastTick ?? link.timestamp), 0.05)
        lastTick = link.targetTimestamp
        let still = UIAccessibility.isReduceMotionEnabled
        if let f = flight {
            let u = f.duration > 0 ? min((now - f.start) / f.duration, 1) : 1
            let e = easeInOut(u)
            var dl = f.to.lon - f.from.lon
            if dl > 180 { dl -= 360 } else if dl < -180 { dl += 360 }
            lon0 = f.from.lon + dl * e
            lat0 = f.from.lat + (f.to.lat - f.from.lat) * e
            zoom = exp(log(f.from.zoom) + (log(f.to.zoom) - log(f.from.zoom)) * e)
            lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
            if u >= 1 { flight = nil }
            needsDraw = true
        } else if touching == 0, abs(spin.dx) + abs(spin.dy) > 2 {
            turn(by: CGVector(dx: spin.dx * dt, dy: spin.dy * dt))
            let decay = exp(-3.2 * dt)
            spin = CGVector(dx: spin.dx * decay, dy: spin.dy * decay)
        } else if touching == 0, selected == nil, !still, now - lastTouch > 8, idleTurns {
            // left alone, it turns as the Earth does, west to east, about one turn in a minute and a half
            lon0 -= 4 * dt
            lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
            needsDraw = true
        }
        let target = selected == nil ? 0.0 : 1.0
        let fadingUp = highlight != target
        if fadingUp {
            highlight = still ? target : highlight + (target - highlight) * min(1, dt * 9)
            if abs(highlight - target) < 0.01 { highlight = target }
            if highlight == 0 { shown = nil }
            needsDraw = true
        }
        let styleTarget = style == .persian ? 1.0 : 0.0
        let restyling = styleMix != styleTarget
        if restyling {
            styleMix = still ? styleTarget : styleMix + (styleTarget - styleMix) * min(1, dt * 6)
            if abs(styleMix - styleTarget) < 0.005 { styleMix = styleTarget }
            needsDraw = true
        }
        if needsDraw || labelsFading { labelsFading = updateLabels(dt: dt) }
        if needsDraw, draw() {
            needsDraw = false
            if let onDrawn { self.onDrawn = nil; onDrawn() }
        }
        // the hands, a fling, a flight, the highlight fading or the style changing want the full rate;
        // the slow turn (and the names fading with it) is smooth at 30
        let moving = touching > 0 || flight != nil || abs(spin.dx) + abs(spin.dy) > 2 || fadingUp || restyling
        if moving != busy {
            busy = moving
            link.preferredFrameRateRange = moving ? CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
                                                  : CAFrameRateRange(minimum: 24, maximum: 30, preferred: 30)
        }
    }

    private var idleTurns: Bool {
        #if DEBUG
        return !held
        #else
        return true
        #endif
    }

    // MARK: the names

    /// The style's faces (Globe/type.json).
    private var faces: [String: GlobeType.Face] { GlobeType.shared?.styles[style.rawValue]?.names ?? [:] }

    /// A face's attributes at `scale`. `silhouette`: the same letters filled and stroked thickly (a
    /// negative width does both) in the face's halo, Wikipedia's red under the chosen name.
    private static func attributes(_ face: GlobeType.Face, _ scale: CGFloat, silhouette: Bool = false) -> [NSAttributedString.Key: Any] {
        let size = CGFloat(face.size) * scale
        let font = UIFont(name: face.font, size: size) ?? .systemFont(ofSize: size)
        let ink = UIColor(hex: silhouette ? face.halo : face.ink) ?? .black
        // UIKit's key for measuring, Core Text's for drawing
        var a: [NSAttributedString.Key: Any] = [.font: font, .kern: CGFloat(face.tracking) * size, .foregroundColor: ink,
                                                NSAttributedString.Key(kCTForegroundColorAttributeName as String): ink.cgColor]
        if silhouette {
            a[.strokeWidth] = -18.0
            a[.strokeColor] = ink
            a[NSAttributedString.Key(kCTStrokeColorAttributeName as String)] = ink.cgColor
        }
        return a
    }

    /// How much larger than at 1x lines and names are drawn on this screen: a larger one is held
    /// further away (`GlobeType.readingScale`).
    private var reading: CGFloat {
        let side = min(bounds.width, bounds.height)
        return CGFloat(GlobeType.shared?.readingScale(shortSide: Double(side > 0 ? side : 400)) ?? 1)
    }

    /// The names' scale: the screen's reading scale, and the reader's text size (up to `readerMax`;
    /// fewer names then fit, and the order of claim decides which).
    private var labelScale: CGFloat {
        let reader = UIFontMetrics(forTextStyle: .footnote).scaledValue(for: 10, compatibleWith: traitCollection) / 10
        return reading * min(max(reader, 1), CGFloat(GlobeType.shared?.readerMax ?? 1.5))
    }
    /// The scale the names were last lettered at.
    private var letteredScale: CGFloat = 0

    /// Points round a name's letters in its halo.
    private var halo: CGFloat { CGFloat(GlobeType.shared?.halo ?? 1.5) * labelScale }

    /// A name's picture: its letters over their halo (a round-joined stroke of the ground's tone,
    /// so no line crosses them), and a Persian capital's rosette left of them.
    private func picture(of l: Label) -> CGImage? {
        let format = UIGraphicsImageRendererFormat()
        format.scale = pixels
        format.opaque = false
        let halo = self.halo, mark = l.mark ? rosette : nil
        // the halo's pass: the same letters in the halo's tone (Core Text strokes in the text's own
        // colour unless the string names another, which would draw the halo in ink)
        let tone = (UIColor(hex: l.halo) ?? .white).withAlphaComponent(0.92)
        let toned = NSMutableAttributedString(attributedString: l.text)
        toned.addAttributes([.foregroundColor: tone, NSAttributedString.Key(kCTForegroundColorAttributeName as String): tone.cgColor,
                             NSAttributedString.Key(kCTStrokeColorAttributeName as String): tone.cgColor],
                            range: NSRange(location: 0, length: toned.length))
        let image = UIGraphicsImageRenderer(size: l.size, format: format).image { context in
            let c = context.cgContext
            let line = CTLineCreateWithAttributedString(l.text)
            let haloLine = CTLineCreateWithAttributedString(toned)
            var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
            let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
            if let mark {
                c.draw(mark, in: CGRect(x: halo + 1, y: (l.size.height - l.markSize) / 2, width: l.markSize, height: l.markSize))
            }
            // Core Text draws with y up
            c.translateBy(x: 0, y: l.size.height)
            c.scaleBy(x: 1, y: -1)
            c.textMatrix = .identity
            let origin = CGPoint(x: l.mark ? l.size.width - width - halo - 1 : (l.size.width - width) / 2,
                                 y: (l.size.height - ascent - descent) / 2 + descent)
            c.setLineJoin(.round)
            c.setLineWidth(halo * 2)
            c.setStrokeColor(tone.cgColor)
            c.setTextDrawingMode(.fillStroke)
            c.textPosition = origin
            CTLineDraw(haloLine, c)
            c.setTextDrawingMode(.fill)
            c.textPosition = origin
            CTLineDraw(line, c)
        }
        return image.cgImage
    }

    /// The shorter name a map would letter.
    private static func short(_ name: String) -> String {
        name == "United States of America" ? "United States" : name
    }

    private func setUpLabels() {
        labelLayer.masksToBounds = true
        layer.addSublayer(labelLayer)
        selectedLabel.alignmentMode = .center
        selectedLabel.isHidden = true
        selectedLetters.alignmentMode = .center
        selectedRule.borderColor = UIColor(red: 0x5F / 255, green: 0x6C / 255, blue: 0xA7 / 255, alpha: 1).cgColor
        selectedRule.borderWidth = 0.5
        selectedRule.isHidden = true
        selectedLabel.addSublayer(selectedRule)
        selectedLabel.addSublayer(selectedLetters)
        labelLayer.addSublayer(selectedLabel)
        makeLabels()
    }

    /// The names, lettered in the style's faces at the names' scale: oceans and seas, countries
    /// (larger the more prominent), capitals.
    private func makeLabels() {
        let faces = self.faces
        guard let data, let type = GlobeType.shared else { labels = []; return }
        let persian = style == .persian
        let scale = labelScale, halo = self.halo
        letteredScale = scale
        func label(_ text: String, _ key: String, _ c: [Double], _ kind: Kind, _ index: Int, from: Double, mark: Bool = false) -> Label? {
            guard c.count == 2, let face = faces[key] else { return nil }
            let s = NSAttributedString(string: text, attributes: Self.attributes(face, scale))
            let size = s.size()
            let room = 2 * halo + 2
            let markSize = CGFloat(face.size) * 0.9 * scale
            return Label(text: s, size: CGSize(width: ceil(size.width) + room + (mark ? markSize * 1.33 : 0), height: ceil(size.height) + room),
                         lon: c[0], lat: c[1], kind: kind, index: index, from: from, mark: mark, markSize: markSize,
                         halo: face.halo, gap: CGFloat(type.gap * face.size) * scale)
        }
        // in the order they claim room: oceans, seas, countries, capitals
        var all: [Label?] = []
        for (i, s) in data.seas.enumerated() where s.ocean { all.append(label(s.n, "ocean", s.c, .sea, i, from: 0)) }
        for (i, s) in data.seas.enumerated().sorted(by: { $0.element.r < $1.element.r }) where !s.ocean {
            all.append(label(s.n, "sea", s.c, .sea, i, from: s.r + 2.5))
        }
        for (i, c) in data.countries.enumerated().sorted(by: { ($0.element.min, $0.element.r) < ($1.element.min, $1.element.r) }) {
            all.append(label(Self.short(c.n).uppercased(), type.countryFace(rank: c.r), c.c, .country, i, from: c.min + 0.5))
        }
        for (i, c) in data.capitals.enumerated().sorted(by: { ($0.element.r ?? 9) < ($1.element.r ?? 9) }) {
            all.append(persian ? label(c.n, "capital", c.c, .capital, i, from: 5.6 + (c.r ?? 5) * 0.25, mark: true)
                               : label("· " + c.n, "capital", c.c, .capital, i, from: 5.6 + (c.r ?? 5) * 0.25))
        }
        labels = all.compactMap { $0 }
    }

    /// Where on its picture a name is pinned to its place: the middle, or a capital's rosette.
    private func anchor(_ l: Label) -> CGPoint {
        l.mark ? CGPoint(x: (halo + 1 + l.markSize / 2) / l.size.width, y: 0.5) : CGPoint(x: 0.5, y: 0.5)
    }

    /// Which names show (in their order of claim, where they face the viewer, the zoom has reached
    /// them and they lie on the globe, if they don't touch one already placed), fading them in and
    /// out. Returns whether any is still fading.
    private func updateLabels(dt: Double) -> Bool {
        let (centre, radius) = place
        var fading = false
        let level = log2(2 * .pi * radius / 256) + 1.2      // the web-map zoom the globe is drawn at
        var placed: [CGRect] = []
        let fade = min(1, dt * 10)
        let gap = CGFloat(GlobeType.shared?.gap ?? 0.4) * 10 * labelScale     // round the chosen name and what covers the screen
        let clear = (keepClear + pageStrips).map { $0.insetBy(dx: -gap, dy: -gap) }
        /// Whether a name's picture, there, may show: wholly on the globe (none spills onto the frame
        /// or the paper), on screen, under nothing else and touching no name already placed.
        func onGlobe(_ r: CGRect) -> Bool {
            let far = max(hypot(Double(r.minX - centre.x), Double(r.minY - centre.y)), hypot(Double(r.maxX - centre.x), Double(r.minY - centre.y)),
                          hypot(Double(r.minX - centre.x), Double(r.maxY - centre.y)), hypot(Double(r.maxX - centre.x), Double(r.maxY - centre.y)))
            return far < radius * 0.99 && bounds.contains(r)
        }
        func fits(_ r: CGRect) -> Bool {
            onGlobe(r) && !clear.contains(where: { $0.intersects(r) }) && !placed.contains(where: { $0.intersects(r) })
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        // the chosen country's name first; like every name, readable or not shown: it fades in time,
        // never by where it is (the cut-off has some slack, so it doesn't flicker there)
        var selectedTarget = 0.0
        if let shown, highlight > 0.01, let data {
            let c = data.countries[shown]
            let (p, facing) = project(lon: c.c[0], lat: c.c[1])
            let r = selectedLabel.bounds.offsetBy(dx: p.x - selectedLabel.bounds.width / 2, dy: p.y - selectedLabel.bounds.height / 2)
            if facing > (selectedAlpha > 0.5 ? 0.2 : 0.3), onGlobe(r), !clear.contains(where: { $0.intersects(r) }) {
                selectedTarget = 1
                placed.append(r.insetBy(dx: -gap / 2, dy: -gap / 2))
            }
            selectedLabel.position = p
        }
        selectedAlpha += (selectedTarget - selectedAlpha) * fade
        if abs(selectedTarget - selectedAlpha) < 0.005 { selectedAlpha = selectedTarget } else { fading = true }
        selectedLabel.isHidden = selectedAlpha * highlight < 0.01
        selectedLabel.opacity = Float(selectedAlpha * highlight)
        for i in labels.indices {
            var l = labels[i]
            var target = 0.0
            var point: CGPoint?
            if level >= l.from, !(l.kind == .country && l.index == shown && highlight > 0.01) {
                let (p, facing) = project(lon: l.lon, lat: l.lat)
                point = facing > 0 ? p : nil
                if facing > (l.alpha > 0.5 ? 0.3 : 0.4) {
                    let a = anchor(l)
                    let r = CGRect(x: p.x - l.size.width * a.x, y: p.y - l.size.height / 2, width: l.size.width, height: l.size.height)
                    let g = r.insetBy(dx: -l.gap / 2, dy: -l.gap / 2)
                    if fits(g) {
                        placed.append(g)
                        target = 1
                    }
                }
            }
            if target == 0 && l.alpha < 0.01 {
                if let layer = l.layer, !layer.isHidden { layer.isHidden = true }
                labels[i].alpha = 0
                continue
            }
            l.alpha += (target - l.alpha) * fade
            if abs(target - l.alpha) < 0.005 { l.alpha = target } else { fading = true }
            if l.layer == nil {
                let t = CALayer()
                t.contents = picture(of: l)
                t.contentsScale = pixels
                t.bounds = CGRect(origin: .zero, size: l.size)
                t.anchorPoint = anchor(l)
                labelLayer.insertSublayer(t, below: selectedLabel)
                l.layer = t
            }
            if let layer = l.layer {
                layer.isHidden = false
                // (fading out, it keeps to its place on the globe while that faces the viewer)
                if let point { layer.position = point }
                layer.opacity = Float(l.alpha)
            }
            labels[i] = l
        }
        CATransaction.commit()
        return fading
    }
}

/// Breaks the display link → view retain cycle.
private final class GlobeLinkTarget: NSObject {
    weak var view: GlobeView?

    init(_ view: GlobeView) { self.view = view }

    @objc func tick(_ link: CADisplayLink) { view?.tick(link) }
}
