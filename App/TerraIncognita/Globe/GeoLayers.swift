import CoreText
import UIKit

/// Geography a prototype asks the globe to draw (terra.js runtimes 2 and 3, `terra.geo`): borders, areas,
/// dashed outlines and tracks, dots, names, a picture and a line hanging to a depth, in the globe's own
/// inks and at its reading scale, moved with the camera every frame. It lies above the globe's drawing
/// and below its markers and takes no touches; the stage that made it takes it away.
final class GeoLayerView: UIView {
    enum Style: String { case border, area, ghost, track, dot, ring, label, symbol, sounding }

    /// One feature: points, line parts, or polygon rings, each place as its unit vector on the globe
    /// (worked out once, so a frame only turns them).
    struct Shape {
        enum Kind { case points, lines, polygon }
        let kind: Kind
        let parts: [[SIMD3<Double>]]
        let label: String?
        /// A sounding's length, in points.
        let length: Double?
        /// Which of the script's features it came from (a multi-part feature makes several shapes).
        var feature = 0
        /// Dated geography (runtime 6): one date for each place of each part, in the unit of the stage's
        /// date (a place without one is -infinity: always there). A line is drawn as far as the date has
        /// come along it; a point once the date has passed it.
        var dates: [[Double]]? = nil
    }

    /// How a dated line's width grows with its age (runtime 6): from the layer's `lineWidth` where the
    /// date has just passed to `to` reading points once a place is `over` date units old (a vine's stem
    /// thickening behind its tip, a road worn wider).
    struct Thickens: Equatable {
        var to: Double
        var over: Double
        /// How much the width varies along the way (0 to 0.6 of itself), place by place, like bark: a
        /// hash of the place, so it is steady as the camera moves and the date plays.
        var grain = 0.0
    }

    struct Layer {
        let id: String
        let style: Style
        let shapes: [Shape]
        let color: UIColor?
        let minZoom: Double
        let maxZoom: Double
        let font: String
        let size: Double
        let image: UIImage?
        let imageSize: CGSize
        /// Labels and symbols: points from their place (right, down), at the reading scale.
        let offset: CGVector
        /// Its features take touches (terra.js runtime 3).
        var touchable = false
        /// A line's width in reading points, for border, area, ghost and track (nil: the style's own).
        var lineWidth: Double?
        /// Light running along its lines the way their coordinates go (border, ghost and track).
        var flow: Flow?
        /// Its places carry dates (runtime 6): the stage's date says how much of it is drawn.
        var dated = false
        /// A dated border's width growing with its age.
        var thickens: Thickens?
        /// Seconds over which a dated point grows in when the date reaches it (0: it is simply there).
        var bloom = 0.0
        var vertices: Int { shapes.reduce(0) { $0 + $1.parts.reduce(0) { $0 + $1.count } } }
    }

    /// Light running along a layer's lines (terra.js runtime 4, `flow`): beads `dash` reading points
    /// long every `dash + gap`, moving `speed` reading points a second. Core Animation moves them (the
    /// dash phase), so no script and no frame of the globe's is spent on it; Reduce Motion shows none.
    struct Flow: Equatable {
        var speed = 24.0
        var dash = 2.0
        var gap = 22.0
        var color: UIColor?
        var width: Double?
    }

    /// The most vertices all layers together may hold (a continent of simplified borders is about
    /// 7,000), the most layers, and the most named or pictured places in one layer.
    static let vertexBudget = 100_000
    static let layerBudget = 64
    static let placesPerLayer = 400

    private var layers: [Layer] = []
    private var sublayers: [String: CALayer] = [:]
    private var version = 0
    /// What the layers were last drawn for: the globe's camera and where its disc lies (the room it
    /// fills can change with the camera still, as when the foot of the screen changes), the size, and
    /// the parts of the screen kept clear of names.
    private var drawn: (version: Int, camera: GlobeCamera, centre: CGPoint, radius: Double, size: CGSize, clear: [CGRect], date: Double?)?
    /// Name pictures, by what they say and how.
    private var pictures: [String: CGImage] = [:]

    /// The stage's date, for dated geography (runtime 6); nil while no date is shown (dated layers then
    /// draw nothing of what carries a date).
    var clock: DateClock?
    /// When each dated point first showed (by layer id, then "shape-part-point"), for its bloom; a layer
    /// just set has its points already past the date counted as long shown, so a pack set at a late date
    /// doesn't bloom all at once.
    private var seen: [String: [String: CFTimeInterval]] = [:]
    private var fresh: Set<String> = []
    /// Until when something is still blooming, so frames keep coming while the camera and date rest.
    private var bloomingUntil: CFTimeInterval = 0
    /// Where a dated line's tip is at a date: how many places the date has passed, and the tip between the
    /// last of them and the next (nil when the date has not reached the first place).
    private static func reach(_ dates: [Double], at date: Double) -> (count: Int, fraction: Double)? {
        guard let first = dates.first, date >= first else { return nil }
        let n = dates.firstIndex { $0 > date } ?? dates.count
        guard n < dates.count else { return (n, 0) }
        let span = dates[n] - dates[n - 1]
        return (n, span > 0 ? (date - dates[n - 1]) / span : 1)
    }
    /// Between two places on the globe, `t` of the way along the great circle.
    private static func between(_ a: SIMD3<Double>, _ b: SIMD3<Double>, _ t: Double) -> SIMD3<Double> {
        let w = acos(min(1, max(-1, (a * b).sum())))
        guard w > 1e-9 else { return a }
        let sa = sin((1 - t) * w) / sin(w), sb = sin(t * w) / sin(w)
        return a * sa + b * sb
    }
    /// A dated line's part as far as the date has come along it, with its tip, and each place's date
    /// (the tip's is the date itself). A part without dates is whole.
    private static func grown(_ part: [SIMD3<Double>], dates: [Double]?, at date: Double?) -> (points: [SIMD3<Double>], dates: [Double])? {
        guard let dates, dates.count == part.count else { return (part, Array(repeating: -.infinity, count: part.count)) }
        guard let date, let r = reach(dates, at: date) else { return nil }
        var points = Array(part[..<r.count]), when = Array(dates[..<r.count])
        // (the tip, once the date has left the last place passed)
        if r.count < part.count, r.count > 0, r.fraction > 0 {
            points.append(between(part[r.count - 1], part[r.count], min(1, r.fraction)))
            when.append(date)
        }
        return points.count > 0 ? (points, when) : nil
    }
    /// A dated point is there once the date has passed it.
    private static func shown(_ dates: [Double]?, _ i: Int, at date: Double?) -> Bool {
        guard let dates, i < dates.count, dates[i] > -.infinity else { return true }
        guard let date else { return false }
        return dates[i] <= date
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        isUserInteractionEnabled = false
        isAccessibilityElement = false
    }

    /// Reduce Motion as the layers were last drawn (a flow shows no light under it: drawn again when it changes).
    private var stillDrawn = UIAccessibility.isReduceMotionEnabled
    /// The layers whose light runs, by id (their animation is put back if the system took it away).
    private var flowing: [String: CAShapeLayer] = [:]

    required init?(coder: NSCoder) { fatalError("GeoLayerView is made in code") }

    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool { false }

    /// VoiceOver activated a touchable feature: (layer id, feature index, where it is on screen).
    var onActivate: ((String, Int, CGPoint) -> Void)?

    /// A touchable named place as VoiceOver finds it: a button that touches it.
    private final class TouchElement: UIAccessibilityElement {
        var activate: (() -> Void)?
        override func accessibilityActivate() -> Bool { activate?(); return activate != nil }
    }

    // MARK: what the page asks

    /// Adds or replaces a layer (false: over the budget, nothing changed).
    @discardableResult
    func set(_ layer: Layer) -> Bool {
        let others = layers.filter { $0.id != layer.id }
        guard others.reduce(0, { $0 + $1.vertices }) + layer.vertices <= Self.vertexBudget,
              others.count < layers.count || layers.count < Self.layerBudget else { return false }
        if let i = layers.firstIndex(where: { $0.id == layer.id }) {
            // (another style draws with other sublayers, another picture with other contents: start its host afresh)
            if layers[i].style != layer.style || layer.style == .symbol {
                sublayers.removeValue(forKey: layer.id)?.removeFromSuperlayer()
                flowing.removeValue(forKey: layer.id)
            }
            layers[i] = layer
        } else {
            layers.append(layer)
        }
        seen[layer.id] = [:]
        if layer.dated, layer.bloom > 0 { fresh.insert(layer.id) } else { fresh.remove(layer.id) }
        version += 1
        return true
    }

    func remove(_ id: String) {
        layers.removeAll { $0.id == id }
        sublayers.removeValue(forKey: id)?.removeFromSuperlayer()
        flowing.removeValue(forKey: id)
        seen.removeValue(forKey: id)
        fresh.remove(id)
        version += 1
    }

    func clear() {
        layers = []
        for (_, l) in sublayers { l.removeFromSuperlayer() }
        sublayers = [:]
        flowing = [:]
        seen = [:]
        fresh = []
        version += 1
    }

    /// How far a dated point has bloomed (0 to 1) at this moment, recording when it first showed.
    private func bloom(of layer: Layer, key: String, now: CFTimeInterval) -> CGFloat {
        guard layer.dated, layer.bloom > 0 else { return 1 }
        var when = seen[layer.id] ?? [:]
        let since: CFTimeInterval
        if let t = when[key] { since = t } else {
            since = fresh.contains(layer.id) ? now - layer.bloom : now
            when[key] = since
            seen[layer.id] = when
        }
        let f = min(1, max(0, (now - since) / layer.bloom))
        if f < 1 { bloomingUntil = max(bloomingUntil, since + layer.bloom) }
        // (eased: it opens quickly and settles)
        return CGFloat(1 - (1 - f) * (1 - f))
    }

    // MARK: each frame

    /// Draws the layers where the globe's camera now puts them, when either has changed.
    func update(globe: GlobeView) {
        let stage = globe.stageCamera
        let size = bounds.size
        let clear = globe.keepClear
        // (the system may take a layer's animations away, as when the app was in the background)
        for (_, light) in flowing where light.animation(forKey: "flow") == nil { Self.run(light) }
        if UIAccessibility.isReduceMotionEnabled != stillDrawn {
            stillDrawn.toggle()
            version += 1
        }
        let now = CACurrentMediaTime()
        let date = layers.contains(where: \.dated) ? clock?.value(at: now) : nil
        if let d = drawn, d.version == version, d.camera == stage.camera, d.centre == stage.centre, d.radius == stage.radius,
           d.size == size, d.clear == clear, d.date == date, now > bloomingUntil { return }
        drawn = (version, stage.camera, stage.centre, stage.radius, size, clear, date)
        let projection = Projection(camera: stage.camera, centre: stage.centre, radius: stage.radius)
        let reading = CGFloat(GlobeType.shared?.readingScale(shortSide: Double(min(size.width, size.height) > 0 ? min(size.width, size.height) : 400)) ?? 1)
        var placed = clear
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for (order, layer) in layers.enumerated() {
            let host = sublayers[layer.id] ?? {
                let l = CALayer()
                self.layer.addSublayer(l)
                sublayers[layer.id] = l
                return l
            }()
            host.zPosition = CGFloat(order)
            host.frame = bounds
            let shown = stage.camera.zoom >= layer.minZoom && stage.camera.zoom <= layer.maxZoom
            host.isHidden = !shown
            guard shown else { continue }
            draw(layer, in: host, projection: projection, reading: reading, placed: &placed, date: date, now: now)
            fresh.remove(layer.id)
        }
        CATransaction.commit()
        exposeTouchables(projection: projection, zoom: stage.camera.zoom, reading: reading, date: date)
    }

    /// The touchable features that have a name and are on screen, as VoiceOver buttons (so a pack's
    /// touchable places can be reached without seeing or aiming at them).
    private func exposeTouchables(projection p: Projection, zoom: Double, reading k: CGFloat, date: Double?) {
        // (made only while VoiceOver runs: they cost nothing a frame otherwise)
        guard UIAccessibility.isVoiceOverRunning else {
            if accessibilityElements?.isEmpty == false { accessibilityElements = [] }
            return
        }
        var elements: [Any] = []
        for layer in layers where layer.touchable && zoom >= layer.minZoom && zoom <= layer.maxZoom {
            for s in layer.shapes where s.kind == .points {
                guard let name = s.label, !name.isEmpty, let v = s.parts.first?.first, Self.shown(s.dates?.first, 0, at: date),
                      let q = p.point(v, facing: 0.1), bounds.contains(q), elements.count < 200 else { continue }
                let e = TouchElement(accessibilityContainer: self)
                e.accessibilityLabel = name
                e.accessibilityTraits = .button
                e.accessibilityFrameInContainerSpace = CGRect(x: q.x - 22 * k, y: q.y - 22 * k, width: 44 * k, height: 44 * k)
                let id = layer.id, feature = s.feature
                e.activate = { [weak self] in self?.onActivate?(id, feature, q) }
                elements.append(e)
            }
        }
        accessibilityElements = elements
    }

    private func draw(_ layer: Layer, in host: CALayer, projection p: Projection, reading k: CGFloat, placed: inout [CGRect], date: Double?, now: CFTimeInterval) {
        switch layer.style {
        case .border, .area, .ghost, .track:
            let shape = host.sublayers?.first as? CAShapeLayer ?? {
                host.sublayers?.forEach { $0.removeFromSuperlayer() }
                let s = CAShapeLayer()
                host.addSublayer(s)
                return s
            }()
            shape.frame = host.bounds
            let centre = path(of: layer, projection: p, filled: layer.style == .area, date: date)
            shape.path = centre
            shape.lineJoin = .round
            shape.lineCap = .round
            let ink = UIColor(red: 0.231, green: 0.29, blue: 0.329, alpha: 1)
            switch layer.style {
            case .border:
                let colour = (layer.color ?? UIColor(red: 0.549, green: 0.518, blue: 0.471, alpha: 1)).cgColor
                shape.lineDashPattern = nil
                if let thickens = layer.thickens, layer.dated, let date {
                    // a stem: the line's width grows with each place's age, as a filled ribbon round the line
                    shape.path = ribbon(of: layer, projection: p, reading: k, date: date, young: layer.lineWidth ?? 1.1, thickens: thickens)
                    shape.fillColor = colour
                    shape.fillRule = .nonZero
                    shape.strokeColor = nil
                    shape.lineWidth = 0
                } else {
                    shape.fillColor = nil
                    shape.strokeColor = colour
                    shape.lineWidth = CGFloat(layer.lineWidth ?? 1.1) * k
                }
            case .area:
                let base = layer.color ?? UIColor(red: 0.757, green: 0.471, blue: 0.275, alpha: 1)
                shape.fillColor = base.withAlphaComponent(0.28).cgColor
                shape.strokeColor = Self.darker(base).cgColor
                shape.lineWidth = CGFloat(layer.lineWidth ?? 2.2) * k
                shape.lineDashPattern = nil
            case .ghost:
                shape.fillColor = nil
                shape.strokeColor = (layer.color ?? ink).withAlphaComponent(0.8).cgColor
                shape.lineWidth = CGFloat(layer.lineWidth ?? 1.6) * k
                shape.lineDashPattern = [NSNumber(value: Double(5 * k)), NSNumber(value: Double(4 * k))]
            default:
                shape.fillColor = nil
                shape.strokeColor = (layer.color ?? ink).withAlphaComponent(0.47).cgColor
                shape.lineWidth = CGFloat(layer.lineWidth ?? 1.2) * k
                shape.lineDashPattern = [NSNumber(value: Double(3 * k)), NSNumber(value: Double(4 * k))]
            }
            flow(layer, along: centre, in: host, reading: k)
        case .dot, .ring:
            let shape = host.sublayers?.first as? CAShapeLayer ?? {
                let s = CAShapeLayer()
                host.addSublayer(s)
                return s
            }()
            shape.frame = host.bounds
            let dots = CGMutablePath()
            for (si, s) in layer.shapes.enumerated() {
                for (pi, part) in s.parts.enumerated() {
                    for (vi, v) in part.enumerated() where Self.shown(s.dates?[pi], vi, at: date) {
                        guard let q = p.point(v, facing: 0.1) else { continue }
                        let r = 3.4 * k * bloom(of: layer, key: "\(si)-\(pi)-\(vi)", now: now)
                        dots.addEllipse(in: CGRect(x: q.x - r, y: q.y - r, width: 2 * r, height: 2 * r))
                    }
                }
            }
            shape.path = dots
            let ink = layer.color ?? UIColor(red: 0.231, green: 0.29, blue: 0.329, alpha: 1)
            if layer.style == .dot {
                shape.fillColor = ink.cgColor
                shape.strokeColor = nil
            } else {
                shape.fillColor = UIColor(red: 0.965, green: 0.949, blue: 0.906, alpha: 1).cgColor
                shape.strokeColor = ink.cgColor
                shape.lineWidth = 1.3 * k
            }
        case .label:
            host.sublayers?.forEach { $0.removeFromSuperlayer() }
            let reach = bounds.insetBy(dx: -120, dy: -60)
            for s in layer.shapes {
                guard let text = s.label, !text.isEmpty, let v = s.parts.first?.first, Self.shown(s.dates?.first, 0, at: date),
                      let q = p.point(v, facing: 0.3), reach.contains(q), let image = picture(text, layer: layer, reading: k) else { continue }
                let w = CGFloat(image.width) / contentScale, h = CGFloat(image.height) / contentScale
                // at its place, or just above or below it when a name already there would hide it
                let margin = -0.2 * CGFloat(layer.size) * k
                var fit: (frame: CGRect, room: CGRect)?
                for shift: CGFloat in [0, -0.85, 0.85] {
                    let f = CGRect(x: q.x + layer.offset.dx * k - w / 2, y: q.y + layer.offset.dy * k - h / 2 + shift * h, width: w, height: h)
                    let room = f.insetBy(dx: margin, dy: margin)
                    if bounds.contains(f), !placed.contains(where: { $0.intersects(room) }) { fit = (f, room); break }
                }
                guard let fit else { continue }
                placed.append(fit.room)
                let l = CALayer()
                l.contents = image
                l.contentsScale = contentScale
                l.frame = fit.frame
                host.addSublayer(l)
            }
        case .symbol:
            guard let image = layer.image?.cgImage else { host.sublayers?.forEach { $0.removeFromSuperlayer() }; return }
            // (the pictures' layers are kept from frame to frame and moved, not made again: a few hundred
            // leaves cost a few hundred frames set, no allocation)
            let existing = host.sublayers ?? []
            var used = 0
            let w = layer.imageSize.width * k, h = layer.imageSize.height * k
            for (si, s) in layer.shapes.enumerated() {
                for (pi, part) in s.parts.enumerated() {
                    for (vi, v) in part.enumerated() where Self.shown(s.dates?[pi], vi, at: date) {
                        guard let q = p.point(v, facing: 0.2), bounds.insetBy(dx: -w, dy: -h).contains(q) else { continue }
                        let l: CALayer
                        if used < existing.count { l = existing[used] } else {
                            l = CALayer()
                            l.contents = image
                            l.contentsGravity = .resizeAspect
                            host.addSublayer(l)
                        }
                        used += 1
                        let b = bloom(of: layer, key: "\(si)-\(pi)-\(vi)", now: now)
                        // (a blooming picture opens from its foot, the place it grows from)
                        let frame = CGRect(x: q.x + layer.offset.dx * k - w / 2, y: q.y + layer.offset.dy * k - h / 2, width: w, height: h)
                        l.frame = b >= 1 ? frame : CGRect(x: frame.midX - w * b / 2, y: frame.maxY - h * b, width: w * b, height: h * b)
                        l.opacity = Float(min(1, 0.3 + 0.7 * b))
                        placed.append(frame)
                    }
                }
            }
            if used < existing.count { for l in existing[used...] { l.removeFromSuperlayer() } }
        case .sounding:
            host.sublayers?.forEach { $0.removeFromSuperlayer() }
            let blue = UIColor(red: 0.11, green: 0.478, blue: 0.69, alpha: 1)
            for s in layer.shapes {
                guard let v = s.parts.first?.first, Self.shown(s.dates?.first, 0, at: date), let q = p.point(v, facing: 0.2),
                      bounds.insetBy(dx: -40, dy: -40).contains(q) else { continue }
                let length = CGFloat(min(max(0, s.length ?? 0), 2000)) * k
                let line = CAShapeLayer()
                let path = CGMutablePath()
                path.move(to: q)
                path.addLine(to: CGPoint(x: q.x, y: q.y + length))
                line.path = path
                line.strokeColor = blue.withAlphaComponent(0.85).cgColor
                line.lineWidth = 1.4 * k
                line.lineDashPattern = [NSNumber(value: Double(2 * k)), NSNumber(value: Double(3 * k))]
                host.addSublayer(line)
                let foot = CAShapeLayer()
                let f = CGMutablePath()
                f.move(to: CGPoint(x: q.x - 8 * k, y: q.y + length))
                f.addLine(to: CGPoint(x: q.x + 8 * k, y: q.y + length))
                foot.path = f
                foot.strokeColor = blue.cgColor
                foot.lineWidth = 2 * k
                foot.lineCap = .round
                host.addSublayer(foot)
                if let text = s.label, !text.isEmpty,
                   let image = picture(text, layer: layer, reading: k, ink: UIColor(red: 0.11, green: 0.36, blue: 0.55, alpha: 1), size: 14, font: "regular") {
                    let w = CGFloat(image.width) / contentScale, h = CGFloat(image.height) / contentScale
                    let l = CALayer()
                    l.contents = image
                    l.contentsScale = contentScale
                    l.frame = CGRect(x: q.x + 10 * k, y: q.y + length - h / 2, width: w, height: h)
                    host.addSublayer(l)
                    placed.append(l.frame)
                }
            }
        }
    }

    /// The light running along a line layer, over its line (a second shape with the same path, dashed,
    /// its dash phase moved by Core Animation), or none: no `flow`, an area, or Reduce Motion.
    private func flow(_ layer: Layer, along centre: CGPath, in host: CALayer, reading k: CGFloat) {
        let existing = host.sublayers?.dropFirst().first as? CAShapeLayer
        guard let flow = layer.flow, layer.style != .area, !UIAccessibility.isReduceMotionEnabled else {
            existing?.removeFromSuperlayer()
            flowing.removeValue(forKey: layer.id)
            return
        }
        let light = existing ?? {
            let l = CAShapeLayer()
            l.fillColor = nil
            l.lineCap = .round
            l.lineJoin = .round
            host.addSublayer(l)
            return l
        }()
        light.frame = host.bounds
        light.path = centre
        light.strokeColor = (flow.color ?? UIColor(white: 1, alpha: 0.9)).cgColor
        light.lineWidth = CGFloat(flow.width ?? layer.lineWidth ?? 1.1) * k
        let dash = CGFloat(flow.dash) * k, period = CGFloat(flow.dash + flow.gap) * k
        let seconds = Double(period) / max(1, flow.speed * Double(k))
        light.lineDashPattern = [NSNumber(value: Double(dash)), NSNumber(value: Double(period - dash))]
        flowing[layer.id] = light
        // (started again only when its measure changes, so the beads don't jump as the camera moves)
        if (light.value(forKey: "flowPeriod") as? CGFloat) != period || (light.value(forKey: "flowSeconds") as? Double) != seconds
            || light.animation(forKey: "flow") == nil {
            light.setValue(period, forKey: "flowPeriod")
            light.setValue(seconds, forKey: "flowSeconds")
            Self.run(light)
        }
    }

    /// The beads moving the way a line's coordinates go: the dash phase falling through one period, again and again.
    private static func run(_ light: CAShapeLayer) {
        guard let period = light.value(forKey: "flowPeriod") as? CGFloat, let seconds = light.value(forKey: "flowSeconds") as? Double else { return }
        let a = CABasicAnimation(keyPath: "lineDashPhase")
        a.fromValue = period
        a.toValue = 0
        a.duration = seconds
        a.repeatCount = .infinity
        a.isRemovedOnCompletion = false
        light.add(a, forKey: "flow")
    }

    /// Lines break where they go behind the globe; an area's corners behind it are drawn on its edge. A
    /// dated line is drawn as far as the date has come along it.
    private func path(of layer: Layer, projection p: Projection, filled: Bool, date: Double?) -> CGPath {
        let path = CGMutablePath()
        for s in layer.shapes {
            for (pi, whole) in s.parts.enumerated() where whole.count > 1 {
                if s.kind == .polygon {
                    let points = whole.map { p.clamped($0) }
                    guard points.contains(where: { $0.facing > 0 }) else { continue }
                    path.move(to: points[0].point)
                    for q in points.dropFirst() { path.addLine(to: q.point) }
                    path.closeSubpath()
                } else {
                    guard let part = layer.dated ? Self.grown(whole, dates: s.dates?[pi], at: date)?.points : whole else { continue }
                    var pen = false
                    for v in part {
                        if let q = p.point(v, facing: 0) {
                            if pen { path.addLine(to: q) } else { path.move(to: q) }
                            pen = true
                        } else {
                            pen = false
                        }
                    }
                }
            }
        }
        return path
    }

    /// A dated border as a ribbon: each place's half-width from its age (young at the tip, `thickens.to`
    /// once `thickens.over` old), the ribbon's edges offset along the line's normals, round at its ends.
    /// Where a line goes behind the globe the ribbon breaks, as the line does.
    private func ribbon(of layer: Layer, projection p: Projection, reading k: CGFloat, date: Double, young: Double, thickens: Thickens) -> CGPath {
        let path = CGMutablePath()
        func lay(_ run: [(CGPoint, CGFloat)]) {
            guard run.count > 1 else {
                if let only = run.first { path.addEllipse(in: CGRect(x: only.0.x - only.1, y: only.0.y - only.1, width: 2 * only.1, height: 2 * only.1)) }
                return
            }
            var left: [CGPoint] = [], right: [CGPoint] = []
            for i in run.indices {
                let (q, w) = run[i]
                let a = run[max(0, i - 1)].0, b = run[min(run.count - 1, i + 1)].0
                var dx = b.x - a.x, dy = b.y - a.y
                let d = hypot(dx, dy)
                if d < 1e-6 { dx = 1; dy = 0 } else { dx /= d; dy /= d }
                left.append(CGPoint(x: q.x - dy * w, y: q.y + dx * w))
                right.append(CGPoint(x: q.x + dy * w, y: q.y - dx * w))
            }
            // (the round ends are laid point by point, from the left edge round to the right, so the ribbon
            // is one simple outline whichever way the screen's angles turn)
            func cap(_ centre: CGPoint, _ radius: CGFloat, from start: CGFloat) {
                for i in 1...8 {
                    let a = start - CGFloat(i) / 8 * .pi
                    path.addLine(to: CGPoint(x: centre.x + cos(a) * radius, y: centre.y + sin(a) * radius))
                }
            }
            path.move(to: left[0])
            for q in left.dropFirst() { path.addLine(to: q) }
            let (tip, tw) = run[run.count - 1]
            let (prev, _) = run[run.count - 2]
            cap(tip, tw, from: atan2(tip.y - prev.y, tip.x - prev.x) + .pi / 2)
            for q in right.reversed().dropFirst() { path.addLine(to: q) }
            let (root, rw) = run[0]
            let (next, _) = run[1]
            cap(root, rw, from: atan2(next.y - root.y, next.x - root.x) - .pi / 2)
            path.closeSubpath()
        }
        for s in layer.shapes where s.kind == .lines {
            for (pi, whole) in s.parts.enumerated() where whole.count > 1 {
                guard let grown = Self.grown(whole, dates: s.dates?[pi], at: date) else { continue }
                var run: [(CGPoint, CGFloat)] = []
                for (i, (v, when)) in zip(grown.points, grown.dates).enumerated() {
                    if let q = p.point(v, facing: 0) {
                        let age = when.isFinite ? min(1, max(0, (date - when) / max(1e-9, thickens.over))) : 1
                        // (the grain: a steady pseudo-random swell at each place along the way)
                        let swell = thickens.grain > 0 ? 1 + thickens.grain * sin(Double(i) * 2.399 + v.x * 37 + v.z * 23) : 1
                        // (the ribbon's edges are each half the width from the line)
                        run.append((q, CGFloat((young + (thickens.to - young) * age) * swell) * k / 2))
                    } else {
                        lay(run)
                        run = []
                    }
                }
                lay(run)
            }
        }
        return path
    }

    private var contentScale: CGFloat { max(1, traitCollection.displayScale) }

    /// A name's picture, its letters over a round-joined halo of the paper's tone (as the globe letters
    /// its own names).
    private func picture(_ text: String, layer: Layer, reading k: CGFloat, ink: UIColor? = nil, size: Double? = nil, font: String? = nil) -> CGImage? {
        let face = font ?? layer.font
        let points = CGFloat(size ?? layer.size) * k
        let key = "\(face)|\(points)|\(text)|\(ink?.description ?? layer.color?.description ?? "")"
        if let cached = pictures[key] { return cached }
        let name: String
        switch face {
        case "italic": name = "LibertinusSerif-Italic"
        case "caps": name = "LibertinusSerif-Semibold"
        default: name = "LibertinusSerif-Regular"
        }
        GlobeData.registerFonts()
        let ctFont = CTFontCreateWithName(name as CFString, points, nil)
        let shown = face == "caps" ? text.uppercased() : text
        let colour = ink ?? layer.color ?? UIColor(red: 0.231, green: 0.29, blue: 0.329, alpha: 1)
        var attributes: [NSAttributedString.Key: Any] = [
            NSAttributedString.Key(kCTFontAttributeName as String): ctFont,
            NSAttributedString.Key(kCTForegroundColorAttributeName as String): colour.cgColor,
        ]
        if face == "caps" { attributes[NSAttributedString.Key(kCTKernAttributeName as String)] = points * 0.08 }
        let text = NSAttributedString(string: shown, attributes: attributes)
        let tone = UIColor(red: 0.965, green: 0.949, blue: 0.906, alpha: 0.92)
        let toned = NSMutableAttributedString(attributedString: text)
        toned.addAttributes([NSAttributedString.Key(kCTForegroundColorAttributeName as String): tone.cgColor,
                             NSAttributedString.Key(kCTStrokeColorAttributeName as String): tone.cgColor],
                            range: NSRange(location: 0, length: toned.length))
        let line = CTLineCreateWithAttributedString(text)
        let haloLine = CTLineCreateWithAttributedString(toned)
        var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
        let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
        let halo = CGFloat(GlobeType.shared?.halo ?? 1.5) * k * 2
        let canvas = CGSize(width: ceil(width + 2 * halo + 2), height: ceil(ascent + descent + 2 * halo + 2))
        let format = UIGraphicsImageRendererFormat()
        format.scale = contentScale
        format.opaque = false
        let image = UIGraphicsImageRenderer(size: canvas, format: format).image { context in
            let c = context.cgContext
            c.translateBy(x: 0, y: canvas.height)
            c.scaleBy(x: 1, y: -1)
            c.textMatrix = .identity
            let origin = CGPoint(x: (canvas.width - width) / 2, y: (canvas.height - ascent - descent) / 2 + descent)
            c.setLineJoin(.round)
            c.setLineWidth(halo * 2)
            c.setStrokeColor(tone.cgColor)
            c.setTextDrawingMode(.fillStroke)
            c.textPosition = origin
            CTLineDraw(haloLine, c)
            c.setTextDrawingMode(.fill)
            c.textPosition = origin
            CTLineDraw(line, c)
        }.cgImage
        if pictures.count > 600 { pictures.removeAll() }
        pictures[key] = image
        return image
    }

    private static func darker(_ colour: UIColor) -> UIColor {
        var h: CGFloat = 0, s: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        guard colour.getHue(&h, saturation: &s, brightness: &b, alpha: &a) else { return colour }
        return UIColor(hue: h, saturation: min(1, s * 1.15), brightness: b * 0.7, alpha: 1)
    }

    // MARK: touch (runtime 3)

    /// The touchable feature under a point of the screen, from the topmost layer down: points and
    /// names within 22 reading points, lines within 14, areas inside. Nil when nothing touchable is there.
    func hit(_ point: CGPoint, globe: GlobeView) -> (layer: String, index: Int)? {
        let stage = globe.stageCamera
        let p = Projection(camera: stage.camera, centre: stage.centre, radius: stage.radius)
        let size = bounds.size
        let k = CGFloat(GlobeType.shared?.readingScale(shortSide: Double(min(size.width, size.height) > 0 ? min(size.width, size.height) : 400)) ?? 1)
        let zoom = stage.camera.zoom
        let date = layers.contains(where: \.dated) ? clock?.value(at: CACurrentMediaTime()) : nil
        for layer in layers.reversed() where layer.touchable && zoom >= layer.minZoom && zoom <= layer.maxZoom {
            var best: (index: Int, distance: CGFloat)?
            func consider(_ index: Int, _ d: CGFloat, within reach: CGFloat) {
                if d <= reach, d < best?.distance ?? .infinity { best = (index, d) }
            }
            for s in layer.shapes {
                switch s.kind {
                case .points:
                    for (pi, part) in s.parts.enumerated() {
                        for (vi, v) in part.enumerated() where Self.shown(s.dates?[pi], vi, at: date) {
                            guard let q = p.point(v, facing: 0.1) else { continue }
                            var c = q
                            if layer.style == .label || layer.style == .symbol {
                                c = CGPoint(x: q.x + layer.offset.dx * k, y: q.y + layer.offset.dy * k)
                            }
                            consider(s.feature, hypot(point.x - c.x, point.y - c.y), within: 22 * k)
                        }
                    }
                case .lines:
                    for (pi, whole) in s.parts.enumerated() where whole.count > 1 {
                        guard let part = layer.dated ? Self.grown(whole, dates: s.dates?[pi], at: date)?.points : whole else { continue }
                        var previous: CGPoint?
                        for v in part {
                            let q = p.point(v, facing: 0)
                            if let a = previous, let b = q { consider(s.feature, Self.distance(point, a, b), within: 14 * k) }
                            previous = q
                        }
                    }
                case .polygon:
                    guard let ring = s.parts.first, ring.count > 2 else { continue }
                    let points = ring.map { p.clamped($0) }
                    guard points.contains(where: { $0.facing > 0 }) else { continue }
                    let path = CGMutablePath()
                    path.addLines(between: points.map(\.point))
                    path.closeSubpath()
                    if path.contains(point, using: .evenOdd) { consider(s.feature, 0, within: 1) }
                }
            }
            if let best { return (layer.id, best.index) }
        }
        return nil
    }

    private static func distance(_ p: CGPoint, _ a: CGPoint, _ b: CGPoint) -> CGFloat {
        let dx = b.x - a.x, dy = b.y - a.y
        let length = dx * dx + dy * dy
        let t = length > 0 ? max(0, min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length)) : 0
        return hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy))
    }

    // MARK: projection

    /// The globe's orthographic projection, from its stage camera (the same numbers terra.js uses).
    struct Projection {
        let ax, bx, ay, by: Double
        let centre: CGPoint
        let radius: Double

        init(camera: GlobeCamera, centre: CGPoint, radius: Double) {
            let a = camera.lon * .pi / 180, b = -camera.lat * .pi / 180
            ax = cos(a); bx = sin(a); ay = cos(b); by = sin(b)
            self.centre = centre
            self.radius = radius
        }

        /// A place's unit vector on the globe, from degrees.
        static func world(lon: Double, lat: Double) -> SIMD3<Double> {
            let la = lat * .pi / 180, lo = lon * .pi / 180
            return SIMD3(cos(la) * sin(lo), sin(la), cos(la) * cos(lo))
        }

        private func view(_ w: SIMD3<Double>) -> SIMD3<Double> {
            let wx = w.x, wy = w.y, wz = w.z
            return SIMD3(ax * wx - bx * wz,
                         bx * by * wx + ay * wy + ax * by * wz,
                         bx * ay * wx - by * wy + ax * ay * wz)
        }

        /// On screen, when it faces the reader by more than `facing`.
        func point(_ v: SIMD3<Double>, facing: Double) -> CGPoint? {
            let w = view(v)
            guard w.z > facing else { return nil }
            return CGPoint(x: Double(centre.x) + w.x * radius, y: Double(centre.y) - w.y * radius)
        }

        /// On screen, or on the globe's edge when behind it.
        func clamped(_ v: SIMD3<Double>) -> (point: CGPoint, facing: Double) {
            let w = view(v)
            var x = w.x, y = w.y
            if w.z <= 0 {
                let r = max(1e-9, (x * x + y * y).squareRoot())
                x /= r; y /= r
            }
            return (CGPoint(x: Double(centre.x) + x * radius, y: Double(centre.y) - y * radius), w.z)
        }
    }

    // MARK: from the page

    /// A layer from `terra.geo.set(id, features, options)`; nil if nothing in it can be drawn.
    /// (`touchable` makes its features take touches; each shape remembers its feature.)
    static func layer(id: String, features: Any?, options: [String: Any], packRoot: URL) -> Layer? {
        guard let style = Style(rawValue: (options["style"] as? String) ?? "border") else { return nil }
        var shapes: [Shape] = []
        func number(_ x: Any?) -> Double? { (x as? NSNumber).map(\.doubleValue).flatMap { $0.isFinite ? $0 : nil } }
        var budget = vertexBudget
        // dated geography (runtime 6): each position's third number is its date
        let dated = (options["dated"] as? Bool) ?? false
        func position(_ x: Any?) -> (SIMD3<Double>, Double)? {
            guard budget > 0, let a = x as? [Any], a.count >= 2, let lon = number(a[0]), let lat = number(a[1]),
                  abs(lat) <= 90, abs(lon) <= 540 else { return nil }
            budget -= 1
            return (Projection.world(lon: lon, lat: lat), a.count > 2 ? (number(a[2]) ?? -.infinity) : -.infinity)
        }
        func ring(_ x: Any?) -> [(SIMD3<Double>, Double)] { ((x as? [Any]) ?? []).compactMap(position) }
        var feature = 0
        func shape(_ kind: Shape.Kind, _ parts: [[(SIMD3<Double>, Double)]], label: String?, length: Double?) -> Shape {
            var s = Shape(kind: kind, parts: parts.map { part in part.map { $0.0 } }, label: label, length: length)
            if dated { s.dates = parts.map { part in part.map { $0.1 } } }
            return s
        }
        func add(geometry: [String: Any], properties: [String: Any], depth: Int = 0) {
            guard depth < 4, budget > 0 else { return }
            let first = shapes.count
            defer { for i in first..<shapes.count { shapes[i].feature = feature } }
            let label = (properties["label"] as? String).map { String($0.prefix(80)) }
            let length = number(properties["length"]).map { min(max($0, 0), 2000) }
            let c = geometry["coordinates"]
            switch geometry["type"] as? String {
            case "Point": if let v = position(c) { shapes.append(shape(.points, [[v]], label: label, length: length)) }
            case "MultiPoint": shapes.append(shape(.points, [ring(c)], label: label, length: length))
            case "LineString": shapes.append(shape(.lines, [ring(c)], label: label, length: length))
            case "MultiLineString": shapes.append(shape(.lines, ((c as? [Any]) ?? []).map(ring), label: label, length: length))
            case "Polygon": shapes.append(shape(.polygon, ((c as? [Any]) ?? []).map(ring), label: label, length: length))
            case "MultiPolygon":
                for polygon in (c as? [Any]) ?? [] {
                    shapes.append(shape(.polygon, ((polygon as? [Any]) ?? []).map(ring), label: label, length: length))
                }
            case "GeometryCollection":
                for g in (geometry["geometries"] as? [[String: Any]]) ?? [] { add(geometry: g, properties: properties, depth: depth + 1) }
            default: break
            }
        }
        var items = (features as? [Any]) ?? []
        if let collection = features as? [String: Any] {
            items = (collection["features"] as? [Any]) ?? [collection]
        }
        // (a named or pictured place costs a layer of its own on screen: a few hundred at most)
        let places = [Style.label, .symbol, .sounding].contains(style) ? placesPerLayer : Int.max
        for (n, item) in items.prefix(50_000).enumerated() {
            feature = n
            guard budget > 0, shapes.count < places, let object = item as? [String: Any] else { continue }
            if (object["type"] as? String) == "Feature" {
                if let g = object["geometry"] as? [String: Any] { add(geometry: g, properties: (object["properties"] as? [String: Any]) ?? [:]) }
            } else {
                add(geometry: object, properties: (object["properties"] as? [String: Any]) ?? [:])
            }
        }
        var image: UIImage?
        if style == .symbol, let name = options["image"] as? String, !name.contains(".."), !name.hasPrefix("/") {
            let file = packRoot.appending(path: name).standardizedFileURL
            if file.path(percentEncoded: false).hasPrefix(packRoot.standardizedFileURL.path(percentEncoded: false)) {
                image = UIImage(contentsOfFile: file.path(percentEncoded: false))
            }
        }
        let width = min(200, max(4, number(options["width"]) ?? 28)), height = min(200, max(4, number(options["height"]) ?? 28))
        let shift = (options["offset"] as? [Any]).flatMap { a in a.count == 2 ? number(a[0]).flatMap { x in number(a[1]).map { CGVector(dx: min(200, max(-200, x)), dy: min(200, max(-200, $0))) } } : nil } ?? .zero
        return Layer(id: id, style: style, shapes: shapes, color: (options["color"] as? String).flatMap { UIColor(hex: $0) },
                     minZoom: number(options["minZoom"]) ?? 0, maxZoom: number(options["maxZoom"]) ?? .infinity,
                     font: (options["font"] as? String) ?? "regular", size: min(40, max(10, number(options["size"]) ?? 15)),
                     image: image, imageSize: CGSize(width: width, height: height), offset: shift,
                     touchable: (options["touchable"] as? Bool) ?? false,
                     lineWidth: number(options["lineWidth"]).map { min(12, max(0.3, $0)) },
                     flow: flowOption(options["flow"]),
                     dated: dated,
                     thickens: dated ? thickensOption(options["thickens"]) : nil,
                     bloom: dated ? min(3, max(0, number(options["bloom"]) ?? 0.6)) : 0)
    }

    /// A dated layer's `thickens`: { to, over } (a width in reading points, an age in the date's unit); nil without one.
    private static func thickensOption(_ value: Any?) -> Thickens? {
        func number(_ x: Any?) -> Double? { (x as? NSNumber).map(\.doubleValue).flatMap { $0.isFinite ? $0 : nil } }
        guard let o = value as? [String: Any], let to = number(o["to"]), let over = number(o["over"]), over > 0 else { return nil }
        return Thickens(to: min(12, max(0.3, to)), over: over, grain: min(0.6, max(0, number(o["grain"]) ?? 0)))
    }

    /// A layer's `flow`: true (the defaults) or { speed, dash, gap, color, width }; nil without one.
    private static func flowOption(_ value: Any?) -> Flow? {
        func number(_ x: Any?) -> Double? { (x as? NSNumber).map(\.doubleValue).flatMap { $0.isFinite ? $0 : nil } }
        guard let o = value as? [String: Any] else { return (value as? Bool) == true ? Flow() : nil }
        var f = Flow()
        if let v = number(o["speed"]) { f.speed = min(400, max(1, v)) }
        if let v = number(o["dash"]) { f.dash = min(60, max(0, v)) }
        if let v = number(o["gap"]) { f.gap = min(400, max(2, v)) }
        f.color = (o["color"] as? String).flatMap { UIColor(hex: $0) }
        f.width = number(o["width"]).map { min(12, max(0.3, $0)) }
        return f
    }
}
