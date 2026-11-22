// The iPhone's frame (toys/README.md): a UIKit app whose one view is a CAMetalLayer the toy's Rust library
// draws on (toy-kit's one frame core, frame.rs, through the C ABI in toy.h). Touches go in as pointer events in
// points; a display link asks for a frame at the screen's rate; the toy's sound, if it has one, is pulled by an
// AVAudioEngine source node on the audio thread. Around it, the chrome the kit describes as data (Chrome.swift),
// in UIKit's own controls; the senses the toy asks for, and no others.
import UIKit
import QuartzCore
import AVFoundation
import CoreMotion
import UniformTypeIdentifiers

final class ToyView: UIView, UIDocumentPickerDelegate {
    override static var layerClass: AnyClass { CAMetalLayer.self }
    var shell: OpaquePointer?
    var synth: OpaquePointer?
    var link: CADisplayLink?
    var engine: AVAudioEngine?
    /// The phone's own motion, for shakes (toy-kit's shake.rs): iOS's shake event comes only while Shake to
    /// Undo is on (Settings › Accessibility › Touch), so the motion is read here as well. No permission:
    /// Core Motion's accelerometer needs none.
    let motion = CMMotionManager()
    var touchIds: [ObjectIdentifier: UInt64] = [:]
    var nextTouchId: UInt64 = 1
    /// The haptic intents (toy-kit's `Haptic`), lightest to heaviest.
    let haptic = UIImpactFeedbackGenerator(style: .light)
    let softHaptic = UIImpactFeedbackGenerator(style: .soft)
    let firmHaptic = UIImpactFeedbackGenerator(style: .medium)
    let thudHaptic = UIImpactFeedbackGenerator(style: .heavy)
    /// The chrome as the toy last described it (toy-kit's chrome.rs).
    var chrome: Chrome?
    /// Whether the device's motion runs with the compass (the magnetometer) on.
    var motionCompass = false
    /// The info mark's one line, in the system's text (it follows the reader's text size; VoiceOver
    /// reads it), shown while the kit plays the mark's demo (toy-kit's info.rs), on a small dark glass
    /// so it reads over a bright scene (frost, snow, paper) as over night.
    let caption = UILabel()
    let captionGlass = UIVisualEffectView(effect: UIBlurEffect(style: .systemUltraThinMaterialDark))
    var captionText: String?
    lazy var infoElement = InfoElement(accessibilityContainer: self)
    /// The toy's own buttons (toy_buttons: Terra's lights), kept by label so VoiceOver's focus stays put.
    var buttonElements: [String: ButtonElement] = [:]
    /// The sheet (toy-kit's info.rs: the one mark's), in the system's text on a dark glass over the live
    /// scene, above the mark: the toy's few rules (and, if it gives one, a dimmer note); under them the rules
    /// as switches, a row each (switches.rs: they stay as the reader leaves them); under a hairline, its
    /// buttons ("Show me", "Where this comes from", which takes the sheet's place). A tap off the sheet closes
    /// it (the kit's), a tap on it does nothing but its controls; it scrolls if the reader's text size needs.
    /// A toy may hang it from the top instead (`toy_card_at_top`, asked as it opens).
    /// tools/kit/shell_text.py draws the same for the Linux check's screenshots: change both together.
    let rules = PanelGlass(effect: UIBlurEffect(style: .systemUltraThinMaterialDark))
    let rulesScroll = UIScrollView()
    let rulesLabel = UILabel()
    var rulesText: String?
    var cardTop = false
    var switchCells: [(UILabel, UISwitch)] = []
    var switchesText: String?
    let hairline = UIView()
    var moreButtons: [UIButton] = []
    var moreText: String?
    /// Words the toy places itself (toy_label: Terra's postcard), in the system's text in the toy's box.
    let label = UILabel()
    var labelNow: Chrome.Label?
    /// (TOY_OPEN=rules, origins, switches or info, or several with commas, set by tools/toys/sim_launch.sh: opens
    /// each once, the first TOY_OPEN_AFTER seconds in, the next TOY_OPEN_EVERY seconds after)
    var openAt: [(String, Double)] = []
    /// The toy's quiet line at the top (toy_title: Screw's song), in the system's text.
    let title = UILabel()
    var titleText: String?
    /// The sound file the reader chose (the toy's "file" event), kept in the app's own storage
    /// (Application Support/opened/) and handed to the toy again at every launch.
    var picking = false

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
        caption.numberOfLines = 2
        caption.textAlignment = .center
        caption.textColor = UIColor(white: 1.0, alpha: 0.95)
        captionGlass.layer.cornerRadius = 14
        captionGlass.layer.cornerCurve = .continuous
        captionGlass.clipsToBounds = true
        captionGlass.contentView.backgroundColor = UIColor(white: 0.06, alpha: 0.62)
        captionGlass.contentView.addSubview(caption)
        captionGlass.alpha = 0
        captionGlass.isUserInteractionEnabled = false
        addSubview(captionGlass)
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
        hairline.backgroundColor = UIColor(white: 1, alpha: 0.16)
        rulesScroll.addSubview(hairline)
        rules.contentView.addSubview(rulesScroll)
        addSubview(rules)
        label.numberOfLines = 0
        label.isUserInteractionEnabled = false
        label.alpha = 0
        addSubview(label)
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
        if let c = readChrome() { apply(c) }
        startSound()
        let center = NotificationCenter.default
        center.addObserver(forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main) { [weak self] _ in
            if let shell = self?.shell { toy_pause(shell) }
            self?.engine?.pause()
            self?.motion.stopDeviceMotionUpdates()
        }
        center.addObserver(forName: UIApplication.willEnterForegroundNotification, object: nil, queue: .main) { [weak self] _ in
            try? self?.engine?.start()
            self?.startMotion()
        }
        center.addObserver(forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main) { [weak self] _ in
            self?.becomeFirstResponder()
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
        startMotion()
        becomeFirstResponder()
    }

    /// The device's own acceleration (gravity out, in g) to the library at 60 Hz, on the main queue as
    /// every other call is; a few hard jolts there are a shake.
    /// With them, what the phone feels (toy-kit's `Sensors`): which way is down, and, where the compass
    /// is, the magnetic field through it (calibrated, in µT) and the heading, for the toys that tilt or
    /// follow north (The Delta, Magnet Marsh). None of it asks permission. Only while the toy asks for a
    /// sense (the chrome's `senses`), and the compass only while it asks for the field or the heading.
    func startMotion() {
        guard let want = chrome?.senses, want.any, motion.isDeviceMotionAvailable else {
            motion.stopDeviceMotionUpdates()
            return
        }
        if motion.isDeviceMotionActive {
            if want.compass == motionCompass { return }
            motion.stopDeviceMotionUpdates()
        }
        motionCompass = want.compass
        motion.deviceMotionUpdateInterval = 1.0 / 60.0
        let frames = CMMotionManager.availableAttitudeReferenceFrames()
        let handler: CMDeviceMotionHandler = { [weak self] m, _ in
            guard let m, let shell = self?.shell else { return }
            let a = m.userAcceleration
            toy_motion(shell, a.x, a.y, a.z, m.timestamp)
            // the device's axes turned to the screen's when the interface is turned (landscape)
            let turn: Double
            switch self?.window?.windowScene?.interfaceOrientation {
            case .landscapeRight: turn = 90
            case .landscapeLeft: turn = -90
            case .portraitUpsideDown: turn = 180
            default: turn = 0
            }
            let (c, s) = (cos(turn * .pi / 180), sin(turn * .pi / 180))
            let g = m.gravity
            let b = m.magneticField
            let known = b.accuracy != .uncalibrated && (b.field.x != 0 || b.field.y != 0 || b.field.z != 0)
            let nan = Double.nan
            // the heading can hold while the field's calibration comes and goes
            let h = m.heading >= 0 ? (m.heading + turn + 360).truncatingRemainder(dividingBy: 360) : nan
            toy_sense(shell, g.x * c - g.y * s, g.x * s + g.y * c, g.z,
                      known ? b.field.x * c - b.field.y * s : nan,
                      known ? b.field.x * s + b.field.y * c : nan,
                      known ? b.field.z : nan, h)
        }
        if want.compass && frames.contains(.xMagneticNorthZVertical) {
            motion.startDeviceMotionUpdates(using: .xMagneticNorthZVertical, to: .main, withHandler: handler)
        } else {
            motion.startDeviceMotionUpdates(to: .main, withHandler: handler)
        }
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
        if chrome?.music == true {
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
            silence.pointee = false
            return noErr
        }
        engine.attach(node)
        engine.connect(node, to: engine.mainMixerNode, format: format)
        do { try engine.start(); self.engine = engine } catch { NSLog("toy: no sound: \(error)") }
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
        toy_frame(shell, link.targetTimestamp)
        takeEvents()
        if let o = openAt.first, CACurrentMediaTime() >= o.1 {
            openAt.removeFirst()
            switch o.0 {
            case "rules", "switches": tell("rules")
            case "origins": tell("origins")
            default: tell(["demo": ["t": CACurrentMediaTime()]])
            }
        }
        if let c = readChrome() { apply(c) }
    }

    /// The chrome, changed: each part shown as it now is.
    func apply(_ c: Chrome) {
        let senses = chrome?.senses
        chrome = c
        showTitle(c.title)
        showCaption(c.caption)
        let rules = c.sheet.flatMap { $0.page == "rules" ? $0 : nil }
        let switches = rules.flatMap { r in
            r.switches.isEmpty ? nil : r.switches.map { "\($0.label)\t\($0.on ? 1 : 0)\n" }.joined()
        }
        let more = rules.map { r in (r.demo ? "demo\n" : "") + (r.origins ? "origins\n" : "") }
        showRules(c.sheet.map(ToyView.sheetText), switches, more)
        showLabel(c.label)
        if senses != c.senses { startMotion() }
    }

    /// The toy's own words (its box in points, alpha and ink; a heading, a quieter line, plain lines), or none.
    func showLabel(_ l: Chrome.Label?) {
        guard l != labelNow else { return }
        let sameWords = l?.lines == labelNow?.lines
        labelNow = l
        guard let l, l.at.count == 4 else {
            label.alpha = 0
            label.isAccessibilityElement = false
            return
        }
        if !sameWords {
            let ink = l.dark ? UIColor(white: 0.11, alpha: 1) : UIColor.white
            let fonts = [UIFont.preferredFont(forTextStyle: .title3).withWeight(.semibold),
                         UIFont.preferredFont(forTextStyle: .subheadline), UIFont.preferredFont(forTextStyle: .subheadline)]
            let alphas: [CGFloat] = [1, 0.6, 0.85]
            let s = NSMutableAttributedString()
            for (i, line) in l.lines.enumerated() {
                let j = min(i, 2)
                let p = NSMutableParagraphStyle()
                p.paragraphSpacingBefore = i == 1 ? 2 : 0
                s.append(NSAttributedString(string: (i > 0 ? "\n" : "") + line,
                                            attributes: [.font: fonts[j], .foregroundColor: ink.withAlphaComponent(alphas[j]), .paragraphStyle: p]))
            }
            label.attributedText = s
            label.accessibilityLabel = l.lines.joined(separator: ". ")
        }
        label.isAccessibilityElement = l.alpha > 0.5
        label.frame = CGRect(x: l.at[0], y: l.at[1], width: l.at[2], height: l.at[3])
        label.sizeToFit()
        label.frame.size.width = l.at[2]
        label.alpha = l.alpha
    }

    /// The switches (lines of label, a tab, 1 or 0) under the rules, made or updated.
    func setSwitches(_ text: String?, animated: Bool) {
        let rows = (text ?? "").split(separator: "\n").map { $0.split(separator: "\t", omittingEmptySubsequences: false) }.filter { $0.count == 2 }
        if rows.count != switchCells.count {
            for (label, toggle) in switchCells { label.removeFromSuperview(); toggle.removeFromSuperview() }
            switchCells = (0..<rows.count).map { i in
                let label = UILabel()
                label.font = UIFont.preferredFont(forTextStyle: .callout)
                label.adjustsFontForContentSizeCategory = true
                label.numberOfLines = 1
                label.textColor = .white
                label.isAccessibilityElement = false
                let toggle = UISwitch()
                toggle.tag = i
                toggle.addTarget(self, action: #selector(switchChanged(_:)), for: .valueChanged)
                rulesScroll.addSubview(label)
                rulesScroll.addSubview(toggle)
                return (label, toggle)
            }
        }
        for (i, row) in rows.enumerated() {
            let (label, toggle) = switchCells[i]
            let on = row[1] == "1"
            label.text = String(row[0])
            label.alpha = on ? 0.95 : 0.55
            toggle.accessibilityLabel = label.text
            if toggle.isOn != on { toggle.setOn(on, animated: animated) }
        }
    }

    @objc func switchChanged(_ toggle: UISwitch) {
        tell(["switch": ["i": toggle.tag, "on": toggle.isOn]])
    }

    /// The sheet's buttons ("demo", "origins", a line each), made or updated.
    func setMore(_ text: String?) {
        guard text != moreText else { return }
        moreText = text
        for b in moreButtons { b.removeFromSuperview() }
        let titles = ["demo": "Show me", "origins": "Where this comes from"]
        moreButtons = (text ?? "").split(separator: "\n").compactMap { what in
            guard let title = titles[String(what)] else { return nil }
            let b = UIButton(type: .system)
            b.setTitle(title, for: .normal)
            b.titleLabel?.font = UIFont.preferredFont(forTextStyle: .callout)
            b.titleLabel?.adjustsFontForContentSizeCategory = true
            b.tintColor = UIColor(red: 0.49, green: 0.75, blue: 1.0, alpha: 1)
            b.accessibilityIdentifier = String(what)
            b.addTarget(self, action: #selector(morePressed(_:)), for: .touchUpInside)
            rulesScroll.addSubview(b)
            return b
        }
        hairline.isHidden = moreButtons.isEmpty
    }

    @objc func morePressed(_ b: UIButton) {
        if b.accessibilityIdentifier == "demo" { tell(["demo": ["t": CACurrentMediaTime()]]) } else { tell("origins") }
    }

    /// The sheet: the rules (or the origins in their place), the switches and the buttons; nil hides it.
    func showRules(_ text: String?, _ switches: String?, _ more: String?) {
        let opening = rulesText == nil
        let changed = text != rulesText || switches != switchesText || more != moreText
        guard changed else { return }
        let textChanged = text != rulesText
        rulesText = text
        switchesText = text == nil ? nil : switches
        guard let text else {
            setMore(nil)
            UIView.animate(withDuration: 0.3, animations: { self.rules.alpha = 0 }, completion: { _ in
                if self.rulesText == nil { self.rules.isHidden = true }
            })
            UIAccessibility.post(notification: .layoutChanged, argument: nil)
            return
        }
        setSwitches(switchesText, animated: !opening)
        setMore(more)
        if textChanged {
            let parts = text.components(separatedBy: "\n\n")
            let body = UIFont.preferredFont(forTextStyle: .callout), note = UIFont.preferredFont(forTextStyle: .footnote)
            let lines = NSMutableParagraphStyle()
            lines.paragraphSpacing = 7
            let noteStyle = NSMutableParagraphStyle()
            noteStyle.paragraphSpacingBefore = 6
            let bodyAttributes: [NSAttributedString.Key: Any] = [.font: body, .foregroundColor: UIColor(white: 1, alpha: 0.95), .paragraphStyle: lines]
            let s = NSMutableAttributedString(string: parts[0], attributes: bodyAttributes)
            if parts.count > 1, !parts[1...].joined().isEmpty {
                let noteAttributes: [NSAttributedString.Key: Any] = [.font: note, .foregroundColor: UIColor(white: 1, alpha: 0.64), .paragraphStyle: noteStyle]
                s.append(NSAttributedString(string: "\n" + parts[1...].joined(separator: "\n"), attributes: noteAttributes))
            }
            rulesLabel.attributedText = s
            rulesLabel.accessibilityLabel = parts.joined(separator: ". ")
            rulesLabel.isHidden = text.isEmpty
        }
        if opening { cardTop = chrome?.sheet?.top ?? false }
        layoutRules()
        guard opening else { return }
        rules.isHidden = false
        UIView.animate(withDuration: 0.25) { self.rules.alpha = 1 }
        var first: UIView? = rulesLabel
        if text.isEmpty {
            first = (switchCells.first?.1 as UIView?) ?? (moreButtons.first as UIView?)
        }
        UIAccessibility.post(notification: .screenChanged, argument: first)
    }

    /// The sheet's place: at most 480 points wide, centred, its foot 60 points above the screen's (clear of
    /// the mark), as tall as it needs up to the safe area's top; or, for a toy that asks, hung 8 points below
    /// the safe area's top. Inside: the text, then a row a switch (its label left, the switch right), then a
    /// hairline and the buttons in a row.
    func layoutRules() {
        let w = min(bounds.width - 32, 480), pad = UIEdgeInsets(top: 14, left: 16, bottom: 14, right: 16)
        let inner = w - pad.left - pad.right
        let text = rulesLabel.isHidden ? .zero : rulesLabel.sizeThatFits(CGSize(width: inner, height: .greatestFiniteMagnitude))
        rulesLabel.frame = CGRect(origin: .zero, size: CGSize(width: inner, height: text.height))
        var y = text.height
        if !switchCells.isEmpty {
            y += text.height > 0 ? 6 : 0
            let row = max(38, ceil(switchCells[0].0.font.lineHeight) + 16)
            for (label, toggle) in switchCells {
                toggle.center = CGPoint(x: inner - toggle.frame.width / 2, y: y + row / 2)
                label.frame = CGRect(x: 0, y: y, width: inner - toggle.frame.width - 8, height: row)
                y += row
            }
        }
        if !moreButtons.isEmpty {
            y += 6
            hairline.frame = CGRect(x: 0, y: y, width: inner, height: 1 / max(contentScaleFactor, 1))
            y += 6
            var x: CGFloat = 0, rowH: CGFloat = 34
            for b in moreButtons {
                let size = b.sizeThatFits(CGSize(width: inner, height: 60))
                if x > 0 && x + size.width > inner { x = 0; y += rowH }
                rowH = max(34, size.height)
                b.frame = CGRect(x: x, y: y, width: size.width, height: rowH)
                x += size.width + 24
            }
            y += rowH
        }
        let foot = bounds.height - 60
        let h = min(y + pad.top + pad.bottom, foot - safeAreaInsets.top - 8)
        rules.frame = CGRect(x: (bounds.width - w) / 2, y: cardTop ? safeAreaInsets.top + 8 : foot - h, width: w, height: h)
        rulesScroll.frame = rules.contentView.bounds.inset(by: pad)
        rulesScroll.contentSize = CGSize(width: inner, height: y)
    }

    /// VoiceOver's two-finger scrub closes the sheet.
    override func accessibilityPerformEscape() -> Bool {
        guard rulesText != nil else { return false }
        tell("close")
        return true
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
            // as wide as the line needs (two lines at most), centred where the line always was; a line too
            // long for one breaks after its first sentence, not wherever the width runs out
            let pad = UIEdgeInsets(top: 8, left: 14, bottom: 8, right: 14), most = min(bounds.width - 48, 560)
            caption.text = text
            if caption.sizeThatFits(CGSize(width: CGFloat.greatestFiniteMagnitude, height: 100)).width > most - pad.left - pad.right,
               let r = text.range(of: ". ") {
                caption.text = text.replacingCharacters(in: r, with: ".\n")
            }
            let fit = caption.sizeThatFits(CGSize(width: most - pad.left - pad.right, height: .greatestFiniteMagnitude))
            let w = min(ceil(fit.width), most - pad.left - pad.right), h = ceil(fit.height)
            let mid = bounds.height - safeAreaInsets.bottom - 74
            captionGlass.frame = CGRect(x: (bounds.width - w) / 2 - pad.left, y: mid - h / 2 - pad.top,
                                        width: w + pad.left + pad.right, height: h + pad.top + pad.bottom)
            caption.frame = CGRect(x: pad.left, y: pad.top, width: w, height: h)
            UIView.animate(withDuration: 0.25) { self.captionGlass.alpha = 1 }
            UIAccessibility.post(notification: .announcement, argument: text)
        } else {
            UIView.animate(withDuration: 0.35) { self.captionGlass.alpha = 0 }
        }
    }

    /// For VoiceOver: the sheet while it shows (its text, switches and buttons), the title, the toy's label and
    /// buttons, and the mark ("About this toy": it opens the sheet).
    override var accessibilityElements: [Any]? {
        get {
            var out: [Any] = []
            guard shell != nil, let chrome else { return [] }
            if rulesText != nil {
                if !rulesLabel.isHidden { out.append(rulesLabel) }
                out.append(contentsOf: switchCells.map { $0.1 })
                out.append(contentsOf: moreButtons)
            }
            if titleText != nil { title.isAccessibilityElement = true; out.append(title) }
            if label.isAccessibilityElement { out.append(label) }
            for (i, b) in chrome.buttons.enumerated() where b.at.count == 2 {
                let e = buttonElements[b.label] ?? ButtonElement(accessibilityContainer: self)
                buttonElements[b.label] = e
                e.accessibilityLabel = b.label
                e.index = i
                e.accessibilityFrameInContainerSpace = CGRect(x: b.at[0] - 22, y: b.at[1] - 22, width: 44, height: 44)
                out.append(e)
            }
            if let at = chrome.mark.at, at.count == 2 {
                infoElement.accessibilityFrameInContainerSpace = CGRect(x: at[0] - 22, y: at[1] - 22, width: 44, height: 44)
                out.append(infoElement)
            }
            return out
        }
        set {}
    }

    override var canBecomeFirstResponder: Bool { true }
    override func motionEnded(_ motion: UIEvent.EventSubtype, with event: UIEvent?) {
        if motion == .motionShake, let shell { toy_shaken(shell, event?.timestamp ?? CACurrentMediaTime()) }
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
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        send(touches, 2)
        // (the sheet's switches and the file picker can take the first responder: the system's shake needs it)
        if !isFirstResponder { becomeFirstResponder() }
    }
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) { send(touches, 3) }
}

final class InfoElement: UIAccessibilityElement {
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityLabel = "About this toy"
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView else { return false }
        view.tell(["mark": ["t": CACurrentMediaTime()]])
        return true
    }
}

/// One of the toy's own buttons (toy_buttons), pressed by its index in this frame's list.
final class ButtonElement: UIAccessibilityElement {
    var index = 0
    override init(accessibilityContainer container: Any) {
        super.init(accessibilityContainer: container)
        accessibilityTraits = .button
    }
    override func accessibilityActivate() -> Bool {
        guard let view = accessibilityContainer as? ToyView else { return false }
        view.tell(["press": ["i": index, "t": CACurrentMediaTime()]])
        UIAccessibility.post(notification: .layoutChanged, argument: nil)
        return true
    }
}

/// The sheet's glass: it keeps the touches that land on it (between its switches) from the scene below.
final class PanelGlass: UIVisualEffectView {
    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {}
    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {}
    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {}
    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {}
}

final class ToyController: UIViewController {
    override func loadView() { view = ToyView(frame: UIScreen.main.bounds) }
    override var prefersStatusBarHidden: Bool { true }
    override var prefersHomeIndicatorAutoHidden: Bool { true }
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { .all }
    // the view asks for the first responder as it reaches the window, before the window is key, which
    // UIKit can refuse: ask again once it shows (the system's shake goes to the first responder)
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        view.becomeFirstResponder()
    }
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

extension UIFont {
    /// The same text style at another weight (title 3, semibold: a postcard's heading).
    func withWeight(_ weight: UIFont.Weight) -> UIFont {
        let d = fontDescriptor.addingAttributes([.traits: [UIFontDescriptor.TraitKey.weight: weight]])
        return UIFont(descriptor: d, size: 0)
    }
}
