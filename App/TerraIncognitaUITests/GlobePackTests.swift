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

    /// The vine (runtime 6, dated geography): it opens close on the qvevri and the first shoots grow as the
    /// years play; the years moved near the strip's end, where the vine has reached the whole world; a touch
    /// on the vine lights the way back and names the place at the foot; then the plain globe.
    @MainActor func testTheVineGrowsWithTheYears() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("vine", in: app)
        let date = app.descendants(matching: .any)["globeDate"].firstMatch
        XCTAssertTrue(date.waitForExistence(timeout: 30), "The globe shows the vine's year")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("vine"), "The vine is in front")
        // the opening: a beat on the qvevri, then the first shoots and the slow pull-back
        Thread.sleep(forTimeInterval: 9)
        attach("55-vine-opens", in: app)
        let play = app.buttons["globeDatePlay"].firstMatch
        if play.exists && play.label == "Pause" { play.tap() }
        date.coordinate(withNormalizedOffset: CGVector(dx: 0.97, dy: 0.5)).tap()
        let late = XCTNSPredicateExpectation(predicate: NSPredicate(format: "NOT (value CONTAINS %@)", "BCE"), object: date)
        XCTAssertEqual(XCTWaiter().wait(for: [late], timeout: 10), .completed, "Near the strip's end the year is in our era")
        Thread.sleep(forTimeInterval: 2)
        attach("56-vine-today", in: app)
        // a touch on the vine where the pull-back left the camera (eastern Anatolia, on the way to Troy):
        // the way back to the qvevri lights and the foot names the place
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        let foot = app.descendants(matching: .any)["globeFootCaption"].firstMatch
        let before = foot.exists ? foot.label : ""
        globe.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.5)).tap()
        let named = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label != %@", before), object: foot)
        _ = XCTWaiter().wait(for: [named], timeout: 4)
        Thread.sleep(forTimeInterval: 1.2)
        attach("57-vine-touched", in: app)
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the vine leaves only the plain globe")
    }

    /// The herds (runtime 6): animals on dated ways, which the globe's date moves, their frames cut from
    /// sprite sheets (a sprite with no frame the app could read is never set, so the first camelid being on
    /// the globe says its kind's sheet was read and cut). It opens close on one of the first camelids and pulls back; moving the date to a mark on its strip moves that animal,
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
}
