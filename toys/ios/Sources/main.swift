// Every toy's device shell (toys/README.md): a UIKit app whose one view is a CAMetalLayer the toy's
// Rust library draws on (toy-kit's C ABI, toy.h). Touches go in as pointer events in points; a
// display link asks for a frame at the screen's rate; the toy's sound, if it has one, is pulled by an
// AVAudioEngine source node on the audio thread. Nothing else lives here: the library is the toy.
import UIKit
import QuartzCore
import AVFoundation
import UniformTypeIdentifiers

final class ToyView: UIView, UIDocumentPickerDelegate {
    override static var layerClass: AnyClass { CAMetalLayer.self }
    var shell: OpaquePointer?
    var synth: OpaquePointer?
    var link: CADisplayLink?
    var engine: AVAudioEngine?
    var touchIds: [ObjectIdentifier: UInt64] = [:]
    var nextTouchId: UInt64 = 1
    let haptic = UIImpactFeedbackGenerator(style: .light)
    /// The info mark's one line, in the system's text (it follows the reader's text size; VoiceOver
    /// reads it), shown while the kit plays the mark's demo (toy-kit's info.rs).
    let caption = UILabel()
    var captionText: String?
    lazy var infoElement = InfoElement(accessibilityContainer: self)
    lazy var rulesElement = RulesElement(accessibilityContainer: self)
    lazy var originsElement = OriginsElement(accessibilityContainer: self)
    /// The toy's own buttons (toy_buttons: Terra's lights), kept by label so VoiceOver's focus stays put.
    var buttonElements: [String: ButtonElement] = [:]
    /// The rules (the rules mark's; toy-kit's info.rs › Rules) in the system's text: the toy's few
    /// rules, then one dimmer line of what is measured and what is drawn, on a dark glass over the live
    /// scene, above the marks. A tap anywhere closes it; it scrolls if the reader's text size needs.
    /// The origins mark's text (where the rules came from) takes the same card; a toy may hang the card
    /// from the top instead (`toy_card_at_top`, asked as it opens).
    /// tools/kit/shell_text.py draws the same for the Linux check's screenshots: change both together.
    let rules = UIVisualEffectView(effect: UIBlurEffect(style: .systemUltraThinMaterialDark))
    let rulesScroll = UIScrollView()
    let rulesLabel = UILabel()
    var rulesText: String?
    var cardTop = false
    /// (TOY_OPEN=rules, origins or info, or several with commas, set by tools/toys/sim_launch.sh: opens
    /// each once, the first TOY_OPEN_AFTER seconds in, the next TOY_OPEN_EVERY seconds after)
    var openAt: [(String, Double)] = []
    /// The toy's quiet line at the top (toy_title: Screw's song), in the system's text.
    let title = UILabel()
    var titleText: String?
    /// The sound file the reader chose (toy_wants_file), kept in the app's own storage
    /// (Application Support/opened/) and handed to the toy again at every launch.
    var picking = false
    /// For a film of the Simulator (tools/toys/sim_film.sh; nothing on a phone sets these):
    /// TOY_RECORD_AUDIO=<name> writes the toy's own sound, as it leaves the engine's mixer, to
    /// Documents/<name> (16-bit WAV), with <name>.start once it begins and, when TOY_RECORD_SECONDS
    /// (120) end it, <name>.clock: the samples rendered against the wall clock, once a frame, so a
    /// screen recording made at the same time lines up, and the scripted touches' wall-clock seconds.
    /// TOY_TOUCHES=<json> plays fingers through the same toy_pointer a UITouch takes, shown by a ring:
    /// [{"at": s, "for": s, "path": [[x, y], ...]}], x and y as fractions of the view, `at` seconds
    /// after the toy starts; a path of one point is a press held for `for`.
    var recorder: AVAudioFile?
    var recording = false
    var script: [(at: Double, dur: Double, path: [CGPoint])] = []
    var scriptDown: [Int: Bool] = [:]
    var scriptStart = 0.0
    var fingerRings: [Int: UIView] = [:]
    /// While recording, a mark for the film's clock: a magenta square at the left edge for a
    /// second, a second into the toy, its wall-clock second written to <name>.mark; the film finds its
    /// first frame and sets the screen recording's clock by it (the Simulator's capture runs behind).
    var markShown: (view: UIView, at: Double)?
    var markDone = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = .black
        (layer as? CAMetalLayer)?.framebufferOnly = true
        registerForTraitChanges([UITraitUserInterfaceStyle.self]) { (self: Self, _: UITraitCollection) in
            if let shell = self.shell { toy_set_night(shell, self.traitCollection.userInterfaceStyle == .dark) }
        }
        caption.font = UIFont.preferredFont(forTextStyle: .callout)
        caption.adjustsFontForContentSizeCategory = true
        caption.adjustsFontSizeToFitWidth = true
        caption.minimumScaleFactor = 0.7
        caption.numberOfLines = 1
        caption.textAlignment = .center
        caption.textColor = UIColor(white: 1.0, alpha: 0.92)
        caption.layer.shadowColor = UIColor.black.cgColor
        caption.layer.shadowOpacity = 0.9
        caption.layer.shadowRadius = 6
        caption.layer.shadowOffset = .zero
        caption.alpha = 0
        caption.isUserInteractionEnabled = false
        addSubview(caption)
        title.font = UIFont.preferredFont(forTextStyle: .footnote)
        title.adjustsFontForContentSizeCategory = true
        title.numberOfLines = 1
        title.lineBreakMode = .byTruncatingTail
        title.textAlignment = .center
        title.textColor = UIColor(white: 1.0, alpha: 0.55)
        title.isUserInteractionEnabled = false
        title.alpha = 0
        addSubview(title)
        rules.layer.cornerRadius = 16
        rules.layer.cornerCurve = .continuous
        rules.clipsToBounds = true
        // (a dark tint on the glass, so it stays dark over paper as over night)
        rules.contentView.backgroundColor = UIColor(white: 0.06, alpha: 0.62)
        rules.alpha = 0
        rules.isHidden = true
        rulesLabel.numberOfLines = 0
        rulesLabel.isAccessibilityElement = true
        rulesScroll.addSubview(rulesLabel)
        rules.contentView.addSubview(rulesScroll)
        rules.addGestureRecognizer(UITapGestureRecognizer(target: self, action: #selector(closeRules)))
        addSubview(rules)
        let env = ProcessInfo.processInfo.environment
        if let what = env["TOY_OPEN"] {
            let first = CACurrentMediaTime() + (Double(env["TOY_OPEN_AFTER"] ?? "") ?? 8), every = Double(env["TOY_OPEN_EVERY"] ?? "") ?? 6
            openAt = what.split(separator: ",").enumerated().map { (String($0.element), first + Double($0.offset) * every) }
        }
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
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
        let assets = Bundle.main.resourcePath! + "/data"
        // the toy's log (a panic's message, a GPU error, a toy that could not start), in the app's
        // Documents, which the Files app shows: On My iPhone › <the toy> › toy-log.txt
        let log = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("toy-log.txt").path
        log.withCString { toy_set_log($0) }
        shell = assets.withCString { toy_new(Unmanaged.passUnretained(metal).toOpaque(), w, h, Double(metal.contentsScale), $0) }
        guard let shell else { NSLog("toy: the library did not start"); return }
        toy_set_reduce_motion(shell, UIAccessibility.isReduceMotionEnabled)
        toy_set_night(shell, traitCollection.userInterfaceStyle == .dark)
        let store = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("toy.json").path
        store.withCString { toy_store(shell, $0) }
        if let kept = keptFile() { open(kept) }
        startSound()
        loadScript()
        let center = NotificationCenter.default
        center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_pause(shell) }
            self?.engine?.pause()
        }
        center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { [weak self] _ in
            try? self?.engine?.start()
        }
        center.addObserver(forName: UIApplication.willTerminateNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_pause(shell) }
        }
        center.addObserver(forName: UIAccessibility.reduceMotionStatusDidChangeNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_set_reduce_motion(shell, UIAccessibility.isReduceMotionEnabled) }
        }
        let l = CADisplayLink(target: self, selector: #selector(tick(_:)))
        l.preferredFrameRateRange = CAFrameRateRange(minimum: 30, maximum: 120, preferred: 120)
        l.add(to: .main, forMode: .common)
        link = l
        becomeFirstResponder()
    }

    /// The toy's sound: an engine whose source node asks the library for samples (the ambient
    /// category: the silent switch silences it, and other apps' audio plays on). The library fills
    /// interleaved stereo into a scratch buffer allocated here, never on the audio thread; it is then
    /// copied into whatever layout the engine hands over (two deinterleaved buffers on a device, or
    /// one interleaved), in chunks no larger than the scratch.
    static let scratchFrames = 4096
    let scratch = UnsafeMutablePointer<Float>.allocate(capacity: ToyView.scratchFrames * 2)
    func startSound() {
        guard let shell, let s = toy_synth(shell) else { return }
        synth = s
        if toy_plays_music(shell) {
            try? AVAudioSession.sharedInstance().setCategory(.playback, mode: .default, options: [])
        } else {
            try? AVAudioSession.sharedInstance().setCategory(.ambient, mode: .default, options: [.mixWithOthers])
        }
        try? AVAudioSession.sharedInstance().setActive(true)
        let engine = AVAudioEngine()
        let hw = engine.outputNode.outputFormat(forBus: 0).sampleRate
        let rate = hw > 0 ? hw : 48000
        guard let format = AVAudioFormat(standardFormatWithSampleRate: rate, channels: 2) else { return }
        let ptr = OpaquePointer(UnsafeMutableRawPointer(s))
        let scratch = self.scratch
        let rendered = self.rendered
        let cap = ToyView.scratchFrames
        let node = AVAudioSourceNode(format: format) { silence, _, frames, list -> OSStatus in
            let buffers = UnsafeMutableAudioBufferListPointer(list)
            let total = Int(frames)
            var done = 0
            while done < total {
                let n = min(cap, total - done)
                toy_audio(ptr, scratch, UInt32(n), 2, rate)
                if buffers.count >= 2 {
                    // deinterleaved: one buffer a channel
                    for c in 0..<2 {
                        guard let d = buffers[c].mData?.assumingMemoryBound(to: Float.self) else { continue }
                        let room = Int(buffers[c].mDataByteSize) / MemoryLayout<Float>.size
                        for i in 0..<n where done + i < room { d[done + i] = scratch[2 * i + c] }
                    }
                } else if buffers.count == 1, let d = buffers[0].mData?.assumingMemoryBound(to: Float.self) {
                    let ch = max(Int(buffers[0].mNumberChannels), 1)
                    let room = Int(buffers[0].mDataByteSize) / MemoryLayout<Float>.size
                    for i in 0..<n {
                        for c in 0..<ch where (done + i) * ch + c < room { d[(done + i) * ch + c] = scratch[2 * i + min(c, 1)] }
                    }
                }
                done += n
            }
            rendered.pointee += Int64(total)
            silence.pointee = false
            return noErr
        }
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: format)
        if let name = ProcessInfo.processInfo.environment["TOY_RECORD_AUDIO"] { record(engine, name) }
        do { try engine.start(); self.engine = engine } catch { NSLog("toy: no sound: \(error)") }
    }

    /// TOY_RECORD_AUDIO (above): a tap on the mixer, written off the audio thread by AVAudioEngine.
    func record(_ engine: AVAudioEngine, _ name: String) {
        let env = ProcessInfo.processInfo.environment
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let mixer = engine.mainMixerNode
        let format = mixer.outputFormat(forBus: 0)
        let settings: [String: Any] = [
            AVFormatIDKey: kAudioFormatLinearPCM, AVSampleRateKey: format.sampleRate,
            AVNumberOfChannelsKey: format.channelCount, AVLinearPCMBitDepthKey: 16,
            AVLinearPCMIsFloatKey: false, AVLinearPCMIsBigEndianKey: false,
        ]
        do {
            recorder = try AVAudioFile(forWriting: docs.appendingPathComponent(name), settings: settings,
                                       commonFormat: format.commonFormat, interleaved: format.isInterleaved)
        } catch { NSLog("toy: can't record the sound: \(error)"); return }
        let limit = AVAudioFramePosition((Double(env["TOY_RECORD_SECONDS"] ?? "") ?? 120) * format.sampleRate)
        recording = true
        var first = true
        var taps = ""
        renderClock = ""
        mixer.installTap(onBus: 0, bufferSize: 4096, format: format) { [weak self] buffer, when in
            guard let self, self.recording, let file = self.recorder else { return }
            // each buffer's place in the file beside the samples rendered so far (the file begins some way
            // into them); tick() logs the samples rendered against the wall clock, and the film lines the
            // file up by both (a buffer's host time here is not when it was rendered: it put the
            // Simulator's sound 0.6 s early)
            taps.append(String(format: "T %lld %.0f %.0f %lld %u\n", file.length, when.sampleTime, format.sampleRate,
                               self.rendered.pointee, buffer.frameLength))
            if first {
                first = false
                let now = String(format: "%.4f\n", Date().timeIntervalSince1970)
                try? now.write(to: docs.appendingPathComponent(name + ".start"), atomically: true, encoding: .utf8)
            }
            try? file.write(from: buffer)
            if file.length >= limit {
                self.recording = false
                let all = taps
                DispatchQueue.main.async {
                    mixer.removeTap(onBus: 0)
                    self.recorder = nil     // (closes the file)
                    try? (all + self.renderClock).write(to: docs.appendingPathComponent(name + ".clock"), atomically: true, encoding: .utf8)
                }
            }
        }
    }

    /// The samples the toy has rendered against the wall clock, once a frame while recording (above).
    /// (In the Simulator the mixer's own timeline stays at 0, so the source node counts them.)
    var renderClock = ""
    let rendered: UnsafeMutablePointer<Int64> = {
        let p = UnsafeMutablePointer<Int64>.allocate(capacity: 1)
        p.initialize(to: 0)
        return p
    }()
    func showMark(_ now: Double) {
        guard let name = ProcessInfo.processInfo.environment["TOY_RECORD_AUDIO"], !markDone, now - scriptStart >= 1 else { return }
        if let shown = markShown {
            if now - shown.at >= 1 { shown.view.removeFromSuperview(); markShown = nil; markDone = true }
            return
        }
        let v = UIView(frame: CGRect(x: 0, y: 70, width: 24, height: 24))     // (clear of the rounded corner)
        v.backgroundColor = UIColor(red: 1, green: 0, blue: 1, alpha: 1)
        v.isUserInteractionEnabled = false
        addSubview(v)
        markShown = (v, now)
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let at = String(format: "%.4f\n", Date().timeIntervalSince1970)
        try? at.write(to: docs.appendingPathComponent(name + ".mark"), atomically: true, encoding: .utf8)
    }

    func logRenderClock() {
        guard recording else { return }
        renderClock.append(String(format: "R %.4f %lld\n", Date().timeIntervalSince1970, rendered.pointee))
    }

    /// TOY_TOUCHES (above): the scripted fingers, moved once a frame.
    func loadScript() {
        guard let json = ProcessInfo.processInfo.environment["TOY_TOUCHES"], let data = json.data(using: .utf8),
              let list = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else { return }
        script = list.compactMap { s in
            guard let at = (s["at"] as? NSNumber)?.doubleValue, let pts = s["path"] as? [[NSNumber]], !pts.isEmpty else { return nil }
            let path = pts.map { CGPoint(x: $0[0].doubleValue, y: $0[1].doubleValue) }
            return (at, (s["for"] as? NSNumber)?.doubleValue ?? 0.1, path)
        }
        scriptStart = CACurrentMediaTime()
    }

    func playScript(_ now: Double) {
        guard let shell, !script.isEmpty else { return }
        let t = now - scriptStart
        for (i, s) in script.enumerated() where t >= s.at && scriptDown[i] != false {
            let f = min(max((t - s.at) / max(s.dur, 0.001), 0), 1)
            let along = f * Double(s.path.count - 1)
            let k = min(Int(along), s.path.count - 1), r = along - Double(k)
            let a = s.path[k], b = s.path[min(k + 1, s.path.count - 1)]
            let p = CGPoint(x: (a.x + (b.x - a.x) * r) * bounds.width, y: (a.y + (b.y - a.y) * r) * bounds.height)
            let id = UInt64(100_000 + i)
            let ring = fingerRings[i] ?? {
                let v = UIView(frame: CGRect(x: 0, y: 0, width: 44, height: 44))
                v.layer.cornerRadius = 22
                v.layer.borderWidth = 2
                v.layer.borderColor = UIColor(white: 1, alpha: 0.7).cgColor
                v.isUserInteractionEnabled = false
                addSubview(v)
                fingerRings[i] = v
                return v
            }()
            ring.center = p
            if scriptDown[i] == nil {
                scriptDown[i] = true
                toy_pointer(shell, id, 0, Double(p.x), Double(p.y), now)
                if recording { renderClock.append(String(format: "D %d %.4f\n", i, Date().timeIntervalSince1970)) }
            } else if f < 1 {
                toy_pointer(shell, id, 1, Double(p.x), Double(p.y), now)
            } else {
                toy_pointer(shell, id, 2, Double(p.x), Double(p.y), now)
                if recording { renderClock.append(String(format: "U %d %.4f\n", i, Date().timeIntervalSince1970)) }
                scriptDown[i] = false
                ring.removeFromSuperview()
            }
        }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        guard let metal = layer as? CAMetalLayer, let shell else { return }
        let w = UInt32(max(bounds.width * metal.contentsScale, 1)), h = UInt32(max(bounds.height * metal.contentsScale, 1))
        if metal.drawableSize != CGSize(width: CGFloat(w), height: CGFloat(h)) {
            metal.drawableSize = CGSize(width: CGFloat(w), height: CGFloat(h))
            toy_resize(shell, w, h, Double(metal.contentsScale))
        }
        if rulesText != nil { layoutRules() }
        layoutTitle()
    }

    @objc func tick(_ link: CADisplayLink) {
        guard let shell else { return }
        playScript(CACurrentMediaTime())
        showMark(CACurrentMediaTime())
        logRenderClock()
        toy_frame(shell, link.targetTimestamp)
        if toy_take_haptic(shell) { haptic.impactOccurred() }
        if toy_wants_file(shell) { pickFile() }
        showTitle(toy_title(shell).map { String(cString: $0) })
        if let o = openAt.first, CACurrentMediaTime() >= o.1 {
            openAt.removeFirst()
            switch o.0 {
            case "rules": toy_rules_open(shell)
            case "origins": toy_origins_open(shell)
            default: toy_info_open(shell, CACurrentMediaTime())
            }
        }
        showCaption(toy_info_line(shell).map { String(cString: $0) })
        showRules((toy_rules_text(shell) ?? toy_origins_text(shell)).map { String(cString: $0) })
    }

    @objc func closeRules() {
        guard let shell, rulesText != nil else { return }
        if toy_origins_text(shell) != nil { toy_origins_open(shell) } else { toy_rules_open(shell) }
    }

    func showRules(_ text: String?) {
        guard text != rulesText else { return }
        rulesText = text
        guard let text else {
            UIView.animate(withDuration: 0.3, animations: { self.rules.alpha = 0 }, completion: { _ in
                if self.rulesText == nil { self.rules.isHidden = true }
            })
            UIAccessibility.post(notification: .layoutChanged, argument: nil)
            return
        }
        let parts = text.components(separatedBy: "\n\n")
        let body = UIFont.preferredFont(forTextStyle: .callout), note = UIFont.preferredFont(forTextStyle: .footnote)
        let lines = NSMutableParagraphStyle()
        lines.paragraphSpacing = 7
        let noteStyle = NSMutableParagraphStyle()
        noteStyle.paragraphSpacingBefore = 6
        let s = NSMutableAttributedString(string: parts[0], attributes: [.font: body, .foregroundColor: UIColor(white: 1, alpha: 0.95), .paragraphStyle: lines])
        if parts.count > 1, !parts[1...].joined().isEmpty {
            let noteAttributes: [NSAttributedString.Key: Any] = [.font: note, .foregroundColor: UIColor(white: 1, alpha: 0.64), .paragraphStyle: noteStyle]
            s.append(NSAttributedString(string: "\n" + parts[1...].joined(separator: "\n"), attributes: noteAttributes))
        }
        rulesLabel.attributedText = s
        rulesLabel.accessibilityLabel = parts.joined(separator: ". ")
        if let shell { cardTop = toy_card_at_top(shell) }
        layoutRules()
        rules.isHidden = false
        UIView.animate(withDuration: 0.25) { self.rules.alpha = 1 }
        UIAccessibility.post(notification: .screenChanged, argument: rulesLabel)
    }

    /// The panel's place: at most 480 points wide, centred, its foot 60 points above the screen's
    /// (clear of the marks), as tall as its text up to the safe area's top; or, for a toy that asks,
    /// hung 8 points below the safe area's top.
    func layoutRules() {
        let w = min(bounds.width - 32, 480), pad = UIEdgeInsets(top: 14, left: 16, bottom: 14, right: 16)
        let text = rulesLabel.sizeThatFits(CGSize(width: w - pad.left - pad.right, height: .greatestFiniteMagnitude))
        let foot = bounds.height - 60
        let h = min(text.height + pad.top + pad.bottom, foot - safeAreaInsets.top - 8)
        rules.frame = CGRect(x: (bounds.width - w) / 2, y: cardTop ? safeAreaInsets.top + 8 : foot - h, width: w, height: h)
        rulesScroll.frame = rules.contentView.bounds.inset(by: pad)
        rulesLabel.frame = CGRect(origin: .zero, size: CGSize(width: w - pad.left - pad.right, height: text.height))
        rulesScroll.contentSize = rulesLabel.frame.size
    }

    func showTitle(_ text: String?) {
        guard text != titleText else { return }
        titleText = text
        title.text = text
        title.accessibilityLabel = text
        layoutTitle()
        UIView.animate(withDuration: 0.6) { self.title.alpha = text == nil ? 0 : 1 }
    }

    func layoutTitle() {
        let w = min(bounds.width - 96, 420)
        title.frame = CGRect(x: (bounds.width - w) / 2, y: safeAreaInsets.top + 14, width: w, height: 24)
    }

    /// Where the chosen file is kept: the app's own storage, not Documents (the Files app shows that).
    var keptDir: URL {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0].appendingPathComponent("opened", isDirectory: true)
    }

    func keptFile() -> URL? {
        (try? FileManager.default.contentsOfDirectory(at: keptDir, includingPropertiesForKeys: nil))?.first
    }

    /// The Files picker, for a sound file of the reader's own.
    func pickFile() {
        guard !picking, let vc = window?.rootViewController else { return }
        picking = true
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: [.audio, .movie], asCopy: true)
        picker.delegate = self
        picker.allowsMultipleSelection = false
        vc.present(picker, animated: true)
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) { picking = false }

    func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL]) {
        picking = false
        guard let url = urls.first else { return }
        let fm = FileManager.default
        try? fm.removeItem(at: keptDir)
        try? fm.createDirectory(at: keptDir, withIntermediateDirectories: true)
        let kept = keptDir.appendingPathComponent(url.lastPathComponent)
        do { try fm.copyItem(at: url, to: kept) } catch { toyLog("could not keep \(url.lastPathComponent): \(error)"); return }
        open(kept)
    }

    /// Decodes a sound file (any format AVAudioFile reads) off the main thread, then hands it to the toy.
    func open(_ url: URL) {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let file = try? AVAudioFile(forReading: url) else {
                DispatchQueue.main.async { self?.toyLog("could not read \(url.lastPathComponent)") }
                return
            }
            let format = file.processingFormat
            guard let buf = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: AVAudioFrameCount(file.length)),
                  (try? file.read(into: buf)) != nil, let ch = buf.floatChannelData else {
                DispatchQueue.main.async { self?.toyLog("could not decode \(url.lastPathComponent)") }
                return
            }
            let name = url.deletingPathExtension().lastPathComponent
            DispatchQueue.main.async {
                guard let shell = self?.shell else { return }
                let right: UnsafePointer<Float>? = format.channelCount > 1 ? UnsafePointer(ch[1]) : nil
                name.withCString { toy_opened_audio(shell, $0, ch[0], right, UInt64(buf.frameLength), format.sampleRate) }
                _ = buf
            }
        }
    }

    func toyLog(_ line: String) {
        NSLog("toy: \(line)")
        let log = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].appendingPathComponent("toy-log.txt")
        if let h = try? FileHandle(forWritingTo: log) {
            h.seekToEndOfFile()
            h.write(Data("\(Int(Date().timeIntervalSince1970)) \(line)\n".utf8))
            try? h.close()
        }
    }

    func showCaption(_ text: String?) {
        guard text != captionText else { return }
        captionText = text
        if let text {
            caption.text = text
            let w = min(bounds.width - 48, 560)
            caption.frame = CGRect(x: (bounds.width - w) / 2, y: bounds.height - safeAreaInsets.bottom - 96, width: w, height: 44)
            UIView.animate(withDuration: 0.25) { self.caption.alpha = 1 }
            UIAccessibility.post(notification: .announcement, argument: text)
        } else {
            UIView.animate(withDuration: 0.35) { self.caption.alpha = 0 }
        }
    }

    /// The marks, for VoiceOver, in their order on screen: "Where the rules come from", "How it works"
    /// (the rules), "About this app" (its demo); and the card's text while it shows.
    override var accessibilityElements: [Any]? {
        get {
            var x = 0.0, y = 0.0
            var out: [Any] = []
            guard let shell else { return [] }
            if rulesText != nil { out.append(rulesLabel) }
            if titleText != nil { title.isAccessibilityElement = true; out.append(title) }
            if let text = toy_buttons(shell).map({ String(cString: $0) }), !text.isEmpty {
                for (i, line) in text.split(separator: "\n").enumerated() {
                    let f = line.split(separator: "\t")
                    guard f.count == 3, let bx = Double(f[1]), let by = Double(f[2]) else { continue }
                    let label = String(f[0])
                    let e = buttonElements[label] ?? ButtonElement(accessibilityContainer: self)
                    buttonElements[label] = e
                    e.accessibilityLabel = label
                    e.index = UInt32(i)
                    e.accessibilityFrameInContainerSpace = CGRect(x: bx - 22, y: by - 22, width: 44, height: 44)
                    out.append(e)
                }
            }
            if toy_origins_spot(shell, &x, &y) {
                originsElement.accessibilityFrameInContainerSpace = CGRect(x: x - 22, y: y - 22, width: 44, height: 44)
                out.append(originsElement)
            }
            if toy_rules_spot(shell, &x, &y) {
                rulesElement.accessibilityFrameInContainerSpace = CGRect(x: x - 22, y: y - 22, width: 44, height: 44)
                out.append(rulesElement)
            }
            if toy_info_spot(shell, &x, &y) {
                infoElement.accessibilityFrameInContainerSpace = CGRect(x: x - 22, y: y - 22, width: 44, height: 44)
                out.append(infoElement)
            }
            return out
        }
        set {}
    }

    override var canBecomeFirstResponder: Bool { true }
    override func motionEnded(_ motion: UIEvent.EventSubtype, with event: UIEvent?) {
        if motion == .motionShake, let shell { toy_shaken(shell) }
    }

    func send(_ touches: Set<UITouch>, _ phase: UInt32) {
        guard let shell else { return }
        for t in touches {
            let key = ObjectIdentifier(t)
            let id: UInt64
            if let known = touchIds[key] { id = known } else { id = nextTouchId; nextTouchId += 1; touchIds[key] = id }
            let p = t.location(in: self)
            toy_pointer(shell, id, phase, Double(p.x), Double(p.y), t.timestamp)
            if phase >= 2 { touchIds.removeValue(forKey: key) }
        }
    }
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 0) }
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 1) }
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 2) }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 3) }
}

final class InfoElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "About this app"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_info_open(shell, CACurrentMediaTime())
        return true
    }
}

/// One of the toy's own buttons (toy_buttons), pressed by its index in this frame's list.
final class ButtonElement: UIAccessibilityElement {
    var index: UInt32 = 0
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_button_press(shell, index, CACurrentMediaTime())
        UIAccessibility.post(notification: .layoutChanged, argument: nil)
        return true
    }
}

final class RulesElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "How it works"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_rules_open(shell)
        return true
    }
}

final class OriginsElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "Where the rules come from"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView, let shell = view.shell else { return false }
        toy_origins_open(shell)
        return true
    }
}

final class ToyController: UIViewController {
    override func loadView() { view = ToyView(frame: UIScreen.main.bounds) }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { .all }
}

// The scene lifecycle (an app built against the iOS 27 SDK must adopt it, or UIKit stops it at launch):
// one scene (Info.plist's UIApplicationSceneManifest), its window made by SceneDelegate.
@main
final class AppDelegate: UIResponder, UIApplicationDelegate {
    func application(_ application: UIApplication, configurationForConnecting connectingSceneSession: UISceneSession,
                     options: UIScene.ConnectionOptions) -> UISceneConfiguration {
        let config = UISceneConfiguration(name: nil, sessionRole: connectingSceneSession.role)
        config.delegateClass = SceneDelegate.self
        return config
    }
}

final class SceneDelegate: UIResponder, UIWindowSceneDelegate {
    var window: UIWindow?
    func scene(_ scene: UIScene, willConnectTo session: UISceneSession, options connectionOptions: UIScene.ConnectionOptions) {
        guard let scene = scene as? UIWindowScene else { return }
        let w = UIWindow(windowScene: scene)
        w.rootViewController = ToyController()
        w.makeKeyAndVisible()
        window = w
    }
}
