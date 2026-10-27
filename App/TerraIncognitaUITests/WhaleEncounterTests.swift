import Foundation
import XCTest

/// Exercises the atlas encounter, real globe gestures and optional native reading surfaces.
/// Held launch times reach review compositions; the full film is captured separately.
final class WhaleEncounterTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func element(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any)[identifier].firstMatch
    }

    @MainActor private func capture(_ name: String) {
        // Allow the short native camera/layer transition to settle, without waiting for the film.
        Thread.sleep(forTimeInterval: 1.3)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "whale-" + name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func reveal(_ target: XCUIElement, in app: XCUIApplication) {
        // The atlas timeline remains in the iPad accessibility tree beneath its centered sheet.
        // Touching that underlying surface can dismiss the sheet instead of scrolling its text.
        let reading = app.scrollViews["whaleReadingScroll"].firstMatch
        XCTAssertTrue(reading.waitForExistence(timeout: 10), "The actual reading scroll surface must be available")
        for _ in 0..<18 {
            if target.exists && target.isHittable { return }
            XCTAssertTrue(reading.exists, "Reading must remain open while its content is explored")
            reading.swipeUp()
        }
        let state = target.exists ? target.label : "Target is absent from the reading sheet"
        XCTAssertTrue(target.exists && target.isHittable, "Optional reading must stay reachable: \(state)")
    }

    @MainActor func testFossilWaitsForTheReaderAndReturnsToAtlas() {
        let app = XCUIApplication()
        app.launchArguments = ["-prototype", "whale", "-globe-at", "30.18333", "29.33333", "3.2"]
        app.launch()
        defer { app.terminate() }
        let fossil = element("whaleFossil", in: app)
        XCTAssertTrue(fossil.waitForExistence(timeout: 45), "The regional fossil must be discoverable at Wadi Al-Hitan")
        XCTAssertTrue(fossil.isHittable)
        XCTAssertFalse(app.buttons["whalePause"].exists, "The fossil must not begin a guided visit on its own")
        capture("00-atlas-fossil")
        fossil.tap()
        let follow = app.buttons["whaleFollow"].firstMatch
        XCTAssertTrue(follow.waitForExistence(timeout: 15))
        XCTAssertTrue(element("whaleDate", in: app).label.contains("Today"))
        Thread.sleep(forTimeInterval: 1)
        XCTAssertTrue(follow.isHittable, "Inspection must wait for a separate choice to visit the past")
        XCTAssertFalse(app.buttons["whalePause"].exists)
        capture("01-fossil-inspection")
        follow.tap()
        let pause = app.buttons["whalePause"].firstMatch
        XCTAssertTrue(pause.waitForExistence(timeout: 10))
        pause.tap()
        let resume = app.buttons["whaleResume"].firstMatch
        XCTAssertTrue(resume.waitForExistence(timeout: 10))
        let stoppedTime = resume.value as? String
        XCTAssertNotNil(stoppedTime)
        Thread.sleep(forTimeInterval: 1.2)
        XCTAssertEqual(stoppedTime, resume.value as? String, "Pause must freeze the actual guided moment")
        resume.tap()
        XCTAssertTrue(pause.waitForExistence(timeout: 10))
        XCUIDevice.shared.press(.home)
        app.activate()
        XCTAssertTrue(resume.waitForExistence(timeout: 15), "Returning from the background must leave the guide paused")
        app.buttons["whaleClose"].tap()
        XCTAssertTrue(fossil.waitForExistence(timeout: 15))
        XCTAssertTrue(fossil.isHittable, "Leaving must preserve the original atlas position")
        openWorkbench(in: app)
        XCTAssertTrue(workbenchItem("water", in: app).exists, "The independent atlas studies must remain available")
        app.buttons["workbenchClose"].firstMatch.tap()
        fossil.tap()
        XCTAssertTrue(follow.waitForExistence(timeout: 15), "Reopening must not require finishing the earlier visit")
        app.buttons["whaleClose"].tap()
        XCTAssertTrue(fossil.waitForExistence(timeout: 15))
    }

    @MainActor func testGlobeYieldsAndDocumentedMomentsShowCompleteArt() {
        let app = XCUIApplication()
        app.launchArguments = ["-whale", "32"]
        app.launch()
        defer { app.terminate() }
        let resume = app.buttons["whaleResume"].firstMatch
        XCTAssertTrue(resume.waitForExistence(timeout: 45))
        let animal = element("whaleStoryAnimal", in: app)
        XCTAssertTrue(animal.waitForExistence(timeout: 10))
        XCTAssertTrue(animal.isHittable)
        capture("02-living-dorudon")
        animal.tap()
        let closeReading = app.buttons["whaleCloseEvidence"].firstMatch
        XCTAssertTrue(closeReading.waitForExistence(timeout: 10), "The living illustration must open its anatomical branch")
        let claim = element("whaleLimbClaim", in: app)
        reveal(claim, in: app)
        XCTAssertTrue(claim.label.localizedCaseInsensitiveContains("tiny hind limbs"))
        closeReading.tap()

        let globe = element("whaleGlobe", in: app)
        XCTAssertTrue(globe.exists)
        let before = globe.value as? String
        XCTAssertNotNil(before)
        resume.tap()
        XCTAssertTrue(app.buttons["whalePause"].waitForExistence(timeout: 10))
        let start = globe.coordinate(withNormalizedOffset: CGVector(dx: 0.40, dy: 0.43))
        let end = globe.coordinate(withNormalizedOffset: CGVector(dx: 0.67, dy: 0.47))
        start.press(forDuration: 0.08, thenDragTo: end)
        XCTAssertTrue(resume.waitForExistence(timeout: 10), "A deliberate drag must yield the guided camera and clock")
        XCTAssertNotEqual(before, globe.value as? String, "The reader's drag must actually move the globe")
        XCTAssertFalse(animal.exists, "Authored animal overlays must yield when the reader explores elsewhere")
        capture("03-reader-has-globe")
        resume.tap()
        XCTAssertTrue(app.buttons["whalePause"].waitForExistence(timeout: 10))
        globe.pinch(withScale: 1.25, velocity: 0.6)
        XCTAssertTrue(resume.waitForExistence(timeout: 10), "A pinch must yield the guide too")

        for (index, date) in [(0, "Today"), (1, "37"), (5, "Today")] {
            let milestone = app.buttons["whaleMilestone\(index)"].firstMatch
            XCTAssertTrue(milestone.isHittable)
            milestone.tap()
            XCTAssertTrue(resume.waitForExistence(timeout: 10))
            XCTAssertTrue(element("whaleDate", in: app).label.contains(date))
            XCTAssertTrue(animal.waitForExistence(timeout: 10), "Documented moments must show complete inspectable illustrations")
            XCTAssertTrue(app.buttons["whaleClose"].isHittable)
            XCTAssertTrue(app.buttons["whaleEvidence"].isHittable)
            capture("04-milestone-\(index)")
        }
        XCTAssertTrue(element("whaleScene", in: app).label.contains("Whales swam here"))
    }

    @MainActor func testLargestTextEvidenceAndLimbIllustrationsRemainReachable() {
        let app = XCUIApplication()
        app.launchArguments = ["-whale", "70", "-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        app.launch()
        defer { app.terminate() }
        let evidence = app.buttons["whaleEvidence"].firstMatch
        XCTAssertTrue(evidence.waitForExistence(timeout: 45))
        XCTAssertTrue(evidence.isHittable)
        capture("large-05-final-place")
        evidence.tap()
        let closeReading = app.buttons["whaleCloseEvidence"].firstMatch
        XCTAssertTrue(closeReading.waitForExistence(timeout: 10))
        let reconstruction = app.staticTexts.matching(NSPredicate(format: "label BEGINSWITH %@", "The illustrated fossil and the living whale")).firstMatch
        reveal(reconstruction, in: app)
        XCTAssertTrue(reconstruction.label.contains("do not identify one individual"))
        capture("large-06-reconstruction-evidence")
        closeReading.tap()
        let limbs = app.buttons["whaleLimbs"].firstMatch
        XCTAssertTrue(limbs.waitForExistence(timeout: 10))
        XCTAssertTrue(limbs.isHittable)
        limbs.tap()
        XCTAssertTrue(closeReading.waitForExistence(timeout: 10))
        let claim = element("whaleLimbClaim", in: app)
        reveal(claim, in: app)
        XCTAssertTrue(claim.label.contains("could not support walking"))
        XCTAssertTrue(claim.label.contains("swimming propulsion"))
        let wholeBody = app.images.matching(NSPredicate(format: "label BEGINSWITH %@", "Illustrated whole Dorudon:")).firstMatch
        reveal(wholeBody, in: app)
        capture("large-07-whole-body")
        let detail = app.images["whaleLimbDetail"].firstMatch
        reveal(detail, in: app)
        XCTAssertTrue(detail.exists, "The optional branch must render actual limb artwork")
        XCTAssertTrue(detail.label.contains("different scale"))
        capture("large-08-hind-limb-detail")
        closeReading.tap()
        XCTAssertTrue(app.buttons["whaleClose"].isHittable)
        app.buttons["whaleClose"].tap()
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 15))
    }

    @MainActor func testReducedMotionCanRevisitAndReadTheStory() {
        let app = XCUIApplication()
        app.launchArguments = ["-whale", "32", "-whale-reduce-motion"]
        app.launch()
        defer { app.terminate() }
        let resume = app.buttons["whaleResume"].firstMatch
        XCTAssertTrue(resume.waitForExistence(timeout: 45))
        XCTAssertTrue(element("whaleStoryAnimal", in: app).waitForExistence(timeout: 10))
        for (index, date) in [(1, "37"), (0, "Today"), (5, "Today")] {
            app.buttons["whaleMilestone\(index)"].tap()
            XCTAssertTrue(resume.exists)
            XCTAssertTrue(element("whaleDate", in: app).label.contains(date))
            XCTAssertTrue(element("whaleStoryAnimal", in: app).waitForExistence(timeout: 10))
        }
        capture("reduced-motion-final")
        app.buttons["whaleMore"].tap()
        let words = app.buttons["whaleStory"].firstMatch
        XCTAssertTrue(words.waitForExistence(timeout: 10))
        words.tap()
        let closeReading = app.buttons["whaleCloseEvidence"].firstMatch
        XCTAssertTrue(closeReading.waitForExistence(timeout: 10))
        let ancient = element("whaleStoryAncient", in: app)
        reveal(ancient, in: app)
        XCTAssertTrue(ancient.label.contains("37 million years ago"))
        capture("reduced-motion-text-equivalent")
        closeReading.tap()
        XCTAssertTrue(app.buttons["whaleClose"].isHittable)
    }
}
