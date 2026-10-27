import XCTest

extension XCTestCase {
    @MainActor func openDemos(in app: XCUIApplication) {
        let menu = app.buttons["demosMenu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 45), "The atlas must expose its consolidated demo menu")
        XCTAssertTrue(menu.isHittable)
        menu.tap()
    }

    @MainActor func demoItem(_ identifier: String, in app: XCUIApplication) -> XCUIElement {
        // iOS can expose a native popup action as a button or a menu item.
        let item = app.descendants(matching: .any)[identifier].firstMatch
        _ = item.waitForExistence(timeout: 3)
        // A native menu can scroll at the largest text size. Restrict any swipe to the
        // popup containing this item; never touch the atlas or a sheet behind the menu.
        for attempt in 0..<12 where !item.isHittable {
            let scroll = app.scrollViews.containing(.any, identifier: identifier).firstMatch
            let menu = app.menus.containing(.any, identifier: identifier).firstMatch
            // The currently presented menu is also safe when its lazy/offscreen action
            // has not entered the accessibility tree yet. Its frame never comes from the atlas.
            let visibleMenu = app.menus.allElementsBoundByIndex.first { $0.isHittable && $0.frame.intersects(app.frame) }
            let popup = scroll.exists ? scroll : (menu.exists ? menu : visibleMenu)
            guard let popup else { break }
            guard popup.exists else { break }
            if item.exists {
                if item.frame.midY < popup.frame.midY { popup.swipeDown() }
                else { popup.swipeUp() }
            } else if attempt < 6 { popup.swipeUp() }
            else { popup.swipeDown() }
        }
        XCTAssertTrue(item.exists, "The demo menu must contain \(identifier)")
        XCTAssertTrue(item.isHittable, "The demo must remain reachable: \(identifier)")
        XCTAssertTrue(app.frame.contains(item.frame), "The demo item must fit the screen: \(identifier), \(item.frame)")
        return item
    }

    @MainActor func chooseDemo(_ identifier: String, in app: XCUIApplication) {
        openDemos(in: app)
        demoItem(identifier, in: app).tap()
    }
}
