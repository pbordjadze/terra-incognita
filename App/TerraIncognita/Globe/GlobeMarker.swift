import UIKit

/// A picture a prototype places at a point of the globe (`GlobeView.add`): it follows the geography,
/// can be tapped, keeps the names clear of it and is one of the map's accessibility elements. It
/// shows only while it faces the reader and the zoom is within `minZoom`...`maxZoom`; the prototype's
/// stage takes it away again when the prototype ends.
///
/// A sprite (runtime 3, `terra.sprites`) is a marker with `looks`: it may move along a `path`, show
/// frames in turn, change how it looks with the zoom (a trace from afar, itself up close), show only
/// now and then, and turn to face the way it moves. A doorway is a marker too.
final class GlobeMarker {
    enum Look {
        /// The picture as drawn, with no ground (an animal, a fossil, a pack's image).
        case picture(UIImage)
        /// A small paper seal with a symbol, ruled in the margin's blue ink.
        case seal(symbol: String)
    }

    /// How a sprite shows at a range of zooms.
    struct SpriteLook {
        var frames: [UIImage]
        var fps: Double
        /// In points, at the globe's reading scale.
        var size: CGSize
        var minZoom: Double
        var maxZoom: Double
        /// Shows for `duration` seconds every `every` seconds (a splash now and then), else always.
        var every: Double?
        var duration: Double?
        /// The picture is drawn facing east; it turns to face the way the sprite moves.
        var face: Bool
    }

    /// A way along great circles between places, the whole of it in `seconds`.
    struct Path {
        enum Loop { case `repeat`, back, once }
        let points: [SIMD3<Double>]
        /// The angle along the way at each point (radians from the first).
        let along: [Double]
        let seconds: Double
        let loop: Loop
        let places: [(lon: Double, lat: Double)]

        init(points places: [(lon: Double, lat: Double)], seconds: Double, loop: Loop) {
            self.places = places
            points = places.map { GeoLayerView.Projection.world(lon: $0.lon, lat: $0.lat) }
            var a = [0.0]
            for i in points.indices.dropFirst() {
                a.append(a[i - 1] + acos(min(1, max(-1, (points[i - 1] * points[i]).sum()))))
            }
            along = a
            self.seconds = seconds
            self.loop = loop
        }

        func isSame(as other: Path) -> Bool {
            other.seconds == seconds && other.loop == loop && other.places.count == places.count
                && zip(other.places, places).allSatisfy { $0.lon == $1.lon && $0.lat == $1.lat }
        }

        /// Where it is `t` seconds after it set out: lon, lat.
        func position(at t: Double) -> (lon: Double, lat: Double) {
            guard points.count > 1, let total = along.last, total > 0 else { return places.first ?? (0, 0) }
            var u = t / seconds
            switch loop {
            case .repeat: u = u - floor(u)
            case .back:
                let v = u.truncatingRemainder(dividingBy: 2)
                u = v <= 1 ? v : 2 - v
            case .once: u = min(max(u, 0), 1)
            }
            let s = u * total
            var i = 1
            while i < along.count - 1, along[i] < s { i += 1 }
            let seg = along[i] - along[i - 1]
            let f = seg > 0 ? (s - along[i - 1]) / seg : 0
            let a = points[i - 1], b = points[i]
            // slerp between the two
            let omega = seg
            let v: SIMD3<Double>
            if omega < 1e-9 { v = a } else { v = (sin((1 - f) * omega) * a + sin(f * omega) * b) / sin(omega) }
            let lat = asin(min(1, max(-1, v.y))) * 180 / .pi
            let lon = atan2(v.x, v.z) * 180 / .pi
            return (lon, lat)
        }
    }

    private(set) var lon: Double
    private(set) var lat: Double
    let label: String
    let minZoom: Double
    let maxZoom: Double
    /// A slow breath, for something alive (still under Reduce Motion).
    let breathes: Bool
    /// VoiceOver's name for opening it directly from the map (beside "Go to …").
    let actionName: String?
    let onOpen: (() -> Void)?
    let view: UIButton = MarkerButton(type: .custom)
    private let live: LiveCount.Token?

    // sprites
    private(set) var looks: [SpriteLook] = []
    private(set) var path: Path?
    private var setOut = CACurrentMediaTime()
    private var frameView: UIImageView?
    private var shownLook: Int?
    /// Hidden now by its look: out of every look's zoom range, or between its showings.
    private(set) var resting = false
    /// It moves or animates, so the globe draws a little more often while it shows.
    var animates: Bool { path != nil || looks.contains { $0.frames.count > 1 || $0.every != nil } }

    init(id: String, lon: Double, lat: Double, look: Look, size: CGSize, label: String, hint: String? = nil,
         minZoom: Double = 0, maxZoom: Double = .infinity, breathes: Bool = false, actionName: String? = nil,
         counted: Bool = true, onOpen: (() -> Void)?) {
        self.lon = lon
        self.lat = lat
        self.label = label
        self.minZoom = minZoom
        self.maxZoom = maxZoom
        self.breathes = breathes
        self.actionName = actionName
        self.onOpen = onOpen
        live = counted ? LiveCount.Token(.markers) : nil
        switch look {
        case .picture(let image):
            view.setImage(image.withRenderingMode(.alwaysOriginal), for: .normal)
            view.imageView?.contentMode = .scaleAspectFit
        case .seal(let symbol):
            view.setImage(UIImage(systemName: symbol), for: .normal)
            view.tintColor = UIColor(hex: "#3B2C16")
            view.backgroundColor = UIColor(hex: "#F2E4C8")
            view.layer.cornerRadius = min(size.width, size.height) / 2
            view.layer.borderWidth = 1.5
            view.layer.borderColor = UIColor(hex: "#6D6793")?.cgColor
        }
        view.bounds = CGRect(origin: .zero, size: size)
        view.accessibilityIdentifier = id
        view.accessibilityLabel = label
        view.accessibilityHint = hint
        view.isUserInteractionEnabled = onOpen != nil
        if let onOpen { view.addAction(UIAction { _ in onOpen() }, for: .touchUpInside) }
    }

    /// A sprite: its looks by zoom, a place or a path. `onTap` makes it touchable.
    convenience init(sprite id: String, at place: (lon: Double, lat: Double)?, path: Path?, looks: [SpriteLook], label: String,
                     hint: String? = nil, counted: Bool = true, onTap: (() -> Void)?) {
        let start = place ?? path?.places.first ?? (0, 0)
        self.init(id: id, lon: start.lon, lat: start.lat, look: .picture(UIImage()), size: looks.first?.size ?? CGSize(width: 28, height: 28),
                  label: label, hint: hint, counted: counted, onOpen: onTap)
        view.setImage(nil, for: .normal)
        let frames = UIImageView()
        frames.contentMode = .scaleAspectFit
        frames.isUserInteractionEnabled = false
        frames.frame = view.bounds
        frames.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        view.addSubview(frames)
        frameView = frames
        self.looks = looks
        self.path = path
        view.isAccessibilityElement = !label.isEmpty
        if onTap == nil { view.accessibilityTraits = .image }
    }

    /// Changes a sprite's looks, place or path; the same path keeps its place along it.
    func update(at place: (lon: Double, lat: Double)?, path: Path?, looks: [SpriteLook]) {
        if let path, let old = self.path, old.isSame(as: path) {
            // keep going where it was
        } else {
            setOut = CACurrentMediaTime()
            self.path = path
        }
        if path == nil, let place { lon = place.lon; lat = place.lat }
        self.looks = looks
        shownLook = nil
    }

    /// A sprite's state for this frame: where it is along its path, which look shows, whether it rests.
    /// The globe calls it before placing the marker; `ahead` is where it will be a moment later.
    func advance(now: CFTimeInterval, zoom: Double, reading: CGFloat, still: Bool,
                 project: (Double, Double) -> CGPoint) {
        guard frameView != nil else { return }
        var heading: CGFloat = 0
        if let path {
            let t = still ? 0 : now - setOut
            let p = path.position(at: t)
            lon = p.lon
            lat = p.lat
            if !still {
                let q = path.position(at: t + 0.4)
                heading = project(q.lon, q.lat).x - project(p.lon, p.lat).x
            }
        }
        guard let index = looks.firstIndex(where: { zoom >= $0.minZoom && zoom <= $0.maxZoom }) else {
            resting = true
            return
        }
        let look = looks[index]
        if index != shownLook {
            shownLook = index
            let size = CGSize(width: look.size.width * reading, height: look.size.height * reading)
            view.bounds = CGRect(origin: .zero, size: size)
            frameView?.frame = view.bounds
            frameView?.stopAnimating()
            if look.frames.count > 1, !still {
                frameView?.animationImages = look.frames
                frameView?.animationDuration = Double(look.frames.count) / look.fps
                frameView?.image = look.frames[0]
                frameView?.startAnimating()
            } else {
                frameView?.animationImages = nil
                frameView?.image = look.frames.first
            }
        }
        if look.face, heading != 0 {
            frameView?.transform = heading < 0 ? CGAffineTransform(scaleX: -1, y: 1) : .identity
        }
        if let every = look.every, let duration = look.duration, !still {
            // each sprite keeps its own phase, so traces don't all blink together
            let phase = (now - setOut + Double(abs(label.hashValue % 997)) / 997 * every).truncatingRemainder(dividingBy: every)
            resting = phase > duration
        } else {
            resting = false
        }
    }
}

/// A marker's button: touchable within a fingertip (44 points) however small it is drawn.
private final class MarkerButton: UIButton {
    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool {
        let dx = max(0, 44 - bounds.width) / 2, dy = max(0, 44 - bounds.height) / 2
        return bounds.insetBy(dx: -dx, dy: -dy).contains(point)
    }
}
