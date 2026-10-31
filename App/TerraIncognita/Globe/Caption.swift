import SwiftUI
import UIKit

// The captions' type and the small helpers the globe shares, kept from the plate films (`Atlas/`) when
// they were retired (painting-company row 195; they are in the tag before-retirement-2026-10-31).

/// How a look sets its captions: its own fonts (files in the look's folder), sizes in points, colours
/// as #rrggbb or #rrggbbaa.
nonisolated struct CaptionStyle: Decodable, Sendable, Equatable {
    nonisolated struct Text: Decodable, Sendable, Equatable {
        /// A PostScript name from the look's fonts, or "serif" for the system's.
        let font: String
        let size: Double
        let color: String
        let tracking: Double?
        let uppercase: Bool?
        let italic: Bool?
    }

    let fonts: [String]?
    let eyebrow: Text?
    let title: Text?
    let text: Text?
    let scrim: String?
}

/// A caption's words (`Globe/type.json` sets their type).
nonisolated enum Plate {
    nonisolated struct Caption: Decodable, Sendable, Equatable {
        let eyebrow: String
        let title: String
        let text: String
    }
}

/// A caption in the globe's type.
struct CaptionView: View {
    let caption: Plate.Caption
    let style: CaptionStyle?

    var body: some View {
        CaptionLines(caption: caption, style: style)
            .allowsHitTesting(false)
            .accessibilityElement(children: .combine)
            .frame(maxWidth: CaptionLines.widest, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 24)
            .padding(.top, 70)
            .padding(.bottom, 36)
            // nearly full by the eyebrow's line (about a third of the way down), so the small type stays
            // clear over a busy map; it takes no touches, so the globe under it does
            .background {
                LinearGradient(stops: [.init(color: .clear, location: 0), .init(color: scrim.opacity(0.85), location: 0.3),
                                       .init(color: scrim, location: 0.55)], startPoint: .top, endPoint: .bottom)
                    .allowsHitTesting(false)
            }
    }

    private var scrim: Color { Color(uiColor: style?.scrim.flatMap { UIColor(hex: $0) } ?? UIColor(white: 0, alpha: 0.55)) }
}

/// A caption's three lines (eyebrow, title, text) in a look's type.
struct CaptionLines: View {
    let caption: Plate.Caption
    let style: CaptionStyle?
    /// The widest a caption runs, in points: on an iPad a line any longer is hard to read.
    static let widest: CGFloat = 560

    var body: some View {
        // (a line left empty takes no room)
        VStack(alignment: .leading, spacing: 6) {
            if !caption.eyebrow.isEmpty {
                line(caption.eyebrow, style?.eyebrow, fallback: Fallback(size: 12, color: "#dc7354", tracking: 2.4, uppercase: true), relativeTo: .caption)
            }
            if !caption.title.isEmpty {
                line(caption.title, style?.title, fallback: Fallback(size: 28, color: "#f7ebcc"), relativeTo: .title)
            }
            if !caption.text.isEmpty {
                line(caption.text, style?.text, fallback: Fallback(size: 17, color: "#ebdec2", italic: true), relativeTo: .body)
            }
        }
    }

    private struct Fallback {
        var size: Double, color: String, tracking = 0.0, uppercase = false, italic = false
    }

    /// One line of the caption in the look's type, or the app's serif when the look sets none. A look
    /// that styles the line means what it leaves out: no italic, no capitals, no tracking.
    private func line(_ string: String, _ t: CaptionStyle.Text?, fallback f: Fallback, relativeTo textStyle: Font.TextStyle) -> some View {
        let size = t?.size ?? f.size
        var font: Font = t.map { $0.font == "serif" ? .system(size: size, design: .serif) : .custom($0.font, size: size, relativeTo: textStyle) }
            ?? .system(size: size, design: .serif)
        if t.map({ $0.italic ?? false }) ?? f.italic { font = font.italic() }
        let upper = t.map { $0.uppercase ?? false } ?? f.uppercase
        return Text(verbatim: upper ? string.uppercased() : string)
            .font(font)
            .tracking(t.map { $0.tracking ?? 0 } ?? f.tracking)
            .foregroundStyle(Color(uiColor: UIColor(hex: t?.color ?? f.color) ?? .white))
            .fixedSize(horizontal: false, vertical: true)
    }
}

extension UIColor {
    /// #rrggbb or #rrggbbaa
    convenience init?(hex: String) {
        var s = hex.trimmingCharacters(in: .whitespaces)
        if s.hasPrefix("#") { s.removeFirst() }
        guard s.count == 6 || s.count == 8, let v = UInt64(s, radix: 16) else { return nil }
        let rgba = s.count == 6 ? (v << 8) | 0xFF : v
        self.init(red: CGFloat((rgba >> 24) & 0xFF) / 255, green: CGFloat((rgba >> 16) & 0xFF) / 255,
                  blue: CGFloat((rgba >> 8) & 0xFF) / 255, alpha: CGFloat(rgba & 0xFF) / 255)
    }
}

nonisolated func smoothstep(_ a: Double, _ b: Double, _ x: Double) -> Double {
    let t = min(max((x - a) / (b - a), 0), 1)
    return t * t * (3 - 2 * t)
}

nonisolated func easeInOut(_ t: Double) -> Double {
    let t = min(max(t, 0), 1)
    return t < 0.5 ? 4 * t * t * t : 1 - pow(-2 * t + 2, 3) / 2
}
