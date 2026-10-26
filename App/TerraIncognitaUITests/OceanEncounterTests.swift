import XCTest

/// Exercises real controls and records what the simulator actually displayed, rather than
/// substituting launch arguments for the reader's gestures.
final class OceanEncounterTests: XCTestCase {
    override func setUpWithError() throws {
        continueAfterFailure = false
    }

    @MainActor private func capture(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func button(_ id: String, in app: XCUIApplication) -> XCUIElement {
        app.buttons[id].firstMatch
    }

    /// Scroll the reading surface, including backwards after inspecting its lower details.
    /// The edge drag avoids competing with a map in the middle of the page.
    @MainActor private func reveal(_ element: XCUIElement, in app: XCUIApplication, towardsTop: Bool = false) {
        guard !element.isHittable else { return }
        for upwards in [!towardsTop, towardsTop] {
            for _ in 0..<8 {
                guard !element.isHittable else { return }
                let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0.99, dy: upwards ? 0.78 : 0.28))
                let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0.99, dy: upwards ? 0.28 : 0.78))
                start.press(forDuration: 0.05, thenDragTo: end)
            }
        }
        XCTAssertTrue(element.isHittable, "A control or explanation must remain reachable when text grows")
    }

    @MainActor private func exercise(largeText: Bool) {
        let app = XCUIApplication()
        app.launchArguments = ["-globe", "-globeStyle", "persian"]
        if largeText {
            app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        }
        app.launch()
        defer { app.terminate() }
        let prefix = largeText ? "large-text-" : ""
        let invitation = button("oceanInvitation", in: app)
        XCTAssertTrue(invitation.waitForExistence(timeout: 45), "The encounter must be discoverable from the globe")
        XCTAssertTrue(invitation.isHittable)
        capture(prefix + "00-globe-doorway")
        invitation.tap()
        let close = button("closeExperience", in: app)
        XCTAssertTrue(close.waitForExistence(timeout: 15))
        capture(prefix + "01-open")

        let loupe = button("oceanLoupe", in: app)
        reveal(loupe, in: app)
        let initialShore = loupe.value as? String
        XCTAssertNotNil(initialShore)
        let field = app.descendants(matching: .any)["oceanMapSurface"].firstMatch
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
        let initialValue = timeline.value as? String
        timeline.adjust(toNormalizedSliderPosition: 0.15)
        let earlierValue = timeline.value as? String
        let era = app.staticTexts["evidenceEra"].firstMatch
        XCTAssertTrue(era.exists)
        let earlierEra = era.label
        timeline.adjust(toNormalizedSliderPosition: 0.9)
        XCTAssertNotEqual(earlierValue, timeline.value as? String, "Time must respond to the reader's hand")
        XCTAssertNotEqual(earlierEra, era.label, "The explanation must respond to time as well as the slider")
        XCTAssertNotNil(initialValue)
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
        XCTAssertTrue(invitation.waitForExistence(timeout: 10))
        invitation.tap()
        XCTAssertTrue(close.waitForExistence(timeout: 10), "Returning must never require completing anything")
        capture(prefix + "08-reopened")
    }

    @MainActor func testFreelyExploreAndReturn() { exercise(largeText: false) }
    @MainActor func testLargestAccessibleText() { exercise(largeText: true) }
}
