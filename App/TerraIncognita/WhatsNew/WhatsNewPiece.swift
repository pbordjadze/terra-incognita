import Foundation

// What's new (painting-company row 187): the owner's two minutes with a build. The pieces the sessions
// shipped are played one at a time from where their makers meant them to be seen; after each, one tap
// (and a voice note or a picture when wanted), and the next. This file is the module's data: a piece,
// an answer, the ledger of what has been seen. It knows nothing of the globe: `start` is the host app's
// to read (TerraWhatsNew.swift), so the same module can serve Paint by Moonlight and Pigeon.
// docs/whatsnew.md is the contract. (This file is plain data, compiled in every build: the pack
// manifest names its type; the mode itself is UNRELEASED.)

/// Free-form JSON, kept as it was written: a piece's `start` (what the host app does with it) and an
/// answer's `state` (what the host app says the reader saw).
nonisolated enum JSONValue: Codable, Equatable, Hashable, Sendable {
    case string(String)
    case number(Double)
    case bool(Bool)
    case null
    case array([JSONValue])
    case object([String: JSONValue])

    init(from decoder: Decoder) throws {
        let c = try decoder.singleValueContainer()
        if c.decodeNil() { self = .null }
        else if let b = try? c.decode(Bool.self) { self = .bool(b) }
        else if let n = try? c.decode(Double.self) { self = .number(n) }
        else if let s = try? c.decode(String.self) { self = .string(s) }
        else if let a = try? c.decode([JSONValue].self) { self = .array(a) }
        else if let o = try? c.decode([String: JSONValue].self) { self = .object(o) }
        else { throw DecodingError.dataCorruptedError(in: c, debugDescription: "not JSON") }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.singleValueContainer()
        switch self {
        case .string(let s): try c.encode(s)
        case .number(let n): try c.encode(n)
        case .bool(let b): try c.encode(b)
        case .null: try c.encodeNil()
        case .array(let a): try c.encode(a)
        case .object(let o): try c.encode(o)
        }
    }

    subscript(key: String) -> JSONValue? {
        if case .object(let o) = self { return o[key] }
        return nil
    }
    var string: String? { if case .string(let s) = self { return s }; return nil }
    var double: Double? { if case .number(let n) = self { return n }; return nil }
    var bool: Bool? { if case .bool(let b) = self { return b }; return nil }
    var array: [JSONValue]? { if case .array(let a) = self { return a }; return nil }
    var object: [String: JSONValue]? { if case .object(let o) = self { return o }; return nil }
}

/// One thing to look at: who it is, where it starts, what its makers would like to know.
/// A session ships one with its work (docs/whatsnew.md): a `whatsNew` entry in a pack's
/// prototype.json, or an entry in the app's WhatsNew.json for native work.
nonisolated struct WhatsNewPiece: Codable, Equatable, Identifiable, Sendable {
    /// Unique and never reused; a pack's piece is the pack's id.
    var id: String
    var title: String
    /// One line: what to look at, in words the owner reads while it plays.
    var what: String?
    /// The one question its makers most want answered (optional; shown under the title).
    var ask: String?
    /// The day it landed, YYYY-MM-DD: the queue plays the newest first.
    var since: String
    /// Bumped by its makers when it changed enough to ask again ("1" if absent). The same piece
    /// answered at another version is new again.
    var version: String?
    /// Where to start, in the host app's terms (Terra: `run`, `camera`, `date`, `playing`,
    /// `wikipedia`, `clouds`, `select`, `open`; docs/whatsnew.md).
    var start: JSONValue?
    /// The makers' pointer (a painting-company row), kept with the answers, never shown.
    var row: Int?

    var versionOrDefault: String { version ?? "1" }

    /// Valid enough to play: an id, a title and a day written YYYY-MM-DD.
    var isSound: Bool {
        !id.isEmpty && id.count <= 60 && !title.isEmpty && Self.isDay(since)
    }

    static func isDay(_ s: String) -> Bool {
        let b = Array(s.utf8)
        guard b.count == 10, b[4] == 45, b[7] == 45 else { return false }
        for (i, c) in b.enumerated() where i != 4 && i != 7 { if c < 48 || c > 57 { return false } }
        return true
    }
}

/// The one tap: how it landed. `skip` is an answer too (not now), so a skipped piece stops asking
/// until its makers change it; the list keeps the way back to it.
nonisolated enum WhatsNewReaction: String, Codable, CaseIterable, Sendable {
    case wow, nice, meh, broken, skip

    var label: String {
        switch self {
        case .wow: "Wow"
        case .nice: "Nice"
        case .meh: "Meh"
        case .broken: "Didn't work"
        case .skip: "Skipped"
        }
    }
    var mark: String {
        switch self {
        case .wow: "🤩"
        case .nice: "🙂"
        case .meh: "😐"
        case .broken: "🐛"
        case .skip: "→"
        }
    }
}

/// What the owner said of one piece, and what was on the screen when they said it. One file each
/// (`<time>-<piece>.json`), with the voice note and picture beside it when there are any; the
/// private repository gets them as they are.
nonisolated struct WhatsNewAnswer: Codable, Equatable, Sendable {
    var app: String
    var build: String
    var appVersion: String
    var device: String
    var os: String
    var piece: String
    var pieceVersion: String
    var title: String
    var ask: String?
    var row: Int?
    var reaction: WhatsNewReaction
    /// When, with the device's offset (Eastern, where the owner is).
    var at: String
    /// How long the piece was on the screen before the answer.
    var seconds: Double
    /// The voice note's and the picture's file names, beside this file.
    var voice: String?
    var screenshot: String?
    /// The host app's account of the screen at that moment (Terra: the camera, the date, what ran).
    var state: JSONValue?
    /// The round it was part of (its start time), to read a session's answers together.
    var round: String
}

/// What has been seen and answered, by piece id: the version answered and how. Kept on the device;
/// a build shows only what isn't here at its current version.
nonisolated struct WhatsNewLedger: Codable, Equatable, Sendable {
    nonisolated struct Entry: Codable, Equatable, Sendable {
        var version: String
        var reaction: WhatsNewReaction
        var at: String
    }
    var entries: [String: Entry] = [:]

    /// Not yet answered at its current version (never seen, or changed since).
    func isDue(_ piece: WhatsNewPiece) -> Bool { entries[piece.id]?.version != piece.versionOrDefault }
    /// Seen before, at another version.
    func isChanged(_ piece: WhatsNewPiece) -> Bool { entries[piece.id] != nil && isDue(piece) }
    func reaction(for piece: WhatsNewPiece) -> WhatsNewReaction? { entries[piece.id]?.reaction }

    mutating func record(_ piece: WhatsNewPiece, _ reaction: WhatsNewReaction, at: String) {
        entries[piece.id] = Entry(version: piece.versionOrDefault, reaction: reaction, at: at)
    }
}

/// The order things are shown in: the newest first, then as their makers listed them.
nonisolated enum WhatsNewQueue {
    static func ordered(_ pieces: [WhatsNewPiece]) -> [WhatsNewPiece] {
        pieces.enumerated().sorted { a, b in
            if a.element.since != b.element.since { return a.element.since > b.element.since }
            return a.offset < b.offset
        }.map(\.element)
    }

    /// What a build has to show: the sound pieces not yet answered at their version, newest first.
    static func due(_ pieces: [WhatsNewPiece], ledger: WhatsNewLedger) -> [WhatsNewPiece] {
        ordered(pieces.filter { $0.isSound && ledger.isDue($0) })
    }

    /// One id each, the central list's order first (it frames the build), then the packs' own entries;
    /// a pack's entry wins over a central one of the same id (the pack travels, its entry with it).
    static func merged(central: [WhatsNewPiece], packs: [WhatsNewPiece]) -> [WhatsNewPiece] {
        let own = Dictionary(packs.map { ($0.id, $0) }, uniquingKeysWith: { a, _ in a })
        var all = central.map { own[$0.id] ?? $0 }
        var seen = Set(all.map(\.id))
        for p in packs where seen.insert(p.id).inserted { all.append(p) }
        return all
    }
        return all
    }
}
