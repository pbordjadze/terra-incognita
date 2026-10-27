import XCTest

extension XCTestCase {
    @MainActor func openDemos(in app: XCUIApplication) {
        let menu = app.buttons["demosMenu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 45), "The atlas must expose its consolidated demo menu")
        XCTAssertTrue(menu.isHittable)
        menu.tap()
        XCTAssertTrue(app.scrollViews["demoPickerScroll"].waitForExistence(timeout: 10),
                      "The demo picker must expose its own scrollable reading surface")
        XCTAssertTrue(app.buttons["demoPickerClose"].isHittable, "The picker can always be left without selecting a demo")
    }

    @MainActor func demoItem(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        let scroll = app.scrollViews["demoPickerScroll"].firstMatch
        XCTAssertTrue(scroll.waitForExistence(timeout: 10), "Reveal choices inside the picker, never on the atlas underneath")
        let item = scroll.buttons[identifier].firstMatch
        XCTAssertTrue(item.waitForExistence(timeout: 10), "The demo picker must contain \(identifier)")
        func visiblyInsidePicker() -> Bool {
            let clip = scroll.frame.intersection(app.frame).insetBy(dx: 2, dy: 8)
            return item.exists && item.isHittable && clip.contains(item.frame)
        }
        for _ in 0..<20 where !visiblyInsidePicker() {
            let clip = scroll.frame.intersection(app.frame).insetBy(dx: 8, dy: 12)
            let upwards = item.frame.midY > clip.midY
            let excess = upwards ? item.frame.maxY - clip.maxY : clip.minY - item.frame.minY
            let distance = min(clip.height * 0.55, max(40, excess + 24))
            let startY = clip.minY + clip.height * (upwards ? 0.8 : 0.2)
            let endY = min(max(startY + (upwards ? -distance : distance), clip.minY + 8), clip.maxY - 8)
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let start = origin.withOffset(CGVector(dx: clip.midX, dy: startY))
            let end = origin.withOffset(CGVector(dx: clip.midX, dy: endY))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        XCTAssertTrue(visiblyInsidePicker(), "The complete demo choice must remain visible within the picker at this text size: \(identifier), \(item.frame); viewport \(scroll.frame)")
        return item
    }

    @MainActor func chooseDemo(_ identifier: String, in app: XCUIApplication) {
        openDemos(in: app)
        demoItem(identifier, in: app).tap()
    }
}
