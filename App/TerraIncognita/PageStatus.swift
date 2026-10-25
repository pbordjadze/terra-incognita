import Foundation
import Observation
import SwiftUI
import os

/// How the page's start is going, from its reports (`TI.log` in `Web/index.html`), which also go
/// to the system log (subsystem com.pbordjadze.terraincognita, category page).
@Observable
final class PageStatus {
    static let log = Logger(subsystem: "com.pbordjadze.terraincognita", category: "page")

    private(set) var line = ""
    private(set) var ready = false
    private(set) var startedAt = Date()
    private(set) var starts = 0

    func report(_ line: String) {
        Self.log.notice("\(line, privacy: .public)")
        self.line = line
        if line.contains(" s page ") { ready = false; startedAt = Date(); starts += 1 }
        if line.hasSuffix(" ready") { ready = true }
    }
}

/// The last step of a start that is taking too long, small at the foot of the screen. It is drawn
/// by the app, not the page, so it stays up to date when the web view's graphics stop.
struct StartLine: View {
    let status: PageStatus

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            if !status.ready, context.date.timeIntervalSince(status.startedAt) > 6 {
                Text(verbatim: message)
                    .font(.caption.monospaced())
                    .foregroundStyle(Color(red: 0.89, green: 0.75, blue: 0.37).opacity(0.85))
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 24)
                    .padding(.bottom, 12)
            }
        }
        .allowsHitTesting(false)
    }

    private var message: String {
        let step = status.line.isEmpty ? "no word from the page yet" : "last step: \(status.line)"
        return status.starts > 1 ? "Still starting (start \(status.starts)), \(step)" : "Still starting, \(step)"
    }
}
