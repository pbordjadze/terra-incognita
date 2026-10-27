import XCTest

final class WaterInteractionTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func control(_ containing: String, _ app: XCUIApplication) -> XCUIElement {
        // WebKit exposes aria-pressed controls as native switches. Query the accessible
        // label across roles so the test exercises the same control users encounter.
        app.webViews.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH[c] %@", containing)).firstMatch
    }
    @MainActor private func capture(_ name: String) {
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = name; shot.lifetime = .keepAlways; add(shot)
    }
    @MainActor private func exercise(large: Bool) {
        let app = XCUIApplication(); app.launchArguments = []
        if large { app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"] }
        app.launch(); defer { app.terminate() }
        let gather = control("Gather", app)
        XCTAssertTrue(gather.waitForExistence(timeout: 60), app.debugDescription)
        XCTAssertTrue(gather.isHittable)
        let prefix = large ? "large-" : "regular-"
        capture(prefix + "01-open")
        let map = app.webViews.descendants(matching: .any).matching(NSPredicate(format: "label BEGINSWITH %@", "Interactive river map of South America.")).firstMatch
        XCTAssertTrue(map.exists, app.debugDescription)
        XCTAssertGreaterThan(map.frame.height, 150)
        let start = map.coordinate(withNormalizedOffset: CGVector(dx: 0.3, dy: 0.25))
        let end = map.coordinate(withNormalizedOffset: CGVector(dx: 0.72, dy: 0.4))
        start.press(forDuration: 0.2, thenDragTo: end, withVelocity: .slow, thenHoldForDuration: 0.1)
        let routed = app.webViews.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "km along mapped channels")).firstMatch
        XCTAssertTrue(routed.waitForExistence(timeout: 10), app.debugDescription)
        capture(prefix + "02-touch")
        gather.tap()
        end.press(forDuration: 0.3, thenDragTo: start, withVelocity: .slow, thenHoldForDuration: 0.2)
        let upstream = app.webViews.staticTexts.matching(NSPredicate(format: "label CONTAINS %@", "Connected channels upstream")).firstMatch
        XCTAssertTrue(upstream.waitForExistence(timeout: 10), app.debugDescription)
        capture(prefix + "03-gather")
        let about = control("About this map", app)
        XCTAssertTrue(about.isHittable); about.tap()
        let close = control("Close map notes", app)
        XCTAssertTrue(close.waitForExistence(timeout: 10))
        XCTAssertTrue(close.isHittable)
        capture(prefix + "04-evidence")
        app.webViews.firstMatch.swipeUp()
        capture(prefix + "05-evidence-scrolled")
        close.tap(); XCTAssertTrue(gather.isHittable)
        capture(prefix + "06-return")
        let pause = control("Pause flowing ink", app)
        if pause.isHittable { pause.tap(); XCTAssertTrue(control("Resume flowing ink", app).exists) }
    }
    @MainActor func testTouchAndEvidence() { exercise(large: false) }
    @MainActor func testLargestText() { exercise(large: true) }
}
