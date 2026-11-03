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
        var picked: UInt32 = 0, takes: UInt32 = 0, chosen: UInt32 = 0, kept: UInt32 = 0
        if adrift_menu(shell, &picked, &takes, &chosen, &kept), !exporting { studioMenu(picked: picked, takes: takes, chosen: chosen, kept: kept) }
        let recording = adrift_recording(shell)
        if recording != wasRecording { wasRecording = recording; songFollows(recording) }
        if UIAccessibility.isVoiceOverRunning { refreshPanelElements(link.targetTimestamp) }
        editor?.follow(day: adrift_day(shell))
    }

    // ---- the edit view (R23): a journey clip's days and speed set in a light viewer over the globe,
    // then exported. Dragging a handle shows the lamps on that day; play runs the clip's days at its
    // speed; Export makes the 60 fps master as set.

    var editor: EditPanel?

    func openEditor() {
        guard let shell, editor == nil, !exporting else { return }
        var born = 0.0, now = 0.0, from = 0.0, to = 0.0
        guard adrift_journey_bounds(shell, 30, &born, &now, &from, &to), adrift_preview_begin(shell) else { return }
        let panel = EditPanel(born: born, now: now, from: from, to: to, basePace: (to - from) / 30)
        panel.onShow = { [weak self] day in if let s = self?.shell { adrift_preview_at(s, day) } }
        panel.onPlay = { [weak self] a, b, pace in if let s = self?.shell { adrift_preview_play(s, a, b, pace) } }
        panel.onClose = { [weak self] in self?.closeEditor() }
        panel.onExport = { [weak self] a, b, secs in
            self?.closeEditor()
            self?.exportTake(journey: secs, from: a, to: b)
        }
        let h: CGFloat = 176
        panel.frame = CGRect(x: 10, y: bounds.maxY - h - max(safeAreaInsets.bottom, 10), width: bounds.width - 20, height: h)
        panel.autoresizingMask = [.flexibleWidth, .flexibleTopMargin]
        addSubview(panel)
        editor = panel
        adrift_preview_at(shell, from)
    }

    func closeEditor() {
        editor?.removeFromSuperview()
        editor = nil
        if let shell { adrift_preview_end(shell) }
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
        // (R21) the video track passes through as written (small, already the clip's bitrate); only if
        // that can't be done is it re-encoded, at HEVC 1080p, never at the highest-quality preset
        // (which re-encoded a 4-minute clip to hundreds of MB)
        func export(_ preset: String, then fallback: String?) {
            try? FileManager.default.removeItem(at: out)
            guard let session = AVAssetExportSession(asset: comp, presetName: preset) else { done(clip); return }
            session.outputURL = out
            session.outputFileType = .mp4
            session.audioMix = mix
            session.exportAsynchronously {
                DispatchQueue.main.async {
                    if session.status == .completed { done(out) } else if let f = fallback { export(f, then: nil) } else { done(clip) }
                }
            }
        }
        export(AVAssetExportPresetPassthrough, then: AVAssetExportPresetHEVC1920x1080)
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
    let progress = UIProgressView(progressViewStyle: .default)

    // ---- the studio's menu (R13): a rest on the record control offers what can be made. A journey
    // clip of the chosen lamps (or every lamp out) at a length picked here, made with no hand in it;
    // the last take again; the lamps copied as text (exact: pasted on any phone they go on the same)
    // or pasted back.

    func studioMenu(picked: UInt32, takes: UInt32, chosen: UInt32, kept: UInt32) {
        let sheet = UIAlertController(title: nil, message: nil, preferredStyle: .actionSheet)
        let lamps = picked == 1 ? "this lamp" : "these \(picked) lamps"
        if picked > 0 {
            sheet.addAction(UIAlertAction(title: "A clip of \(lamps)' journey", style: .default) { [weak self] _ in self?.openEditor() })
        }
        if takes > 0 {
            sheet.addAction(UIAlertAction(title: "The last take again", style: .default) { [weak self] _ in self?.exportTake() })
        }
        // a lamp kept for good stays through a clear-all and every update (R13)
        let these = chosen == 1 ? "this lamp" : "these \(chosen) lamps"
        if chosen > 0 && kept < chosen {
            sheet.addAction(UIAlertAction(title: "Keep \(these)", style: .default) { [weak self] _ in if let s = self?.shell { _ = adrift_keep(s, true) } })
        } else if chosen > 0 {
            sheet.addAction(UIAlertAction(title: "Let \(these) go", style: .default) { [weak self] _ in if let s = self?.shell { _ = adrift_keep(s, false) } })
        }
        if picked > 0 {
            sheet.addAction(UIAlertAction(title: "Copy \(lamps)", style: .default) { [weak self] _ in self?.copyLamps() })
        }
        if UIPasteboard.general.string?.hasPrefix("adrift lamps") == true {
            sheet.addAction(UIAlertAction(title: "Paste lamps", style: .default) { [weak self] _ in self?.pasteLamps() })
        }
        // the preset (R21): what a put makes from now on; things already out keep their own
        if let shell {
            let wind = adrift_medium(shell) == 1
            sheet.addAction(UIAlertAction(title: wind ? "Lamps on the sea" : "Seeds on the wind", style: .default) { [weak self] _ in if let s = self?.shell { adrift_set_medium(s, wind ? 0 : 1) } })
        }
        sheet.addAction(UIAlertAction(title: "Cancel", style: .cancel))
        present(sheet)
    }

    /// The export's frame rate (R22): TikTok's ceiling.
    static let exportFps: Double = 60

    func journeyLengths() {
        let sheet = UIAlertController(title: "How long?", message: nil, preferredStyle: .actionSheet)
        for (title, secs) in [("30 seconds", 30.0), ("1 minute", 60.0), ("2 minutes", 120.0), ("4 minutes", 240.0), ("5 minutes", 300.0)] {
            sheet.addAction(UIAlertAction(title: title, style: .default) { [weak self] _ in self?.exportTake(journey: secs) })
        }
        sheet.addAction(UIAlertAction(title: "Cancel", style: .cancel))
        present(sheet)
    }

    func copyLamps() {
        guard let shell else { return }
        let n = adrift_lamps_text(shell, nil, 0)
        guard n > 0 else { return }
        var buf = [UInt8](repeating: 0, count: n + 1)
        _ = adrift_lamps_text(shell, &buf, buf.count)
        UIPasteboard.general.string = String(decoding: buf[0..<n], as: UTF8.self)
    }

    func pasteLamps() {
        guard let shell, let text = UIPasteboard.general.string else { return }
        _ = text.withCString { adrift_lamps_paste(shell, $0) }
    }

    func present(_ sheet: UIAlertController) {
        // on the iPad it points at the record control (top right)
        sheet.popoverPresentationController?.sourceView = self
        sheet.popoverPresentationController?.sourceRect = CGRect(x: bounds.maxX - 44, y: 40, width: 1, height: 1)
        window?.rootViewController?.present(sheet, animated: true)
    }

    /// Exports the newest take, or (R13) a journey clip of the picked lamps `journey` seconds long.
    func exportTake(journey: Double? = nil, from: Double = .nan, to: Double = .nan) {
        guard let shell, !exporting else { return }
        var w: UInt32 = 0, h: UInt32 = 0
        let frames = journey.map { adrift_journey_begin_edit(shell, $0, from, to, .nan, Self.exportFps, &w, &h) } ?? adrift_export_begin(shell, Self.exportFps, &w, &h)
        guard frames > 0 else { return }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent("Adrift-\(Int(Date().timeIntervalSince1970)).mp4")
        try? FileManager.default.removeItem(at: url)
        guard let writer = try? AVAssetWriter(outputURL: url, fileType: .mp4) else { adrift_export_end(shell); return }
        let input = AVAssetWriterInput(mediaType: .video, outputSettings: [
            // (R22) a master for TikTok: 60 frames a second (its ceiling) and HEVC at 12 Mbit/s, so its
            // re-encode starts from a clean source; about 90 MB a minute, kept as is (the check's
            // ffmpeg encodes alike; tools/adrift/tiktok.sh re-encodes it roughly as TikTok does)
            AVVideoCodecKey: AVVideoCodecType.hevc,
            AVVideoWidthKey: Int(w),
            AVVideoHeightKey: Int(h),
            AVVideoCompressionPropertiesKey: [AVVideoAverageBitRateKey: 12_000_000, AVVideoExpectedSourceFrameRateKey: Int(Self.exportFps), AVVideoMaxKeyFrameIntervalKey: 120],
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
        // a long clip shows how far it has come (R13)
        progress.frame = CGRect(x: bounds.midX - 80, y: bounds.midY + 34, width: 160, height: 4)
        progress.progress = 0
        progress.trackTintColor = UIColor.black.withAlphaComponent(0.08)
        progress.progressTintColor = UIColor(red: 0.72, green: 0.42, blue: 0.32, alpha: 1)
        if frames > 300 { addSubview(progress) }
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
                    self.progress.removeFromSuperview()
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
                adaptor.append(pb, withPresentationTime: CMTime(value: index, timescale: CMTimeScale(Self.exportFps)))
                index += 1
                self.progress.progress = Float(index) / Float(max(frames, 1))
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


/// The edit view's panel (R23): the lamps' days as a line, the clip's first and last days as two
/// handles on it, its speed, play, Export and close. The length follows from the days and the speed.
final class EditPanel: UIVisualEffectView {
    let born: Double, now: Double, basePace: Double
    var from: Double, to: Double
    let lo: Double, hi: Double
    var speed = 1.0
    var onShow: ((Double) -> Void)?
    var onPlay: ((Double, Double, Double) -> Void)?
    var onExport: ((Double, Double, Double) -> Void)?
    var onClose: (() -> Void)?

    let track = UIView(), span = UIView(), head = UIView()
    let startKnob = UIView(), endKnob = UIView(), nowTick = UIView()
    let words = UILabel()
    let speeds = UISegmentedControl(items: ["½×", "1×", "2×", "4×"])
    let play = UIButton(type: .system), export = UIButton(type: .system), close = UIButton(type: .system)

    init(born: Double, now: Double, from: Double, to: Double, basePace: Double) {
        self.born = born; self.now = now; self.from = from; self.to = to
        self.basePace = max(basePace, 0.01)
        lo = born
        hi = max(max(now, to) + max(30, (now - born) * 0.25), from + 1)
        super.init(effect: UIBlurEffect(style: .systemUltraThinMaterial))
        layer.cornerRadius = 22
        clipsToBounds = true
        let ink = UIColor.label
        track.backgroundColor = ink.withAlphaComponent(0.18)
        span.backgroundColor = UIColor(red: 0.96, green: 0.65, blue: 0.17, alpha: 0.9)
        nowTick.backgroundColor = ink.withAlphaComponent(0.45)
        head.backgroundColor = ink
        head.isHidden = true
        for k in [startKnob, endKnob] {
            k.backgroundColor = .white
            k.layer.cornerRadius = 14
            k.layer.shadowColor = UIColor.black.cgColor
            k.layer.shadowOpacity = 0.25
            k.layer.shadowRadius = 3
            k.layer.shadowOffset = CGSize(width: 0, height: 1)
            k.addGestureRecognizer(UIPanGestureRecognizer(target: self, action: #selector(drag(_:))))
        }
        words.font = .monospacedDigitSystemFont(ofSize: 15, weight: .medium)
        words.textColor = ink
        words.textAlignment = .center
        speeds.selectedSegmentIndex = 1
        speeds.addTarget(self, action: #selector(speedChanged), for: .valueChanged)
        play.setImage(UIImage(systemName: "play.fill"), for: .normal)
        play.addTarget(self, action: #selector(playTapped), for: .touchUpInside)
        close.setImage(UIImage(systemName: "xmark"), for: .normal)
        close.addTarget(self, action: #selector(closeTapped), for: .touchUpInside)
        export.setTitle("Export", for: .normal)
        export.titleLabel?.font = .systemFont(ofSize: 17, weight: .semibold)
        export.addTarget(self, action: #selector(exportTapped), for: .touchUpInside)
        for v in [track, nowTick, span, head, startKnob, endKnob, words, speeds, play, export, close] { contentView.addSubview(v) }
        startKnob.accessibilityLabel = "Start"
        endKnob.accessibilityLabel = "End"
        refresh()
    }
    required init?(coder: NSCoder) { fatalError() }

    var pace: Double { basePace * speed }
    var seconds: Double { min(max((to - from) / pace, 3), 600) }

    func x(_ day: Double) -> CGFloat {
        let w = bounds.width - 56
        return 28 + CGFloat((day - lo) / (hi - lo)) * w
    }
    func day(_ x: CGFloat) -> Double {
        let w = max(bounds.width - 56, 1)
        return lo + Double((x - 28) / w) * (hi - lo)
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        let w = bounds.width
        close.frame = CGRect(x: 8, y: 8, width: 40, height: 40)
        play.frame = CGRect(x: w / 2 - 20, y: 8, width: 40, height: 40)
        export.frame = CGRect(x: w - 96, y: 8, width: 88, height: 40)
        words.frame = CGRect(x: 16, y: 54, width: w - 32, height: 22)
        let ty: CGFloat = 104
        track.frame = CGRect(x: 28, y: ty - 1.5, width: w - 56, height: 3)
        speeds.frame = CGRect(x: 28, y: 132, width: w - 56, height: 32)
        refresh()
    }

    func refresh() {
        let ty: CGFloat = 104
        let a = x(from), b = x(to)
        span.frame = CGRect(x: a, y: ty - 2.5, width: max(b - a, 1), height: 5)
        startKnob.frame = CGRect(x: a - 14, y: ty - 14, width: 28, height: 28)
        endKnob.frame = CGRect(x: b - 14, y: ty - 14, width: 28, height: 28)
        nowTick.frame = CGRect(x: x(now) - 1, y: ty - 10, width: 2, height: 20)
        let s = Int(seconds.rounded())
        words.text = "\(Self.date(from)) – \(Self.date(to))   \(s / 60):" + String(format: "%02d", s % 60)
        startKnob.accessibilityValue = Self.date(from)
        endKnob.accessibilityValue = Self.date(to)
    }

    /// A day since 2000-01-01 as the clip's date (day and month: the sea is a typical year's).
    static func date(_ day: Double) -> String {
        let f = DateFormatter()
        f.setLocalizedDateFormatFromTemplate("d MMM")
        f.timeZone = TimeZone(identifier: "UTC")
        return f.string(from: Date(timeIntervalSince1970: 946_684_800 + day * 86_400))
    }

    @objc func drag(_ g: UIPanGestureRecognizer) {
        let d = day(g.location(in: self).x)
        if g.view === startKnob {
            from = min(max(d, lo), to - 1)
            onShow?(from)
        } else {
            to = max(min(d, hi), from + 1)
            onShow?(to)
        }
        head.isHidden = true
        refresh()
    }

    @objc func speedChanged() {
        speed = [0.5, 1, 2, 4][speeds.selectedSegmentIndex]
        refresh()
    }

    @objc func playTapped() {
        head.isHidden = false
        onPlay?(from, to, pace)
    }

    /// The playhead follows the app's date while a preview plays.
    func follow(day: Double) {
        guard !head.isHidden else { return }
        head.frame = CGRect(x: x(day) - 1, y: 104 - 12, width: 2, height: 24)
        if day >= to - 1e-6 { head.isHidden = true }
    }

    @objc func exportTapped() { onExport?(from, to, seconds) }
    @objc func closeTapped() { onClose?() }
}
