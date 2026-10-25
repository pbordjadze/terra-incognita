import Foundation
import UniformTypeIdentifiers
import WebKit

/// Serves the app's `Web` folder at terra://app/…, so the page, its scripts and its textures
/// share one origin: WebGL may read the images, and fetch() reads the shader and the data.
/// Nothing is fetched from the network.
final class BundleScheme: NSObject, WKURLSchemeHandler {
    static let scheme = "terra"
    static let start = URL(string: "terra://app/index.html")!

    private let root = Bundle.main.bundleURL.appending(path: "Web", directoryHint: .isDirectory).standardizedFileURL

    func webView(_ webView: WKWebView, start task: any WKURLSchemeTask) {
        guard let url = task.request.url else {
            task.didFailWithError(URLError(.badURL))
            return
        }
        var path = url.path
        if path.hasPrefix("/") { path.removeFirst() }
        if path.isEmpty { path = "index.html" }
        let file = root.appending(path: path).standardizedFileURL
        // Only files inside Web/: a path with .. in it is refused.
        guard file.path.hasPrefix(root.path + "/"), let data = try? Data(contentsOf: file) else {
            task.didFailWithError(URLError(.fileDoesNotExist))
            return
        }
        let type = UTType(filenameExtension: file.pathExtension)?.preferredMIMEType ?? "application/octet-stream"
        let headers = ["Content-Type": type, "Content-Length": String(data.count), "Cache-Control": "no-cache"]
        guard let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: headers) else {
            task.didFailWithError(URLError(.cannotParseResponse))
            return
        }
        task.didReceive(response)
        task.didReceive(data)
        task.didFinish()
    }

    func webView(_ webView: WKWebView, stop task: any WKURLSchemeTask) {}
}
