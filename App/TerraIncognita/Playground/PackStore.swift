import CryptoKit
import Foundation
import Observation
import OSLog

// The web packs the workbench can open: the ones bundled in the app (`Prototypes/<id>/`), the ones
// downloaded from a source's catalog (Application Support/Playground/packs/<id>/r<revision>/), and
// the ones a source lists that are not here yet. The contract is docs/playground.md.
//
// Network: this file reads catalogs (`refresh`) and downloads a pack's files (`prepare`), and does
// nothing else with the network. The globe and every bundled experience never come here.

// MARK: - The contract's data

/// `prototype.json`.
nonisolated struct PackManifest: Codable, Equatable, Sendable {
    nonisolated struct Anchor: Codable, Equatable, Sendable { var lon: Double; var lat: Double; var zoom: Double? }
    nonisolated struct Marker: Codable, Equatable, Sendable {
        var id: String; var lon: Double; var lat: Double; var image: String; var width: Double; var height: Double
        var label: String; var minZoom: Double?; var maxZoom: Double?; var opens: Bool?
    }
    var id: String; var title: String; var summary: String; var runtime: Int; var presentation: String
    var entry: String?; var anchor: Anchor?; var markers: [Marker]?; var overlay: String?; var taps: Bool?
    var shelf: String?; var background: String?
}

/// `catalog.json`, as a source publishes it.
nonisolated struct PackCatalog: Codable, Sendable {
    var format: Int
    var generated: String?
    var packs: [PackEntry]
}

nonisolated struct PackEntry: Codable, Equatable, Sendable {
    nonisolated struct File: Codable, Equatable, Sendable { var path: String; var bytes: Int; var sha256: String }
    var id: String; var revision: Int; var runtime: Int; var title: String; var summary: String
    var presentation: String; var shelf: String?; var hash: String; var path: String; var bytes: Int
    var files: [File]; var published: String?; var retired: Bool?
}

/// A pack ready to open.
struct WebPack: Equatable {
    enum Origin: String, Equatable { case bundled, downloaded }
    let manifest: PackManifest
    /// The folder holding prototype.json.
    let root: URL
    let origin: Origin
    /// 0 when bundled.
    let revision: Int
    var id: String { manifest.id }
}

/// Everything the workbench shows about one pack id.
struct PackListing: Identifiable, Equatable {
    enum State: Equatable { case ready, updateAvailable(Int), downloading(Double), needsNewBuild(Int), failed(String), remoteOnly }
    let id: String
    var title: String
    var summary: String
    var presentation: String
    var shelf: String
    /// What opens now without a download (the pinned or newest installed revision, else the bundled one).
    var current: WebPack?
    var bundled: Bool
    /// Newest first.
    var installedRevisions: [Int]
    /// The newest non-retired entry across the sources.
    var remote: PackEntry?
    var state: State
    var canUsePrevious: Bool
}

/// What went wrong, in words the workbench can show.
nonisolated struct PackError: LocalizedError, Sendable {
    let message: String
    init(_ message: String) { self.message = message }
    var errorDescription: String? { message }
}

/// state.json: the reader's choice of an older revision (id -> revision, 0 meaning the bundled one).
private nonisolated struct PackState: Codable, Sendable {
    var pins: [String: Int]
}

/// `Prototypes/_runtime/sources.json`.
private nonisolated struct PackSourcesFile: Decodable, Sendable {
    nonisolated struct Item: Decodable, Sendable { var name: String?; var url: String }
    var sources: [Item]
}

// MARK: - Rules (pure, usable anywhere)

nonisolated enum PackRules {
    static let maxFiles = 500
    static let maxPackBytes = 50_000_000
    static let maxFileBytes = 25_000_000
    static let maxCatalogBytes = 2_000_000

    /// Lower-case letters, digits and hyphens, 2 to 40 characters, not starting or ending with a hyphen.
    static func isValidID(_ id: String) -> Bool {
        let bytes = Array(id.utf8)
        guard bytes.count >= 2, bytes.count <= 40 else { return false }
        guard bytes.first != 45, bytes.last != 45 else { return false }
        for byte in bytes {
            switch byte {
            case 97...122, 48...57, 45: continue
            default: return false
            }
        }
        return true
    }

    /// `^[A-Za-z0-9._-]+(/[A-Za-z0-9._-]+)*$` with no segment starting with a dot.
    static func isSafePath(_ path: String) -> Bool {
        guard !path.isEmpty, path.utf8.count <= 300 else { return false }
        for segment in path.split(separator: "/", omittingEmptySubsequences: false) {
            guard !segment.isEmpty, segment.utf8.first != 46 else { return false }
            for byte in segment.utf8 {
                switch byte {
                case 97...122, 65...90, 48...57, 45, 46, 95: continue
                default: return false
                }
            }
        }
        return true
    }

    static func isSHA256(_ text: String) -> Bool {
        guard text.utf8.count == 64 else { return false }
        for byte in text.utf8 {
            switch byte {
            case 48...57, 97...102, 65...70: continue
            default: return false
            }
        }
        return true
    }

    /// A source must be https; http only for localhost and 127.0.0.1 (the simulator and the preview).
    static func validate(source url: URL) throws {
        guard let scheme = url.scheme?.lowercased(),
              let host = url.host(percentEncoded: false)?.lowercased(), !host.isEmpty else {
            throw PackError("A source must be a full web address, like https://example.com/catalog.json.")
        }
        guard url.user(percentEncoded: false) == nil, url.password(percentEncoded: false) == nil else {
            throw PackError("A source address cannot carry a name or password.")
        }
        switch scheme {
        case "https":
            return
        case "http":
            if host == "localhost" || host == "127.0.0.1" { return }
            throw PackError("Only https sources are allowed (http works for localhost and 127.0.0.1 only).")
        default:
            throw PackError("Only https sources are allowed.")
        }
    }

    static func hex<Bytes: Sequence>(_ bytes: Bytes) -> String where Bytes.Element == UInt8 {
        let digits = Array("0123456789abcdef".utf8)
        var out: [UInt8] = []
        out.reserveCapacity(64)
        for byte in bytes {
            out.append(digits[Int(byte >> 4)])
            out.append(digits[Int(byte & 15)])
        }
        return String(decoding: out, as: UTF8.self)
    }
}

// MARK: - Disk work (off the main actor)

nonisolated enum PackDisk {
    /// The regular, non-hidden files under `root` as POSIX relative paths in byte order. Symbolic links
    /// are left out.
    static func listFiles(in root: URL) -> [String] {
        var result: [String] = []
        let keys: [URLResourceKey] = [.isRegularFileKey, .isDirectoryKey, .isSymbolicLinkKey]
        func walk(_ directory: URL, prefix: String) {
            guard let items = try? FileManager.default.contentsOfDirectory(
                at: directory, includingPropertiesForKeys: keys, options: [.skipsHiddenFiles]) else { return }
            for item in items {
                guard let values = try? item.resourceValues(forKeys: Set(keys)) else { continue }
                if values.isSymbolicLink == true { continue }
                let name = item.lastPathComponent
                if values.isDirectory == true {
                    walk(item, prefix: prefix + name + "/")
                } else if values.isRegularFile == true {
                    result.append(prefix + name)
                }
            }
        }
        walk(root, prefix: "")
        result.sort { $0.utf8.lexicographicallyPrecedes($1.utf8) }
        return result
    }

    static func sha256Hex(of url: URL) throws -> String {
        let data = try Data(contentsOf: url, options: .mappedIfSafe)
        return PackRules.hex(SHA256.hash(data: data))
    }

    /// The pack's content hash, as the publisher computes it (docs/playground.md): the SHA-256 of
    /// "<path> <sha256 of the file>\n" for each file in byte order of the path. Nil if a file cannot be read.
    @concurrent static func contentHash(of root: URL) async -> String? {
        var text = ""
        for relative in listFiles(in: root) {
            guard let digest = try? sha256Hex(of: root.appending(path: relative)) else { return nil }
            text += relative + " " + digest + "\n"
        }
        return PackRules.hex(SHA256.hash(data: Data(text.utf8)))
    }
}

// MARK: - Network work (off the main actor)

nonisolated enum PackNet {
    private static func check(_ response: URLResponse) throws {
        guard let http = response as? HTTPURLResponse else { throw PackError("the answer was not HTTP") }
        guard http.statusCode == 200 else { throw PackError("the server answered \(http.statusCode)") }
        // (a redirect may not leave https, or loopback for the simulator)
        if let landed = http.url { try PackRules.validate(source: landed) }
    }

    private static func request(_ url: URL) -> URLRequest {
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.cachePolicy = .reloadIgnoringLocalCacheData
        request.timeoutInterval = 15
        return request
    }

    @concurrent static func catalog(at url: URL, using session: URLSession) async throws -> PackCatalog {
        let (data, response) = try await session.data(for: request(url))
        try check(response)
        guard data.count <= PackRules.maxCatalogBytes else { throw PackError("the catalog is over 2 MB") }
        let catalog: PackCatalog
        do {
            catalog = try JSONDecoder().decode(PackCatalog.self, from: data)
        } catch {
            throw PackError("the catalog could not be read")
        }
        guard catalog.format == 1 else {
            throw PackError("the catalog is format \(catalog.format); this app reads format 1")
        }
        return catalog
    }

    /// Fetches one file of a pack, checks its size and SHA-256 against the catalog, and writes it
    /// under `staging` at its path.
    @concurrent static func fetch(_ url: URL, expecting file: PackEntry.File, into staging: URL,
                                  using session: URLSession) async throws {
        let (data, response) = try await session.data(for: request(url))
        do { try check(response) } catch { throw PackError("\(file.path): \(error.localizedDescription)") }
        guard data.count == file.bytes else {
            throw PackError("\(file.path): \(data.count) bytes arrived, the catalog says \(file.bytes)")
        }
        guard PackRules.hex(SHA256.hash(data: data)) == file.sha256.lowercased() else {
            throw PackError("\(file.path): does not match its checksum")
        }
        let destination = staging.appending(path: file.path)
        try FileManager.default.createDirectory(at: destination.deletingLastPathComponent(), withIntermediateDirectories: true)
        try data.write(to: destination, options: .atomic)
    }
}

// MARK: - The store

/// Finds the packs, keeps the downloaded ones, and says what the workbench should show.
/// Everything observable changes on the main actor.
@Observable
final class PackStore {
    /// The terra.js version this app hosts (Prototypes/_runtime/terra.js; docs/playground.md).
    nonisolated static let runtime = 1
    static let shared = PackStore()

    /// One per pack id, by title.
    private(set) var listings: [PackListing] = []
    /// Bundled, added and session sources, de-duplicated.
    private(set) var sources: [URL] = []
    /// The reader's own (removable), kept in UserDefaults "playground.sources".
    private(set) var addedSources: [URL] = []
    private(set) var checking = false
    private(set) var lastChecked: Date?
    /// Human-readable, from the last check or download.
    private(set) var problems: [String] = []
    /// Newest last, at most 200: terra.log lines and store events.
    private(set) var log: [String] = []

    private struct Selection {
        let pack: WebPack
        let pinned: Bool
        let index: Int
        let count: Int
    }

    private nonisolated struct SourceEntry: Sendable {
        let entry: PackEntry
        let catalog: URL
    }

    private let storage: URL
    @ObservationIgnored private var bundledSources: [URL] = []
    @ObservationIgnored private var sessionSources: [URL] = []
    @ObservationIgnored private var catalogs: [URL: PackCatalog] = [:]
    @ObservationIgnored private var remotes: [String: SourceEntry] = [:]
    @ObservationIgnored private var bundledPacks: [String: WebPack] = [:]
    /// Newest first.
    @ObservationIgnored private var installedPacks: [String: [WebPack]] = [:]
    @ObservationIgnored private var pins: [String: Int] = [:]
    @ObservationIgnored private var progress: [String: Double] = [:]
    @ObservationIgnored private var failures: [String: String] = [:]
    /// Content hashes by root path ("" when a pack could not be read).
    @ObservationIgnored private var hashes: [String: String] = [:]
    @ObservationIgnored private var hashing: Set<String> = []
    @ObservationIgnored private var installs: [String: Task<Void, any Error>] = [:]
    @ObservationIgnored private var refreshing: Task<Void, Never>?

    private static let sourcesKey = "playground.sources"
    private static let logger = Logger(subsystem: "com.pbordjadze.terraincognita", category: "playground")

    init() {
        storage = Self.makeStorage()
        loadPins()
        removeLeftovers()
        bundledPacks = scanBundled()
        installedPacks = scanAllInstalled()
        bundledSources = loadBundledSources()
        addedSources = loadAddedSources()
        #if DEBUG
        let arguments = ProcessInfo.processInfo.arguments
        if let index = arguments.firstIndex(of: "-packs-url"), index + 1 < arguments.count {
            if let url = URL(string: arguments[index + 1]) {
                do {
                    try PackRules.validate(source: url)
                    sessionSources.append(url)
                } catch {
                    record("-packs-url: \(error.localizedDescription)")
                }
            }
        }
        #endif
        rebuildSources()
        rebuild()
    }

    // MARK: log

    func record(_ line: String) {
        log.append(line)
        if log.count > 200 { log.removeFirst(log.count - 200) }
        Self.logger.info("\(line, privacy: .public)")
    }

    // MARK: sources

    /// `https`, or `http` for localhost and 127.0.0.1; anything else is refused with a readable message.
    func addSource(_ url: URL, persist: Bool = true) throws {
        try PackRules.validate(source: url)
        let text = url.absoluteString
        if persist {
            if !addedSources.contains(where: { $0.absoluteString == text }) {
                addedSources.append(url)
                saveAddedSources()
            }
            sessionSources.removeAll { $0.absoluteString == text }
        } else if !sources.contains(where: { $0.absoluteString == text }) {
            sessionSources.append(url)
        }
        rebuildSources()
        record("source added: \(text)")
    }

    /// Takes away a source the reader added (the bundled ones stay).
    func removeSource(_ url: URL) {
        let text = url.absoluteString
        addedSources.removeAll { $0.absoluteString == text }
        sessionSources.removeAll { $0.absoluteString == text }
        saveAddedSources()
        rebuildSources()
        catalogs = catalogs.filter { sources.contains($0.key) }
        mergeRemotes()
        rebuild()
        record("source removed: \(text)")
    }

    private func rebuildSources() {
        var all: [URL] = []
        for url in bundledSources + addedSources + sessionSources {
            if !all.contains(where: { $0.absoluteString == url.absoluteString }) { all.append(url) }
        }
        sources = all
    }

    private func loadBundledSources() -> [URL] {
        guard let file = Bundle.main.resourceURL?.appending(path: "Prototypes/_runtime/sources.json"),
              let data = try? Data(contentsOf: file),
              let parsed = try? JSONDecoder().decode(PackSourcesFile.self, from: data) else { return [] }
        var urls: [URL] = []
        for item in parsed.sources {
            guard let url = URL(string: item.url) else {
                record("bundled source \(item.name ?? "?") has no usable address")
                continue
            }
            do {
                try PackRules.validate(source: url)
                urls.append(url)
            } catch {
                record("bundled source \(item.name ?? url.absoluteString): \(error.localizedDescription)")
            }
        }
        return urls
    }

    private func loadAddedSources() -> [URL] {
        var urls: [URL] = []
        for text in UserDefaults.standard.stringArray(forKey: Self.sourcesKey) ?? [] {
            guard let url = URL(string: text), (try? PackRules.validate(source: url)) != nil else { continue }
            urls.append(url)
        }
        return urls
    }

    private func saveAddedSources() {
        UserDefaults.standard.set(addedSources.map { $0.absoluteString }, forKey: Self.sourcesKey)
    }

    private static func name(of url: URL) -> String {
        url.host(percentEncoded: false) ?? url.absoluteString
    }

    // MARK: refresh

    private static func makeSession() -> URLSession {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
        configuration.urlCache = nil
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.timeoutIntervalForRequest = 15
        configuration.timeoutIntervalForResource = 120
        return URLSession(configuration: configuration)
    }

    /// Reads every source's catalog (at most every 30 seconds unless forced). A check already under
    /// way is waited for, not repeated.
    func refresh(force: Bool = false) async {
        // (no sources, nothing to check: only the bundled packs)
        guard !sources.isEmpty else { return }
        if let running = refreshing {
            await running.value
            return
        }
        if !force, let last = lastChecked, Date().timeIntervalSince(last) < 30 { return }
        let task = Task { [self] in
            await self.performRefresh()
            self.refreshing = nil
        }
        refreshing = task
        await task.value
    }

    private func performRefresh() async {
        checking = true
        problems = []
        let targets = sources
        let session = Self.makeSession()
        for url in targets {
            do {
                let fetched = try await PackNet.catalog(at: url, using: session)
                catalogs[url] = fetched
            } catch {
                problems.append("\(Self.name(of: url)): \(error.localizedDescription)")
            }
        }
        session.finishTasksAndInvalidate()
        catalogs = catalogs.filter { targets.contains($0.key) }
        failures = [:]
        lastChecked = Date()
        checking = false
        mergeRemotes()
        rebuild()
        let sourceWord = targets.count == 1 ? "source" : "sources"
        let problemNote = problems.isEmpty ? "" : ", \(problems.count) failed"
        record("checked \(targets.count) \(sourceWord)\(problemNote)")
    }

    /// The newest non-retired entry for each id across the sources' catalogs.
    private func mergeRemotes() {
        var best: [String: SourceEntry] = [:]
        for source in sources {
            guard let catalog = catalogs[source] else { continue }
            for entry in catalog.packs where entry.retired != true && PackRules.isValidID(entry.id) && entry.revision >= 1 {
                if let have = best[entry.id], have.entry.revision >= entry.revision { continue }
                best[entry.id] = SourceEntry(entry: entry, catalog: source)
            }
        }
        remotes = best
    }

    // MARK: choosing a revision

    /// Installed revisions, newest first, then the bundled one.
    private func candidates(_ id: String) -> [WebPack] {
        var list = installedPacks[id] ?? []
        if let bundled = bundledPacks[id] { list.append(bundled) }
        return list
    }

    private func selection(_ id: String) -> Selection? {
        let list = candidates(id)
        guard let newest = list.first else { return nil }
        if let pin = pins[id], let index = list.firstIndex(where: { $0.revision == pin }) {
            return Selection(pack: list[index], pinned: index != 0, index: index, count: list.count)
        }
        return Selection(pack: newest, pinned: false, index: 0, count: list.count)
    }

    /// Steps the choice down: a newer installed revision, then an older one, then the bundled one.
    func usePrevious(_ id: String) {
        guard let choice = selection(id), choice.index + 1 < choice.count else { return }
        let revision = candidates(id)[choice.index + 1].revision
        pins[id] = revision
        savePins()
        rebuild()
        record(revision == 0 ? "\(id): using the bundled revision" : "\(id): using r\(revision)")
    }

    /// Clears the choice: the newest revision opens again.
    func useNewest(_ id: String) {
        guard pins[id] != nil else { return }
        pins[id] = nil
        savePins()
        rebuild()
        record("\(id): using the newest revision")
    }

    /// Deletes every downloaded revision of a pack (the bundled one stays).
    func removeDownloads(_ id: String) {
        guard PackRules.isValidID(id), installs[id] == nil else { return }
        try? FileManager.default.removeItem(at: packsDirectory.appending(path: id, directoryHint: .isDirectory))
        for pack in installedPacks[id] ?? [] { hashes[Self.key(pack.root)] = nil }
        installedPacks[id] = nil
        pins[id] = nil
        savePins()
        rebuild()
        record("\(id): downloads removed")
    }

    // MARK: listings

    private func rebuild() {
        var ids = Set(bundledPacks.keys)
        ids.formUnion(installedPacks.keys)
        ids.formUnion(remotes.keys)
        var result: [PackListing] = []
        for id in ids {
            let choice = selection(id)
            let current = choice?.pack
            let remote = remotes[id]?.entry
            let manifest = current?.manifest
            result.append(PackListing(
                id: id,
                title: manifest?.title ?? remote?.title ?? id,
                summary: manifest?.summary ?? remote?.summary ?? "",
                presentation: manifest?.presentation ?? remote?.presentation ?? "screen",
                shelf: manifest?.shelf ?? remote?.shelf ?? "current",
                current: current,
                bundled: bundledPacks[id] != nil,
                installedRevisions: (installedPacks[id] ?? []).map { $0.revision },
                remote: remote,
                state: state(for: id, current: current, pinned: choice?.pinned ?? false, remote: remote),
                canUsePrevious: choice.map { $0.index + 1 < $0.count } ?? false))
        }
        result.sort {
            let order = $0.title.localizedCaseInsensitiveCompare($1.title)
            return order == .orderedSame ? $0.id < $1.id : order == .orderedAscending
        }
        if result != listings { listings = result }
        warmHashes()
    }

    private func state(for id: String, current: WebPack?, pinned: Bool, remote: PackEntry?) -> PackListing.State {
        if let fraction = progress[id] { return .downloading(fraction) }
        if let remote, remote.runtime > Self.runtime { return .needsNewBuild(remote.runtime) }
        guard let current else {
            if let message = failures[id] { return .failed(message) }
            return remote == nil ? .failed("Nothing to open.") : .remoteOnly
        }
        // (a reader who stepped back to an older revision is not nagged to update)
        if !pinned, let remote, isUpdate(remote, over: current) { return .updateAvailable(remote.revision) }
        return .ready
    }

    /// Whether the remote entry differs from what opens now. Unknown (the hash is still being
    /// computed) counts as no, and the listing is rebuilt when the hash arrives.
    private func isUpdate(_ remote: PackEntry, over current: WebPack) -> Bool {
        guard remote.runtime <= Self.runtime else { return false }
        if current.origin == .downloaded, remote.revision <= current.revision { return false }
        guard let known = hashes[Self.key(current.root)] else { return false }
        return known != remote.hash
    }

    // MARK: content hashes

    private static func key(_ root: URL) -> String { root.path(percentEncoded: false) }

    private func warmHashes() {
        for listing in listings {
            guard let current = listing.current, let remote = listing.remote, remote.runtime <= Self.runtime else { continue }
            if current.origin == .downloaded, remote.revision <= current.revision { continue }
            startHashing(current.root)
        }
    }

    private func startHashing(_ root: URL) {
        let key = Self.key(root)
        guard hashes[key] == nil, !hashing.contains(key) else { return }
        hashing.insert(key)
        Task { [self] in
            let value = await PackDisk.contentHash(of: root) ?? ""
            self.hashing.remove(key)
            self.hashes[key] = value
            self.rebuild()
        }
    }

    private func contentHash(of root: URL) async -> String {
        let key = Self.key(root)
        if let known = hashes[key] { return known }
        let value = await PackDisk.contentHash(of: root) ?? ""
        hashes[key] = value
        return value
    }

    // MARK: prepare

    /// The newest usable revision of a pack, downloading it first if a compatible newer one is listed.
    /// A download that fails leaves what was there; it throws only when there is nothing to open.
    func prepare(_ id: String) async throws -> WebPack {
        guard PackRules.isValidID(id) else { throw PackError("“\(id)” is not a prototype id.") }
        failures[id] = nil
        if let remote = remotes[id] {
            if remote.entry.runtime > Self.runtime {
                if selection(id) == nil {
                    throw PackError("“\(remote.entry.title)” needs a newer build of the app (it uses runtime \(remote.entry.runtime); this one has \(Self.runtime)).")
                }
            } else if await wantsDownload(remote.entry) {
                do {
                    try await install(remote)
                } catch {
                    let reason = error.localizedDescription
                    problems.append("\(id): \(reason)")
                    record("\(id) r\(remote.entry.revision) could not be installed: \(reason)")
                    if selection(id) == nil {
                        failures[id] = reason
                        rebuild()
                        throw PackError(reason)
                    }
                }
            }
        }
        guard let choice = selection(id) else {
            throw PackError("“\(id)” is not on this device, and no source lists it.")
        }
        let manifest = try Self.loadManifest(at: choice.pack.root, expecting: id)
        return WebPack(manifest: manifest, root: choice.pack.root, origin: choice.pack.origin, revision: choice.pack.revision)
    }

    private func wantsDownload(_ entry: PackEntry) async -> Bool {
        guard let choice = selection(entry.id) else { return true }
        if choice.pinned { return false }
        let current = choice.pack
        if current.origin == .downloaded, entry.revision <= current.revision { return false }
        return await contentHash(of: current.root) != entry.hash
    }

    private func install(_ remote: SourceEntry) async throws {
        let id = remote.entry.id
        if let running = installs[id] {
            try await running.value
            return
        }
        let task = Task { [self] in
            try await self.performInstall(remote)
        }
        installs[id] = task
        defer { installs[id] = nil }
        try await task.value
    }

    private func performInstall(_ remote: SourceEntry) async throws {
        let entry = remote.entry
        let id = entry.id
        let files = try Self.validatedFiles(of: entry)
        let base = try Self.baseURL(of: entry, catalog: remote.catalog)
        let folder = packsDirectory.appending(path: id, directoryHint: .isDirectory)
        let staging = folder.appending(path: ".staging-\(UUID().uuidString)", directoryHint: .isDirectory)
        progress[id] = 0
        rebuild()
        let session = Self.makeSession()
        defer { session.finishTasksAndInvalidate() }
        do {
            try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
            let total = max(1, files.reduce(0) { $0 + $1.bytes })
            var done = 0
            for file in files {
                let url = try Self.fileURL(file.path, in: base)
                try await PackNet.fetch(url, expecting: file, into: staging, using: session)
                done += file.bytes
                progress[id] = Double(done) / Double(total)
                rebuild()
            }
            // it must be this pack, in a runtime this app hosts
            _ = try Self.loadManifest(at: staging, expecting: id)
            let destination = folder.appending(path: "r\(entry.revision)", directoryHint: .isDirectory)
            if FileManager.default.fileExists(atPath: destination.path(percentEncoded: false)) {
                try? FileManager.default.removeItem(at: staging)
            } else {
                try FileManager.default.moveItem(at: staging, to: destination)
            }
        } catch {
            try? FileManager.default.removeItem(at: staging)
            progress[id] = nil
            rebuild()
            throw error
        }
        installedPacks[id] = scanInstalled(id)
        pins[id] = nil
        savePins()
        prune(id)
        progress[id] = nil
        rebuild()
        record("\(id) r\(entry.revision) installed")
    }

    /// Keeps the current and the one before; deletes older ones.
    private func prune(_ id: String) {
        guard let list = installedPacks[id], list.count > 2 else { return }
        for old in list.dropFirst(2) {
            try? FileManager.default.removeItem(at: old.root)
            hashes[Self.key(old.root)] = nil
        }
        installedPacks[id] = Array(list.prefix(2))
    }

    // MARK: entry checks

    /// The files of an entry, after refusing what a pack may not contain.
    nonisolated static func validatedFiles(of entry: PackEntry) throws -> [PackEntry.File] {
        guard PackRules.isValidID(entry.id), entry.revision >= 1, entry.revision <= 1_000_000 else {
            throw PackError("the catalog's entry for “\(entry.id)” is not valid")
        }
        guard !entry.files.isEmpty, entry.files.count <= PackRules.maxFiles else {
            throw PackError("a pack may have 1 to \(PackRules.maxFiles) files; this one has \(entry.files.count)")
        }
        var seen = Set<String>()
        var total = 0
        for file in entry.files {
            guard PackRules.isSafePath(file.path) else { throw PackError("“\(file.path)” is not a path a pack may contain") }
            guard seen.insert(file.path.lowercased()).inserted else { throw PackError("“\(file.path)” is listed twice") }
            guard file.bytes >= 0, file.bytes <= PackRules.maxFileBytes else {
                throw PackError("“\(file.path)” is over the \(PackRules.maxFileBytes / 1_000_000) MB file limit")
            }
            guard PackRules.isSHA256(file.sha256) else { throw PackError("“\(file.path)” has no valid checksum") }
            total += file.bytes
            guard total <= PackRules.maxPackBytes else {
                throw PackError("the pack is over the \(PackRules.maxPackBytes / 1_000_000) MB limit")
            }
        }
        guard seen.contains("prototype.json") else { throw PackError("the pack has no prototype.json") }
        return entry.files
    }

    /// The folder URL of a pack's files: the entry's `path`, relative to its catalog.
    nonisolated static func baseURL(of entry: PackEntry, catalog: URL) throws -> URL {
        var path = entry.path
        if !path.hasSuffix("/") { path += "/" }
        guard PackRules.isSafePath(String(path.dropLast())) else {
            throw PackError("“\(entry.path)” is not a folder a pack may live in")
        }
        guard let url = URL(string: path, relativeTo: catalog)?.absoluteURL else {
            throw PackError("the pack's address could not be made")
        }
        try PackRules.validate(source: url)
        return url
    }

    nonisolated static func fileURL(_ path: String, in base: URL) throws -> URL {
        let encoded = path.split(separator: "/", omittingEmptySubsequences: false).map { segment -> String in
            let text = String(segment)
            return text.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? text
        }.joined(separator: "/")
        guard let url = URL(string: encoded, relativeTo: base)?.absoluteURL else {
            throw PackError("the address of “\(path)” could not be made")
        }
        return url
    }

    // MARK: manifests and folders

    /// Reads a pack's prototype.json. Its id must be the folder's, its runtime one this app hosts,
    /// and every path it names must be inside the pack.
    nonisolated static func loadManifest(at root: URL, expecting id: String) throws -> PackManifest {
        guard let data = try? Data(contentsOf: root.appending(path: "prototype.json")) else {
            throw PackError("prototype.json is missing")
        }
        let manifest: PackManifest
        do {
            manifest = try JSONDecoder().decode(PackManifest.self, from: data)
        } catch {
            throw PackError("prototype.json could not be read")
        }
        guard manifest.id == id else { throw PackError("prototype.json names “\(manifest.id)”, not “\(id)”") }
        guard manifest.runtime >= 1, manifest.runtime <= runtime else {
            throw PackError("it needs runtime \(manifest.runtime); this app has \(runtime)")
        }
        guard manifest.presentation == "screen" || manifest.presentation == "globe" else {
            throw PackError("presentation must be “screen” or “globe”")
        }
        var named: [String] = []
        if let entry = manifest.entry { named.append(entry) }
        if let overlay = manifest.overlay { named.append(overlay) }
        for marker in manifest.markers ?? [] { named.append(marker.image) }
        for path in named where !PackRules.isSafePath(path) {
            throw PackError("“\(path)” is not a path inside the pack")
        }
        return manifest
    }

    private var packsDirectory: URL { storage.appending(path: "packs", directoryHint: .isDirectory) }
    private var stateURL: URL { storage.appending(path: "state.json") }

    private static func makeStorage() -> URL {
        let manager = FileManager.default
        let support = (try? manager.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true))
            ?? manager.temporaryDirectory
        var folder = support.appending(path: "Playground", directoryHint: .isDirectory)
        try? manager.createDirectory(at: folder, withIntermediateDirectories: true)
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        try? folder.setResourceValues(values)
        try? manager.createDirectory(at: folder.appending(path: "packs", directoryHint: .isDirectory), withIntermediateDirectories: true)
        return folder
    }

    private func loadPins() {
        guard let data = try? Data(contentsOf: stateURL),
              let state = try? JSONDecoder().decode(PackState.self, from: data) else { return }
        pins = state.pins
    }

    private func savePins() {
        guard let data = try? JSONEncoder().encode(PackState(pins: pins)) else { return }
        try? data.write(to: stateURL, options: .atomic)
    }

    /// Staging folders a crash left behind.
    private func removeLeftovers() {
        let manager = FileManager.default
        let ids = (try? manager.contentsOfDirectory(atPath: packsDirectory.path(percentEncoded: false))) ?? []
        for id in ids {
            let folder = packsDirectory.appending(path: id, directoryHint: .isDirectory)
            for name in (try? manager.contentsOfDirectory(atPath: folder.path(percentEncoded: false))) ?? [] where name.hasPrefix(".staging-") {
                try? manager.removeItem(at: folder.appending(path: name, directoryHint: .isDirectory))
            }
        }
    }

    /// Prototypes/<id>/ in the app bundle (`_runtime` is the host's own, not a pack).
    private func scanBundled() -> [String: WebPack] {
        var found: [String: WebPack] = [:]
        guard let base = Bundle.main.resourceURL?.appending(path: "Prototypes", directoryHint: .isDirectory),
              let items = try? FileManager.default.contentsOfDirectory(
                at: base, includingPropertiesForKeys: [.isDirectoryKey], options: [.skipsHiddenFiles]) else { return found }
        for item in items {
            let name = item.lastPathComponent
            guard !name.hasPrefix("_"), PackRules.isValidID(name),
                  (try? item.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory == true else { continue }
            do {
                let manifest = try Self.loadManifest(at: item, expecting: name)
                found[name] = WebPack(manifest: manifest, root: item, origin: .bundled, revision: 0)
            } catch {
                record("bundled pack \(name) is unusable: \(error.localizedDescription)")
            }
        }
        return found
    }

    /// The `r<N>` folders of one pack, newest first. Only a complete download ever has that name.
    private func scanInstalled(_ id: String) -> [WebPack] {
        let folder = packsDirectory.appending(path: id, directoryHint: .isDirectory)
        let names = (try? FileManager.default.contentsOfDirectory(atPath: folder.path(percentEncoded: false))) ?? []
        var packs: [WebPack] = []
        for name in names {
            guard let revision = Self.revision(ofFolder: name) else { continue }
            let root = folder.appending(path: name, directoryHint: .isDirectory)
            do {
                let manifest = try Self.loadManifest(at: root, expecting: id)
                packs.append(WebPack(manifest: manifest, root: root, origin: .downloaded, revision: revision))
            } catch {
                record("\(id) r\(revision) is unusable: \(error.localizedDescription)")
            }
        }
        packs.sort { $0.revision > $1.revision }
        return packs
    }

    private func scanAllInstalled() -> [String: [WebPack]] {
        var found: [String: [WebPack]] = [:]
        let ids = (try? FileManager.default.contentsOfDirectory(atPath: packsDirectory.path(percentEncoded: false))) ?? []
        for id in ids where PackRules.isValidID(id) {
            let packs = scanInstalled(id)
            if !packs.isEmpty { found[id] = packs }
        }
        return found
    }

    /// "r12" is revision 12; anything else (".staging-…", "r0", "r+3") is not a revision.
    private nonisolated static func revision(ofFolder name: String) -> Int? {
        guard name.hasPrefix("r") else { return nil }
        let digits = name.dropFirst()
        guard !digits.isEmpty, digits.utf8.count <= 7, digits.utf8.allSatisfy({ $0 >= 48 && $0 <= 57 }),
              let value = Int(digits), value >= 1 else { return nil }
        return value
    }
}
