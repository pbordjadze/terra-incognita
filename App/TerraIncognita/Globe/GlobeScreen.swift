import Foundation
import SwiftUI
import UIKit

/// The whole globe, with a caption naming the country in the locator's red, or saying what to do.
/// Kept once made: `active` false while the film shows instead.
struct GlobeScreen: View {
    let active: Bool
    @State private var country: GlobeData.Country?

    init(active: Bool) {
        self.active = active
        GlobeData.registerFonts()
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            GlobeCanvasView(active: active, country: $country)
                .ignoresSafeArea()
            CaptionView(caption: caption, style: Self.style)
                .id(caption.title)
                .transition(.opacity.combined(with: .offset(y: 8)))
        }
        .animation(.easeInOut(duration: 0.35), value: caption.title)
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
}

private struct GlobeCanvasView: UIViewRepresentable {
    let active: Bool
    @Binding var country: GlobeData.Country?

    func makeUIView(context: Context) -> GlobeView {
        let view = GlobeView(frame: .zero)
        view.onSelect = { country = $0 }
        // (a country lit from the start, by -globe-select; told after this update, not during it)
        let initial = view.selectedCountry
        Task { country = initial }
        return view
    }

    func updateUIView(_ view: GlobeView, context: Context) {
        if view.suspended == active { view.suspended = !active }
    }
}
