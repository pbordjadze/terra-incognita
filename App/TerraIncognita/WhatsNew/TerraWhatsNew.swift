#if UNRELEASED
import Foundation
import Observation
import SwiftUI
import UIKit

// Terra's side of What's new (painting-company row 187): what a piece's `start` means on the globe,
// where the pieces come from (WhatsNew.json beside this file for native work, each pack's own
// `whatsNew` in prototype.json), and the host the shared module drives. The playground is used as a
// reader would use it: prototypes start and end through it, the camera flies, the switches turn;
// nothing is drawn here. docs/whatsnew.md is the contract.

/// Where a Terra piece starts (`start` in a piece; every key optional):
/// `run` the prototypes to start, in order, the last in front; `camera` `{ lon, lat, zoom }` (else the
/// first one's anchor); `date` the front pack's date, in its unit, once it shows one, and `playing`;
/// `wikipedia` and `clouds` the switches; `select` a country lit by name; `open` its panel at once.
struct TerraStart: Equatable {
    var run: [String] = []
    var camera: GlobeCamera?
    var date: Double?
    var playing: Bool?
    var wikipedia: Bool?
    var clouds: Bool?
    var select: String?
    var open = false
    /// The other experiences' doorways stay on the globe (they are hidden while a piece plays, so
    /// nothing else is in the way; the living globe is about them).
    var doorways = false

    init(_ json: JSONValue?) {
        guard let json else { return }
        run = json["run"]?.array?.compactMap(\.string) ?? []
        if let c = json["camera"], let lon = c["lon"]?.double, let lat = c["lat"]?.double {
            camera = GlobeCamera(lon: lon, lat: lat, zoom: c["zoom"]?.double ?? 1)
        }
        date = json["date"]?.double
        playing = json["playing"]?.bool
        wikipedia = json["wikipedia"]?.bool
        clouds = json["clouds"]?.bool
        select = json["select"]?.string
        open = json["open"]?.bool ?? false
        doorways = json["doorways"]?.bool ?? false
    }
}

/// The What's new mode on Terra's playground: the pieces, the session, and the host the session drives.
@Observable
final class TerraWhatsNew: WhatsNewHost {
    static let destination = WhatsNewDestination(repository: "pbordjadze/painting-company", folder: "interviews/paul/terra")
    /// A round is a few pieces, two minutes; the rest wait for the next (the card offers to keep going).
    static let roundSize = 5
    let session: WhatsNewSession
    @ObservationIgnored private weak var playground: Playground?
    /// The pieces for native work, from WhatsNew.json in the bundle.
    @ObservationIgnored private let central: [WhatsNewPiece]
    /// Chosen in the workbench, started once it has closed.
    var pending: (pieces: [WhatsNewPiece], single: Bool)?
    /// The switches as the reader had them before the round, put back after.
    @ObservationIgnored private var switches: (wikipedia: Bool, clouds: Bool)?
    @ObservationIgnored private var launched = false

    init(playground: Playground) {
        self.playground = playground
        var reset = false
        #if DEBUG
        reset = ProcessInfo.processInfo.arguments.contains("-whatsnew-reset")
        #endif
        let store = WhatsNewStore(app: "terra-incognita", destination: Self.destination, reset: reset)
        session = WhatsNewSession(store: store)
        central = Self.loadCentral()
        session.host = self
    }

    var playing: Bool { session.phase == .playing }
    var finished: Bool { session.phase == .finished }

    /// Every piece this build can show: the packs' own (bundled or downloaded, in their manifests) and
    /// the central list's; a pack's wins over a central entry of the same id.
    var pieces: [WhatsNewPiece] {
        let packs = (playground?.packs.listings ?? []).compactMap { listing -> WhatsNewPiece? in
            guard var p = listing.current?.manifest.whatsNew else { return nil }
            p.id = listing.id
            if p.title.isEmpty { p.title = listing.title }
            // (a pack's piece shows the pack unless its start says what to run)
            if p.start?["run"] == nil {
                var start = p.start?.object ?? [:]
                start["run"] = .array([.string(listing.id)])
                p.start = .object(start)
            }
            return p
        }
        return WhatsNewQueue.merged(central: central, packs: packs).filter { piece in
            // (a piece that runs a prototype this build doesn't have waits for the build that does)
            TerraStart(piece.start).run.allSatisfy { id in playground?.prototype(id) != nil }
        }
    }

    var due: [WhatsNewPiece] { WhatsNewQueue.due(pieces, ledger: session.store.ledger) }

    private static func loadCentral() -> [WhatsNewPiece] {
        guard let url = Bundle.main.url(forResource: "WhatsNew", withExtension: "json"),
              let data = try? Data(contentsOf: url) else { return [] }
        struct File: Decodable { var pieces: [WhatsNewPiece] }
        return (try? JSONDecoder().decode(File.self, from: data))?.pieces ?? []
    }

    // MARK: starting a round

    /// Plays these pieces now (the pill, the launch, the list): a round of a few at a time, the rest
    /// waiting (`single`: one chosen piece, alone).
    func play(_ pieces: [WhatsNewPiece], single: Bool = false) {
        guard !pieces.isEmpty else { return }
        let round = single ? pieces : Array(pieces.prefix(Self.roundSize))
        session.waiting = single ? 0 : pieces.count - round.count
        if playground?.workbenchShown == true {
            pending = (round, single)
            playground?.workbenchShown = false
        } else {
            playground?.screen = nil
            session.start(round, single: single)
        }
    }

    /// The card's Keep going: the next few due pieces.
    func keepGoing() {
        session.dismiss()
        play(due)
    }

    /// The workbench closed: a choice made there starts.
    func workbenchDismissed() {
        guard let pending else { return }
        self.pending = nil
        session.start(pending.pieces, single: pending.single)
    }

    #if DEBUG
    /// `-whatsnew` plays every due piece at launch; `-whatsnew-piece <id>` that one (CI's checks).
    func globeDrawn() {
        guard !launched else { return }
        launched = true
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-whatsnew-piece"), i + 1 < args.count {
            let id = args[i + 1]
            if let piece = pieces.first(where: { $0.id == id }) {
                DispatchQueue.main.async { [self] in play([piece], single: true) }
            }
        } else if args.contains("-whatsnew") {
            DispatchQueue.main.async { [self] in play(due) }
        }
    }
    #endif

    // MARK: WhatsNewHost

    func begin(_ piece: WhatsNewPiece) async {
        guard let playground else { return }
        let start = TerraStart(piece.start)
        playground.end()
        if switches == nil { switches = (WikipediaWords.shared.on, CloudsSwitch.shared.on) }
        if let w = start.wikipedia, WikipediaWords.shared.on != w { WikipediaWords.shared.toggle() }
        if let c = start.clouds, CloudsSwitch.shared.on != c { CloudsSwitch.shared.toggle() }
        playground.globe?.clearSelection()
        playground.heldStill = true
        // nothing else in the way: the other doorways go, the clouds thin as they do under an experience
        playground.hostHidesDoorways = !start.doorways
        playground.hostVeils = start.clouds != true
        if start.run.isEmpty {
            if let camera = start.camera { playground.globe?.setAuthoredCamera(camera, duration: 0.9) }
        } else {
            for id in start.run { playground.start(id, entry: .launch, camera: start.camera) }
            if let date = start.date { playground.stage?.preset(date: date, playing: start.playing) }
        }
        if let name = start.select { _ = playground.globe?.select(countryNamed: name) }
        // (the flight lands before the taps are offered, so the reader sees the place, not the way to it)
        try? await Task.sleep(for: .milliseconds(UIAccessibility.isReduceMotionEnabled ? 100 : 900))
        if start.open { playground.stage?.open() }
    }

    func end(_ piece: WhatsNewPiece) {
        guard let playground else { return }
        playground.screen = nil
        playground.end()
        playground.heldStill = false
        playground.hostHidesDoorways = false
        playground.hostVeils = false
        playground.globe?.clearSelection()
    }

    func roundEnded() {
        if let switches {
            if WikipediaWords.shared.on != switches.wikipedia { WikipediaWords.shared.toggle() }
            if CloudsSwitch.shared.on != switches.clouds { CloudsSwitch.shared.toggle() }
        }
        switches = nil
        playground?.heldStill = false
        playground?.hostHidesDoorways = false
        playground?.hostVeils = false
    }

    /// What the reader is looking at: the camera, what runs and which is in front, its date, the
    /// switches, the country lit. Enough for a session to put the globe back there.
    func state() -> JSONValue? {
        guard let playground, let globe = playground.globe else { return nil }
        let camera = globe.restingCamera
        var state: [String: JSONValue] = [
            "camera": .object(["lon": .number((camera.lon * 1000).rounded() / 1000), "lat": .number((camera.lat * 1000).rounded() / 1000),
                               "zoom": .number((camera.zoom * 1000).rounded() / 1000)]),
            "running": .array(playground.stages.map { .string($0.prototype.id) }),
            "wikipedia": .bool(WikipediaWords.shared.on),
            "clouds": .bool(CloudsSwitch.shared.on),
            "month": .number(Double(globe.month)),
        ]
        if let front = playground.stage {
            state["front"] = .string(front.prototype.id)
            if let t = front.time {
                state["date"] = .object(["unit": .string(t.unit.rawValue), "value": .number(t.value), "playing": .bool(t.playing)])
            }
        }
        if let country = globe.selectedCountry { state["selected"] = .string(country.n) }
        return .object(state)
    }

    var canOpen: Bool { playground?.stage?.canOpen ?? false }
    func open() { playground?.stage?.open() }

    /// The window as the reader sees it, the globe's Metal layers made readable for the frame.
    func snapshot() async -> UIImage? {
        guard let globe = playground?.globe, let window = globe.window else { return nil }
        globe.capturable = true
        try? await Task.sleep(for: .milliseconds(150))
        let image = WhatsNewSnapshot.capture(window)
        globe.capturable = false
        return image
    }
}

/// The mode's chrome on the playground: the pill while pieces are due, the bar and the taps while a
/// piece plays (the globe keeps its room between them), the card at the end.
struct WhatsNewChrome: ViewModifier {
    let mode: TerraWhatsNew
    /// Something else has the screen (a lens): the pill waits.
    let quiet: Bool
    /// Where the bar and the band are (global), so the globe keeps its names clear of them.
    @Binding var frames: [CGRect]
    @State private var sendingShown = false
    @State private var barFrame = CGRect.zero
    @State private var bandFrame = CGRect.zero

    private var session: WhatsNewSession { mode.session }

    func body(content: Content) -> some View {
        content
            // (under a lens, the experience on the globe has the screen: the bar and the band wait for it to close)
            .safeAreaInset(edge: .top, spacing: 0) {
                if mode.playing, !quiet {
                    WhatsNewBar(session: session).opacity(session.capturing ? 0 : 1)
                        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { barFrame = $0; frames = [barFrame, bandFrame] }
                }
            }
            .safeAreaInset(edge: .bottom, spacing: 0) {
                if mode.playing, !quiet {
                    WhatsNewReactionRow(session: session).opacity(session.capturing ? 0 : 1)
                        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { bandFrame = $0; frames = [barFrame, bandFrame] }
                }
            }
            .onChange(of: mode.playing) { _, playing in if !playing { barFrame = .zero; bandFrame = .zero; frames = [] } }
            .onChange(of: quiet) { _, quiet in if quiet { barFrame = .zero; bandFrame = .zero; frames = [] } }
            .overlay {
                if mode.finished {
                    ZStack {
                        Color.black.opacity(0.25).ignoresSafeArea().onTapGesture { session.dismiss() }
                        WhatsNewFinishCard(session: session, openSending: { sendingShown = true }, keepGoing: { mode.keepGoing() })
                    }
                    .transition(.opacity)
                }
            }
            .overlay(alignment: .top) {
                // the pill: under the controls' row, clear of the iPad's window controls at the top left
                if !mode.playing, !mode.finished, !quiet, !mode.due.isEmpty {
                    WhatsNewPill(count: mode.due.count) { mode.play(mode.due) }
                        .padding(.top, 68)
                        .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.25), value: mode.playing)
            .animation(.easeInOut(duration: 0.25), value: mode.finished)
            .sheet(isPresented: $sendingShown) {
                NavigationStack { WhatsNewSendingView(session: session) }
                    .presentationDetents([.large])
                    .preferredColorScheme(.light)
            }
    }
}

#endif
