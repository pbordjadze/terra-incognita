import Foundation
import Observation

/// What the workbench counts: the resources a prototype makes and must give back when it ends.
nonisolated enum LiveKind: String, CaseIterable, Sendable {
    case stages, markers, globes, films, scripts
}

/// How many of each kind exist now, for the workbench's leak check: once a prototype has ended,
/// every count is back where the plain globe has it (one globe, nothing else).
@Observable
final class LiveCount {
    static let shared = LiveCount()
    private(set) var counts: [LiveKind: Int] = [:]
    /// What the labelled tokens alive are (a stage's prototype id), for finding what a leak is.
    private(set) var labels: [ObjectIdentifier: String] = [:]

    var summary: String {
        LiveKind.allCases.map { "\($0.rawValue) \(counts[$0, default: 0])" }.joined(separator: " · ")
    }

    /// The labelled things alive, by name: "alive: herds, vine" (empty when nothing labelled lives).
    var detail: String {
        let names = labels.values.sorted()
        return names.isEmpty ? "" : "alive: " + names.joined(separator: ", ")
    }

    fileprivate func change(_ kind: LiveKind, by delta: Int, token: ObjectIdentifier, label: String?) {
        counts[kind, default: 0] += delta
        if let label { if delta > 0 { labels[token] = label } else { labels[token] = nil } }
    }

    /// Counted while it lives: keep one as a stored property of the thing counted. (Its deinit may
    /// run anywhere; the count changes on the main actor a moment later.)
    nonisolated final class Token: Sendable {
        let kind: LiveKind
        /// A name for the thing counted (a stage's prototype id), so the workbench can say what is alive.
        let label: String?
        init(_ kind: LiveKind, label: String? = nil) {
            self.kind = kind
            self.label = label
            let id = ObjectIdentifier(self)
            Task { @MainActor in LiveCount.shared.change(kind, by: 1, token: id, label: label) }
        }
        deinit {
            let kind = kind, label = label, id = ObjectIdentifier(self)
            Task { @MainActor in LiveCount.shared.change(kind, by: -1, token: id, label: label) }
        }
    }
}
