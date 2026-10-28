import ImageIO
import UIKit

/// Pictures a prototype lays on the globe (terra.js runtime 3, `terra.images`): each a picture in
/// longitude and latitude (every column the same span of longitude, every row the same span of
/// latitude), drawn curving with the globe, in step with its camera. A picture is cut into a grid of
/// tiles that share its one decoded image, and each tile is mapped onto the globe by the projected
/// places of its corners, so it costs a few dozen layers moved per frame. It lies above the globe's
/// drawing and below its geography, sprites and markers, takes no touches, and goes with its stage.
final class GlobeImageLayer: UIView {
    struct Picture {
        let id: String
        let image: CGImage
        let west, south, east, north: Double
        let opacity: Double
        let minZoom: Double
        let maxZoom: Double
        /// The zoom range over which it fades in above `minZoom`.
        let fade: Double

        /// Reads the picture, at most `longest` pixels a side (larger ones are scaled down as they are read).
        init?(id: String, file: URL, west: Double, south: Double, east: Double, north: Double,
              opacity: Double, minZoom: Double, maxZoom: Double, fade: Double, longest: Int = GlobeImageLayer.longestSide) {
            guard let source = CGImageSourceCreateWithURL(file as CFURL, nil) else { return nil }
            let options: [CFString: Any] = [kCGImageSourceCreateThumbnailFromImageAlways: true,
                                            kCGImageSourceCreateThumbnailWithTransform: true,
                                            kCGImageSourceShouldCacheImmediately: true,
                                            kCGImageSourceThumbnailMaxPixelSize: longest]
            guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary) else { return nil }
            self.id = id
            self.image = image
            self.west = west; self.south = south; self.east = east; self.north = north
            self.opacity = opacity
            self.minZoom = minZoom
            self.maxZoom = maxZoom
            self.fade = fade
        }
    }

    static let longestSide = 4096
    static let pictureBudget = 8

    private struct Placed {
        let picture: Picture
        let host: CALayer
        let tiles: [CALayer]
        let columns: Int
        let rows: Int
        /// Each tile corner's place on the globe, (columns + 1) × (rows + 1), row by row from the north-west.
        let corners: [SIMD3<Double>]
    }

    private var placed: [Placed] = []
    private var version = 0
    private var drawn: (version: Int, camera: GlobeCamera, centre: CGPoint, radius: Double, size: CGSize)?

    override init(frame: CGRect) {
        super.init(frame: frame)
        isOpaque = false
        backgroundColor = .clear
        isUserInteractionEnabled = false
        isAccessibilityElement = false
    }

    required init?(coder: NSCoder) { fatalError("GlobeImageLayer is made in code") }

    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool { false }

    var isEmpty: Bool { placed.isEmpty }

    // MARK: what the script asks

    /// Adds or replaces a picture (false: over the budget, nothing changed).
    @discardableResult
    func set(_ picture: Picture) -> Bool {
        if let i = placed.firstIndex(where: { $0.picture.id == picture.id }) {
            placed[i].host.removeFromSuperlayer()
            placed[i] = make(picture, order: i)
        } else {
            guard placed.count < Self.pictureBudget else { return false }
            placed.append(make(picture, order: placed.count))
        }
        version += 1
        return true
    }

    func remove(_ id: String) {
        placed.filter { $0.picture.id == id }.forEach { $0.host.removeFromSuperlayer() }
        placed.removeAll { $0.picture.id == id }
        version += 1
    }

    func clear() {
        placed.forEach { $0.host.removeFromSuperlayer() }
        placed = []
        version += 1
    }

    /// A tile at most about two degrees across, so its straight edges stay on the curving globe.
    private func make(_ picture: Picture, order: Int) -> Placed {
        let host = CALayer()
        host.zPosition = CGFloat(order)
        layer.addSublayer(host)
        let span = (lon: picture.east - picture.west, lat: picture.north - picture.south)
        let columns = min(48, max(2, Int((span.lon / 2).rounded(.up))))
        let rows = min(48, max(2, Int((span.lat / 2).rounded(.up))))
        var corners: [SIMD3<Double>] = []
        for r in 0...rows {
            let lat = picture.north - span.lat * Double(r) / Double(rows)
            for c in 0...columns {
                corners.append(GeoLayerView.Projection.world(lon: picture.west + span.lon * Double(c) / Double(columns), lat: lat))
            }
        }
        var tiles: [CALayer] = []
        let w = CGFloat(picture.image.width) / CGFloat(columns), h = CGFloat(picture.image.height) / CGFloat(rows)
        for r in 0..<rows {
            for c in 0..<columns {
                let tile = CALayer()
                tile.contents = picture.image
                tile.contentsRect = CGRect(x: CGFloat(c) / CGFloat(columns), y: CGFloat(r) / CGFloat(rows),
                                           width: 1 / CGFloat(columns), height: 1 / CGFloat(rows))
                tile.anchorPoint = .zero
                // (a hair larger than its share, so neighbours overlap instead of showing a seam)
                tile.bounds = CGRect(x: 0, y: 0, width: w + 0.6, height: h + 0.6)
                tile.minificationFilter = .trilinear
                tile.edgeAntialiasingMask = []
                tile.isHidden = true
                host.addSublayer(tile)
                tiles.append(tile)
            }
        }
        return Placed(picture: picture, host: host, tiles: tiles, columns: columns, rows: rows, corners: corners)
    }

    // MARK: each frame

    /// Lays the pictures where the globe's camera now puts them, when either has changed.
    func update(globe: GlobeView) {
        let stage = globe.stageCamera
        let size = bounds.size
        if let d = drawn, d.version == version, d.camera == stage.camera, d.centre == stage.centre, d.radius == stage.radius,
           d.size == size { return }
        drawn = (version, stage.camera, stage.centre, stage.radius, size)
        let projection = GeoLayerView.Projection(camera: stage.camera, centre: stage.centre, radius: stage.radius)
        let zoom = stage.camera.zoom
        let view = bounds.insetBy(dx: -40, dy: -40)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for p in placed {
            let pic = p.picture
            let shown = zoom >= pic.minZoom && zoom <= pic.maxZoom
            let rise = pic.fade > 0 ? min(1, max(0, (zoom - pic.minZoom) / pic.fade)) : 1
            p.host.isHidden = !shown || rise <= 0
            p.host.opacity = Float(pic.opacity * rise)
            guard !p.host.isHidden else { continue }
            p.host.frame = bounds
            let points = p.corners.map { projection.point($0, facing: 0.02) }
            let stride = p.columns + 1
            for r in 0..<p.rows {
                for c in 0..<p.columns {
                    let tile = p.tiles[r * p.columns + c]
                    guard let nw = points[r * stride + c], let ne = points[r * stride + c + 1],
                          let sw = points[(r + 1) * stride + c], let se = points[(r + 1) * stride + c + 1],
                          view.intersects(CGRect(x: min(nw.x, sw.x), y: min(nw.y, ne.y),
                                                 width: max(ne.x, se.x) - min(nw.x, sw.x), height: max(sw.y, se.y) - min(nw.y, ne.y)))
                    else {
                        tile.isHidden = true
                        continue
                    }
                    let w = tile.bounds.width - 0.6, h = tile.bounds.height - 0.6
                    // the tile's north-west, north-east and south-west corners fix its affine map; the
                    // fourth lands within a hair of its place at this size
                    tile.position = nw
                    tile.setAffineTransform(CGAffineTransform(a: (ne.x - nw.x) / w, b: (ne.y - nw.y) / w,
                                                              c: (sw.x - nw.x) / h, d: (sw.y - nw.y) / h, tx: 0, ty: 0))
                    tile.isHidden = false
                }
            }
        }
        CATransaction.commit()
    }
}
