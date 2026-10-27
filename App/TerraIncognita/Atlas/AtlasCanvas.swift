import CoreGraphics
import Foundation
import QuartzCore
import UIKit

/// Plays a look's plates as one continuous zoom. Each plate is a stack of finished pictures (its
/// layers), drawn by Core Animation; every frame only moves and scales them, so the film holds the
/// display's full rate. At most three plates are on screen: the one the camera is in, its parent
/// behind it while it doesn't yet cover the screen, and its child fading up as the camera dives.
final class AtlasCanvas: UIView, UIGestureRecognizerDelegate {
    enum Mode: Equatable { case film, free, returning, ended }

    var onCaption: ((Plate.Caption?) -> Void)?
    var onMode: ((Mode) -> Void)?

    private(set) var mode: Mode = .film {
        didSet { if mode != oldValue { updateAccessibilityProgress(force: true); onMode?(mode) } }
    }
    /// While the globe is shown the film holds still and nothing is drawn.
    var suspended = false {
        didSet {
            guard suspended != oldValue else { return }
            if suspended { suspendedAt = CACurrentMediaTime() }
            else if let since = suspendedAt {
                if mode == .returning { returnStart += CACurrentMediaTime() - since }
                suspendedAt = nil
            }
            displayLink?.isPaused = suspended
            lastTick = nil
            updateAccessibilityProgress(force: true)
        }
    }
    private var suspendedAt: CFTimeInterval?
    private var reportedProgressSecond = -1
    private var look: Look?
    private var pending: Look?
    private var ladder: Ladder?
    private var film: Film?
    private var camera = Camera(plate: 0, x: 0, y: 0, zoom: 1)
    private var filmTime = 0.0
    private var nodes: [Int: PlateNode] = [:]
    private var displayLink: CADisplayLink?
    private var lastTick: CFTimeInterval?
    private var caption: Plate.Caption?
    // free mode: the fling after a pan, in screen points per second
    private var fling = CGVector.zero
    private var touching = 0
    // returning to the film: from where, to which moment, since when
    private var returnFrom = Camera(plate: 0, x: 0, y: 0, zoom: 1)
    private var returnStart: CFTimeInterval = 0
    // Reduce Motion: the background over the map, how far a cut has dipped through it
    private let veil = CALayer()
    private var dip = 0.0
    private var busy = true
    #if DEBUG
    /// `-film 21.5` (CI's screenshots): the film held at that moment.
    private let frozenAt: Double? = {
        let args = ProcessInfo.processInfo.arguments
        guard let i = args.firstIndex(of: "-film"), i + 1 < args.count else { return nil }
        return Double(args[i + 1])
    }()
    #endif

    override init(frame: CGRect) {
        super.init(frame: frame)
        backgroundColor = UIColor(red: 0.07, green: 0.08, blue: 0.10, alpha: 1)
        veil.backgroundColor = backgroundColor?.cgColor
        veil.zPosition = 1000
        veil.opacity = 0
        layer.addSublayer(veil)
        isMultipleTouchEnabled = true
        let pan = UIPanGestureRecognizer(target: self, action: #selector(pan(_:)))
        pan.maximumNumberOfTouches = 2
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(pinch(_:)))
        for g in [pan, pinch] as [UIGestureRecognizer] { g.delegate = self; addGestureRecognizer(g) }
        isAccessibilityElement = true
        accessibilityLabel = String(localized: "A map zooming from the whole world down to a neighbourhood")
        accessibilityTraits = .image
        updateAccessibilityProgress(force: true)
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    // MARK: the look

    /// Shows a look, keeping the camera's place: the same scale, the same spot of the plate. Before the
    /// view has a size, the look waits for it.
    func show(_ look: Look) {
        guard look != self.look else { return }
        guard bounds.width > 0, bounds.height > 0 else { pending = look; setNeedsLayout(); return }
        pending = nil
        guard let set = Atlas.plates(of: look) else { return }
        let old = ladder
        self.look = look
        for node in nodes.values { node.container.removeFromSuperlayer() }
        nodes = [:]
        let new = Ladder(plates: set.plates, screen: bounds.size)
        ladder = new
        let film = Film(new)
        self.film = film
        filmTime = min(filmTime, film.duration)
        if let old {
            // the same scale (looks may pass through different plates on the way), the same spot
            var c = camera
            while c.plate > 0, old.plates[c.plate].passes { c = old.up(c) }
            let level = old.plates[...c.plate].filter { !$0.passes }.count - 1
            let stops = new.plates.indices.filter { !new.plates[$0].passes }
            let k = stops[min(level, stops.count - 1)], from = old.plates[c.plate]
            let mapped = Camera(plate: k, x: c.x / from.width * new.plates[k].width, y: c.y / from.height * new.plates[k].height,
                                zoom: c.zoom / old.restZoom(c.plate) * new.restZoom(k))
            switch mode {
            case .film, .ended: camera = new.normalized(film.camera(at: filmTime))
            case .free: camera = new.held(mapped)
            case .returning: camera = new.normalized(mapped); beginReturn()
            }
        } else {
            camera = new.rest(0)
        }
        loadAround(camera.plate, now: true)
        updateAccessibilityProgress(force: true)
        render()
    }

    // MARK: the film and the reader's hands

    func resume() {
        guard let film else { return }
        if mode == .ended || filmTime >= film.duration { filmTime = 0 }
        // under Reduce Motion the return's own dip is the cut: it lands after the dive under way
        if UIAccessibility.isReduceMotionEnabled, let end = film.diveEnd(at: filmTime) { filmTime = end }
        fling = .zero
        mode = .returning
        beginReturn()
    }

    /// Whether the film has played to its end (touching the map after doesn't make it resumable).
    var atEnd: Bool { film.map { filmTime >= $0.duration } ?? false }

    /// Actual film progress for VoiceOver, refreshed once a second and on playback-state changes.
    /// The integer check avoids formatting strings on ordinary display-link frames.
    private func updateAccessibilityProgress(force: Bool = false) {
        let second = Int(filmTime.rounded(.down))
        guard force || second != reportedProgressSecond else { return }
        reportedProgressSecond = second
        let state: String
        switch mode {
        case .film: state = "Film"
        case .free: state = "Exploring"
        case .returning: state = "Returning to film"
        case .ended: state = "Film ended"
        }
        accessibilityValue = "\(state) · \(look?.name ?? "Atlas") · \(second) seconds · \(suspended ? "Paused" : "Active")"
    }

    /// The glide back to the film starts from here and now (again after a new look or screen size,
    /// whose plates and zooms the old start isn't measured in), in the wider of the two plates.
    private func beginReturn() {
        guard let film, let ladder else { return }
        let target = film.camera(at: filmTime)
        returnFrom = ladder.express(camera, in: min(camera.plate, target.plate))
        returnStart = CACurrentMediaTime()
        if suspended { suspendedAt = returnStart }
    }

    private func enterFree() {
        if mode != .free { mode = .free }
        fling = .zero
    }

    @objc private func pan(_ g: UIPanGestureRecognizer) {
        switch g.state {
        case .began: touching += 1; enterFree()
        case .changed:
            let d = g.translation(in: self)
            g.setTranslation(.zero, in: self)
            camera.x -= Double(d.x) / camera.zoom
            camera.y -= Double(d.y) / camera.zoom
            settle()
        case .ended, .cancelled, .failed:
            touching = max(0, touching - 1)
            let v = g.velocity(in: self)
            fling = g.state == .ended ? CGVector(dx: v.x, dy: v.y) : .zero
        default: break
        }
    }

    @objc private func pinch(_ g: UIPinchGestureRecognizer) {
        switch g.state {
        case .began: touching += 1; enterFree()
        case .changed:
            guard let ladder else { return }
            let f = g.location(in: self)
            let fx = Double(f.x - bounds.midX), fy = Double(f.y - bounds.midY)
            // the plate's point under the fingers stays under them
            let px = camera.x + fx / camera.zoom, py = camera.y + fy / camera.zoom
            let z0 = camera.zoom, wanted = z0 * Double(g.scale)
            g.scale = 1
            // no deeper into a plate than its limit (a camera already past it, having climbed out of
            // the next plate, holds its zoom)
            camera.zoom = min(wanted, max(z0, ladder.maxZoom(camera.plate)))
            camera.x = px - fx / camera.zoom
            camera.y = py - fy / camera.zoom
            camera = ladder.steered(camera, from: z0, toward: wanted)
            settle()
        case .ended, .cancelled, .failed: touching = max(0, touching - 1)
        default: break
        }
    }

    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool { true }

    private func settle() {
        guard let ladder else { return }
        camera = ladder.held(camera)
        render()
    }

    // MARK: each frame

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, displayLink == nil {
            let link = CADisplayLink(target: DisplayLinkTarget(self), selector: #selector(DisplayLinkTarget.tick(_:)))
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
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        veil.frame = bounds
        CATransaction.commit()
        if let pending, bounds.width > 0 { show(pending) }
        guard let ladder, ladder.screen != bounds.size, bounds.width > 0 else { return }
        // a new size (rotation, a new window): the same view, measured against the new screen
        let rest = ladder.restZoom(camera.plate)
        let resized = Ladder(plates: ladder.plates, screen: bounds.size)
        camera.zoom *= resized.restZoom(camera.plate) / rest
        self.ladder = resized
        film = Film(resized)
        if mode == .returning { beginReturn() }
        if mode == .free { camera = resized.held(camera) }
        render()
    }

    fileprivate func tick(_ link: CADisplayLink) {
        let dt = min(link.targetTimestamp - (lastTick ?? link.timestamp), 0.05)
        lastTick = link.targetTimestamp
        guard let ladder, let film else { return }
        // Reduce Motion: no dives, no flights back; the camera cuts at the bottom of a dip instead
        let still = UIAccessibility.isReduceMotionEnabled
        dip = 0
        switch mode {
        case .film:
            filmTime += dt
            #if DEBUG
            if let frozenAt { filmTime = frozenAt }
            #endif
            if filmTime >= film.duration { filmTime = film.duration; mode = .ended }
            if still {
                let cut = film.cut(at: filmTime)
                camera = ladder.normalized(cut.camera)
                dip = cut.dip
            } else {
                camera = ladder.normalized(film.camera(at: filmTime))
            }
        case .returning:
            let u = (CACurrentMediaTime() - returnStart) / (still ? 0.8 : 1.4)
            let target = still ? film.cut(at: filmTime).camera : film.camera(at: filmTime)
            if still {
                camera = ladder.normalized(u < 0.5 ? returnFrom : target)
                dip = smoothstep(0, 1, 1 - abs(2 * u - 1))
            } else {
                camera = ladder.normalized(glide(returnFrom, ladder.express(target, in: returnFrom.plate), easeInOut(u)))
            }
            if u >= 1 { mode = .film }
        case .free:
            if touching == 0, abs(fling.dx) + abs(fling.dy) > 4 {
                camera.x -= Double(fling.dx) * dt / camera.zoom
                camera.y -= Double(fling.dy) * dt / camera.zoom
                let decay = exp(-4.5 * dt)
                fling = CGVector(dx: fling.dx * decay, dy: fling.dy * decay)
                camera = ladder.held(camera)
            }
        case .ended:
            break
        }
        // only the film, a flight back and the reader's hands need the full rate; the sprites' slow
        // motions are smooth at 30, which spares the battery on the end screen
        let moving = mode == .film || mode == .returning || touching > 0 || abs(fling.dx) + abs(fling.dy) > 4
        if moving != busy {
            busy = moving
            link.preferredFrameRateRange = moving ? CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
                                                  : CAFrameRateRange(minimum: 24, maximum: 30, preferred: 30)
        }
        let shown = mode == .film ? film.caption(at: filmTime).map { ladder.plates[$0].caption } : nil
        if shown != caption { caption = shown; onCaption?(shown) }
        updateAccessibilityProgress()
        render()
    }

    private func render() {
        guard let ladder, let look else { return }
        let b = camera.plate
        let current = ladder.opacity(b, zoom: camera.zoom)
        var visible: [(Int, Double)] = []
        // behind a plate that isn't opaque or doesn't hold the whole view, the nearest wider plate
        // that does (or the first)
        var back = b
        if b > 0, current < 1 || !ladder.inside(camera) {
            back = b - 1
            while back > 0, !ladder.inside(ladder.express(camera, in: back)) { back -= 1 }
            visible.append((back, 1))
        }
        visible.append((b, visible.isEmpty ? 1 : current))
        if b + 1 < ladder.count {
            let child = ladder.down(camera)
            let o = ladder.opacity(b + 1, zoom: child.zoom)
            if o > 0.001 { visible.append((b + 1, o)) }
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        veil.opacity = Float(dip)
        let shownIDs = Set(visible.map(\.0))
        for (k, node) in nodes where !shownIDs.contains(k) { node.container.isHidden = true }
        let still = UIAccessibility.isReduceMotionEnabled
        let t = CACurrentMediaTime()
        for (k, opacity) in visible {
            let node = self.node(k, look: look, ladder: ladder)
            let c = ladder.express(camera, in: k)
            let p = ladder.plates[k]
            node.container.isHidden = false
            node.container.opacity = Float(opacity)
            node.container.zPosition = CGFloat(k)
            node.container.position = CGPoint(x: Double(bounds.midX) - c.x * c.zoom, y: Double(bounds.midY) - c.y * c.zoom)
            node.container.transform = CATransform3DMakeScale(CGFloat(c.zoom), CGFloat(c.zoom), 1)
            // a plate drawn over the one behind has soft edges, so its border never shows as a seam:
            // wide while it fades up, hardening as it takes over, and narrow while the plate behind
            // shows past it, growing from nothing as the view first reaches past its edge (the reader
            // can stop short of the closer plate, or pan off its edge, where the film never does)
            let fade = k > b || (k == b && opacity < 1) ? 0.2 * (1 - opacity) : 0
            let narrow = k > back ? min(0.04, ladder.overhang(c) / p.width) : 0
            node.feather(max(fade, narrow), plateWidth: p.width)
            // nearer layers move further with the camera, in proportion to its offset from the centre,
            // measured at rest so a dive doesn't fling them apart
            let s = ladder.restZoom(k) / c.zoom
            let ox = -(c.x - p.width / 2) * s, oy = -(c.y - p.height / 2) * s
            let depth = still ? 0 : 1.0
            // names swell as the camera dives; past about twice their size they fade, as a map's do
            // (a pass plate's names are its scale's, magnified), except on the last scale, which no
            // closer one takes over from
            let scale = ladder.stop(of: k)
            let names = scale == ladder.count - 1 ? 1
                : Float(1 - smoothstep(1.7, 2.8, ladder.express(camera, in: scale).zoom / ladder.restZoom(scale)))
            for (ln, spec) in zip(node.layers, p.layers) where spec.depth != 0 || spec.motion != nil || spec.isLabels {
                ln.place(spec, plate: node.container.bounds, offset: CGPoint(x: ox * spec.depth * depth, y: oy * spec.depth * depth), time: t, still: still)
                if spec.isLabels { ln.layer.opacity = names }
            }
        }
        CATransaction.commit()
        loadAround(b, now: false, back: back)
    }

    // MARK: the pictures

    private func node(_ k: Int, look: Look, ladder: Ladder) -> PlateNode {
        if let n = nodes[k] { return n }
        let p = ladder.plates[k]
        let container = CALayer()
        container.isHidden = true
        container.anchorPoint = .zero
        container.bounds = CGRect(x: 0, y: 0, width: p.width, height: p.height)
        let layers = p.layers.map { spec in
            let ln = LayerNode(spec, plate: container.bounds)
            container.addSublayer(ln.layer)
            return ln
        }
        layer.addSublayer(container)
        let n = PlateNode(container: container, layers: layers)
        nodes[k] = n
        return n
    }

    /// Plates b - 1 (or the wider one drawn behind, `back`) to b + 2 have their pictures; the rest let
    /// theirs go. The camera's own plate is decoded at once the first time (nothing would show
    /// otherwise), the others in the background.
    private func loadAround(_ b: Int, now: Bool, back: Int? = nil) {
        guard let ladder, let look else { return }
        let keep = max(0, min(back ?? b, b - 1))...min(ladder.count - 1, b + 2)
        // (setting contents outside a transaction without actions would cross-fade them, keeping the
        // old picture alive a quarter second)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        defer { CATransaction.commit() }
        for (k, n) in nodes where !keep.contains(k) && n.loaded { n.unload() }
        for k in keep {
            let n = node(k, look: look, ladder: ladder)
            guard !n.loaded, !n.loading else { continue }
            n.loading = true
            let urls = ladder.plates[k].layers.map { Atlas.url(of: $0, in: look) }
            if now && k == b {
                // (decoding ahead can fail where the hardware decoder is missing, as in the simulator:
                // then the picture decodes as it is first drawn)
                for (ln, url) in zip(n.layers, urls) {
                    let image = UIImage(contentsOfFile: url.path)
                    ln.image.contents = (image?.preparingForDisplay() ?? image)?.cgImage
                }
                n.loaded = true
                n.loading = false
            } else {
                Task { [weak self] in
                    var images: [UIImage?] = []
                    for url in urls {
                        guard let image = UIImage(contentsOfFile: url.path) else { images.append(nil); continue }
                        images.append(await image.byPreparingForDisplay() ?? image)
                    }
                    // the look may have changed while these were decoding
                    guard let self, self.nodes[k] === n else { return }
                    CATransaction.begin()
                    CATransaction.setDisableActions(true)
                    for (ln, image) in zip(n.layers, images) { ln.image.contents = image?.cgImage }
                    CATransaction.commit()
                    n.loaded = true
                    n.loading = false
                }
            }
        }
    }
}

private final class PlateNode {
    let container: CALayer
    let layers: [LayerNode]
    private let edge = CALayer()
    var loaded = false
    var loading = false

    init(container: CALayer, layers: [LayerNode]) {
        self.container = container
        self.layers = layers
        edge.contents = Self.softEdge
        // the corners and sides of the picture keep their size; only the middle stretches
        edge.contentsCenter = CGRect(x: 0.45, y: 0.45, width: 0.1, height: 0.1)
    }

    func unload() {
        for l in layers { l.image.contents = nil }
        loaded = false
    }

    /// Fades the plate's edges over `fraction` of its width, or not at all.
    func feather(_ fraction: Double, plateWidth: Double) {
        guard fraction > 0.002 else {
            if container.mask != nil { container.mask = nil }
            return
        }
        edge.frame = container.bounds
        // the soft edge is 45% of the picture's 64 pixels; contentsScale sets how wide it draws
        edge.contentsScale = CGFloat(64 * 0.45 / (fraction * plateWidth))
        if container.mask !== edge { container.mask = edge }
    }

    /// White, opaque in the middle and fading to nothing at its edges.
    private static let softEdge: CGImage? = {
        let n = 64
        var px = [UInt8](repeating: 0, count: n * n * 4)
        for y in 0..<n {
            for x in 0..<n {
                let d = Double(min(x, n - 1 - x, y, n - 1 - y)) / (Double(n) * 0.45)
                let a = UInt8(255 * smoothstep(0, 1, d))
                let i = (y * n + x) * 4
                px[i] = a; px[i + 1] = a; px[i + 2] = a; px[i + 3] = a
            }
        }
        guard let provider = CGDataProvider(data: Data(px) as CFData) else { return nil }
        return CGImage(width: n, height: n, bitsPerComponent: 8, bitsPerPixel: 32, bytesPerRow: n * 4,
                       space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue),
                       provider: provider, decode: nil, shouldInterpolate: true, intent: .defaultIntent)
    }()
}

/// One layer of a plate: the layer moved on screen, the one that shows its picture (the same, or a
/// sheen's mask), and a sheen's band of light.
private final class LayerNode {
    let layer: CALayer
    let image: CALayer
    private let band: CAGradientLayer?

    init(_ spec: Plate.Layer, plate: CGRect) {
        let frame = spec.rect ?? plate
        if let m = spec.motion, m.kind == "sheen" {
            // the picture is a mask: light shows only where the gold is
            let host = CALayer(), mask = CALayer(), band = CAGradientLayer()
            host.frame = frame
            mask.frame = host.bounds
            mask.contentsGravity = .resize
            host.mask = mask
            let light = UIColor(hex: m.color ?? "#fff4cc") ?? .white
            let peak = CGFloat(m.opacity ?? 0.55)
            band.colors = [light.withAlphaComponent(0).cgColor, light.withAlphaComponent(peak).cgColor, light.withAlphaComponent(0).cgColor]
            band.startPoint = CGPoint(x: 0, y: 0.5)
            band.endPoint = CGPoint(x: 1, y: 0.5)
            band.isHidden = true
            host.addSublayer(band)
            layer = host
            image = mask
            self.band = band
        } else {
            let l = CALayer()
            l.frame = frame
            layer = l
            image = l
            band = nil
        }
        image.contentsGravity = .resize
        image.minificationFilter = .trilinear
        image.magnificationFilter = .linear
    }

    /// Where the layer sits this frame: its place, the camera's parallax `offset`, and its motion.
    func place(_ spec: Plate.Layer, plate: CGRect, offset: CGPoint, time t: Double, still: Bool) {
        let base = spec.rect ?? plate
        var dx = Double(offset.x), dy = Double(offset.y)
        let m = spec.motion
        switch m?.kind ?? "" {
        case "bob" where !still:
            let w = m!.wave(t), amp = m!.amp ?? [0, 0]
            dx += (amp.first ?? 0) * w
            dy += (amp.count > 1 ? amp[1] : 0) * w
        case "breathe" where !still:
            let lo = m!.min ?? 0.6, hi = m!.max ?? 1
            layer.opacity = Float(lo + (hi - lo) * (0.5 + 0.5 * m!.wave(t)))
        case "sheen":
            sweep(m!, size: base.size, time: t, still: still)
        default: break
        }
        if let m, m.kind == "sway" {
            // turns about its anchor, a point of the plate
            let a = m.anchor ?? [Double(base.midX), Double(base.midY)]
            let ax = a.first ?? Double(base.midX), ay = a.count > 1 ? a[1] : Double(base.midY)
            layer.anchorPoint = CGPoint(x: (ax - Double(base.minX)) / Double(base.width), y: (ay - Double(base.minY)) / Double(base.height))
            layer.bounds = CGRect(origin: .zero, size: base.size)
            layer.position = CGPoint(x: ax + dx, y: ay + dy)
            let angle = still ? 0 : (m.angle ?? 0) * .pi / 180 * m.wave(t)
            layer.transform = CATransform3DMakeRotation(CGFloat(angle), 0, 0, 1)
        } else {
            layer.frame = base.offsetBy(dx: dx, dy: dy)
        }
    }

    /// The band of light crosses the gold in the first third of each period, then waits.
    private func sweep(_ m: Plate.Motion, size: CGSize, time t: Double, still: Bool) {
        guard let band else { return }
        image.frame = CGRect(origin: .zero, size: size)
        let period = max(m.period ?? 7, 1)
        let u = (t / period + (m.phase ?? 0)).truncatingRemainder(dividingBy: 1)
        guard !still, u < 0.34 else { band.isHidden = true; return }
        let w = Double(size.width) * (m.width ?? 0.25), diagonal = hypot(Double(size.width), Double(size.height))
        band.isHidden = false
        band.bounds = CGRect(x: 0, y: 0, width: w, height: diagonal * 1.2)
        let e = easeInOut(u / 0.34)
        band.position = CGPoint(x: -w + (Double(size.width) + 2 * w) * e, y: Double(size.height) / 2)
        band.transform = CATransform3DMakeRotation(CGFloat((m.angle ?? 20) * .pi / 180), 0, 0, 1)
    }
}

extension UIColor {
    /// #rrggbb or #rrggbbaa
    convenience init?(hex: String) {
        var s = hex.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("#") { s.removeFirst() }
        guard s.count == 6 || s.count == 8, let v = UInt64(s, radix: 16) else { return nil }
        let rgba = s.count == 6 ? (v << 8) | 0xFF : v
        self.init(red: CGFloat((rgba >> 24) & 0xFF) / 255, green: CGFloat((rgba >> 16) & 0xFF) / 255,
                  blue: CGFloat((rgba >> 8) & 0xFF) / 255, alpha: CGFloat(rgba & 0xFF) / 255)
    }
}

/// Breaks the display link → view retain cycle.
private final class DisplayLinkTarget: NSObject {
    weak var view: AtlasCanvas?

    init(_ view: AtlasCanvas) { self.view = view }

    @objc func tick(_ link: CADisplayLink) { view?.tick(link) }
}
