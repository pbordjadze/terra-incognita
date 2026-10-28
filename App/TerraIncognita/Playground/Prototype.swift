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
        /// A web pack: bundled with the app, downloaded, or only listed by a source so far.
        case pack(PackListing.State, revision: Int, bundled: Bool)
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
    /// What starting it does with the stage. Everything it adds there goes when it ends.
    let start: (PrototypeStage) -> Void
}

/// A full-screen surface over the paused globe: a native experience or a pack's page.
struct PlaygroundScreen: Identifiable {
    let id: String
    let content: () -> AnyView
}

/// An accessible, visible way in for a prototype on the globe, above the caption.
struct Invitation: Equatable {
    let id: String
    let title: String
    let action: String
}

/// One prototype's time on the globe. It is made when the prototype starts and owns everything the
/// prototype adds (markers, an overlay, geography drawn by the globe, a date, words at the foot, a
/// style, a screen, an invitation); `end` takes all of it away, so the plain globe is exactly as it
/// was. The workbench's isolation rule lives here.
@Observable
final class PrototypeStage {
    let prototype: Prototype
    /// The globe's style while it runs (the parked Persian style is a prototype of its own).
    var style: GlobeStyle = .wikipedia
    private(set) var invitation: Invitation?
    /// It ends when its screen closes (an own-screen prototype, or a pack's screen presentation).
    var endsWithScreen: Bool
    private(set) var ended = false
    /// Getting ready (a pack downloading): the chip says so.
    var loading = false
    /// Something here opens a screen: the chip offers Open.
    private(set) var canOpen = false
    /// The globe's date while it runs (runtime 2), and what it says at the foot of the screen.
    private(set) var time: GlobeTime?
    private(set) var caption: StageCaption?

    @ObservationIgnored private weak var playground: Playground?
    @ObservationIgnored private var markers: [GlobeMarker] = []
    @ObservationIgnored private var overlay: WebPackOverlay?
    @ObservationIgnored private var invitationAction: (() -> Void)?
    /// A globe pack's own page (its sources, its longer account), for the chip's Open.
    @ObservationIgnored private var openAction: (() -> Void)?
    @ObservationIgnored private var geo: GeoLayerView?
    @ObservationIgnored private var playback: Task<Void, Never>?
    /// It changed how the globe behaves (hid today's borders, stopped its turn): undone at the end.
    @ObservationIgnored private var globeChanged = false
    private let live = LiveCount.Token(.stages)

    init(prototype: Prototype, playground: Playground) {
        self.prototype = prototype
        self.playground = playground
        endsWithScreen = prototype.ownScreen
    }

    private var globe: GlobeView? { playground?.globe }

    /// Moves the globe (Reduce Motion: at once).
    func fly(to camera: GlobeCamera, duration: Double = 0.9) {
        guard !ended else { return }
        globe?.setAuthoredCamera(camera, duration: UIAccessibility.isReduceMotionEnabled ? 0 : duration)
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
        playground?.screen = PlaygroundScreen(id: id) { AnyView(content()) }
    }

    func invite(_ invitation: Invitation, action: @escaping () -> Void) {
        guard !ended else { return }
        self.invitation = invitation
        invitationAction = action
        canOpen = true
    }

    func acceptInvitation() { invitationAction?() }

    /// What the chip's Open does: the first marker that opens something, else the invitation, else a
    /// globe pack's own page.
    func open() {
        if let open = markers.first(where: { $0.onOpen != nil })?.onOpen { open() }
        else if let invitationAction { invitationAction() }
        else { openAction?() }
    }

    /// A pack on the globe: its anchor, markers, and overlay; or its own screen.
    func start(pack: WebPack) {
        loading = false
        guard !ended else { return }
        let m = pack.manifest
        if let a = m.anchor { fly(to: GlobeCamera(lon: a.lon, lat: a.lat, zoom: a.zoom ?? 1)) }
        if m.presentation != "globe" {
            endsWithScreen = true
            present("pack:" + pack.id) { WebPackScreen(pack: pack) }
            return
        }
        for marker in m.markers ?? [] {
            let image = UIImage(contentsOfFile: pack.root.appending(path: marker.image).path(percentEncoded: false))
            let opens = (marker.opens ?? false) && (m.entry != nil || FileManager.default.fileExists(
                atPath: pack.root.appending(path: "index.html").path(percentEncoded: false)))
            place(GlobeMarker(id: "\(pack.id).\(marker.id)", lon: marker.lon, lat: marker.lat,
                              look: image.map { GlobeMarker.Look.picture($0) } ?? .seal(symbol: "questionmark"),
                              size: CGSize(width: marker.width, height: marker.height), label: marker.label,
                              minZoom: marker.minZoom ?? 0, maxZoom: marker.maxZoom ?? .infinity,
                              onOpen: opens ? { [weak self] in
                                  self?.present("pack:\(pack.id):\(marker.id)") { WebPackScreen(pack: pack, params: ["marker": marker.id]) }
                              } : nil))
        }
        if !canOpen, FileManager.default.fileExists(atPath: pack.root.appending(path: m.entry ?? "index.html").path(percentEncoded: false)) {
            openAction = { [weak self] in self?.present("pack:\(pack.id):page") { WebPackScreen(pack: pack) } }
            canOpen = true
        }
        if m.overlay != nil, let globe {
            let overlay = WebPackOverlay(pack: pack) { [weak self] event in self?.handle(event, pack: pack) }
            self.overlay = overlay
            globe.addOverlay(overlay)
            globe.onFrame = { [weak self, weak overlay, weak globe] in
                guard let globe else { return }
                if let overlay {
                    let s = globe.stageCamera
                    overlay.update(camera: s.camera, centre: s.centre, radius: s.radius)
                }
                self?.geo?.update(globe: globe)
            }
            if m.taps ?? false {
                globe.onWorldTap = { [weak overlay] point, place in
                    overlay?.tap(at: point, place: place)
                    return overlay != nil
                }
            }
        }
    }

    private func handle(_ event: WebPackEvent, pack: WebPack) {
        switch event {
        case .close: playground?.end()
        case .open(let params): present("pack:\(pack.id):open") { WebPackScreen(pack: pack, params: params) }
        case .flyTo(let camera, let duration): fly(to: camera, duration: duration)
        case .log(let line): PackStore.shared.record("\(pack.id): \(line)")
        case .geoSet(let layer):
            if !geoLayers().set(layer) { PackStore.shared.record("\(pack.id): error: geo layer \(layer.id) is over the globe's budget") }
        case .geoRemove(let id): geo?.remove(id)
        case .geoClear: geo?.clear()
        case .timeShow(let time, let carry): show(time, carry: carry)
        case .timeSet(let value): setTime(value, by: nil)
        case .timePlay(let playing): play(playing)
        case .timeHide:
            stopPlayback()
            time = nil
        case .caption(let caption): self.caption = caption
        case .globe(let historical, let still):
            globeChanged = true
            globe?.historicalReference = historical
            globe?.holdsStill = still
        }
    }

    /// The globe's own drawing of what the page asks for, made on first use, under the markers.
    private func geoLayers() -> GeoLayerView {
        if let geo { return geo }
        let view = GeoLayerView()
        geo = view
        globe?.addOverlay(view)
        return view
    }

    // MARK: the globe's date (runtime 2)

    private func show(_ shown: GlobeTime, carry: Bool) {
        guard !ended else { return }
        stopPlayback()
        var t = shown
        let wanted = t.playing
        t.playing = false
        if carry, let year = playground?.lastYear {
            let v = t.value(forYear: year)
            if v >= t.min, v <= t.max { t.value = v }
        }
        time = t
        playground?.lastYear = t.year
        overlay?.time(t.value, playing: false, by: .host)
        if wanted, !UIAccessibility.isReduceMotionEnabled { play(true) }
    }

    /// Moves the date: the reader's drag (which pauses playback), playback's tick, or the page's own
    /// `terra.time.set` (`by` nil: the page knows already).
    func setTime(_ value: Double, by mover: GlobeTime.Mover?) {
        guard !ended, var t = time, value.isFinite else { return }
        if mover == .reader, t.playing { stopPlayback(); t.playing = false }
        t.value = t.clamped(value)
        time = t
        playground?.lastYear = t.year
        if let mover { overlay?.time(t.value, playing: t.playing, by: mover) }
    }

    /// Plays the date forward at its rate, about thirty steps a second, to its end.
    func play(_ playing: Bool) {
        guard !ended, var t = time else { return }
        stopPlayback()
        if playing, t.value >= t.max { t.value = t.min }
        t.playing = playing && t.playable
        time = t
        overlay?.time(t.value, playing: t.playing, by: .reader)
        guard t.playing else { return }
        playback = Task { [weak self] in
            var last = ContinuousClock.now
            while !Task.isCancelled {
                try? await Task.sleep(for: .milliseconds(33))
                guard let self, !Task.isCancelled, var t = self.time, t.playing else { return }
                let now = ContinuousClock.now
                let elapsed = (now - last).components
                let dt = Double(elapsed.seconds) + Double(elapsed.attoseconds) / 1e18
                last = now
                let next = min(t.max, t.value + t.rate * dt)
                if next >= t.max { t.playing = false }
                t.value = next
                self.time = t
                self.playground?.lastYear = t.year
                self.overlay?.time(t.value, playing: t.playing, by: .play)
                if !t.playing { return }
            }
        }
    }

    private func stopPlayback() {
        playback?.cancel()
        playback = nil
        if var t = time, t.playing { t.playing = false; time = t }
    }

    /// It could not start: say why, and give the reader back the plain globe.
    func fail(_ message: String) {
        guard !ended else { return }
        playground?.problem = message
        if playground?.stage === self { playground?.end() } else { end() }
    }

    /// Takes away everything it added. Safe to call more than once.
    func end() {
        guard !ended else { return }
        ended = true
        for m in markers { globe?.remove(m) }
        markers = []
        if let overlay {
            overlay.stop()
            overlay.removeFromSuperview()
            globe?.onFrame = nil
            globe?.onWorldTap = nil
        }
        overlay = nil
        stopPlayback()
        time = nil
        caption = nil
        geo?.clear()
        geo?.removeFromSuperview()
        geo = nil
        if globeChanged {
            globe?.historicalReference = false
            globe?.holdsStill = false
            globeChanged = false
        }
        invitation = nil
        invitationAction = nil
        openAction = nil
        canOpen = false
        style = .wikipedia
    }
}
