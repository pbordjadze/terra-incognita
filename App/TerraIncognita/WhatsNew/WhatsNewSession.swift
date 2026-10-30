#if UNRELEASED
import Foundation
import Observation
import UIKit

/// What the host app does for a round: puts itself where a piece begins, takes the piece away, says
/// what the reader is looking at, and makes a picture of it. Terra's is `TerraWhatsNewHost`; another
/// app writes its own against the same six calls, and the rest of the module is shared.
protocol WhatsNewHost: AnyObject {
    /// Puts the app where the piece begins (its `start`), with nothing else in the way.
    func begin(_ piece: WhatsNewPiece) async
    /// Takes the piece away.
    func end(_ piece: WhatsNewPiece)
    /// The round is over: the app as the reader had it before (its switches put back).
    func roundEnded()
    /// What the reader is looking at now, as the answer records it.
    func state() -> JSONValue?
    /// Something of the piece to open (a panel, a page), if any.
    var canOpen: Bool { get }
    func open()
    /// A picture of the screen as the reader sees it (the module hides its own chrome first).
    func snapshot() async -> UIImage?
}

/// One round with a build: the pieces due, played in turn, each answered with a tap (and a voice note
/// or a picture when wanted), the answer written to the store at once so nothing is lost by leaving
/// early. `WhatsNewView` is its chrome.
@Observable
final class WhatsNewSession {
    enum Phase: Equatable { case idle, playing, finished }

    let store: WhatsNewStore
    let recorder = WhatsNewRecorder()
    @ObservationIgnored weak var host: (any WhatsNewHost)?
    private(set) var phase = Phase.idle
    /// This round's pieces, in order, and where we are.
    private(set) var pieces: [WhatsNewPiece] = []
    private(set) var index = 0
    /// Between pieces (the next one is being put in place): the chrome waits.
    private(set) var changing = false
    /// The current piece's voice note and picture, when made.
    private(set) var voice: URL?
    private(set) var screenshot: URL?
    /// The last picture, shown a moment as a thumbnail.
    private(set) var lastPicture: UIImage?
    /// The screen is being photographed: the chrome hides itself for the frame.
    private(set) var capturing = false
    /// This round's answers, in order (the finish card's tally).
    private(set) var answers: [WhatsNewAnswer] = []
    /// Something the reader should know (the microphone refused, say), until dismissed.
    var notice: String?
    /// Sending to the private repository: how far, and what it said.
    private(set) var sending = false
    private(set) var sendProgress: (done: Int, of: Int) = (0, 0)
    private(set) var sendResult: String?
    private(set) var roundID = ""
    @ObservationIgnored private var began = Date()
    @ObservationIgnored private var appeared = Date()
    /// What this round is: every due piece, or one chosen from the list.
    private(set) var single = false

    init(store: WhatsNewStore) {
        self.store = store
    }

    var current: WhatsNewPiece? { phase == .playing && index < pieces.count ? pieces[index] : nil }
    var canGoBack: Bool { phase == .playing && index > 0 }
    var tally: [WhatsNewReaction: Int] { answers.reduce(into: [:]) { $0[$1.reaction, default: 0] += 1 } }

    // MARK: the round

    /// Begins a round with these pieces (the first starts now).
    func start(_ pieces: [WhatsNewPiece], single: Bool = false) {
        guard !pieces.isEmpty, phase != .playing else { return }
        self.pieces = pieces
        self.single = single
        index = 0
        answers = []
        sendResult = nil
        began = Date()
        roundID = WhatsNewStore.stamp(began)
        phase = .playing
        Task { await show(0) }
    }

    /// Puts the piece at `i` in place (the one before is taken away).
    private func show(_ i: Int) async {
        guard phase == .playing, i >= 0, i < pieces.count else { return }
        changing = true
        recorder.discard()
        voice = nil
        screenshot = nil
        lastPicture = nil
        if index != i, index < pieces.count { host?.end(pieces[index]) }
        index = i
        await host?.begin(pieces[i])
        appeared = Date()
        changing = false
    }

    /// The tap: the answer is written, and the next piece comes (or the round ends).
    func react(_ reaction: WhatsNewReaction) {
        guard let piece = current, !changing else { return }
        if recorder.recording, let url = recorder.stop() { voice = url }
        let now = Date()
        let answer = WhatsNewAnswer(
            app: store.app, build: WhatsNewStore.build, appVersion: WhatsNewStore.appVersion,
            device: WhatsNewStore.device, os: WhatsNewStore.os,
            piece: piece.id, pieceVersion: piece.versionOrDefault, title: piece.title, ask: piece.ask, row: piece.row,
            reaction: reaction, at: WhatsNewStore.iso(now), seconds: (now.timeIntervalSince(appeared) * 10).rounded() / 10,
            voice: voice?.lastPathComponent, screenshot: screenshot?.lastPathComponent,
            state: host?.state(), round: roundID)
        store.save(answer, stamp: WhatsNewStore.stamp(now))
        store.record(piece, reaction, at: answer.at)
        answers.removeAll { $0.piece == piece.id }
        answers.append(answer)
        UIImpactFeedbackGenerator(style: reaction == .skip ? .light : .medium).impactOccurred()
        Task {
            // (a beat, so the tap is seen to land before the scene changes)
            try? await Task.sleep(for: .milliseconds(reaction == .skip ? 150 : 450))
            if index + 1 < pieces.count { await show(index + 1) } else { finish() }
        }
    }

    /// The piece before, again (its answer stands until given again).
    func back() {
        guard canGoBack, !changing else { return }
        Task { await show(index - 1) }
    }

    /// Leaves the round where it is: what was answered stays answered, the rest stays due.
    func leave() {
        guard phase == .playing else { return }
        recorder.discard()
        if index < pieces.count { host?.end(pieces[index]) }
        host?.roundEnded()
        phase = .idle
        pieces = []
    }

    private func finish() {
        guard phase == .playing else { return }
        if index < pieces.count { host?.end(pieces[index]) }
        host?.roundEnded()
        phase = .finished
    }

    /// The finish card is closed.
    func dismiss() {
        phase = .idle
        pieces = []
    }

    // MARK: the voice note and the picture

    func toggleRecording() {
        guard let piece = current else { return }
        if recorder.recording {
            if let url = recorder.stop() { voice = url }
            return
        }
        if let old = voice { try? FileManager.default.removeItem(at: old); voice = nil }
        let url = store.folder(round: roundID).appending(path: "\(WhatsNewStore.stamp())-\(piece.id).m4a")
        Task {
            let started = await recorder.start(to: url)
            if !started, let why = recorder.problem { notice = why }
        }
    }

    func takePicture() {
        guard let piece = current, let host, !capturing else { return }
        Task {
            capturing = true
            // (two frames, so the chrome is gone from the screen before it is drawn)
            try? await Task.sleep(for: .milliseconds(80))
            let image = await host.snapshot()
            capturing = false
            guard let image else { notice = "The screen could not be photographed."; return }
            if let old = screenshot { try? FileManager.default.removeItem(at: old) }
            let url = store.folder(round: roundID).appending(path: "\(WhatsNewStore.stamp())-\(piece.id).jpg")
            if WhatsNewSnapshot.write(image, to: url) {
                screenshot = url
                lastPicture = image
                UIImpactFeedbackGenerator(style: .rigid).impactOccurred()
                let shown = lastPicture
                try? await Task.sleep(for: .seconds(2.5))
                if lastPicture == shown { lastPicture = nil }
            } else {
                notice = "The picture could not be saved."
            }
        }
    }

    // MARK: sending

    var hasToken: Bool { !store.tokenIgnored && WhatsNewKeychain.token() != nil }
    var unsentCount: Int { store.unsent().count }

    /// Sends every answer not yet in the private repository, one file at a time.
    func send() {
        guard !sending, hasToken, let token = WhatsNewKeychain.token() else { return }
        let files = store.unsent()
        guard !files.isEmpty else { sendResult = "Nothing left to send."; return }
        sending = true
        sendResult = nil
        sendProgress = (0, files.count)
        Task {
            var failed: String?
            for (i, file) in files.enumerated() {
                let path = store.remotePath(of: file)
                do {
                    try await WhatsNewCourier.put(file, at: path, destination: store.destination, token: token,
                                                  message: "\(store.app): \(file.lastPathComponent)")
                    store.markSent(file)
                    sendProgress = (i + 1, files.count)
                } catch {
                    failed = error.localizedDescription
                    break
                }
            }
            sending = false
            if let failed {
                let left = store.unsent().count
                sendResult = "\(failed) \(left) file\(left == 1 ? "" : "s") left to send; try again later."
            } else {
                sendResult = "Sent \(files.count) file\(files.count == 1 ? "" : "s") to \(store.destination.repository)."
            }
        }
    }
}
#endif
