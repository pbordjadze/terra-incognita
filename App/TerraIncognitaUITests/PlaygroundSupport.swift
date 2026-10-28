import XCTest

/// Entering and leaving prototypes the way a reader does: the workbench button, a row, the chip.
extension XCTestCase {
    /// What the globe leaves alive with no prototype running: one globe, nothing else (the doorways are
    /// the atlas's own, not counted).
    static let plainGlobeLive = "stages 0 · markers 0 · globes 1 · films 0 · scripts 0 · web views 0"

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

    /// The open workbench's list: the collection view SwiftUI gives its identifier to, or else the
    /// sheet's only one.
    @MainActor func workbenchList(in app: XCUIApplication) -> XCUIElement? {
        guard app.collectionViews.firstMatch.waitForExistence(timeout: 10) else { return nil }
        let named = app.collectionViews["workbenchList"].firstMatch
        return named.exists ? named : app.collectionViews.firstMatch
    }

    /// Where the list's rows can be seen, a little in from its edges.
    @MainActor private func clip(_ list: XCUIElement, in app: XCUIApplication) -> CGRect {
        list.frame.intersection(app.frame).insetBy(dx: 0, dy: 16)
    }

    /// Drags the list's content by `dy` points (positive: up, showing what is below), from inside the
    /// list, never from the sheet's grabber, so the sheet itself is never pulled down.
    @MainActor private func drag(_ list: XCUIElement, by dy: CGFloat, in app: XCUIApplication) {
        let c = clip(list, in: app)
        let distance = min(abs(dy), c.height * 0.6)
        let startY = dy > 0 ? c.maxY - c.height * 0.15 : c.minY + c.height * 0.15
        let origin = app.coordinate(withNormalizedOffset: .zero)
        origin.withOffset(CGVector(dx: c.midX, dy: startY))
            .press(forDuration: 0.05, thenDragTo: origin.withOffset(CGVector(dx: c.midX, dy: dy > 0 ? startY - distance : startY + distance)))
    }

    /// Drags the open workbench until the element lies inside the list. The list makes and lets go of
    /// rows lazily, so while it doesn't exist the list moves by most of a screen; once it does, each
    /// drag is measured from where it is.
    @MainActor func revealInWorkbench(_ element: XCUIElement, in app: XCUIApplication) {
        guard let list = workbenchList(in: app) else { return }
        /// It exists with a real frame (a row the list has let go of may linger with none).
        func located() -> Bool { element.exists && !element.frame.isEmpty }
        func visible() -> Bool {
            guard located() else { return false }
            let c = clip(list, in: app), f = element.frame
            // (a row taller than the list at the largest text counts once its middle is inside)
            let inside = c.contains(f) || (f.height > c.height * 0.7 && c.contains(CGPoint(x: f.midX, y: f.midY)))
            // asked only once it is inside: XCTest fails outright on an element with no hit point
            return inside && element.isHittable
        }
        // While it doesn't exist, look further down; when the list stops moving (its end), look the
        // other way, in smaller steps, since a row the list has let go of may be above.
        var direction: CGFloat = 1
        for _ in 0..<30 where !visible() {
            let c = clip(list, in: app)
            if located() {
                let f = element.frame
                if f.midY > c.midY { drag(list, by: max(40, f.maxY - c.maxY + 24), in: app) } else { drag(list, by: -max(40, c.minY - f.minY + 24), in: app) }
                continue
            }
            let cell = list.cells.firstMatch
            let before = cell.exists ? cell.frame : .zero
            drag(list, by: direction * c.height * (direction > 0 ? 0.6 : 0.4), in: app)
            if (cell.exists ? cell.frame : .zero) == before { direction = -direction }
        }
    }

    /// Every prototype the open workbench lists, gathered in one pass down the list with one snapshot
    /// a screenful (each query of a busy app costs seconds, so finding each row in turn costs minutes).
    @MainActor func listedPrototypes(in app: XCUIApplication) -> Set<String> {
        guard let list = workbenchList(in: app) else { return [] }
        func identifiers(_ element: any XCUIElementSnapshot) -> [String] {
            [element.identifier] + element.children.flatMap(identifiers)
        }
        var listed = Set<String>()
        var before: [String] = []
        for _ in 0..<20 {
            guard let snapshot = try? app.snapshot() else { break }
            let now = identifiers(snapshot).filter { !$0.isEmpty }
            listed.formUnion(now.filter { $0.hasPrefix("prototype_") }.map { String($0.dropFirst("prototype_".count)) })
            // the end of the list, or it no longer moves
            if now.contains("playgroundLive") || now == before { break }
            before = now
            drag(list, by: clip(list, in: app).height * 0.6, in: app)
        }
        return listed
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

    /// Text anywhere on screen, by what it says.
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
