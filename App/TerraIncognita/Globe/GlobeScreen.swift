import Foundation
import SwiftUI
import UIKit

/// The whole globe, with a caption naming the chosen country, or saying what to do. Kept once made:
/// `active` false while the film shows instead.
struct GlobeScreen: View {
    let active: Bool
    let style: GlobeStyle
    /// What else covers the screen (the buttons), in global coordinates: no name goes under it.
    let keepClear: [CGRect]
    /// Its first frame is on screen.
    let onDrawn: () -> Void
    @State private var country: GlobeData.Country?
    @State private var captionFrame = CGRect.zero

    init(active: Bool, style: GlobeStyle, keepClear: [CGRect] = [], onDrawn: @escaping () -> Void = {}) {
        self.active = active
        self.style = style
        self.keepClear = keepClear
        self.onDrawn = onDrawn
        GlobeData.registerFonts()
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            GlobeCanvasView(active: active, style: style, keepClear: keepClear + [captionFrame], onDrawn: onDrawn, country: $country)
                .ignoresSafeArea()
            Group {
                if style == .persian {
                    PersianCaption(caption: caption)
                } else {
                    CaptionView(caption: caption, style: Self.style)
                }
            }
            .id(caption.title + style.rawValue)
            .transition(.opacity.combined(with: .offset(y: 8)))
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { captionFrame = $0 }
        }
        .animation(.easeInOut(duration: 0.35), value: caption.title)
        .animation(.easeInOut(duration: 0.35), value: style)
    }

    private var caption: Plate.Caption {
        guard let c = country else {
            return Plate.Caption(eyebrow: String(localized: "The globe"), title: String(localized: "The whole world"),
                                 text: String(localized: "Spin it, pinch it, tap a country."))
        }
        let region = [c.sub, c.cont].compactMap { $0 }.first ?? ""
        var facts: [String] = []
        if let cap = c.cap { facts.append(String(localized: "Capital \(cap)")) }
        if let pop = c.pop, pop > 0 { facts.append(Self.people(pop)) }
        return Plate.Caption(eyebrow: region, title: c.long ?? c.n, text: facts.joined(separator: " · "))
    }

    /// "34 million people", to two figures.
    private static func people(_ n: Double) -> String {
        func figures(_ x: Double) -> String { x >= 10 ? String(Int(x.rounded())) : x.formatted(.number.precision(.fractionLength(0...1))) }
        if n >= 1e9 { return String(localized: "\(figures(n / 1e9)) billion people") }
        if n >= 1e6 { return String(localized: "\(figures(n / 1e6)) million people") }
        if n >= 1e3 { return String(localized: "\(figures(n / 1e3)) thousand people") }
        return String(localized: "\(Int(n)) people")
    }

    /// The caption in Libertinus, Wikipedia's serif, over the globe's paper; the eyebrow in the
    /// locator's red.
    static let style = CaptionStyle(
        fonts: nil,
        eyebrow: .init(font: "LibertinusSerif-Regular", size: 12.5, color: "#C12838", tracking: 2.2, uppercase: true, italic: nil),
        title: .init(font: "LibertinusSerif-Semibold", size: 30, color: "#2A2925", tracking: nil, uppercase: nil, italic: nil),
        text: .init(font: "LibertinusSerif-Italic", size: 17, color: "#5A564E", tracking: nil, uppercase: nil, italic: nil),
        scrim: "#F6F2E7F2")

    /// The Persian caption's type: the eyebrow in lapis, the title in the manuscript's ink.
    static let persianStyle = CaptionStyle(
        fonts: nil,
        eyebrow: .init(font: "LibertinusSerif-Regular", size: 12.5, color: "#4E5B9B", tracking: 2.2, uppercase: true, italic: nil),
        title: .init(font: "LibertinusSerif-Semibold", size: 28, color: "#2A2119", tracking: nil, uppercase: nil, italic: nil),
        text: .init(font: "LibertinusSerif-Italic", size: 17, color: "#5A4E40", tracking: nil, uppercase: nil, italic: nil),
        scrim: nil)
}

/// The caption in the Persian style, as the manuscript opens its text: an illuminated headpiece with
/// the title lettered in its gold cartouche, over a quiet panel ruled in gold, pink and lapis.
private struct PersianCaption: View {
    let caption: Plate.Caption
    /// The headpiece (Globe/headpiece.png), and the box inside its cartouche that the title may
    /// fill, as fractions of it (tools/globe/ornament.py).
    static let headpiece = UIImage(contentsOfFile: GlobeData.root.appending(path: "headpiece.png").path(percentEncoded: false))
    static let cartouche = CGRect(x: 0.29, y: 0.2, width: 0.42, height: 0.6)

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let head = Self.headpiece {
                Image(uiImage: head)
                    .resizable()
                    .aspectRatio(head.size.width / max(head.size.height, 1), contentMode: .fit)
                    .overlay {
                        GeometryReader { g in
                            Text(verbatim: caption.title)
                                .font(.custom("LibertinusSerif-Semibold", size: 22, relativeTo: .title))
                                .foregroundStyle(Color(red: 0x2A / 255, green: 0x21 / 255, blue: 0x19 / 255))
                                .lineLimit(1)
                                .minimumScaleFactor(0.4)
                                .frame(width: g.size.width * Self.cartouche.width, height: g.size.height * Self.cartouche.height)
                                .position(x: g.size.width * Self.cartouche.midX, y: g.size.height * Self.cartouche.midY)
                        }
                    }
            }
            CaptionLines(caption: Plate.Caption(eyebrow: caption.eyebrow, title: Self.headpiece == nil ? caption.title : "", text: caption.text),
                         style: GlobeScreen.persianStyle)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.horizontal, 16)
                .padding(.vertical, 12)
        }
        .padding(PersianPanel.inset)
        .background(PersianPanel())
        .frame(maxWidth: CaptionLines.widest)
        .padding(.horizontal, 14)
        .padding(.bottom, 10)
        .allowsHitTesting(false)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Text(verbatim: [caption.title, caption.eyebrow, caption.text].filter { !$0.isEmpty }.joined(separator: ", ")))
    }
}

/// The panel's ground and its rulings.
private struct PersianPanel: View {
    /// The rulings' depth, in points.
    static let inset: CGFloat = 5

    var body: some View {
        Canvas { ctx, size in
            func colour(_ hex: UInt32) -> Color {
                Color(red: Double((hex >> 16) & 0xFF) / 255, green: Double((hex >> 8) & 0xFF) / 255, blue: Double(hex & 0xFF) / 255)
            }
            let r = CGRect(origin: .zero, size: size)
            ctx.fill(Path(r), with: .color(colour(0xF7F0DF)))
            ctx.stroke(Path(r.insetBy(dx: 1, dy: 1)), with: .color(colour(0xA8894F)), lineWidth: 2)
            ctx.stroke(Path(r.insetBy(dx: 3.2, dy: 3.2)), with: .color(colour(0xD59A86)), lineWidth: 0.8)
            ctx.stroke(Path(r.insetBy(dx: Self.inset - 0.25, dy: Self.inset - 0.25)), with: .color(colour(0x4E5B9B)), lineWidth: 0.5)
        }
    }
}

private struct GlobeCanvasView: UIViewRepresentable {
    let active: Bool
    let style: GlobeStyle
    /// In global (the window's) coordinates.
    let keepClear: [CGRect]
    let onDrawn: () -> Void
    @Binding var country: GlobeData.Country?

    func makeUIView(context: Context) -> GlobeView {
        let view = GlobeView(frame: .zero)
        view.setStyle(style, animated: false)
        view.onSelect = { country = $0 }
        view.onDrawn = onDrawn
        // (a country lit from the start, by -globe-select; told after this update, not during it)
        let initial = view.selectedCountry
        Task { country = initial }
        return view
    }

    func updateUIView(_ view: GlobeView, context: Context) {
        if view.suspended == active { view.suspended = !active }
        if view.style != style { view.setStyle(style, animated: true) }
        view.keepClear = keepClear.filter { !$0.isEmpty }.map { view.convert($0, from: nil) }
    }
}
