import XCTest

/// Verify the actual bundled WKWebView and its return route; the browser harness covers the camera.
final class LeafTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func capture(_ name: String) {
        Thread.sleep(forTimeInterval: 0.7)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func exercise(largeText: Bool) {
        let app = XCUIApplication()
        app.launchArguments = ["-globe", "-globeStyle", "persian"]
        if largeText {
            app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        }
        app.launch()
        defer { app.terminate() }
        let door = app.buttons["An Aegean leaf"].firstMatch
        XCTAssertTrue(door.waitForExistence(timeout: 45), "The separate leaf must remain reachable beside the globe")
        door.tap()
        let menu = app.descendants(matching: .any)["Open the leaf menu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 45), "The bundled leaf menu must remain accessible")
        capture(largeText ? "leaf-large-opening" : "leaf-opening")
        menu.tap()
        let place = app.buttons["Read note: Santorini"].firstMatch
        XCTAssertTrue(place.waitForExistence(timeout: 10))
        for _ in 0..<12 where !place.isHittable {
            let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.73))
            let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.43))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        XCTAssertTrue(place.isHittable, "Optional notes remain reachable at the reader's text size")
        place.tap()
        let close = app.buttons["Close the note"].firstMatch
        XCTAssertTrue(close.waitForExistence(timeout: 10))
        capture(largeText ? "leaf-large-note" : "leaf-note")
        XCTAssertTrue(close.isHittable, "Leaving a note must not require reading to its end")
        close.tap()
        let back = app.buttons["Return to the atlas"].firstMatch
        XCTAssertTrue(back.waitForExistence(timeout: 10))
        back.tap()
        XCTAssertTrue(door.waitForExistence(timeout: 15), "The existing globe must still be present after leaving the leaf")
    }

    @MainActor func testBundledLeafAndReturn() { exercise(largeText: false) }
    @MainActor func testLeafAtLargestText() { exercise(largeText: true) }
}
