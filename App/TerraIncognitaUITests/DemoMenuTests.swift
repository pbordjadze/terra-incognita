import Foundation
import XCTest

/// The ordinary atlas menu is the shared route; story-specific tests cover the journeys themselves.
final class DemoMenuTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func capture(_ name: String) {
        Thread.sleep(forTimeInterval: 0.7)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "demo-menu-" + name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func assertAtlas(_ expected: String, in app: XCUIApplication) {
        let menu = app.buttons["demosMenu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 15), "Leaving a demo must return to the atlas")
        let predicate = NSPredicate(format: "value CONTAINS %@", expected)
        XCTAssertTrue(predicate.evaluate(with: menu)
            || XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: predicate, object: menu)], timeout: 10) == .completed,
            "The atlas must retain its selected presentation: \(expected); actual \(String(describing: menu.value))")
        XCTAssertTrue(menu.isHittable)
        XCTAssertFalse(app.scrollViews["demoPickerScroll"].exists, "The picker must finish dismissing before the atlas is restored")
    }

    @MainActor private func assertInspection(_ prefix: String, in app: XCUIApplication) {
        let follow = app.buttons[prefix + "Follow"].firstMatch
        XCTAssertTrue(follow.waitForExistence(timeout: 30), "Choosing a story must first inspect its atlas object")
        let date = app.descendants(matching: .any)[prefix + "Date"].firstMatch
        XCTAssertTrue(date.exists)
        XCTAssertTrue(date.label.contains("Today"))
        XCTAssertFalse(app.buttons[prefix + "Pause"].exists, "Menu selection must not start the guided clock")
        let close = app.buttons[prefix + "Close"].firstMatch
        XCTAssertTrue(close.isHittable)
        XCTAssertTrue(app.frame.contains(close.frame), "Leaving must stay within the screen")
        let other = prefix == "camelid" ? "whale" : "camelid"
        XCTAssertFalse(app.buttons[other + "Follow"].exists, "Only the selected story may be presented")
        XCTAssertFalse(app.buttons[other + "Pause"].exists, "The previous story must not keep playing underneath")
    }

    @MainActor private func filmSeconds(_ film: XCUIElement) -> Int {
        guard let value = film.value as? String,
              let progress = value.components(separatedBy: " · ").first(where: { $0.hasSuffix(" seconds") }),
              let seconds = Int(progress.components(separatedBy: " ")[0]) else {
            XCTFail("The film must expose its actual playback time to accessibility")
            return -1
        }
        return seconds
    }

    @MainActor func testBothStoriesOpenVoluntarilyFromTheGlobe() {
        let app = XCUIApplication()
        app.launchArguments = ["-globe", "-globeStyle", "persian", "-globe-at", "30.18333", "29.33333", "3.2"]
        app.launch()
        defer { app.terminate() }
        let fossil = app.descendants(matching: .any)["whaleFossil"].firstMatch
        XCTAssertTrue(fossil.waitForExistence(timeout: 45))
        assertAtlas("Globe", in: app)
        openDemos(in: app)
        XCTAssertTrue(demoItem("demoCamelid", in: app).exists)
        XCTAssertTrue(demoItem("demoWhale", in: app).exists)
        capture("ordinary-globe")
        app.buttons["demoPickerClose"].tap()
        assertAtlas("Globe", in: app)
        XCTAssertTrue(fossil.isHittable, "Cancelling the picker must preserve the atlas position")
        chooseDemo("demoCamelid", in: app)

        for prefix in ["camelid", "whale"] {
            if prefix == "whale" { chooseDemo("demoWhale", in: app) }
            assertInspection(prefix, in: app)
            capture(prefix + "-from-globe")
            app.buttons[prefix + "Follow"].tap()
            let pause = app.buttons[prefix + "Pause"].firstMatch
            XCTAssertTrue(pause.waitForExistence(timeout: 10))
            pause.tap()
            XCTAssertTrue(app.buttons[prefix + "Resume"].waitForExistence(timeout: 10))
            app.buttons[prefix + "Close"].tap()
            assertAtlas("Globe", in: app)
            XCTAssertTrue(fossil.waitForExistence(timeout: 15), "A menu visit must preserve the original atlas position")
            XCTAssertTrue(fossil.isHittable)
        }

        chooseDemo("demoCamelid", in: app)
        assertInspection("camelid", in: app)
        Thread.sleep(forTimeInterval: 1)
        XCTAssertTrue(app.buttons["camelidFollow"].isHittable, "A reopened story must wait for a fresh choice to follow")
        app.buttons["camelidClose"].tap()
        assertAtlas("Globe", in: app)
    }

    @MainActor func testBothStoriesReturnToTheSelectedFilm() {
        let app = XCUIApplication()
        app.launchArguments = ["-look", "ffern"]
        app.launch()
        defer { app.terminate() }
        assertAtlas("Film", in: app)
        assertAtlas("Paper Almanac", in: app)
        let film = app.descendants(matching: .any)["mainAtlasFilm"].firstMatch
        XCTAssertTrue(film.waitForExistence(timeout: 45))

        for (identifier, prefix) in [("demoCamelid", "camelid"), ("demoWhale", "whale")] {
            // Check the real running clock on the first visit. The second also verifies
            // restoration if the short atlas film has reached its natural ending by then.
            let before = filmSeconds(film)
            let started = Date()
            if prefix == "camelid" {
                XCTAssertTrue((film.value as? String)?.hasPrefix("Film · ") == true)
                XCTAssertTrue((film.value as? String)?.contains("Active") == true)
            }
            chooseDemo(identifier, in: app)
            assertInspection(prefix, in: app)
            let covered = Date()
            if prefix == "camelid" { Thread.sleep(forTimeInterval: 3) }
            capture(prefix + "-from-film")
            let coveredDuration = Date().timeIntervalSince(covered)
            app.buttons[prefix + "Close"].tap()
            XCTAssertTrue(film.waitForExistence(timeout: 15))
            let after = filmSeconds(film)
            let wallDuration = Date().timeIntervalSince(started)
            if prefix == "camelid" {
                XCTAssertGreaterThanOrEqual(after, before, "Returning must preserve the film's progress")
                XCTAssertLessThanOrEqual(Double(after - before), wallDuration - coveredDuration + 1.5,
                    "The atlas film must not advance while covered by a story; allow for whole-second accessibility rounding")
            }
            assertAtlas("Film", in: app)
            assertAtlas("Paper Almanac", in: app)
        }
        capture("film-restored")
        chooseDemo("demoGlobe", in: app)
        assertAtlas("Globe", in: app)
        Thread.sleep(forTimeInterval: 3)
        chooseDemo("demoFilm", in: app)
        assertAtlas("Film", in: app)
        assertAtlas("Paper Almanac", in: app)
        XCTAssertTrue(film.waitForExistence(timeout: 15))
        let playing = NSPredicate(format: "value BEGINSWITH %@ AND value ENDSWITH %@", "Film · ", "Active")
        XCTAssertEqual(XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: playing, object: film)], timeout: 5), .completed,
                       "Returning from the globe must finish its short camera return and play the film")
        let resumedAt = filmSeconds(film)
        let advances = NSPredicate(format: "value != %@", film.value as? String ?? "")
        XCTAssertEqual(XCTWaiter.wait(for: [XCTNSPredicateExpectation(predicate: advances, object: film)], timeout: 5), .completed,
                       "The real film clock must advance after resuming from a hidden globe interval")
        XCTAssertGreaterThan(filmSeconds(film), resumedAt)
    }

    @MainActor func testLargestTextMenuReachesEveryDemoAndFilmLook() {
        let app = XCUIApplication()
        app.launchArguments = ["-globe", "-globeStyle", "persian", "-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        app.launch()
        defer { app.terminate() }
        let menu = app.buttons["demosMenu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 45))
        XCTAssertTrue(app.frame.contains(menu.frame), "The menu launcher must fit at the largest text size")
        openDemos(in: app)
        let closePicker = app.buttons["demoPickerClose"].firstMatch
        XCTAssertTrue(closePicker.isHittable)
        XCTAssertTrue(app.frame.contains(closePicker.frame), "The picker exit must fit at the largest text size")
        capture("largest-text-opening")
        for identifier in ["demoCamelid", "demoWhale", "demoOcean", "demoWater", "demoLeaf", "demoGlobe", "demoFilm", "demoFilmLooks"] {
            XCTAssertTrue(demoItem(identifier, in: app).exists)
        }
        capture("largest-text-scrolled")
        demoItem("demoGlobe", in: app).tap()

        for (identifier, prefix) in [("demoCamelid", "camelid"), ("demoWhale", "whale")] {
            chooseDemo(identifier, in: app)
            assertInspection(prefix, in: app)
            app.buttons[prefix + "Close"].tap()
            assertAtlas("Globe", in: app)
        }
        for (identifier, closeIdentifier) in [("demoOcean", "closeExperience"), ("demoWater", "closeWaterStudy"), ("demoLeaf", "Return to the atlas")] {
            chooseDemo(identifier, in: app)
            let close = app.buttons[closeIdentifier].firstMatch
            XCTAssertTrue(close.waitForExistence(timeout: 45))
            XCTAssertTrue(close.isHittable)
            XCTAssertTrue(app.frame.contains(close.frame), "The demo's return control must fit at the largest text size")
            if identifier == "demoOcean" {
                XCTAssertTrue(app.scrollViews["oceanExperience"].exists)
            } else if identifier == "demoWater" {
                let gather = app.webViews.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH[c] %@", "Gather")).firstMatch
                XCTAssertTrue(gather.waitForExistence(timeout: 45), "The water demo must load its actual bundled controls")
            } else {
                XCTAssertTrue(app.buttons["Open the leaf menu"].waitForExistence(timeout: 45), "The leaf must load its actual bundled controls")
            }
            close.tap()
            assertAtlas("Globe", in: app)
        }
        chooseDemo("demoFilmLooks", in: app)
        demoItem("demoLook_sampler", in: app).tap()
        assertAtlas("Film", in: app)
        assertAtlas("The Sampler", in: app)
        capture("largest-text-selected-look")
        chooseDemo("demoGlobe", in: app)
        assertAtlas("Globe", in: app)
        chooseDemo("demoFilm", in: app)
        assertAtlas("Film", in: app)
        assertAtlas("The Sampler", in: app)
        chooseDemo("demoGlobe", in: app)
        assertAtlas("Globe", in: app)
    }
}
