import XCTest

/// The clouds (`App/TerraIncognita/Clouds/`), part of the stage: over the opening globe with its
/// names still there, a real pinch down through them, what a frame costs in the workbench's log, and
/// the switch beside the workbench turning them off and on, the Live counts the plain globe's
/// throughout. Part of `PlaygroundTests`, so `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testCloudsAreOnTheGlobeAndTurnOff() {
        let app = XCUIApplication()
        // October's sky over Africa, its drift held: the same pictures every run
        app.launchArguments = ["-clouds-days", "0.4", "-clouds-month", "10", "-globe-at", "15", "8", "1"]
        app.launch()
        defer { app.terminate() }

        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        XCTAssertTrue(globe.waitForExistence(timeout: 15))
        // (a toggle: VoiceOver and the tests see it as a switch, so it is found by its identifier alone)
        let clouds = app.descendants(matching: .any)["cloudsButton"].firstMatch
        XCTAssertTrue(clouds.waitForExistence(timeout: 10), "The clouds' switch is beside the workbench button")
        // (what it says, once the tap has been heard)
        func says(_ value: String) -> Bool {
            let said = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", value), object: clouds)
            return XCTWaiter().wait(for: [said], timeout: 5) == .completed
        }
        // (the switch is remembered: a run before may have left it off)
        if clouds.value as? String == "Off" { clouds.tap() }
        XCTAssertTrue(says("On"), "The clouds are on until the reader turns them off")
        sleep(3)   // the sky decoded, and faded in
        attach("40-clouds-earth", in: app)

        // down through them, with a reader's own pinch
        globe.pinch(withScale: 2.4, velocity: 2)
        sleep(2)
        attach("41-clouds-passing", in: app)
        globe.pinch(withScale: 2.6, velocity: 2)
        sleep(2)
        attach("42-clouds-below", in: app)
        sleep(8)   // a report in the workbench's log every ten seconds while they draw

        openWorkbench(in: app)
        let report = text(containing: "clouds:", in: app)
        revealInWorkbench(report, in: app)
        attach("43-clouds-workbench", in: app)
        let cost = XCTAttachment(string: report.exists ? report.label : "no report from the clouds")
        cost.name = "43-clouds-cost"
        cost.lifetime = .keepAlways
        add(cost)
        XCTAssertTrue(report.exists, "The clouds say what a frame costs, in the workbench's log")
        assertLive(Self.plainGlobeLive, in: app, "The clouds belong to the globe: no stage, nothing more live")
        app.buttons["workbenchClose"].firstMatch.tap()

        clouds.tap()
        XCTAssertTrue(says("Off"), "The switch turns them off")
        sleep(1)   // faded out
        attach("44-clouds-off", in: app)
        clouds.tap()
        XCTAssertTrue(says("On"), "The switch brings them back")
        sleep(1)
        attach("45-clouds-on-again", in: app)
    }
}
