import XCTest

/// The quiet globe's prototypes (`App/TerraIncognita/Quiet/`): names on demand, the clouds clear where
/// the reader looks, the Earth at night and the quiet frame, each a rule the plain globe takes while
/// the prototype runs and gives back when it ends. All four run together on the one globe, and ending
/// them leaves the plain globe's counts. Part of `PlaygroundTests`, so `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testQuietGlobeRulesRunTogetherAndLeaveNothingBehind() {
        let app = XCUIApplication()
        // over India, where the night's lights and the land's names both show; the sky held
        app.launchArguments = ["-globe-at", "78", "26", "3.2", "-globe-month", "10", "-clouds-days", "0.4", "-clouds-month", "10"]
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 45))
        XCTAssertTrue(text(containing: "The whole world", in: app).waitForExistence(timeout: 10), "Today's caption, before any rule")
        sleep(2)
        attach("50-today", in: app)

        startPrototype("quiet-names", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 10))
        sleep(2)
        attach("51-names-on-demand", in: app)
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        XCTAssertTrue(globe.exists, "The names' rule changes nothing but the names")
        globe.tap()
        sleep(2)
        attach("52-names-touched", in: app)

        startPrototype("quiet-clouds", in: app)
        sleep(2)
        attach("53-clouds-clear-where-you-look", in: app)

        startPrototype("night", in: app)
        sleep(3)   // the night's map read, the style crossfaded
        XCTAssertTrue(globe.exists, "The night is the one globe, restyled")
        attach("54-night", in: app)

        startPrototype("quiet-frame", in: app)
        XCTAssertTrue(text(containing: "The whole world", in: app).waitForNonExistence(timeout: 10), "The quiet frame says nothing at rest")
        sleep(1)
        attach("55-quiet-frame", in: app)
        let chip = app.staticTexts["otherPrototypes"].firstMatch
        XCTAssertTrue(chip.exists && chip.label.contains("3"), "All four rules run together: \(chip.exists ? chip.label : "no chip")")

        openWorkbench(in: app)
        assertLive(from: "stages 4", to: "globes 1 · films 0 · scripts 0", in: app, "Four rules, one globe, nothing more")
        app.buttons["workbenchClose"].firstMatch.tap()
        endFront(leaving: "The Earth at night", in: app)
        endFront(leaving: "Clouds clear", in: app)
        endFront(leaving: "Names on demand", in: app)
        endPrototype(in: app)
        XCTAssertTrue(text(containing: "The whole world", in: app).waitForExistence(timeout: 10), "The frame comes back with today's look")
        sleep(2)
        attach("56-today-again", in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the rules gives back the plain globe")
    }
}
