import Foundation
import Observation
import SwiftUI
import UIKit

/// The playground: the globe with every prototype's doorway on it, at most one active prototype (its
/// stage), and the screen or panel over the globe when one is open. A doorway or the workbench
/// chooses; this starts and ends, so that ending anything returns the globe as it was. One running at
/// a time is the workbench's rule for clean comparison, not a decision about the finished atlas.
@Observable
final class Playground {
    /// The terra.js version this app hosts (Prototypes/_runtime/terra.js; docs/playground.md).
    static let runtime = PackStore.runtime
    let packs = PackStore.shared
    private(set) var stage: PrototypeStage?
    var screen: PlaygroundScreen?
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
    @ObservationIgnored private var panelClosed: (() -> Void)?
    /// Every prototype's doorway on the globe while it isn't running, by prototype id.
    @ObservationIgnored private var doorways: [String: GlobeMarker] = [:]
    @ObservationIgnored private(set) weak var globe: GlobeView?

    private enum Choice { case start(String), plainGlobe }
    /// A choice made while the workbench or a screen was showing: UIKit finishes dismissing it first.
    @ObservationIgnored private var pending: Choice?
    @ObservationIgnored private var globeReady = false
    @ObservationIgnored private var waiting: [() -> Void] = []
    @ObservationIgnored private var launched = false

    /// Natives first, then the packs (bundled, downloaded, listed by a source).
    var prototypes: [Prototype] { Catalog.native + packs.listings.map(Self.prototype(for:)) }

    func prototype(_ id: String) -> Prototype? { prototypes.first { $0.id == id } }

    // MARK: choosing

    /// From the workbench: it closes, then the prototype starts.
    func choose(_ id: String) { request(.start(id)) }
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
        case .start(let id): start(id)
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
        if let choice = pending {
            pending = nil
            run(choice)
        } else if stage?.endsWithScreen == true {
            end()
        }
    }

    // MARK: starting and ending

    func start(_ id: String) {
        guard let prototype = prototype(id) else {
            problem = String(localized: "There is no prototype called “\(id)” here.")
            return
        }
        whenGlobeReady { [self] in
            end()
            let stage = PrototypeStage(prototype: prototype, playground: self)
            self.stage = stage
            recent = id
            problem = nil
            if let anchor = prototype.anchor { stage.fly(to: anchor) }
            prototype.start(stage)
            refreshDoorways()
        }
    }

    /// Back to the atlas: the stage takes away everything the prototype added.
    func end() {
        stage?.end()
        stage = nil
        if screen != nil { screen = nil }
        refreshDoorways()
    }

    // MARK: the panel

    func show(_ panel: PackPanel, onClose: @escaping () -> Void) {
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

    /// Places every current prototype's doorway, except the running one's (its stage shows it).
    /// Touching one starts its prototype; a native one opens straight away.
    func refreshDoorways() {
        guard let globe else { return }
        var wanted: [String: Prototype.Doorway] = [:]
        for p in Catalog.native where p.shelf == .current { if let d = p.doorway { wanted[p.id] = d } }
        for listing in packs.listings where listing.shelf != "parked" {
            if let pack = listing.current, let d = Self.doorway(of: pack) { wanted[listing.id] = d }
        }
        if let running = stage?.prototype.id { wanted[running] = nil }
        for (id, marker) in doorways where wanted[id] == nil {
            globe.remove(marker)
            doorways[id] = nil
        }
        for (id, door) in wanted where doorways[id] == nil {
            let marker = GlobeMarker(sprite: "doorway.\(id)", at: (door.lon, door.lat), path: nil, looks: door.looks,
                                     label: door.label, hint: String(localized: "Starts it"), counted: false) { [weak self] in
                self?.enter(id)
            }
            doorways[id] = marker
            globe.add(marker)
        }
    }

    /// A doorway was touched.
    private func enter(_ id: String) {
        request(.start(id))
        // a native prototype's doorway opens it (its experience is the point of touching it)
        if let stage, stage.prototype.id == id, case .native = stage.prototype.made, stage.canOpen { stage.open() }
    }

    /// A pack's doorway from its manifest, its pictures read from the pack.
    private static func doorway(of pack: PlaygroundPack) -> Prototype.Doorway? {
        guard let d = pack.manifest.doorway else { return nil }
        let images = d.look.pictures.prefix(24).compactMap { name -> UIImage? in
            guard PackRules.isSafePath(name) else { return nil }
            return UIImage(contentsOfFile: pack.root.appending(path: name).path(percentEncoded: false))
        }
        guard !images.isEmpty else { return nil }
        let size = d.look.size ?? [28, 28]
        let look = GlobeMarker.SpriteLook(
            frames: images, fps: min(30, max(0.2, d.look.fps ?? 6)),
            size: CGSize(width: min(200, max(4, size.first ?? 28)), height: min(200, max(4, size.dropFirst().first ?? size.first ?? 28))),
            minZoom: d.look.minZoom ?? 0, maxZoom: d.look.maxZoom ?? .infinity,
            every: d.look.every.map { max(0.2, $0) }, duration: d.look.for.map { max(0.05, $0) }, face: false)
        return Prototype.Doorway(lon: d.lon, lat: d.lat, label: String(d.label.prefix(120)), looks: [look])
    }

    // MARK: the globe

    /// The globe has drawn its first frame: prototypes can start on it, and the doorways go on it.
    func globeDrawn(_ view: GlobeView) {
        globe = view
        globeReady = true
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
                request(.start(id))
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
    /// workbench. The older flags still work: `-camelid <s>`/`-camelid-play`, `-whale <s>`/
    /// `-whale-play`, `-leaf`, `-look <id>`/`-film <s>`, `-globeStyle persian`.
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
            else if args.contains("-whale") || args.contains("-whale-play") { id = "whale"; open = true }
            else if args.contains("-leaf") { id = "leaf" }
            else if args.contains("-look") || args.contains("-film") { id = "film" }
            else if value("-globeStyle") == "persian" { id = "persian" }
        }
        if let id {
            start(id)
            if open { stage?.open() }
        }
        if args.contains("-workbench") { workbenchShown = true }
    }
    #endif
}
