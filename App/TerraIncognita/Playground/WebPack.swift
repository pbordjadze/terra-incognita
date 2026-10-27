import Foundation
import SwiftUI
import UIKit
import WebKit

// A web pack's page in a web view: as its own screen over the paused globe (`WebPackScreen`), or
// transparent over the live globe (`WebPackOverlay`). Both are the same `PackWebController`:
//   - a non-persistent web view that can load only this pack's own files, from terra-pack://<id>/…
//     (`PackSchemeHandler`); every http(s) and ws(s) request is blocked by a content rule and the
//     navigation policy, and a Content-Security-Policy says the same from the page's side;
//   - terra.js (Prototypes/_runtime/terra.js) injected before the page's scripts, and the bridge it
//     talks to (message handler "terra"); nothing else of the app is reachable from the page.
// docs/playground.md is the contract.

/// What a pack's page asks of the host.
enum WebPackEvent {
    case close, open([String: String]), flyTo(GlobeCamera, duration: Double), log(String)
}

// MARK: - The screen

/// A pack's page as its own screen over the paused globe.
struct WebPackScreen: View {
    private let pack: WebPack
    private let params: [String: String]
    private let background: PackBackground
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var scenePhase
    @State private var failure: String?

    init(pack: WebPack, params: [String: String] = [:]) {
        self.pack = pack
        self.params = params
        self.background = PackBackground(hex: pack.manifest.background)
    }

    var body: some View {
        ZStack {
            background.color.ignoresSafeArea()
            if let failure {
                failureView(failure)
            } else {
                PackWebView(pack: pack, params: params, active: scenePhase == .active,
                            onClose: { dismiss() }, onFailure: { failure = $0 })
                    .ignoresSafeArea()
            }
        }
        .overlay(alignment: .topLeading) {
            // (terra.js documents the top-left 56 pt as the app's: no page can trap the reader)
            if failure == nil {
                Button { dismiss() } label: {
                    Image(systemName: "chevron.backward").font(.system(size: 20)).frame(width: 44, height: 44)
                }
                .buttonStyle(.glass)
                .accessibilityLabel("Back to the globe")
                .accessibilityIdentifier("packBack")
                .padding(.leading, 16)
                .padding(.top, 8)
            }
        }
        .statusBarHidden()
        .persistentSystemOverlays(.hidden)
        .preferredColorScheme(background.isDark ? .dark : .light)
    }

    private func failureView(_ reason: String) -> some View {
        VStack(spacing: 16) {
            Text("This prototype could not open.")
                .font(.title2.weight(.semibold))
                .multilineTextAlignment(.center)
            Text(reason)
                .font(.body)
                .multilineTextAlignment(.center)
            Button { dismiss() } label: {
                Label("Back to the globe", systemImage: "chevron.backward")
                    .frame(minHeight: 44)
                    .padding(.horizontal, 12)
            }
            .buttonStyle(.glass)
            .accessibilityIdentifier("packBack")
        }
        .foregroundStyle(background.ink)
        .padding(24)
        .frame(maxWidth: 560)
    }
}

/// The screen's colour while the page loads (and behind a failure): the manifest's `background`.
private struct PackBackground {
    let red: Double
    let green: Double
    let blue: Double

    /// "#RRGGBB" or "#RGB"; anything else is the default paper, #F6F2E7.
    init(hex: String?) {
        var text = (hex ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if text.hasPrefix("#") { text.removeFirst() }
        if text.count == 3 { text = text.map { "\($0)\($0)" }.joined() }
        if text.count == 6, let value = UInt32(text, radix: 16) {
            red = Double((value >> 16) & 255) / 255
            green = Double((value >> 8) & 255) / 255
            blue = Double(value & 255) / 255
        } else {
            red = 246.0 / 255
            green = 242.0 / 255
            blue = 231.0 / 255
        }
    }

    var color: Color { Color(red: red, green: green, blue: blue) }
    var isDark: Bool { 0.299 * red + 0.587 * green + 0.114 * blue < 0.5 }
    var ink: Color { isDark ? Color(white: 0.95) : Color(red: 0.13, green: 0.12, blue: 0.10) }
}

private struct PackWebView: UIViewRepresentable {
    let pack: WebPack
    let params: [String: String]
    let active: Bool
    let onClose: () -> Void
    let onFailure: (String) -> Void

    func makeCoordinator() -> Coordinator {
        Coordinator(pack: pack, params: params, active: active, onClose: onClose, onFailure: onFailure)
    }

    func makeUIView(context: Context) -> WKWebView {
        let controller = context.coordinator.controller
        controller.start()
        return controller.webView ?? WKWebView(frame: .zero)
    }

    func updateUIView(_ view: WKWebView, context: Context) {
        context.coordinator.controller.setActive(active)
    }

    static func dismantleUIView(_ view: WKWebView, coordinator: Coordinator) {
        coordinator.controller.stop()
    }

    @MainActor final class Coordinator {
        let controller: PackWebController

        init(pack: WebPack, params: [String: String], active: Bool,
             onClose: @escaping () -> Void, onFailure: @escaping (String) -> Void) {
            controller = PackWebController(
                pack: pack, mode: .screen, params: params, active: active, observesAppActivity: false,
                onEvent: { event in
                    switch event {
                    case .close: onClose()
                    case .log(let line): PackStore.shared.record("\(pack.id): \(line)")
                    case .open, .flyTo: break // (an overlay's to ask for)
                    }
                },
                onFailure: onFailure)
        }
    }
}

// MARK: - The overlay

/// A pack's overlay page, transparent over the live globe; it never takes touches.
final class WebPackOverlay: UIView {
    private let controller: PackWebController

    init(pack: WebPack, onEvent: @escaping (WebPackEvent) -> Void) {
        controller = PackWebController(
            pack: pack, mode: .overlay, params: [:],
            active: UIApplication.shared.applicationState == .active, observesAppActivity: true,
            onEvent: onEvent, onFailure: nil)
        super.init(frame: .zero)
        isOpaque = false
        backgroundColor = .clear
        clipsToBounds = true
        isUserInteractionEnabled = false
        if let web = controller.webView {
            web.frame = bounds
            web.autoresizingMask = [.flexibleWidth, .flexibleHeight]
            addSubview(web)
        }
    }

    required init?(coder: NSCoder) { fatalError("WebPackOverlay is made in code") }

    /// Loads the page once the overlay is on the globe, so it is laid out at its real size.
    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window != nil { controller.start() }
    }

    override func layoutSubviews() {
        super.layoutSubviews()
        controller.webView?.frame = bounds
    }

    /// No touch ever lands here, whatever the web view thinks.
    override func point(inside point: CGPoint, with event: UIEvent?) -> Bool { false }

    /// The camera, every globe frame (the page is told only when it changed, one message at a time).
    func update(camera: GlobeCamera, centre: CGPoint, radius: Double) {
        let size = bounds.size
        guard size.width > 0, size.height > 0 else { return }
        controller.updateCamera(.init(
            lon: camera.lon, lat: camera.lat, zoom: camera.zoom,
            cx: Double(centre.x), cy: Double(centre.y), radius: radius,
            width: Double(size.width), height: Double(size.height)))
    }

    /// The globe was tapped at `point` (this view's coordinates; the place under it, if any).
    func tap(at point: CGPoint, place: (lon: Double, lat: Double)?) {
        controller.sendTap(x: Double(point.x), y: Double(point.y), place: place)
    }

    /// Tears the page down. Safe to call more than once.
    func stop() {
        controller.stop()
    }
}

// MARK: - The web view's host

/// The shared host of a pack's web view: its configuration, the bridge, the settings it reports and
/// the camera it is sent. Owns the web view; `stop()` lets all of it go.
final class PackWebController: NSObject, WKNavigationDelegate {
    enum Mode: String { case screen, overlay }

    struct Settings: Equatable {
        var reduceMotion: Bool
        var textScale: Double
        var active: Bool
    }

    struct CameraState: Equatable {
        var lon: Double
        var lat: Double
        var zoom: Double
        var cx: Double
        var cy: Double
        var radius: Double
        var width: Double
        var height: Double

        var finite: Bool {
            [lon, lat, zoom, cx, cy, radius, width, height].allSatisfy { $0.isFinite }
        }

        var payload: [String: Any] {
            ["lon": lon, "lat": lat, "zoom": zoom, "cx": cx, "cy": cy, "radius": radius, "width": width, "height": height]
        }
    }

    let pack: WebPack
    let mode: Mode
    /// What the page asked for. Cleared by `stop()`.
    var onEvent: ((WebPackEvent) -> Void)?
    /// The page could not be opened (or its process stopped). Cleared by `stop()`.
    var onFailure: ((String) -> Void)?
    private(set) var webView: WKWebView?

    private let params: [String: String]
    private let userContent = WKUserContentController()
    private let scheme: PackSchemeHandler
    private let live = LiveCount.Token(.webViews)
    private var bridge: PackBridge?
    private var settings: Settings
    private var sentSettings: Settings?
    private let observesApp: Bool
    private var observers: [any NSObjectProtocol] = []
    private var scriptsInstalled = false
    private var started = false
    private var ready = false
    private var stopped = false
    private var camera: CameraState?
    private var sentCamera: CameraState?
    private var cameraInFlight = false

    init(pack: WebPack, mode: Mode, params: [String: String], active: Bool, observesAppActivity: Bool,
         onEvent: ((WebPackEvent) -> Void)?, onFailure: ((String) -> Void)?) {
        self.pack = pack
        self.mode = mode
        self.params = params
        self.observesApp = observesAppActivity
        self.settings = Self.systemSettings(active: active)
        self.onEvent = onEvent
        self.onFailure = onFailure
        self.scheme = PackSchemeHandler(packID: pack.id, root: pack.root)
        super.init()

        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        configuration.allowsInlineMediaPlayback = true
        configuration.setURLSchemeHandler(scheme, forURLScheme: PackWebKit.scheme)
        configuration.userContentController = userContent

        // (the user content controller holds its handlers strongly: this proxy holds us weakly)
        let bridge = PackBridge(target: self)
        self.bridge = bridge
        userContent.add(bridge, name: "terra")
        installScripts()

        let view = WKWebView(frame: .zero, configuration: configuration)
        view.navigationDelegate = self
        view.isOpaque = false
        view.backgroundColor = .clear
        view.scrollView.backgroundColor = .clear
        view.scrollView.contentInsetAdjustmentBehavior = .never
        view.allowsLinkPreview = false
        view.isInspectable = true // a development build: Safari's Web Inspector can attach
        switch mode {
        case .screen:
            view.scrollView.bounces = false
        case .overlay:
            view.scrollView.isScrollEnabled = false
            view.isUserInteractionEnabled = false
        }
        webView = view
        observe()
    }

    // MARK: setting up

    /// `window.__terraBoot`, then terra.js, before any script of the page, in the main frame only.
    /// Without terra.js nothing is installed and `begin()` reports it (the page is not opened).
    private func installScripts() {
        guard let file = Bundle.main.resourceURL?.appending(path: "Prototypes/_runtime/terra.js"),
              let runtime = try? String(contentsOf: file, encoding: .utf8) else { return }
        let boot: [String: Any] = [
            "pack": ["id": pack.id, "revision": pack.revision, "origin": pack.origin.rawValue] as [String: Any],
            "mode": mode.rawValue,
            "params": params,
            "settings": settingsPayload(settings)
        ]
        guard let json = Self.json(boot) else { return }
        userContent.addUserScript(WKUserScript(source: "window.__terraBoot = \(json);",
                                               injectionTime: .atDocumentStart, forMainFrameOnly: true))
        userContent.addUserScript(WKUserScript(source: runtime, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        scriptsInstalled = true
    }

    private func observe() {
        let center = NotificationCenter.default
        let system: [Notification.Name] = [UIAccessibility.reduceMotionStatusDidChangeNotification,
                                           UIContentSizeCategory.didChangeNotification]
        for name in system {
            let token = center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                Task { @MainActor in self?.systemSettingsChanged() }
            }
            observers.append(token)
        }
        if observesApp {
            let activity: [(Notification.Name, Bool)] = [(UIApplication.didBecomeActiveNotification, true),
                                                         (UIApplication.willResignActiveNotification, false)]
            for (name, isActive) in activity {
                let token = center.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in
                    Task { @MainActor in self?.setActive(isActive) }
                }
                observers.append(token)
            }
        }
    }

    /// Starts loading the page (once).
    func start() {
        guard !started, !stopped else { return }
        started = true
        Task { [self] in await self.begin() }
    }

    private var page: String {
        switch mode {
        case .screen: return pack.manifest.entry ?? "index.html"
        case .overlay: return pack.manifest.overlay ?? "overlay.html"
        }
    }

    private func pageURL() -> URL? {
        var components = URLComponents()
        components.scheme = PackWebKit.scheme
        components.host = pack.id
        components.path = "/" + page
        return components.url
    }

    /// The content rule is compiled first and added before the first load. If it cannot be, the page
    /// is not opened at all.
    private func begin() async {
        guard !stopped, let view = webView else { return }
        guard scriptsInstalled else {
            fail("The app's runtime (Prototypes/_runtime/terra.js) is missing, so the page was not opened.")
            return
        }
        guard FileManager.default.fileExists(atPath: pack.root.appending(path: page).path(percentEncoded: false)),
              let url = pageURL() else {
            fail("The page “\(page)” is not in this pack.")
            return
        }
        do {
            let rules = try await PackWebKit.contentRules()
            guard !stopped else { return }
            userContent.add(rules)
            view.load(URLRequest(url: url))
        } catch {
            fail("The network block could not be set up (\(error.localizedDescription)), so the page was not opened.")
        }
    }

    private func fail(_ reason: String) {
        guard !stopped else { return }
        PackStore.shared.record("\(pack.id): \(reason)")
        onFailure?(reason)
    }

    // MARK: tearing down

    /// The page is told it is closing, then everything is let go: observers, the bridge, the scripts,
    /// the delegates and the web view. Safe to call more than once.
    func stop() {
        guard !stopped else { return }
        stopped = true
        for token in observers { NotificationCenter.default.removeObserver(token) }
        observers.removeAll()
        if let view = webView {
            view.evaluateJavaScript("window.terra&&window.terra._receive({\"type\":\"close\"})", completionHandler: nil)
            view.stopLoading()
            view.navigationDelegate = nil
            view.removeFromSuperview()
        }
        userContent.removeScriptMessageHandler(forName: "terra")
        userContent.removeAllUserScripts()
        userContent.removeAllContentRuleLists()
        scheme.invalidate()
        webView = nil
        bridge = nil
        onEvent = nil
        onFailure = nil
        camera = nil
        sentCamera = nil
    }

    // MARK: settings

    private static func systemSettings(active: Bool) -> Settings {
        let body = Double(UIFont.preferredFont(forTextStyle: .body).pointSize)
        return Settings(reduceMotion: UIAccessibility.isReduceMotionEnabled,
                        textScale: min(3.2, max(1, body / 17)),
                        active: active)
    }

    private func settingsPayload(_ s: Settings) -> [String: Any] {
        ["reduceMotion": s.reduceMotion, "textScale": s.textScale, "active": s.active]
    }

    /// The scene went inactive or active again.
    func setActive(_ active: Bool) {
        guard !stopped, settings.active != active else { return }
        settings.active = active
        sendSettings()
    }

    private func systemSettingsChanged() {
        guard !stopped else { return }
        let current = Self.systemSettings(active: settings.active)
        guard current != settings else { return }
        settings = current
        sendSettings()
    }

    private func sendSettings() {
        guard ready, !stopped, settings != sentSettings else { return }
        sentSettings = settings
        send(["type": "settings", "settings": settingsPayload(settings)])
    }

    // MARK: talking to the page

    private static func json(_ object: Any) -> String? {
        guard JSONSerialization.isValidJSONObject(object),
              let data = try? JSONSerialization.data(withJSONObject: object),
              let text = String(data: data, encoding: .utf8) else { return nil }
        return text
    }

    /// `terra._receive(message)`, without waiting for an answer.
    private func send(_ message: [String: Any]) {
        guard !stopped, let view = webView, let json = Self.json(message) else { return }
        view.evaluateJavaScript("window.terra&&window.terra._receive(\(json))", completionHandler: nil)
    }

    /// The page can be told things from here on: settings first, then the camera.
    private func markReady() {
        guard !stopped else { return }
        ready = true
        sendSettings()
        pumpCamera()
    }

    func updateCamera(_ state: CameraState) {
        guard !stopped, state.finite else { return }
        camera = state
        pumpCamera()
    }

    /// At most one camera message is in flight; when it completes, the latest is sent if it changed
    /// meanwhile. (The trailing `;0` keeps the script's value defined.)
    private func pumpCamera() {
        guard ready, !stopped, !cameraInFlight, let state = camera, state != sentCamera, let view = webView else { return }
        let message: [String: Any] = ["type": "camera", "camera": state.payload]
        guard let json = Self.json(message) else { return }
        cameraInFlight = true
        sentCamera = state
        let script = "window.terra&&window.terra._receive(\(json));0"
        Task { [weak self] in
            _ = try? await view.evaluateJavaScript(script)
            self?.cameraSettled()
        }
    }

    private func cameraSettled() {
        cameraInFlight = false
        pumpCamera()
    }

    func sendTap(x: Double, y: Double, place: (lon: Double, lat: Double)?) {
        guard ready, !stopped, x.isFinite, y.isFinite else { return }
        var message: [String: Any] = ["type": "tap", "x": x, "y": y]
        if let place, place.lon.isFinite, place.lat.isFinite {
            message["lon"] = place.lon
            message["lat"] = place.lat
        } else {
            message["lon"] = NSNull()
            message["lat"] = NSNull()
        }
        send(message)
    }

    // MARK: messages from the page

    fileprivate func receive(_ message: WKScriptMessage) {
        guard !stopped, message.frameInfo.isMainFrame,
              let body = message.body as? [String: Any], let type = body["type"] as? String else { return }
        switch type {
        case "ready": markReady()
        case "close": onEvent?(.close)
        case "open": onEvent?(.open(Self.stringParameters(body["params"])))
        case "flyTo": pageFlies(body)
        case "log": pageLogged(body)
        default: break
        }
    }

    private static func number(_ value: Any?) -> Double? {
        guard let boxed = value as? NSNumber, boxed.doubleValue.isFinite else { return nil }
        return boxed.doubleValue
    }

    private static func stringParameters(_ value: Any?) -> [String: String] {
        guard let raw = value as? [String: Any] else { return [:] }
        var result: [String: String] = [:]
        for (key, item) in raw.prefix(32) where key.count <= 64 {
            if let text = item as? String {
                result[key] = String(text.prefix(1024))
            } else if let boxed = item as? NSNumber {
                result[key] = boxed.stringValue
            }
        }
        return result
    }

    private func pageFlies(_ body: [String: Any]) {
        guard mode == .overlay, let lon = Self.number(body["lon"]), let lat = Self.number(body["lat"]) else { return }
        let zoom = Self.number(body["zoom"]) ?? camera?.zoom ?? 1
        let seconds = min(10, max(0, Self.number(body["duration"]) ?? 0.8))
        onEvent?(.flyTo(GlobeCamera(lon: lon, lat: lat, zoom: zoom), duration: seconds))
    }

    private func pageLogged(_ body: [String: Any]) {
        let text = String(((body["message"] as? String) ?? "").prefix(2000))
        onEvent?(.log((body["level"] as? String) == "error" ? "error: " + text : text))
    }

    // MARK: navigation

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        markReady()
    }

    /// Only this pack's own files, and about:blank. Nothing leaves for Safari or another app.
    func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                 decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
        guard let url = action.request.url else {
            decisionHandler(.cancel)
            return
        }
        let ours = url.scheme?.lowercased() == PackWebKit.scheme
            && url.host(percentEncoded: false)?.lowercased() == pack.id
        decisionHandler(ours || url.absoluteString == "about:blank" ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) {
        navigationFailed(error)
    }

    func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) {
        navigationFailed(error)
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        fail("The page's process stopped.")
    }

    private func navigationFailed(_ error: Error) {
        let nsError = error as NSError
        if nsError.domain == NSURLErrorDomain && nsError.code == NSURLErrorCancelled { return }
        fail("The page could not load (\(error.localizedDescription)).")
    }
}

/// The message handler the web view retains, holding the controller weakly.
private final class PackBridge: NSObject, WKScriptMessageHandler {
    weak var target: PackWebController?

    init(target: PackWebController) {
        self.target = target
        super.init()
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        target?.receive(message)
    }
}

// MARK: - WebKit pieces shared by both

enum PackWebKit {
    static let scheme = "terra-pack"

    /// Blocks every http(s) and ws(s) request. (WebKit's content-blocker expressions have no
    /// alternation, so the two are separate rules.)
    private static let ruleIdentifier = "com.pbordjadze.terraincognita.pack-network"
    private static let ruleSource = #"[{"trigger":{"url-filter":"^https?://"},"action":{"type":"block"}},{"trigger":{"url-filter":"^wss?://"},"action":{"type":"block"}}]"#
    private static var compiled: WKContentRuleList?

    /// Compiled once per launch. Throws if WebKit refuses it: the caller must not load the page then.
    static func contentRules() async throws -> WKContentRuleList {
        if let compiled { return compiled }
        let result = try await WKContentRuleListStore.default().compileContentRuleList(
            forIdentifier: ruleIdentifier, encodedContentRuleList: ruleSource)
        guard let list = result as WKContentRuleList? else {
            throw PackError("WebKit returned no rule list")
        }
        compiled = list
        return list
    }
}

// MARK: - terra-pack://<id>/<path>

/// Serves one pack's files, and nothing else.
final class PackSchemeHandler: NSObject, WKURLSchemeHandler {
    static let policy = "default-src 'self' terra-pack: data: blob: 'unsafe-inline'; connect-src 'self' terra-pack:; frame-src 'none'; form-action 'none'"

    private enum Resolution {
        case refused, missing
        case file(URL)
    }

    private let packID: String
    private let root: URL
    private let rootPath: String
    /// Requests being answered. A task `webView(_:stop:)` has taken away is not in here, and is
    /// never answered.
    private var tasks: [ObjectIdentifier: any WKURLSchemeTask] = [:]

    init(packID: String, root: URL) {
        let standard = root.standardizedFileURL
        var path = standard.path(percentEncoded: false)
        while path.count > 1 && path.hasSuffix("/") { path.removeLast() }
        self.packID = packID
        self.root = standard
        self.rootPath = path
        super.init()
    }

    /// Drops every request in flight (the web view is going away).
    func invalidate() {
        tasks.removeAll()
    }

    func webView(_ webView: WKWebView, start urlSchemeTask: any WKURLSchemeTask) {
        let request = urlSchemeTask.request
        guard let url = request.url else {
            urlSchemeTask.didFailWithError(URLError(.badURL))
            return
        }
        guard (request.httpMethod ?? "GET") == "GET" else {
            send(urlSchemeTask, url: url, status: 405, type: "text/plain; charset=utf-8", body: Data("Method not allowed".utf8))
            return
        }
        switch resolve(url) {
        case .refused:
            send(urlSchemeTask, url: url, status: 403, type: "text/plain; charset=utf-8", body: Data("Forbidden".utf8))
        case .missing:
            send(urlSchemeTask, url: url, status: 404, type: "text/plain; charset=utf-8", body: Data("Not found".utf8))
        case .file(let file):
            let key = ObjectIdentifier(urlSchemeTask as AnyObject)
            tasks[key] = urlSchemeTask
            let range = Self.byteRange(from: request.value(forHTTPHeaderField: "Range"))
            let type = Self.mimeType(forExtension: file.pathExtension)
            Task { [weak self] in
                let read = await PackFiles.read(file, range: range)
                self?.finish(key, url: url, type: type, read: read)
            }
        }
    }

    func webView(_ webView: WKWebView, stop urlSchemeTask: any WKURLSchemeTask) {
        tasks.removeValue(forKey: ObjectIdentifier(urlSchemeTask as AnyObject))
    }

    // MARK: answering

    private func finish(_ key: ObjectIdentifier, url: URL, type: String, read: PackFileRead) {
        guard let task = tasks.removeValue(forKey: key) else { return } // stopped meanwhile
        switch read {
        case .failed:
            send(task, url: url, status: 404, type: "text/plain; charset=utf-8", body: Data("Not found".utf8))
        case .whole(let data):
            send(task, url: url, status: 200, type: type, body: data, extra: ["Accept-Ranges": "bytes"])
        case .part(let data, let first, let total):
            send(task, url: url, status: 206, type: type, body: data,
                 extra: ["Accept-Ranges": "bytes", "Content-Range": "bytes \(first)-\(first + data.count - 1)/\(total)"])
        case .unsatisfiable(let total):
            send(task, url: url, status: 416, type: "text/plain; charset=utf-8", body: Data(),
                 extra: ["Content-Range": "bytes */\(total)"])
        }
    }

    private func send(_ task: any WKURLSchemeTask, url: URL, status: Int, type: String, body: Data,
                      extra: [String: String] = [:]) {
        var fields = extra
        fields["Content-Type"] = type
        fields["Content-Length"] = String(body.count)
        fields["Cache-Control"] = "no-store"
        fields["Content-Security-Policy"] = Self.policy
        guard let response = HTTPURLResponse(url: url, statusCode: status, httpVersion: "HTTP/1.1", headerFields: fields) else {
            task.didFailWithError(URLError(.badServerResponse))
            return
        }
        task.didReceive(response)
        if !body.isEmpty { task.didReceive(body) }
        task.didFinish()
    }

    /// terra-pack://<this pack>/<path> inside the pack's folder, a file: no other host, no `..`,
    /// no leaving the root, no directory.
    private func resolve(_ url: URL) -> Resolution {
        guard url.scheme?.lowercased() == PackWebKit.scheme,
              url.host(percentEncoded: false)?.lowercased() == packID else { return .refused }
        let parts = url.path(percentEncoded: false).split(separator: "/", omittingEmptySubsequences: true)
        guard !parts.isEmpty else { return .missing }
        var file = root
        for part in parts {
            if part == "." || part == ".." || part.utf8.contains(0) { return .refused }
            file = file.appending(path: String(part))
        }
        file = file.standardizedFileURL
        let path = file.path(percentEncoded: false)
        guard path.hasPrefix(rootPath + "/") else { return .refused }
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory), !isDirectory.boolValue else {
            return .missing
        }
        return .file(file)
    }

    /// One range, as Safari asks for media: "bytes=0-", "bytes=100-199", "bytes=-500".
    private static func byteRange(from header: String?) -> PackByteRange? {
        guard let header, header.hasPrefix("bytes=") else { return nil }
        let spec = header.dropFirst(6)
        guard !spec.contains(","), let dash = spec.firstIndex(of: "-") else { return nil }
        let head = String(spec[spec.startIndex..<dash])
        let tail = String(spec[spec.index(after: dash)...])
        if head.isEmpty && tail.isEmpty { return nil }
        let first = Int(head)
        let last = Int(tail)
        if (!head.isEmpty && first == nil) || (!tail.isEmpty && last == nil) { return nil }
        if let first, first < 0 { return nil }
        if let last, last < 0 { return nil }
        return PackByteRange(first: first, last: last)
    }

    private static func mimeType(forExtension ext: String) -> String {
        switch ext.lowercased() {
        case "html", "htm": return "text/html; charset=utf-8"
        case "js", "mjs": return "text/javascript; charset=utf-8"
        case "css": return "text/css; charset=utf-8"
        case "json": return "application/json; charset=utf-8"
        case "png": return "image/png"
        case "jpg", "jpeg": return "image/jpeg"
        case "gif": return "image/gif"
        case "webp": return "image/webp"
        case "svg": return "image/svg+xml"
        case "woff": return "font/woff"
        case "woff2": return "font/woff2"
        case "otf": return "font/otf"
        case "ttf": return "font/ttf"
        case "mp3": return "audio/mpeg"
        case "m4a": return "audio/mp4"
        case "wav": return "audio/wav"
        case "mp4": return "video/mp4"
        case "txt": return "text/plain; charset=utf-8"
        default: return "application/octet-stream"
        }
    }
}

/// A byte range as the request header gives it: `first`...`last`, `first`... (to the end), or
/// `last` alone for "the final `last` bytes" (first nil).
nonisolated struct PackByteRange: Sendable {
    var first: Int?
    var last: Int?
}

nonisolated enum PackFileRead: Sendable {
    case failed
    case whole(Data)
    case part(Data, first: Int, total: Int)
    case unsatisfiable(total: Int)
}

nonisolated enum PackFiles {
    /// Reads a pack file, or the part of it asked for, off the main actor.
    @concurrent static func read(_ url: URL, range: PackByteRange?) async -> PackFileRead {
        guard let range else {
            guard let data = try? Data(contentsOf: url, options: .mappedIfSafe) else { return .failed }
            return .whole(data)
        }
        do {
            let handle = try FileHandle(forReadingFrom: url)
            defer { try? handle.close() }
            let end = try handle.seekToEnd()
            let size = Int(clamping: end)
            let first: Int
            let last: Int
            if let start = range.first {
                first = start
                last = min(range.last ?? (size - 1), size - 1)
            } else if let suffix = range.last {
                first = max(0, size - suffix)
                last = size - 1
            } else {
                return .failed
            }
            guard size > 0, first <= last, first < size else { return .unsatisfiable(total: size) }
            try handle.seek(toOffset: UInt64(first))
            let data = try handle.read(upToCount: last - first + 1) ?? Data()
            return .part(data, first: first, total: size)
        } catch {
            return .failed
        }
    }
}
