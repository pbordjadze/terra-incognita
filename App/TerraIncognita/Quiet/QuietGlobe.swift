import Foundation
import SwiftUI

/// The quiet globe (painting-company row 174): four prototypes of the plain globe's own look, each
/// toggled in the workbench against today's, so the morning is a comparison, not an undo. None puts
/// anything on the globe: each asks the stage for a rule (`GlobeRequests`), which goes when it ends,
/// and they run together. The clouds and the sea stay full while they run (`veils`).
enum QuietGlobe {
    /// Almost no names (`GlobeView.quietNames`): the one or two greatest features in view, and what
    /// the reader touches near, for a while; countries last.
    static let names = Prototype(
        id: "quiet-names", title: "Names on demand",
        summary: "Almost no names: the one or two greatest things in view, and what you touch near, for a while. Countries last.",
        veils: false
    ) { stage in
        stage.quietNames()
    }

    /// The clouds clear where the reader looks (`CloudsLayer`, `Clouds.metal`'s quiet rule): gathered
    /// toward the globe's rim zoomed out and the screen's edges close in; close in, their edges drawn
    /// finer (the coarse noise worked out, not read from the tile).
    static let clouds = Prototype(
        id: "quiet-clouds", title: "Clouds clear where you look",
        summary: "The clouds gather toward the globe's rim zoomed out and the screen's edges close in, and clear where you look.",
        veils: false
    ) { stage in
        stage.quietClouds()
    }

    /// The Earth at night (`GlobeStyle.night`): the ground dark, its lights as satellites see them
    /// (NASA's Black Marble, Globe/ground/night.jpg), the clouds a thin veil, the sea still.
    static let night = Prototype(
        id: "night", title: "The Earth at night",
        summary: "The ground dark, with the lights of its cities and roads as satellites see them.",
        veils: false
    ) { stage in
        stage.style = .night
    }

    /// The frame's quietest form (`QuietCaption`): no card at the foot until a country is chosen.
    static let frame = Prototype(
        id: "quiet-frame", title: "The quiet frame",
        summary: "Nothing at the foot until a country is chosen; then one line.",
        veils: false
    ) { stage in
        stage.quietFrame()
    }
}

/// The quiet frame: nothing at rest (the globe takes the room the card had), and for a chosen country
/// one line at the foot, its name and what a map's margin would say of it, on a soft halo of the
/// paper so it stays readable over the globe close in. It never explains what the reader is already
/// doing.
struct QuietCaption: View {
    let country: GlobeData.Country?

    var body: some View {
        if let country {
            Text(verbatim: Self.line(for: country))
                .font(.custom("LibertinusSerif-Italic", size: 16, relativeTo: .callout))
                .foregroundStyle(Color(red: 0.35, green: 0.34, blue: 0.31))
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.horizontal, 14)
                .padding(.vertical, 8)
                .background(Color(red: 0.965, green: 0.949, blue: 0.906).opacity(0.86), in: Capsule())
                .frame(maxWidth: CaptionLines.widest)
                .padding(.horizontal, 24)
                .padding(.bottom, 14)
                .allowsHitTesting(false)
                .accessibilityIdentifier("quietCaption")
        }
    }

    /// "India · New Delhi · 1.4 billion people".
    static func line(for c: GlobeData.Country) -> String {
        var parts = [c.long ?? c.n]
        if let cap = c.cap { parts.append(cap) }
        if let pop = c.pop, pop > 0 { parts.append(GlobeScreen.people(pop)) }
        return parts.joined(separator: " · ")
    }
}
