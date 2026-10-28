import CoreGraphics
import CoreText
import Foundation
import ImageIO

/// The globe's bundled data (`Globe/`, built by tools/globe/build.py): the countries, seas and
/// capitals with where their names go, and which country is under each point of the map.
nonisolated struct GlobeData: Sendable {
    nonisolated struct Country: Decodable, Sendable {
        /// The name, and the long name (the caption's title).
        let n: String
        let long: String?
        let cont: String?
        let sub: String?
        let pop: Double?
        let cap: String?
        /// Where the name goes: longitude, latitude.
        let c: [Double]
        /// Natural Earth's label rank, and the web-map zoom from which the name shows.
        let r: Double
        let min: Double
    }

    nonisolated struct Sea: Decodable, Sendable {
        let n: String
        let r: Double
        let ocean: Bool
        let c: [Double]
    }

    nonisolated struct Capital: Decodable, Sendable {
        let n: String
        let c: [Double]
        let r: Double?
    }

    /// One of the land's own names for the ground style (Globe/regions.json, tools/globe/ground.py): a
    /// range, desert, plateau, basin or plain, its class, Natural Earth's rank, where the name goes.
    nonisolated struct Region: Decodable, Sendable {
        let n: String
        let k: String
        let r: Double
        let c: [Double]
    }

    private nonisolated struct Regions: Decodable {
        let regions: [Region]
    }

    private nonisolated struct File: Decodable {
        let countries: [Country]
        let seas: [Sea]
        let capitals: [Capital]
    }

    let countries: [Country]
    let seas: [Sea]
    let capitals: [Capital]
    /// The land's own names (none if regions.json is missing: the ground then names only seas and countries).
    var regions: [Region] = []
    /// 1 + the index of the country under each pixel of the map (0: none), row by row: the tap's
    /// lookup, and the shader's texture of it.
    let ids: [UInt8]
    let width: Int
    let height: Int

    static let root = Bundle.main.bundleURL.appending(path: "Globe", directoryHint: .isDirectory)

    static func load() -> GlobeData? {
        guard let json = try? Data(contentsOf: root.appending(path: "globe.json")),
              let file = try? JSONDecoder().decode(File.self, from: json),
              let map = grey(root.appending(path: "ids.png")) else { return nil }
        var data = GlobeData(countries: file.countries, seas: file.seas, capitals: file.capitals, ids: map.bytes, width: map.width, height: map.height)
        if let regions = try? Data(contentsOf: root.appending(path: "regions.json")),
           let file = try? JSONDecoder().decode(Regions.self, from: regions) {
            data.regions = file.regions
        }
        return data
    }

    /// The index of the country at a point of the globe, if any.
    func country(lon: Double, lat: Double) -> Int? {
        guard lon.isFinite, lat.isFinite else { return nil }
        let x = (Int(((lon + 180) / 360 * Double(width)).rounded(.down)) % width + width) % width
        let y = min(max(Int(((90 - lat) / 180 * Double(height)).rounded(.down)), 0), height - 1)
        let v = Int(ids[y * width + x])
        return v > 0 && v <= countries.count ? v - 1 : nil
    }

    /// The image's decoded bytes as they are (no colour management, which would change the values).
    static func grey(_ url: URL) -> (bytes: [UInt8], width: Int, height: Int)? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil),
              image.bitsPerPixel == 8, let data = image.dataProvider?.data,
              let bytes = CFDataGetBytePtr(data) else { return nil }
        let w = image.width, h = image.height, row = image.bytesPerRow
        guard CFDataGetLength(data) >= row * h else { return nil }
        var out = [UInt8](repeating: 0, count: w * h)
        out.withUnsafeMutableBufferPointer { dst in
            guard let base = dst.baseAddress else { return }
            for y in 0..<h { (base + y * w).update(from: bytes + y * row, count: w) }
        }
        return (out, w, h)
    }

    /// Makes the globe's fonts available; the first call registers them, later ones do nothing.
    static func registerFonts() { _ = fontsRegistered }

    /// (every font in fonts/, so that any face type.json names, which ci/check_globe.py finds there, is usable)
    private static let fontsRegistered: Bool = {
        let folder = root.appending(path: "fonts", directoryHint: .isDirectory)
        let files = (try? FileManager.default.contentsOfDirectory(at: folder, includingPropertiesForKeys: nil)) ?? []
        for file in files where ["otf", "ttf"].contains(file.pathExtension.lowercased()) {
            _ = CTFontManagerRegisterFontsForURL(file as CFURL, .process, nil)
        }
        return true
    }()
}

/// The globe's type and its scale (Globe/type.json, CLAUDE.md › Design rules): the names' faces in
/// each style, the captions', and how much larger everything is drawn on a larger screen.
nonisolated struct GlobeType: Decodable, Sendable {
    nonisolated struct Face: Decodable, Sendable {
        let font: String
        /// Points at reading scale 1.
        let size: Double
        /// A fraction of the size.
        let tracking: Double
        let ink: String
        /// The tone round the letters: the ground's (a name), or the name's panel or silhouette (the
        /// chosen country's).
        let halo: String
    }

    nonisolated struct Cartouche: Decodable, Sendable {
        let font: String
        let size: Double
        let ink: String
        let ground: String
        /// Where the title may go, as fractions of the headpiece: x, y, width, height.
        let box: [Double]
    }

    nonisolated struct Style: Decodable, Sendable {
        let names: [String: Face]
        let caption: CaptionStyle
        let captionGround: String
        let cartouche: Cartouche?
    }

    nonisolated struct Reading: Decodable, Sendable {
        /// Two screens' shorter sides, in points, and the scale each is drawn at.
        let shortSide: [Double]
        let factor: [Double]
    }

    let reading: Reading
    /// The most the reader's text size enlarges the names.
    let readerMax: Double
    /// Points round a name's letters in its halo, and between two names as a fraction of the size.
    let halo: Double
    let gap: Double
    /// How near, in points, a tap on the sea may be to a country to choose it.
    let touch: Double
    /// The label ranks up to which a country's name is set in the first and second sizes.
    let countryTiers: [Double]
    let styles: [String: Style]

    static let shared: GlobeType? = {
        guard let json = try? Data(contentsOf: GlobeData.root.appending(path: "type.json")) else { return nil }
        return try? JSONDecoder().decode(GlobeType.self, from: json)
    }()

    /// The scale things are drawn at on a screen whose shorter side is `side` points: a larger
    /// screen is held further away, so its names and lines are drawn larger.
    func readingScale(shortSide side: Double) -> Double {
        guard reading.shortSide.count == 2, reading.factor.count == 2, reading.shortSide[1] > reading.shortSide[0] else { return 1 }
        let t = min(max((side - reading.shortSide[0]) / (reading.shortSide[1] - reading.shortSide[0]), 0), 1)
        return reading.factor[0] + (reading.factor[1] - reading.factor[0]) * t
    }

    /// The key of a country's face, by its label rank: the more prominent, the larger.
    func countryFace(rank: Double) -> String {
        if let first = countryTiers.first, rank <= first { return "country1" }
        if countryTiers.count > 1, rank <= countryTiers[1] { return "country2" }
        return "country3"
    }
}
