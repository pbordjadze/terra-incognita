import XCTest

/// The `hold` scheme and the light running along lines (runtime 4, October 29), through A gathering of water,
/// alive: a plain drag across the rivers turns the globe and lights nothing; a finger that rests a moment, then
/// draws across them, lights them all, and the foot names where their water goes; ending it leaves nothing.
final class RiverPackTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    @MainActor func testARestingFingerDrawsAndADragTurns() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("gathering-of-water-alive", in: app)
        let foot = app.descendants(matching: .any)["globeFootCaption"].firstMatch
        XCTAssertTrue(foot.waitForExistence(timeout: 30), "the rivers say at the foot what this is")
        Thread.sleep(forTimeInterval: 2.5)              // the flight to the rivers, and the network drawn
        let idle = foot.label
        // (the foot's title reads in capitals)
        XCTAssertTrue(idle.lowercased().contains("a gathering of water"), "the foot names it first (got “\(idle)”)")
        attach("66-gathering-of-water-alive-on-globe", in: app)

        // a plain drag across the middle of the basin: the globe's, so nothing is lit
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        globe.coordinate(withNormalizedOffset: CGVector(dx: 0.28, dy: 0.42))
            .press(forDuration: 0.05, thenDragTo: globe.coordinate(withNormalizedOffset: CGVector(dx: 0.62, dy: 0.5)))
        Thread.sleep(forTimeInterval: 1.5)
        XCTAssertEqual(foot.label, idle, "a plain drag turns the globe and lights no river")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.exists, "the rivers still run after a drag")

        // a finger that rests, then draws back across them: the pack's, so they light, all at once
        globe.coordinate(withNormalizedOffset: CGVector(dx: 0.66, dy: 0.5))
            .press(forDuration: 0.8, thenDragTo: globe.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.42)))
        Thread.sleep(forTimeInterval: 3)                // the light running down to the sea
        XCTAssertTrue(foot.label.lowercased().contains("toward"),
                      "drawing across the rivers lights them and names where their water goes (got “\(foot.label)”)")
        XCTAssertFalse(foot.label.contains(" km"), "the foot carries a name, not the numbers")
        attach("66-gathering-of-water-alive-drawn", in: app)

        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the rivers leaves nothing they made")
    }
}
