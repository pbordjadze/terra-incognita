import Foundation
import SwiftUI
import UIKit
import WebKit

/// The bundled page in a web view that neither scrolls nor zooms: every touch goes to the map.
struct MapView: UIViewRepresentable {
    func makeCoordinator() -> BundleScheme { BundleScheme() }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.setURLSchemeHandler(context.coordinator, forURLScheme: BundleScheme.scheme)
        let web = WKWebView(frame: .zero, configuration: config)
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
}
