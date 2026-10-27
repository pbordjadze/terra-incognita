import XCTest

/// Entering and leaving prototypes the way a reader does: the workbench button, a row, the chip.
extension XCTestCase {
    /// What the plain globe leaves alive: one globe, nothing else.
    static let plainGlobeLive = "stages 0 · markers 0 · globes 1 · films 0 · web views 0"

    @MainActor func openWorkbench(in app: XCUIApplication) {
        let button = app.buttons["workbenchButton"].firstMatch
        XCTAssertTrue(button.waitForExistence(timeout: 45), "The globe must offer its workbench")
        XCTAssertTrue(button.isHittable)
        button.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForExistence(timeout: 10),
                      "The workbench must open, and can always be left without choosing")
    }

    /// A row of the open workbench, scrolled into view.
    @MainActor func workbenchItem(_ id: String, in app: XCUIApplication) -> XCUIElement {
        let item = app.buttons["prototype_\(id)"].firstMatch
        revealInWorkbench(item, in: app)
        XCTAssertTrue(item.exists && item.isHittable, "The workbench must list \(id)")
        return item
    }

    /// Swipes the open workbench until the element is on screen (rows below the fold are made lazily).
    @MainActor func revealInWorkbench(_ element: XCUIElement, in app: XCUIApplication, upwards: Bool = true) {
        let list = app.collectionViews["workbenchList"].firstMatch.exists ? app.collectionViews["workbenchList"].firstMatch
                                                                          : app.descendants(matching: .any)["workbenchList"].firstMatch
        for _ in 0..<14 where !(element.exists && element.isHittable) {
            if upwards { list.swipeUp(velocity: .slow) } else { list.swipeDown(velocity: .slow) }
        }
        if !(element.exists && element.isHittable) {
            for _ in 0..<14 where !(element.exists && element.isHittable) { list.swipeDown(velocity: .slow) }
        }
    }

    /// From the globe: the workbench, a prototype, and (`open`) its screen through the chip's Open.
    @MainActor func startPrototype(_ id: String, open: Bool = false, in app: XCUIApplication) {
        openWorkbench(in: app)
        workbenchItem(id, in: app).tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10), "Choosing closes the workbench")
        if open {
            let openButton = app.buttons["openActivePrototype"].firstMatch
            XCTAssertTrue(openButton.waitForExistence(timeout: 30), "An active prototype on the globe offers Open")
            openButton.tap()
        }
    }

    /// Back to the plain globe from an active prototype on the globe.
    @MainActor func endPrototype(in app: XCUIApplication) {
        let end = app.buttons["endActivePrototype"].firstMatch
        XCTAssertTrue(end.waitForExistence(timeout: 10), "An active prototype can always be ended from its chip")
        end.tap()
        XCTAssertTrue(app.buttons["endActivePrototype"].firstMatch.waitForNonExistence(timeout: 10))
    }

    /// The workbench's live counts, read when they match (the counts settle a moment after release).
    @MainActor func assertLive(_ expected: String, in app: XCUIApplication, _ message: String, file: StaticString = #filePath, line: UInt = #line) {
        let live = app.staticTexts["playgroundLive"].firstMatch
        revealInWorkbench(live, in: app)
        let settled = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label == %@", expected), object: live)
        let result = XCTWaiter().wait(for: [settled], timeout: 15)
        XCTAssertEqual(result, .completed, "\(message): \(live.exists ? live.label : "no live counts")", file: file, line: line)
    }

    /// Text inside a web view (or anywhere), by what it says.
    @MainActor func text(containing words: String, in app: XCUIApplication) -> XCUIElement {
        app.descendants(matching: .any).matching(NSPredicate(format: "label CONTAINS %@", words)).firstMatch
    }

    @MainActor func attach(_ name: String, in app: XCUIApplication) {
        let attachment = XCTAttachment(screenshot: app.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
