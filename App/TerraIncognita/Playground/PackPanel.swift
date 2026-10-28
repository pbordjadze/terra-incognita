import SwiftUI

/// Longer words and their sources a pack shows over the globe (terra.js runtime 3, `terra.panel`):
/// what you're looking at first, then why it's interesting, then where it comes from. Plain text only.
struct PackPanel: Identifiable, Equatable {
    struct Section: Equatable {
        let title: String
        let text: String
    }
    struct Source: Equatable {
        let title: String
        let url: URL?
        let note: String
    }

    let id = UUID()
    let pack: String
    let title: String
    let text: String
    let sections: [Section]
    let sources: [Source]

    /// From `terra.panel.show(content)`, bounded: a panel is a page or two, not a book.
    init(_ pack: String, _ content: [String: Any]) {
        func text(_ x: Any?, _ limit: Int) -> String { String(((x as? String) ?? "").prefix(limit)) }
        self.pack = pack
        title = text(content["title"], 120)
        self.text = text(content["text"], 4000)
        sections = ((content["sections"] as? [Any]) ?? []).prefix(12).compactMap { item in
            guard let s = item as? [String: Any] else { return nil }
            return Section(title: text(s["title"], 120), text: text(s["text"], 4000))
        }
        sources = ((content["sources"] as? [Any]) ?? []).prefix(24).compactMap { item in
            guard let s = item as? [String: Any] else { return nil }
            // (links leave for Safari only when tapped, and only https)
            let url = (s["url"] as? String).flatMap(URL.init(string:)).flatMap { $0.scheme?.lowercased() == "https" ? $0 : nil }
            return Source(title: text(s["title"], 300), url: url, note: text(s["note"], 600))
        }
    }

    static func == (a: PackPanel, b: PackPanel) -> Bool { a.id == b.id }
}

/// The panel's sheet: half height (the globe stays live above it), pulled up for more.
struct PackPanelView: View {
    let panel: PackPanel
    @Environment(\.dismiss) private var dismiss

    private static let ink = Color(red: 0.145, green: 0.165, blue: 0.18)
    private static let faint = Color(red: 0.36, green: 0.38, blue: 0.39)
    private static let paper = Color(red: 0.965, green: 0.949, blue: 0.906)

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                if !panel.title.isEmpty {
                    Text(panel.title)
                        .font(.system(.title2, design: .serif).weight(.semibold))
                        .accessibilityAddTraits(.isHeader)
                        .accessibilityIdentifier("packPanelTitle")
                }
                paragraphs(panel.text)
                ForEach(Array(panel.sections.enumerated()), id: \.offset) { _, section in
                    VStack(alignment: .leading, spacing: 6) {
                        if !section.title.isEmpty {
                            Text(section.title)
                                .font(.system(.headline, design: .serif))
                                .accessibilityAddTraits(.isHeader)
                        }
                        paragraphs(section.text)
                    }
                }
                if !panel.sources.isEmpty {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Sources")
                            .font(.system(.subheadline, design: .serif).weight(.semibold))
                            .foregroundStyle(Self.faint)
                            .accessibilityAddTraits(.isHeader)
                        ForEach(Array(panel.sources.enumerated()), id: \.offset) { _, source in
                            VStack(alignment: .leading, spacing: 2) {
                                if let url = source.url {
                                    Link(source.title, destination: url)
                                        .font(.system(.footnote, design: .serif))
                                } else {
                                    Text(source.title).font(.system(.footnote, design: .serif))
                                }
                                if !source.note.isEmpty {
                                    Text(source.note).font(.caption).foregroundStyle(Self.faint)
                                }
                            }
                        }
                    }
                    .padding(.top, 6)
                }
            }
            .foregroundStyle(Self.ink)
            .frame(maxWidth: 560, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 24)
            .padding(.top, 28)
            .padding(.bottom, 32)
        }
        .background(Self.paper)
        // (named before the close button is laid over it: named after, the button takes the name too)
        .accessibilityIdentifier("packPanel")
        .overlay(alignment: .topTrailing) {
            Button { dismiss() } label: {
                Image(systemName: "xmark").font(.system(size: 14, weight: .semibold)).frame(width: 44, height: 44)
            }
            .foregroundStyle(Self.faint)
            .accessibilityLabel("Close")
            .accessibilityIdentifier("packPanelClose")
            .padding(.trailing, 8)
            .padding(.top, 4)
        }
    }

    @ViewBuilder private func paragraphs(_ text: String) -> some View {
        let parts = text.components(separatedBy: "\n\n").map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        ForEach(Array(parts.enumerated()), id: \.offset) { _, part in
            Text(part)
                .font(.system(.body, design: .serif))
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
