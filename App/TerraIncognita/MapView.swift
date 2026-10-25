import Foundation
import SwiftUI
import UIKit
import WebKit

/// The bundled page in a web view that neither scrolls nor zooms: every touch goes to the map.
struct MapView: UIViewRepresentable {
    let status: PageStatus

    func makeCoordinator() -> Coordinator { Coordinator(status: status) }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.setURLSchemeHandler(context.coordinator.scheme, forURLScheme: BundleScheme.scheme)
        config.userContentController.add(context.coordinator, name: "page")
        #if DEBUG
        // `-tour t213s` and `-diag no2d` (CI) reach the page before its scripts run, whatever
        // becomes of the URL
        let args = ProcessInfo.processInfo.arguments
        for (flag, name) in [("-tour", "TI_TOUR"), ("-diag", "TI_DIAG")] {
            guard let i = args.firstIndex(of: flag), i + 1 < args.count else { continue }
            let value = args[i + 1].filter { $0.isLetter || $0.isNumber || $0 == "." }
            config.userContentController.addUserScript(WKUserScript(source: "window.\(name) = '\(value)';",
                                                                    injectionTime: .atDocumentStart, forMainFrameOnly: true))
        }
        #endif
        let web = WKWebView(frame: .zero, configuration: config)
        web.navigationDelegate = context.coordinator
        web.uiDelegate = context.coordinator
        web.isOpaque = false
        web.backgroundColor = .black
        web.scrollView.backgroundColor = .black
        web.scrollView.isScrollEnabled = false
        web.scrollView.bounces = false
        web.scrollView.contentInsetAdjustmentBehavior = .never
        web.scrollView.pinchGestureRecognizer?.isEnabled = false
        #if DEBUG
        web.isInspectable = true
        #endif
        web.load(URLRequest(url: BundleScheme.start))
        return web
    }

    func updateUIView(_ web: WKWebView, context: Context) {}

    /// Serves the bundle, passes on the page's reports, sends links out to Safari, and brings the
    /// page back if iOS ends its process (the page keeps the film's place in its address, so a
    /// reload resumes there).
    final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate, WKScriptMessageHandler {
        let scheme = BundleScheme()
        let status: PageStatus

        init(status: PageStatus) {
            self.status = status
            super.init()
        }

        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            if let line = message.body as? String { status.report(line) }
        }

        func webView(_ webView: WKWebView, decidePolicyFor navigationAction: WKNavigationAction) async -> WKNavigationActionPolicy {
            guard let url = navigationAction.request.url, url.scheme == "https" || url.scheme == "http" else { return .allow }
            UIApplication.shared.open(url, options: [:], completionHandler: nil)
            return .cancel
        }

        func webView(_ webView: WKWebView, createWebViewWith configuration: WKWebViewConfiguration,
                     for navigationAction: WKNavigationAction, windowFeatures: WKWindowFeatures) -> WKWebView? {
            if let url = navigationAction.request.url { UIApplication.shared.open(url, options: [:], completionHandler: nil) }
            return nil
        }

        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
            status.report("the page's process ended: reloading")
            webView.reload()
        }
    }
}
