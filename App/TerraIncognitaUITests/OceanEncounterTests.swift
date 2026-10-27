import Foundation
import XCTest

/// Exercises real controls and records what the simulator actually displayed, rather than
/// substituting launch arguments for the reader's gestures.
final class OceanEncounterTests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor private func capture(_ name: String) {
        // The app's crossfade and scroll-to-place transitions last up to 0.4 seconds.
        // XCTest can finish a tap before those visual layers settle; picture the final state.
        Thread.sleep(forTimeInterval: 0.65)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func button(_ id: String, in app: XCUIApplication) -> XCUIElement {
        app.buttons[id].firstMatch
    }

    /// Scroll the reading surface, including backwards after inspecting its lower details.
    /// Start on actual reading content outside the map, whose own drag moves its glass.
    @MainActor private func reveal(_ element: XCUIElement, in app: XCUIApplication, towardsTop: Bool = false) {
        guard !element.isHittable else { return }
        // At the largest accessible size this authored page is more than ten screens long.
        // Follow the target's current frame instead of reversing after an arbitrary short run.
        for _ in 0..<24 {
            guard !element.isHittable else { return }
            let readingSurface = app.scrollViews["oceanExperience"].firstMatch
            let viewport = readingSurface.exists ? readingSurface.frame : app.frame
            let upwards: Bool
            if element.exists {
                upwards = element.frame.midY > viewport.midY
            } else {
                upwards = !towardsTop
            }
            var startY = viewport.minY + viewport.height * (upwards ? 0.75 : 0.25)
            let map = app.otherElements["Indian Ocean viewing table"].firstMatch
            if map.exists && map.frame.insetBy(dx: -10, dy: -10).contains(CGPoint(x: viewport.midX, y: startY)) {
                let above = max(0, min(map.frame.minY - 12, viewport.maxY) - viewport.minY)
                let below = max(0, viewport.maxY - max(map.frame.maxY + 12, viewport.minY))
                startY = below >= above ? viewport.maxY - below / 2 : viewport.minY + above / 2
            }
            let distance = min(viewport.height * 0.55, 500)
            let endY = min(max(startY + (upwards ? -distance : distance), viewport.minY + 18), viewport.maxY - 18)
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let start = origin.withOffset(CGVector(dx: viewport.midX, dy: startY))
            let end = origin.withOffset(CGVector(dx: viewport.midX, dy: endY))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        let state = element.exists ? "Exists as \(element.elementType): \(element.label)" : "Element query did not match"
        XCTAssertTrue(element.isHittable, "A control or explanation must remain reachable when text grows. \(state).\n\(app.debugDescription)")
    }

    @MainActor private func exercise(largeText: Bool) {
        let app = XCUIApplication()
        app.launchArguments = []
        if largeText {
            app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        }
        app.launch()
        defer { app.terminate() }
        let prefix = largeText ? "large-text-" : ""
        let menu = button("workbenchButton", in: app)
        XCTAssertTrue(menu.waitForExistence(timeout: 45), "The encounter must be discoverable from the workbench")
        XCTAssertTrue(menu.isHittable)
        startPrototype("ocean", in: app)
        // on the globe: its seed off Madagascar and its invitation, either of which opens it
        let invitation = button("oceanInvitation", in: app)
        XCTAssertTrue(invitation.waitForExistence(timeout: 30))
        capture(prefix + "00-globe-doorway")
        invitation.tap()
        let close = button("closeExperience", in: app)
        XCTAssertTrue(close.waitForExistence(timeout: 15))
        capture(prefix + "01-open")

        // Query the stable identity without depending on the representation's accessibility trait.
        let loupe = app.descendants(matching: .any)["oceanLoupe"].firstMatch
        reveal(loupe, in: app)
        let initialShore = loupe.value as? String
        XCTAssertNotNil(initialShore)
        // A container identifier propagates through SwiftUI to its children, so use the
        // container's descriptive label and leave every interactive child its own identity.
        let field = app.otherElements["Indian Ocean viewing table"].firstMatch
        XCTAssertTrue(field.exists)
        // A horizontal drag on the map itself must move the glass, without scrolling the page
        // or relying on the discrete place buttons to supply a different rendering.
        let mapStart = field.coordinate(withNormalizedOffset: CGVector(dx: 0.49, dy: 0.48))
        let mapEnd = field.coordinate(withNormalizedOffset: CGVector(dx: 0.83, dy: 0.48))
        mapStart.press(forDuration: 0.08, thenDragTo: mapEnd)
        XCTAssertNotEqual(initialShore, loupe.value as? String, "Dragging the field must change the glass's geographic focus")
        capture(prefix + "01-glass-moved")

        let borneo = button("place_borneo", in: app)
        reveal(borneo, in: app)
        XCTAssertTrue(borneo.waitForExistence(timeout: 10))
        borneo.tap()
        let placeNote = app.staticTexts["Southern Borneo"].firstMatch
        XCTAssertTrue(placeNote.waitForExistence(timeout: 10), "A geographic mark must open its own situated detail")
        reveal(placeNote, in: app)
        capture(prefix + "01-place-detail")
        let aside = app.buttons["Put this note aside"].firstMatch
        reveal(aside, in: app)
        aside.tap()
        XCTAssertFalse(placeNote.exists)

        var claims = Set<String>()
        for (index, id) in ["lensSeeds", "lensWords", "lensPeople"].enumerated() {
            let lens = button(id, in: app)
            reveal(lens, in: app, towardsTop: true)
            XCTAssertTrue(lens.waitForExistence(timeout: 10), "The reader must be free to change subject")
            XCTAssertTrue(lens.isHittable)
            lens.tap()
            capture(prefix + "0\(index + 2)-\(id)")
            let claim = app.staticTexts["encounterClaim"].firstMatch
            reveal(claim, in: app)
            XCTAssertTrue(claim.waitForExistence(timeout: 10))
            XCTAssertFalse(claim.label.isEmpty)
            claims.insert(claim.label)
            capture(prefix + "0\(index + 2)-\(id)-detail")
            if id == "lensWords" {
                let tangent = button("wordTangent", in: app)
                reveal(tangent, in: app)
                let inherited = tangent.label
                tangent.tap()
                XCTAssertNotEqual(inherited, tangent.label, "The word's tangent must open a different relationship")
                capture(prefix + "03-word-tangent")
                tangent.tap()
                XCTAssertEqual(inherited, tangent.label, "The tangent must allow a direct return")
            }
        }
        XCTAssertEqual(claims.count, 3, "Each lens must show its own account, not just change a highlight")

        let seeds = button("lensSeeds", in: app)
        reveal(seeds, in: app, towardsTop: true)
        seeds.tap()
        let timeline = app.sliders["evidenceTime"].firstMatch
        reveal(timeline, in: app)
        XCTAssertTrue(timeline.waitForExistence(timeout: 10))
        let earlierValue = timeline.value as? String
        let era = app.staticTexts["evidenceEra"].firstMatch
        XCTAssertTrue(era.exists)
        let earlierEra = era.label
        // Keep the spoken value meaningful to a person. Exercise the actual thumb movement
        // rather than asking XCTest to parse that semantic value as a percentage.
        // Start at its actual initial thumb. A preliminary short drag does not reliably move
        // the iPad thumb, so its assumed endpoint cannot be the next gesture's starting point.
        // Hold long enough for the surrounding scroll view to hand the touch to the slider.
        timeline.coordinate(withNormalizedOffset: CGVector(dx: 0.02, dy: 0.5))
            .press(forDuration: 0.35, thenDragTo: timeline.coordinate(withNormalizedOffset: CGVector(dx: 0.9, dy: 0.5)))
        XCTAssertNotEqual(earlierValue, timeline.value as? String, "Time must respond to the reader's hand (slider frame: \(timeline.frame), era: \(era.label))")
        XCTAssertNotEqual(earlierEra, era.label, "The explanation must respond to time as well as the slider")
        XCTAssertNotNil(earlierValue)
        capture(prefix + "05-time")

        let evidence = button("evidenceButton", in: app)
        XCTAssertTrue(evidence.isHittable)
        evidence.tap()
        let closeEvidence = button("closeEvidence", in: app)
        XCTAssertTrue(closeEvidence.waitForExistence(timeout: 10))
        capture(prefix + "06-evidence")
        // Sources remain readable beyond the first screen, then can be left without a gate.
        app.swipeUp()
        capture(prefix + "07-evidence-scrolled")
        closeEvidence.tap()
        XCTAssertTrue(close.waitForExistence(timeout: 10))
        close.tap()
        XCTAssertTrue(menu.waitForExistence(timeout: 10))
        XCTAssertTrue(invitation.waitForExistence(timeout: 10), "Closing its screen leaves the encounter on the globe")
        invitation.tap()
        XCTAssertTrue(close.waitForExistence(timeout: 10), "Returning must never require completing anything")
        capture(prefix + "08-reopened")
    }

    @MainActor func testFreelyExploreAndReturn() { exercise(largeText: false) }
    @MainActor func testLargestAccessibleText() { exercise(largeText: true) }
}
