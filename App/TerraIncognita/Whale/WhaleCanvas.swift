import CoreText
import Foundation
import QuartzCore
import SwiftUI
import UIKit

/// A local illustrated visit, composed above the atlas's existing geographic renderer.
/// Images are decoded once. Native layer transforms animate the whale independently of Metal.
final class WhaleCanvas: UIView, UIGestureRecognizerDelegate {
    let globe = GlobeView(frame: .zero)
    private let story: WhaleStory
    private let livingImage: UIImage?
    private let fossilImage: UIImage?
    private let limbImage: UIImage?
    private let desertImage: UIImage?
    private let seaImage: UIImage?
    private let desert = CALayer()
    private let marine = CALayer()
    private let fossil = CALayer()
    private let living = CALayer()
    private let limb = CALayer()
    private let leader = CAShapeLayer()
    private let detailLeader = CAShapeLayer()
    private let localityDot = CAShapeLayer()
    private let localityName = CALayer()
    private let regionName = CALayer()
    private let detailName = CALayer()
    private var bodySlices: [CALayer] = []
    private var preparedBodySize = CGSize.zero
    private var link: CADisplayLink?
    private var lastTick: CFTimeInterval?
    private var reportedAt = -1.0
    private var drawnScene = -1
    private var isActive = true
    private var resumeCamera: GlobeCamera?
    private var resumedAt = 0.0
    private var globalKeepClear: [CGRect] = []
    private var localKeepClear: [CGRect] = []
    private var lastComposition: Composition?
    private var artHitFrame = CGRect.zero
    private var visibleLiving = false
    private var accessibleCamera: GlobeCamera?
    private var accessibleDescription = ""
    #if DEBUG
    private var captureToken: String?
    #endif
    private lazy var animalAccess = WhaleAccessibleArt(container: self) { [weak self] in self?.inspectArt() }
    var reduceMotion = false {
        didSet {
            guard reduceMotion != oldValue else { return }
            lastComposition = nil
            compose()
        }
    }

    /// UNESCO's property reference point, not the location of an invented excavation.
    private static let longitude = 30.18333
    private static let latitude = 29.33333
    private static let sliceCount = 32

    private struct Composition: Equatable {
        let camera: GlobeCamera
        let time: Double
        let scene: Int
        let followed: Bool
        let exploring: Bool
        let held: Bool
        let reduced: Bool
        let room: CGRect
        let size: CGSize
    }

    init(story: WhaleStory, style: GlobeStyle) {
        self.story = story
        livingImage = WhaleArt.image(.living)
        fossilImage = WhaleArt.image(.fossil)
        limbImage = WhaleArt.image(.limb)
        desertImage = WhaleArt.image(.desert)
        seaImage = WhaleArt.image(.sea)
        super.init(frame: .zero)
        backgroundColor = UIColor(hex: "#F2E6CC")
        addSubview(globe)
        globe.authoredMode = true
        globe.setStyle(style, animated: false)
        globe.accessibilityIdentifier = "whaleGlobe"
        globe.onUserNavigation = { [weak self] in
            guard let self else { return }
            story.pause()
            story.exploring = true
            resumeCamera = nil
            lastComposition = nil
            compose()
        }
        globe.onFrame = { [weak self] in self?.compose() }
        #if DEBUG
        let arguments = ProcessInfo.processInfo.arguments
        if let index = arguments.firstIndex(of: "-whale-capture-token"), index + 1 < arguments.count {
            let token = arguments[index + 1]
            if UUID(uuidString: token) != nil {
                captureToken = token
                globe.onDrawn = {
                    // First submitted frame, not a GPU-completion or performance measurement.
                    let marker = FileManager.default.temporaryDirectory.appending(path: "whale-capture-ready.txt")
                    do { try token.write(to: marker, atomically: true, encoding: .utf8) }
                    catch { NSLog("Whale capture readiness marker failed: %@", error.localizedDescription) }
                }
            }
        }
        #endif

        let inspect = UITapGestureRecognizer(target: self, action: #selector(inspectTapped(_:)))
        inspect.delegate = self
        addGestureRecognizer(inspect)
        for recognizer in globe.gestureRecognizers ?? [] {
            if let tap = recognizer as? UITapGestureRecognizer {
                if tap.numberOfTapsRequired == 1 { tap.require(toFail: inspect) }
                else { inspect.require(toFail: tap) }
            }
        }
        for layer in [desert, marine, leader, localityDot, fossil, living, detailLeader, limb,
                      localityName, regionName, detailName] { globe.layer.addSublayer(layer) }
        for layer in [desert, marine, fossil, limb] {
            layer.contentsGravity = .resizeAspect
            layer.contentsScale = 3
        }
        desert.contents = desertImage?.cgImage
        marine.contents = seaImage?.cgImage
        fossil.contents = fossilImage?.cgImage
        limb.contents = limbImage?.cgImage
        leader.fillColor = nil
        leader.strokeColor = UIColor(hex: "#52686D")?.withAlphaComponent(0.7).cgColor
        leader.lineWidth = 1
        leader.lineCap = .round
        detailLeader.fillColor = nil
        detailLeader.strokeColor = UIColor(hex: "#52686D")?.withAlphaComponent(0.8).cgColor
        detailLeader.lineWidth = 0.9
        detailLeader.lineCap = .round
        localityDot.fillColor = UIColor(hex: "#344D55")?.cgColor
        localityDot.strokeColor = UIColor(hex: "#F2E6CC")?.cgColor
        localityDot.lineWidth = 2
        for label in [localityName, regionName, detailName] {
            label.contentsScale = 3
            label.contentsGravity = .resizeAspect
        }
        for _ in 0..<Self.sliceCount {
            let slice = CALayer()
            slice.contents = livingImage?.cgImage
            slice.contentsGravity = .resize
            slice.contentsScale = 3
            living.addSublayer(slice)
            bodySlices.append(slice)
        }
        story.canvas = self
        globe.setAuthoredCamera(camera(at: story.preciseTime))
    }

    required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
    isolated deinit { link?.invalidate() }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil, link == nil {
            let ticker = CADisplayLink(target: WhaleClockLink(self), selector: #selector(WhaleClockLink.tick(_:)))
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
        updateClearAreas()
        lastComposition = nil
        compose()
    }

    func setKeepClear(_ frames: [CGRect]) {
        guard globalKeepClear != frames else { return }
        globalKeepClear = frames
        updateClearAreas()
        lastComposition = nil
        compose()
    }

    private func updateClearAreas() {
        guard bounds.width > 0, bounds.height > 0 else { return }
        localKeepClear = globalKeepClear.map { convert($0, from: nil).intersection(bounds) }
            .filter { !$0.isNull && $0.width > 0 && $0.height > 0 }
        if localKeepClear.isEmpty {
            let top = safeAreaInsets.top + 150
            let bottom = max(140, safeAreaInsets.bottom + 120)
            localKeepClear = [CGRect(x: 0, y: 0, width: bounds.width, height: top),
                              CGRect(x: 0, y: bounds.height - bottom, width: bounds.width, height: bottom)]
        }
        globe.keepClear = localKeepClear
    }

    private var room: CGRect {
        var top = safeAreaInsets.top + 100
        var bottom = bounds.height - max(safeAreaInsets.bottom + 110, 130)
        for clear in localKeepClear where clear.width > bounds.width * 0.45 {
            if clear.midY < bounds.midY { top = max(top, clear.maxY + 10) }
            else { bottom = min(bottom, clear.minY - 10) }
        }
        return CGRect(x: 18, y: top, width: max(bounds.width - 36, 1), height: max(bottom - top, 50))
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
        drawnScene = -1
        lastComposition = nil
        compose()
    }

    func resumeFromCurrentCamera() {
        resumeCamera = globe.authoredCamera
        resumedAt = story.preciseTime
        lastTick = nil
        lastComposition = nil
    }

    /// Fossil stone stays still. Touch reveals the drawing's ink instead of making bones breathe.
    func attend() {
        guard !reduceMotion else { return }
        let response = CABasicAnimation(keyPath: "opacity")
        response.fromValue = 0.58
        response.toValue = 1
        response.duration = 0.65
        response.timingFunction = CAMediaTimingFunction(name: .easeOut)
        fossil.add(response, forKey: "attention")
    }

    fileprivate func tick(_ frame: CADisplayLink) {
        let dt = min(max(frame.timestamp - (lastTick ?? frame.timestamp), 0), 0.1)
        lastTick = frame.timestamp
        guard story.playing else { return }
        story.preciseTime = min(story.preciseTime + dt, WhaleStory.duration)
        let fresh = WhaleStory.scene(at: story.preciseTime)
        if fresh != story.scene || frame.timestamp - reportedAt > 0.25 || (story.scene == 0 && story.preciseTime >= 2 && story.date == "Today") {
            story.reportTime()
            reportedAt = frame.timestamp
        }
        if reduceMotion {
            if drawnScene != story.scene {
                globe.setAuthoredCamera(camera(at: WhaleStory.milestones[story.scene]))
            }
        } else {
            let target = camera(at: story.preciseTime)
            if let from = resumeCamera {
                let fraction = min((story.preciseTime - resumedAt) / 2, 1)
                globe.setAuthoredCamera(mix(from, target, smooth(fraction)))
                if fraction >= 1 { resumeCamera = nil }
            } else { globe.setAuthoredCamera(target) }
        }
        if story.preciseTime >= WhaleStory.duration {
            story.pause()
            compose()
            #if DEBUG
            if let token = captureToken {
                let marker = FileManager.default.temporaryDirectory.appending(path: "whale-capture-complete.txt")
                do { try token.write(to: marker, atomically: true, encoding: .utf8) }
                catch { NSLog("Whale capture completion marker failed: %@", error.localizedDescription) }
                captureToken = nil
            }
            #endif
        }
    }

    private func smooth(_ fraction: Double) -> Double {
        let x = min(max(fraction, 0), 1)
        return x * x * (3 - 2 * x)
    }

    private func mix(_ from: GlobeCamera, _ to: GlobeCamera, _ fraction: Double) -> GlobeCamera {
        var longitude = (to.lon - from.lon).truncatingRemainder(dividingBy: 360)
        if longitude > 180 { longitude -= 360 }
        if longitude < -180 { longitude += 360 }
        return GlobeCamera(lon: from.lon + longitude * fraction,
                           lat: from.lat + (to.lat - from.lat) * fraction,
                           zoom: from.zoom + (to.zoom - from.zoom) * fraction)
    }

    private func camera(at time: Double) -> GlobeCamera {
        let keys: [(Double, GlobeCamera)] = [
            (0, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 3.5)),
            (4, GlobeCamera(lon: 24, lat: 28, zoom: 1.3)),
            (10, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 5.8)),
            (25, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 6.4)),
            (43, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 6.4)),
            (56, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 6.4)),
            (64, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 5.0)),
            (82, GlobeCamera(lon: Self.longitude, lat: Self.latitude, zoom: 5.0))
        ]
        guard let index = keys.indices.dropLast().first(where: { time < keys[$0 + 1].0 }) else { return keys.last!.1 }
        let a = keys[index], b = keys[index + 1]
        return mix(a.1, b.1, smooth((time - a.0) / (b.0 - a.0)))
    }

    private func fittedRect(image: UIImage?, width: CGFloat, center: CGPoint, maximumHeight: CGFloat) -> CGRect {
        let aspect = image.map { $0.size.height / max($0.size.width, 1) } ?? 0.3
        var size = CGSize(width: width, height: width * aspect)
        if size.height > maximumHeight {
            size.width *= maximumHeight / max(size.height, 1)
            size.height = maximumHeight
        }
        return CGRect(x: center.x - size.width / 2, y: center.y - size.height / 2,
                      width: max(size.width, 1), height: max(size.height, 1))
    }

    private func bodyDisplacement(_ u: CGFloat, time: Double, width: CGFloat) -> CGFloat {
        guard !reduceMotion && !story.heldComposition else { return 0 }
        let phase = CGFloat(time * 2 * .pi / 3.8)
        let weight = min(max((u - 0.28) / 0.72, 0), 1)
        let eased = weight * weight * (3 - 2 * weight)
        return sin(phase - u * 1.9) * width * 0.018 * eased
    }

    private func poseWhale(in frame: CGRect, time: Double) {
        living.frame = frame
        func displacement(_ u: CGFloat) -> CGFloat {
            bodyDisplacement(u, time: time, width: frame.width)
        }
        let needsLayout = preparedBodySize != frame.size
        if needsLayout { preparedBodySize = frame.size }
        let overlap = 0.45 / max(frame.width, 1)
        for (index, slice) in bodySlices.enumerated() {
            let start = max(CGFloat(index) / CGFloat(Self.sliceCount) - overlap, 0)
            let end = min(CGFloat(index + 1) / CGFloat(Self.sliceCount) + overlap, 1)
            let u = (start + end) / 2
            if needsLayout {
                slice.contentsRect = CGRect(x: start, y: 0, width: end - start, height: 1)
                slice.bounds = CGRect(x: 0, y: 0, width: frame.width * (end - start), height: frame.height)
            }
            let dy = displacement(u)
            let slope = (displacement(min(u + 0.001, 1)) - displacement(max(u - 0.001, 0))) / max(frame.width * 0.002, 0.001)
            slice.position = CGPoint(x: frame.width * u, y: frame.height / 2 + dy)
            slice.setAffineTransform(CGAffineTransform(rotationAngle: atan(slope)))
        }
    }

    /// Small atlas labels use the same separate halo/ink passes as GlobeView's name pictures.
    private func label(_ layer: CALayer, text: String, frame: CGRect, size: CGFloat = 14) {
        if layer.name != text || layer.bounds.size != frame.size {
            let ink = UIColor(hex: "#33494E") ?? .black
            let paper = UIColor(hex: "#F2E6CC") ?? .white
            let letters = NSAttributedString(string: text, attributes: [
                NSAttributedString.Key(kCTFontAttributeName as String): CTFontCreateWithName("LibertinusSerif-Regular" as CFString, size, nil),
                NSAttributedString.Key(kCTForegroundColorAttributeName as String): ink.cgColor
            ])
            let halo = NSMutableAttributedString(attributedString: letters)
            halo.addAttributes([NSAttributedString.Key(kCTForegroundColorAttributeName as String): paper.cgColor,
                                NSAttributedString.Key(kCTStrokeColorAttributeName as String): paper.cgColor],
                               range: NSRange(location: 0, length: halo.length))
            let format = UIGraphicsImageRendererFormat()
            format.scale = 3; format.opaque = false
            let picture = UIGraphicsImageRenderer(size: frame.size, format: format).image { renderer in
                let context = renderer.cgContext
                let line = CTLineCreateWithAttributedString(letters)
                let haloLine = CTLineCreateWithAttributedString(halo)
                var ascent: CGFloat = 0, descent: CGFloat = 0, leading: CGFloat = 0
                let width = CGFloat(CTLineGetTypographicBounds(line, &ascent, &descent, &leading))
                context.translateBy(x: 0, y: frame.height)
                context.scaleBy(x: 1, y: -1)
                context.textMatrix = .identity
                let origin = CGPoint(x: (frame.width - width) / 2,
                                     y: (frame.height - ascent - descent) / 2 + descent)
                context.setLineJoin(.round)
                context.setLineWidth(3)
                context.setStrokeColor(paper.cgColor)
                context.setTextDrawingMode(.fillStroke)
                context.textPosition = origin
                CTLineDraw(haloLine, context)
                context.setTextDrawingMode(.fill)
                context.textPosition = origin
                CTLineDraw(line, context)
            }
            layer.contents = picture.cgImage
            layer.name = text
        }
        layer.frame = frame
        layer.opacity = 1
    }

    private func compose() {
        guard bounds.width > 0, bounds.height > 0 else { return }
        let current = story.followed ? story.scene : -1
        let viewport = room
        let stamp = Composition(camera: globe.authoredCamera, time: story.preciseTime, scene: current,
                                followed: story.followed, exploring: story.exploring,
                                held: story.heldComposition, reduced: reduceMotion,
                                room: viewport, size: bounds.size)
        guard stamp != lastComposition else { return }
        lastComposition = stamp
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        for layer in [desert, marine, fossil, living, limb, detailLeader, localityDot, leader,
                      localityName, regionName, detailName] { layer.opacity = 0 }
        artHitFrame = .zero
        visibleLiving = false
        globe.historicalReference = story.historical

        if !story.exploring {
            let point = globe.projectedLocation(lon: Self.longitude, lat: Self.latitude)
            let width = min(viewport.width * 1.03, 760)
            let height = min(width * 0.58, viewport.height * 0.56)
            let y = min(max(point.point.y + 28, viewport.minY + viewport.height * 0.32), viewport.maxY - height)
            let habitat = CGRect(x: bounds.midX - width / 2, y: y, width: width, height: max(height, 45))
            desert.frame = habitat
            marine.frame = habitat
            let isMarine = (1...3).contains(current)
            let leavingMarine = current == 4 && !story.heldComposition && !reduceMotion
            var alpha: Float = 1
            if current == 0 {
                // After the date cuts to the ancient interval, the present-day fossil has left.
                // The direct Today landmark at zero remains a complete inspection composition.
                alpha = reduceMotion ? (story.preciseTime < 2 ? 1 : 0) : Float(max(1 - story.preciseTime / 2, 0))
            }
            if story.followed && !story.heldComposition && !reduceMotion {
                if current == 4 { alpha = Float(max(1 - (story.preciseTime - 56) / 1.2, 0)) }
                else if current != 0 {
                    // The same living animal remains continuously visible across the marine
                    // views. A new name or anatomical emphasis does not restart its entrance.
                    let start = isMarine ? 10 : WhaleStory.milestones[current]
                    alpha = Float(min(max((story.preciseTime - start) / 1.2, 0), 1))
                }
            }
            if isMarine || leavingMarine {
                // A discrete temporal cut lets the body and habitat leave together. The atlas
                // then stands alone until Today, without a tide draining or an animal stranding.
                marine.opacity = seaImage == nil ? 0 : alpha
                let arrival = current == 1 && !story.heldComposition && !reduceMotion
                    ? smooth((story.preciseTime - 16) / 7) : 1
                living.opacity = livingImage == nil ? 0 : alpha * Float(arrival)
                let detailProgress = current >= 3 ? (story.heldComposition || reduceMotion ? 1 : smooth((story.preciseTime - 43) / 1.5)) : 0
                let bodyCenter = CGPoint(x: habitat.midX + CGFloat(1 - arrival) * width * 0.08,
                                         y: habitat.minY + habitat.height * (0.45 - 0.15 * CGFloat(detailProgress)))
                let body = fittedRect(image: livingImage, width: min(width * 0.91, 640),
                                      center: bodyCenter, maximumHeight: habitat.height * 0.64)
                poseWhale(in: body, time: story.preciseTime)
                visibleLiving = true
                if living.opacity > 0.3 { artHitFrame = body.insetBy(dx: 0, dy: -10) }
                if current == 3 {
                    let detail = fittedRect(image: limbImage, width: min(width * 0.29, 150),
                                            center: CGPoint(x: habitat.midX + width * 0.20,
                                                            y: habitat.minY + habitat.height * 0.76),
                                            maximumHeight: habitat.height * 0.42)
                    limb.frame = detail
                    limb.opacity = limbImage == nil ? 0 : alpha * Float(detailProgress)
                    // The enlarged view is tied to the small appendages near the rear flank.
                    // This is an illustration connector, not a measured anatomical diagram.
                    let hindlimbs = CGPoint(x: body.minX + body.width * 0.78,
                                            y: body.minY + body.height * 0.82
                                                + bodyDisplacement(0.78, time: story.preciseTime, width: body.width))
                    let connection = UIBezierPath()
                    connection.move(to: CGPoint(x: hindlimbs.x, y: hindlimbs.y + 4))
                    connection.addCurve(to: CGPoint(x: detail.midX, y: detail.minY - 3),
                                        controlPoint1: CGPoint(x: hindlimbs.x, y: hindlimbs.y + 22),
                                        controlPoint2: CGPoint(x: detail.midX, y: detail.minY - 20))
                    detailLeader.path = connection.cgPath
                    detailLeader.opacity = alpha * Float(detailProgress) * 0.8
                    label(detailName, text: "Enlarged", frame: CGRect(x: detail.midX - 55, y: detail.maxY + 2, width: 110, height: 22), size: 12)
                    detailName.opacity *= alpha * Float(detailProgress)
                    if limb.opacity > 0.3 { artHitFrame = artHitFrame.union(detail) }
                }
            } else if current != 4 {
                desert.opacity = desertImage == nil ? 0 : alpha
                fossil.opacity = fossilImage == nil ? 0 : alpha
                fossil.frame = fittedRect(image: fossilImage, width: min(width * 0.89, 640),
                                          center: CGPoint(x: habitat.midX, y: habitat.midY - habitat.height * 0.02),
                                          maximumHeight: habitat.height * 0.72)
                if fossil.opacity > 0.3 { artHitFrame = fossil.frame.insetBy(dx: 0, dy: -12) }
            }
            if point.facing > 0.3 && current != 4 {
                localityDot.path = UIBezierPath(ovalIn: CGRect(x: point.point.x - 3, y: point.point.y - 3, width: 6, height: 6)).cgPath
                localityDot.opacity = current == 0 && !story.heldComposition ? 1 : alpha
                let labelWidth = min(viewport.width, 185)
                let labelX = min(max(point.point.x - labelWidth / 2, 18), bounds.width - labelWidth - 18)
                let labelY = max(viewport.minY, min(point.point.y - 27, habitat.minY - 29))
                label(localityName, text: "Wadi Al-Hitan", frame: CGRect(x: labelX, y: labelY, width: labelWidth, height: 25))
                localityName.opacity = localityDot.opacity
                let path = UIBezierPath()
                path.move(to: point.point)
                path.addCurve(to: CGPoint(x: habitat.midX, y: habitat.minY + 4),
                              controlPoint1: CGPoint(x: point.point.x, y: point.point.y + 14),
                              controlPoint2: CGPoint(x: habitat.midX, y: habitat.minY - 8))
                leader.path = path.cgPath
                leader.opacity = alpha * 0.65
            }
            if current == 0 && story.preciseTime >= 2 && story.preciseTime < 8 {
                let north = globe.projectedLocation(lon: 12, lat: 25)
                if north.facing > 0.4 {
                    label(regionName, text: "North Africa", frame: CGRect(x: max(18, min(north.point.x - 75, bounds.width - 168)),
                                                                         y: max(viewport.minY, min(north.point.y - 36, habitat.minY - 56)),
                                                                         width: 150, height: 25))
                }
            }
            var exclusions: [CGRect] = []
            if desert.opacity > 0.05 || marine.opacity > 0.05 { exclusions.append(habitat.insetBy(dx: -6, dy: -6)) }
            if localityName.opacity > 0 { exclusions.append(localityName.frame) }
            if regionName.opacity > 0 { exclusions.append(regionName.frame) }
            globe.authoredLabelExclusions = exclusions
        } else { globe.authoredLabelExclusions = [] }

        var elements: [Any] = []
        if !story.exploring && !artHitFrame.isEmpty {
            animalAccess.accessibilityIdentifier = "whaleStoryAnimal"
            animalAccess.accessibilityLabel = visibleLiving ? "Dorudon, illustrative living reconstruction" : "Whale fossil, illustrative reconstruction"
            animalAccess.accessibilityHint = story.followed ? (visibleLiving ? "Inspect its tiny hind limbs" : "Inspect the fossil evidence") : "Responds to touch. Visit the past whenever you choose."
            animalAccess.accessibilityFrameInContainerSpace = artHitFrame
            elements.append(animalAccess)
        }
        elements.append(globe)
        accessibilityElements = elements
        if accessibleDescription != story.description {
            accessibleDescription = story.description
            globe.accessibilityLabel = accessibleDescription
        }
        if accessibleCamera != globe.authoredCamera {
            accessibleCamera = globe.authoredCamera
            let camera = globe.authoredCamera
            globe.accessibilityValue = String(format: "Longitude %.2f, latitude %.2f, scale %.2f", camera.lon, camera.lat, camera.zoom)
        }
        drawnScene = story.scene
        CATransaction.commit()
    }

    private func inspectArt() {
        guard !story.exploring, !artHitFrame.isEmpty else { return }
        guard story.followed else { attend(); return }
        story.open(visibleLiving ? .limbs : .evidence)
    }

    @objc private func inspectTapped(_ gesture: UITapGestureRecognizer) { inspectArt() }

    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        !story.exploring && !artHitFrame.isEmpty && artHitFrame.contains(touch.location(in: globe))
    }
}

private final class WhaleAccessibleArt: UIAccessibilityElement {
    let activate: () -> Void
    init(container: Any, activate: @escaping () -> Void) {
        self.activate = activate
        super.init(accessibilityContainer: container)
        accessibilityTraits = .button
        isAccessibilityElement = true
    }
    override func accessibilityActivate() -> Bool { activate(); return true }
}

private final class WhaleClockLink: NSObject {
    weak var canvas: WhaleCanvas?
    init(_ canvas: WhaleCanvas) { self.canvas = canvas }
    @objc func tick(_ link: CADisplayLink) { canvas?.tick(link) }
}

struct WhaleCanvasView: UIViewRepresentable {
    let story: WhaleStory
    let style: GlobeStyle
    let active: Bool
    let reduceMotion: Bool
    var keepClear: [CGRect] = []
    func makeUIView(context: Context) -> WhaleCanvas { WhaleCanvas(story: story, style: style) }
    func updateUIView(_ view: WhaleCanvas, context: Context) {
        view.reduceMotion = reduceMotion
        view.setKeepClear(keepClear)
        view.setActive(active)
    }
}
