import XCTest

/// Runtime 2 and on: packs whose geography, date, words and (runtime 5) moving things the globe itself
/// draws. Each test starts one from the workbench, moves its date, and ends it, after which nothing it
/// asked for is left.
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
        // (the foot's first line is set in capitals: look for its words below it)
        XCTAssertTrue(text(containing: "explore the deep ocean", in: app).waitForExistence(timeout: 15), "The foot says what this is")
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

    /// The herds (runtime 5): animals on dated ways, which the globe's date moves. It opens close on one
    /// of the first camelids and pulls back; moving the date to its next mark moves that animal, and
    /// the date reads as years ago; then the plain globe again.
    @MainActor func testTheHerdsMoveWithTheDate() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("herds", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The herds show the globe's date")
        XCTAssertTrue((date.value as? String ?? "").contains("million years ago"), "It starts in deep time, told as years ago")
        // the opening: close on the first camelid (drawn large), then out to see its herd
        let one = app.buttons["herds.first-1"].firstMatch
        XCTAssertTrue(one.waitForExistence(timeout: 20), "The first camelid is on the globe, touchable")
        attach("57-herds-close", in: app)
        Thread.sleep(forTimeInterval: 9)
        attach("58-herds-herd", in: app)
        let play = app.buttons["globeDatePlay"].firstMatch
        if play.exists, play.label == "Pause" { play.tap() }
        XCTAssertTrue(one.waitForExistence(timeout: 10), "Pulled back, the first camelid is still in view, with its herd")
        let before = one.frame
        let label = date.value as? String
        // the next mark on the strip (where the evidence is): the herd has walked on
        date.increment()
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value != %@", label ?? ""), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [moved], timeout: 10), .completed, "The date moves to its next mark")
        Thread.sleep(forTimeInterval: 1)
        attach("59-herds-later", in: app)
        if one.exists {
            let after = one.frame
            XCTAssertGreaterThan(hypot(after.midX - before.midX, after.midY - before.midY), 8, "The animal moved with the date")
        } else {
            XCTAssertTrue(app.buttons.matching(NSPredicate(format: "identifier BEGINSWITH 'herds.'")).firstMatch.exists,
                          "The herds are still on the globe after the date moved")
        }
        endPrototype(in: app)
        XCTAssertFalse(app.descendants(matching: .any)["globeDate"].firstMatch.exists, "The date goes with the herds")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the herds leaves only the plain globe")
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
