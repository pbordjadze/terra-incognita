import XCTest

/// Packs whose date the globe keeps at the foot (runtime 2 and after; runtime 5 moves things with it): each
/// starts from the workbench, shows its date, has it moved along its strip, and ends, after which nothing it
/// asked for is left.
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

    /// The herds (runtime 5): animals on dated ways, which the globe's date moves. It opens close on one
    /// of the first camelids and pulls back; moving the date to a mark on its strip moves that animal,
    /// and the date reads as years ago; then the plain globe again.
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
        // the strip's mark at 21 million years ago (a touch within a fingertip of a mark lands on it): the
        // herd has walked on, east across the plains
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.126, dy: 0.5)).tap()
        let moved = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value != %@", label ?? ""), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [moved], timeout: 10), .completed, "Touching the strip moves the date")
        XCTAssertTrue((date.value as? String ?? "").contains("million years ago"), "Still in deep time, told as years ago")
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

    /// The night crossing (runtime 5, zoomed deeper than any pack before it): the night of 326 BCE at the
    /// foot; a touch on the near camp and the column peels away; a drag on the column along its way moves
    /// the night under the finger, so the column is elsewhere; the strip moved to the second channel, where
    /// the soldiers wade; then the plain globe again.
    @MainActor func testTheNightCrossingMovesUnderTheFinger() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("hydaspes", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The night crossing shows its date")
        XCTAssertEqual(date.value as? String, "326 BCE", "The date reads the year of the crossing")
        let camp = app.buttons["hydaspes.camp-a"].firstMatch
        XCTAssertTrue(camp.waitForExistence(timeout: 20), "The near camp is on the globe, touchable")
        Thread.sleep(forTimeInterval: 2.5)
        attach("60-night-camps", in: app)
        camp.tap()
        let column = app.buttons["hydaspes.column"].firstMatch
        XCTAssertTrue(column.waitForExistence(timeout: 10), "Touching the camp: the column peels away from it")
        let before = column.frame
        // a drag on the column, down the valley the way it marches (south-west on the screen): the night
        // moves under the finger and the column with it
        column.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
            .press(forDuration: 0.2, thenDragTo: column.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5))
                .withOffset(CGVector(dx: -140, dy: 110)))
        Thread.sleep(forTimeInterval: 1)
        attach("61-night-march", in: app)
        if column.exists {
            let after = column.frame
            XCTAssertGreaterThan(hypot(after.midX - before.midX, after.midY - before.midY), 20, "The column moved with the drag")
        }
        XCTAssertEqual(date.value as? String, "326 BCE", "The year is the same all night")
        // the strip moved to the second channel: the soldiers wade it to the chest
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.6, dy: 0.5)).tap()
        let waders = app.buttons["hydaspes.waders"].firstMatch
        XCTAssertTrue(waders.waitForExistence(timeout: 10), "At the second channel the soldiers are wading")
        Thread.sleep(forTimeInterval: 1.5)
        attach("62-night-waders", in: app)
        endPrototype(in: app)
        XCTAssertFalse(app.descendants(matching: .any)["globeDate"].firstMatch.exists, "The date goes with the night")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the night crossing leaves only the plain globe")
    }
}
