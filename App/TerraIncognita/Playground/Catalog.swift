import Foundation
import SwiftUI
import UIKit

/// The native prototypes, one entry each, in the workbench's order. A new one: its code in its own
/// folder under App/TerraIncognita/, and one entry here (docs/playground.md). Packs need no entry:
/// `PackStore` finds their folders and sources. A doorway puts it on the globe while it isn't running,
/// with a small life of its own (runtime 4): a glint now and then from afar, itself from `minZoom`.
enum Catalog {
    /// A doorway's looks: from `minZoom`, the picture itself; before that, from afar, a glint now and
    /// then (every `every` seconds, for 1.2: the eye is drawn before it knows to what). The picture's
    /// look comes first, so at the boundary itself (a double tap lands exactly on 2) the picture wins.
    private static func looks(_ image: UIImage, size: CGSize, minZoom: Double, every: Double, face: Bool) -> [GlobeMarker.SpriteLook] {
        [GlobeMarker.SpriteLook(frames: [image], fps: 1, size: size, minZoom: minZoom, maxZoom: .infinity, every: nil, duration: nil, face: face),
         GlobeMarker.SpriteLook(frames: [Glint.image], fps: 1, size: CGSize(width: 11, height: 11), minZoom: 0, maxZoom: minZoom,
                                every: every, duration: 1.2, face: false)]
    }

    /// A doorway: its picture from `minZoom`, a glint before that, and its life (a way it stirs along,
    /// a slow breath).
    private static func doorway(_ image: UIImage?, lon: Double, lat: Double, label: String, size: CGSize, minZoom: Double,
                                every: Double = 11, path: GlobeMarker.Path? = nil, breathes: Bool = false) -> Prototype.Doorway? {
        guard let image else { return nil }
        // (a picture that walks, drawn facing east, turns to face the way it goes)
        return Prototype.Doorway(lon: lon, lat: lat, label: label, looks: looks(image, size: size, minZoom: minZoom, every: every, face: path != nil),
                                 path: path, breathes: breathes)
    }

    static let native: [Prototype] = [camelid, whale, ocean, QuietGlobe.names, QuietGlobe.clouds, QuietGlobe.night, QuietGlobe.frame,
                                      persian, wikipedia, stillSea, film]

    /// The dromedary in Arabia opens the illustrated lineage (`Camelid/`), on the globe itself. Its
    /// doorway stirs: the camel walks a little way over the Nafud's sands and back, breathing, facing
    /// the way it goes (the picture faces east); from afar, a glint.
    static let camelid = Prototype(
        id: "camelid", title: "Camel and Llama",
        summary: "An illustrated lineage of the camels, from North America to Arabia and the Andes. Tap the dromedary.",
        anchor: GlobeCamera(lon: 46, lat: 24, zoom: 1.5),
        doorway: doorway(CamelidArt.image(.dromedary), lon: 46, lat: 24, label: "Dromedary camel",
                         size: CGSize(width: 40, height: 34), minZoom: 1.35, every: 11,
                         path: GlobeMarker.Path(points: [(lon: 45.7, lat: 23.9), (lon: 46.3, lat: 24.1)], seconds: 140, loop: .back),
                         breathes: true)
    ) { stage in
        guard let image = CamelidArt.image(.dromedary) else { return }
        stage.place(GlobeMarker(id: "camelidAnimal", lon: 46, lat: 24, look: .picture(image), size: CGSize(width: 76, height: 64),
                                label: "Dromedary camel", hint: "Inspect this animal", breathes: true) { [weak stage] in
            stage?.openLens { CamelidExperience.lens(on: $0, close: $1) }
        })
    }

    /// A fossil at Wadi Al-Hitan opens the visit to its ancient sea (`Whale/`), on the globe itself. The
    /// point is UNESCO's property reference, locating the fossil-bearing region, not an excavation; it
    /// shows from 2×.
    static let whale = Prototype(
        id: "whale", title: "Whale and Desert",
        summary: "A whale fossil in Egypt's Western Desert opens a visit to the shallow sea it swam in. Tap the fossil.",
        anchor: GlobeCamera(lon: 30.18333, lat: 29.33333, zoom: 3.2),
        doorway: doorway(WhaleArt.image(.fossil), lon: 30.18333, lat: 29.33333, label: "Whale fossil, Wadi Al-Hitan",
                         size: CGSize(width: 34, height: 27), minZoom: 2, every: 13)
    ) { stage in
        guard let image = WhaleArt.image(.fossil) else { return }
        stage.place(GlobeMarker(id: "whaleFossil", lon: 30.18333, lat: 29.33333, look: .picture(image), size: CGSize(width: 56, height: 44),
                                label: "Whale fossil, Wadi Al-Hitan", hint: "Inspect this illustrated fossil", minZoom: 2) { [weak stage] in
            stage?.openLens { WhaleExperience.lens(on: $0, close: $1) }
        })
    }

    /// The Indian Ocean encounter (`Experience/`): its seed off Madagascar, and its invitation.
    static let ocean = Prototype(
        id: "ocean", title: "A small thing, far from home",
        summary: "The Indian Ocean encounter: a seed off Madagascar opens crops, words and lives that crossed the sea.",
        anchor: GlobeCamera(lon: 68, lat: -9, zoom: 1),
        doorway: doorway(UIImage(systemName: "leaf.circle")?.withTintColor(UIColor(red: 0.231, green: 0.173, blue: 0.086, alpha: 1),
                                                                         renderingMode: .alwaysOriginal),
                         lon: 46.3, lat: -19.0, label: "A small thing, far from home", size: CGSize(width: 26, height: 26), minZoom: 1.35, every: 9)
    ) { stage in
        let open: () -> Void = { [weak stage] in stage?.present("ocean") { OceanExperience() } }
        stage.place(GlobeMarker(id: "oceanSeed", lon: 46.3, lat: -19.0, look: .seal(symbol: "leaf"), size: CGSize(width: 44, height: 44),
                                label: "A small thing, far from home", actionName: "Look closer at the island's seed", onOpen: open))
        stage.invite(Invitation(id: "oceanInvitation", title: "A small thing, far from home", action: "Look closer  ↗"), action: open)
    }

    /// The globe's second style, parked: the ground is the base (`Globe.metal`'s P_).
    static let persian = Prototype(
        id: "persian", title: "The Persian globe",
        summary: "The globe in colours and a frame studied from a 1529 Persian manuscript.",
        shelf: .parked
    ) { stage in
        stage.style = .persian
    }

    /// The globe's first base style, parked when the ground became the base (October 28, 2026):
    /// Wikipedia's location-map colours, the political map.
    static let wikipedia = Prototype(
        id: "wikipedia", title: "The Wikipedia globe",
        summary: "The political map in Wikipedia's location-map colours, as the globe was until the ground.",
        shelf: .parked
    ) { stage in
        stage.style = .wikipedia
    }

    /// The sea as it was before it moved, parked when the ocean joined the stage (October 28, 2026):
    /// no currents, light, glitter, surf or rings.
    static let stillSea = Prototype(
        id: "still-sea", title: "The still sea",
        summary: "The globe with its sea still, as it was before the currents, the light on the water and the surf.",
        shelf: .parked
    ) { stage in
        stage.stillSea()
    }

    /// The first atlas, parked: the plate films and their five looks (`Atlas/`), made only when opened.
    static let film = Prototype(
        id: "film", title: "The film",
        summary: "A flight from the whole world down to a street, in five looks. Touch to wander.",
        shelf: .parked, ownScreen: true
    ) { stage in
        stage.present("film") { FilmScreen() }
    }
}

/// A glint: a small warm light with four fine rays, drawn once, on nothing (the same recipe as
/// tools/playground/glint_png.py draws for the packs). A doorway's look from afar (shown now and then,
/// fading up and down), the same mark for every experience: it means something is here, and the
/// picture itself comes as the reader does.
enum Glint {
    static let image: UIImage = {
        let side: CGFloat = 44
        // the atlas's gold (the ring's B08A3E), deep enough to read on the pale land, and its lifted glow
        let gold = UIColor(red: 176 / 255, green: 138 / 255, blue: 62 / 255, alpha: 1)
        let light = UIColor(red: 232 / 255, green: 208 / 255, blue: 140 / 255, alpha: 1)
        return UIGraphicsImageRenderer(size: CGSize(width: side, height: side)).image { context in
            let c = context.cgContext
            let mid = CGPoint(x: side / 2, y: side / 2)
            let colors = [gold.cgColor, light.withAlphaComponent(0.55).cgColor, light.withAlphaComponent(0).cgColor] as CFArray
            if let shade = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(), colors: colors, locations: [0, 0.35, 1]) {
                c.drawRadialGradient(shade, startCenter: mid, startRadius: 0, endCenter: mid, endRadius: side * 0.34, options: [])
            }
            c.setLineCap(.round)
            let rays: [(dx: CGFloat, dy: CGFloat, reach: CGFloat, width: CGFloat)] = [
                (1, 0, 0.46, 1.8), (0, 1, 0.46, 1.8), (0.7071, 0.7071, 0.22, 1.2), (-0.7071, 0.7071, 0.22, 1.2),
            ]
            for ray in rays {
                c.setStrokeColor(UIColor.white.withAlphaComponent(0.92).cgColor)
                c.setLineWidth(ray.width)
                c.move(to: CGPoint(x: mid.x - ray.dx * side * ray.reach, y: mid.y - ray.dy * side * ray.reach))
                c.addLine(to: CGPoint(x: mid.x + ray.dx * side * ray.reach, y: mid.y + ray.dy * side * ray.reach))
                c.strokePath()
            }
        }
    }()
}
