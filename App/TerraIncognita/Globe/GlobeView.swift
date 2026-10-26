import CoreGraphics
import CoreText
import Foundation
import ImageIO
import Metal
import QuartzCore
import UIKit
import simd

/// The whole globe, drawn live by Metal (`Globe.metal`) in the colours of Wikipedia's location maps,
/// a little prettier: drag to spin it, pinch to zoom, double-tap to fly in, tap a country to light it
/// in the locator's red. Left alone it turns slowly, as the Earth does.
final class GlobeView: UIView, UIGestureRecognizerDelegate {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    /// A country was tapped (nil: the selection was cleared).
    var onSelect: ((GlobeData.Country?) -> Void)?
    /// The country in the locator's red, if any.
    var selectedCountry: GlobeData.Country? { selected.flatMap { data?.countries[$0] } }

    private let data = GlobeData.load()
    private let device = MTLCreateSystemDefaultDevice()
    private var queue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?
    private var geo: MTLTexture?
    private var terrain: MTLTexture?
    private var ids: MTLTexture?
    private var displayLink: CADisplayLink?
    private var lastTick: CFTimeInterval?
    private var needsDraw = true

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
    private let labelLayer = CALayer()
    private var labels: [Label] = []
    private let selectedLabel = CATextLayer()
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
        var layer: CATextLayer?
        var alpha = 0.0
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        GlobeData.registerFonts()
        backgroundColor = UIColor(red: 0xF6 / 255, green: 0xF2 / 255, blue: 0xE7 / 255, alpha: 1)
        isMultipleTouchEnabled = true
        setUpMetal()
        setUpLabels()
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
        ids = Self.texture(GlobeData.root.appending(path: "ids.png"), device: device, queue: queue)
    }

    /// The image's decoded bytes uploaded as they are: colour management would bend the distances
    /// the map is made of. RGBA images get mipmaps; a grey one (the countries) is read exactly.
    private static func texture(_ url: URL, device: MTLDevice, queue: MTLCommandQueue) -> MTLTexture? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil), image.bitsPerComponent == 8,
              let data = image.dataProvider?.data, let bytes = CFDataGetBytePtr(data) else { return nil }
        let w = image.width, h = image.height, row = image.bytesPerRow, channels = image.bitsPerPixel / 8
        guard channels == 1 || channels == 3 || channels == 4, CFDataGetLength(data) >= row * h else { return nil }
        let grey = channels == 1
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: grey ? .r8Unorm : .rgba8Unorm, width: w, height: h, mipmapped: !grey)
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
        } else {
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: bytes, bytesPerRow: row)
        }
        if !grey, let commands = queue.makeCommandBuffer(), let blit = commands.makeBlitCommandEncoder() {
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
    }

    private func draw() {
        guard let pipeline, let queue, let geo, let terrain, let ids, let metalLayer,
              bounds.width > 0, bounds.height > 0, let drawable = metalLayer.nextDrawable(),
              let commands = queue.makeCommandBuffer() else { return }
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .dontCare
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = commands.makeRenderCommandEncoder(descriptor: pass) else { return }
        let scale = Double(metalLayer.contentsScale)
        let (centre, radius) = place
        let m = rotation
        let ring = 1 - smoothstep(1.25, 1.7, zoom)
        var u = Uniforms(
            u0: SIMD4(Float(Double(bounds.width) * scale), Float(Double(bounds.height) * scale), Float(Double(centre.x) * scale), Float(Double(centre.y) * scale)),
            u1: SIMD4(Float(radius * scale), Float(scale), Float(zoom), Float(ring)),
            u2: SIMD4(Float(shown.map { $0 + 1 } ?? 0), Float(highlight), 0, 0),
            r0: SIMD4(Float(m[0][0]), Float(m[1][0]), Float(m[2][0]), 0),
            r1: SIMD4(Float(m[0][1]), Float(m[1][1]), Float(m[2][1]), 0),
            r2: SIMD4(Float(m[0][2]), Float(m[1][2]), Float(m[2][2]), 0))
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        encoder.setFragmentTexture(geo, index: 0)
        encoder.setFragmentTexture(terrain, index: 1)
        encoder.setFragmentTexture(ids, index: 2)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        commands.present(drawable)
        commands.commit()
    }

    // MARK: the camera

    /// The globe's centre and radius on screen, in points (tools/globe/preview.py's `layout`).
    private var place: (centre: CGPoint, radius: Double) {
        let w = Double(bounds.width), h = Double(bounds.height)
        return (CGPoint(x: w / 2, y: h * 0.43), 0.455 * min(w, h * 0.55) * zoom)
    }

    /// World from view: Ry(lon0) . Rx(-lat0). (simd matrices are indexed by column: m[column][row].)
    private var rotation: simd_double3x3 {
        let a = lon0 * .pi / 180, b = -lat0 * .pi / 180
        return simd_double3x3(rows: [SIMD3<Double>(cos(a), sin(a) * sin(b), sin(a) * cos(b)),
                                     SIMD3<Double>(0, cos(b), -sin(b)),
                                     SIMD3<Double>(-sin(a), cos(a) * sin(b), cos(a) * cos(b))])
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
        guard let p = unproject(g.location(in: self)), let k = data?.country(lon: p.lon, lat: p.lat), k != selected else {
            select(nil, animated: true)
            return
        }
        select(k, animated: true)
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
        selected = k
        if k != nil { shown = k }
        if !animated || UIAccessibility.isReduceMotionEnabled { highlight = k == nil ? 0 : 1 }
        if let k, let data {
            let c = data.countries[k]
            selectedLabel.string = NSAttributedString(string: Self.short(c.n).uppercased(), attributes: Self.attributes(.selected))
            selectedLabel.bounds = CGRect(origin: .zero, size: selectedLabel.preferredFrameSize())
            accessibilityValue = c.long ?? c.n
        } else {
            accessibilityValue = nil
        }
        onSelect?(k.flatMap { data?.countries[$0] })
        needsDraw = true
    }

    // MARK: each frame

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, displayLink == nil {
            let link = CADisplayLink(target: GlobeLinkTarget(self), selector: #selector(GlobeLinkTarget.tick(_:)))
            link.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 120, preferred: 120)
            link.add(to: .main, forMode: .common)
            displayLink = link
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
        metalLayer?.contentsScale = scale
        metalLayer?.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        labelLayer.frame = bounds
        CATransaction.commit()
        needsDraw = true
    }

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
        if highlight != target {
            highlight = still ? target : highlight + (target - highlight) * min(1, dt * 9)
            if abs(highlight - target) < 0.01 { highlight = target }
            if highlight == 0 { shown = nil }
            needsDraw = true
        }
        updateLabels(dt: dt)
        if needsDraw {
            needsDraw = false
            draw()
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

    private enum Style { case country, sea, ocean, capital, selected }

    private static func attributes(_ style: Style) -> [NSAttributedString.Key: Any] {
        func font(_ name: String, _ size: CGFloat) -> UIFont { UIFont(name: name, size: size) ?? .systemFont(ofSize: size) }
        func ink(_ hex: UInt32) -> [NSAttributedString.Key: Any] {
            let c = UIColor(red: CGFloat((hex >> 16) & 0xFF) / 255, green: CGFloat((hex >> 8) & 0xFF) / 255, blue: CGFloat(hex & 0xFF) / 255, alpha: 1)
            // UIKit's key for measuring, Core Text's for CATextLayer, which draws with it
            return [.foregroundColor: c, NSAttributedString.Key(kCTForegroundColorAttributeName as String): c.cgColor]
        }
        switch style {
        case .country: return ink(0x5F5A51).merging([.font: font("LibertinusSerif-Regular", 9.5), .kern: 0.9]) { a, _ in a }
        case .sea: return ink(0x4A8DBB).merging([.font: font("UncialAntiqua-Regular", 8.5), .kern: 0.4]) { a, _ in a }
        case .ocean: return ink(0x3E84B6).merging([.font: font("UncialAntiqua-Regular", 10), .kern: 1.2]) { a, _ in a }
        case .capital: return ink(0x4A463F).merging([.font: font("LibertinusSerif-Italic", 9.5)]) { a, _ in a }
        case .selected: return ink(0xFFF7EA).merging([.font: font("LibertinusSerif-Semibold", 11), .kern: 1.0]) { a, _ in a }
        }
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
        labelLayer.addSublayer(selectedLabel)
        guard let data else { return }
        func label(_ text: String, _ style: Style, _ c: [Double], _ kind: Kind, _ index: Int, from: Double) -> Label? {
            guard c.count == 2 else { return nil }
            let s = NSAttributedString(string: text, attributes: Self.attributes(style))
            let size = s.size()
            return Label(text: s, size: CGSize(width: ceil(size.width) + 2, height: ceil(size.height) + 2), lon: c[0], lat: c[1], kind: kind, index: index, from: from)
        }
        // in the order they claim room: oceans, seas, countries, capitals
        var all: [Label?] = []
        for (i, s) in data.seas.enumerated() where s.ocean { all.append(label(s.n, .ocean, s.c, .sea, i, from: 0)) }
        for (i, s) in data.seas.enumerated().sorted(by: { $0.element.r < $1.element.r }) where !s.ocean {
            all.append(label(s.n, .sea, s.c, .sea, i, from: s.r + 2.5))
        }
        for (i, c) in data.countries.enumerated().sorted(by: { ($0.element.min, $0.element.r) < ($1.element.min, $1.element.r) }) {
            all.append(label(Self.short(c.n).uppercased(), .country, c.c, .country, i, from: c.min + 0.5))
        }
        for (i, c) in data.capitals.enumerated().sorted(by: { ($0.element.r ?? 9) < ($1.element.r ?? 9) }) {
            all.append(label("· " + c.n, .capital, c.c, .capital, i, from: 5.6 + (c.r ?? 5) * 0.25))
        }
        labels = all.compactMap { $0 }
    }

    /// Each frame: which names show (in their order, where they face the viewer and the zoom has
    /// reached them, if they don't touch one already placed), fading them in and out.
    private func updateLabels(dt: Double) {
        let radius = place.radius
        let level = log2(2 * .pi * radius / 256) + 1.2      // the web-map zoom the globe is drawn at
        var placed: [CGRect] = []
        let fade = min(1, dt * 10)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        if let shown, highlight > 0.01, let data {
            let c = data.countries[shown]
            let (p, facing) = project(lon: c.c[0], lat: c.c[1])
            selectedLabel.isHidden = facing < 0.2
            selectedLabel.position = p
            selectedLabel.opacity = Float(highlight * min(1, (facing - 0.2) / 0.2))
            placed.append(CGRect(x: p.x - selectedLabel.bounds.width / 2 - 3, y: p.y - selectedLabel.bounds.height / 2 - 3,
                                 width: selectedLabel.bounds.width + 6, height: selectedLabel.bounds.height + 6))
        } else {
            selectedLabel.isHidden = true
        }
        for i in labels.indices {
            var l = labels[i]
            var target = 0.0
            var point: CGPoint?
            if level >= l.from, !(l.kind == .country && l.index == shown && highlight > 0.01) {
                let (p, facing) = project(lon: l.lon, lat: l.lat)
                point = facing > 0 ? p : nil
                if facing > 0.25 {
                    let r = CGRect(x: p.x - l.size.width / 2 - 3, y: p.y - l.size.height / 2 - 3, width: l.size.width + 6, height: l.size.height + 6)
                    if bounds.insetBy(dx: -20, dy: -20).intersects(r), !placed.contains(where: { $0.intersects(r) }) {
                        placed.append(r)
                        target = min(1, (facing - 0.25) / 0.25)
                    }
                }
            }
            if target == 0 && l.alpha < 0.01 {
                if let layer = l.layer, !layer.isHidden { layer.isHidden = true }
                labels[i].alpha = 0
                continue
            }
            l.alpha += (target - l.alpha) * fade
            if l.layer == nil {
                let t = CATextLayer()
                t.string = l.text
                t.contentsScale = pixels
                t.alignmentMode = .center
                t.bounds = CGRect(origin: .zero, size: l.size)
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
    }
}

/// Breaks the display link → view retain cycle.
private final class GlobeLinkTarget: NSObject {
    weak var view: GlobeView?

    init(_ view: GlobeView) { self.view = view }

    @objc func tick(_ link: CADisplayLink) { view?.tick(link) }
}
