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

    static let native: [Prototype] = [QuietGlobe.names, QuietGlobe.clouds, QuietGlobe.night, QuietGlobe.frame]
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
