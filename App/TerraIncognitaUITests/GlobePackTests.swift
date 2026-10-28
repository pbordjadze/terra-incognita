import XCTest

/// Runtime 2: packs whose geography, date and words the globe itself draws. Each test starts one from
/// the workbench, moves its date, and ends it, after which nothing it asked for is left.
final class GlobePackTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    /// The Challenger's voyage: its date and words at the foot, the date moved along its strip, then
    /// the plain globe again.
    @MainActor func testTheVoyageIsDrawnByTheGlobe() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("sounding-the-deep", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The globe shows the voyage's date")
        XCTAssertTrue(text(containing: "HMS Challenger", in: app).waitForExistence(timeout: 15), "The foot says what this is")
        attach("50-voyage-far", in: app)
        // closer (a double tap doubles the zoom where it lands): the stations and passages appear
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        globe.doubleTap()
        XCTAssertTrue(text(containing: "Each dot is a station", in: app).waitForExistence(timeout: 10), "Closer, the foot explains the dots")
        Thread.sleep(forTimeInterval: 1.5)
        attach("52-voyage-near", in: app)
        let play = app.buttons["globeDatePlay"].firstMatch
        XCTAssertTrue(play.exists, "The voyage can play")
        if play.label == "Pause" { play.tap() }
        let before = date.value as? String
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.5)).tap()
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value != %@", before ?? ""), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [moved], timeout: 10), .completed, "Touching the strip moves the date")
        attach("51-voyage-moved", in: app)
        endPrototype(in: app)
        XCTAssertFalse(app.descendants(matching: .any)["globeDate"].firstMatch.exists, "The date goes with the prototype")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the voyage leaves only the plain globe")
    }

    /// The old maps: Poland on the 1945 map to start, an early map with no Poland, then the plain globe.
    @MainActor func testOldMapsChangeWithTheDate() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("borders-on-old-maps", in: app)
        XCTAssertTrue(text(containing: "the 1945 map", in: app).waitForExistence(timeout: 30), "It opens on the 1945 map")
        XCTAssertTrue(text(containing: "Poland", in: app).exists, "with Poland chosen")
        attach("55-old-maps-1945", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.exists)
        // near the strip's start: the 1530 map, which has no Poland
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.05, dy: 0.5)).tap()
        XCTAssertTrue(text(containing: "no Poland", in: app).waitForExistence(timeout: 10), "An early map says it has no Poland")
        attach("56-old-maps-early", in: app)
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the old maps leaves only the plain globe")
    }
}
