import CoreGraphics
import CoreText
import Foundation

/// A look the app can show, as listed in `Atlas/looks.json` (tools/plates/export.cjs writes it).
nonisolated struct Look: Identifiable, Decodable, Sendable, Equatable {
    let id: String
    let name: String
    /// The look's plates.json, relative to `Atlas/`.
    let plates: String
}

/// One look's scales as pictures, widest first. Each plate after the first sits inside the one before
/// it, at that plate's `child` rectangle, so the camera can dive from one into the next.
nonisolated struct PlateSet: Decodable, Sendable {
    let name: String
    let plates: [Plate]
    let style: CaptionStyle?
}

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

nonisolated struct Plate: Decodable, Sendable {
    nonisolated struct Layer: Decodable, Sendable {
        /// Relative to the look's folder.
        let file: String
        /// How far the layer moves with the camera beyond the plate itself, as a fraction of the
        /// camera's offset from the plate's centre: 0 is the ground, more is nearer the eye.
        let depth: Double
        /// A sprite: the part of the plate the picture covers (x, y, width, height); nil for all of it.
        let frame: [Double]?
        let motion: Motion?

        var rect: CGRect? {
            guard let f = frame, f.count == 4, f[2] > 0, f[3] > 0 else { return nil }
            return CGRect(x: f[0], y: f[1], width: f[2], height: f[3])
        }
    }

    /// Gentle life, in plate pixels and seconds (tools/plates/kit.js describes each kind).
    nonisolated struct Motion: Decodable, Sendable {
        /// bob, sway, breathe or sheen; anything else is left still (a newer exporter's kind)
        let kind: String
        let period: Double?
        let phase: Double?
        let amp: [Double]?
        let angle: Double?
        let anchor: [Double]?
        let min: Double?
        let max: Double?
        let width: Double?
        let color: String?
        let opacity: Double?

        /// sin of the motion's phase at time t
        func wave(_ t: Double) -> Double {
            sin(2 * .pi * (t / Swift.max(period ?? 6, 0.5) + (phase ?? 0)))
        }
    }

    nonisolated struct Caption: Decodable, Sendable, Equatable {
        let eyebrow: String
        let title: String
        let text: String
    }

    let id: String
    let size: [Double]
    let layers: [Layer]
    /// The next plate's footprint in this plate's pixels (x, y, width, height); nil for the last.
    let child: [Double]?
    let caption: Caption
    /// A plate the film dives through without stopping: a wider scale drawn again magnified, so no
    /// picture is blown up far before the next takes over (tools/plates/export.cjs adds them).
    let pass: Bool?

    var passes: Bool { pass ?? false }

    var width: Double { size[0] }
    var height: Double { size[1] }
}

/// The bundled `Atlas` folder.
nonisolated enum Atlas {
    static let root = Bundle.main.bundleURL.appending(path: "Atlas", directoryHint: .isDirectory)

    static func looks() -> [Look] {
        guard let data = try? Data(contentsOf: root.appending(path: "looks.json")),
              let looks = try? JSONDecoder().decode([Look].self, from: data) else { return [] }
        return looks
    }

    static func plates(of look: Look) -> PlateSet? {
        guard let data = try? Data(contentsOf: root.appending(path: look.plates)),
              let set = try? JSONDecoder().decode(PlateSet.self, from: data),
              valid(set) else { return nil }
        return set
    }

    /// Makes a look's caption fonts available to the app (once: a font registered again is refused,
    /// which is harmless).
    static func registerFonts(of look: Look, _ style: CaptionStyle) {
        let folder = root.appending(path: look.plates).deletingLastPathComponent()
        for file in style.fonts ?? [] {
            _ = CTFontManagerRegisterFontsForURL(folder.appending(path: file) as CFURL, .process, nil)
        }
    }

    /// The file of a layer, inside the look's folder.
    static func url(of layer: Plate.Layer, in look: Look) -> URL {
        root.appending(path: look.plates).deletingLastPathComponent().appending(path: layer.file)
    }

    /// Every plate but the last has a child with a positive size, and all have a size and layers:
    /// the camera divides by these.
    private static func valid(_ set: PlateSet) -> Bool {
        guard !set.plates.isEmpty else { return false }
        for (i, p) in set.plates.enumerated() {
            guard p.size.count == 2, p.width > 0, p.height > 0, !p.layers.isEmpty else { return false }
            if i < set.plates.count - 1 {
                guard let c = p.child, c.count == 4, c[2] > 0, c[3] > 0 else { return false }
            }
        }
        return true
    }
}
