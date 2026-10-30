#if UNRELEASED
import SwiftUI
import UIKit

// The module's chrome, shared by any host app: the pill that says a build has something to show, the
// bar over a playing piece, the row of taps under it, the card at the end, the way back (the list),
// and the sending settings. System type and glass, like the host's own controls; emoji for the taps,
// because they read in a glance on a phone held at arm's length.

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
                Text("Show me").font(.subheadline).foregroundStyle(.secondary)
            }
            .padding(.horizontal, 14)
            .frame(minHeight: 44)
            .contentShape(Capsule())
        }
        .buttonStyle(.plain)
        .foregroundStyle(WhatsNewStyle.gold)
        .glassEffect(.regular, in: Capsule())
        .accessibilityLabel(count == 1 ? Text("What's new: 1 new thing. Show me") : Text("What's new: \(count) new things. Show me"))
        .accessibilityIdentifier("whatsNewPill")
    }
}

enum WhatsNewStyle {
    /// The atlas's gold, deep enough to read on glass.
    static let gold = Color(red: 0.62, green: 0.47, blue: 0.16)
    static let recording = Color(red: 0.80, green: 0.18, blue: 0.14)
}

/// Over a playing piece: where we are, what it is and what to look at, the way back, Open and Leave.
struct WhatsNewBar: View {
    let session: WhatsNewSession
    @Environment(\.horizontalSizeClass) private var sizeClass

    var body: some View {
        if let piece = session.current {
            HStack(alignment: .center, spacing: 6) {
                Button { session.back() } label: {
                    Image(systemName: "chevron.left").font(.system(size: 15, weight: .semibold)).frame(width: 40, height: 44)
                }
                .disabled(!session.canGoBack)
                .opacity(session.canGoBack ? 1 : 0.3)
                .accessibilityLabel("Previous piece")
                .accessibilityIdentifier("whatsNewBack")
                VStack(alignment: .leading, spacing: 2) {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(verbatim: "\(session.index + 1) of \(session.pieces.count)")
                            .font(.caption.weight(.semibold).monospacedDigit())
                            .foregroundStyle(WhatsNewStyle.gold)
                            .accessibilityIdentifier("whatsNewProgress")
                        Text(piece.title)
                            .font(.subheadline.weight(.semibold))
                            .lineLimit(1)
                            .accessibilityIdentifier("whatsNewTitle")
                    }
                    if let line = piece.what ?? piece.ask {
                        Text(line)
                            .font(.footnote)
                            .foregroundStyle(.secondary)
                            .lineLimit(sizeClass == .compact ? 2 : 3)
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("whatsNewWhat")
                    }
                    if piece.what != nil, let ask = piece.ask {
                        Text(ask)
                            .font(.footnote.italic())
                            .foregroundStyle(WhatsNewStyle.gold)
                            .lineLimit(2)
                            .fixedSize(horizontal: false, vertical: true)
                            .accessibilityIdentifier("whatsNewAsk")
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.vertical, 8)
                if session.host?.canOpen == true {
                    Button { session.host?.open() } label: {
                        Text("Open").font(.subheadline).padding(.horizontal, 6).frame(minHeight: 44).contentShape(Rectangle())
                    }
                    .accessibilityIdentifier("whatsNewOpen")
                }
                Button { session.leave() } label: {
                    Image(systemName: "xmark").font(.system(size: 14, weight: .semibold)).frame(width: 44, height: 44)
                }
                .accessibilityLabel("Leave What's new")
                .accessibilityIdentifier("whatsNewLeave")
            }
            .padding(.leading, 4)
            .padding(.trailing, 4)
            .foregroundStyle(.primary)
            .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 22))
            .frame(maxWidth: 640)
            .padding(.horizontal, 12)
            .padding(.top, 6)
            .accessibilityElement(children: .contain)
            .accessibilityIdentifier("whatsNewBar")
            .animation(.easeInOut(duration: 0.2), value: piece.id)
        }
    }
}

/// Under a playing piece: the taps (Wow, Nice, Meh), the voice note and the picture, Skip and
/// Didn't work. Big enough for a thumb; one tap and the next piece comes.
struct WhatsNewReactionRow: View {
    let session: WhatsNewSession
    @Environment(\.horizontalSizeClass) private var sizeClass

    var body: some View {
        VStack(spacing: 6) {
            HStack(spacing: sizeClass == .compact ? 10 : 16) {
                micButton
                ForEach([WhatsNewReaction.wow, .nice, .meh], id: \.self) { reaction in
                    Button { session.react(reaction) } label: {
                        VStack(spacing: 1) {
                            Text(verbatim: reaction.mark).font(.system(size: 30))
                            Text(reaction.label).font(.caption2.weight(.semibold))
                        }
                        .frame(width: sizeClass == .compact ? 68 : 84, height: 64)
                        .contentShape(RoundedRectangle(cornerRadius: 20))
                    }
                    .buttonStyle(.plain)
                    .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 20))
                    .accessibilityLabel(reaction.label)
                    .accessibilityIdentifier("react_\(reaction.rawValue)")
                }
                cameraButton
            }
            HStack(spacing: 28) {
                Button { session.react(.skip) } label: {
                    Text("Skip").font(.footnote.weight(.medium)).padding(.horizontal, 10).frame(minHeight: 32)
                }
                .accessibilityIdentifier("react_skip")
                Button { session.react(.broken) } label: {
                    Text("Didn't work").font(.footnote.weight(.medium)).padding(.horizontal, 10).frame(minHeight: 32)
                }
                .accessibilityIdentifier("react_broken")
            }
            .foregroundStyle(.secondary)
            .buttonStyle(.plain)
            if let notice = session.notice {
                Text(notice).font(.footnote).foregroundStyle(WhatsNewStyle.recording).multilineTextAlignment(.center)
                    .padding(.horizontal, 20)
                    .onTapGesture { session.notice = nil }
                    .accessibilityIdentifier("whatsNewNotice")
            }
        }
        .padding(.top, 8)
        .padding(.bottom, 4)
        .frame(maxWidth: .infinity)
        .disabled(session.changing)
        .opacity(session.changing ? 0.5 : 1)
        .overlay(alignment: .topTrailing) {
            if let picture = session.lastPicture {
                Image(uiImage: picture)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .frame(height: 72)
                    .clipShape(RoundedRectangle(cornerRadius: 6))
                    .overlay(RoundedRectangle(cornerRadius: 6).stroke(.white.opacity(0.8), lineWidth: 1))
                    .shadow(radius: 4)
                    .padding(.trailing, 16)
                    .offset(y: -84)
                    .transition(.scale(scale: 0.6).combined(with: .opacity))
                    .accessibilityLabel(Text("Picture taken, \(Int((WhatsNewSnapshot.darkness(of: picture) * 100).rounded())) percent dark"))
                    .accessibilityIdentifier("whatsNewPicture")
            }
        }
        .animation(.easeInOut(duration: 0.25), value: session.lastPicture == nil)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("whatsNewReactions")
    }

    private var micButton: some View {
        Button { session.toggleRecording() } label: {
            VStack(spacing: 1) {
                Image(systemName: session.recorder.recording ? "stop.circle.fill" : (session.voice == nil ? "mic" : "mic.fill"))
                    .font(.system(size: 22, weight: .medium))
                Text(session.recorder.recording ? Self.clock(session.recorder.seconds) : (session.voice == nil ? "Say" : "Said"))
                    .font(.caption2.weight(.semibold).monospacedDigit())
            }
            .frame(width: 56, height: 64)
            .contentShape(RoundedRectangle(cornerRadius: 20))
        }
        .buttonStyle(.plain)
        .foregroundStyle(session.recorder.recording ? WhatsNewStyle.recording : .primary)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 20))
        .accessibilityLabel(session.recorder.recording ? Text("Stop recording") : Text("Record a voice note"))
        .accessibilityIdentifier("whatsNewMic")
    }

    private var cameraButton: some View {
        Button { session.takePicture() } label: {
            VStack(spacing: 1) {
                Image(systemName: session.screenshot == nil ? "camera" : "camera.fill").font(.system(size: 22, weight: .medium))
                Text(session.screenshot == nil ? "Picture" : "Taken").font(.caption2.weight(.semibold))
            }
            .frame(width: 56, height: 64)
            .contentShape(RoundedRectangle(cornerRadius: 20))
        }
        .buttonStyle(.plain)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 20))
        .disabled(session.capturing)
        .accessibilityLabel("Take a picture of the screen")
        .accessibilityIdentifier("whatsNewCamera")
    }

    private static func clock(_ s: Double) -> String { String(format: "%d:%02d", Int(s) / 60, Int(s) % 60) }
}

/// At the end of a round: the tally, Send (to the private repository, when a token is set), Share and Done.
struct WhatsNewFinishCard: View {
    let session: WhatsNewSession
    let openSending: () -> Void
    @State private var bundle: URL?

    var body: some View {
        VStack(spacing: 14) {
            Text(session.single ? "Noted" : "That's all for this build")
                .font(.title3.weight(.semibold))
            Text(tallyLine)
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("whatsNewTally")
            if session.hasToken {
                Button { session.send() } label: {
                    HStack(spacing: 8) {
                        if session.sending { ProgressView().controlSize(.small) }
                        Text(sendLabel).fontWeight(.semibold)
                    }
                    .frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.borderedProminent)
                .tint(WhatsNewStyle.gold)
                .disabled(session.sending || session.unsentCount == 0)
                .accessibilityIdentifier("whatsNewSend")
            } else {
                Button(action: openSending) {
                    Text("Set up sending to the company").fontWeight(.semibold).frame(maxWidth: .infinity, minHeight: 44)
                }
                .buttonStyle(.borderedProminent)
                .tint(WhatsNewStyle.gold)
                .accessibilityIdentifier("whatsNewSetUp")
            }
            if let result = session.sendResult {
                Text(result).font(.footnote).foregroundStyle(.secondary).multilineTextAlignment(.center)
                    .accessibilityIdentifier("whatsNewSendResult")
            }
            HStack(spacing: 12) {
                if let bundle {
                    ShareLink(item: bundle) {
                        Label("Share…", systemImage: "square.and.arrow.up").frame(maxWidth: .infinity, minHeight: 44)
                    }
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("whatsNewShare")
                }
                Button { session.dismiss() } label: { Text("Done").frame(maxWidth: .infinity, minHeight: 44) }
                    .buttonStyle(.bordered)
                    .accessibilityIdentifier("whatsNewDone")
            }
        }
        .padding(22)
        .frame(maxWidth: 380)
        .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 26))
        .padding(24)
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("whatsNewFinish")
        .task { bundle = session.store.bundleForSharing() }
    }

    private var tallyLine: String {
        let t = session.tally
        var parts = [WhatsNewReaction.wow, .nice, .meh, .broken].compactMap { r -> String? in
            guard let n = t[r], n > 0 else { return nil }
            return "\(r.mark) \(n)"
        }
        if let s = t[.skip], s > 0 { parts.append("\(s) skipped") }
        return parts.isEmpty ? "Nothing answered" : parts.joined(separator: "  ·  ")
    }

    private var sendLabel: String {
        if session.sending { return "Sending \(session.sendProgress.done) of \(session.sendProgress.of)…" }
        let n = session.unsentCount
        return n == 0 ? "All sent" : "Send to the company (\(n))"
    }
}

/// The way back: every piece, new ones first, each with what was said of it; one tap plays it again
/// (alone). Also Play all new, the sending settings, and (for the owner) See everything again.
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
                Button { play(due, false) } label: {
                    rowLabel(due.isEmpty ? "Nothing new" : (due.count == 1 ? "Play the new one" : "Play all \(due.count) new"),
                             detail: "Each from where its makers meant it to be seen; one tap after each.")
                }
                .disabled(due.isEmpty)
                .accessibilityIdentifier("whatsNewPlayAll")
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
                Text("Answers stay on this device until sent; only you can send them. docs/whatsnew.md says how a session adds a piece.")
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
        let pending = n == 0 ? "nothing waiting" : "\(n) file\(n == 1 ? "" : "s") waiting"
        return session.hasToken ? "To \(session.store.destination.repository) · \(pending)" : "Not set up · \(pending)"
    }

    private func row(_ p: WhatsNewPiece, due: Bool) -> some View {
        Button { play([p], true) } label: {
            HStack(alignment: .firstTextBaseline, spacing: 10) {
                VStack(alignment: .leading, spacing: 3) {
                    Text(p.title).font(.body.weight(.semibold)).foregroundStyle(Color(uiColor: .label))
                    if let what = p.what { Text(what).font(.subheadline).foregroundStyle(Color(uiColor: .secondaryLabel)).lineLimit(2) }
                    Text(detail(p, due: due)).font(.caption).foregroundStyle(Color(uiColor: .secondaryLabel))
                }
                Spacer(minLength: 8)
                if let r = ledger.reaction(for: p) {
                    Text(verbatim: r.mark).font(.title3).accessibilityLabel(Text(r.label))
                }
            }
            .contentShape(Rectangle())
        }
        .accessibilityIdentifier("piece_\(p.id)")
    }

    private func detail(_ p: WhatsNewPiece, due: Bool) -> String {
        var s = "Since \(p.since)"
        if due { s += ledger.isChanged(p) ? " · changed since you saw it" : " · new" }
        else if let e = ledger.entries[p.id] { s += " · \(e.reaction.label.lowercased()) on \(String(e.at.prefix(10)))" }
        return s
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
