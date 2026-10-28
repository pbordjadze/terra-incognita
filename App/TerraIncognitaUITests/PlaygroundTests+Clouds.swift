import XCTest

/// The clouds (`App/TerraIncognita/Clouds/`): listed, drawn over the globe with its names still
/// there, a real pinch down through them, what a frame costs in the workbench's log, and nothing
/// left when they end. Part of `PlaygroundTests`, so `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testCloudsComeDownAndGoWithoutLeavingAnything() {
        let app = XCUIApplication()
        // October's sky, its drift held: the same pictures every run
        app.launchArguments = ["-clouds-days", "0.4", "-clouds-month", "10"]
        app.launch()
        defer { app.terminate() }

        startPrototype("clouds", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 15))
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("Clouds"), "The chip names the clouds")
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        XCTAssertTrue(globe.waitForExistence(timeout: 15), "The clouds are drawn over the globe, which stays")
        sleep(4)   // the flight to Africa, and the sky decoded
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
        app.buttons["workbenchClose"].firstMatch.tap()

        endPrototype(in: app)
        attach("44-clouds-gone", in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the clouds leaves only the globe")
    }
}
