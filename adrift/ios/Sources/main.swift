// Adrift's device shell: a UIKit app whose one view is a CAMetalLayer that the Rust library draws
// on (adrift-core's ffi.rs). Touches go in as pointer events in points; a display link asks for a
// frame at the screen's own rate. Nothing else lives here: the library is the app.
import UIKit
import QuartzCore
import AVFoundation
import CoreMedia
import CoreVideo

final class AdriftView: UIView {
    override class var layerClass: AnyClass { CAMetalLayer.self }
    var shell: OpaquePointer?
    var link: CADisplayLink?
    var touchIds: [ObjectIdentifier: UInt64] = [:]
    var nextTouchId: UInt64 = 1

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = UIColor(red: 0.957, green: 0.945, blue: 0.918, alpha: 1)
        (layer as? CAMetalLayer)?.framebufferOnly = true
        (layer as? CAMetalLayer)?.presentsWithTransaction = false
    }
    required init?(coder: NSCoder) { fatalError() }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        guard window != nil else { link?.invalidate(); link = nil; return }
        let scale = window?.screen.nativeScale ?? UIScreen.main.scale
        contentScaleFactor = scale
        (layer as? CAMetalLayer)?.contentsScale = scale
        start()
    }

    func start() {
        guard shell == nil, let metal = layer as? CAMetalLayer else { return }
        let scale = Double(metal.contentsScale)
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
        let assets = Bundle.main.resourcePath! + "/data"
        shell = assets.withCString { adrift_new(Unmanaged.passUnretained(metal).toOpaque(), w, h, scale, $0) }
        if shell == nil { NSLog("adrift: the library did not start"); return }
        adrift_set_reduce_motion(shell, UIAccessibility.isReduceMotionEnabled)
        adrift_set_night(shell, traitCollection.userInterfaceStyle == .dark)
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("adrift.json").path
        docs.withCString { adrift_store(shell, $0) }
        restoreSong()
        NotificationCenter.default.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { adrift_pause(shell) }
        }
        NotificationCenter.default.addObserver(forName: UIApplication.willTerminateNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { adrift_pause(shell) }
        }
        let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
        l.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 120, preferred: 120)
        l.add(to: .main, forMode: .common)
        link = l
        becomeFirstResponder()
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        guard let metal = layer as? CAMetalLayer, let shell else { return }
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        if metal.drawableSize != CGSize(width: CGFloat(w), height: CGFloat(h)) {
            metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
            adrift_resize(shell, w, h, Double(metal.contentsScale))
        }
    }

    let haptic = UIImpactFeedbackGenerator(style: .light)

    @objc func tick(_ link: CADisplayLink) {
        guard let shell else { return }
        adrift_frame(shell, link.targetTimestamp)
        if adrift_take_haptic(shell) { haptic.impactOccurred() }
        if adrift_take_export(shell) { exportTake() }
        let recording = adrift_recording(shell)
        if recording != wasRecording { wasRecording = recording; songFollows(recording) }
        if UIAccessibility.isVoiceOverRunning { refreshPanelElements(link.targetTimestamp) }
    }

    // ---- the song (R12): an audio file opened into Adrift (from Files, or another app's share sheet)
    // plays from its start whenever a take begins and stops when it ends; the export lays it under
    // the clip from the take's first moment, fading over the clip's last beat. Kept between launches.

    var wasRecording = false
    var song: AVAudioPlayer?
    var songURL: URL?

    static var songDir: URL { FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("song", isDirectory: true) }

    /// Takes the file as the song, copying it in; a file that won't play is let go.
    func setSong(_ url: URL) {
        let fm = FileManager.default
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        try? fm.removeItem(at: Self.songDir)
        try? fm.createDirectory(at: Self.songDir, withIntermediateDirectories: true)
        let dest = Self.songDir.appendingPathComponent(url.lastPathComponent)
        do { try fm.copyItem(at: url, to: dest) } catch { NSLog("adrift: song not copied: \(error)"); return }
        loadSong(dest)
    }

    func loadSong(_ url: URL) {
        guard let player = try? AVAudioPlayer(contentsOf: url) else { NSLog("adrift: song won't play"); return }
        player.prepareToPlay()
        song = player
        songURL = url
        if let shell { url.deletingPathExtension().lastPathComponent.withCString { adrift_set_song(shell, $0) } }
    }

    /// The song kept from before, if any.
    func restoreSong() {
        let files = (try? FileManager.default.contentsOfDirectory(at: Self.songDir, includingPropertiesForKeys: nil)) ?? []
        if let first = files.first { loadSong(first) }
    }

    func songFollows(_ recording: Bool) {
        guard let song else { return }
        if recording {
            try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
            try? AVAudioSession.sharedInstance().setActive(true)
            song.currentTime = 0
            song.play()
        } else {
            song.stop()
        }
    }

    /// The written clip with the song laid under it from `start` for `length` seconds, fading over the
    /// last second; calls back with the new file, or the clip as it was if anything fails.
    func withSong(_ clip: URL, start: Double, length: Double, done: @escaping (URL) -> Void) {
        guard let songURL else { done(clip); return }
        let comp = AVMutableComposition()
        let video = AVURLAsset(url: clip)
        let audio = AVURLAsset(url: songURL)
        guard let vt = video.tracks(withMediaType: .video).first,
              let at = audio.tracks(withMediaType: .audio).first,
              let cv = comp.addMutableTrack(withMediaType: .video, preferredTrackID: kCMPersistentTrackID_Invalid),
              let ca = comp.addMutableTrack(withMediaType: .audio, preferredTrackID: kCMPersistentTrackID_Invalid)
        else { done(clip); return }
        let scale: CMTimeScale = 600
        let videoRange = CMTimeRange(start: .zero, duration: video.duration)
        let songLength = min(length, audio.duration.seconds)
        do {
            try cv.insertTimeRange(videoRange, of: vt, at: .zero)
            if songLength > 0 {
                try ca.insertTimeRange(CMTimeRange(start: .zero, duration: CMTime(seconds: songLength, preferredTimescale: scale)), of: at, at: CMTime(seconds: start, preferredTimescale: scale))
            }
        } catch { done(clip); return }
        let mix = AVMutableAudioMix()
        let params = AVMutableAudioMixInputParameters(track: ca)
        let fade = min(1.0, songLength)
        params.setVolumeRamp(fromStartVolume: 1, toEndVolume: 0, timeRange: CMTimeRange(start: CMTime(seconds: start + songLength - fade, preferredTimescale: scale), duration: CMTime(seconds: fade, preferredTimescale: scale)))
        mix.inputParameters = [params]
        let out = clip.deletingPathExtension().appendingPathExtension("song.mp4")
        try? FileManager.default.removeItem(at: out)
        guard let session = AVAssetExportSession(asset: comp, presetName: AVAssetExportPresetHighestQuality) else { done(clip); return }
        session.outputURL = out
        session.outputFileType = .mp4
        session.audioMix = mix
        session.exportAsynchronously {
            DispatchQueue.main.async { done(session.status == .completed ? out : clip) }
        }
    }

    // ---- the panel's marks for VoiceOver (R10): the marks carry no words on screen; each is an
    // element with its lamp's words, and activating it taps the mark

    var panelChecked = 0.0
    var panelKey = ""

    func refreshPanelElements(_ t: Double) {
        guard let shell, t - panelChecked > 0.5 else { return }
        panelChecked = t
        var items: [(CGRect, String)] = []
        let n = adrift_panel_count(shell)
        var rect = [Float](repeating: 0, count: 4)
        var buf = [CChar](repeating: 0, count: 256)
        for i in 0..<n {
            guard adrift_panel_item(shell, i, &rect, &buf, buf.count) else { break }
            items.append((CGRect(x: CGFloat(rect[0]), y: CGFloat(rect[1]), width: CGFloat(rect[2]), height: CGFloat(rect[3])), String(cString: buf)))
        }
        let key = items.map { "\($0.0)\($0.1)" }.joined(separator: "|")
        guard key != panelKey else { return }
        panelKey = key
        accessibilityElements = items.map { item -> PanelElement in
            let e = PanelElement(accessibilityContainer: self)
            e.accessibilityLabel = item.1
            e.accessibilityFrameInContainerSpace = item.0
            e.accessibilityTraits = .button
            e.view = self
            return e
        }
    }

    /// A tap at a point, as a finger would give it (VoiceOver's activation of a panel mark).
    func tap(at p: CGPoint) {
        guard let shell else { return }
        let id = nextTouchId
        nextTouchId += 1
        let t = CACurrentMediaTime()
        adrift_pointer(shell, id, 0, Double(p.x), Double(p.y), t)
        adrift_pointer(shell, id, 2, Double(p.x), Double(p.y), t + 0.05)
    }

    // ---- the studio's export (R9): the newest take replayed offline by the library at 30 fps,
    // written as H.264 by AVAssetWriter, then offered by the share sheet (Save Video, Messages, ...)

    var exporting = false
    let spinner = UIActivityIndicatorView(style: .large)

    func exportTake() {
        guard let shell, !exporting else { return }
        var w: UInt32 = 0, h: UInt32 = 0
        let frames = adrift_export_begin(shell, 30, &w, &h)
        guard frames > 0 else { return }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("Adrift-\(Int(Date().timeIntervalSince1970)).mp4")
        try? FileManager.default.removeItem(at: url)
        guard let writer = try? AVAssetWriter(outputURL: url, fileType: .mp4) else { adrift_export_end(shell); return }
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            AVVideoCodecKey: AVVideoCodecType.h264,
            AVVideoWidthKey: Int(w),
            AVVideoHeightKey: Int(h),
            AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 12_000_000],
        ])
        input.expectsMediaDataInRealTime = false
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(assetWriterInput: input, sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA,
            kCVPixelBufferWidthKey as String: Int(w),
            kCVPixelBufferHeightKey as String: Int(h),
        ])
        writer.add(input)
        guard writer.startWriting() else { adrift_export_end(shell); return }
        writer.startSession(atSourceTime: .zero)
        exporting = true
        link?.isPaused = true
        spinner.center = CGPoint(x: bounds.midX, y: bounds.midY)
        addSubview(spinner)
        spinner.startAnimating()
        var index: Int64 = 0
        func finish() {
            input.markAsFinished()
            writer.finishWriting { [weak self] in
                DispatchQueue.main.async {
                    guard let self, let shell = self.shell else { return }
                    var songStart = 0.0, songLength = 0.0
                    let hasSong = adrift_export_song(shell, &songStart, &songLength)
                    adrift_export_end(shell)
                    self.spinner.stopAnimating()
                    self.spinner.removeFromSuperview()
                    self.exporting = false
                    self.link?.isPaused = false
                    if writer.status == .completed {
                        if hasSong && self.song != nil {
                            self.withSong(url, start: songStart, length: songLength) { self.share($0) }
                        } else {
                            self.share(url)
                        }
                    } else { NSLog("adrift: export failed: \(String(describing: writer.error))") }
                }
            }
        }
        func step() {
            guard let shell = self.shell else { return }
            for _ in 0..<4 {
                guard input.isReadyForMoreMediaData else { break }
                guard let pool = adaptor.pixelBufferPool else { finish(); return }
                var buffer: CVPixelBuffer?
                CVPixelBufferPoolCreatePixelBuffer(nil, pool, &buffer)
                guard let pb = buffer else { finish(); return }
                CVPixelBufferLockBaseAddress(pb, [])
                let base = CVPixelBufferGetBaseAddress(pb)!.assumingMemoryBound(to: UInt8.self)
                let ok = adrift_export_frame(shell, base, CVPixelBufferGetBytesPerRow(pb))
                CVPixelBufferUnlockBaseAddress(pb, [])
                if !ok { finish(); return }
                adaptor.append(pb, withPresentationTime: CMTime(value: index, timescale: 30))
                index += 1
            }
            DispatchQueue.main.async { step() }
        }
        DispatchQueue.main.async { step() }
    }

    func share(_ url: URL) {
        let sheet = UIActivityViewController(activityItems: [url], applicationActivities: nil)
        sheet.popoverPresentationController?.sourceView = self
        sheet.popoverPresentationController?.sourceRect = CGRect(x: bounds.midX, y: bounds.midY, width: 1, height: 1)
        window?.rootViewController?.present(sheet, animated: true)
    }

    override func traitCollectionDidChange(_ previous: UITraitCollection?) {
        super.traitCollectionDidChange(previous)
        if let shell { adrift_set_night(shell, traitCollection.userInterfaceStyle == .dark) }
    }

    override var canBecomeFirstResponder: Bool { true }
    override func motionEnded(_ motion: UIEvent.EventSubtype, with event: UIEvent?) {
        if motion == .motionShake, let shell { adrift_shaken(shell) }
    }

    func send(_ touches: Set<UITouch>, _ phase: UInt32) {
        guard let shell else { return }
        for t in touches {
            let key = ObjectIdentifier(t)
            let id: UInt64
            if let known = touchIds[key] { id = known } else { id = nextTouchId; nextTouchId += 1; touchIds[key] = id }
            let p = t.location(in: self)
            adrift_pointer(shell, id, phase, Double(p.x), Double(p.y), t.timestamp)
            if phase >= 2 { touchIds.removeValue(forKey: key) }
        }
    }
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 0) }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 1) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 2) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 3) }
}

final class AdriftController: UIViewController {
    override func loadView() { view = AdriftView(frame: UIScreen.main.bounds) }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { .all }
}

@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil) -> Bool {
        let w = UIWindow(frame: UIScreen.main.bounds)
        w.rootViewController = AdriftController()
        w.makeKeyAndVisible()
        window = w
        return true
    }

    /// An audio file opened in Adrift becomes the song takes are conducted to (R12).
    func application(_ app: UIApplication, open url: URL, options: [UIApplication.OpenURLOptionsKey: Any] = [:]) -> Bool {
        guard let view = window?.rootViewController?.view as? AdriftView else { return false }
        view.setSong(url)
        return true
    }
}

final class PanelElement: UIAccessibilityElement {
    weak var view: AdriftView?
    override func accessibilityActivate() -> Bool {
        guard let view else { return false }
        let f = accessibilityFrameInContainerSpace
        view.tap(at: CGPoint(x: f.midX, y: f.midY))
        return true
    }
}
