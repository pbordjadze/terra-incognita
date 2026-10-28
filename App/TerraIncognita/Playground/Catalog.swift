import Foundation
import SwiftUI
import UIKit

/// The native prototypes, one entry each, in the workbench's order. A new one: its code in its own
/// folder under App/TerraIncognita/, and one entry here (docs/playground.md). Packs need no entry:
/// `PackStore` finds their folders and sources. A doorway puts it on the globe while it isn't running.
enum Catalog {
    /// One still picture as a doorway's look, shown from `minZoom` (the whole-Earth view stays quiet).
    private static func doorway(_ image: UIImage?, lon: Double, lat: Double, label: String, size: CGSize, minZoom: Double) -> Prototype.Doorway? {
        guard let image else { return nil }
        return Prototype.Doorway(lon: lon, lat: lat, label: label, looks: [GlobeMarker.SpriteLook(
            frames: [image], fps: 1, size: size, minZoom: minZoom, maxZoom: .infinity, every: nil, duration: nil, face: false)])
    }

    static let native: [Prototype] = [camelid, whale, ocean, water, leaf, persian, film]

    /// The dromedary in Arabia opens the illustrated lineage (`Camelid/`).
    static let camelid = Prototype(
        id: "camelid", title: "Camel and Llama",
        summary: "An illustrated lineage of the camels, from North America to Arabia and the Andes. Tap the dromedary.",
        anchor: GlobeCamera(lon: 46, lat: 24, zoom: 1.5),
        doorway: doorway(CamelidArt.image(.dromedary), lon: 46, lat: 24, label: "Dromedary camel",
                         size: CGSize(width: 40, height: 34), minZoom: 1.35)
    ) { stage in
        guard let image = CamelidArt.image(.dromedary) else { return }
        stage.place(GlobeMarker(id: "camelidAnimal", lon: 46, lat: 24, look: .picture(image), size: CGSize(width: 76, height: 64),
                                label: "Dromedary camel", hint: "Inspect this animal", breathes: true) { [weak stage] in
            stage?.present("camelid") { CamelidExperience(style: .wikipedia) }
        })
    }

    /// A fossil at Wadi Al-Hitan opens the visit to its ancient sea (`Whale/`). The point is UNESCO's
    /// property reference, locating the fossil-bearing region, not an excavation; it shows from 2×.
    static let whale = Prototype(
        id: "whale", title: "Whale and Desert",
        summary: "A whale fossil in Egypt's Western Desert opens a visit to the shallow sea it swam in. Tap the fossil.",
        anchor: GlobeCamera(lon: 30.18333, lat: 29.33333, zoom: 3.2),
        doorway: doorway(WhaleArt.image(.fossil), lon: 30.18333, lat: 29.33333, label: "Whale fossil, Wadi Al-Hitan",
                         size: CGSize(width: 34, height: 27), minZoom: 2)
    ) { stage in
        guard let image = WhaleArt.image(.fossil) else { return }
        stage.place(GlobeMarker(id: "whaleFossil", lon: 30.18333, lat: 29.33333, look: .picture(image), size: CGSize(width: 56, height: 44),
                                label: "Whale fossil, Wadi Al-Hitan", hint: "Inspect this illustrated fossil", minZoom: 2) { [weak stage] in
            stage?.present("whale") { WhaleExperience(style: .wikipedia) }
        })
    }

    /// The Indian Ocean encounter (`Experience/`): its seed off Madagascar, and its invitation.
    static let ocean = Prototype(
        id: "ocean", title: "A small thing, far from home",
        summary: "The Indian Ocean encounter: a seed off Madagascar opens crops, words and lives that crossed the sea.",
        anchor: GlobeCamera(lon: 68, lat: -9, zoom: 1),
        doorway: doorway(UIImage(systemName: "leaf.circle")?.withTintColor(UIColor(red: 0.231, green: 0.173, blue: 0.086, alpha: 1),
                                                                         renderingMode: .alwaysOriginal),
                         lon: 46.3, lat: -19.0, label: "A small thing, far from home", size: CGSize(width: 26, height: 26), minZoom: 1.35)
    ) { stage in
        let open: () -> Void = { [weak stage] in stage?.present("ocean") { OceanExperience() } }
        stage.place(GlobeMarker(id: "oceanSeed", lon: 46.3, lat: -19.0, look: .seal(symbol: "leaf"), size: CGSize(width: 44, height: 44),
                                label: "A small thing, far from home", actionName: "Look closer at the island's seed", onOpen: open))
        stage.invite(Invitation(id: "oceanInvitation", title: "A small thing, far from home", action: "Look closer  ↗"), action: open)
    }

    /// The river study (`Water/`, a bundled web page in its own host).
    static let water = Prototype(
        id: "water", title: "A gathering of water",
        summary: "A river study in South America: touch the network to follow where its water goes.",
        ownScreen: true
    ) { stage in
        stage.present("water") { WaterStudyScreen() }
    }

    /// The Aegean leaf (`Leaf/`, a bundled web page in its own host).
    static let leaf = Prototype(
        id: "leaf", title: "The sea between",
        summary: "An engraved leaf of the Aegean to look at and touch, with optional notes on its places.",
        ownScreen: true
    ) { stage in
        stage.present("leaf") { LeafScreen() }
    }

    /// The globe's second style, parked: the Wikipedia style is the base for now (`Globe.metal`'s P_).
    static let persian = Prototype(
        id: "persian", title: "The Persian globe",
        summary: "The globe in colours and a frame studied from a 1529 Persian manuscript.",
        shelf: .parked
    ) { stage in
        stage.style = .persian
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
