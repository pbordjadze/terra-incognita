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

    var summary: String {
        LiveKind.allCases.map { "\($0.rawValue) \(counts[$0, default: 0])" }.joined(separator: " · ")
    }

    fileprivate func change(_ kind: LiveKind, by delta: Int) {
        counts[kind, default: 0] += delta
    }

    /// Counted while it lives: keep one as a stored property of the thing counted. (Its deinit may
    /// run anywhere; the count changes on the main actor a moment later.)
    nonisolated final class Token: Sendable {
        let kind: LiveKind
        init(_ kind: LiveKind) {
            self.kind = kind
            Task { @MainActor in LiveCount.shared.change(kind, by: 1) }
        }
        deinit {
            let kind = kind
            Task { @MainActor in LiveCount.shared.change(kind, by: -1) }
        }
    }
}
