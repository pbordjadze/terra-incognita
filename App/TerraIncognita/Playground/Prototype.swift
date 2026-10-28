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
/// prototype adds (markers, an overlay, a style, a screen, an invitation); `end` takes all of it
/// away, so the plain globe is exactly as it was. The workbench's isolation rule lives here.
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

    @ObservationIgnored private weak var playground: Playground?
    @ObservationIgnored private var markers: [GlobeMarker] = []
    @ObservationIgnored private var overlay: WebPackOverlay?
    @ObservationIgnored private var invitationAction: (() -> Void)?
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

    /// What a native prototype draws over the globe in step with it (`GlobeFollower`), between the
    /// globe's drawing and its names; taken away at the end.
    @ObservationIgnored private var followers: [any GlobeFollower] = []

    func follow(_ follower: any GlobeFollower) {
        guard !ended, let globe else { return }
        followers.append(follower)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        follower.layer.frame = globe.bounds
        // (first: over the globe's drawing, which is its layer's own contents, and under the names)
        globe.layer.insertSublayer(follower.layer, at: 0)
        CATransaction.commit()
        globe.onFrame = { [weak self, weak globe] in
            guard let self, let globe else { return }
            for f in self.followers { f.frame(of: globe) }
        }
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

    /// What the chip's Open does: the first marker that opens something.
    func open() {
        if let open = markers.first(where: { $0.onOpen != nil })?.onOpen { open() } else { invitationAction?() }
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
        if m.overlay != nil, let globe {
            let overlay = WebPackOverlay(pack: pack) { [weak self] event in self?.handle(event, pack: pack) }
            self.overlay = overlay
            globe.addOverlay(overlay)
            globe.onFrame = { [weak overlay, weak globe] in
                guard let overlay, let globe else { return }
                let s = globe.stageCamera
                overlay.update(camera: s.camera, centre: s.centre, radius: s.radius)
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
        }
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
        for f in followers { f.stop(); f.layer.removeFromSuperlayer() }
        if !followers.isEmpty { globe?.onFrame = nil }
        followers = []
        if let overlay {
            overlay.stop()
            overlay.removeFromSuperview()
            globe?.onFrame = nil
            globe?.onWorldTap = nil
        }
        overlay = nil
        invitation = nil
        invitationAction = nil
        canOpen = false
        style = .wikipedia
    }
}
