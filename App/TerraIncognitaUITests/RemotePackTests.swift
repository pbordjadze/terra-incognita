import XCTest

/// A compatible change reaching the installed app without a new build: CI serves a catalog whose
/// hello-pack differs from the bundled copy (ci/remote_packs.sh, tools/playground/serve.cjs), and
/// passes its URL as TEST_RUNNER_PACKS_URL. The app downloads it, opens it, rolls back, rolls forward,
/// and still opens it when the source is unreachable.
final class RemotePackTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    @MainActor func testRemoteRevisionArrivesRollsBackAndWorksOffline() throws {
        guard let source = ProcessInfo.processInfo.environment["PACKS_URL"], !source.isEmpty else {
            throw XCTSkip("No local pack server (CI sets TEST_RUNNER_PACKS_URL)")
        }
        let app = XCUIApplication()
        app.launchArguments = ["-packs-url", source, "-playground-reset"]
        app.launch()

        openWorkbench(in: app)
        let row = workbenchItem("hello-pack", in: app)
        let listed = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS %@", "available"), object: row)
        XCTAssertEqual(XCTWaiter().wait(for: [listed], timeout: 30), .completed, "The workbench sees the newer revision: \(row.label)")
        attach("40-update-listed", in: app)
        row.tap()

        // it downloads, starts on the globe, and its page is the published one
        let open = app.buttons["openActivePrototype"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 45))
        open.tap()
        pageShows("Second words.", "The remote revision's page opens", in: app)
        attach("41-remote-revision", in: app)
        app.buttons["packBack"].firstMatch.tap()

        // back to the bundled copy, and forward again
        openWorkbench(in: app)
        let previous = app.buttons["previous_hello-pack"].firstMatch
        revealInWorkbench(previous, in: app)
        previous.tap()
        let revision = app.staticTexts["revision_hello-pack"].firstMatch
        let rolledBack = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label CONTAINS %@", "bundled"), object: revision)
        XCTAssertEqual(XCTWaiter().wait(for: [rolledBack], timeout: 10), .completed, revision.label)
        app.buttons["workbenchClose"].firstMatch.tap()
        startPrototype("hello-pack", open: true, in: app)
        pageShows("First words.", "Previous restores the bundled page", in: app)
        app.buttons["packBack"].firstMatch.tap()
        openWorkbench(in: app)
        let newest = app.buttons["newest_hello-pack"].firstMatch
        revealInWorkbench(newest, in: app)
        newest.tap()
        app.buttons["workbenchClose"].firstMatch.tap()
        app.terminate()

        // the source gone: the downloaded revision still opens
        app.launchArguments = ["-packs-url", "http://127.0.0.1:9/catalog.json"]
        app.launch()
        defer { app.terminate() }
        startPrototype("hello-pack", open: true, in: app)
        pageShows("Second words.", "Offline, the installed revision opens", in: app)
        attach("42-offline", in: app)
    }

    /// The page says this within a minute; if not, what the screen showed is kept with the failure.
    @MainActor private func pageShows(_ words: String, _ message: String, in app: XCUIApplication,
                                      file: StaticString = #filePath, line: UInt = #line) {
        let shown = webText(containing: words, in: app).waitForExistence(timeout: 60)
        if !shown { attach("failed-\(words)", in: app) }
        XCTAssertTrue(shown, message, file: file, line: line)
    }
}
