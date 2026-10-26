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
}

nonisolated struct Plate: Decodable, Sendable {
    nonisolated struct Layer: Decodable, Sendable {
        /// Relative to the look's folder.
        let file: String
        /// How far the layer moves with the camera beyond the plate itself, as a fraction of the
        /// camera's offset from the plate's centre: 0 is the ground, more is nearer the eye.
        let depth: Double
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
