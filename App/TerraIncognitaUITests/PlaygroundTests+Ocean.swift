import XCTest

/// The ocean (`App/TerraIncognita/Ocean/`), part of the stage: the month's currents over the opening
/// globe with its names still there, closer in down to the surf, a touch on the water ringing it, what
/// a frame costs in the workbench's log, and the Live counts the plain globe's throughout. Part of
/// `PlaygroundTests`, so `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testTheOceanMovesAndRingsWhereTouched() {
        let app = XCUIApplication()
        // October's sea off the Carolinas, its light held at a moment: the same pictures every run
        app.launchArguments = ["-ocean-time", "12", "-ocean-month", "10", "-globe-at", "-68", "36", "2.6"]
        app.launch()
        defer { app.terminate() }

        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        XCTAssertTrue(globe.waitForExistence(timeout: 15))
        // (the clouds off for these pictures, if they are on; the switch is remembered, so put back)
        let clouds = app.descendants(matching: .any)["cloudsButton"].firstMatch
        let cloudsWereOn = clouds.waitForExistence(timeout: 10) && clouds.value as? String == "On"
        if cloudsWereOn { clouds.tap() }
        sleep(3)   // the currents decoded, and faded in
        attach("50-ocean-gulf-stream", in: app)

        // a touch on the open water rings it
        globe.coordinate(withNormalizedOffset: CGVector(dx: 0.62, dy: 0.5)).tap()
        Thread.sleep(forTimeInterval: 1.2)
        attach("51-ocean-touched", in: app)

        // closer, to the surf
        globe.pinch(withScale: 2.2, velocity: 2)
        sleep(2)
        attach("52-ocean-closer", in: app)
        sleep(9)   // a report in the workbench's log every ten seconds while it draws

        openWorkbench(in: app)
        let report = text(containing: "ocean:", in: app)
        revealInWorkbench(report, in: app)
        let cost = XCTAttachment(string: report.exists ? report.label : "no report from the ocean")
        cost.name = "53-ocean-cost"
        cost.lifetime = .keepAlways
        add(cost)
        XCTAssertTrue(report.exists, "The ocean says what a frame costs, in the workbench's log")
        assertLive(Self.plainGlobeLive, in: app, "The ocean belongs to the globe: no stage, nothing more live")
        app.buttons["workbenchClose"].firstMatch.tap()
        if cloudsWereOn { clouds.tap() }
    }
}
