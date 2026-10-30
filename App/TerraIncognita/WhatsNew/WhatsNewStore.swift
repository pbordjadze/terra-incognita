#if UNRELEASED
import Foundation
import Observation
import Security
import UIKit

/// Where the owner's answers live on the device (Application Support/WhatsNew/): the ledger of what
/// has been seen, and every round's answers with their voice notes and pictures, until they are sent.
/// The answers are the owner's private data: nothing here leaves the device but by `WhatsNewCourier`
/// at the owner's tap, or by the share sheet.
@Observable
final class WhatsNewStore {
    let root: URL
    private(set) var ledger: WhatsNewLedger
    /// File names already in the private repository (`WhatsNewCourier`), so a retry sends the rest.
    private(set) var sent: Set<String>
    /// The app's name in the answers (`terra-incognita`), and where they go.
    let app: String
    let destination: WhatsNewDestination
    /// A reset (CI's checks) also leaves the owner's token alone and unused.
    let tokenIgnored: Bool

    init(app: String, destination: WhatsNewDestination, root: URL? = nil, reset: Bool = false) {
        self.app = app
        self.destination = destination
        tokenIgnored = reset
        let base = root ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appending(path: "WhatsNew", directoryHint: .isDirectory)
        self.root = base
        if reset { try? FileManager.default.removeItem(at: base) }
        try? FileManager.default.createDirectory(at: base.appending(path: "answers"), withIntermediateDirectories: true)
        ledger = (try? JSONDecoder().decode(WhatsNewLedger.self, from: Data(contentsOf: base.appending(path: "ledger.json")))) ?? WhatsNewLedger()
        sent = (try? JSONDecoder().decode(Set<String>.self, from: Data(contentsOf: base.appending(path: "sent.json")))) ?? []
    }

    // MARK: the ledger

    func record(_ piece: WhatsNewPiece, _ reaction: WhatsNewReaction, at: String) {
        ledger.record(piece, reaction, at: at)
        write(ledger, to: "ledger.json")
    }

    /// Forgets what was seen (the workbench's way to see everything again). The answers stay.
    func forgetSeen() {
        ledger = WhatsNewLedger()
        write(ledger, to: "ledger.json")
    }

    // MARK: the answers

    /// A round's folder: `answers/<round>/`.
    func folder(round: String) -> URL {
        let url = root.appending(path: "answers/\(round)", directoryHint: .isDirectory)
        try? FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }

    /// Writes an answer (`<stamp>-<piece>.json`) beside its media, and returns the file.
    @discardableResult
    func save(_ answer: WhatsNewAnswer, stamp: String) -> URL? {
        let url = folder(round: answer.round).appending(path: "\(stamp)-\(answer.piece).json")
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
        guard let data = try? encoder.encode(answer) else { return nil }
        try? data.write(to: url, options: .atomic)
        return url
    }

    /// Every file of every round, oldest round first: what is on the device.
    func files() -> [URL] {
        let answers = root.appending(path: "answers", directoryHint: .isDirectory)
        let rounds = ((try? FileManager.default.contentsOfDirectory(at: answers, includingPropertiesForKeys: nil)) ?? [])
            .filter { (try? $0.resourceValues(forKeys: [.isDirectoryKey]).isDirectory) == true }
            .sorted { $0.lastPathComponent < $1.lastPathComponent }
        return rounds.flatMap { round in
            ((try? FileManager.default.contentsOfDirectory(at: round, includingPropertiesForKeys: nil)) ?? [])
                .filter { !$0.lastPathComponent.hasPrefix(".") }
                .sorted { $0.lastPathComponent < $1.lastPathComponent }
        }
    }

    /// The path a file takes in the repository: `<folder>/<app>/<round>/<name>`.
    func remotePath(of file: URL) -> String {
        "\(destination.folder)/\(file.deletingLastPathComponent().lastPathComponent)/\(file.lastPathComponent)"
    }

    /// Files not yet in the private repository.
    func unsent() -> [URL] { files().filter { !sent.contains(remotePath(of: $0)) } }

    func markSent(_ file: URL) {
        sent.insert(remotePath(of: file))
        write(sent, to: "sent.json")
    }

    /// Every answer on the device, newest first (the list's "what you said").
    func answers() -> [WhatsNewAnswer] {
        files().filter { $0.pathExtension == "json" }
            .compactMap { try? JSONDecoder().decode(WhatsNewAnswer.self, from: Data(contentsOf: $0)) }
            .sorted { $0.at > $1.at }
    }

    /// The device's answers as one zip, for the share sheet (the system's own archiver, through a file
    /// coordinator's upload copy). Nil with nothing to share.
    func bundleForSharing() -> URL? {
        let answers = root.appending(path: "answers", directoryHint: .isDirectory)
        guard !files().isEmpty else { return nil }
        let out = FileManager.default.temporaryDirectory.appending(path: "\(app)-answers-\(Self.stamp()).zip")
        try? FileManager.default.removeItem(at: out)
        var copied: URL?
        var failure: NSError?
        NSFileCoordinator().coordinate(readingItemAt: answers, options: .forUploading, error: &failure) { zipped in
            copied = (try? FileManager.default.copyItem(at: zipped, to: out)) != nil ? out : nil
        }
        return failure == nil ? copied : nil
    }

    private func write<T: Encodable>(_ value: T, to name: String) {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        if let data = try? encoder.encode(value) { try? data.write(to: root.appending(path: name), options: .atomic) }
    }

    // MARK: stamps and the device

    /// `2026-10-30T0912-33` in the owner's time (Eastern): a file name's beginning, sortable.
    nonisolated static func stamp(_ date: Date = Date()) -> String {
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(identifier: "America/New_York") ?? .current
        let c = cal.dateComponents([.year, .month, .day, .hour, .minute, .second], from: date)
        return String(format: "%04d-%02d-%02dT%02d%02d-%02d", c.year ?? 0, c.month ?? 0, c.day ?? 0, c.hour ?? 0, c.minute ?? 0, c.second ?? 0)
    }

    /// ISO 8601 with the owner's offset: `2026-10-30T09:12:33-04:00`.
    nonisolated static func iso(_ date: Date = Date()) -> String {
        let f = ISO8601DateFormatter()
        f.timeZone = TimeZone(identifier: "America/New_York") ?? .current
        f.formatOptions = [.withInternetDateTime]
        return f.string(from: date)
    }

    static var build: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleVersion") as? String ?? "0" }
    static var appVersion: String { Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "0" }
    /// The hardware model (`iPad16,6`), not the owner's name for it.
    static var device: String {
        var info = utsname()
        uname(&info)
        let model = withUnsafeBytes(of: &info.machine) { raw in String(decoding: raw.prefix { $0 != 0 }, as: UTF8.self) }
        return model.isEmpty ? UIDevice.current.model : model
    }
    static var os: String { UIDevice.current.systemVersion }
}

/// Where the answers go: a private repository, by the GitHub contents API, under one folder. No
/// secret here: the token is the owner's, entered once and kept in the Keychain (`WhatsNewKeychain`).
nonisolated struct WhatsNewDestination: Sendable, Equatable {
    /// `owner/name`.
    let repository: String
    /// The folder the app's answers land under, e.g. `interviews/paul/terra`.
    let folder: String
    var branch: String? = nil
    /// Where to see them.
    var browseURL: URL? { URL(string: "https://github.com/\(repository)/tree/\(branch ?? "main")/\(folder)") }
}

/// The owner's token, in the Keychain and nowhere else (not UserDefaults, not a file, never a repository).
nonisolated enum WhatsNewKeychain {
    private static let service = "whatsnew.token"

    static func token() -> String? {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
                                    kSecReturnData as String: true, kSecMatchLimit as String: kSecMatchLimitOne]
        var item: CFTypeRef?
        guard SecItemCopyMatching(query as CFDictionary, &item) == errSecSuccess, let data = item as? Data else { return nil }
        return String(data: data, encoding: .utf8).flatMap { $0.isEmpty ? nil : $0 }
    }

    static func set(_ token: String) {
        clear()
        let trimmed = token.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, let data = trimmed.data(using: .utf8) else { return }
        let item: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service,
                                   kSecValueData as String: data,
                                   kSecAttrAccessible as String: kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly]
        SecItemAdd(item as CFDictionary, nil)
    }

    static func clear() {
        let query: [String: Any] = [kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service]
        SecItemDelete(query as CFDictionary)
    }
}

/// Carries the answers to the private repository: one HTTPS PUT a file, through the GitHub contents
/// API, with the owner's token, only when the owner taps Send. A file already there is left as it
/// is (a retry after a half-sent round sends the rest). This is the module's one use of the network.
nonisolated enum WhatsNewCourier {
    nonisolated struct Failure: LocalizedError {
        let message: String
        var errorDescription: String? { message }
    }

    private static func makeSession() -> URLSession {
        let c = URLSessionConfiguration.ephemeral
        c.requestCachePolicy = .reloadIgnoringLocalCacheData
        c.urlCache = nil
        c.httpCookieStorage = nil
        c.httpShouldSetCookies = false
        c.timeoutIntervalForRequest = 30
        c.timeoutIntervalForResource = 300
        return URLSession(configuration: c)
    }

    private static func request(_ method: String, _ url: URL, token: String, body: Data? = nil) -> URLRequest {
        var r = URLRequest(url: url)
        r.httpMethod = method
        r.setValue("Bearer \(token)", forHTTPHeaderField: "Authorization")
        r.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        r.setValue("2022-11-28", forHTTPHeaderField: "X-GitHub-Api-Version")
        if let body {
            r.setValue("application/json", forHTTPHeaderField: "Content-Type")
            r.httpBody = body
        }
        return r
    }

    private static func encodedPath(_ path: String) -> String {
        path.split(separator: "/").map { $0.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? String($0) }.joined(separator: "/")
    }

    /// Whether the token opens the repository: its full name, or why not.
    @concurrent static func check(_ destination: WhatsNewDestination, token: String) async throws -> String {
        guard let url = URL(string: "https://api.github.com/repos/\(destination.repository)") else { throw Failure(message: "bad repository") }
        let (data, response) = try await makeSession().data(for: request("GET", url, token: token))
        let code = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard code == 200 else { throw Failure(message: Self.explain(code, data)) }
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let perms = json?["permissions"] as? [String: Any]
        guard perms?["push"] as? Bool == true else { throw Failure(message: "The token opens \(destination.repository) but cannot write to it: it needs Contents: Read and write.") }
        return json?["full_name"] as? String ?? destination.repository
    }

    /// Puts one file at `path` (unless one is there already). `message` is the commit's.
    @concurrent static func put(_ file: URL, at path: String, destination: WhatsNewDestination, token: String, message: String) async throws {
        let session = makeSession()
        guard let url = URL(string: "https://api.github.com/repos/\(destination.repository)/contents/\(encodedPath(path))") else {
            throw Failure(message: "bad path \(path)")
        }
        // already there? (a retry after a half-sent round)
        var probe = url
        if let branch = destination.branch, var parts = URLComponents(url: url, resolvingAgainstBaseURL: false) {
            parts.queryItems = [URLQueryItem(name: "ref", value: branch)]
            probe = parts.url ?? url
        }
        if let found = try? await session.data(for: request("GET", probe, token: token)),
           (found.1 as? HTTPURLResponse)?.statusCode == 200 {
            return
        }
        let content = try Data(contentsOf: file).base64EncodedString()
        var body: [String: Any] = ["message": message, "content": content]
        if let branch = destination.branch { body["branch"] = branch }
        let (data, response) = try await session.data(for: request("PUT", url, token: token, body: try JSONSerialization.data(withJSONObject: body)))
        let code = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard code == 201 || code == 200 else { throw Failure(message: Self.explain(code, data)) }
    }

    private static func explain(_ code: Int, _ data: Data) -> String {
        let said = (try? JSONSerialization.jsonObject(with: data) as? [String: Any])?["message"] as? String
        switch code {
        case 0: return "No answer from GitHub (is the device online?)."
        case 401: return "GitHub refused the token (expired, or mistyped)."
        case 403: return "GitHub refused: \(said ?? "forbidden"). The token needs Contents: Read and write on the repository."
        case 404: return "GitHub finds no such repository for this token: give it access to the repository."
        case 409, 422: return "GitHub would not take the file: \(said ?? "conflict")."
        default: return "GitHub answered \(code)\(said.map { ": \($0)" } ?? "")."
        }
    }
}
#endif
