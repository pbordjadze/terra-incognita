import QuartzCore
import CoreText
import SwiftUI
import UIKit

/// Composition lives in native layers above the existing Metal globe. Only transforms and a small
/// projected range path change each frame; the illustrations and map textures are reused.
final class CamelidCanvas: UIView, UIGestureRecognizerDelegate {
    let globe = GlobeView(frame: .zero)
    private let story: CamelidStory
    private let art = CALayer()
    private let secondArt = CALayer()
    private let branch = CAShapeLayer()
    private let range = CAShapeLayer()
    private let rangeEdge = CAShapeLayer()
    private let ground = CAShapeLayer()
    private let branchLabel = CATextLayer()
    private let camelPlace = CALayer()
    private let llamaPlace = CALayer()
    private let originPlace = CALayer()
    private let asiaPlace = CALayer()
    private let northAmericaPlace = CALayer()
    private var link: CADisplayLink?
    private var lastTick: CFTimeInterval?
    private var reportedAt = -1.0
    private var drawnScene = -1
    private var resumeCamera: GlobeCamera?
    private var resumedAt = 0.0
    private var isActive = true
    private lazy var firstAnimalAccess = CamelidAccessibleAnimal(container: self) { [weak self] in self?.inspectAnimal(second: false) }
    private lazy var secondAnimalAccess = CamelidAccessibleAnimal(container: self) { [weak self] in self?.inspectAnimal(second: true) }
    var reduceMotion = false

    init(story: CamelidStory, style: GlobeStyle) {
        self.story = story
        super.init(frame: .zero)
        backgroundColor = UIColor(hex: "#F2E4C8")
        addSubview(globe)
        globe.authoredMode = true
        globe.setStyle(style, animated: false)
        globe.accessibilityIdentifier = "camelidGlobe"
        globe.onUserNavigation = { [weak self] in
            self?.story.pause(); self?.story.exploring = true; self?.resumeCamera = nil
        }
        globe.onFrame = { [weak self] in self?.compose() }
        let inspect = UITapGestureRecognizer(target: self, action: #selector(inspectTapped(_:)))
        inspect.delegate = self
        addGestureRecognizer(inspect)
        // A tap on an illustration opens its evidence. Drags and pinches still belong to the
        // underlying globe; there is no view-sized transparent button over the map.
        for recognizer in globe.gestureRecognizers ?? [] {
            if let tap = recognizer as? UITapGestureRecognizer {
                if tap.numberOfTapsRequired == 1 { tap.require(toFail: inspect) }
                else { inspect.require(toFail: tap) }
            }
        }
        for layer in [rangeEdge, range, ground, branch, branchLabel, art, secondArt, camelPlace, llamaPlace, originPlace, asiaPlace, northAmericaPlace] { globe.layer.addSublayer(layer) }
        range.fillColor = nil
        range.strokeColor = UIColor(hex: "#6D6793")?.withAlphaComponent(0.18).cgColor
        range.lineWidth = 32
        range.lineCap = .round
        range.lineJoin = .round
        rangeEdge.fillColor = nil
        rangeEdge.strokeColor = UIColor(hex: "#6D6793")?.withAlphaComponent(0.10).cgColor
        rangeEdge.lineWidth = 52
        rangeEdge.lineCap = .round
        rangeEdge.lineJoin = .round
        branch.fillColor = nil
        branch.strokeColor = UIColor(hex: "#6D6793")?.cgColor
        branch.lineWidth = 1.6
        branch.lineCap = .round
        ground.fillColor = UIColor(hex: "#79A18B")?.withAlphaComponent(0.15).cgColor
        branchLabel.font = UIFont(name: "LibertinusSerif-Regular", size: 14)
        branchLabel.fontSize = 14
        branchLabel.contentsScale = 3
        branchLabel.foregroundColor = UIColor(hex: "#3B2C16")?.cgColor
        branchLabel.alignmentMode = .center
        for label in [camelPlace, llamaPlace, originPlace, asiaPlace, northAmericaPlace] {
            label.contentsScale = 3
            label.contentsGravity = .resizeAspect
        }
        for layer in [art, secondArt] { layer.contentsGravity = .resizeAspect; layer.contentsScale = 3 }
        story.canvas = self
        globe.setAuthoredCamera(camera(at: story.preciseTime))
        #if DEBUG
        if !story.followed { globe.setAuthoredCamera(GlobeCamera(lon: 46, lat: 24, zoom: 1.55)) }
        #else
        if !story.followed { globe.setAuthoredCamera(GlobeCamera(lon: 46, lat: 24, zoom: 1.55)) }
        #endif
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    isolated deinit { link?.invalidate() }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, link == nil {
            let ticker = CADisplayLink(target: CamelidLink(self), selector: #selector(CamelidLink.tick(_:)))
            ticker.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 120, preferred: 60)
            ticker.add(to: .main, forMode: .common)
            ticker.isPaused = !isActive
            link = ticker
            globe.suspended = !isActive
        } else if window == nil {
            link?.invalidate(); link = nil; lastTick = nil; globe.suspended = true
        }
    }
    override func layoutSubviews() {
        super.layoutSubviews()
        globe.frame = bounds
        let top = safeAreaInsets.top + 105
        let bottom = max(160, safeAreaInsets.bottom + 145)
        globe.keepClear = [CGRect(x: 0, y: 0, width: bounds.width, height: top),
                           CGRect(x: 0, y: bounds.height - bottom, width: bounds.width, height: bottom)]
        compose()
    }
    func setActive(_ active: Bool) {
        guard active != isActive else { return }
        isActive = active
        globe.suspended = !active
        link?.isPaused = !active
        lastTick = nil
    }
    func seek() {
        resumeCamera = nil
        globe.setAuthoredCamera(camera(at: story.preciseTime), duration: reduceMotion ? 0 : 1.2)
        drawnScene = -1; compose()
    }
    func resumeFromCurrentCamera() {
        resumeCamera = globe.authoredCamera
        resumedAt = story.preciseTime
    }
    func attend() {
        guard !reduceMotion else { return }
        let response = CAKeyframeAnimation(keyPath: "transform.scale")
        response.values = [1, 1.016, 1.008, 1]
        response.keyTimes = [0, 0.25, 0.65, 1]
        response.duration = 0.85
        response.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
        art.add(response, forKey: "attention")
    }
    private func inspectAnimal(second: Bool) {
        guard story.followed, !story.exploring, story.scene > 0 else { return }
        story.open(second && story.scene == 5 ? .family : .evidence)
    }
    @objc private func inspectTapped(_ tap: UITapGestureRecognizer) {
        let point = tap.location(in: globe)
        if secondArt.opacity > 0.3 && secondArt.frame.contains(point) { inspectAnimal(second: true) }
        else if art.opacity > 0.3 && art.frame.contains(point) { inspectAnimal(second: false) }
    }
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        guard story.followed, !story.exploring, story.scene > 0 else { return false }
        let point = touch.location(in: globe)
        return (art.opacity > 0.3 && art.frame.contains(point)) || (secondArt.opacity > 0.3 && secondArt.frame.contains(point))
    }
    fileprivate func tick(_ frame: CADisplayLink) {
        let dt = min(max(frame.timestamp - (lastTick ?? frame.timestamp), 0), 0.1)
        lastTick = frame.timestamp
        guard story.playing else { return }
        story.preciseTime = min(story.preciseTime + dt, CamelidStory.duration)
        if frame.timestamp - reportedAt > 0.25 {
            story.reportTime(); reportedAt = frame.timestamp
        }
        if reduceMotion {
            // Hold meaningful compositions, then cut on milestones. No spinning or implied
            // intermediate historical geography is required to understand the low-motion story.
            if drawnScene != scene(at: story.preciseTime) {
                globe.setAuthoredCamera(camera(at: CamelidStory.milestones[scene(at: story.preciseTime)]))
            }
        } else {
            let target = camera(at: story.preciseTime)
            if let from = resumeCamera {
                let p = min((story.preciseTime - resumedAt) / 2, 1)
                globe.setAuthoredCamera(mix(from, target, smooth(p)))
                if p >= 1 { resumeCamera = nil }
            } else { globe.setAuthoredCamera(target) }
        }
        if story.preciseTime >= CamelidStory.duration { story.pause() }
    }

    private func scene(at t: Double) -> Int { CamelidStory.milestones.lastIndex(where: { t >= $0 }) ?? 0 }
    private func smooth(_ p: Double) -> Double { let x = min(max(p, 0), 1); return x * x * (3 - 2 * x) }
    private func mix(_ a: GlobeCamera, _ b: GlobeCamera, _ f: Double) -> GlobeCamera {
        var dl = (b.lon - a.lon).truncatingRemainder(dividingBy: 360)
        if dl > 180 { dl -= 360 }; if dl < -180 { dl += 360 }
        return GlobeCamera(lon: a.lon + dl * f, lat: a.lat + (b.lat - a.lat) * f, zoom: a.zoom + (b.zoom - a.zoom) * f)
    }
    private func camera(at t: Double) -> GlobeCamera {
        if t >= 69 && (story.heldComposition || reduceMotion) { return GlobeCamera(lon: -30, lat: 25, zoom: 0.9) }
        let keys: [(Double, GlobeCamera)] = [
            (0, GlobeCamera(lon: 46, lat: 24, zoom: 1.55)),
            (4, GlobeCamera(lon: -5, lat: 35, zoom: 0.92)),
            (10, GlobeCamera(lon: -105, lat: 37, zoom: 1.5)),
            (26, GlobeCamera(lon: -105, lat: 37, zoom: 1.35)),
            (39, GlobeCamera(lon: -175, lat: 58, zoom: 1.35)),
            (50, GlobeCamera(lon: -175, lat: 58, zoom: 1.35)),
            (54, GlobeCamera(lon: -90, lat: 7, zoom: 1.24)),
            (65, GlobeCamera(lon: -72, lat: -16, zoom: 1.4)),
            (69, GlobeCamera(lon: -72, lat: -16, zoom: 1.15)),
            (77, GlobeCamera(lon: -100, lat: 27, zoom: 0.95)),
            (84, GlobeCamera(lon: -30, lat: 25, zoom: 0.9)),
            (87, GlobeCamera(lon: -30, lat: 25, zoom: 0.9))
        ]
        guard let i = keys.indices.dropLast().first(where: { t < keys[$0 + 1].0 }) else { return keys.last!.1 }
        let a = keys[i], b = keys[i + 1]
        return mix(a.1, b.1, smooth((t - a.0) / (b.0 - a.0)))
    }
    private func sprite(_ layer: CALayer, kind: CamelidAnimal, center: CGPoint, width: CGFloat, flip: Bool = false) {
        let image = CamelidArt.image(kind)
        layer.contents = image?.cgImage
        let aspect = image.map { $0.size.height / max($0.size.width, 1) } ?? 0.85
        layer.bounds = CGRect(x: 0, y: 0, width: width, height: width * aspect)
        layer.position = center
        let breathing = reduceMotion || !story.playing ? 1 : 1 + 0.004 * sin(story.preciseTime * 1.6)
        layer.setAffineTransform(CGAffineTransform(scaleX: flip ? -1 : 1, y: breathing))
    }
    private func placeRegion(_ label: CALayer, text: String, lon: Double, lat: Double, width: CGFloat) {
        let p = globe.projectedLocation(lon: lon, lat: lat)
        guard p.facing > 0.25 else { label.opacity = 0; return }
        let size = CGSize(width: width, height: 25)
        if label.name != text || label.bounds.size != size {
            let ink = UIColor(hex: "#3B2C16") ?? .black
            let tone = (UIColor(hex: "#F2E4C8") ?? .white).withAlphaComponent(0.92)
            let letters = NSAttributedString(string: text, attributes: [
                NSAttributedString.Key(kCTFontAttributeName as String): CTFontCreateWithName("LibertinusSerif-Regular" as CFString, 14, nil),
                NSAttributedString.Key(kCTForegroundColorAttributeName as String): ink.cgColor
            ])
            let toned = NSMutableAttributedString(attributedString: letters)
            toned.addAttributes([
                NSAttributedString.Key(kCTForegroundColorAttributeName as String): tone.cgColor,
                NSAttributedString.Key(kCTStrokeColorAttributeName as String): tone.cgColor
            ], range: NSRange(location: 0, length: toned.length))
            let format = UIGraphicsImageRendererFormat()
            format.scale = 3
            format.opaque = false
            let image = UIGraphicsImageRenderer(size: size, format: format).image { context in
                let c = context.cgContext
                let line = CTLineCreateWithAttributedString(letters)
                let haloLine = CTLineCreateWithAttributedString(toned)
                var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
                let textWidth = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
                c.translateBy(x: 0, y: size.height)
                c.scaleBy(x: 1, y: -1)
                c.textMatrix = .identity
                let origin = CGPoint(x: (size.width - textWidth) / 2,
                                     y: (size.height - ascent - descent) / 2 + descent)
                c.setLineJoin(.round)
                c.setLineWidth(3)
                c.setStrokeColor(tone.cgColor)
                c.setTextDrawingMode(.fillStroke)
                c.textPosition = origin
                CTLineDraw(haloLine, c)
                c.setTextDrawingMode(.fill)
                c.textPosition = origin
                CTLineDraw(line, c)
            }
            label.contents = image.cgImage
            label.name = text
        }
        label.frame = CGRect(x: min(max(p.point.x - width / 2, 18), bounds.width - width - 18), y: p.point.y + 8, width: width, height: 25)
        label.opacity = 1
    }
    private func placeModernAnimal(_ layer: CALayer, label: CALayer, kind: CamelidAnimal, place: String,
                                   lon: Double, lat: Double, width: CGFloat, flip: Bool) {
        let p = globe.projectedLocation(lon: lon, lat: lat)
        let image = CamelidArt.image(kind)
        let fittedWidth = min(width, 2 * max(min(p.point.x - 20, bounds.width - 20 - p.point.x), 0))
        let height = fittedWidth * (image.map { $0.size.height / max($0.size.width, 1) } ?? 0.85)
        sprite(layer, kind: kind, center: CGPoint(x: p.point.x, y: p.point.y - height / 2), width: fittedWidth, flip: flip)
        let safe = bounds.insetBy(dx: 18, dy: 18)
        guard p.facing > 0.25, fittedWidth >= 72, safe.contains(layer.frame) else { layer.opacity = 0; label.opacity = 0; return }
        layer.opacity = 1
        placeRegion(label, text: place, lon: lon, lat: lat, width: max(width, 85))
    }
    private func compose() {
        guard bounds.width > 0 else { return }
        CATransaction.begin(); CATransaction.setDisableActions(true)
        let t = story.preciseTime
        // The displayed date and visible era must change together, even between clock reports.
        let current = story.followed ? story.scene : -1
        let w = bounds.width, h = bounds.height
        let maximum = min(w * 0.55, 260)
        let baseY = h * 0.58
        art.opacity = 1; secondArt.opacity = 0; branch.opacity = 0; range.opacity = 0; rangeEdge.opacity = 0; ground.opacity = 0; branchLabel.opacity = 0
        camelPlace.opacity = 0; llamaPlace.opacity = 0; originPlace.opacity = 0; asiaPlace.opacity = 0; northAmericaPlace.opacity = 0
        globe.historicalReference = story.followed && current < 5
        if !story.followed {
            sprite(art, kind: .dromedary, center: CGPoint(x: w * 0.51, y: h * 0.52), width: min(w * 0.61, 310))
        } else if current == 0 {
            sprite(art, kind: .dromedary, center: CGPoint(x: w * 0.51, y: baseY), width: maximum)
            art.opacity = Float(max(1 - t / 5, 0))
        } else if current == 1 {
            sprite(art, kind: .early, center: CGPoint(x: w * 0.5, y: baseY), width: maximum * 0.72)
            ground.path = UIBezierPath(ovalIn: CGRect(x: w * 0.23, y: baseY + maximum * 0.3, width: w * 0.54, height: 14)).cgPath
            ground.opacity = 1
            art.opacity = story.heldComposition || reduceMotion ? 1 : Float(min((t - 10) / 1.2, 1))
        } else if current == 2 {
            let size = min(w * 0.4, 205)
            sprite(art, kind: .camelini, center: CGPoint(x: w * 0.28, y: baseY - 15), width: size)
            sprite(secondArt, kind: .lamini, center: CGPoint(x: w * 0.74, y: baseY - 15), width: size * 0.9, flip: true)
            secondArt.opacity = 1
            let tree = UIBezierPath()
            tree.move(to: CGPoint(x: w * 0.5, y: baseY + size * 0.7))
            tree.addLine(to: CGPoint(x: w * 0.5, y: baseY + size * 0.43))
            tree.addCurve(to: CGPoint(x: w * 0.28, y: baseY + size * 0.26), controlPoint1: CGPoint(x: w * 0.5, y: baseY + size * 0.3), controlPoint2: CGPoint(x: w * 0.28, y: baseY + size * 0.45))
            tree.move(to: CGPoint(x: w * 0.5, y: baseY + size * 0.43))
            tree.addCurve(to: CGPoint(x: w * 0.74, y: baseY + size * 0.26), controlPoint1: CGPoint(x: w * 0.5, y: baseY + size * 0.3), controlPoint2: CGPoint(x: w * 0.74, y: baseY + size * 0.45))
            branch.path = tree.cgPath; branch.opacity = 1
            branchLabel.string = "Family relationships"
            branchLabel.frame = CGRect(x: w * 0.12, y: baseY + size * 0.73, width: w * 0.76, height: 30)
            branchLabel.opacity = 1
            branch.strokeEnd = reduceMotion || story.heldComposition ? 1 : min(max((t - 26) / 5, 0), 1)
        } else if current == 5 {
            let size = min(max(w * 0.3, 110), 205)
            placeModernAnimal(art, label: camelPlace, kind: .dromedary, place: "Arabia", lon: 46, lat: 24, width: size, flip: true)
            placeModernAnimal(secondArt, label: llamaPlace, kind: .llama, place: "Andes", lon: -72, lat: -16, width: size * 0.96, flip: false)
            placeRegion(originPlace, text: "North America · origin", lon: -100, lat: 52, width: min(205, w * 0.54))
        } else {
            let northern = current == 3
            sprite(art, kind: northern ? .camelini : .lamini,
                   center: CGPoint(x: w * 0.56, y: baseY + 12), width: maximum * 0.8)
            let locations: [(Double, Double)] = northern
                ? [(-133, 54), (-150, 61), (-168, 65), (178, 65), (158, 60), (138, 53)]
                : [(-104, 29), (-95, 21), (-85, 13), (-79, 7), (-75, -4), (-69, -18)]
            let path = UIBezierPath()
            var started = false
            for (lon, lat) in locations {
                let p = globe.projectedLocation(lon: lon, lat: lat)
                guard p.facing > 0.05 else { started = false; continue }
                if started { path.addLine(to: p.point) } else { path.move(to: p.point); started = true }
            }
            range.path = path.cgPath; range.opacity = 1
            rangeEdge.path = path.cgPath; rangeEdge.opacity = 1
            range.strokeEnd = reduceMotion || story.heldComposition ? 1 : min(max((t - (northern ? 39 : 54)) / 9, 0), 1)
            rangeEdge.strokeEnd = range.strokeEnd
            if northern {
                placeRegion(asiaPlace, text: "Asia", lon: 135, lat: 61, width: 80)
                placeRegion(northAmericaPlace, text: "North America", lon: -135, lat: 62, width: 120)
            }
        }
        if story.followed, current > 0, !reduceMotion, !story.heldComposition {
            let begin = CamelidStory.milestones[current]
            let end = current < 5 ? CamelidStory.milestones[current + 1] : 1000
            let alpha = Float(min(max((t - begin) / 1.1, 0), 1) * min(max((end - t) / 0.6, 0), 1))
            for layer in [art, secondArt, branch, branchLabel, range, rangeEdge, ground, camelPlace, llamaPlace, originPlace, asiaPlace, northAmericaPlace] { layer.opacity *= alpha }
        }
        if story.exploring {
            for layer in [art, secondArt, branch, branchLabel, range, rangeEdge, ground, camelPlace, llamaPlace, originPlace, asiaPlace, northAmericaPlace] { layer.opacity = 0 }
        }
        if current == 3 && t >= 50 {
            let departure = Float(max(1 - (t - 50) / 0.8, 0))
            for layer in [art, range, rangeEdge, asiaPlace, northAmericaPlace] { layer.opacity *= departure }
        }
        if current == 5 && !story.exploring {
            // Each modern animal stands at its geographic anchor. Keep contemporary map labels
            // clear of the animals and the three regional labels.
            func stableFrame(_ layer: CALayer) -> CGRect {
                CGRect(x: layer.position.x - layer.bounds.width / 2, y: layer.position.y - layer.bounds.height / 2,
                       width: layer.bounds.width, height: layer.bounds.height).insetBy(dx: -8, dy: -8)
            }
            globe.authoredLabelExclusions = [art.opacity > 0 ? stableFrame(art) : .zero, secondArt.opacity > 0 ? stableFrame(secondArt) : .zero,
                                            camelPlace.opacity > 0 ? camelPlace.frame : .zero, llamaPlace.opacity > 0 ? llamaPlace.frame : .zero, originPlace.frame]
        } else if !story.followed && !story.exploring {
            globe.authoredLabelExclusions = [art.frame.insetBy(dx: -8, dy: -8)]
        } else { globe.authoredLabelExclusions = [] }
        let camera = globe.authoredCamera
        var accessible: [Any] = []
        if story.followed && !story.exploring && current > 0 && art.opacity > 0.3 {
            let firstKind: CamelidAnimal = current == 1 ? .early : (current == 4 ? .lamini : (current == 5 ? .dromedary : .camelini))
            firstAnimalAccess.accessibilityLabel = firstKind.name
            firstAnimalAccess.accessibilityHint = "Inspect this illustration and its evidence"
            firstAnimalAccess.accessibilityFrameInContainerSpace = art.frame
            firstAnimalAccess.accessibilityIdentifier = "camelidStoryAnimal"
            accessible.append(firstAnimalAccess)
        }
        if story.followed && !story.exploring && current > 0 && secondArt.opacity > 0.3 {
            secondAnimalAccess.accessibilityLabel = current == 5 ? "Llama" : CamelidAnimal.lamini.name
            secondAnimalAccess.accessibilityHint = current == 5 ? "Compare llama and alpaca" : "Inspect this illustration and its evidence"
            secondAnimalAccess.accessibilityFrameInContainerSpace = secondArt.frame
            secondAnimalAccess.accessibilityIdentifier = "camelidStorySecondAnimal"
            accessible.append(secondAnimalAccess)
        }
        accessible.append(globe)
        accessibilityElements = accessible
        globe.accessibilityLabel = story.exploring ? "Freely exploring the globe. The historical sequence is paused at \(story.date). Resume to return to its composition." : story.description
        globe.accessibilityValue = String(format: "Longitude %.1f, latitude %.1f, scale %.2f", camera.lon, camera.lat, camera.zoom)
        drawnScene = current
        CATransaction.commit()
    }
}

private final class CamelidAccessibleAnimal: UIAccessibilityElement {
    let activate: () -> Void
    init(container: Any, activate: @escaping () -> Void) {
        self.activate = activate
        super.init(accessibilityContainer: container)
        accessibilityTraits = .button
        isAccessibilityElement = true
    }
    override func accessibilityActivate() -> Bool { activate(); return true }
}

private final class CamelidLink: NSObject {
    weak var canvas: CamelidCanvas?
    init(_ canvas: CamelidCanvas) { self.canvas = canvas }
    @objc func tick(_ link: CADisplayLink) { canvas?.tick(link) }
}

struct CamelidCanvasView: UIViewRepresentable {
    let story: CamelidStory
    let style: GlobeStyle
    let active: Bool
    let reduceMotion: Bool
    func makeUIView(context: Context) -> CamelidCanvas { CamelidCanvas(story: story, style: style) }
    func updateUIView(_ view: CamelidCanvas, context: Context) {
        view.reduceMotion = reduceMotion
        view.setActive(active)
    }
}
