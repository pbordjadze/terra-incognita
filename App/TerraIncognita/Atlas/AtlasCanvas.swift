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

    private(set) var mode: Mode = .film { didSet { if mode != oldValue { onMode?(mode) } } }
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
        isMultipleTouchEnabled = true
        let pan = UIPanGestureRecognizer(target: self, action: #selector(pan(_:)))
        pan.maximumNumberOfTouches = 2
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(pinch(_:)))
        for g in [pan, pinch] as [UIGestureRecognizer] { g.delegate = self; addGestureRecognizer(g) }
        isAccessibilityElement = true
        accessibilityLabel = String(localized: "A map zooming from the whole world down to a neighbourhood")
        accessibilityTraits = .image
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
        film = Film(new)
        filmTime = min(filmTime, film?.duration ?? 0)
        if let old {
            // the same scale (looks may pass through different plates on the way), the same spot
            var c = camera
            while c.plate > 0, old.plates[c.plate].passes { c = old.up(c) }
            let level = old.plates[...c.plate].filter { !$0.passes }.count - 1
            let stops = new.plates.indices.filter { !new.plates[$0].passes }
            let k = stops[min(level, stops.count - 1)], from = old.plates[c.plate]
            camera = new.normalized(Camera(plate: k, x: c.x / from.width * new.plates[k].width, y: c.y / from.height * new.plates[k].height,
                                           zoom: c.zoom / old.restZoom(c.plate) * new.restZoom(k)))
        } else {
            camera = new.rest(0)
        }
        loadAround(camera.plate, now: true)
        render()
    }

    // MARK: the film and the reader's hands

    func resume() {
        guard let film, let ladder else { return }
        if mode == .ended { filmTime = 0 }
        let target = film.camera(at: filmTime)
        let common = min(camera.plate, target.plate)
        returnFrom = ladder.express(camera, in: common)
        returnStart = CACurrentMediaTime()
        fling = .zero
        mode = .returning
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
            let f = g.location(in: self)
            let fx = Double(f.x - bounds.midX), fy = Double(f.y - bounds.midY)
            // the plate's point under the fingers stays under them
            let px = camera.x + fx / camera.zoom, py = camera.y + fy / camera.zoom
            camera.zoom *= Double(g.scale)
            g.scale = 1
            camera.x = px - fx / camera.zoom
            camera.y = py - fy / camera.zoom
            settle()
        case .ended, .cancelled, .failed: touching = max(0, touching - 1)
        default: break
        }
    }

    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool { true }

    private func settle() {
        guard let ladder else { return }
        camera = ladder.clamped(ladder.normalized(camera))
        render()
    }

    // MARK: each frame

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, displayLink == nil {
            let link = CADisplayLink(target: DisplayLinkTarget(self), selector: #selector(DisplayLinkTarget.tick(_:)))
            link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
            link.add(to: .main, forMode: .common)
            displayLink = link
        } else if window == nil {
            displayLink?.invalidate()
            displayLink = nil
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        if let pending, bounds.width > 0 { show(pending) }
        guard let ladder, ladder.screen != bounds.size, bounds.width > 0 else { return }
        // a new size (rotation, a new window): the same view, measured against the new screen
        let rest = ladder.restZoom(camera.plate)
        let resized = Ladder(plates: ladder.plates, screen: bounds.size)
        camera.zoom *= resized.restZoom(camera.plate) / rest
        self.ladder = resized
        film = Film(resized)
        render()
    }

    fileprivate func tick(_ link: CADisplayLink) {
        let dt = min(link.targetTimestamp - (lastTick ?? link.timestamp), 0.05)
        lastTick = link.targetTimestamp
        guard let ladder, let film else { return }
        switch mode {
        case .film:
            filmTime += dt
            #if DEBUG
            if let frozenAt { filmTime = frozenAt }
            #endif
            if filmTime >= film.duration { filmTime = film.duration; mode = .ended }
            camera = ladder.normalized(film.camera(at: filmTime))
        case .returning:
            let u = (CACurrentMediaTime() - returnStart) / 1.4
            let target = film.camera(at: filmTime)
            let to = ladder.express(target, in: returnFrom.plate)
            camera = ladder.normalized(glide(returnFrom, to, easeInOut(u)))
            if u >= 1 { mode = .film }
        case .free:
            if touching == 0, abs(fling.dx) + abs(fling.dy) > 4 {
                camera.x -= Double(fling.dx) * dt / camera.zoom
                camera.y -= Double(fling.dy) * dt / camera.zoom
                let decay = exp(-4.5 * dt)
                fling = CGVector(dx: fling.dx * decay, dy: fling.dy * decay)
                camera = ladder.clamped(ladder.normalized(camera))
            }
        case .ended:
            break
        }
        let shown = mode == .film ? film.caption(at: filmTime).map { ladder.plates[$0].caption } : nil
        if shown != caption { caption = shown; onCaption?(shown) }
        render()
    }

    private func render() {
        guard let ladder, let look else { return }
        let b = camera.plate
        let current = ladder.opacity(b, zoom: camera.zoom)
        let covers = camera.zoom >= ladder.coverZoom(b) * 0.999
        var visible: [(Int, Double)] = []
        if b > 0, current < 1 || !covers { visible.append((b - 1, 1)) }
        visible.append((b, visible.isEmpty ? 1 : current))
        if b + 1 < ladder.count {
            let child = ladder.down(camera)
            let o = ladder.opacity(b + 1, zoom: child.zoom)
            if o > 0.001 { visible.append((b + 1, o)) }
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        let shownIDs = Set(visible.map(\.0))
        for (k, node) in nodes where !shownIDs.contains(k) { node.container.isHidden = true }
        for (k, opacity) in visible {
            let node = self.node(k, look: look, ladder: ladder)
            let c = ladder.express(camera, in: k)
            let p = ladder.plates[k]
            node.container.isHidden = false
            node.container.opacity = Float(opacity)
            node.container.zPosition = CGFloat(k)
            node.container.position = CGPoint(x: Double(bounds.midX) - c.x * c.zoom, y: Double(bounds.midY) - c.y * c.zoom)
            node.container.transform = CATransform3DMakeScale(CGFloat(c.zoom), CGFloat(c.zoom), 1)
            // nearer layers move further with the camera, in proportion to its offset from the centre,
            // measured at rest so a dive doesn't fling them apart
            let s = ladder.restZoom(k) / c.zoom
            let ox = -(c.x - p.width / 2) * s, oy = -(c.y - p.height / 2) * s
            for (layer, spec) in zip(node.layers, p.layers) where spec.depth != 0 {
                layer.frame = CGRect(x: ox * spec.depth, y: oy * spec.depth, width: p.width, height: p.height)
            }
        }
        CATransaction.commit()
        loadAround(b, now: false)
    }

    // MARK: the pictures

    private func node(_ k: Int, look: Look, ladder: Ladder) -> PlateNode {
        if let n = nodes[k] { return n }
        let p = ladder.plates[k]
        let container = CALayer()
        container.isHidden = true
        container.anchorPoint = .zero
        container.bounds = CGRect(x: 0, y: 0, width: p.width, height: p.height)
        let layers = p.layers.map { _ in
            let l = CALayer()
            l.frame = container.bounds
            l.contentsGravity = .resize
            l.minificationFilter = .trilinear
            l.magnificationFilter = .linear
            container.addSublayer(l)
            return l
        }
        layer.addSublayer(container)
        let n = PlateNode(container: container, layers: layers)
        nodes[k] = n
        return n
    }

    /// Plates b - 1 to b + 2 have their pictures; the rest let theirs go. The camera's own plate is
    /// decoded at once the first time (nothing would show otherwise), the others in the background.
    private func loadAround(_ b: Int, now: Bool) {
        guard let ladder, let look else { return }
        let keep = max(0, b - 1)...min(ladder.count - 1, b + 2)
        for (k, n) in nodes where !keep.contains(k) && n.loaded { n.unload() }
        for k in keep {
            let n = node(k, look: look, ladder: ladder)
            guard !n.loaded, !n.loading else { continue }
            n.loading = true
            let urls = ladder.plates[k].layers.map { Atlas.url(of: $0, in: look) }
            if now && k == b {
                for (l, url) in zip(n.layers, urls) { l.contents = UIImage(contentsOfFile: url.path)?.preparingForDisplay()?.cgImage }
                n.loaded = true
                n.loading = false
            } else {
                Task { [weak self] in
                    var images: [UIImage?] = []
                    for url in urls { images.append(await UIImage(contentsOfFile: url.path)?.byPreparingForDisplay()) }
                    // the look may have changed while these were decoding
                    guard let self, self.nodes[k] === n else { return }
                    for (l, image) in zip(n.layers, images) { l.contents = image?.cgImage }
                    n.loaded = true
                    n.loading = false
                }
            }
        }
    }
}

private final class PlateNode {
    let container: CALayer
    let layers: [CALayer]
    var loaded = false
    var loading = false

    init(container: CALayer, layers: [CALayer]) {
        self.container = container
        self.layers = layers
    }

    func unload() {
        for l in layers { l.contents = nil }
        loaded = false
    }
}

/// Breaks the display link → view retain cycle.
private final class DisplayLinkTarget: NSObject {
    weak var view: AtlasCanvas?

    init(_ view: AtlasCanvas) { self.view = view }

    @objc func tick(_ link: CADisplayLink) { view?.tick(link) }
}
