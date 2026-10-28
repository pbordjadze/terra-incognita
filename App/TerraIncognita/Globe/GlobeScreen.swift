import Foundation
import SwiftUI
import UIKit

/// The whole globe, with a caption naming the chosen country, or saying what to do, and an active
/// prototype's invitation above it if it has one. `active` false while a screen or the workbench covers it.
struct GlobeScreen: View {
    let active: Bool
    let style: GlobeStyle
    /// What else covers the screen (the buttons), in global coordinates: no name goes under it.
    let keepClear: [CGRect]
    let invitation: Invitation?
    let onInvitation: () -> Void
    /// Its first frame is on screen: the view, for the playground's prototypes.
    let onDrawn: (GlobeView) -> Void
    @State private var country: GlobeData.Country?
    @State private var captionFrame = CGRect.zero
    @Environment(\.dynamicTypeSize) private var typeSize

    init(active: Bool, style: GlobeStyle, keepClear: [CGRect] = [], invitation: Invitation? = nil,
         onInvitation: @escaping () -> Void = {}, onDrawn: @escaping (GlobeView) -> Void = { _ in }) {
        self.active = active
        self.style = style
        self.keepClear = keepClear
        self.invitation = invitation
        self.onInvitation = onInvitation
        self.onDrawn = onDrawn
        GlobeData.registerFonts()
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            GlobeCanvasView(active: active, style: style, keepClear: keepClear + [captionFrame], onDrawn: onDrawn, country: $country)
                .ignoresSafeArea()
            VStack(spacing: 0) {
                if let invitation {
                    Button(action: onInvitation) {
                        VStack(spacing: 4) {
                            Text(invitation.title)
                                .font(.custom("LibertinusSerif-Regular", size: 23, relativeTo: .title2))
                            Text(invitation.action)
                                .font(.custom("LibertinusSerif-Regular", size: 17, relativeTo: .body))
                        }
                        .foregroundStyle(Color(red: 0.23, green: 0.17, blue: 0.09))
                        .padding(14)
                        .frame(maxWidth: .infinity)
                        .background(Color(red: 0.95, green: 0.89, blue: 0.78))
                        .overlay(alignment: .top) { Rectangle().fill(Color(red: 0.77, green: 0.67, blue: 0.43)).frame(height: 1) }
                        .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityIdentifier(invitation.id)
                    .padding(.horizontal, 30)
                    .padding(.bottom, 8)
                }
                if typeSize.isAccessibilitySize && country == nil && invitation != nil {
                    // The invitation already names this view; leave room for the globe at large type.
                    EmptyView()
                } else if style == .persian {
                    PersianCaption(caption: caption)
                } else {
                    CaptionView(caption: caption, style: Self.style)
                }
            }
            .id(caption.title + style.rawValue + (invitation?.id ?? ""))
            .transition(.opacity.combined(with: .offset(y: 8)))
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { captionFrame = $0 }
        }
        .animation(.easeInOut(duration: 0.35), value: caption.title)
        .animation(.easeInOut(duration: 0.35), value: style)
        .animation(.easeInOut(duration: 0.35), value: invitation)
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
        let title = style == .persian ? Self.cartouche(c.n) : (c.long ?? c.n)
        return Plate.Caption(eyebrow: region, title: title, text: facts.joined(separator: " · "))
    }

    /// The name the Persian letters in the headpiece's cartouche: the short name, "&" for "and" (as
    /// Natural Earth's own abbreviations write it), so the longest still fit a 375 pt phone's
    /// cartouche at 13 pt (`PersianCaption`'s floor); tools/globe/preview.py's `cartouche` mirrors it.
    static func cartouche(_ name: String) -> String {
        switch name {
        case "United States of America": "United States"
        case "Heard I. and McDonald Is.": "Heard & McDonald Is."
        default: name.replacingOccurrences(of: " and ", with: " & ")
        }
    }

    /// "34 million people", to two figures.
    private static func people(_ n: Double) -> String {
        func figures(_ x: Double) -> String { x >= 10 ? String(Int(x.rounded())) : x.formatted(.number.precision(.fractionLength(0...1))) }
        if n >= 1e9 { return String(localized: "\(figures(n / 1e9)) billion people") }
        if n >= 1e6 { return String(localized: "\(figures(n / 1e6)) million people") }
        if n >= 1e3 { return String(localized: "\(figures(n / 1e3)) thousand people") }
        return String(localized: "\(Int(n)) people")
    }

    /// The captions' type in each style (Globe/type.json): Libertinus, Wikipedia's serif, over the
    /// globe's paper, the eyebrow in the locator's red; in the Persian, the eyebrow in lapis and the
    /// title in the manuscript's ink.
    static let style = GlobeType.shared?.styles[GlobeStyle.wikipedia.rawValue]?.caption
    static let persianStyle = GlobeType.shared?.styles[GlobeStyle.persian.rawValue]?.caption
}

/// The caption in the Persian style, as the manuscript opens its text: an illuminated headpiece with
/// the title lettered in its gold cartouche, over a quiet panel ruled in gold, pink and lapis.
struct PersianCaption: View {
    let caption: Plate.Caption
    /// How far in from the safe area the caption sits, below and at the sides: inside the page's
    /// frame (Globe.metal's PAGE_MARGIN and PAGE_DEPTH, 22.81 reading points, at most 27.4 points on
    /// the largest screen), which comes in when the globe is looked at closer.
    static let pageRoom: CGFloat = 30
    /// The headpiece (Globe/headpiece.png), and the title's face and the box inside the cartouche
    /// it may fill, as fractions of the headpiece (Globe/type.json).
    static let headpiece = UIImage(contentsOfFile: GlobeData.root.appending(path: "headpiece.png").path(percentEncoded: false))
    static let face = GlobeType.shared?.styles[GlobeStyle.persian.rawValue]?.cartouche
    static let cartouche = face.flatMap { $0.box.count == 4 ? CGRect(x: $0.box[0], y: $0.box[1], width: $0.box[2], height: $0.box[3]) : nil }
        ?? CGRect(x: 0.29, y: 0.2, width: 0.42, height: 0.6)

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let head = Self.headpiece {
                Image(uiImage: head)
                    .resizable()
                    .aspectRatio(head.size.width / max(head.size.height, 1), contentMode: .fit)
                    .overlay {
                        GeometryReader { g in
                            Text(verbatim: caption.title)
                                .font(.custom(Self.face?.font ?? "LibertinusSerif-Semibold", size: Self.face?.size ?? 24, relativeTo: .title))
                                .foregroundStyle(Color(uiColor: Self.face.flatMap { UIColor(hex: $0.ink) } ?? .black))
                                .lineLimit(1)
                                .minimumScaleFactor(0.55)     // (24 pt down to 13.2: the longest name fits a 375 pt phone)
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
        .padding(.horizontal, Self.pageRoom)
        .padding(.bottom, Self.pageRoom)
        .allowsHitTesting(false)
        .accessibilityElement(children: .ignore)
        .accessibilityLabel(Text(verbatim: [caption.title, caption.eyebrow, caption.text].filter { !$0.isEmpty }.joined(separator: ", ")))
    }
}

/// The panel's ground and its rulings: the manuscript's text panels and their gold, pink and lapis
/// rulings (Globe.metal's P_ colours).
private struct PersianPanel: View {
    /// The rulings' depth, in points.
    static let inset: CGFloat = 5

    var body: some View {
        Canvas { ctx, size in
            func colour(_ hex: UInt32) -> Color {
                Color(red: Double((hex >> 16) & 0xFF) / 255, green: Double((hex >> 8) & 0xFF) / 255, blue: Double(hex & 0xFF) / 255)
            }
            let r = CGRect(origin: .zero, size: size)
            ctx.fill(Path(r), with: .color(colour(0xE5DBBE)))       // the text panels' paper
            ctx.stroke(Path(r.insetBy(dx: 1, dy: 1)), with: .color(colour(0xC5AA6E)), lineWidth: 2)
            ctx.stroke(Path(r.insetBy(dx: 3.2, dy: 3.2)), with: .color(colour(0xD1A38F)), lineWidth: 0.8)
            ctx.stroke(Path(r.insetBy(dx: Self.inset - 0.25, dy: Self.inset - 0.25)), with: .color(colour(0x5F6CA7)), lineWidth: 0.5)
        }
    }
}

private struct GlobeCanvasView: UIViewRepresentable {
    let active: Bool
    let style: GlobeStyle
    /// In global (the window's) coordinates.
    let keepClear: [CGRect]
    let onDrawn: (GlobeView) -> Void
    @Binding var country: GlobeData.Country?

    func makeUIView(context: Context) -> GlobeView {
        let view = GlobeView(frame: .zero)
        view.setAtlasAccessibilityIdentifier("mainAtlasGlobe")
        view.setStyle(style, animated: false)
        view.onSelect = { country = $0 }
        view.onDrawn = { [weak view] in if let view { onDrawn(view) } }
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
