import Foundation
import SwiftUI
import UIKit
import WebKit

/// The bundled page in a web view that neither scrolls nor zooms: every touch goes to the map.
struct MapView: UIViewRepresentable {
    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.setURLSchemeHandler(context.coordinator.scheme, forURLScheme: BundleScheme.scheme)
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
        web.load(URLRequest(url: Self.startURL()))
        return web
    }

    func updateUIView(_ web: WKWebView, context: Context) {}

    private static func startURL() -> URL {
        #if DEBUG
        // `-tour t213p` opens the film at 213 s, paused (CI's screenshots); `-tour t213` plays from there.
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-tour"), i + 1 < args.count,
           let moment = URL(string: "terra://app/index.html#" + args[i + 1]) {
            return moment
        }
        #endif
        return BundleScheme.start
    }

    /// Serves the bundle, sends links out to Safari, and brings the page back if iOS ends its
    /// process (the page keeps the film's place in its address, so a reload resumes there).
    final class Coordinator: NSObject, WKNavigationDelegate, WKUIDelegate {
        let scheme = BundleScheme()

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
            webView.reload()
        }
    }
}
