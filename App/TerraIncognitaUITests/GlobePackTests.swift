import XCTest

/// Packs whose date the globe keeps at the foot (runtime 2 and after): each starts from the workbench,
/// shows its date, has it moved along its strip, and ends, after which nothing it asked for is left.
final class GlobePackTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    /// The lead line: HMS Challenger's voyage dated at the foot, playing; the date moved along its strip;
    /// then the plain globe again.
    @MainActor func testTheVoyageIsDrawnByTheGlobe() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("lead-line", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The globe shows the voyage's date")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("lead line"), "The lead line is in front")
        attach("50-voyage-far", in: app)
        let play = app.buttons["globeDatePlay"].firstMatch
        XCTAssertTrue(play.exists, "The voyage can play")
        if play.label == "Pause" { play.tap() }
        let before = date.value as? String
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.5)).tap()
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value != %@", before ?? ""), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [moved], timeout: 10), .completed, "Touching the strip moves the date")
        Thread.sleep(forTimeInterval: 1.5)
        attach("51-voyage-moved", in: app)
        endPrototype(in: app)
        XCTAssertFalse(app.descendants(matching: .any)["globeDate"].firstMatch.exists, "The date goes with the prototype")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the voyage leaves only the plain globe")
    }

    /// The vine: eight thousand years at the foot, from 6000 BCE; the years moved near the strip's end,
    /// where the vine has reached the whole world; then the plain globe.
    @MainActor func testTheVineGrowsWithTheYears() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("vine", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The globe shows the vine's year")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("vine"), "The vine is in front")
        let play = app.buttons["globeDatePlay"].firstMatch
        if play.exists && play.label == "Pause" { play.tap() }
        attach("55-vine-begins", in: app)
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.97, dy: 0.5)).tap()
        let late = XCTNSPredicateExpectation(predicate: NSPredicate(format: "NOT (value CONTAINS %@)", "BCE"), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [late], timeout: 10), .completed, "Near the strip's end the year is in our era")
        Thread.sleep(forTimeInterval: 1.5)
        attach("56-vine-today", in: app)
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the vine leaves only the plain globe")
    }
}
