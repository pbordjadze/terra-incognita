import Foundation
import XCTest

/// Real native controls and gestures, with held launch times used only to reach review scenes.
final class CamelidEncounterTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func element(_ id: String, in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)[id].firstMatch
    }

    @MainActor private func capture(_ name: String) {
        Thread.sleep(forTimeInterval: 0.8)
        let picture = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        picture.name = "camelid-" + name
        picture.lifetime = .keepAlways
        add(picture)
    }

    @MainActor private func reveal(_ target: XCUIElement, in app: XCUIApplication) {
        // An iPad sheet leaves the atlas's horizontal timeline in the accessibility tree.
        // Swipe the reading surface itself; touching behind that sheet dismisses it.
        let reading = app.scrollViews["camelidReadingScroll"].firstMatch
        XCTAssertTrue(reading.waitForExistence(timeout: 10), "The reading sheet must expose its scrolling surface")
        for _ in 0..<18 {
            if target.exists && target.isHittable { return }
            XCTAssertTrue(reading.exists, "The reading sheet must remain open while exploring its content")
            reading.swipeUp()
        }
        let state = target.exists ? target.label : "Target is absent from the reading sheet"
        XCTAssertTrue(target.exists && target.isHittable, "Reading must remain reachable as text grows: \(state)")
    }

    @MainActor func testCamelWaitsForTheReaderAndCanBeLeft() {
        let app = XCUIApplication()
        app.launchArguments = ["-prototype", "camelid", "-globe-at", "46", "24", "1.5"]
        app.launch()
        defer { app.terminate() }
        let animal = element("camelidAnimal", in: app)
        XCTAssertTrue(animal.waitForExistence(timeout: 45))
        XCTAssertTrue(animal.isHittable)
        XCTAssertFalse(element("camelidPause", in: app).exists, "The atlas animal must not start a guided sequence by itself")
        capture("00-atlas-camel")
        animal.tap()
        let follow = app.buttons["camelidFollow"].firstMatch
        XCTAssertTrue(follow.waitForExistence(timeout: 15))
        XCTAssertTrue(element("camelidDate", in: app).label.contains("Today"))
        Thread.sleep(forTimeInterval: 1)
        XCTAssertTrue(follow.isHittable, "Inspection must wait for a separate choice to follow")
        capture("01-inspection")
        app.buttons["camelidClose"].tap()
        XCTAssertTrue(animal.waitForExistence(timeout: 15))
        animal.tap()
        XCTAssertTrue(follow.waitForExistence(timeout: 15))
        follow.tap()
        XCTAssertTrue(app.buttons["camelidPause"].waitForExistence(timeout: 10))
        app.buttons["camelidPause"].tap()
        let resume = app.buttons["camelidResume"].firstMatch
        XCTAssertTrue(resume.waitForExistence(timeout: 10))
        let pausedTime = resume.value as? String
        XCTAssertNotNil(pausedTime)
        Thread.sleep(forTimeInterval: 1.2)
        XCTAssertEqual(pausedTime, resume.value as? String, "The clock must stay at the reader's paused moment")
        resume.tap()
        XCTAssertTrue(app.buttons["camelidPause"].waitForExistence(timeout: 10))
        XCUIDevice.shared.press(.home)
        app.activate()
        XCTAssertTrue(resume.waitForExistence(timeout: 15), "Returning from the background must leave playback paused")
        app.buttons["camelidClose"].tap()
        XCTAssertTrue(animal.waitForExistence(timeout: 15), "Leaving must not require an ending")
    }

    @MainActor func testGlobeGesturesYieldAndMilestonesRemainCoherent() {
        let app = XCUIApplication()
        app.launchArguments = ["-camelid", "17"]
        app.launch()
        defer { app.terminate() }
        let resume = app.buttons["camelidResume"].firstMatch
        XCTAssertTrue(resume.waitForExistence(timeout: 45))
        let globe = element("camelidGlobe", in: app)
        XCTAssertTrue(globe.exists)
        let originalCamera = globe.value as? String
        XCTAssertNotNil(originalCamera)
        resume.tap()
        XCTAssertTrue(app.buttons["camelidPause"].waitForExistence(timeout: 10))
        let start = globe.coordinate(withNormalizedOffset: CGVector(dx: 0.42, dy: 0.45))
        let end = globe.coordinate(withNormalizedOffset: CGVector(dx: 0.68, dy: 0.48))
        start.press(forDuration: 0.08, thenDragTo: end)
        XCTAssertTrue(resume.waitForExistence(timeout: 10), "A deliberate globe drag must pause the guided clock")
        Thread.sleep(forTimeInterval: 0.8)
        XCTAssertNotEqual(originalCamera, globe.value as? String, "The map must actually move under the reader's hand")
        capture("02-free-globe")
        resume.tap()
        XCTAssertTrue(app.buttons["camelidPause"].waitForExistence(timeout: 10))
        globe.pinch(withScale: 1.3, velocity: 0.6)
        XCTAssertTrue(resume.waitForExistence(timeout: 10), "Pinching must yield camera control too")

        let expected = [(1, "44"), (2, "17"), (3, "7"), (4, "3"), (5, "Today")]
        for (index, date) in expected {
            let milestone = app.buttons["camelidMilestone\(index)"].firstMatch
            XCTAssertTrue(milestone.isHittable, "Every documented moment must be directly revisitable")
            milestone.tap()
            XCTAssertTrue(resume.waitForExistence(timeout: 10))
            XCTAssertTrue(element("camelidDate", in: app).label.contains(date))
            XCTAssertTrue(element("camelidStoryAnimal", in: app).waitForExistence(timeout: 5),
                          "A revisited milestone must show an inspectable illustration, not an empty transition frame")
            capture("03-milestone-\(index)")
        }
        XCTAssertTrue(element("camelidScene", in: app).label.contains("cousins"))
    }

    @MainActor private func exerciseReading(largeText: Bool) {
        let app = XCUIApplication()
        app.launchArguments = ["-camelid", "82"]
        if largeText {
            app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        }
        app.launch()
        defer { app.terminate() }
        let evidence = largeText ? app.buttons["camelidEvidence"].firstMatch : element("camelidStoryAnimal", in: app)
        XCTAssertTrue(evidence.waitForExistence(timeout: 45))
        XCTAssertTrue(evidence.isHittable)
        capture(largeText ? "large-04-cousins" : "04-cousins")
        evidence.tap()
        let returnButton = app.buttons["camelidCloseEvidence"].firstMatch
        XCTAssertTrue(returnButton.waitForExistence(timeout: 10))
        let reconstruction = app.staticTexts["An ancient animal is an illustration"].firstMatch
        reveal(reconstruction, in: app)
        capture(largeText ? "large-05-evidence" : "05-evidence")
        returnButton.tap()
        let family = largeText ? app.buttons["camelidFamily"].firstMatch : element("camelidStorySecondAnimal", in: app)
        XCTAssertTrue(family.waitForExistence(timeout: 10))
        XCTAssertTrue(family.isHittable)
        family.tap()
        XCTAssertTrue(returnButton.waitForExistence(timeout: 10))
        reveal(element("camelidFamilyLlama", in: app), in: app)
        capture(largeText ? "large-06-llama" : "06-llama")
        reveal(element("camelidFamilyAlpaca", in: app), in: app)
        capture(largeText ? "large-07-alpaca" : "07-alpaca")
        returnButton.tap()
        XCTAssertTrue(app.buttons["camelidClose"].waitForExistence(timeout: 10))
        app.buttons["camelidClose"].tap()
        openWorkbench(in: app)
        XCTAssertTrue(workbenchItem("ocean", in: app).exists, "The independent encounter must remain available")
        workbenchItem("plain", in: app).tap()
    }

    @MainActor func testEvidenceAndLivingFamily() { exerciseReading(largeText: false) }
    @MainActor func testLargestAccessibleReading() { exerciseReading(largeText: true) }

    @MainActor func testReducedMotionCanVisitAndReadEveryMoment() {
        let app = XCUIApplication()
        app.launchArguments = ["-camelid", "17", "-camelid-reduce-motion"]
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["camelidResume"].waitForExistence(timeout: 45))
        for index in [2, 3, 4, 5, 1] {
            app.buttons["camelidMilestone\(index)"].tap()
            XCTAssertTrue(app.buttons["camelidResume"].exists)
            XCTAssertFalse(element("camelidScene", in: app).label.isEmpty)
        }
        capture("reduced-motion-origin")
        app.buttons["camelidMore"].tap()
        let words = app.buttons["camelidStory"].firstMatch
        XCTAssertTrue(words.waitForExistence(timeout: 10))
        words.tap()
        XCTAssertTrue(app.buttons["camelidCloseEvidence"].waitForExistence(timeout: 10))
        let account = app.staticTexts["About 44 million years ago · North America"].firstMatch
        reveal(account, in: app)
        capture("reduced-motion-text-equivalent")
        app.buttons["camelidCloseEvidence"].tap()
        XCTAssertTrue(app.buttons["camelidClose"].exists)
    }
}
