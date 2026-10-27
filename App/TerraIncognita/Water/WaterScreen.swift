import SwiftUI
import WebKit
import UIKit

/// Isolated, bundled study. A separate test bundle identifier opts into its launch route;
/// ordinary builds keep the existing atlas. The web surface shares the tested drawing code.
struct WaterScreen: View {
    @Environment(\.dynamicTypeSize) private var dynamicTypeSize
    @Environment(\.accessibilityReduceMotion) private var reduceMotion
    @State private var showingAtlas = false

    var body: some View {
        Group {
            if showingAtlas {
                ZStack(alignment: .topTrailing) {
                    AtlasScreen()
                    Button("Study II") { showingAtlas = false }
                        .buttonStyle(.borderedProminent)
                        .padding(.top, 60)
                        .padding(.trailing, 20)
                }
            } else {
                WaterWebView(textScale: textScale, reduceMotion: reduceMotion)
                    .ignoresSafeArea()
                    .preferredColorScheme(.light)
                    .overlay(alignment: .topTrailing) {
                        // The old globe and film remain available, without becoming part of the study.
                        Button { showingAtlas = true } label: {
                            Image(systemName: "globe.europe.africa")
                                .font(.system(size: 17))
                                .frame(width: 44, height: 44)
                        }
                        .foregroundStyle(Color(red: 0.16, green: 0.25, blue: 0.31))
                        .accessibilityLabel("Open the existing atlas")
                        .padding(.trailing, 18)
                        .padding(.top, 0)
                    }
            }
        }
    }

    private var textScale: Double {
        switch dynamicTypeSize {
        case .xSmall, .small, .medium, .large: 1
        case .xLarge: 1.1
        case .xxLarge: 1.2
        case .xxxLarge: 1.3
        case .accessibility1: 1.4
        case .accessibility2: 1.5
        case .accessibility3: 1.6
        case .accessibility4: 1.7
        case .accessibility5: 1.8
        @unknown default: 1
        }
    }
}

private struct WaterWebView: UIViewRepresentable {
    let textScale: Double
    let reduceMotion: Bool

    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        let script = "window.__nativeReaderScale = \(textScale); window.__nativeReduceMotion = \(reduceMotion ? "true" : "false");"
        configuration.userContentController.addUserScript(WKUserScript(source: script, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        let view = WKWebView(frame: .zero, configuration: configuration)
        view.isOpaque = true
        view.backgroundColor = UIColor(red: 244/255, green: 240/255, blue: 230/255, alpha: 1)
        view.scrollView.backgroundColor = view.backgroundColor
        view.scrollView.bounces = false
        view.scrollView.contentInsetAdjustmentBehavior = .never
        view.navigationDelegate = context.coordinator
        view.allowsLinkPreview = false
        view.isInspectable = false
        if let root = Bundle.main.resourceURL?.appendingPathComponent("Water", isDirectory: true),
           FileManager.default.fileExists(atPath: root.appending(path: "index.html").path(percentEncoded: false)) {
            context.coordinator.root = root
            view.loadFileURL(root.appending(path: "index.html"), allowingReadAccessTo: root)
        } else {
            view.loadHTMLString("<html><body><h1>The map could not be opened.</h1><p>The bundled study is missing. Please reinstall the test build.</p></body></html>", baseURL: nil)
        }
        return view
    }

    func updateUIView(_ view: WKWebView, context: Context) {
        context.coordinator.scale = textScale
        context.coordinator.reduced = reduceMotion
        view.evaluateJavaScript("window.setReaderScale?.(\(textScale)); window.setReducedMotion?.(\(reduceMotion ? "true" : "false"));", completionHandler: nil)
    }

    @MainActor final class Coordinator: NSObject, WKNavigationDelegate {
        var root: URL?
        var scale: Double = 1
        var reduced = false
        private var recoveringContent = false

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            recoveringContent = false
            #if DEBUG
            print("Study II: bundled document loaded")
            #endif
            webView.evaluateJavaScript("window.setReaderScale?.(\(scale)); window.setReducedMotion?.(\(reduced ? "true" : "false"));", completionHandler: nil)
        }

        func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
            guard !recoveringContent else { return }
            recoveringContent = true
            webView.reload()
        }

        func webView(_ webView: WKWebView, decidePolicyFor action: WKNavigationAction,
                     decisionHandler: @escaping @MainActor @Sendable (WKNavigationActionPolicy) -> Void) {
            guard let url = action.request.url else { decisionHandler(.cancel); return }
            let path = root?.standardizedFileURL.path ?? ""
            let prefix = path.hasSuffix("/") ? path : path + "/"
            if url.isFileURL, root != nil,
               url.standardizedFileURL.path.hasPrefix(prefix) {
                decisionHandler(.allow)
            } else if url.absoluteString == "about:blank" {
                decisionHandler(.allow)
            } else {
                // Only an explicit tap on a cited source opens Safari; the map makes no requests.
                if action.navigationType == .linkActivated, url.scheme == "https", ["doi.org", "creativecommons.org"].contains(url.host ?? "") {
                    UIApplication.shared.open(url)
                }
                decisionHandler(.cancel)
            }
        }
    }
}
