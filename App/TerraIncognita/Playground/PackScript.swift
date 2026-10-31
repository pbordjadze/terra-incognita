import Foundation
import JavaScriptCore
import UIKit

// A pack's scripts, run in JavaScriptCore in the app's own process: no web page, no web view
// (docs/runtime3.md is the contract). terra.js (Prototypes/_runtime/terra.js) runs first, then the
// pack's `scripts` in order, in one context. The script decides; the globe draws: everything it asks
// for comes out of here as a `PackScriptEvent`, which the prototype's stage turns into the globe's
// own drawing. The script can reach nothing but its pack's files and this bridge.

/// What a pack's script asks of the host.
enum PackScriptEvent {
    case close, flyTo(GlobeCamera, duration: Double), log(String), listensForOpen
    case geoSet(GeoLayerView.Layer), geoRemove(String), geoClear
    case imageSet(GlobeImageLayer.Picture), imageRemove(String), imageClear
    case spriteSet(String, SpriteSpec), spriteRemove(String), spriteClear
    case touchScheme(GlobeView.DragScheme)
    case timeShow(GlobeTime, carry: Bool), timeSet(Double), timePlay(Bool), timeSpeed(Double), timeHide
    case caption(StageCaption?)
    case globe(historical: Bool, still: Bool, maxZoom: Double?)
    case panelShow(PackPanel), panelHide
}

/// A sprite as a script describes it (`terra.sprites.set`), its pictures already read.
struct SpriteSpec {
    var at: (lon: Double, lat: Double)?
    var path: GlobeMarker.Path?
    var looks: [GlobeMarker.SpriteLook]
    var label: String
    var touchable: Bool
}

/// One running pack's script context. Made when the pack starts; `stop()` lets all of it go.
final class PackScript {
    struct Settings: Equatable {
        var reduceMotion: Bool
        var textScale: Double
        var active: Bool
    }

    struct CameraState: Equatable {
        var lon, lat, zoom, cx, cy, radius, width, height: Double
        var finite: Bool { [lon, lat, zoom, cx, cy, radius, width, height].allSatisfy { $0.isFinite } }
        var payload: [String: Any] {
            ["lon": lon, "lat": lat, "zoom": zoom, "cx": cx, "cy": cy, "radius": radius, "width": width, "height": height]
        }
    }

    /// How the reader came in (`terra.entry`): by a doorway, the workbench, a link or a debug launch,
    /// and where the reader was.
    struct Entry {
        var by: String
        var lon, lat, zoom: Double
    }

    let pack: PlaygroundPack
    var onEvent: ((PackScriptEvent) -> Void)?
    /// The script registered `terra.on('open')`: the chip offers Open.
    private(set) var listensForOpen = false
    private let entry: Entry
    /// In front (`terra.focus`): the stage says.
    private var front: Bool

    private var context: JSContext?
    private let live = LiveCount.Token(.scripts)
    private var settings: Settings
    private var observers: [any NSObjectProtocol] = []
    private var timers: [Int: Timer] = [:]
    private var stopped = false
    private var camera: CameraState?
    private var sentCamera: CameraState?
    private var lastCameraEvent = 0.0
    private var lastCameraChange = 0.0
    private var cameraSettle: Timer?
    /// The pictures this pack has laid on the globe, decoded, by path.
    private var decoded: [String: CGImage] = [:]
    /// Its sprites' pictures by path, read once: a herd's animals share their frames.
    private var pictures: [String: UIImage] = [:]

    /// The most timers a pack may keep at once, and the shortest repeat.
    static let timerBudget = 256
    static let shortestRepeat = 1.0 / 60

    init(pack: PlaygroundPack, active: Bool, entry: Entry, front: Bool = true) {
        self.pack = pack
        self.settings = Self.systemSettings(active: active)
        self.entry = entry
        self.front = front
    }

    // MARK: starting and stopping

    /// Runs terra.js and the pack's scripts. False (with the reason logged) if it could not start.
    @discardableResult
    func start() -> Bool {
        guard context == nil, !stopped else { return false }
        guard let vm = JSVirtualMachine(), let context = JSContext(virtualMachine: vm) else {
            fail("JavaScriptCore could not make a context.")
            return false
        }
        context.name = "pack \(pack.id)"
        let id = pack.id
        context.exceptionHandler = { _, exception in
            let message = exception?.toString() ?? "unknown error"
            let line = exception?.objectForKeyedSubscript("line")?.toString() ?? "?"
            let source = (exception?.objectForKeyedSubscript("sourceURL")?.toString() ?? "").split(separator: "/").last.map(String.init) ?? ""
            PackStore.shared.record("\(id): error: \(message) (\(source):\(line))")
        }
        // (the block holds the script weakly: the context holds the block)
        let send: @convention(block) (JSValue?) -> JSValue? = { [weak self] message in
            guard let current = JSContext.current() else { return nil }
            guard let self, let body = message?.toDictionary() as? [String: Any] else { return JSValue(undefinedIn: current) }
            guard let answer = self.receive(body) else { return JSValue(undefinedIn: current) }
            return JSValue(object: answer, in: current)
        }
        context.setObject(send, forKeyedSubscript: "__terraSend" as NSString)
        context.setObject(boot, forKeyedSubscript: "__terraBoot" as NSString)
        self.context = context

        guard let runtime = Bundle.main.resourceURL?.appending(path: "Prototypes/_runtime/terra.js"),
              let source = try? String(contentsOf: runtime, encoding: .utf8) else {
            fail("The app's runtime (Prototypes/_runtime/terra.js) is missing, so the pack was not started.")
            return false
        }
        context.evaluateScript(source, withSourceURL: runtime)
        observe()
        for path in pack.manifest.scripts ?? [] {
            guard !stopped else { return false }
            guard let text = read(path) else {
                fail("Its script “\(path)” is not in the pack.")
                return false
            }
            context.evaluateScript(text, withSourceURL: URL(string: "terra-pack://\(pack.id)/\(path)"))
        }
        PackStore.shared.record("\(pack.id) r\(pack.revision): started (\(pack.manifest.scripts?.count ?? 0) scripts)")
        return true
    }

    /// The script is told it is ending; then its timers, observers and context are let go. Safe to call
    /// more than once.
    func stop() {
        guard !stopped else { return }
        emit(["type": "close"])
        stopped = true
        for (_, t) in timers { t.invalidate() }
        timers = [:]
        cameraSettle?.invalidate()
        cameraSettle = nil
        for token in observers { NotificationCenter.default.removeObserver(token) }
        observers = []
        context?.setObject(nil, forKeyedSubscript: "__terraSend" as NSString)
        context = nil
        decoded = [:]
        pictures = [:]
        onEvent = nil
    }

    private func fail(_ reason: String) {
        PackStore.shared.record("\(pack.id): error: \(reason)")
        onEvent?(.log("error: " + reason))
    }

    private var boot: [String: Any] {
        var b: [String: Any] = [
            "pack": ["id": pack.id, "revision": pack.revision, "origin": pack.origin.rawValue] as [String: Any],
            "settings": settingsPayload(settings),
            "focus": front,
            "entry": ["by": entry.by, "lon": entry.lon, "lat": entry.lat, "zoom": entry.zoom] as [String: Any],
        ]
        if let camera { b["camera"] = camera.payload }
        return b
    }

    // MARK: telling the script

    /// `terra._receive(message)`.
    private func emit(_ message: [String: Any]) {
        guard !stopped, let terra = context?.objectForKeyedSubscript("terra"), !terra.isUndefined else { return }
        terra.invokeMethod("_receive", withArguments: [message])
    }

    /// The camera, every globe frame: the script hears of it at most twenty times a second while it
    /// moves, and once when it stops; `terra.camera` is always now.
    func update(camera state: CameraState) {
        guard !stopped, state.finite, state != camera else { return }
        camera = state
        let now = CACurrentMediaTime()
        lastCameraChange = now
        if now - lastCameraEvent >= 0.05 {
            sendCamera()
        }
        // once it has stopped (no change for a tenth of a second), the script hears where it came to rest
        armSettle()
    }

    /// A one-shot check that re-arms itself until the camera has rested (so it never outlives the script).
    private func armSettle() {
        guard cameraSettle == nil, !stopped else { return }
        let settle = Timer(timeInterval: 0.05, repeats: false) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self else { return }
                self.cameraSettle = nil
                guard !self.stopped else { return }
                if CACurrentMediaTime() - self.lastCameraChange >= 0.1 { self.sendCamera() } else { self.armSettle() }
            }
        }
        RunLoop.main.add(settle, forMode: .common)
        cameraSettle = settle
    }

    private func sendCamera() {
        guard let camera, camera != sentCamera else { return }
        sentCamera = camera
        lastCameraEvent = CACurrentMediaTime()
        emit(["type": "camera"])
    }

    /// A touch on the world: a tap, or a drag's began/moved/ended. `target` names the touchable thing.
    func touch(_ phase: String, target: [String: Any]?, at point: CGPoint, place: (lon: Double, lat: Double)?) {
        var message: [String: Any] = ["type": "touch", "phase": phase, "x": Double(point.x), "y": Double(point.y),
                                      "target": target ?? NSNull()]
        if let place, place.lon.isFinite, place.lat.isFinite {
            message["lon"] = place.lon
            message["lat"] = place.lat
        } else {
            message["lon"] = NSNull()
            message["lat"] = NSNull()
        }
        emit(message)
    }

    /// The globe's date moved, by the reader, playback or the host.
    func time(_ value: Double, playing: Bool, by mover: GlobeTime.Mover) {
        guard value.isFinite else { return }
        emit(["type": "time", "value": value, "playing": playing, "by": mover.rawValue])
    }

    /// The chip's Open (or the doorway's).
    func open() { emit(["type": "open", "params": [String: Any]()]) }

    /// It came to the front, or left it (runtime 4: `terra.focus`, `terra.on('focus')`).
    func focus(_ isFront: Bool, by: String) {
        guard isFront != front else { return }
        front = isFront
        emit(["type": "focus", "front": isFront, "by": by])
    }

    /// The reader closed the panel.
    func panelClosed() { emit(["type": "panel", "open": false]) }

    // MARK: settings

    private static func systemSettings(active: Bool) -> Settings {
        let body = Double(UIFont.preferredFont(forTextStyle: .body).pointSize)
        return Settings(reduceMotion: UIAccessibility.isReduceMotionEnabled, textScale: min(3.2, max(1, body / 17)), active: active)
    }

    private func settingsPayload(_ s: Settings) -> [String: Any] {
        ["reduceMotion": s.reduceMotion, "textScale": s.textScale, "active": s.active, "wikipedia": false]
    }

    private func observe() {
        let center = NotificationCenter.default
        let system: [Notification.Name] = [UIAccessibility.reduceMotionStatusDidChangeNotification,
                                           UIContentSizeCategory.didChangeNotification]
        for name in system {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.settingsChanged(active: nil) }
            })
        }
        for (name, active) in [(UIApplication.didBecomeActiveNotification, true), (UIApplication.willResignActiveNotification, false)] {
            observers.append(center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                MainActor.assumeIsolated { self?.settingsChanged(active: active) }
            })
        }
    }

    private func settingsChanged(active: Bool?) {
        guard !stopped else { return }
        let next = Self.systemSettings(active: active ?? settings.active)
        guard next != settings else { return }
        settings = next
        emit(["type": "settings", "settings": settingsPayload(next)])
    }

    // MARK: what the script asks

    /// One message from `__terraSend`; what it returns is the script's answer (`read`, `camera`).
    private func receive(_ body: [String: Any]) -> Any? {
        guard !stopped, let type = body["type"] as? String else { return nil }
        switch type {
        case "log":
            let text = String(((body["message"] as? String) ?? "").prefix(2000))
            let level = body["level"] as? String
            onEvent?(.log(level == "error" ? "error: " + text : level == "warn" ? "warning: " + text : text))
        case "close": onEvent?(.close)
        case "read":
            guard let path = body["path"] as? String else { return nil }
            return read(path)
        case "camera": return camera?.payload
        case "listen":
            if (body["event"] as? String) == "open", !listensForOpen {
                listensForOpen = true
                onEvent?(.listensForOpen)
            }
        case "flyTo":
            guard let lon = Self.number(body["lon"]), let lat = Self.number(body["lat"]) else { return nil }
            let zoom = Self.number(body["zoom"]) ?? camera?.zoom ?? 1
            onEvent?(.flyTo(GlobeCamera(lon: lon, lat: lat, zoom: zoom), duration: min(10, max(0, Self.number(body["duration"]) ?? 0.8))))
        case "timer": timer(body)
        case "geo": geo(body)
        case "image": image(body)
        case "sprite": sprite(body)
        case "touch":
            if (body["op"] as? String) == "scheme" {
                let scheme: GlobeView.DragScheme
                switch body["scheme"] as? String {
                case "one-finger": scheme = .oneFinger
                case "hold": scheme = .hold
                default: scheme = .touchables
                }
                onEvent?(.touchScheme(scheme))
            }
        case "time": dates(body)
        case "caption":
            // (the foot is a caption, not a page: a line or two)
            let title = String(((body["title"] as? String) ?? "").prefix(120)), text = String(((body["text"] as? String) ?? "").prefix(280))
            onEvent?(.caption(title.isEmpty && text.isEmpty ? nil : StageCaption(title: title, text: text)))
        case "globe":
            onEvent?(.globe(historical: (body["historical"] as? Bool) ?? false, still: (body["still"] as? Bool) ?? false,
                            maxZoom: Self.number(body["maxZoom"])))
        case "panel":
            if (body["op"] as? String) == "show" {
                onEvent?(.panelShow(PackPanel(pack.id, (body["content"] as? [String: Any]) ?? [:])))
            } else {
                onEvent?(.panelHide)
            }
        default: break
        }
        return nil
    }

    /// A file of the pack as text, if it is one (a path inside the pack, at most 25 MB).
    private func read(_ path: String) -> String? {
        guard PackRules.isSafePath(path) else { return nil }
        let file = pack.root.appending(path: path)
        guard let size = (try? file.resourceValues(forKeys: [.fileSizeKey]))?.fileSize, size <= PackRules.maxFileBytes else { return nil }
        return try? String(contentsOf: file, encoding: .utf8)
    }

    /// A picture of the pack (for geography symbols, sprites and images).
    func picture(_ path: String) -> UIImage? {
        if let known = pictures[path] { return known }
        guard PackRules.isSafePath(path), let image = UIImage(contentsOfFile: pack.root.appending(path: path).path(percentEncoded: false)) else { return nil }
        // (a pack's pictures are bounded by its size; they go when it stops)
        pictures[path] = image
        return image
    }

    private func timer(_ body: [String: Any]) {
        guard let id = (body["id"] as? NSNumber)?.intValue else { return }
        timers.removeValue(forKey: id)?.invalidate()
        guard (body["op"] as? String) == "set", timers.count < Self.timerBudget else { return }
        let repeats = (body["repeat"] as? Bool) ?? false
        let seconds = max(repeats ? Self.shortestRepeat : 0, (Self.number(body["ms"]) ?? 0) / 1000)
        let t = Timer(timeInterval: seconds, repeats: repeats) { [weak self] _ in
            MainActor.assumeIsolated {
                guard let self, !self.stopped else { return }
                if !repeats { self.timers.removeValue(forKey: id) }
                self.emit(["type": "timer", "id": id])
            }
        }
        RunLoop.main.add(t, forMode: .common)
        timers[id] = t
    }

    private func geo(_ body: [String: Any]) {
        switch body["op"] as? String {
        case "set":
            guard let id = body["id"] as? String, !id.isEmpty, id.count <= 64 else { return }
            if let layer = GeoLayerView.layer(id: id, features: body["features"], options: (body["options"] as? [String: Any]) ?? [:],
                                              packRoot: pack.root) {
                onEvent?(.geoSet(layer))
            } else {
                onEvent?(.log("error: geo layer \(id) has a style the globe does not draw"))
            }
        case "remove": if let id = body["id"] as? String { onEvent?(.geoRemove(id)) }
        case "clear": onEvent?(.geoClear)
        default: break
        }
    }

    private func image(_ body: [String: Any]) {
        switch body["op"] as? String {
        case "set":
            guard let id = body["id"] as? String, !id.isEmpty, id.count <= 64 else { return }
            let o = (body["options"] as? [String: Any]) ?? [:]
            guard let name = o["image"] as? String, PackRules.isSafePath(name),
                  let b = (o["bounds"] as? [Any])?.compactMap(Self.number), b.count == 4, b[2] > b[0], b[3] > b[1],
                  b[1] >= -90, b[3] <= 90, b[2] - b[0] <= 360 else {
                onEvent?(.log("error: image \(id) needs an image of the pack and bounds [west, south, east, north]"))
                return
            }
            // (decoded once per pack: setting the same picture again costs nothing)
            guard let image = decoded[name] ?? GlobeImageLayer.Picture.read(pack.root.appending(path: name)) else {
                onEvent?(.log("error: image \(id): “\(name)” could not be read as a picture"))
                return
            }
            decoded[name] = image
            onEvent?(.imageSet(GlobeImageLayer.Picture(
                id: id, image: image, west: b[0], south: b[1], east: b[2], north: b[3],
                opacity: min(1, max(0, Self.number(o["opacity"]) ?? 1)),
                minZoom: Self.number(o["minZoom"]) ?? 0, maxZoom: Self.number(o["maxZoom"]) ?? .infinity,
                fade: max(0, Self.number(o["fade"]) ?? 0))))
        case "remove": if let id = body["id"] as? String { onEvent?(.imageRemove(id)) }
        case "clear": onEvent?(.imageClear)
        default: break
        }
    }

    private func sprite(_ body: [String: Any]) {
        switch body["op"] as? String {
        case "set":
            guard let id = body["id"] as? String, !id.isEmpty, id.count <= 64 else { return }
            let o = (body["options"] as? [String: Any]) ?? [:]
            var spec = SpriteSpec(at: nil, path: nil, looks: [], label: String(((o["label"] as? String) ?? "").prefix(300)),
                                  touchable: (o["touchable"] as? Bool) ?? false)
            if let at = (o["at"] as? [Any])?.compactMap(Self.number), at.count == 2 { spec.at = (at[0], at[1]) }
            if let p = o["path"] as? [String: Any] {
                let points = ((p["coordinates"] as? [Any]) ?? []).prefix(2000).compactMap { ($0 as? [Any])?.compactMap(Self.number) }
                    .filter { $0.count >= 2 && abs($0[1]) <= 90 }.map { (lon: $0[0], lat: $0[1]) }
                let loop: GlobeMarker.Path.Loop
                switch p["loop"] {
                case let s as String where s == "back": loop = .back
                case let b as Bool where b == false: loop = .once
                default: loop = .repeat
                }
                // a dated way (runtime 5): one date for each place, rising; the stage's date moves it
                var dates: [Double]?
                if let given = p["dates"] as? [Any] {
                    let d = given.compactMap(Self.number)
                    guard d.count == given.count, d.count == points.count, (p["coordinates"] as? [Any])?.count == points.count,
                          zip(d, d.dropFirst()).allSatisfy({ $1 >= $0 }) else {
                        onEvent?(.log("error: sprite \(id): path.dates needs a number for each place, rising"))
                        return
                    }
                    dates = d
                }
                spec.path = GlobeMarker.Path(points: points, seconds: max(0.1, Self.number(p["seconds"]) ?? 30), loop: loop, dates: dates)
            }
            for look in ((o["looks"] as? [Any]) ?? []).prefix(8) {
                guard let l = look as? [String: Any] else { continue }
                let names = ((l["frames"] as? [Any])?.compactMap { $0 as? String } ?? (l["image"] as? String).map { [$0] } ?? []).prefix(24)
                let images = names.compactMap(picture)
                guard !images.isEmpty else {
                    onEvent?(.log("error: sprite \(id) has a look with no picture of the pack"))
                    continue
                }
                let size = (l["size"] as? [Any])?.compactMap(Self.number)
                spec.looks.append(GlobeMarker.SpriteLook(
                    frames: images, fps: min(30, max(0.2, Self.number(l["fps"]) ?? 6)),
                    // (up to 400 points a side, runtime 5: an animal drawn close)
                    size: CGSize(width: min(400, max(4, size?.first ?? 28)), height: min(400, max(4, size?.dropFirst().first ?? size?.first ?? 28))),
                    minZoom: Self.number(l["minZoom"]) ?? 0, maxZoom: Self.number(l["maxZoom"]) ?? .infinity,
                    every: Self.number(l["every"]).map { max(0.2, $0) }, duration: Self.number(l["for"]).map { max(0.05, $0) },
                    face: (l["face"] as? Bool) ?? false))
            }
            guard spec.at != nil || (spec.path?.points.isEmpty == false), !spec.looks.isEmpty else {
                onEvent?(.log("error: sprite \(id) needs a place (at or path) and a look"))
                return
            }
            onEvent?(.spriteSet(id, spec))
        case "remove": if let id = body["id"] as? String { onEvent?(.spriteRemove(id)) }
        case "clear": onEvent?(.spriteClear)
        default: break
        }
    }

    private func dates(_ body: [String: Any]) {
        switch body["op"] as? String {
        case "show":
            let options = (body["options"] as? [String: Any]) ?? [:]
            guard var time = GlobeTime.from(options) else {
                onEvent?(.log("error: terra.time.show needs a min below its max"))
                return
            }
            time.playing = (options["playing"] as? Bool) ?? false
            onEvent?(.timeShow(time, carry: (options["carry"] as? Bool) ?? false))
        case "set": if let v = Self.number(body["value"]) { onEvent?(.timeSet(v)) }
        case "play": onEvent?(.timePlay((body["playing"] as? Bool) ?? true))
        case "speed": if let f = Self.number(body["speed"]) { onEvent?(.timeSpeed(min(100, max(0.001, f)))) }
        case "hide": onEvent?(.timeHide)
        default: break
        }
    }

    static func number(_ value: Any?) -> Double? {
        guard let boxed = value as? NSNumber, boxed.doubleValue.isFinite else { return nil }
        return boxed.doubleValue
    }
}
