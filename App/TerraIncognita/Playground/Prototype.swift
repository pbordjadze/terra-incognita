import Foundation
import Observation
import SwiftUI
import UIKit

/// One thing to try on the workbench: who it is, where on the globe it belongs (if anywhere), and
/// what starting it does. Deliberately small; add fields when two real prototypes need the same one.
/// Adding a prototype adds an entry (`Catalog.swift`, or a pack folder) and changes no other.
struct Prototype: Identifiable {
    enum Shelf: String { case current, parked }
    enum Made: Equatable {
        case native
        /// A pack: bundled with the app, downloaded, or only listed by a source so far.
        case pack(PackListing.State, revision: Int, bundled: Bool)
    }
    /// A place on the globe and how it looks there (a sprite's looks: a trace from afar, itself closer),
    /// with a small life of its own (runtime 4): a way it stirs along, a slow breath.
    struct Doorway {
        let lon: Double
        let lat: Double
        let label: String
        let looks: [GlobeMarker.SpriteLook]
        var path: GlobeMarker.Path? = nil
        var breathes = false
    }

    let id: String
    let title: String
    let summary: String
    var shelf: Shelf = .current
    /// Where the globe goes when it starts; nil leaves the camera where the reader had it.
    var anchor: GlobeCamera?
    /// Its own screen (it ends when the screen closes), or it lives on the globe until ended.
    var ownScreen = false
    var made: Made = .native
    /// How it shows on the globe while it isn't running (touching it starts it), so the atlas has its
    /// things on it; nil for none (a parked direction, a pack not on this device yet).
    var doorway: Doorway?
    /// What starting it does with the stage. Everything it adds there goes when it ends.
    let start: (PrototypeStage) -> Void
}

/// How the reader came into an experience (runtime 4, `terra.entry`): by its doorway it starts where
/// the reader is, with no flight to its anchor.
enum StageEntry: String, Equatable {
    case doorway, workbench, link, launch
}

/// What a stage asks of the globe as a whole. Several stages run at once (runtime 4), so none sets
/// the globe itself: the playground merges every stage's requests (`Playground.applyGlobeState`),
/// and a stage's go when it ends.
struct GlobeRequests: Equatable {
    /// No idle turn (any stage asking holds it).
    var still = false
    /// Today's borders and names hidden (the front stage's).
    var historical = false
    /// The parked still sea (any stage asking hides it).
    var seaHidden = false
    /// How one-finger drags are shared (the front stage's).
    var scheme = GlobeView.DragScheme.touchables
    /// Deeper than 8 for its close views (the deepest any stage asks for).
    var maxZoom: Double?
}

/// A full-screen surface over the paused globe: a native experience with a map of its own (the ocean),
/// or the films.
struct PlaygroundScreen: Identifiable {
    let id: String
    let content: () -> AnyView
}

/// A native experience on the globe itself (`PrototypeStage.openLens`; the camelid and whale
/// sequences): a canvas lying on the one globe, composing in step with it and driving its authored
/// camera, and its chrome (date, scrubber, buttons, its reading) over the playground.
struct PlaygroundLens: Identifiable {
    let id: String
    let canvas: any GlobeLensCanvas
    let chrome: () -> AnyView
}

/// The part of a lens on the globe: a view laid over it, above its names (`GlobeView.addLens`).
/// `attach` sets the lens's hooks on the globe and its camera where the experience begins. When the
/// lens closes the view leaves the globe, which stops its clock, and the stage puts back whatever it
/// set (`GlobeLease`); off the globe, it changes nothing there.
protocol GlobeLensCanvas: UIView {
    func attach()
}

/// An accessible, visible way in for a prototype on the globe, above the caption.
struct Invitation: Equatable {
    let id: String
    let title: String
    let action: String
}

/// One prototype's time on the globe. It is made when the prototype starts and owns everything the
/// prototype adds (markers and sprites, a pack's script, geography and pictures drawn by the globe, a
/// date, words at the foot, a panel, a style, a screen, a lens, an invitation); `end` takes all of it
/// away, so the globe is exactly as it was. Several stages share the globe at once (runtime 4): the
/// playground keeps the one in front, routes touches, and merges what they ask of the globe.
@Observable
final class PrototypeStage {
    let prototype: Prototype
    /// How the reader came in.
    let entry: StageEntry
    /// The globe's style while it runs (the parked Persian and Wikipedia styles are prototypes of
    /// their own); the front-most stage that sets one wins.
    var style: GlobeStyle = .base
    private(set) var invitation: Invitation?
    /// It ends when its screen closes (an own-screen prototype, or a pack's screen presentation).
    var endsWithScreen: Bool
    private(set) var ended = false
    /// Getting ready (a pack downloading): the chip says so.
    var loading = false
    /// Something here opens a screen: the chip offers Open.
    private(set) var canOpen = false
    /// The globe's date while it runs (runtime 2), and what it says at the foot of the screen; shown
    /// while this stage is in front.
    private(set) var time: GlobeTime?
    private(set) var caption: StageCaption?
    /// A native experience on the globe itself, while it is open: the playground shows its chrome.
    private(set) var lens: PlaygroundLens?

    /// In front: the one the reader is with (the playground says). Its date and caption are the foot,
    /// its scheme and `historical` the globe's, and its flights are honoured.
    @ObservationIgnored private(set) var front = true
    /// What it asks of the globe as a whole; the playground merges every stage's.
    @ObservationIgnored private(set) var requests = GlobeRequests()
    /// A pack that takes every tap on the globe (`"taps": true`), while in front.
    @ObservationIgnored private(set) var takesTaps = false
    /// Where the reader was when it started (for `terra.entry`).
    @ObservationIgnored private let entryCamera: GlobeCamera?

    @ObservationIgnored private weak var playground: Playground?
    /// What the open lens may change on the globe, as it was before: put back when it closes.
    @ObservationIgnored private var lease: GlobeLease?
    @ObservationIgnored private var markers: [GlobeMarker] = []
    @ObservationIgnored private var script: PackScript?
    @ObservationIgnored private var sprites: [String: GlobeMarker] = [:]
    @ObservationIgnored private var images: GlobeImageLayer?
    @ObservationIgnored private var invitationAction: (() -> Void)?
    /// A globe pack's own page (its sources, its longer account), for the chip's Open.
    @ObservationIgnored private var openAction: (() -> Void)?
    @ObservationIgnored private var geo: GeoLayerView?
    @ObservationIgnored private var playback: Task<Void, Never>?
    /// Flights dropped have been noted in the log (once each reason).
    @ObservationIgnored private var noted: Set<String> = []
    private let live = LiveCount.Token(.stages)
    static let spriteBudget = 64

    init(prototype: Prototype, playground: Playground, entry: StageEntry = .workbench, at camera: GlobeCamera? = nil) {
        self.prototype = prototype
        self.playground = playground
        self.entry = entry
        entryCamera = camera
        endsWithScreen = prototype.ownScreen
    }

    private var globe: GlobeView? { playground?.globe }

    /// A change to what it asks of the globe: the playground applies the merged state.
    private func request(_ change: (inout GlobeRequests) -> Void) {
        guard !ended else { return }
        var r = requests
        change(&r)
        guard r != requests else { return }
        requests = r
        playground?.globeRequestsChanged()
    }

    /// Holds the globe where the prototype put it (no idle turn) until the prototype ends.
    func holdStill() { request { $0.still = true } }

    /// The parked still sea: the globe's sea hidden until the prototype ends.
    func stillSea() { request { $0.seaHidden = true } }

    /// Words at the foot for a prototype that gives none of its own: what it is, and what to touch.
    func describe(title: String, text: String) {
        guard !ended, caption == nil else { return }
        caption = StageCaption(title: title, text: text)
    }

    /// Moves the globe (Reduce Motion: at once). A script's flight (`byScript`) is honoured only while
    /// its stage is in front and the reader's hands are off the globe: no pack takes the stage.
    func fly(to camera: GlobeCamera, duration: Double = 0.9, byScript: Bool = false) {
        guard !ended, let globe else { return }
        if byScript {
            guard front else { note("flyTo", "a flight asked for from the background is dropped (the reader is with another experience)"); return }
            guard !globe.handsOn else { note("hands", "a flight asked for while the reader's hands are on the globe is dropped"); return }
        }
        globe.setAuthoredCamera(camera, duration: UIAccessibility.isReduceMotionEnabled ? 0 : duration)
    }

    private func note(_ key: String, _ line: String) {
        guard noted.insert(key).inserted else { return }
        PackStore.shared.record("\(prototype.id): \(line)")
    }

    func place(_ marker: GlobeMarker) {
        guard !ended else { return }
        markers.append(marker)
        globe?.add(marker)
        if marker.onOpen != nil { canOpen = true }
    }

    /// Opens a screen over the paused globe.
    func present<Content: View>(_ id: String, @ViewBuilder _ content: @escaping () -> Content) {
        guard !ended else { return }
        playground?.bringToFront(self)
        playground?.present(PlaygroundScreen(id: id) { AnyView(content()) }, from: self)
    }

    /// Opens a native experience on the globe itself (a lens): made for the globe, with the way to
    /// close it, its canvas lies on the globe and its chrome over the playground. The globe is lent as
    /// the reader has it, with no country chosen, and whatever the lens changes there comes back when
    /// it closes (`closeLens`, or the prototype ending). One lens at a time: another stage's closes.
    func openLens(_ make: (_ globe: GlobeView, _ close: @escaping () -> Void) -> PlaygroundLens) {
        guard !ended, let globe else { return }
        playground?.bringToFront(self)
        playground?.closeLenses(except: self)
        closeLens()
        lease = GlobeLease(globe)
        let lens = make(globe) { [weak self] in self?.closeLens() }
        globe.selectedIndex = nil
        globe.authoredMode = true
        globe.addLens(lens.canvas)
        lens.canvas.attach()
        self.lens = lens
    }

    /// Back to the globe as the reader left it: the lens's canvas leaves it, and what the lens changed
    /// is put back. Safe to call more than once.
    func closeLens() {
        guard let lens else { return }
        self.lens = nil
        if let globe {
            globe.removeLens(lens.canvas)
            lease?.restore(globe)
        }
        lease = nil
        // (the lease put back the globe as the lens found it; the stages' merged state is applied again)
        playground?.globeRequestsChanged()
    }

    func invite(_ invitation: Invitation, action: @escaping () -> Void) {
        guard !ended else { return }
        self.invitation = invitation
        invitationAction = action
        canOpen = true
    }

    func acceptInvitation() { invitationAction?() }

    /// What the chip's Open does: a globe pack's own page, else the first marker that opens something,
    /// else the invitation. (A pack's touchable sprites answer the reader's touch, not the chip.)
    func open() {
        if let openAction { openAction() }
        else if let open = markers.first(where: { $0.onOpen != nil })?.onOpen { open() }
        else { invitationAction?() }
    }

    // MARK: the stage shared (runtime 4)

    /// The playground says whether this stage is in front. Leaving the front pauses its date; the
    /// script hears of both.
    func setFront(_ isFront: Bool, by: String) {
        guard !ended, isFront != front else { return }
        front = isFront
        if !isFront { stopPlayback() }
        script?.focus(isFront, by: by)
    }

    /// Each globe frame: the script hears the camera, and what the globe draws for it follows.
    func frame() {
        guard !ended, let globe else { return }
        if let script {
            let s = globe.stageCamera
            let size = globe.bounds.size
            script.update(camera: .init(lon: s.camera.lon, lat: s.camera.lat, zoom: s.camera.zoom,
                                        cx: Double(s.centre.x), cy: Double(s.centre.y), radius: s.radius,
                                        width: Double(size.width), height: Double(size.height)))
        }
        images?.update(globe: globe)
        geo?.update(globe: globe)
    }

    /// A touchable feature of this stage's geography under a point, if any.
    func geoHit(_ point: CGPoint, globe: GlobeView) -> (layer: String, index: Int)? {
        guard !ended else { return nil }
        return geo?.hit(point, globe: globe)
    }

    /// A touch on the world that is this stage's (the playground routed it here).
    func touch(_ phase: String, target: [String: Any]?, at point: CGPoint, place: (lon: Double, lat: Double)?) {
        guard !ended else { return }
        script?.touch(phase, target: target, at: point, place: place)
    }

    /// A pack on the globe (runtime 3): its anchor (unless entered by its doorway: it starts where the
    /// reader is), then its scripts, whose requests become the globe's own drawing.
    func start(pack: PlaygroundPack) {
        loading = false
        guard !ended, let globe else { return }
        let m = pack.manifest
        let was = entryCamera ?? globe.restingCamera
        if entry != .doorway, let a = m.anchor { fly(to: GlobeCamera(lon: a.lon, lat: a.lat, zoom: a.zoom ?? 1)) }
        // a pack the globe draws for holds it where it flew, from the start: its scripts may take a
        // moment, and the globe turns again after it has been left alone a while
        holdStill()
        takesTaps = m.taps ?? false
        let script = PackScript(pack: pack, active: UIApplication.shared.applicationState == .active,
                                entry: PackScript.Entry(by: entry.rawValue, lon: was.lon, lat: was.lat, zoom: was.zoom), front: front)
        self.script = script
        script.onEvent = { [weak self] event in self?.handle(event, pack: pack) }
        if !script.start() {
            fail(String(localized: "\(prototype.title) could not start. The workbench's log says why."))
        }
    }

    private func handle(_ event: PackScriptEvent, pack: PlaygroundPack) {
        // (a script told it is ending may still ask for things: nothing new outlives the stage)
        if ended {
            guard case .log = event else { return }
        }
        switch event {
        case .close: end()
        case .flyTo(let camera, let duration): fly(to: camera, duration: duration, byScript: true)
        case .log(let line): PackStore.shared.record("\(pack.id): \(line)")
        case .listensForOpen:
            if openAction == nil {
                openAction = { [weak self] in self?.script?.open() }
                canOpen = true
            }
        case .geoSet(let layer):
            if !geoLayers().set(layer) { PackStore.shared.record("\(pack.id): error: geo layer \(layer.id) is over the globe's budget") }
        case .geoRemove(let id): geo?.remove(id)
        case .geoClear: geo?.clear()
        case .imageSet(let picture):
            if !imageLayer().set(picture) { PackStore.shared.record("\(pack.id): error: image \(picture.id) is over the globe's budget") }
        case .imageRemove(let id): images?.remove(id)
        case .imageClear: images?.clear()
        case .spriteSet(let id, let spec): setSprite(id, spec, pack: pack)
        case .spriteRemove(let id):
            if let marker = sprites.removeValue(forKey: id) { remove(marker) }
        case .spriteClear:
            for (_, marker) in sprites { remove(marker) }
            sprites = [:]
        case .touchScheme(let scheme): request { $0.scheme = scheme }
        case .timeShow(let time, let carry): show(time, carry: carry)
        case .timeSet(let value): setTime(value, by: nil)
        case .timePlay(let playing): play(playing, by: .host)
        case .timeSpeed(let speed):
            if var t = time, t.speed != speed { t.speed = speed; time = t; dateMoved() }
        case .timeHide:
            stopPlayback()
            time = nil
            dateMoved()
        case .caption(let caption): self.caption = caption
        case .globe(let historical, let still, let maxZoom):
            request {
                $0.historical = historical
                $0.still = still
                if let maxZoom { $0.maxZoom = maxZoom }
            }
        case .panelShow(let panel): playground?.show(panel, from: self) { [weak self] in self?.script?.panelClosed() }
        case .panelHide: playground?.hidePanel(from: pack.id)
        }
    }

    /// A sprite: made, or changed in place (keeping its place along the same path). A touch on it
    /// brings its stage to the front.
    private func setSprite(_ id: String, _ spec: SpriteSpec, pack: PlaygroundPack) {
        if let marker = sprites[id] {
            marker.update(at: spec.at, path: spec.path, looks: spec.looks)
            return
        }
        guard sprites.count < Self.spriteBudget else {
            PackStore.shared.record("\(pack.id): error: sprite \(id) is over the globe's budget of \(Self.spriteBudget)")
            return
        }
        let tap: (() -> Void)? = spec.touchable ? { [weak self] in
            guard let self, let marker = self.sprites[id] else { return }
            self.playground?.bringToFront(self)
            self.touch("tap", target: ["sprite": id], at: marker.view.center, place: (marker.lon, marker.lat))
        } : nil
        let marker = GlobeMarker(sprite: "\(pack.id).\(id)", at: spec.at, path: spec.path, looks: spec.looks,
                                 label: spec.label, onTap: tap)
        marker.clock = clock
        sprites[id] = marker
        markers.append(marker)
        globe?.add(marker)
    }

    private func remove(_ marker: GlobeMarker) {
        globe?.remove(marker)
        markers.removeAll { $0 === marker }
    }

    /// Pictures lying on the globe, made on first use, under the geography.
    private func imageLayer() -> GlobeImageLayer {
        if let images { return images }
        let view = GlobeImageLayer()
        images = view
        // (addOverlay puts it lowest of what the globe draws for a pack, just above the sky: the
        // geography, made later or earlier, stays above it)
        globe?.addOverlay(view)
        return view
    }

    /// The globe's own drawing of what the page asks for, made on first use, under the markers.
    private func geoLayers() -> GeoLayerView {
        if let geo { return geo }
        let view = GeoLayerView()
        geo = view
        globe?.addOverlay(view)
        // its touchable named places are VoiceOver buttons that touch them
        view.onActivate = { [weak self, weak view] id, index, point in
            guard let self, let globe = self.globe, view != nil else { return }
            let place = globe.unprojectedPlace(point)
            self.playground?.bringToFront(self)
            self.touch("tap", target: ["layer": id, "index": index], at: point, place: place)
        }
        globe?.accessibleOverlays.append(view)
        // (geography lies above any pictures, whichever was asked for first)
        if let images { view.superview?.insertSubview(view, aboveSubview: images) }
        return view
    }

    // MARK: the globe's date (runtime 2)

    private func show(_ shown: GlobeTime, carry: Bool) {
        guard !ended else { return }
        stopPlayback()
        var t = shown
        let wanted = t.playing
        t.playing = false
        // from the date the last time-aware prototype left, or the nearest end of this one's range to it
        if carry, let year = playground?.lastYear { t.value = t.clamped(t.value(forYear: year)) }
        time = t
        dateMoved()
        playground?.lastYear = t.year
        script?.time(t.value, playing: false, by: .host)
        if wanted, front, !UIAccessibility.isReduceMotionEnabled { play(true) }
    }

    /// Moves the date: the reader's drag (which pauses playback), playback's tick, or the page's own
    /// `terra.time.set` (`by` nil: the page knows already).
    func setTime(_ value: Double, by mover: GlobeTime.Mover?) {
        guard !ended, var t = time, value.isFinite else { return }
        if mover == .reader, t.playing { stopPlayback(); t.playing = false }
        t.value = t.clamped(value)
        time = t
        dateMoved()
        playground?.lastYear = t.year
        if let mover { script?.time(t.value, playing: t.playing, by: mover) }
    }

    /// Plays the date forward at its rate, about thirty steps a second, to its end; the page hears of
    /// it only when something changed. It holds while the globe is out of sight (the workbench or a
    /// screen over it, the app in the background), and a stage in the background does not play.
    func play(_ playing: Bool, by mover: GlobeTime.Mover = .reader) {
        guard !ended, var t = time else { return }
        let was = (t.playing, t.value)
        stopPlayback()
        if playing, t.value >= t.max { t.value = t.min }
        t.playing = playing && t.playable && front
        time = t
        dateMoved()
        if (t.playing, t.value) != was { script?.time(t.value, playing: t.playing, by: mover) }
        guard t.playing else { return }
        playback = Task { [weak self] in
            var last = ContinuousClock.now
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(33))
                guard let self, !Task.isCancelled, var t = self.time, t.playing else { return }
                let now = ContinuousClock.now
                let elapsed = (now - last).components
                // (a pause, a lock or the background is not time played)
                let dt = min(0.1, Double(elapsed.seconds) + Double(elapsed.attoseconds) / 1e18)
                last = now
                guard self.playground?.screen == nil, self.playground?.workbenchShown != true,
                      UIApplication.shared.applicationState == .active else { continue }
                let next = t.advanced(t.value, by: dt)
                if next >= t.max { t.playing = false }
                t.value = next
                self.time = t
                self.dateMoved()
                self.playground?.lastYear = t.year
                self.script?.time(t.value, playing: t.playing, by: .play)
                if !t.playing { return }
            }
        }
    }

    /// The globe's month follows the front stage's date to the day (`GlobeTime.month`: the ground's and
    /// the clouds' season); the playground applies it, and the month it had comes back when no date is shown.
    /// Sprites on dated ways (runtime 5) read the date from here every frame.
    private func dateMoved() {
        syncClocks()
        playground?.globeRequestsChanged()
    }

    /// The date as this stage's sprites read it, from now.
    private var clock: DateClock? { time.map { DateClock(time: $0, stamp: CACurrentMediaTime()) } }

    private func syncClocks() {
        let clock = self.clock
        for marker in sprites.values { marker.clock = clock }
    }

    private func stopPlayback() {
        playback?.cancel()
        playback = nil
        if var t = time, t.playing { t.playing = false; time = t; syncClocks() }
    }

    /// It could not start: say why, and give the reader back the globe without it.
    func fail(_ message: String) {
        guard !ended else { return }
        playground?.problem = message
        end()
    }

    /// Takes away everything it added, and leaves the playground's list. Safe to call more than once.
    func end() {
        guard !ended else { return }
        ended = true
        closeLens()
        for m in markers { globe?.remove(m) }
        markers = []
        if let script {
            script.stop()
            playground?.hidePanel(from: script.pack.id)
        }
        script = nil
        sprites = [:]
        images?.clear()
        if let images { globe?.removeOverlay(images) }
        images = nil
        stopPlayback()
        time = nil
        caption = nil
        geo?.clear()
        if let geo {
            globe?.removeOverlay(geo)
            globe?.accessibleOverlays.removeAll { $0 === geo }
        }
        geo = nil
        requests = GlobeRequests()
        takesTaps = false
        invitation = nil
        invitationAction = nil
        openAction = nil
        canOpen = false
        style = .base
        playground?.stageEnded(self)
    }
}

/// What a lens may change on the globe, as it was when the lens opened (`PrototypeStage.openLens`):
/// its hooks, its modes, its camera and chosen country, what VoiceOver reads of it and the gestures on
/// it. What the lens draws lies in its canvas and leaves with it; the globe's style, room, pause and
/// keep-clear stay the playground's, which a lens never sets.
private struct GlobeLease {
    let authoredMode: Bool
    let historicalReference: Bool
    let labelExclusions: [CGRect]
    let onFrame: (() -> Void)?
    let onUserNavigation: (() -> Void)?
    let onDrawn: (() -> Void)?
    let camera: GlobeCamera
    let selected: Int?
    let label: String?
    let value: String?
    let identifier: String?
    let recognizers: [UIGestureRecognizer]

    init(_ globe: GlobeView) {
        authoredMode = globe.authoredMode
        historicalReference = globe.historicalReference
        labelExclusions = globe.authoredLabelExclusions
        onFrame = globe.onFrame
        onUserNavigation = globe.onUserNavigation
        onDrawn = globe.onDrawn
        // (where a flight is taking it: a debug launch opens the lens while the stage flies to its anchor)
        camera = globe.restingCamera
        selected = globe.selectedIndex
        label = globe.atlasLabel
        value = globe.atlasValue
        identifier = globe.accessibilityIdentifier
        recognizers = globe.gestureRecognizers ?? []
    }

    func restore(_ globe: GlobeView) {
        for g in globe.gestureRecognizers ?? [] where !recognizers.contains(where: { $0 === g }) {
            globe.removeGestureRecognizer(g)
        }
        globe.onFrame = onFrame
        globe.onUserNavigation = onUserNavigation
        globe.onDrawn = onDrawn
        globe.authoredLabelExclusions = labelExclusions
        globe.historicalReference = historicalReference
        globe.setAuthoredCamera(camera)
        globe.selectedIndex = selected
        globe.atlasLabel = label
        globe.atlasValue = value
        globe.setAtlasAccessibilityIdentifier(identifier)
        globe.authoredMode = authoredMode
    }
}
