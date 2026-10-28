import CoreGraphics
import CoreText
import Foundation
import ImageIO
import Metal
import OSLog
import QuartzCore
import UIKit
import simd

/// The globe's styles: Wikipedia's location-map colours, a little prettier; the Persian, the same map
/// in colours studied from a Persian manuscript, inside its frame (Globe.metal's P_ palette); and the
/// ground, the land as satellites saw it that month, in the atlas's light key (Globe/ground/).
enum GlobeStyle: String, CaseIterable, Identifiable {
    case persian, wikipedia, ground

    /// The plain globe's style: the ground since October 28, 2026 (`.wikipedia` undoes it; the
    /// Wikipedia style is then a parked prototype, as the Persian is).
    static let base: GlobeStyle = .ground

    var id: String { rawValue }
    var name: String {
        switch self {
        case .persian: String(localized: "Persian")
        case .wikipedia: String(localized: "Wikipedia")
        case .ground: String(localized: "The ground")
        }
    }
}

/// A camera position for the camelid sequence; geographic degrees and the globe's existing scale.
struct GlobeCamera: Equatable {
    var lon: Double
    var lat: Double
    var zoom: Double
}

/// The whole globe, drawn live by Metal (`Globe.metal`) in either style: drag to spin it, pinch to
/// zoom, double-tap to fly in, tap a country to light it (in the locator's red, or gilded). Left alone
/// it turns slowly, as the Earth does.
final class GlobeView: UIView, UIGestureRecognizerDelegate {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    /// A country was tapped (nil: the selection was cleared).
    var onSelect: ((GlobeData.Country?) -> Void)?
    /// The first frame is on screen (the veil over the switch to it can lift).
    var onDrawn: (() -> Void)?
    /// What the active prototype has placed on the globe (`add`, `remove`): nothing on the plain globe.
    private(set) var markers: [GlobeMarker] = []
    private let live = LiveCount.Token(.globes)
    /// Once the atlas contains accessible objects it becomes a container. Keep the globe itself
    /// as a separate element so VoiceOver can still describe and directly manipulate the map.
    private var globeAccessibilityElement: UIAccessibilityElement?
    /// Explicit reader navigation yields the authored sequence before the camera moves.
    var onUserNavigation: (() -> Void)?
    /// Geographic overlays follow the same camera after it has advanced for this frame.
    var onFrame: (() -> Void)?
    /// The stage's sky (Clouds/): over the globe's drawing and under everything else on it, drawn in
    /// step with the camera each frame. Set once, by the screen that makes the opening globe.
    var sky: CloudsLayer? {
        didSet {
            oldValue?.stop()
            oldValue?.layer.removeFromSuperlayer()
            if let sky { layer.insertSublayer(sky.layer, at: 0) }
        }
    }
    /// A tap on the globe, offered first to the active prototype (a web overlay that asked for taps):
    /// true when it took the tap, and no country is chosen. `place` is nil off the globe.
    var onWorldTap: ((_ point: CGPoint, _ place: (lon: Double, lat: Double)?) -> Bool)?
    /// How one-finger drags are shared between the globe and the active pack (terra.js runtime 3):
    /// `touchables`, a drag that begins on something touchable is the pack's (`worldDragBegins` says);
    /// `oneFinger`, every one-finger drag is the pack's and two fingers turn and zoom the globe.
    enum DragScheme { case touchables, oneFinger }
    var dragScheme = DragScheme.touchables {
        didSet { panRecognizer?.minimumNumberOfTouches = dragScheme == .oneFinger ? 2 : 1 }
    }
    /// Whether a one-finger drag starting at this point is the pack's (the `touchables` scheme).
    var worldDragBegins: ((CGPoint) -> Bool)?
    /// A drag that is the pack's: "began", "moved", "ended", with the place under the finger.
    var onWorldDrag: ((_ phase: String, _ point: CGPoint, _ place: (lon: Double, lat: Double)?) -> Void)?
    private weak var panRecognizer: UIPanGestureRecognizer?
    private weak var worldDragRecognizer: UIPanGestureRecognizer?
    /// Where the touch that may become a world drag began.
    private var worldDragStart: CGPoint?
    /// The authored sequence shares this renderer without its idle spin or other encounter marks.
    var authoredMode = false {
        didSet {
            guard authoredMode != oldValue else { return }
            if authoredMode { flight = nil; spin = .zero }
            for m in markers { m.view.isHidden = authoredMode }
            needsDraw = true
        }
    }
    /// A prototype asked the globe to stay where the reader leaves it: no idle turn.
    var holdsStill = false
    /// Modern coastlines remain an explicitly labelled geographic reference. Contemporary political
    /// lines, names and country selection are suppressed while illustrating prehistoric periods.
    var historicalReference = false {
        didSet {
            guard historicalReference != oldValue else { return }
            CATransaction.begin()
            CATransaction.setDisableActions(true)
            labelLayer.isHidden = historicalReference
            CATransaction.commit()
            if historicalReference { select(nil, animated: false) }
            labelsFading = true
            needsDraw = true
        }
    }

    var authoredCamera: GlobeCamera { GlobeCamera(lon: lon0, lat: lat0, zoom: zoom) }

    /// The camera and where the globe's disc lies in this view, for anything drawn in step with it
    /// (terra.js projects with the same numbers).
    var stageCamera: (camera: GlobeCamera, centre: CGPoint, radius: Double) {
        let (centre, radius) = place
        return (GlobeCamera(lon: lon0, lat: lat0, zoom: zoom), centre, radius)
    }

    func setAuthoredCamera(_ camera: GlobeCamera, duration: Double = 0) {
        guard camera.lon.isFinite, camera.lat.isFinite, camera.zoom.isFinite else { return }
        let longitude = (camera.lon + 180).truncatingRemainder(dividingBy: 360)
        let target = (lon: (longitude < 0 ? longitude + 360 : longitude) - 180,
                      lat: min(max(camera.lat, -80), 80),
                      zoom: min(max(camera.zoom, Self.zooms.lowerBound), zoomLimit))
        spin = .zero
        lastTouch = CACurrentMediaTime()
        if duration == 0, flight == nil, lon0 == target.lon, lat0 == target.lat, zoom == target.zoom {
            return
        }
        if duration > 0, duration.isFinite, !UIAccessibility.isReduceMotionEnabled {
            flight = Flight(from: (lon: lon0, lat: lat0, zoom: zoom), to: target,
                            start: CACurrentMediaTime(), duration: duration)
        } else {
            flight = nil
            lon0 = target.lon; lat0 = target.lat; zoom = target.zoom
        }
        needsDraw = true
    }

    func projectedLocation(lon: Double, lat: Double) -> (point: CGPoint, facing: Double) {
        project(lon: lon, lat: lat)
    }

    /// The place under a point of this view, or nil off the globe.
    func unprojectedPlace(_ point: CGPoint) -> (lon: Double, lat: Double)? { unproject(point) }

    /// Places a marker; it follows the geography from the next frame.
    func add(_ marker: GlobeMarker) {
        guard !markers.contains(where: { $0 === marker }) else { return }
        marker.view.isHidden = true
        addSubview(marker.view)
        markers.append(marker)
        exposeAtlasObjects()
        needsDraw = true
    }

    /// Takes a marker away (the prototype that placed it has ended).
    func remove(_ marker: GlobeMarker) {
        marker.view.removeFromSuperview()
        markers.removeAll { $0 === marker }
        exposeAtlasObjects()
        labelsFading = true
        needsDraw = true
    }

    /// Something added above the globe's drawing that follows it (a prototype's web overlay): below
    /// the markers, taking no touches.
    func addOverlay(_ overlay: UIView) {
        overlay.isUserInteractionEnabled = false
        overlay.frame = bounds
        overlay.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        insertSubview(overlay, at: 0)
    }

    /// Views above the globe whose own accessibility elements VoiceOver should reach (a pack's
    /// touchable geography); the stage adds and removes them.
    var accessibleOverlays: [UIView] = [] { didSet { exposeAtlasObjects() } }

    private func exposeAtlasObjects() {
        // the plain globe is one element; with markers, it becomes their container
        if markers.isEmpty && accessibleOverlays.isEmpty && globeAccessibilityElement == nil { return }
        if globeAccessibilityElement == nil {
            let element = UIAccessibilityElement(accessibilityContainer: self)
            element.accessibilityLabel = accessibilityLabel
            element.accessibilityHint = accessibilityHint
            element.accessibilityTraits = accessibilityTraits
            element.accessibilityValue = accessibilityValue
            element.accessibilityIdentifier = accessibilityIdentifier
            globeAccessibilityElement = element
        }
        guard let element = globeAccessibilityElement else { return }
        isAccessibilityElement = false
        element.accessibilityFrameInContainerSpace = bounds
        // each marker can be found from the map (it may be behind the globe or too small yet to show)
        element.accessibilityCustomActions = markers.flatMap { m -> [UIAccessibilityCustomAction] in
            var actions = [UIAccessibilityCustomAction(name: String(localized: "Go to \(m.label)")) { [weak self, weak m] _ in
                guard let self, let m, !self.authoredMode else { return false }
                self.touched()
                self.select(nil, animated: false)
                let duration = UIAccessibility.isReduceMotionEnabled ? 0.0 : 0.8
                self.setAuthoredCamera(GlobeCamera(lon: m.lon, lat: m.lat, zoom: max(self.zoom, m.minZoom + 0.2)), duration: duration)
                DispatchQueue.main.asyncAfter(deadline: .now() + duration + 0.1) { [weak m] in
                    guard let view = m?.view, !view.isHidden else { return }
                    UIAccessibility.post(notification: .layoutChanged, argument: view)
                }
                return true
            }]
            if let name = m.actionName, let open = m.onOpen {
                actions.append(UIAccessibilityCustomAction(name: name) { _ in open(); return true })
            }
            return actions
        }
        // Small objects precede the full-size map element, so touch exploration can reach them
        // where their frames overlap the map's direct-interaction surface.
        var objects: [Any] = markers.map(\.view)
        objects.append(contentsOf: accessibleOverlays as [Any])
        objects.append(element)
        accessibilityElements = objects
    }

    /// Name the map's direct-interaction accessibility element without hiding its small objects.
    func setAtlasAccessibilityIdentifier(_ identifier: String) {
        accessibilityIdentifier = identifier
        globeAccessibilityElement?.accessibilityIdentifier = identifier
    }

    /// The country in the locator's red, if any.
    var selectedCountry: GlobeData.Country? { selected.flatMap { data?.countries[$0] } }
    /// Parts of the screen covered by something else (the caption, the buttons), in this view's
    /// points: no name is placed under them.
    var keepClear: [CGRect] = [] {
        didSet {
            let fresh = clearBounds != bounds.size
            clearBounds = bounds.size
            guard keepClear != oldValue || fresh else { return }
            if !roomSettled { measureRoom() }
            labelsFading = true
            needsDraw = true
        }
    }
    /// Authored illustrations exclude labels without changing the globe's measured room or scale.
    /// These are local view coordinates, independent of the surrounding controls in `keepClear`.
    var authoredLabelExclusions: [CGRect] = [] {
        didSet {
            guard authoredLabelExclusions != oldValue else { return }
            labelsFading = true
        }
    }
    /// The part of the screen the globe and its frame fill: between the buttons and the caption.
    /// Measured when the screen's size, its safe area or the style changes, not when the caption's
    /// text does, so a tap never moves the globe.
    private var room: CGRect?
    private var roomMeasured = false
    /// When the room was first measured from rects laid out for this screen: for a moment after,
    /// the caption and buttons may still be settling, and each new place of theirs measures it again.
    private var roomSince = 0.0
    private var roomSettled: Bool { roomMeasured && CACurrentMediaTime() - roomSince > 0.6 }
    /// The size the view had when `keepClear` was last set: rects from another size are stale.
    private var clearBounds = CGSize.zero
    /// While the film shows instead, nothing moves or is drawn (the globe is kept, its pictures
    /// decoded once).
    var suspended = false {
        didSet {
            // (the film's buttons sit elsewhere, and a rotation meanwhile settled the room by them)
            if oldValue && !suspended { unsettleRoom() }
            displayLink?.isPaused = suspended
            lastTick = nil
            needsDraw = true
        }
    }

    private let data = GlobeData.load()
    private let device = MTLCreateSystemDefaultDevice()
    private var queue: MTLCommandQueue?
    private var pipeline: MTLRenderPipelineState?
    private var geo: MTLTexture?
    private var terrain: MTLTexture?
    private var ids: MTLTexture?
    private var band: MTLTexture?
    private var fleuron: MTLTexture?
    private var braid: MTLTexture?
    /// The Persian capitals' marker (Globe/rosette.png).
    private var rosette: CGImage?
    /// The style, and how far the drawing has crossed to it: 0 Wikipedia's, 1 the Persian; and from
    /// Wikipedia's to the ground's, 0 to 1.
    private(set) var style = GlobeStyle.persian
    private var styleMix = 1.0
    private var groundMix = 0.0
    /// The ground style's month (1 to 12): its land and its sea ice. This month, unless told
    /// (`-globe-month n` in debug builds); a new month's maps are read when it is next drawn.
    var month = Calendar.current.component(.month, from: Date()) {
        didSet {
            month = min(max(month, 1), 12)
            if month != oldValue { loadGround() }
        }
    }
    /// The ground style's maps, read the first time the style is chosen (Globe/ground/): the month's
    /// land, its sea ice, the sea floor's relief; and a small copy of the land for the names' halos.
    private var ground: MTLTexture?
    private var ice: MTLTexture?
    private var seaFloor: MTLTexture?
    private var groundMonth = 0
    private var groundSample: (bytes: [UInt8], width: Int, height: Int)?
    private var displayLink: CADisplayLink?
    /// The rate the display link was last asked for: 0 idle, 1 something living moves, 2 the hands.
    private var busyRate = -1
    private var lastTick: CFTimeInterval?
    private var needsDraw = true
    private var busy = true
    private var labelsFading = true
    private var cachedRotation: (lon: Double, lat: Double, m: simd_double3x3)?

    // the camera: the point of the globe at the screen's centre, in degrees, and how far in
    private var lon0 = -40.0
    private var lat0 = 22.0
    private var zoom = 1.0
    private static let zooms = 0.8...8.0
    /// The deepest the reader may zoom: 8, unless the active pack asks for more for its own close
    /// views (`terra.globe.set({ maxZoom })`, up to 32); the stage puts it back when it ends.
    var zoomLimit = GlobeView.zooms.upperBound {
        didSet {
            zoomLimit = min(max(zoomLimit, Self.zooms.upperBound), 32)
            if zoom > zoomLimit { zoom = zoomLimit; needsDraw = true }
        }
    }
    // a fling, in degrees a second; a double tap's flight
    private var spin = CGVector.zero
    private var touching = 0
    private var lastTouch: CFTimeInterval = -100
    private var flight: Flight?
    // the country in the locator's red, and how far it has faded up (it fades down still drawn)
    private var selected: Int?
    private var shown: Int?
    private var highlight = 0.0
    /// How far the chosen country's name has faded in where it is (times the highlight).
    private var selectedAlpha = 0.0
    private let labelLayer = CALayer()
    private var labels: [Label] = []
    /// The chosen country's name: in Wikipedia's style a thick red silhouette of it (the halo, for
    /// where it reaches past its country onto the sea), the cream letters on top as its sublayer; in
    /// the Persian a small text panel, ruled in gold and lapis (`selectedRule`), the letters ink.
    private let selectedLabel = CATextLayer()
    private let selectedLetters = CATextLayer()
    private let selectedRule = CALayer()
    /// The screen's pixels per point, for the names' pictures.
    private var pixels: CGFloat = 3
    #if DEBUG
    /// `-globe-at lon lat zoom` (CI's screenshots): the camera held there, no turning.
    private var held = false
    #endif

    private struct Flight {
        let from: (lon: Double, lat: Double, zoom: Double)
        let to: (lon: Double, lat: Double, zoom: Double)
        let start: CFTimeInterval
        let duration: Double
    }

    private enum Kind { case country, sea, region, capital }

    /// A name on the globe: where it goes, from what zoom, its picture once first shown.
    private struct Label {
        let text: NSAttributedString
        let size: CGSize
        let lon: Double
        let lat: Double
        let kind: Kind
        let index: Int
        /// The web-map zoom from which it shows, and (the ground) the globe's zoom, whatever the screen.
        let from: Double
        var minZoom = 0.0
        /// A Persian capital's rosette, left of its name and on the city itself, and its size.
        let mark: Bool
        let markSize: CGFloat
        /// Its halo's tone, and the room it keeps from other names (points).
        let halo: String
        let gap: CGFloat
        var layer: CALayer?
        var alpha = 0.0
    }

    override init(frame: CGRect) {
        super.init(frame: frame)
        GlobeData.registerFonts()
        backgroundColor = UIColor(red: 0xF6 / 255, green: 0xF2 / 255, blue: 0xE7 / 255, alpha: 1)
        isMultipleTouchEnabled = true
        setUpMetal()
        setUpLabels()
        // the names follow the reader's text size
        registerForTraitChanges([UITraitPreferredContentSizeCategory.self]) { (view: GlobeView, _: UITraitCollection) in
            view.relabel()
        }
        let pan = UIPanGestureRecognizer(target: self, action: #selector(pan(_:)))
        pan.maximumNumberOfTouches = 2
        let pinch = UIPinchGestureRecognizer(target: self, action: #selector(pinch(_:)))
        let double = UITapGestureRecognizer(target: self, action: #selector(doubleTap(_:)))
        double.numberOfTapsRequired = 2
        let tap = UITapGestureRecognizer(target: self, action: #selector(tap(_:)))
        tap.require(toFail: double)
        // a pack's drag on the world (runtime 3): the globe's own turn waits until it is clear the
        // drag is not the pack's
        let world = UIPanGestureRecognizer(target: self, action: #selector(worldDragged(_:)))
        world.maximumNumberOfTouches = 1
        pan.require(toFail: world)
        for g in [pan, pinch, double, tap, world] as [UIGestureRecognizer] { g.delegate = self; addGestureRecognizer(g) }
        panRecognizer = pan
        worldDragRecognizer = world
        isAccessibilityElement = true
        accessibilityLabel = String(localized: "The globe")
        accessibilityHint = String(localized: "Drag to spin it, pinch to zoom, tap a country to name it")
        accessibilityTraits = [.image, .allowsDirectInteraction]
        #if DEBUG
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-globe-at"), i + 3 < args.count,
           let lon = Double(args[i + 1]), let lat = Double(args[i + 2]), let z = Double(args[i + 3]) {
            lon0 = lon; lat0 = lat; zoom = z; held = true
        }
        if let i = args.firstIndex(of: "-globe-month"), i + 1 < args.count, let m = Int(args[i + 1]) {
            month = min(max(m, 1), 12)
        }
        if let i = args.firstIndex(of: "-globe-select"), i + 1 < args.count,
           let k = data?.countries.firstIndex(where: { $0.n == args[i + 1] }) {
            select(k, animated: false)
        }
        #endif
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    // MARK: Metal

    private var metalLayer: CAMetalLayer? { layer as? CAMetalLayer }

    private func setUpMetal() {
        guard let device, let metalLayer, let library = device.makeDefaultLibrary(),
              let vertex = library.makeFunction(name: "globe_vertex"),
              let fragment = library.makeFunction(name: "globe_fragment") else { return }
        metalLayer.device = device
        metalLayer.pixelFormat = .bgra8Unorm
        metalLayer.framebufferOnly = true
        metalLayer.isOpaque = true
        let d = MTLRenderPipelineDescriptor()
        d.vertexFunction = vertex
        d.fragmentFunction = fragment
        d.colorAttachments[0].pixelFormat = .bgra8Unorm
        pipeline = try? device.makeRenderPipelineState(descriptor: d)
        queue = device.makeCommandQueue()
        guard let queue else { return }
        geo = Self.texture(GlobeData.root.appending(path: "geo.png"), device: device, queue: queue)
        terrain = Self.texture(GlobeData.root.appending(path: "terrain.png"), device: device, queue: queue)
        band = Self.texture(GlobeData.root.appending(path: "band.png"), device: device, queue: queue, premultiply: true)
        fleuron = Self.texture(GlobeData.root.appending(path: "fleuron.png"), device: device, queue: queue, premultiply: true)
        braid = Self.texture(GlobeData.root.appending(path: "braid.png"), device: device, queue: queue)
        if let source = CGImageSourceCreateWithURL(GlobeData.root.appending(path: "rosette.png") as CFURL, nil) {
            rosette = CGImageSourceCreateImageAtIndex(source, 0, nil)
        }
        // (an ornament that won't load leaves its place empty; the globe still draws)
        let blank = Self.blank(device: device)
        self.blank = blank
        if band == nil || fleuron == nil || braid == nil {
            Logger(subsystem: "com.pbordjadze.terraincognita", category: "globe")
                .error("ornament missing or unreadable: band \(self.band != nil) fleuron \(self.fleuron != nil) braid \(self.braid != nil)")
        }
        band = band ?? blank
        fleuron = fleuron ?? blank
        braid = braid ?? blank
        if geo == nil || terrain == nil || data == nil {
            Logger(subsystem: "com.pbordjadze.terraincognita", category: "globe")
                .error("globe data missing or unreadable: geo \(self.geo != nil) terrain \(self.terrain != nil) data \(self.data != nil)")
        }
        // the countries' map, from the bytes the tap reads (decoded once)
        if let data, data.ids.count == data.width * data.height {
            let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .r8Unorm, width: data.width, height: data.height, mipmapped: false)
            d.usage = .shaderRead
            ids = device.makeTexture(descriptor: d)
            ids?.replace(region: MTLRegionMake2D(0, 0, data.width, data.height), mipmapLevel: 0, withBytes: data.ids, bytesPerRow: data.width)
        }
    }

    /// What the shader reads in place of a map not (yet) read: the ground's, while the style isn't on.
    private var blank: MTLTexture?

    /// Reads the ground style's maps for the month, if not read already (only once the style is
    /// chosen: the globes a prototype makes for itself in Wikipedia's style never read them).
    private func loadGround() {
        guard style == .ground || groundMix > 0, groundMonth != month, let device, let queue else { return }
        let folder = GlobeData.root.appending(path: "ground", directoryHint: .isDirectory)
        let name = String(format: "%02d", month)
        let land = Self.colourTexture(folder.appending(path: "\(name).jpg"), device: device, queue: queue)
        let sea = Self.texture(folder.appending(path: "ice-\(name).png"), device: device, queue: queue)
        if seaFloor == nil { seaFloor = Self.texture(folder.appending(path: "floor.png"), device: device, queue: queue) }
        if land == nil || sea == nil || seaFloor == nil {
            Logger(subsystem: "com.pbordjadze.terraincognita", category: "globe")
                .error("ground maps missing or unreadable for month \(name): land \(land != nil) ice \(sea != nil) floor \(self.seaFloor != nil)")
        }
        ground = land
        ice = sea
        groundMonth = month
        groundSample = Self.sample(folder.appending(path: "\(name).jpg"), size: 512)
        relabel()
        needsDraw = true
    }

    /// A colour picture (the ground's month) drawn into RGBA bytes and uploaded, with mipmaps: unlike
    /// a map of distances it may pass through colour management, so whatever layout the decoder
    /// gives (a JPEG's often isn't RGBA) is drawn into one the shader reads.
    private static func colourTexture(_ url: URL, device: MTLDevice, queue: MTLCommandQueue) -> MTLTexture? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { return nil }
        let w = image.width, h = image.height
        var rgba = [UInt8](repeating: 255, count: w * h * 4)
        let drawn = rgba.withUnsafeMutableBytes { buffer -> Bool in
            guard let c = CGContext(data: buffer.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else { return false }
            c.interpolationQuality = .none
            c.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
            return true
        }
        guard drawn else { return nil }
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: w, height: h, mipmapped: true)
        d.usage = .shaderRead
        guard let texture = device.makeTexture(descriptor: d) else { return nil }
        texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: rgba, bytesPerRow: w * 4)
        if let commands = queue.makeCommandBuffer(), let blit = commands.makeBlitCommandEncoder() {
            blit.generateMipmaps(for: texture)
            blit.endEncoding()
            commands.commit()
        }
        return texture
    }

    /// A small RGB copy of a map (its longer side `size` pixels), to look colours up on the CPU.
    private static func sample(_ url: URL, size: Int) -> (bytes: [UInt8], width: Int, height: Int)? {
        let options = [kCGImageSourceCreateThumbnailFromImageAlways: true, kCGImageSourceThumbnailMaxPixelSize: size] as CFDictionary
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options) else { return nil }
        let w = image.width, h = image.height
        var rgba = [UInt8](repeating: 0, count: w * h * 4)
        let drawn = rgba.withUnsafeMutableBytes { buffer -> Bool in
            guard let c = CGContext(data: buffer.baseAddress, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
                                    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.noneSkipLast.rawValue) else { return false }
            c.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
            return true
        }
        return drawn ? (rgba, w, h) : nil
    }

    /// The tone of a name's halo on the ground style: the ground under its place, lifted 40% towards
    /// the paper, so the darkest forest still gives its ink the contrast type.json holds it to.
    private func groundHalo(lon: Double, lat: Double, fallback: String) -> String {
        guard let s = groundSample, s.width > 0, s.height > 0 else { return fallback }
        let x = (Int(((lon + 180) / 360 * Double(s.width)).rounded(.down)) % s.width + s.width) % s.width
        let y = min(max(Int(((90 - lat) / 180 * Double(s.height)).rounded(.down)), 0), s.height - 1)
        let i = (y * s.width + x) * 4
        let paper: [Double] = [0xF6, 0xF2, 0xE7]
        var c: [Int] = []
        for k in 0..<3 {
            let ground = Double(s.bytes[i + k])
            let lifted: Double = ground * 0.6 + paper[k] * 0.4
            c.append(Int(lifted.rounded()))
        }
        return String(format: "#%02X%02X%02X", c[0], c[1], c[2])
    }

    /// One transparent pixel, in place of an ornament that won't load.
    private static func blank(device: MTLDevice) -> MTLTexture? {
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: 1, height: 1, mipmapped: false)
        d.usage = .shaderRead
        let texture = device.makeTexture(descriptor: d)
        let clear: [UInt8] = [0, 0, 0, 0]
        texture?.replace(region: MTLRegionMake2D(0, 0, 1, 1), mipmapLevel: 0, withBytes: clear, bytesPerRow: 4)
        return texture
    }

    /// An RGBA map's decoded bytes uploaded as they are (colour management would bend the distances
    /// it is made of), with mipmaps. Bytes in any order but R, G, B, A are refused rather than read
    /// wrong. A picture with transparency (`premultiply`) is premultiplied if it isn't, so that its
    /// mipmaps blend its edges right.
    private static func texture(_ url: URL, device: MTLDevice, queue: MTLCommandQueue, premultiply: Bool = false) -> MTLTexture? {
        guard let source = CGImageSourceCreateWithURL(url as CFURL, nil),
              let image = CGImageSourceCreateImageAtIndex(source, 0, nil), image.bitsPerComponent == 8,
              let data = image.dataProvider?.data, let bytes = CFDataGetBytePtr(data) else { return nil }
        let w = image.width, h = image.height, row = image.bytesPerRow, channels = image.bitsPerPixel / 8
        let order = image.byteOrderInfo
        guard order == .orderDefault || order == .order32Big else { return nil }
        switch (channels, image.alphaInfo) {
        case (4, .last), (4, .premultipliedLast), (4, .noneSkipLast), (3, .none): break
        default: return nil
        }
        guard CFDataGetLength(data) >= row * h else { return nil }
        let d = MTLTextureDescriptor.texture2DDescriptor(pixelFormat: .rgba8Unorm, width: w, height: h, mipmapped: true)
        d.usage = .shaderRead
        guard let texture = device.makeTexture(descriptor: d) else { return nil }
        if channels == 3 {
            // (a decoder may drop an alpha that is opaque everywhere)
            var rgba = [UInt8](repeating: 255, count: w * h * 4)
            rgba.withUnsafeMutableBufferPointer { dst in
                for y in 0..<h {
                    for x in 0..<w {
                        let s = y * row + x * 3, t = (y * w + x) * 4
                        dst[t] = bytes[s]; dst[t + 1] = bytes[s + 1]; dst[t + 2] = bytes[s + 2]
                    }
                }
            }
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: rgba, bytesPerRow: w * 4)
        } else if premultiply && image.alphaInfo == .last {
            var rgba = [UInt8](repeating: 0, count: w * h * 4)
            rgba.withUnsafeMutableBufferPointer { dst in
                for y in 0..<h {
                    for x in 0..<w {
                        let s = y * row + x * 4, t = (y * w + x) * 4
                        let a = UInt16(bytes[s + 3])
                        dst[t] = UInt8((UInt16(bytes[s]) * a + 127) / 255)
                        dst[t + 1] = UInt8((UInt16(bytes[s + 1]) * a + 127) / 255)
                        dst[t + 2] = UInt8((UInt16(bytes[s + 2]) * a + 127) / 255)
                        dst[t + 3] = bytes[s + 3]
                    }
                }
            }
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: rgba, bytesPerRow: w * 4)
        } else {
            texture.replace(region: MTLRegionMake2D(0, 0, w, h), mipmapLevel: 0, withBytes: bytes, bytesPerRow: row)
        }
        if let commands = queue.makeCommandBuffer(), let blit = commands.makeBlitCommandEncoder() {
            blit.generateMipmaps(for: texture)
            blit.endEncoding()
            commands.commit()
        }
        return texture
    }

    private nonisolated struct Uniforms {
        var u0: SIMD4<Float>
        var u1: SIMD4<Float>
        var u2: SIMD4<Float>
        var r0: SIMD4<Float>
        var r1: SIMD4<Float>
        var r2: SIMD4<Float>
        var u3: SIMD4<Float>
        var u4: SIMD4<Float>
    }

    /// Draws a frame; false if it couldn't (no drawable yet), so the next tick tries again.
    private func draw() -> Bool {
        guard let pipeline, let queue, let geo, let terrain, let ids, let band, let fleuron, let braid, let metalLayer,
              bounds.width > 0, bounds.height > 0, let drawable = metalLayer.nextDrawable(),
              let commands = queue.makeCommandBuffer() else { return false }
        let pass = MTLRenderPassDescriptor()
        pass.colorAttachments[0].texture = drawable.texture
        pass.colorAttachments[0].loadAction = .dontCare
        pass.colorAttachments[0].storeAction = .store
        guard let encoder = commands.makeRenderCommandEncoder(descriptor: pass) else { return false }
        let scale = Double(metalLayer.contentsScale)
        let (centre, radius) = place
        let m = rotation
        // (the ground's maps not read, or unreadable: Wikipedia's colours rather than a black land)
        let groundShown = ground != nil && ice != nil && seaFloor != nil ? groundMix : 0
        let ring = (1 - smoothstep(1.25, 1.7, zoom)) * (1 - groundShown)
        var u = Uniforms(
            u0: SIMD4(Float(Double(bounds.width) * scale), Float(Double(bounds.height) * scale), Float(Double(centre.x) * scale), Float(Double(centre.y) * scale)),
            u1: SIMD4(Float(radius * scale), Float(scale * Double(reading)), Float(zoom), Float(ring)),
            u2: SIMD4(Float(shown.map { $0 + 1 } ?? 0), Float(highlight), Float(styleMix), Float(pageFrame)),
            r0: SIMD4(Float(m[0][0]), Float(m[1][0]), Float(m[2][0]), 0),
            r1: SIMD4(Float(m[0][1]), Float(m[1][1]), Float(m[2][1]), 0),
            r2: SIMD4(Float(m[0][2]), Float(m[1][2]), Float(m[2][2]), 0),
            u3: SIMD4(Float(Double(safeAreaInsets.top) * scale), Float(Double(safeAreaInsets.bottom) * scale),
                      Float(Double(safeAreaInsets.left) * scale), Float(Double(safeAreaInsets.right) * scale)),
            u4: SIMD4(historicalReference ? 1 : 0, Float(groundShown), 0, 0))
        encoder.setRenderPipelineState(pipeline)
        encoder.setFragmentBytes(&u, length: MemoryLayout<Uniforms>.stride, index: 0)
        encoder.setFragmentTexture(geo, index: 0)
        encoder.setFragmentTexture(terrain, index: 1)
        encoder.setFragmentTexture(ids, index: 2)
        encoder.setFragmentTexture(band, index: 3)
        encoder.setFragmentTexture(fleuron, index: 4)
        encoder.setFragmentTexture(braid, index: 5)
        encoder.setFragmentTexture(ground ?? blank, index: 6)
        encoder.setFragmentTexture(ice ?? blank, index: 7)
        encoder.setFragmentTexture(seaFloor ?? blank, index: 8)
        encoder.drawPrimitives(type: .triangle, vertexStart: 0, vertexCount: 3)
        encoder.endEncoding()
        commands.present(drawable)
        commands.commit()
        return true
    }

    // MARK: the camera

    /// The globe's centre and radius on screen, in points (tools/globe/preview.py's `layout`): as large
    /// as its room allows with its frame round it (Wikipedia's gold ring reaches 11 reading points
    /// past the edge; the Persian frame's finials 0.1833 of the radius), 14 pt from the room's edges.
    private var place: (centre: CGPoint, radius: Double) {
        let box = room ?? CGRect(x: 0, y: Double(bounds.height) * 0.08, width: Double(bounds.width), height: Double(bounds.height) * 0.7)
        let half = min(Double(box.width), Double(box.height)) / 2 - 14
        let ring = 11 * Double(reading)
        let r = (half - ring) + (half / 1.1833 - (half - ring)) * styleMix
        return (CGPoint(x: box.midX, y: box.midY), max(r, 40) * zoom)
    }

    /// How far the Persian page's frame has come in: as the globe's own frame leaves the screen, the
    /// screen becomes the manuscript's panel (Globe.metal's page frame).
    private var pageFrame: Double { styleMix * smoothstep(1.3, 2.0, zoom) }

    /// The page frame's four strips, in points, while it shows: no name goes under them.
    private var pageStrips: [CGRect] {
        guard pageFrame > 0.01 else { return [] }
        let d = (6 + 16.81) * reading, s = safeAreaInsets, w = bounds.width, h = bounds.height
        return [CGRect(x: 0, y: 0, width: w, height: s.top + d), CGRect(x: 0, y: h - s.bottom - d, width: w, height: s.bottom + d),
                CGRect(x: 0, y: 0, width: s.left + d, height: h), CGRect(x: w - s.right - d, y: 0, width: s.right + d, height: h)]
    }

    /// Measures the globe's room from the safe area and what covers the screen (`keepClear`: the
    /// buttons above, the caption below).
    private func measureRoom() {
        var top = Double(safeAreaInsets.top), bottom = Double(bounds.height - safeAreaInsets.bottom)
        for r in keepClear where !r.isEmpty {
            if r.midY < bounds.midY { top = max(top, Double(r.maxY)) } else { bottom = min(bottom, Double(r.minY)) }
        }
        room = CGRect(x: 0, y: top, width: Double(bounds.width), height: max(bottom - top, Double(bounds.height) * 0.4))
        // (rects laid out for another size, as just after a rotation, don't settle it: their
        // successors are measured again when they come)
        let measured = !keepClear.isEmpty && clearBounds == bounds.size
        if measured && !roomMeasured { roomSince = CACurrentMediaTime() }
        roomMeasured = measured
        needsDraw = true
    }

    /// The screen, its safe area or the style changed: the room is measured again, and again as
    /// what covers the screen settles into its new place.
    private func unsettleRoom() {
        roomMeasured = false
        measureRoom()
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        unsettleRoom()
    }

    /// World from view: Ry(lon0) . Rx(-lat0), built from its columns (simd matrices are indexed by
    /// column: m[column][row]), and kept while the camera stays put: every name asks for it each frame.
    private var rotation: simd_double3x3 {
        if let c = cachedRotation, c.lon == lon0, c.lat == lat0 { return c.m }
        let a = lon0 * .pi / 180, b = -lat0 * .pi / 180
        let m = simd_double3x3(SIMD3<Double>(cos(a), 0, -sin(a)),
                               SIMD3<Double>(sin(a) * sin(b), cos(b), cos(a) * sin(b)),
                               SIMD3<Double>(sin(a) * cos(b), -sin(b), cos(a) * cos(b)))
        cachedRotation = (lon: lon0, lat: lat0, m: m)
        return m
    }

    /// Where a point of the globe is on screen, and how far it faces the viewer (1 at the centre, 0 at
    /// the edge, negative behind).
    private func project(lon: Double, lat: Double) -> (point: CGPoint, facing: Double) {
        let la = lat * .pi / 180, lo = lon * .pi / 180
        let w = SIMD3<Double>(cos(la) * sin(lo), sin(la), cos(la) * cos(lo))
        let v = rotation.transpose * w
        let (centre, radius) = place
        return (CGPoint(x: Double(centre.x) + v.x * radius, y: Double(centre.y) - v.y * radius), v.z)
    }

    /// The point of the globe under a point of the screen, if it is on the globe.
    private func unproject(_ p: CGPoint) -> (lon: Double, lat: Double)? {
        let (centre, radius) = place
        let x = (Double(p.x) - Double(centre.x)) / radius, y = (Double(centre.y) - Double(p.y)) / radius
        let r2 = x * x + y * y
        guard r2 < 1 else { return nil }
        let w = rotation * SIMD3<Double>(x, y, sqrt(1 - r2))
        return (atan2(w.x, w.z) * 180 / .pi, asin(min(max(w.y, -1), 1)) * 180 / .pi)
    }

    /// Turns the globe as a finger moving `d` points would drag its centre.
    private func turn(by d: CGVector) {
        let radius = place.radius
        guard radius > 0 else { return }
        lon0 -= Double(d.dx) / radius * 180 / .pi
        lat0 = min(max(lat0 + Double(d.dy) / radius * 180 / .pi, -80), 80)
        lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
        needsDraw = true
    }

    // MARK: the reader's hands

    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        if markers.contains(where: { touch.view?.isDescendant(of: $0.view) == true }) { return false }
        if gestureRecognizer === worldDragRecognizer { worldDragStart = touch.location(in: self) }
        return !(touch.view is UIControl)
    }

    override func gestureRecognizerShouldBegin(_ g: UIGestureRecognizer) -> Bool {
        guard g === worldDragRecognizer else { return super.gestureRecognizerShouldBegin(g) }
        guard onWorldDrag != nil, !authoredMode else { return false }
        if dragScheme == .oneFinger { return true }
        return worldDragBegins?(worldDragStart ?? g.location(in: self)) ?? false
    }

    @objc private func worldDragged(_ g: UIPanGestureRecognizer) {
        let p = g.location(in: self)
        switch g.state {
        case .began:
            touched()
            let start = worldDragStart ?? p
            onWorldDrag?("began", start, unproject(start))
            if start != p { onWorldDrag?("moved", p, unproject(p)) }
        case .changed: onWorldDrag?("moved", p, unproject(p))
        case .ended, .cancelled, .failed:
            onWorldDrag?("ended", p, unproject(p))
            lastTouch = CACurrentMediaTime()
        default: break
        }
    }

    private func touched() {
        lastTouch = CACurrentMediaTime()
        flight = nil
        spin = .zero
        #if DEBUG
        held = false
        #endif
    }

    @objc private func pan(_ g: UIPanGestureRecognizer) {
        switch g.state {
        case .began: onUserNavigation?(); touching += 1; touched()
        case .changed:
            let d = g.translation(in: self)
            g.setTranslation(.zero, in: self)
            turn(by: CGVector(dx: d.x, dy: d.y))
        case .ended, .cancelled, .failed:
            touching = max(0, touching - 1)
            lastTouch = CACurrentMediaTime()
            // the fling, in points a second; Reduce Motion lets the globe stop where it's left
            let v = g.velocity(in: self)
            spin = g.state == .ended && !UIAccessibility.isReduceMotionEnabled ? CGVector(dx: v.x, dy: v.y) : .zero
        default: break
        }
    }

    @objc private func pinch(_ g: UIPinchGestureRecognizer) {
        switch g.state {
        case .began: onUserNavigation?(); touching += 1; touched()
        case .changed:
            let f = g.location(in: self)
            let under = unproject(f)
            zoom = min(max(zoom * Double(g.scale), Self.zooms.lowerBound), zoomLimit)
            g.scale = 1
            // the point under the fingers stays under them (two passes: the turn is a little curved)
            if let under {
                for _ in 0..<2 {
                    let now = project(lon: under.lon, lat: under.lat).point
                    turn(by: CGVector(dx: f.x - now.x, dy: f.y - now.y))
                }
            }
            needsDraw = true
        case .ended, .cancelled, .failed: touching = max(0, touching - 1); lastTouch = CACurrentMediaTime()
        default: break
        }
    }

    @objc private func doubleTap(_ g: UITapGestureRecognizer) {
        onUserNavigation?()
        touched()
        let target = unproject(g.location(in: self)) ?? (lon: lon0, lat: lat0)
        let z = zoom < zoomLimit / 2 ? min(zoom * 2, zoomLimit) : 1
        fly(to: (lon: zoom < 4 ? target.lon : lon0, lat: zoom < 4 ? target.lat : lat0, zoom: z))
    }

    @objc private func tap(_ g: UITapGestureRecognizer) {
        touched()
        let p = g.location(in: self)
        if let onWorldTap, onWorldTap(p, unproject(p)) { return }
        guard !historicalReference else { return }
        guard let k = country(near: g.location(in: self)), k != selected else {
            select(nil, animated: true)
            return
        }
        select(k, animated: true)
    }

    /// The country under a point of the screen or, off its coast, the nearest within a fingertip
    /// (`GlobeType.touch` reading points): small islands and states are smaller than a finger.
    private func country(near p: CGPoint) -> Int? {
        let reach = CGFloat(GlobeType.shared?.touch ?? 10) * reading
        if let w = unproject(p), let k = data?.country(lon: w.lon, lat: w.lat) { return k }
        // rings half a point apart, each probed every half point round it: the first land met is
        // the nearest (to half a point), however small the island
        var r: CGFloat = 0.5
        while r <= reach {
            let n = Int((4 * .pi * r).rounded(.up))
            for i in 0..<n {
                let a = 2 * Double.pi * Double(i) / Double(n)
                let q = CGPoint(x: p.x + r * CGFloat(cos(a)), y: p.y + r * CGFloat(sin(a)))
                if let w = unproject(q), let k = data?.country(lon: w.lon, lat: w.lat) { return k }
            }
            r += 0.5
        }
        return nil
    }

    func gestureRecognizer(_ g: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        !(g is UITapGestureRecognizer) && !(other is UITapGestureRecognizer)
    }

    private func fly(to target: (lon: Double, lat: Double, zoom: Double)) {
        let duration = UIAccessibility.isReduceMotionEnabled ? 0 : 0.7
        flight = Flight(from: (lon: lon0, lat: lat0, zoom: zoom),
                        to: (lon: min(max(target.lon, -180), 180), lat: min(max(target.lat, -80), 80), zoom: target.zoom),
                        start: CACurrentMediaTime(), duration: duration)
    }

    private func select(_ k: Int?, animated: Bool) {
        let still = !animated || UIAccessibility.isReduceMotionEnabled
        // straight from one country to another, the new one fades up from nothing too
        if let k, let s = shown, s != k, !still { highlight = 0 }
        selected = k
        if k != nil { shown = k }
        if still { highlight = k == nil ? 0 : 1 }
        if let k, let data {
            letterSelected()
            accessibilityValue = data.countries[k].long ?? data.countries[k].n
        } else {
            accessibilityValue = nil
        }
        globeAccessibilityElement?.accessibilityValue = accessibilityValue
        onSelect?(k.flatMap { data?.countries[$0] })
        needsDraw = true
    }

    /// Changes the style: crossfaded, the names lettered afresh (Reduce Motion: at once).
    func setStyle(_ new: GlobeStyle, animated: Bool) {
        guard new != style || !animated else { return }
        style = new
        if !animated || UIAccessibility.isReduceMotionEnabled {
            styleMix = new == .persian ? 1 : 0
            groundMix = new == .ground ? 1 : 0
        }
        loadGround()
        // (the other style's caption is another size: measured again once it has been laid out)
        unsettleRoom()
        backgroundColor = new == .persian ? UIColor(red: 0xF3 / 255, green: 0xE9 / 255, blue: 0xD2 / 255, alpha: 1)
                                          : UIColor(red: 0xF6 / 255, green: 0xF2 / 255, blue: 0xE7 / 255, alpha: 1)
        relabel()
    }

    /// Letters every name afresh (a new style, a new text size).
    private func relabel() {
        for l in labels { l.layer?.removeFromSuperlayer() }
        makeLabels()
        letterSelected()
        labelsFading = true
        needsDraw = true
    }

    /// The chosen country's name, in the style's dress.
    private func letterSelected() {
        guard let k = shown ?? selected, let data else { return }
        let name = Self.short(data.countries[k].n).uppercased()
        let persian = style == .persian
        guard let face = faces["selected"] else { return }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        selectedLetters.string = NSAttributedString(string: name, attributes: Self.attributes(face, labelScale))
        selectedLabel.string = persian ? nil : NSAttributedString(string: name, attributes: Self.attributes(face, labelScale, silhouette: true))
        let size = selectedLetters.preferredFrameSize()
        let pad = persian ? CGSize(width: 14 * labelScale, height: 7 * labelScale) : CGSize(width: 4, height: 2)
        selectedLabel.bounds = CGRect(x: 0, y: 0, width: ceil(size.width) + pad.width, height: ceil(size.height) + pad.height)
        selectedLetters.frame = persian ? CGRect(x: pad.width / 2, y: pad.height / 2, width: ceil(size.width), height: ceil(size.height))
                                        : selectedLabel.bounds
        selectedLabel.backgroundColor = persian ? UIColor(hex: face.halo)?.cgColor : nil
        selectedLabel.borderColor = UIColor(red: 0xC5 / 255, green: 0xAA / 255, blue: 0x6E / 255, alpha: 1).cgColor
        selectedLabel.borderWidth = persian ? 1.2 : 0
        selectedLabel.shadowColor = UIColor(hex: persian ? face.ink : face.halo)?.cgColor
        selectedLabel.shadowOpacity = persian ? 0.22 : 0.8
        selectedLabel.shadowRadius = persian ? 1.5 : 2
        selectedLabel.shadowOffset = persian ? CGSize(width: 0, height: 0.5) : .zero
        selectedRule.frame = selectedLabel.bounds.insetBy(dx: 2.2, dy: 2.2)
        selectedRule.isHidden = !persian
        CATransaction.commit()
    }

    // MARK: each frame

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, displayLink == nil {
            let link = CADisplayLink(target: GlobeLinkTarget(self), selector: #selector(GlobeLinkTarget.tick(_:)))
            link.preferredFrameRateRange = CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
            link.add(to: .main, forMode: .common)
            link.isPaused = suspended
            displayLink = link
            busy = true
            busyRate = 2
        } else if window == nil {
            displayLink?.invalidate()
            displayLink = nil
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        globeAccessibilityElement?.accessibilityFrameInContainerSpace = bounds
        let scale = traitCollection.displayScale > 0 ? traitCollection.displayScale : pixels
        pixels = scale
        selectedLabel.contentsScale = scale
        selectedLetters.contentsScale = scale
        metalLayer?.contentsScale = scale
        metalLayer?.drawableSize = CGSize(width: bounds.width * scale, height: bounds.height * scale)
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        labelLayer.frame = bounds
        CATransaction.commit()
        if room.map({ $0.width != Double(bounds.width) }) ?? true || roomBounds != bounds.size {
            roomBounds = bounds.size
            unsettleRoom()
        }
        // (a screen of another size reads at another scale)
        if abs(labelScale - letteredScale) > 0.001 { relabel() }
        needsDraw = true
    }

    /// The size the room was measured for.
    private var roomBounds = CGSize.zero

    fileprivate func tick(_ link: CADisplayLink) {
        let now = CACurrentMediaTime()
        let dt = min(link.targetTimestamp - (lastTick ?? link.timestamp), 0.05)
        lastTick = link.targetTimestamp
        let still = UIAccessibility.isReduceMotionEnabled
        if let f = flight {
            let u = f.duration > 0 ? min((now - f.start) / f.duration, 1) : 1
            let e = easeInOut(u)
            var dl = f.to.lon - f.from.lon
            if dl > 180 { dl -= 360 } else if dl < -180 { dl += 360 }
            lon0 = f.from.lon + dl * e
            lat0 = f.from.lat + (f.to.lat - f.from.lat) * e
            zoom = exp(log(f.from.zoom) + (log(f.to.zoom) - log(f.from.zoom)) * e)
            lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
            if u >= 1 { flight = nil }
            needsDraw = true
        } else if touching == 0, abs(spin.dx) + abs(spin.dy) > 2 {
            turn(by: CGVector(dx: spin.dx * dt, dy: spin.dy * dt))
            let decay = exp(-3.2 * dt)
            spin = CGVector(dx: spin.dx * decay, dy: spin.dy * decay)
        } else if touching == 0, selected == nil, !still, now - lastTouch > 8, idleTurns {
            // left alone, it turns as the Earth does, west to east, about one turn in a minute and a half
            lon0 -= 4 * dt
            lon0 = (lon0 + 540).truncatingRemainder(dividingBy: 360) - 180
            needsDraw = true
        }
        let target = selected == nil ? 0.0 : 1.0
        let fadingUp = highlight != target
        if fadingUp {
            highlight = still ? target : highlight + (target - highlight) * min(1, dt * 9)
            if abs(highlight - target) < 0.01 { highlight = target }
            if highlight == 0 { shown = nil }
            needsDraw = true
        }
        let styleTarget = style == .persian ? 1.0 : 0.0
        let groundTarget = style == .ground ? 1.0 : 0.0
        let restyling = styleMix != styleTarget || groundMix != groundTarget
        if restyling {
            styleMix = still ? styleTarget : styleMix + (styleTarget - styleMix) * min(1, dt * 6)
            if abs(styleMix - styleTarget) < 0.005 { styleMix = styleTarget }
            groundMix = still ? groundTarget : groundMix + (groundTarget - groundMix) * min(1, dt * 6)
            if abs(groundMix - groundTarget) < 0.005 { groundMix = groundTarget }
            needsDraw = true
        }
        var animating = false
        for m in markers {
            m.advance(now: now, zoom: zoom, reading: reading, still: still) { lon, lat in project(lon: lon, lat: lat).point }
            let p = project(lon: m.lon, lat: m.lat)
            m.view.center = p.point
            let hidden = authoredMode || m.resting || zoom < m.minZoom || zoom > m.maxZoom || p.facing < 0.35
                || !bounds.contains(m.view.frame) || keepClear.contains(where: { $0.intersects(m.view.frame) })
            if m.view.isHidden != hidden { m.view.isHidden = hidden; labelsFading = true }
            if !hidden, m.animates, !still { animating = true }
            // A tiny weight/breath shift belongs to the marker's layer; it never requests a map draw.
            if m.breathes, let image = m.view.imageView {
                image.transform = CGAffineTransform(scaleX: 1, y: still ? 1 : 1 + 0.004 * sin(now * 1.2))
            }
        }
        sky?.frame(of: self)
        onFrame?()
        if historicalReference { labelsFading = false }
        else if needsDraw || labelsFading { labelsFading = updateLabels(dt: dt) }
        if needsDraw, draw() {
            needsDraw = false
            if let onDrawn { self.onDrawn = nil; onDrawn() }
        }
        // the hands, a fling, a flight, the highlight fading or the style changing want the full rate;
        // the slow turn (and the names fading with it) is smooth at 30
        let moving = authoredMode || touching > 0 || flight != nil || abs(spin.dx) + abs(spin.dy) > 2 || fadingUp || restyling
        // something living on the globe (a sprite on its way) is smooth at 60
        let rate = moving ? 2 : animating ? 1 : 0
        if rate != busyRate {
            busyRate = rate
            busy = moving
            link.preferredFrameRateRange = rate == 2 ? CAFrameRateRange(minimum: 60, maximum: 120, preferred: 120)
                : rate == 1 ? CAFrameRateRange(minimum: 30, maximum: 60, preferred: 60)
                : CAFrameRateRange(minimum: 24, maximum: 30, preferred: 30)
        }
    }

    private var idleTurns: Bool {
        if authoredMode || holdsStill { return false }
        #if DEBUG
        return !held
        #else
        return true
        #endif
    }

    // MARK: the names

    /// The style's faces (Globe/type.json).
    private var faces: [String: GlobeType.Face] { GlobeType.shared?.styles[style.rawValue]?.names ?? [:] }

    /// A face's attributes at `scale`. `silhouette`: the same letters filled and stroked thickly (a
    /// negative width does both) in the face's halo, Wikipedia's red under the chosen name.
    private static func attributes(_ face: GlobeType.Face, _ scale: CGFloat, silhouette: Bool = false) -> [NSAttributedString.Key: Any] {
        let size = CGFloat(face.size) * scale
        let font = UIFont(name: face.font, size: size) ?? .systemFont(ofSize: size)
        let ink = UIColor(hex: silhouette ? face.halo : face.ink) ?? .black
        // UIKit's key for measuring, Core Text's for drawing
        var a: [NSAttributedString.Key: Any] = [.font: font, .kern: CGFloat(face.tracking) * size, .foregroundColor: ink,
                                                NSAttributedString.Key(kCTForegroundColorAttributeName as String): ink.cgColor]
        if silhouette {
            a[.strokeWidth] = -18.0
            a[.strokeColor] = ink
            a[NSAttributedString.Key(kCTStrokeColorAttributeName as String)] = ink.cgColor
        }
        return a
    }

    /// How much larger than at 1x lines and names are drawn on this screen: a larger one is held
    /// further away (`GlobeType.readingScale`).
    private var reading: CGFloat {
        let side = min(bounds.width, bounds.height)
        return CGFloat(GlobeType.shared?.readingScale(shortSide: Double(side > 0 ? side : 400)) ?? 1)
    }

    /// The names' scale: the screen's reading scale, and the reader's text size (up to `readerMax`;
    /// fewer names then fit, and the order of claim decides which).
    private var labelScale: CGFloat {
        let reader = UIFontMetrics(forTextStyle: .footnote).scaledValue(for: 10, compatibleWith: traitCollection) / 10
        return reading * min(max(reader, 1), CGFloat(GlobeType.shared?.readerMax ?? 1.5))
    }
    /// The scale the names were last lettered at.
    private var letteredScale: CGFloat = 0

    /// Points round a name's letters in its halo.
    private var halo: CGFloat { CGFloat(GlobeType.shared?.halo ?? 1.5) * labelScale }

    /// A name's picture: its letters over their halo (a round-joined stroke of the ground's tone,
    /// so no line crosses them), and a Persian capital's rosette left of them.
    private func picture(of l: Label) -> CGImage? {
        let format = UIGraphicsImageRendererFormat()
        format.scale = pixels
        format.opaque = false
        let halo = self.halo, mark = l.mark ? rosette : nil
        // the halo's pass: the same letters in the halo's tone (Core Text strokes in the text's own
        // colour unless the string names another, which would draw the halo in ink)
        let tone = (UIColor(hex: l.halo) ?? .white).withAlphaComponent(0.92)
        let toned = NSMutableAttributedString(attributedString: l.text)
        toned.addAttributes([.foregroundColor: tone, NSAttributedString.Key(kCTForegroundColorAttributeName as String): tone.cgColor,
                             NSAttributedString.Key(kCTStrokeColorAttributeName as String): tone.cgColor],
                            range: NSRange(location: 0, length: toned.length))
        let image = UIGraphicsImageRenderer(size: l.size, format: format).image { context in
            let c = context.cgContext
            let line = CTLineCreateWithAttributedString(l.text)
            let haloLine = CTLineCreateWithAttributedString(toned)
            var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
            let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
            if let mark {
                c.draw(mark, in: CGRect(x: halo + 1, y: (l.size.height - l.markSize) / 2, width: l.markSize, height: l.markSize))
            }
            // Core Text draws with y up
            c.translateBy(x: 0, y: l.size.height)
            c.scaleBy(x: 1, y: -1)
            c.textMatrix = .identity
            let origin = CGPoint(x: l.mark ? l.size.width - width - halo - 1 : (l.size.width - width) / 2,
                                 y: (l.size.height - ascent - descent) / 2 + descent)
            c.setLineJoin(.round)
            c.setLineWidth(halo * 2)
            c.setStrokeColor(tone.cgColor)
            c.setTextDrawingMode(.fillStroke)
            c.textPosition = origin
            CTLineDraw(haloLine, c)
            c.setTextDrawingMode(.fill)
            c.textPosition = origin
            CTLineDraw(line, c)
        }
        return image.cgImage
    }

    /// The shorter name a map would letter.
    private static func short(_ name: String) -> String {
        name == "United States of America" ? "United States" : name
    }

    private func setUpLabels() {
        labelLayer.masksToBounds = true
        layer.addSublayer(labelLayer)
        selectedLabel.alignmentMode = .center
        selectedLabel.isHidden = true
        selectedLetters.alignmentMode = .center
        selectedRule.borderColor = UIColor(red: 0x5F / 255, green: 0x6C / 255, blue: 0xA7 / 255, alpha: 1).cgColor
        selectedRule.borderWidth = 0.5
        selectedRule.isHidden = true
        selectedLabel.addSublayer(selectedRule)
        selectedLabel.addSublayer(selectedLetters)
        labelLayer.addSublayer(selectedLabel)
        makeLabels()
    }

    /// The names, lettered in the style's faces at the names' scale: oceans and seas, countries
    /// (larger the more prominent), capitals.
    private func makeLabels() {
        let faces = self.faces
        guard let data, let type = GlobeType.shared else { labels = []; return }
        let persian = style == .persian
        let scale = labelScale, halo = self.halo
        letteredScale = scale
        func label(_ text: String, _ key: String, _ c: [Double], _ kind: Kind, _ index: Int, from: Double, mark: Bool = false,
                   minZoom: Double = 0) -> Label? {
            guard c.count == 2, let face = faces[key] else { return nil }
            let s = NSAttributedString(string: text, attributes: Self.attributes(face, scale))
            let size = s.size()
            let room = 2 * halo + 2
            let markSize = CGFloat(face.size) * 0.9 * scale
            let tone = style == .ground && kind != .sea ? groundHalo(lon: c[0], lat: c[1], fallback: face.halo) : face.halo
            return Label(text: s, size: CGSize(width: ceil(size.width) + room + (mark ? markSize * 1.33 : 0), height: ceil(size.height) + room),
                         lon: c[0], lat: c[1], kind: kind, index: index, from: from, minZoom: minZoom, mark: mark, markSize: markSize,
                         halo: tone, gap: CGFloat(type.gap * face.size) * scale)
        }
        // in the order they claim room: oceans, seas, (on the ground, the land's own names), countries,
        // capitals. The ground keeps the whole Earth unnamed, names the seas and the land's great
        // features from x1.5, the countries from x3.2 and the capitals from x5 on any screen (a larger
        // screen's room would otherwise name every country at once), each on a halo of the ground
        // under it (`groundHalo`).
        let onGround = style == .ground
        let sea = onGround ? 1.5 : 0, country = onGround ? 3.2 : 0, capital = onGround ? 5.0 : 0
        var all: [Label?] = []
        for (i, s) in data.seas.enumerated() where s.ocean { all.append(label(s.n, "ocean", s.c, .sea, i, from: 0, minZoom: sea)) }
        for (i, s) in data.seas.enumerated().sorted(by: { $0.element.r < $1.element.r }) where !s.ocean {
            all.append(label(s.n, "sea", s.c, .sea, i, from: s.r + 2.5, minZoom: sea))
        }
        if onGround {
            for (i, r) in data.regions.enumerated() {
                all.append(label(r.n.uppercased(), "region", r.c, .region, i, from: 4 + (r.r - 1) * 0.9,
                                 minZoom: r.r <= 1 ? 1.5 : r.r <= 2 ? 2.5 : 4))
            }
        }
        for (i, c) in data.countries.enumerated().sorted(by: { ($0.element.min, $0.element.r) < ($1.element.min, $1.element.r) }) {
            all.append(label(Self.short(c.n).uppercased(), type.countryFace(rank: c.r), c.c, .country, i, from: c.min + 0.5, minZoom: country))
        }
        for (i, c) in data.capitals.enumerated().sorted(by: { ($0.element.r ?? 9) < ($1.element.r ?? 9) }) {
            all.append(persian ? label(c.n, "capital", c.c, .capital, i, from: 5.6 + (c.r ?? 5) * 0.25, mark: true)
                               : label("· " + c.n, "capital", c.c, .capital, i, from: 5.6 + (c.r ?? 5) * 0.25, minZoom: capital))
        }
        labels = all.compactMap { $0 }
    }

    /// Where on its picture a name is pinned to its place: the middle, or a capital's rosette.
    private func anchor(_ l: Label) -> CGPoint {
        l.mark ? CGPoint(x: (halo + 1 + l.markSize / 2) / l.size.width, y: 0.5) : CGPoint(x: 0.5, y: 0.5)
    }

    /// Which names show (in their order of claim, where they face the viewer, the zoom has reached
    /// them and they lie on the globe, if they don't touch one already placed), fading them in and
    /// out. Returns whether any is still fading.
    private func updateLabels(dt: Double) -> Bool {
        let (centre, radius) = place
        var fading = false
        let level = log2(2 * .pi * radius / 256) + 1.2      // the web-map zoom the globe is drawn at
        var placed: [CGRect] = []
        let fade = min(1, dt * 10)
        let gap = CGFloat(GlobeType.shared?.gap ?? 0.4) * 10 * labelScale     // round the chosen name and what covers the screen
        let markerClear = markers.compactMap { $0.view.isHidden ? nil : $0.view.frame }
        let clear = (keepClear + pageStrips + markerClear + authoredLabelExclusions).map { $0.insetBy(dx: -gap, dy: -gap) }
        /// Whether a name's picture, there, may show: wholly on the globe (none spills onto the frame
        /// or the paper), on screen, under nothing else and touching no name already placed.
        func onGlobe(_ r: CGRect) -> Bool {
            let far = max(hypot(Double(r.minX - centre.x), Double(r.minY - centre.y)), hypot(Double(r.maxX - centre.x), Double(r.minY - centre.y)),
                          hypot(Double(r.minX - centre.x), Double(r.maxY - centre.y)), hypot(Double(r.maxX - centre.x), Double(r.maxY - centre.y)))
            return far < radius * 0.99 && bounds.contains(r)
        }
        func fits(_ r: CGRect) -> Bool {
            onGlobe(r) && !clear.contains(where: { $0.intersects(r) }) && !placed.contains(where: { $0.intersects(r) })
        }
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        // the chosen country's name first; like every name, readable or not shown: it fades in time,
        // never by where it is (the cut-off has some slack, so it doesn't flicker there)
        var selectedTarget = 0.0
        if let shown, highlight > 0.01, let data {
            let c = data.countries[shown]
            let (p, facing) = project(lon: c.c[0], lat: c.c[1])
            let r = selectedLabel.bounds.offsetBy(dx: p.x - selectedLabel.bounds.width / 2, dy: p.y - selectedLabel.bounds.height / 2)
            if facing > (selectedAlpha > 0.5 ? 0.2 : 0.3), onGlobe(r), !clear.contains(where: { $0.intersects(r) }) {
                selectedTarget = 1
                placed.append(r.insetBy(dx: -gap / 2, dy: -gap / 2))
            }
            selectedLabel.position = p
        }
        selectedAlpha += (selectedTarget - selectedAlpha) * fade
        if abs(selectedTarget - selectedAlpha) < 0.005 { selectedAlpha = selectedTarget } else { fading = true }
        selectedLabel.isHidden = selectedAlpha * highlight < 0.01
        selectedLabel.opacity = Float(selectedAlpha * highlight)
        for i in labels.indices {
            var l = labels[i]
            var target = 0.0
            var point: CGPoint?
            if level >= l.from, zoom >= l.minZoom, !(l.kind == .country && l.index == shown && highlight > 0.01) {
                let (p, facing) = project(lon: l.lon, lat: l.lat)
                point = facing > 0 ? p : nil
                if facing > (l.alpha > 0.5 ? 0.3 : 0.4) {
                    let a = anchor(l)
                    let r = CGRect(x: p.x - l.size.width * a.x, y: p.y - l.size.height / 2, width: l.size.width, height: l.size.height)
                    let g = r.insetBy(dx: -l.gap / 2, dy: -l.gap / 2)
                    if fits(g) {
                        placed.append(g)
                        target = 1
                    }
                }
            }
            if target == 0 && l.alpha < 0.01 {
                if let layer = l.layer, !layer.isHidden { layer.isHidden = true }
                labels[i].alpha = 0
                continue
            }
            l.alpha += (target - l.alpha) * fade
            if abs(target - l.alpha) < 0.005 { l.alpha = target } else { fading = true }
            if l.layer == nil {
                let t = CALayer()
                t.contents = picture(of: l)
                t.contentsScale = pixels
                t.bounds = CGRect(origin: .zero, size: l.size)
                t.anchorPoint = anchor(l)
                labelLayer.insertSublayer(t, below: selectedLabel)
                l.layer = t
            }
            if let layer = l.layer {
                layer.isHidden = false
                // (fading out, it keeps to its place on the globe while that faces the viewer)
                if let point { layer.position = point }
                layer.opacity = Float(l.alpha)
            }
            labels[i] = l
        }
        CATransaction.commit()
        return fading
    }
}

/// Breaks the display link → view retain cycle.
private final class GlobeLinkTarget: NSObject {
    weak var view: GlobeView?

    init(_ view: GlobeView) { self.view = view }

    @objc func tick(_ link: CADisplayLink) { view?.tick(link) }
}
