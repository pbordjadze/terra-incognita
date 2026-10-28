import CoreText
import UIKit

/// Geography a prototype asks the globe to draw (terra.js runtime 2, `terra.geo`): borders, areas,
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
        var vertices: Int { shapes.reduce(0) { $0 + $1.parts.reduce(0) { $0 + $1.count } } }
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
    private var drawn: (version: Int, camera: GlobeCamera, centre: CGPoint, radius: Double, size: CGSize, clear: [CGRect])?
    /// Name pictures, by what they say and how.
    private var pictures: [String: CGImage] = [:]

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        isUserInteractionEnabled = false
        isAccessibilityElement = false
    }

    required init?(coder: NSCoder) { fatalError("GeoLayerView is made in code") }

    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool { false }

    // MARK: what the page asks

    /// Adds or replaces a layer (false: over the budget, nothing changed).
    @discardableResult
    func set(_ layer: Layer) -> Bool {
        let others = layers.filter { $0.id != layer.id }
        guard others.reduce(0, { $0 + $1.vertices }) + layer.vertices <= Self.vertexBudget,
              others.count < layers.count || layers.count < Self.layerBudget else { return false }
        if let i = layers.firstIndex(where: { $0.id == layer.id }) {
            // (another style draws with other sublayers: start its host afresh)
            if layers[i].style != layer.style { sublayers.removeValue(forKey: layer.id)?.removeFromSuperlayer() }
            layers[i] = layer
        } else {
            layers.append(layer)
        }
        version += 1
        return true
    }

    func remove(_ id: String) {
        layers.removeAll { $0.id == id }
        sublayers.removeValue(forKey: id)?.removeFromSuperlayer()
        version += 1
    }

    func clear() {
        layers = []
        for (_, l) in sublayers { l.removeFromSuperlayer() }
        sublayers = [:]
        version += 1
    }

    // MARK: each frame

    /// Draws the layers where the globe's camera now puts them, when either has changed.
    func update(globe: GlobeView) {
        let stage = globe.stageCamera
        let size = bounds.size
        let clear = globe.keepClear
        if let d = drawn, d.version == version, d.camera == stage.camera, d.centre == stage.centre, d.radius == stage.radius,
           d.size == size, d.clear == clear { return }
        drawn = (version, stage.camera, stage.centre, stage.radius, size, clear)
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
            draw(layer, in: host, projection: projection, reading: reading, placed: &placed)
        }
        CATransaction.commit()
    }

    private func draw(_ layer: Layer, in host: CALayer, projection p: Projection, reading k: CGFloat, placed: inout [CGRect]) {
        switch layer.style {
        case .border, .area, .ghost, .track:
            let shape = host.sublayers?.first as? CAShapeLayer ?? {
                host.sublayers?.forEach { $0.removeFromSuperlayer() }
                let s = CAShapeLayer()
                host.addSublayer(s)
                return s
            }()
            shape.frame = host.bounds
            shape.path = path(of: layer, projection: p, filled: layer.style == .area)
            shape.lineJoin = .round
            shape.lineCap = .round
            let ink = UIColor(red: 0.231, green: 0.29, blue: 0.329, alpha: 1)
            switch layer.style {
            case .border:
                shape.fillColor = nil
                shape.strokeColor = (layer.color ?? UIColor(red: 0.549, green: 0.518, blue: 0.471, alpha: 1)).cgColor
                shape.lineWidth = 1.1 * k
                shape.lineDashPattern = nil
            case .area:
                let base = layer.color ?? UIColor(red: 0.757, green: 0.471, blue: 0.275, alpha: 1)
                shape.fillColor = base.withAlphaComponent(0.28).cgColor
                shape.strokeColor = Self.darker(base).cgColor
                shape.lineWidth = 2.2 * k
                shape.lineDashPattern = nil
            case .ghost:
                shape.fillColor = nil
                shape.strokeColor = (layer.color ?? ink).withAlphaComponent(0.8).cgColor
                shape.lineWidth = 1.6 * k
                shape.lineDashPattern = [NSNumber(value: Double(5 * k)), NSNumber(value: Double(4 * k))]
            default:
                shape.fillColor = nil
                shape.strokeColor = (layer.color ?? ink).withAlphaComponent(0.47).cgColor
                shape.lineWidth = 1.2 * k
                shape.lineDashPattern = [NSNumber(value: Double(3 * k)), NSNumber(value: Double(4 * k))]
            }
        case .dot, .ring:
            let shape = host.sublayers?.first as? CAShapeLayer ?? {
                let s = CAShapeLayer()
                host.addSublayer(s)
                return s
            }()
            shape.frame = host.bounds
            let r = 3.4 * k
            let dots = CGMutablePath()
            for s in layer.shapes {
                for part in s.parts {
                    for v in part {
                        guard let q = p.point(v, facing: 0.1) else { continue }
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
                guard let text = s.label, !text.isEmpty, let v = s.parts.first?.first, let q = p.point(v, facing: 0.3),
                      reach.contains(q), let image = picture(text, layer: layer, reading: k) else { continue }
                let w = CGFloat(image.width) / contentScale, h = CGFloat(image.height) / contentScale
                let frame = CGRect(x: q.x + layer.offset.dx * k - w / 2, y: q.y + layer.offset.dy * k - h / 2, width: w, height: h)
                let room = frame.insetBy(dx: -0.2 * CGFloat(layer.size) * k, dy: -0.2 * CGFloat(layer.size) * k)
                guard bounds.contains(frame), !placed.contains(where: { $0.intersects(room) }) else { continue }
                placed.append(room)
                let l = CALayer()
                l.contents = image
                l.contentsScale = contentScale
                l.frame = frame
                host.addSublayer(l)
            }
        case .symbol:
            host.sublayers?.forEach { $0.removeFromSuperlayer() }
            guard let image = layer.image?.cgImage else { return }
            let w = layer.imageSize.width * k, h = layer.imageSize.height * k
            for s in layer.shapes {
                for part in s.parts {
                    for v in part {
                        guard let q = p.point(v, facing: 0.2), bounds.insetBy(dx: -w, dy: -h).contains(q) else { continue }
                        let l = CALayer()
                        l.contents = image
                        l.contentsGravity = .resizeAspect
                        l.frame = CGRect(x: q.x + layer.offset.dx * k - w / 2, y: q.y + layer.offset.dy * k - h / 2, width: w, height: h)
                        host.addSublayer(l)
                        placed.append(l.frame)
                    }
                }
            }
        case .sounding:
            host.sublayers?.forEach { $0.removeFromSuperlayer() }
            let blue = UIColor(red: 0.11, green: 0.478, blue: 0.69, alpha: 1)
            for s in layer.shapes {
                guard let v = s.parts.first?.first, let q = p.point(v, facing: 0.2), bounds.insetBy(dx: -40, dy: -40).contains(q) else { continue }
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

    /// Lines break where they go behind the globe; an area's corners behind it are drawn on its edge.
    private func path(of layer: Layer, projection p: Projection, filled: Bool) -> CGPath {
        let path = CGMutablePath()
        for s in layer.shapes {
            for part in s.parts where part.count > 1 {
                if s.kind == .polygon {
                    let points = part.map { p.clamped($0) }
                    guard points.contains(where: { $0.facing > 0 }) else { continue }
                    path.move(to: points[0].point)
                    for q in points.dropFirst() { path.addLine(to: q.point) }
                    path.closeSubpath()
                } else {
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
    static func layer(id: String, features: Any?, options: [String: Any], packRoot: URL) -> Layer? {
        guard let style = Style(rawValue: (options["style"] as? String) ?? "border") else { return nil }
        var shapes: [Shape] = []
        func number(_ x: Any?) -> Double? { (x as? NSNumber).map(\.doubleValue).flatMap { $0.isFinite ? $0 : nil } }
        var budget = vertexBudget
        func position(_ x: Any?) -> SIMD3<Double>? {
            guard budget > 0, let a = x as? [Any], a.count >= 2, let lon = number(a[0]), let lat = number(a[1]),
                  abs(lat) <= 90, abs(lon) <= 540 else { return nil }
            budget -= 1
            return Projection.world(lon: lon, lat: lat)
        }
        func ring(_ x: Any?) -> [SIMD3<Double>] { ((x as? [Any]) ?? []).compactMap(position) }
        func add(geometry: [String: Any], properties: [String: Any], depth: Int = 0) {
            guard depth < 4, budget > 0 else { return }
            let label = (properties["label"] as? String).map { String($0.prefix(80)) }
            let length = number(properties["length"]).map { min(max($0, 0), 2000) }
            let c = geometry["coordinates"]
            switch geometry["type"] as? String {
            case "Point": if let v = position(c) { shapes.append(Shape(kind: .points, parts: [[v]], label: label, length: length)) }
            case "MultiPoint": shapes.append(Shape(kind: .points, parts: [ring(c)], label: label, length: length))
            case "LineString": shapes.append(Shape(kind: .lines, parts: [ring(c)], label: label, length: length))
            case "MultiLineString": shapes.append(Shape(kind: .lines, parts: ((c as? [Any]) ?? []).map(ring), label: label, length: length))
            case "Polygon": shapes.append(Shape(kind: .polygon, parts: ((c as? [Any]) ?? []).map(ring), label: label, length: length))
            case "MultiPolygon":
                for polygon in (c as? [Any]) ?? [] {
                    shapes.append(Shape(kind: .polygon, parts: ((polygon as? [Any]) ?? []).map(ring), label: label, length: length))
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
        for item in items.prefix(50_000) {
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
                     image: image, imageSize: CGSize(width: width, height: height), offset: shift)
    }
}
