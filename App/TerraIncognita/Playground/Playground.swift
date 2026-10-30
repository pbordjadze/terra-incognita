import Foundation
import Observation
import SwiftUI
import UIKit

/// The playground: the globe with every prototype's doorway on it, the running prototypes (their
/// stages, the one in front last), and the screen or panel over the globe when one is open. A
/// doorway or the workbench starts one beside the others; this starts, ends and keeps the globe's
/// merged state, so that ending any one returns the globe exactly as it was without it (runtime 4,
/// docs/runtime3.md › Many at once).
@Observable
final class Playground {
    /// The terra.js version this app hosts (Prototypes/_runtime/terra.js; docs/playground.md).
    static let runtime = PackStore.runtime
    /// The most experiences that run at once: starting another ends the one the reader left longest ago.
    static let stageBudget = 6
    let packs = PackStore.shared
    /// The running prototypes, the one in front (the reader's) last.
    private(set) var stages: [PrototypeStage] = []
    /// The prototype in front: the chip's, the foot's, the gesture scheme's.
    var stage: PrototypeStage? { stages.last }
    var screen: PlaygroundScreen?
    /// Whose screen is open (it may end with it).
    @ObservationIgnored private weak var screenOwner: PrototypeStage?
    var workbenchShown = false
    /// The date the last time-aware prototype left (as a year with its fraction), for one that carries it.
    @ObservationIgnored var lastYear: Double?
    /// The last prototype started, for the workbench's "Again".
    private(set) var recent: String?
    /// Something that went wrong starting a prototype, shown until dismissed.
    var problem: String?
    /// A source a link proposed, waiting for the reader's yes.
    var proposedSource: URL?
    /// A pack's longer words and sources, over the globe (runtime 3, `terra.panel`).
    var panel: PackPanel?
    /// The quiet frame (a running prototype asked): the caption gone at rest.
    private(set) var quietFrame = false
    /// The host (What's new) holds the globe where it put it, no idle turn, while a piece plays.
    var heldStill = false { didSet { if heldStill != oldValue { applyGlobeState() } } }
    /// An experience is running on the globe (or the host asks): the clouds and the sea give way to it.
    var veiled: Bool { hostVeils || stages.contains { $0.prototype.veils } }
    /// The host (What's new) thins the clouds and the sea while a piece plays, and takes the other
    /// experiences' doorways off the globe, so nothing else is in the way.
    var hostVeils = false
    var hostHidesDoorways = false { didSet { if hostHidesDoorways != oldValue { refreshDoorways() } } }
    @ObservationIgnored private var panelClosed: (() -> Void)?
    /// Every prototype's doorway on the globe while it isn't running, by prototype id.
    @ObservationIgnored private var doorways: [String: GlobeMarker] = [:]
    @ObservationIgnored private(set) weak var globe: GlobeView?
    /// The globe's month before any stage's date moved it.
    @ObservationIgnored private var baseMonth: Int?
    /// Whose drag on the world is in progress, and what it began on.
    @ObservationIgnored private weak var dragStage: PrototypeStage?
    @ObservationIgnored private var dragTarget: [String: Any]?

    private enum Choice { case start(String, StageEntry), plainGlobe }
    /// A choice made while the workbench or a screen was showing: UIKit finishes dismissing it first.
    @ObservationIgnored private var pending: Choice?
    @ObservationIgnored private var globeReady = false
    @ObservationIgnored private var waiting: [() -> Void] = []
    @ObservationIgnored private var launched = false

    /// Natives first, then the packs (bundled, downloaded, listed by a source).
    var prototypes: [Prototype] { Catalog.native + packs.listings.map(Self.prototype(for:)) }

    func prototype(_ id: String) -> Prototype? { prototypes.first { $0.id == id } }

    func isRunning(_ id: String) -> Bool { stages.contains { $0.prototype.id == id } }

    /// The globe's style: the front-most running prototype that sets one, else the plain globe's.
    var style: GlobeStyle { stages.last { $0.style != .base }?.style ?? .base }

    // MARK: choosing

    /// From the workbench: it closes, then the prototype starts (or comes to the front).
    func choose(_ id: String) { request(.start(id, .workbench)) }
    func choosePlainGlobe() { request(.plainGlobe) }

    private func request(_ choice: Choice) {
        if workbenchShown {
            pending = choice
            workbenchShown = false
        } else if screen != nil {
            pending = choice
            screen = nil
        } else {
            run(choice)
        }
    }

    private func run(_ choice: Choice) {
        switch choice {
        case .start(let id, let entry): start(id, entry: entry)
        case .plainGlobe: end()
        }
    }

    func workbenchDismissed() {
        // (the workbench may have downloaded packs: their doorways join the globe)
        refreshDoorways()
        guard let choice = pending else { return }
        pending = nil
        run(choice)
    }

    func screenClosed() {
        let owner = screenOwner
        screenOwner = nil
        if let choice = pending {
            pending = nil
            run(choice)
        } else if let owner, owner.endsWithScreen {
            owner.end()
        }
    }

    // MARK: starting and ending

    /// Starts a prototype beside the ones running (one already running comes to the front instead).
    /// `camera`: where to fly instead of its anchor (What's new starts a piece where its makers meant).
    func start(_ id: String, entry: StageEntry = .workbench, camera: GlobeCamera? = nil) {
        guard let prototype = prototype(id) else {
            problem = String(localized: "There is no prototype called “\(id)” here.")
            return
        }
        whenGlobeReady { [self] in
            if let running = stages.first(where: { $0.prototype.id == id }) {
                bringToFront(running)
                return
            }
            // room for it: the one the reader left longest ago goes, never the one in front
            while stages.count >= Self.stageBudget, let oldest = stages.first {
                packs.record("\(oldest.prototype.id): ended to make room (at most \(Self.stageBudget) experiences run at once)")
                oldest.end()
            }
            globe?.clearSelection()
            let stage = PrototypeStage(prototype: prototype, playground: self, entry: entry, at: globe?.restingCamera)
            stage.cameraOverride = camera
            stages.last?.setFront(false, by: "host")
            stages.append(stage)
            recent = id
            problem = nil
            // held where it flew (or where the reader touched it), so its subject isn't carried off by the idle turn
            if prototype.anchor != nil || camera != nil || entry == .doorway { stage.holdStill() }
            if entry != .doorway, let anchor = camera ?? prototype.anchor { stage.fly(to: anchor) }
            prototype.start(stage)
            // a native prototype on the globe says at the foot what it is and what to touch
            if prototype.made == .native, prototype.doorway != nil {
                stage.describe(title: prototype.title, text: prototype.summary, wikipedia: prototype.wikipedia)
            }
            refreshDoorways()
            applyGlobeState()
        }
    }

    /// Back to the atlas: every stage takes away everything its prototype added.
    func end() {
        for stage in stages.reversed() { stage.end() }
        if screen != nil { screen = nil }
        refreshDoorways()
    }

    /// The chip's ✕: the prototype in front ends; the next one is in front.
    func endFront() { stages.last?.end() }

    /// Ends one prototype if it runs (the workbench switching a pack's revision, or removing its
    /// downloads: the running copy is the old one, and the next start opens the chosen one).
    func end(_ id: String) { stages.first { $0.prototype.id == id }?.end() }

    /// A stage has ended (`PrototypeStage.end`): off the list, its screen closed, the next in front.
    func stageEnded(_ stage: PrototypeStage) {
        guard let i = stages.firstIndex(where: { $0 === stage }) else { return }
        let wasFront = i == stages.count - 1
        stages.remove(at: i)
        if screenOwner === stage {
            screenOwner = nil
            if screen != nil { screen = nil }
        }
        if wasFront, let next = stages.last { next.setFront(true, by: "host") }
        refreshDoorways()
        applyGlobeState()
    }

    /// The reader is with this prototype now: it comes to the front (its date and caption at the foot,
    /// its scheme the globe's); the one that was there hears it is in the background.
    func bringToFront(_ stage: PrototypeStage, by: String = "reader") {
        guard let i = stages.firstIndex(where: { $0 === stage }), i != stages.count - 1 else { return }
        let was = stages.last
        stages.remove(at: i)
        stages.append(stage)
        was?.setFront(false, by: by)
        stage.setFront(true, by: by)
        applyGlobeState()
    }

    /// One lens at a time: the others' close.
    func closeLenses(except stage: PrototypeStage) {
        for other in stages where other !== stage { other.closeLens() }
    }

    /// A stage opens a screen over the paused globe.
    func present(_ screen: PlaygroundScreen, from stage: PrototypeStage) {
        screenOwner = stage
        self.screen = screen
    }

    // MARK: the globe's merged state

    /// A stage asked something of the globe, or its date moved: the globe takes the merged state.
    func globeRequestsChanged() { applyGlobeState() }

    /// What the globe does for all of them: still, the sea hidden and the quiet rules while any stage
    /// asks, the deepest zoom any asks for, and the front stage's scheme, historical reference and month.
    private func applyGlobeState() {
        guard let globe else { return }
        let front = stages.last
        globe.holdsStill = heldStill || stages.contains { $0.requests.still }
        globe.seaHidden = stages.contains { $0.requests.seaHidden }
        globe.quietNames = stages.contains { $0.requests.quietNames }
        globe.cloudsQuiet = stages.contains { $0.requests.quietClouds }
        let frame = stages.contains { $0.requests.quietFrame }
        if quietFrame != frame { quietFrame = frame }
        globe.historicalReference = front?.requests.historical ?? false
        globe.dragScheme = front?.requests.scheme ?? .touchables
        globe.zoomLimit = stages.compactMap { $0.requests.maxZoom }.max() ?? 8
        if baseMonth == nil { baseMonth = globe.month }
        let month = front?.time?.month ?? baseMonth ?? globe.month
        if globe.month != month { globe.month = month }
    }

    // MARK: touches on the world

    /// Each globe frame: every stage's script hears the camera, and what the globe draws for them follows.
    private func frame() {
        for stage in stages { stage.frame() }
    }

    /// A tap on the globe: the front-most stage with a touchable feature under it takes it (and comes
    /// to the front); else the front stage, if it takes every tap; else a country is chosen.
    private func worldTap(_ point: CGPoint, _ place: (lon: Double, lat: Double)?) -> Bool {
        guard let globe else { return false }
        for stage in stages.reversed() {
            if let hit = stage.geoHit(point, globe: globe) {
                bringToFront(stage)
                stage.touch("tap", target: ["layer": hit.layer, "index": hit.index], at: point, place: place)
                return true
            }
        }
        if let front = stages.last, front.takesTaps {
            front.touch("tap", target: nil, at: point, place: place)
            return true
        }
        return false
    }

    /// Whether a one-finger drag beginning here is a stage's (it began on something touchable of its).
    private func dragBegins(_ point: CGPoint) -> Bool {
        dragStage = nil
        dragTarget = nil
        guard let globe else { return false }
        for stage in stages.reversed() {
            if let hit = stage.geoHit(point, globe: globe) {
                dragStage = stage
                dragTarget = ["layer": hit.layer, "index": hit.index]
                return true
            }
        }
        return false
    }

    /// A drag that is a stage's: the one it began on, or (the one-finger scheme) the front stage's.
    private func worldDrag(_ phase: String, _ point: CGPoint, _ place: (lon: Double, lat: Double)?) {
        if phase == "began" {
            if dragStage == nil, let globe, let front = stages.last {
                dragStage = front
                dragTarget = front.geoHit(point, globe: globe).map { ["layer": $0.layer, "index": $0.index] }
            }
            if let stage = dragStage { bringToFront(stage) }
        }
        dragStage?.touch(phase, target: dragTarget, at: point, place: place)
        if phase == "ended" {
            dragStage = nil
            dragTarget = nil
        }
    }

    // MARK: the panel

    /// A stage's panel: shown while it is the one in front (a background experience's would take the
    /// screen from the reader; a touch on its things brings it to the front first).
    func show(_ panel: PackPanel, from stage: PrototypeStage, onClose: @escaping () -> Void) {
        guard stage === stages.last else {
            packs.record("\(panel.pack): its panel is not shown while the reader is with another experience")
            return
        }
        panelClosed = onClose
        self.panel = panel
    }

    /// The pack's script hid its panel, or the pack ended.
    func hidePanel(from pack: String) {
        guard panel?.pack == pack else { return }
        panelClosed = nil
        panel = nil
    }

    func panelDismissed() {
        let closed = panelClosed
        panelClosed = nil
        closed?()
    }

    // MARK: doorways: the atlas's things on the one globe

    /// Places every current prototype's doorway, except the running ones' (their stages show them).
    /// Touching one starts its prototype beside the others; a native one opens straight away.
    func refreshDoorways() {
        guard let globe else { return }
        var wanted: [String: Prototype.Doorway] = [:]
        for p in Catalog.native where p.shelf == .current { if let d = p.doorway { wanted[p.id] = d } }
        for listing in packs.listings where listing.shelf != "parked" {
            if let pack = listing.current, let d = Self.doorway(of: pack) { wanted[listing.id] = d }
        }
        for stage in stages { wanted[stage.prototype.id] = nil }
        if hostHidesDoorways { wanted = [:] }
        for (id, marker) in doorways where wanted[id] == nil {
            globe.remove(marker)
            doorways[id] = nil
        }
        for (id, door) in wanted where doorways[id] == nil {
            let marker = GlobeMarker(sprite: "doorway.\(id)", at: (door.lon, door.lat), path: door.path, looks: door.looks,
                                     label: door.label, hint: String(localized: "Starts it"), counted: false,
                                     breathes: door.breathes) { [weak self] in
                self?.enter(id)
            }
            doorways[id] = marker
            globe.add(marker)
        }
    }

    /// A doorway was touched: its prototype starts where the reader is.
    private func enter(_ id: String) {
        request(.start(id, .doorway))
        // a native prototype's doorway opens it (its experience is the point of touching it)
        if let stage = stages.last, stage.prototype.id == id, case .native = stage.prototype.made, stage.canOpen { stage.open() }
    }

    /// A pack's doorway from its manifest, its pictures read from the pack: its looks by zoom, the way
    /// it stirs along, its breath (runtime 4).
    private static func doorway(of pack: PlaygroundPack) -> Prototype.Doorway? {
        guard let d = pack.manifest.doorway else { return nil }
        let looks = (d.looks ?? d.look.map { [$0] } ?? []).prefix(8).compactMap { look -> GlobeMarker.SpriteLook? in
            let images = look.pictures.prefix(24).compactMap { name -> UIImage? in
                guard PackRules.isSafePath(name) else { return nil }
                return UIImage(contentsOfFile: pack.root.appending(path: name).path(percentEncoded: false))
            }
            guard !images.isEmpty else { return nil }
            let size = look.size ?? [28, 28]
            return GlobeMarker.SpriteLook(
                frames: images, fps: min(30, max(0.2, look.fps ?? 6)),
                size: CGSize(width: min(200, max(4, size.first ?? 28)), height: min(200, max(4, size.dropFirst().first ?? size.first ?? 28))),
                minZoom: look.minZoom ?? 0, maxZoom: look.maxZoom ?? .infinity,
                every: look.every.map { max(0.2, $0) }, duration: look.for.map { max(0.05, $0) }, face: look.face ?? false)
        }
        guard !looks.isEmpty else { return nil }
        var path: GlobeMarker.Path?
        if let p = d.path {
            let points = p.coordinates.prefix(200).filter { $0.count >= 2 && abs($0[1]) <= 90 }.map { (lon: $0[0], lat: $0[1]) }
            if points.count > 1 {
                path = GlobeMarker.Path(points: points, seconds: max(1, p.seconds ?? 60), loop: p.loop?.marker ?? .repeat)
            }
        }
        return Prototype.Doorway(lon: d.lon, lat: d.lat, label: String(d.label.prefix(120)), looks: looks,
                                 path: path, breathes: d.breathes ?? false)
    }

    // MARK: the globe

    /// The globe has drawn its first frame: prototypes can start on it, the doorways go on it, and its
    /// touches on the world come here to be routed.
    func globeDrawn(_ view: GlobeView) {
        globe = view
        globeReady = true
        baseMonth = view.month
        view.onFrame = { [weak self] in self?.frame() }
        view.onWorldTap = { [weak self] point, place in self?.worldTap(point, place) ?? false }
        view.worldDragBegins = { [weak self] point in self?.dragBegins(point) ?? false }
        view.onWorldDrag = { [weak self] phase, point, place in self?.worldDrag(phase, point, place) }
        refreshDoorways()
        let ready = waiting
        waiting = []
        ready.forEach { $0() }
        #if DEBUG
        if !launched {
            launched = true
            // after this frame is on screen, so a debug launch's screen opens over a drawn globe
            DispatchQueue.main.async { [self] in launchFromArguments() }
        }
        #endif
    }

    private func whenGlobeReady(_ action: @escaping () -> Void) {
        if globeReady { action() } else { waiting.append(action) }
    }

    // MARK: links

    /// `terraincognita://open/<id>` starts a prototype (a pack is looked for in the sources first);
    /// `terraincognita://source?url=<catalog>` proposes a source, which the reader must accept.
    func handle(_ url: URL) {
        #if WORKBENCH
        guard url.scheme == "terraincognita" else { return }
        switch url.host() {
        case "open":
            guard let id = url.pathComponents.dropFirst().first, !id.isEmpty else { return }
            Task {
                if !Catalog.native.contains(where: { $0.id == id }) { await packs.refresh(force: true) }
                request(.start(id, .link))
            }
        case "source":
            let value = URLComponents(url: url, resolvingAgainstBaseURL: false)?.queryItems?.first { $0.name == "url" }?.value
            proposedSource = value.flatMap(URL.init(string:))
        default:
            break
        }
        #endif
    }

    // MARK: packs as prototypes

    private static func prototype(for listing: PackListing) -> Prototype {
        let revision = listing.current?.revision ?? listing.remote?.revision ?? 0
        return Prototype(id: listing.id, title: listing.title, summary: listing.summary,
                         shelf: listing.shelf == "parked" ? .parked : .current,
                         made: .pack(listing.state, revision: revision, bundled: listing.bundled)) { stage in
            stage.loading = true
            Task {
                do {
                    let pack = try await PackStore.shared.prepare(listing.id)
                    stage.start(pack: pack)
                } catch {
                    stage.loading = false
                    stage.fail(String(localized: "\(listing.title) could not open: \(error.localizedDescription)"))
                }
            }
        }
    }

    // MARK: debug launches (CI's tests and screenshots)

    #if DEBUG
    /// `-prototype <id>` starts one (`-prototype-open`: and opens it); `-workbench` opens the
    /// workbench. The older flags still work: `-camelid <s>`/`-camelid-play`, `-look <id>`/`-film <s>`,
    /// `-globeStyle persian` (or `wikipedia`).
    private func launchFromArguments() {
        let args = ProcessInfo.processInfo.arguments
        func value(_ flag: String) -> String? {
            guard let i = args.firstIndex(of: flag), i + 1 < args.count else { return nil }
            return args[i + 1]
        }
        var id = value("-prototype")
        var open = args.contains("-prototype-open")
        if id == nil {
            if args.contains("-camelid") || args.contains("-camelid-play") { id = "camelid"; open = true }
            else if args.contains("-look") || args.contains("-film") { id = "film" }
            else if value("-globeStyle") == "persian" { id = "persian" }
            else if value("-globeStyle") == "wikipedia" { id = "wikipedia" }
        }
        if let id {
            start(id, entry: .launch)
            if open { stage?.open() }
        }
        if args.contains("-workbench") { workbenchShown = true }
    }
    #endif
}
