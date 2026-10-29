import Foundation
import Observation
import SwiftUI

/// Wikipedia's words (painting-company row 177): a switch beside the clouds' that puts, in place of
/// every paragraph the atlas wrote itself, Wikipedia's own words on the same subject: the article's
/// first sentences as they stand, and a link to the article. With it on, a short tip (one line, what to
/// touch) stays, a name or a date stays, and a paragraph with no article behind it goes, so what is
/// left to read is Wikipedia's. The articles are bundled (`Wikipedia.json`, built by
/// tools/wikipedia/build.py; a pack carries its own `wikipedia.json`): the app fetches nothing. Off
/// until the reader turns it on, and remembered.
@Observable
final class WikipediaWords {
    static let shared = WikipediaWords()
    nonisolated private static let key = "words.wikipedia"
    /// Posted when the reader turns it: the running packs hear it as `terra.settings.wikipedia`.
    nonisolated static let changed = Notification.Name("WikipediaWords.changed")

    private(set) var on: Bool
    /// Held by a launch argument (CI's tests): not remembered.
    private let held: Bool

    private init() {
        #if DEBUG
        // `-wikipedia-words on|off`: the switch as the test wants it, and the choice not kept
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-wikipedia-words"), i + 1 < args.count {
            on = args[i + 1] == "on"
            held = true
            return
        }
        #endif
        held = false
        on = UserDefaults.standard.bool(forKey: Self.key)
    }

    func toggle() {
        on.toggle()
        if !held { UserDefaults.standard.set(on, forKey: Self.key) }
        NotificationCenter.default.post(name: Self.changed, object: nil)
    }
}

/// One article's words as bundled: the title as Wikipedia shows it, its short description, the lead
/// as plain text, its URL and the revision the words come from.
struct WikipediaArticle: Equatable, Decodable, Sendable {
    let title: String
    let description: String
    let extract: String
    let url: URL
    let timestamp: String?

    private enum CodingKeys: String, CodingKey { case title, description, extract, url, timestamp }

    init(title: String, description: String = "", extract: String, url: URL, timestamp: String? = nil) {
        self.title = title
        self.description = description
        self.extract = extract
        self.url = url
        self.timestamp = timestamp
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        title = try c.decode(String.self, forKey: .title)
        description = (try? c.decode(String.self, forKey: .description)) ?? ""
        extract = try c.decode(String.self, forKey: .extract)
        url = try c.decode(URL.self, forKey: .url)
        timestamp = try? c.decode(String.self, forKey: .timestamp)
    }

    /// For a card: the first sentence, and the second while the two stay under 300 characters.
    var snippet: String { Self.sentences(extract, upTo: 300) }

    /// For the foot of the screen: the first sentence, cut at a word past 170 characters.
    var line: String {
        let first = Self.sentences(extract, upTo: 0)
        guard first.count > 170 else { return first }
        let head = first.prefix(170)
        let cut = head.lastIndex(of: " ").map { head[..<$0] } ?? head
        return String(cut).trimmingCharacters(in: CharacterSet(charactersIn: " ,;:")) + "…"
    }

    /// The description as a line under the title: "Mediterranean between Greece and Turkey".
    var descriptionLine: String {
        guard let first = description.first else { return "" }
        return first.uppercased() + description.dropFirst()
    }

    /// The year of the revision the words come from, for the fine print.
    var year: String? { timestamp.map { String($0.prefix(4)) } }

    /// The first sentence of `text`, then each next one while the whole stays within `limit` characters
    /// (0: the first alone). The whole text when the tokenizer finds no sentence in it.
    static func sentences(_ text: String, upTo limit: Int) -> String {
        var out = ""
        text.enumerateSubstrings(in: text.startIndex..., options: [.bySentences]) { piece, _, _, stop in
            guard let piece = piece?.trimmingCharacters(in: .whitespacesAndNewlines), !piece.isEmpty else { return }
            if out.isEmpty { out = piece; return }
            if out.count + 1 + piece.count <= limit { out += " " + piece } else { stop = true }
        }
        return out.isEmpty ? text.trimmingCharacters(in: .whitespacesAndNewlines) : out
    }
}

/// The bundled articles, by the key the atlas uses for each (a country's name, a passage's subject). The
/// app's is `Wikipedia.json`; a pack's `wikipedia.json` is laid over it for that pack's words.
final class WikipediaIndex: Sendable {
    static let shared = WikipediaIndex(url: Bundle.main.url(forResource: "Wikipedia", withExtension: "json"))

    private let articles: [String: WikipediaArticle]
    /// The day the words were fetched (ISO 8601), for the fine print.
    let fetched: String?

    private struct File: Decodable {
        let fetched: String?
        let articles: [String: WikipediaArticle]
    }

    init(articles: [String: WikipediaArticle], fetched: String? = nil) {
        self.articles = articles
        self.fetched = fetched
    }

    /// From a file; empty when there is none or it can't be read (the words then fall away, nothing breaks).
    convenience init(url: URL?) {
        guard let url, let data = try? Data(contentsOf: url),
              let file = try? JSONDecoder().decode(File.self, from: data) else {
            self.init(articles: [:])
            return
        }
        self.init(articles: file.articles, fetched: file.fetched)
    }

    var isEmpty: Bool { articles.isEmpty }
    var count: Int { articles.count }

    subscript(key: String?) -> WikipediaArticle? {
        guard let key else { return nil }
        return articles[key]
    }

    /// This index with another's articles over it (a pack's over the app's).
    func overlaid(with other: WikipediaIndex?) -> WikipediaIndex {
        guard let other, !other.isEmpty else { return self }
        return WikipediaIndex(articles: articles.merging(other.articles) { _, theirs in theirs }, fetched: other.fetched ?? fetched)
    }
}

/// What stays when Wikipedia's words are on.
enum WordsRule {
    /// A tip stays: one sentence, under 120 characters (what to touch, where you are). A paragraph goes.
    static func isTip(_ text: String) -> Bool {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, t.count <= 120, !t.contains("\n") else { return false }
        var sentences = 0
        t.enumerateSubstrings(in: t.startIndex..., options: [.bySentences]) { piece, _, _, stop in
            if let piece, !piece.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { sentences += 1 }
            if sentences > 1 { stop = true }
        }
        return sentences <= 1
    }

    /// The text to show for a caption or a line: the article's first sentence when there is one, the
    /// atlas's own words when they are a tip, else nothing.
    static func line(_ text: String, article: WikipediaArticle?) -> String {
        guard WikipediaWords.shared.on else { return text }
        if let article { return article.line }
        return isTip(text) ? text : ""
    }
}

/// A paragraph of the atlas's own words, or, with Wikipedia's words on, Wikipedia's for its subject: the
/// article's card, the words themselves when they are a tip, else nothing.
///
///     Words(text: body, wikipedia: "Dorudon", tone: tone) { text in Text(text) }
struct Words<Own: View>: View {
    let text: String
    let article: WikipediaArticle?
    var tone = WikipediaCard.Tone.paper
    let own: (String) -> Own

    /// By the article's key, looked up in `index` (the app's, or a pack's laid over it).
    init(text: String, wikipedia: String?, tone: WikipediaCard.Tone = .paper, index: WikipediaIndex = .shared,
         @ViewBuilder own: @escaping (String) -> Own) {
        self.init(text: text, article: index[wikipedia], tone: tone, own: own)
    }

    /// With the article already found (a pack panel's, resolved when it was shown).
    init(text: String, article: WikipediaArticle?, tone: WikipediaCard.Tone = .paper, @ViewBuilder own: @escaping (String) -> Own) {
        self.text = text
        self.article = article
        self.tone = tone
        self.own = own
    }

    var body: some View {
        if WikipediaWords.shared.on {
            if let article {
                WikipediaCard(article: article, tone: tone)
            } else if WordsRule.isTip(text) {
                own(text)
            }
        } else {
            own(text)
        }
    }
}
