import SwiftUI

/// Wikipedia's words for one subject, where the atlas's paragraph was: a small W and "From Wikipedia",
/// the article's title and its short description, its first sentence or two as written, the link to the
/// article, and the licence line its words ask for. In Libertinus, the type Wikipedia's own mark is cut
/// in, so the card reads as the atlas's page and not as a web page pasted in. A tone fits it to the sheet
/// it lies on (each experience's ink and paper).
struct WikipediaCard: View {
    let article: WikipediaArticle
    var tone = Tone.paper

    /// The card's colours on a sheet: its ink, a fainter ink for the small lines, and the link's colour.
    struct Tone {
        var ink: Color
        var faint: Color
        var accent: Color
        /// The pack panel's paper.
        static let paper = Tone(ink: Color(red: 0.145, green: 0.165, blue: 0.18), faint: Color(red: 0.36, green: 0.38, blue: 0.39),
                                accent: Color(red: 0.2, green: 0.36, blue: 0.62))
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 7) {
            HStack(alignment: .firstTextBaseline, spacing: 6) {
                WikipediaMark(size: 15)
                Text("From Wikipedia")
                    .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
                    .tracking(1.6)
                    .textCase(.uppercase)
            }
            .foregroundStyle(tone.faint)
            .accessibilityHidden(true)
            Text(verbatim: article.title)
                .font(.custom("LibertinusSerif-Semibold", size: 21, relativeTo: .title3))
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityAddTraits(.isHeader)
                .accessibilityLabel(Text("From Wikipedia: \(article.title)"))
            if !article.description.isEmpty {
                Text(verbatim: article.descriptionLine)
                    .font(.custom("LibertinusSerif-Italic", size: 16, relativeTo: .callout))
                    .foregroundStyle(tone.faint)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Text(verbatim: article.snippet)
                .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
                .lineSpacing(3)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, 2)
            HStack(alignment: .firstTextBaseline) {
                WikipediaLink(url: article.url, color: tone.accent, long: true)
                Spacer(minLength: 8)
                Text(verbatim: "CC BY-SA 4.0" + (article.year.map { " · \($0)" } ?? ""))
                    .font(.custom("LibertinusSerif-Regular", size: 11, relativeTo: .caption2))
                    .foregroundStyle(tone.faint)
                    .accessibilityLabel(Text("Licence CC BY-SA 4.0"))
            }
            .padding(.top, 2)
        }
        .foregroundStyle(tone.ink)
        .padding(.leading, 14)
        .padding(.vertical, 2)
        .overlay(alignment: .leading) {
            RoundedRectangle(cornerRadius: 1).fill(tone.accent.opacity(0.55)).frame(width: 2)
        }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("wikipediaCard")
    }
}

/// The W, in the type Wikipedia's wordmark is cut in (Libertinus is Linux Libertine's heir).
struct WikipediaMark: View {
    var size: CGFloat = 15
    var body: some View {
        Text(verbatim: "W")
            .font(.custom("LibertinusSerif-Semibold", size: size, relativeTo: .caption))
            .accessibilityHidden(true)
    }
}

/// "Read on Wikipedia ↗", or the short "Wikipedia ↗" at the foot: Safari opens it only when tapped.
struct WikipediaLink: View {
    let url: URL
    var color: Color
    var long = false

    var body: some View {
        Link(destination: url) {
            HStack(alignment: .firstTextBaseline, spacing: 4) {
                if !long { WikipediaMark(size: 13) }
                Text(long ? "Read on Wikipedia" : "Wikipedia")
                    .font(.custom("LibertinusSerif-Regular", size: long ? 16 : 14, relativeTo: long ? .callout : .footnote))
                Image(systemName: "arrow.up.right")
                    .font(.system(size: long ? 11 : 10, weight: .semibold))
                    .accessibilityHidden(true)
            }
            .frame(minHeight: 44)
            .contentShape(Rectangle())
        }
        .foregroundStyle(color)
        .accessibilityLabel(Text("Read on Wikipedia"))
        .accessibilityHint(Text("Opens the article in Safari"))
        .accessibilityIdentifier("wikipediaLink")
    }
}

/// The switch, beside the clouds': a W in glass; on, the W white on the ink's disc. VoiceOver sees a toggle.
struct WikipediaButton: View {
    private var words: WikipediaWords { .shared }
    private static let ink = Color(red: 0.145, green: 0.165, blue: 0.18)

    var body: some View {
        Button { words.toggle() } label: {
            ZStack {
                if words.on { Circle().fill(Self.ink).frame(width: 30, height: 30) }
                Text(verbatim: "W")
                    .font(.custom("LibertinusSerif-Semibold", size: 20))
                    .foregroundStyle(words.on ? Color.white : Color.primary)
            }
            .frame(width: 44, height: 44)
            .contentShape(Rectangle())
        }
        .buttonStyle(.glass)
        .accessibilityLabel("Wikipedia's words")
        .accessibilityValue(words.on ? "On" : "Off")
        .accessibilityHint("Puts Wikipedia's words, and links to its articles, in place of the atlas's own")
        .accessibilityAddTraits(.isToggle)
        .accessibilityIdentifier("wikipediaButton")
    }
}
