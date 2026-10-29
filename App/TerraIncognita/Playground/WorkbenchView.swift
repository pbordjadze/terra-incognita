import Foundation
import SwiftUI

/// The developer's menu: every prototype in one list, the plain globe at the top, the packs' sources
/// and revisions, and what is alive right now. Plain system controls on purpose: it is a tool for
/// trying things, not part of the atlas.
struct WorkbenchView: View {
    let playground: Playground
    @State private var sourceText = ""
    @State private var sourceError: String?

    private var packs: PackStore { playground.packs }

    var body: some View {
        NavigationStack {
            List {
                Section {
                    Button { playground.choosePlainGlobe() } label: {
                        rowLabel("The plain globe", detail: plainDetail, active: playground.stages.isEmpty)
                    }
                    .accessibilityIdentifier("prototype_plain")
                    if let recent = playground.recent, recent != playground.stage?.prototype.id, let again = playground.prototype(recent) {
                        Button { playground.choose(recent) } label: {
                            rowLabel("Again: \(again.title)", detail: "The last one you started")
                        }
                        .accessibilityIdentifier("prototypeAgain")
                    }
                }
                Section("Prototypes") {
                    ForEach(playground.prototypes.filter { $0.shelf == .current }) { row($0) }
                }
                Section {
                    ForEach(playground.prototypes.filter { $0.shelf == .parked }) { row($0) }
                } header: {
                    Text("Parked")
                } footer: {
                    Text("Earlier directions, kept whole and one tap away.")
                }
                packsSection
                Section("Live") {
                    Text(LiveCount.shared.summary)
                        .font(.footnote.monospaced())
                        .accessibilityIdentifier("playgroundLive")
                    ForEach(Array(packs.log.suffix(8).enumerated()), id: \.offset) { _, line in
                        Text(line).font(.caption.monospaced()).foregroundStyle(.secondary)
                    }
                }
            }
            .accessibilityIdentifier("workbenchList")
            .navigationTitle("Workbench")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { playground.workbenchShown = false }
                        .accessibilityIdentifier("workbenchClose")
                }
            }
            .refreshable { await packs.refresh(force: true) }
            .task { await packs.refresh() }
        }
        // the app's gold accent is too pale for controls on white: a deep blue (over 7:1)
        .tint(Color(red: 0.13, green: 0.33, blue: 0.55))
    }

    /// What choosing the plain globe does: nothing, or end what runs (several run at once).
    private var plainDetail: String {
        switch playground.stages.count {
        case 0: "Showing now"
        case 1: "End \(playground.stage?.prototype.title ?? "")"
        default: "End all \(playground.stages.count) running"
        }
    }

    private func row(_ p: Prototype) -> some View {
        Button { playground.choose(p.id) } label: {
            rowLabel(p.title, summary: p.summary, detail: detail(p), active: playground.isRunning(p.id))
        }
        .disabled(isUnavailable(p))
        .accessibilityIdentifier("prototype_\(p.id)")
    }

    private func rowLabel(_ title: String, summary: String? = nil, detail: String, active: Bool = false) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            VStack(alignment: .leading, spacing: 3) {
                // (explicit label colours: inside a list's button, .primary means the tint)
                Text(title).font(.body.weight(.semibold)).foregroundStyle(Color(uiColor: .label))
                if let summary { Text(summary).font(.subheadline).foregroundStyle(Color(uiColor: .secondaryLabel)) }
                Text(detail).font(.caption).foregroundStyle(Color(uiColor: .secondaryLabel))
            }
            Spacer(minLength: 8)
            if active {
                Image(systemName: "checkmark").foregroundStyle(.tint).accessibilityLabel("Active")
            }
        }
        .contentShape(Rectangle())
    }

    private func isUnavailable(_ p: Prototype) -> Bool {
        if case .pack(.needsNewBuild, _, _) = p.made { return true }
        return false
    }

    /// What it is and where it opens, in a few words (and, running, whether it is the one in front).
    private func detail(_ p: Prototype) -> String {
        var place = p.ownScreen ? "own screen" : "on the globe"
        if playground.stage?.prototype.id == p.id { place += " · in front" }
        else if playground.isRunning(p.id) { place += " · running behind" }
        switch p.made {
        case .native:
            return "Native · \(place)"
        case .pack(let state, let revision, let bundled):
            let made = revision > 0 ? "Pack r\(revision)" : (bundled ? "Pack · bundled" : "Pack")
            switch state {
            case .ready: return "\(made) · \(place)"
            case .remoteOnly: return "\(made) · from a source, downloads on first open"
            case .updateAvailable(let r): return "\(made) · r\(r) available, downloads on open"
            case .downloading(let f): return "\(made) · downloading \(Int((f * 100).rounded()))%"
            case .needsNewBuild(let runtime): return "\(made) · needs a newer app (runtime \(runtime))"
            case .failed(let why): return "\(made) · \(why)"
            }
        }
    }

    // MARK: packs

    private var packsSection: some View {
        Section {
            Button {
                Task { await packs.refresh(force: true) }
            } label: {
                HStack {
                    Text("Check sources now")
                    Spacer()
                    if packs.checking { ProgressView() }
                }
            }
            .disabled(packs.checking || packs.sources.isEmpty)
            .accessibilityIdentifier("checkPacks")
            Text(status)
                .font(.footnote)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("packsStatus")
            ForEach(packs.problems, id: \.self) { problem in
                Text(problem).font(.footnote).foregroundStyle(.red)
            }
            ForEach(packs.sources, id: \.self) { url in
                HStack {
                    Text(url.absoluteString).font(.footnote.monospaced()).lineLimit(2).textSelection(.enabled)
                    Spacer()
                    if packs.addedSources.contains(url) {
                        Button(role: .destructive) { packs.removeSource(url) } label: { Image(systemName: "minus.circle") }
                            .buttonStyle(.borderless)
                            .accessibilityLabel("Remove this source")
                    }
                }
            }
            HStack {
                TextField("https://…/catalog.json", text: $sourceText)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .keyboardType(.URL)
                    .accessibilityIdentifier("addSourceField")
                Button("Add") { addSource() }
                    .buttonStyle(.borderless)
                    .disabled(sourceText.isEmpty)
                    .accessibilityIdentifier("addSource")
            }
            if let sourceError { Text(sourceError).font(.footnote).foregroundStyle(.red) }
            ForEach(packs.listings.filter { !$0.installedRevisions.isEmpty || $0.canUsePrevious }) { listing in
                revisions(listing)
            }
        } header: {
            Text("Packs")
        } footer: {
            Text("Sources are checked only while the workbench is open; the globe itself never uses the network. A downloaded revision is kept with the one before it.")
        }
    }

    private var status: String {
        if packs.checking { return "Checking…" }
        guard let date = packs.lastChecked else {
            return packs.sources.isEmpty ? "No sources yet: only bundled packs." : "Not checked yet."
        }
        return "Checked at \(Self.eastern.string(from: date)) ET"
    }

    private func revisions(_ listing: PackListing) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(listing.id).font(.footnote.monospaced())
            Text(showing(listing))
                .font(.footnote)
                .foregroundStyle(.secondary)
                .accessibilityIdentifier("revision_\(listing.id)")
            HStack(spacing: 16) {
                // (a running copy is the old revision: it ends, and the next start opens the chosen one)
                if listing.canUsePrevious {
                    Button("Previous") { playground.end(listing.id); packs.usePrevious(listing.id) }
                        .accessibilityIdentifier("previous_\(listing.id)")
                }
                if let newest = listing.installedRevisions.first, (listing.current?.revision ?? 0) < newest {
                    Button("Newest") { playground.end(listing.id); packs.useNewest(listing.id) }
                        .accessibilityIdentifier("newest_\(listing.id)")
                }
                if !listing.installedRevisions.isEmpty {
                    Button("Remove downloads", role: .destructive) { playground.end(listing.id); packs.removeDownloads(listing.id) }
                        .accessibilityIdentifier("remove_\(listing.id)")
                }
            }
            .buttonStyle(.borderless)
            .font(.footnote)
        }
    }

    private func showing(_ listing: PackListing) -> String {
        let installed = listing.installedRevisions.map { "r\($0)" }.joined(separator: ", ")
        let now: String
        switch listing.current?.origin {
        case .downloaded?: now = "opens r\(listing.current?.revision ?? 0)"
        case .bundled?: now = "opens the bundled copy"
        case nil: now = "nothing installed"
        }
        return installed.isEmpty ? now : "\(now) · downloaded \(installed)"
    }

    private func addSource() {
        guard let url = URL(string: sourceText.trimmingCharacters(in: .whitespacesAndNewlines)) else {
            sourceError = "That is not a URL."
            return
        }
        do {
            try packs.addSource(url)
            sourceText = ""
            sourceError = nil
            Task { await packs.refresh(force: true) }
        } catch {
            sourceError = error.localizedDescription
        }
    }

    /// Times shown here are Eastern, like everything else given to the owner.
    private static let eastern: DateFormatter = {
        let f = DateFormatter()
        f.timeZone = TimeZone(identifier: "America/New_York")
        f.dateFormat = "h:mm:ss a"
        return f
    }()
}
