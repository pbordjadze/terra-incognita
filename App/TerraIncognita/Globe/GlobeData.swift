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

    private nonisolated struct File: Decodable {
        let countries: [Country]
        let seas: [Sea]
        let capitals: [Capital]
    }

    let countries: [Country]
    let seas: [Sea]
    let capitals: [Capital]
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
        return GlobeData(countries: file.countries, seas: file.seas, capitals: file.capitals, ids: map.bytes, width: map.width, height: map.height)
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

    /// Makes the globe's fonts available (once; registering again is refused, harmlessly).
    static func registerFonts() {
        let folder = root.appending(path: "fonts", directoryHint: .isDirectory)
        for file in ["LibertinusSerif-Regular.otf", "LibertinusSerif-Italic.otf", "LibertinusSerif-Semibold.otf", "UncialAntiqua-Regular.ttf"] {
            _ = CTFontManagerRegisterFontsForURL(folder.appending(path: file) as CFURL, .process, nil)
        }
    }
}
