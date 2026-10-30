#if UNRELEASED
import SwiftUI
import UIKit

// The module's chrome, shared by any host app: the pill that says a build has something to show, the
// bar over a playing piece, the band of taps under it, the card at the end, the way back (the list),
// and the sending settings. Everything sits on the caption's paper (light in any scheme, legible over
// clouds, sea or night); the one tap the owner is asked for is the biggest thing on the screen.

/// The way in, while a build has pieces due: on the globe near the other controls.
struct WhatsNewPill: View {
    let count: Int
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 6) {
                Image(systemName: "sparkles").font(.system(size: 14, weight: .semibold))
                Text(count == 1 ? "1 new thing" : "\(count) new things")
                    .font(.subheadline.weight(.semibold))
                Text("Show me").font(.subheadline).foregroundStyle(WhatsNewStyle.faint)
            }
            .padding(.horizontal, 14)
            .frame(minHeight: 44)
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .foregroundStyle(WhatsNewStyle.gold)
        .whatsNewPaper(22)
        .shadow(color: .black.opacity(0.08), radius: 6, y: 2)
        .accessibilityLabel(count == 1 ? Text("What's new: 1 new thing. Show me") : Text("What's new: \(count) new things. Show me"))
        .accessibilityIdentifier("whatsNewPill")
    }
}

enum WhatsNewStyle {
    /// The atlas's gold as an ink: deep enough to read on the paper (about 6:1).
    static let gold = Color(red: 0.45, green: 0.33, blue: 0.08)
    static let recording = Color(red: 0.80, green: 0.18, blue: 0.14)
    /// The ground the chrome sits on: the caption's paper, a little translucent, the same over clouds,
    /// sea or night (glass takes the scheme and the ground behind it, and came out dark over the sky).
    static let paper = Color(red: 0.97, green: 0.95, blue: 0.90).opacity(0.94)
    /// The band's ground: the paper, near opaque, so nothing of the globe leaks between the taps.
    static let band = Color(red: 0.97, green: 0.95, blue: 0.90).opacity(0.97)
    static let ink = Color(red: 0.16, green: 0.13, blue: 0.09)
    static let faint = Color(red: 0.16, green: 0.13, blue: 0.09).opacity(0.62)
    static let rule = Color(red: 0.16, green: 0.13, blue: 0.09).opacity(0.14)
    /// The three verdicts' grounds, told apart by colour at arm's length: warm, green, grey.
    static func tint(_ reaction: WhatsNewReaction) -> Color {
        switch reaction {
        case .wow: Color(red: 0.98, green: 0.86, blue: 0.52)
        case .nice: Color(red: 0.80, green: 0.90, blue: 0.74)
        default: Color(red: 0.88, green: 0.86, blue: 0.82)
        }
    }
}

extension View {
    /// The mode's ground: paper, rounded, a hairline.
    func whatsNewPaper(_ radius: CGFloat) -> some View {
        background(WhatsNewStyle.paper, in: RoundedRectangle(cornerRadius: radius, style: .continuous))
            .overlay(RoundedRectangle(cornerRadius: radius, style: .continuous).strokeBorder(WhatsNewStyle.rule, lineWidth: 0.5))
            .foregroundStyle(WhatsNewStyle.ink)
            .environment(\.colorScheme, .light)
    }
}

/// Over a playing piece: where we are and its name on one line, the makers' question (or what to look
/// at) as the line that matters, the way back a piece, its panel, and Later (the round waits).
struct WhatsNewBar: View {
    let session: WhatsNewSession

    var body: some View {
        if let piece = session.current {
            HStack(alignment: .center, spacing: 4) {
                Button { session.back() } label: {
                    Image(systemName: "chevron.left").font(.system(size: 15, weight: .semibold)).frame(width: 36, height: 44)
                }
                .disabled(!session.canGoBack)
                .opacity(session.canGoBack ? 1 : 0.3)
                .accessibilityLabel("Previous piece")
                .accessibilityIdentifier("whatsNewBack")
                VStack(alignment: .leading, spacing: 3) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(verbatim: "\(session.index + 1) of \(session.pieces.count)")
                            .font(.caption.weight(.semibold).monospacedDigit())
                            .foregroundStyle(WhatsNewStyle.faint)
                            .accessibilityIdentifier("whatsNewProgress")
                        Text(piece.title)
                            .font(.subheadline.weight(.semibold))
                            .lineLimit(1)
                            .accessibilityIdentifier("whatsNewTitle")
                    }
                    if let line = piece.ask ?? piece.what {
                        Text(line)
                            .font(.system(size: 15, weight: .medium))
                            .foregroundStyle(WhatsNewStyle.gold)
                            .lineLimit(3)
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("whatsNewLine")
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 9)
                if session.host?.canOpen == true {
                    Button { session.host?.open() } label: {
                        Text("Panel").font(.subheadline).padding(.horizontal, 6).frame(minHeight: 44).contentShape(Rectangle())
                    }
                    .accessibilityLabel("Open its panel")
                    .accessibilityIdentifier("whatsNewOpen")
                }
                Button { session.leave() } label: {
                    Text("Later").font(.subheadline).padding(.horizontal, 8).frame(minHeight: 44).contentShape(Rectangle())
                }
                .accessibilityLabel("Later: leave What's new, keeping what you answered")
                .accessibilityIdentifier("whatsNewLeave")
            }
            .padding(.leading, 4)
            .padding(.trailing, 6)
            .whatsNewPaper(22)
            .shadow(color: .black.opacity(0.08), radius: 6, y: 2)
            .frame(maxWidth: 640)
            .padding(.horizontal, 12)
            .padding(.top, 6)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("whatsNewBar")
            .animation(.easeInOut(duration: 0.2), value: piece.id)
        }
    }
}

/// Under a playing piece, on a band of paper: the three verdicts, big, told apart by colour; the voice
/// note and the picture as small things at the ends; Skip and Didn't work as plain words below.
struct WhatsNewReactionRow: View {
    let session: WhatsNewSession
    @Environment(\.horizontalSizeClass) private var sizeClass

    private var compact: Bool { sizeClass == .compact }

    var body: some View {
        VStack(spacing: 4) {
            HStack(spacing: compact ? 8 : 14) {
                micButton
                ForEach([WhatsNewReaction.wow, .nice, .meh], id: \.self) { reaction in
                    Button { session.react(reaction) } label: {
                        Text(reaction.label)
                            .font(.system(size: compact ? 17 : 19, weight: .semibold))
                            .frame(width: compact ? 80 : 104, height: compact ? 54 : 58)
                            .contentShape(RoundedRectangle(cornerRadius: 18))
                    }
                    .buttonStyle(.plain)
                    .background(WhatsNewStyle.tint(reaction), in: RoundedRectangle(cornerRadius: 18, style: .continuous))
                    .overlay(RoundedRectangle(cornerRadius: 18, style: .continuous).strokeBorder(WhatsNewStyle.rule, lineWidth: 0.5))
                    .accessibilityLabel(reaction.label)
                    .accessibilityIdentifier("react_\(reaction.rawValue)")
                }
                cameraButton
            }
            HStack(spacing: 24) {
                Button { session.react(.skip) } label: {
                    Text("Skip").font(.footnote.weight(.medium)).frame(minWidth: 72, minHeight: 36).contentShape(Rectangle())
                }
                .accessibilityIdentifier("react_skip")
                Button { session.react(.broken) } label: {
                    Text("Didn't work").font(.footnote.weight(.medium)).frame(minWidth: 96, minHeight: 36).contentShape(Rectangle())
                }
                .accessibilityIdentifier("react_broken")
            }
            .buttonStyle(.plain)
            .foregroundStyle(WhatsNewStyle.faint)
            if let notice = session.notice {
                Text(notice).font(.footnote).foregroundStyle(WhatsNewStyle.recording).multilineTextAlignment(.center)
                    .padding(.horizontal, 20)
                    .onTapGesture { session.notice = nil }
                    .accessibilityIdentifier("whatsNewNotice")
            }
        }
        .padding(.top, 10)
        .padding(.bottom, 2)
        .frame(maxWidth: .infinity)
        .foregroundStyle(WhatsNewStyle.ink)
        .background(WhatsNewStyle.band.ignoresSafeArea(edges: .bottom))
        .overlay(alignment: .top) { Rectangle().fill(WhatsNewStyle.rule).frame(height: 0.5) }
        .environment(\.colorScheme, .light)
        .disabled(session.changing)
        .opacity(session.changing ? 0.6 : 1)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("whatsNewReactions")
    }

    private var micButton: some View {
        Button { session.toggleRecording() } label: {
            VStack(spacing: 2) {
                Image(systemName: session.recorder.recording ? "stop.circle.fill" : (session.voice == nil ? "mic" : "mic.fill"))
                    .font(.system(size: 20, weight: .medium))
                Text(session.recorder.recording ? Self.clock(session.recorder.seconds) : (session.voice == nil ? "Say" : "Said"))
                    .font(.caption2.weight(.semibold).monospacedDigit())
            }
            .frame(width: 48, height: compact ? 54 : 58)
            .contentShape(RoundedRectangle(cornerRadius: 16))
        }
        .buttonStyle(.plain)
        .foregroundStyle(session.recorder.recording ? WhatsNewStyle.recording : WhatsNewStyle.faint)
        .accessibilityLabel(session.recorder.recording ? Text("Stop recording") : Text("Record a voice note"))
        .accessibilityIdentifier("whatsNewMic")
    }

    private var cameraButton: some View {
        Button { session.takePicture() } label: {
            ZStack {
                if let picture = session.lastPicture {
                    Image(uiImage: picture)
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                        .frame(width: 44, height: compact ? 50 : 54)
                        .clipShape(RoundedRectangle(cornerRadius: 10))
                        .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(WhatsNewStyle.gold, lineWidth: 1.5))
                } else {
                    VStack(spacing: 2) {
                        Image(systemName: session.screenshot == nil ? "camera" : "camera.fill").font(.system(size: 20, weight: .medium))
                        Text(session.screenshot == nil ? "Picture" : "Taken").font(.caption2.weight(.semibold))
                    }
                }
            }
            .frame(width: 48, height: compact ? 54 : 58)
            .contentShape(RoundedRectangle(cornerRadius: 16))
        }
        .buttonStyle(.plain)
        .foregroundStyle(WhatsNewStyle.faint)
        .disabled(session.capturing)
        .accessibilityLabel(session.lastPicture.map { Text("Picture taken, \(Int((WhatsNewSnapshot.darkness(of: $0) * 100).rounded())) percent dark; take another") }
                            ?? Text("Take a picture of the screen"))
        .accessibilityIdentifier("whatsNewCamera")
    }

    private static func clock(_ s: Double) -> String { String(format: "%d:%02d", Int(s) / 60, Int(s) % 60) }
}

/// At the end of a round: what was said, Done first; Keep going when more pieces wait; Share; and the
/// way to the company only as a quiet line until it is set up.
struct WhatsNewFinishCard: View {
    let session: WhatsNewSession
    let openSending: () -> Void
    let keepGoing: () -> Void
    @State private var bundle: URL?

    var body: some View {
        VStack(spacing: 14) {
            Text(session.single ? "Noted" : (session.waiting > 0 ? "That's all for now" : "That's all for this build"))
                .font(.title3.weight(.semibold))
            Text(tallyLine)
                .font(.subheadline)
                .foregroundStyle(WhatsNewStyle.faint)
                .accessibilityIdentifier("whatsNewTally")
            Button { session.dismiss() } label: { Text("Done").fontWeight(.semibold).frame(maxWidth: .infinity, minHeight: 44) }
                .buttonStyle(.borderedProminent)
                .tint(WhatsNewStyle.gold)
                .accessibilityIdentifier("whatsNewDone")
            HStack(spacing: 12) {
                if session.waiting > 0, !session.single {
                    Button(action: keepGoing) {
                        Text(session.waiting == 1 ? "One more" : "Keep going (\(session.waiting))").frame(maxWidth: .infinity, minHeight: 44)
                    }
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("whatsNewKeepGoing")
                }
                if let bundle {
                    ShareLink(item: bundle) {
                        Label("Share…", systemImage: "square.and.arrow.up").frame(maxWidth: .infinity, minHeight: 44)
                    }
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("whatsNewShare")
                }
            }
            if session.hasToken {
                Button { session.send() } label: {
                    HStack(spacing: 8) {
                        if session.sending { ProgressView().controlSize(.small) }
                        Text(sendLabel)
                    }
                    .frame(maxWidth: .infinity, minHeight: 40)
                }
                .buttonStyle(.bordered)
                .disabled(session.sending || session.unsentCount == 0)
                .accessibilityIdentifier("whatsNewSend")
            } else {
                Button(action: openSending) {
                    Text("Not sent yet · set up sending").font(.footnote).foregroundStyle(WhatsNewStyle.faint).frame(minHeight: 32)
                }
                .buttonStyle(.plain)
                .accessibilityIdentifier("whatsNewSetUp")
            }
            if let result = session.sendResult {
                Text(result).font(.footnote).foregroundStyle(WhatsNewStyle.faint).multilineTextAlignment(.center)
                    .accessibilityIdentifier("whatsNewSendResult")
            }
        }
        .padding(22)
        .frame(maxWidth: 380)
        .whatsNewPaper(26)
        .shadow(color: .black.opacity(0.18), radius: 18, y: 6)
        .padding(24)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("whatsNewFinish")
        .task { bundle = session.store.bundleForSharing() }
    }

    /// "2 wow · 1 nice · 1 didn't work"; the skipped are not counted against anyone.
    private var tallyLine: String {
        let t = session.tally
        let parts = [WhatsNewReaction.wow, .nice, .meh, .broken].compactMap { r -> String? in
            guard let n = t[r], n > 0 else { return nil }
            return "\(n) \(r.label.lowercased())"
        }
        if parts.isEmpty { return session.waiting > 0 ? "\(session.waiting) more are waiting" : "Nothing answered this time" }
        var line = parts.joined(separator: " · ")
        if session.waiting > 0 { line += " · \(session.waiting) more waiting" }
        return line
    }

    private var sendLabel: String {
        if session.sending { return "Sending \(session.sendProgress.done) of \(session.sendProgress.of)…" }
        let n = session.unsentCount
        return n == 0 ? "All sent" : "Send to the company (\(n))"
    }
}

/// The way back: every piece on one line, the new ones first, each with what was said of it; a tap
/// plays it alone. Also Play the new, the sending settings, and (for the owner) See everything again.
struct WhatsNewListView: View {
    let session: WhatsNewSession
    let pieces: [WhatsNewPiece]
    /// Plays these pieces (closing whatever list this sits in first).
    let play: ([WhatsNewPiece], _ single: Bool) -> Void
    @State private var sendingShown = false

    private var ledger: WhatsNewLedger { session.store.ledger }
    private var due: [WhatsNewPiece] { WhatsNewQueue.due(pieces, ledger: ledger) }
    private var seen: [WhatsNewPiece] { WhatsNewQueue.ordered(pieces.filter { $0.isSound && !ledger.isDue($0) }) }

    var body: some View {
        List {
            Section {
                if !due.isEmpty {
                    Button { play(due, false) } label: {
                        rowLabel(due.count == 1 ? "Play the new one" : "Play the \(due.count) new", detail: "A few at a time, one tap after each")
                    }
                    .accessibilityIdentifier("whatsNewPlayAll")
                }
                Button { sendingShown = true } label: {
                    rowLabel("Sending", detail: sendingDetail)
                }
                .accessibilityIdentifier("whatsNewSending")
            }
            if !due.isEmpty {
                Section("New") { ForEach(due) { row($0, due: true) } }
            }
            if !seen.isEmpty {
                Section("Seen") { ForEach(seen) { row($0, due: false) } }
            }
            Section {
                Button(role: .destructive) { session.store.forgetSeen() } label: {
                    Text("See everything again").font(.footnote)
                }
                .disabled(seen.isEmpty)
                .accessibilityIdentifier("whatsNewForget")
            } footer: {
                Text("A tap plays a piece again. Answers stay on this device until you send them.")
            }
        }
        .accessibilityIdentifier("whatsNewList")
        .navigationTitle("What's new")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $sendingShown) {
            NavigationStack { WhatsNewSendingView(session: session) }
                .presentationDetents([.large])
                .preferredColorScheme(.light)
        }
    }

    private var sendingDetail: String {
        let n = session.unsentCount
        if n == 0 { return session.hasToken ? "All sent to \(session.store.destination.repository)" : "Nothing to send" }
        return session.hasToken ? "\(n) not yet sent · tap to send" : "Not sent yet · tap to set up"
    }

    private func row(_ p: WhatsNewPiece, due: Bool) -> some View {
        Button { play([p], true) } label: {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                Text(p.title).font(.body.weight(.semibold)).foregroundStyle(Color(uiColor: .label)).lineLimit(1)
                Spacer(minLength: 8)
                Text(detail(p, due: due)).font(.caption).foregroundStyle(Color(uiColor: .secondaryLabel)).lineLimit(1)
                Text(verbatim: ledger.reaction(for: p)?.mark ?? "·")
                    .font(.body)
                    .frame(width: 26)
                    .accessibilityLabel(Text(ledger.reaction(for: p)?.label ?? "Not yet"))
            }
            .contentShape(Rectangle())
        }
        .accessibilityIdentifier("piece_\(p.id)")
    }

    private func detail(_ p: WhatsNewPiece, due: Bool) -> String {
        if due { return ledger.isChanged(p) ? "changed" : Self.day(p.since) }
        if let e = ledger.entries[p.id] { return Self.day(String(e.at.prefix(10))) }
        return Self.day(p.since)
    }

    /// "Oct 29" from "2026-10-29" (with the year when it isn't this one).
    static func day(_ iso: String) -> String {
        let parts = iso.split(separator: "-").compactMap { Int($0) }
        guard parts.count == 3, (1...12).contains(parts[1]) else { return iso }
        let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]
        let thisYear = Calendar.current.component(.year, from: Date())
        return parts[0] == thisYear ? "\(months[parts[1] - 1]) \(parts[2])" : "\(months[parts[1] - 1]) \(parts[2]), \(parts[0])"
    }

    private func rowLabel(_ title: String, detail: String) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title).font(.body.weight(.semibold)).foregroundStyle(Color(uiColor: .label))
            Text(detail).font(.caption).foregroundStyle(Color(uiColor: .secondaryLabel))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }
}

/// Where the answers go and the owner's token: entered once, kept in the Keychain, checked against
/// the repository; Send now; Share instead.
struct WhatsNewSendingView: View {
    let session: WhatsNewSession
    @State private var token = ""
    @State private var checking = false
    @State private var checked: String?
    @State private var bundle: URL?
    @Environment(\.dismiss) private var dismiss

    private var destination: WhatsNewDestination { session.store.destination }

    var body: some View {
        List {
            Section {
                Text("Answers become files in \(destination.repository) under \(destination.folder)/, a private repository. They go only when you tap Send, over HTTPS, with your own token, which stays in this device's Keychain.")
                    .font(.footnote)
                if session.hasToken {
                    Label("A token is set", systemImage: "checkmark.seal").font(.subheadline)
                    Button("Check it") { check(WhatsNewKeychain.token() ?? "") }.disabled(checking)
                        .accessibilityIdentifier("whatsNewCheckToken")
                    Button("Remove the token", role: .destructive) { WhatsNewKeychain.clear(); checked = nil }
                        .accessibilityIdentifier("whatsNewRemoveToken")
                } else {
                    SecureField("github_pat_…", text: $token)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                        .accessibilityIdentifier("whatsNewTokenField")
                    Button("Save and check") { save() }
                        .disabled(token.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty || checking)
                        .accessibilityIdentifier("whatsNewSaveToken")
                }
                if checking { ProgressView() }
                if let checked { Text(checked).font(.footnote).foregroundStyle(.secondary).accessibilityIdentifier("whatsNewTokenResult") }
            } header: {
                Text("Token")
            } footer: {
                Text("A fine-grained personal access token (github.com › Settings › Developer settings), for \(destination.repository) only, with Contents: Read and write. Nothing else is asked of it.")
            }
            Section("Waiting to be sent") {
                let n = session.unsentCount
                Text(n == 0 ? "Nothing" : "\(n) file\(n == 1 ? "" : "s")").font(.subheadline)
                Button { session.send() } label: {
                    HStack { Text("Send now"); if session.sending { Spacer(); ProgressView() } }
                }
                .disabled(!session.hasToken || session.sending || n == 0)
                .accessibilityIdentifier("whatsNewSendNow")
                if let result = session.sendResult { Text(result).font(.footnote).foregroundStyle(.secondary) }
                if let bundle {
                    ShareLink(item: bundle) { Label("Share the answers instead", systemImage: "square.and.arrow.up") }
                }
            }
        }
        .navigationTitle("Sending")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done") { dismiss() }.accessibilityIdentifier("whatsNewSendingDone") } }
        .accessibilityIdentifier("whatsNewSendingView")
        .task { bundle = session.store.bundleForSharing() }
    }

    private func save() {
        let t = token.trimmingCharacters(in: .whitespacesAndNewlines)
        WhatsNewKeychain.set(t)
        token = ""
        check(t)
    }

    private func check(_ t: String) {
        checking = true
        checked = nil
        Task {
            do {
                let name = try await WhatsNewCourier.check(destination, token: t)
                checked = "The token opens \(name) and can write to it."
            } catch {
                checked = error.localizedDescription
            }
            checking = false
        }
    }
}
#endif
