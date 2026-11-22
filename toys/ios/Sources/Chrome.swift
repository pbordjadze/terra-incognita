// The chrome as data (toy-kit's chrome.rs): what surrounds the toy, read once a frame when it changed
// (toy_chrome), what the toy asks of the frame (toy_events: haptics as intents, a file), and what the reader
// did in the frame's own controls, sent back (toy_send). The kit says what; this file and main.swift decide
// how it looks on an iPhone.
import UIKit

struct Chrome: Decodable, Equatable {
    struct About: Decodable, Equatable { let name: String; let place: String; let line: String }
    struct Mark: Decodable, Equatable {
        /// "none", "demo" or "sheet"
        let what: String
        let off: Bool
        /// Where the kit draws the mark (points), unless the frame draws its own.
        let at: [Double]?
    }
    struct Switch: Decodable, Equatable { let label: String; let on: Bool }
    struct Credit: Decodable, Equatable {
        let name: String; let maker: String; let place: String; let year: Int; let credit: String
    }
    struct Sheet: Decodable, Equatable {
        /// "rules" or "origins"
        let page: String
        let lines: [String]
        let note: String
        let switches: [Switch]
        let style: Credit?
        let demo: Bool
        let origins: Bool
        let top: Bool
    }
    struct Label: Decodable, Equatable {
        /// Its box in points: left, top, width, height.
        let at: [Double]
        let alpha: Double
        let dark: Bool
        let lines: [String]
    }
    struct Button: Decodable, Equatable { let label: String; let at: [Double] }
    struct Senses: Decodable, Equatable {
        let shake: Bool; let gravity: Bool; let field: Bool; let heading: Bool
        var any: Bool { shake || gravity || field || heading }
        /// The compass: the magnetometer, which costs more than the rest.
        var compass: Bool { field || heading }
    }
    let about: About?
    let mark: Mark
    let sheet: Sheet?
    let caption: String?
    let label: Label?
    let title: String?
    let buttons: [Button]
    let back: String?
    let music: Bool
    let senses: Senses
}

/// What the toy asks of the frame, once.
struct ToyEvent: Decodable {
    /// "haptic" or "file"
    let event: String
    /// "tick", "soft", "firm" or "thud", for a haptic
    let haptic: String?
}

extension ToyView {
    /// The chrome, if it changed since the last frame.
    func readChrome() -> Chrome? {
        guard let shell, let p = toy_chrome(shell) else { return nil }
        return try? JSONDecoder().decode(Chrome.self, from: Data(String(cString: p).utf8))
    }

    /// What the toy asks now: each haptic played as iOS plays its kind, the Files picker for a file.
    func takeEvents() {
        guard let shell, let p = toy_events(shell),
              let events = try? JSONDecoder().decode([ToyEvent].self, from: Data(String(cString: p).utf8)) else { return }
        for e in events {
            switch e.event {
            case "haptic": feel(e.haptic ?? "tick")
            case "file": pickFile()
            default: break
            }
        }
    }

    /// A haptic intent in UIKit's impacts: a tick the lightest, a thud the heaviest.
    func feel(_ kind: String) {
        switch kind {
        case "soft": softHaptic.impactOccurred()
        case "firm": firmHaptic.impactOccurred()
        case "thud": thudHaptic.impactOccurred()
        default: haptic.impactOccurred()
        }
    }

    /// Tells the toy what the reader did in the frame's controls (toy-kit's `chrome::In`, as JSON).
    func tell(_ message: [String: Any]) {
        guard let shell, let data = try? JSONSerialization.data(withJSONObject: message),
              let json = String(data: data, encoding: .utf8) else { return }
        json.withCString { toy_send(shell, $0) }
    }

    /// A message with no fields ("rules", "origins", "close").
    func tell(_ word: String) {
        guard let shell else { return }
        "\"\(word)\"".withCString { toy_send(shell, $0) }
    }

    /// The sheet's text as the glass shows it: the lines, then a blank line and the dimmer notes (the toy's
    /// own, then the style's credit).
    static func sheetText(_ s: Chrome.Sheet) -> String {
        var notes = s.note.isEmpty ? [] : [s.note]
        if let c = s.style { notes.append(c.credit) }
        let body = s.lines.joined(separator: "\n")
        return notes.isEmpty ? body : body + "\n\n" + notes.joined(separator: "\n")
    }
}
