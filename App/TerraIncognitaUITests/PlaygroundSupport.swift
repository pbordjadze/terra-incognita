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

    /// Drags the open workbench until the element lies inside the list. The list makes and lets go of
    /// rows lazily, so while it doesn't exist the list moves by most of a screen; once it does, each
    /// drag is measured from where it is. Drags start inside the list, never on the sheet's grabber, so the
    /// sheet itself is never pulled down.
    @MainActor func revealInWorkbench(_ element: XCUIElement, in app: XCUIApplication) {
        // (the List carries its identifier when SwiftUI passes it on; otherwise it is the sheet's
        // only collection view)
        let named = app.collectionViews["workbenchList"].firstMatch
        let list = named.waitForExistence(timeout: 3) ? named : app.collectionViews.firstMatch
        guard list.waitForExistence(timeout: 10) else { return }
        func clip() -> CGRect { list.frame.intersection(app.frame).insetBy(dx: 0, dy: 16) }
        func visible() -> Bool {
            guard element.exists, element.isHittable else { return false }
            let c = clip(), f = element.frame
            // (a row taller than the list at the largest text counts once its middle is inside)
            return c.contains(f) || (f.height > c.height * 0.7 && c.contains(CGPoint(x: f.midX, y: f.midY)))
        }
        /// Positive: the content moves up, showing what is below.
        func drag(_ dy: CGFloat) {
            let c = clip()
            let distance = min(abs(dy), c.height * 0.6)
            let startY = dy > 0 ? c.maxY - c.height * 0.15 : c.minY + c.height * 0.15
            let origin = app.coordinate(withNormalizedOffset: .zero)
            origin.withOffset(CGVector(dx: c.midX, dy: startY))
                .press(forDuration: 0.05, thenDragTo: origin.withOffset(CGVector(dx: c.midX, dy: dy > 0 ? startY - distance : startY + distance)))
        }
        // While it doesn't exist, look further down; when the list stops moving (its end), look the
        // other way, in smaller steps, since a row the list has let go of may be above.
        var direction: CGFloat = 1
        for _ in 0..<30 where !visible() {
            let c = clip()
            if element.exists {
                let f = element.frame
                if f.midY > c.midY { drag(max(40, f.maxY - c.maxY + 24)) } else { drag(-max(40, c.minY - f.minY + 24)) }
                continue
            }
            let cell = list.cells.firstMatch
            let before = cell.exists ? cell.frame : .zero
            drag(direction * c.height * (direction > 0 ? 0.6 : 0.4))
            if (cell.exists ? cell.frame : .zero) == before { direction = -direction }
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
