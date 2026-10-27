import SwiftUI
import UIKit
import WebKit

/// An offline, authored atlas leaf. This host is separate from the Metal globe and plate player.
struct LeafScreen: View {
    @Environment(\.dismiss) private var dismiss
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @Environment(\.dynamicTypeSize) private var typeSize
    @Environment(\.scenePhase) private var scenePhase
    @State private var failed = false

    var body: some View {
        ZStack {
            Color(red: 0.957, green: 0.945, blue: 0.910).ignoresSafeArea()
            if failed {
                VStack(spacing: 24) {
                    Text("This leaf could not open.").font(.title2)
                    Button("Return to the atlas") { dismiss() }
                }
                .foregroundStyle(Color(red: 0.16, green: 0.27, blue: 0.32))
            } else {
                LeafWebView(
                    reduceMotion: reduceMotion,
                    textSize: textScale,
                    active: scenePhase == .active,
                    onClose: { dismiss() },
                    onFailure: { failed = true }
                )
                .ignoresSafeArea()
            }
        }
        .statusBarHidden()
        .persistentSystemOverlays(.hidden)
        .preferredColorScheme(.light)
    }

    private var textScale: Double {
        // Explicitly read the environment so changes while the leaf is open update the web view.
        _ = typeSize
        return min(3.2, max(1, Double(UIFont.preferredFont(forTextStyle: .body).pointSize / 17)))
    }
}

private struct LeafWebView: UIViewRepresentable {
    var reduceMotion: Bool
    var textSize: Double
    var active: Bool
    var onClose: () -> Void
    var onFailure: () -> Void

    func makeCoordinator() -> Coordinator { Coordinator(onClose: onClose, onFailure: onFailure) }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.websiteDataStore = .nonPersistent()
        config.preferences.javaScriptCanOpenWindowsAutomatically = false
        config.userContentController.add(context.coordinator, name: "leaf")
        let view = WKWebView(frame: .zero, configuration: config)
        view.navigationDelegate = context.coordinator
        view.isOpaque = false
        view.backgroundColor = .clear
        view.scrollView.backgroundColor = .clear
        view.scrollView.isScrollEnabled = false
        view.scrollView.bounces = false
        view.scrollView.contentInsetAdjustmentBehavior = .never
        view.allowsLinkPreview = false
        view.isInspectable = false
        #if DEBUG
        view.isInspectable = true
        #endif
        context.coordinator.webView = view
        guard let root = Bundle.main.resourceURL?.appendingPathComponent("Leaf", isDirectory: true),
              FileManager.default.fileExists(atPath: root.appendingPathComponent("index.html").path) else {
            DispatchQueue.main.async { onFailure() }
            return view
        }
        view.loadFileURL(root.appendingPathComponent("index.html"), allowingReadAccessTo: root)
        return view
    }

    func updateUIView(_ view: WKWebView, context: Context) {
        context.coordinator.settings = Settings(reduceMotion: reduceMotion, textSize: textSize, active: active)
        context.coordinator.applySettings()
    }

    static func dismantleUIView(_ view: WKWebView, coordinator: Coordinator) {
        view.evaluateJavaScript("window.leafDispose?.()", completionHandler: nil)
        view.stopLoading()
        view.configuration.userContentController.removeScriptMessageHandler(forName: "leaf")
        view.navigationDelegate = nil
    }

    struct Settings: Equatable {
        var reduceMotion: Bool
        var textSize: Double
        var active: Bool
    }

    @MainActor final class Coordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
        weak var webView: WKWebView?
        var settings = Settings(reduceMotion: false, textSize: 1, active: true)
        private var applied: Settings?
        private var loaded = false
        let onClose: () -> Void
        let onFailure: () -> Void

        init(onClose: @escaping () -> Void, onFailure: @escaping () -> Void) {
            self.onClose = onClose
            self.onFailure = onFailure
        }

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            guard message.frameInfo.isMainFrame, message.name == "leaf", message.body as? String == "close" else { return }
            onClose()
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            loaded = true
            applySettings()
        }

        func applySettings() {
            guard loaded, applied != settings else { return }
            applied = settings
            let script = "window.leafSettings?.({reduceMotion:\(settings.reduceMotion),textSize:\(settings.textSize),active:\(settings.active)})"
            webView?.evaluateJavaScript(script, completionHandler: nil)
        }

        func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                     decisionHandler: @escaping (WKNavigationActionPolicy) -> Void) {
            // No external browsing, popups or downloads. All content is inside the bundled leaf.
            guard let url = action.request.url, url.isFileURL,
                  let root = Bundle.main.resourceURL?.appendingPathComponent("Leaf").standardizedFileURL.path,
                  url.standardizedFileURL.path.hasPrefix(root + "/") else {
                decisionHandler(.cancel)
                return
            }
            decisionHandler(.allow)
        }

        func webView(_ webView: WKWebView, didFail navigation: WKNavigation!, withError error: Error) { onFailure() }
        func webView(_ webView: WKWebView, didFailProvisionalNavigation navigation: WKNavigation!, withError error: Error) { onFailure() }
        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) { onFailure() }
    }
}
