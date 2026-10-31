import XCTest

/// The playground's promises: the app opens on the plain globe; any prototype can be started, left
/// and followed by another without leaving anything behind; adding a prototype adds a row and changes
/// none of the others.
final class PlaygroundTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    @MainActor func testColdLaunchIsThePlainGlobe() {
        let app = XCUIApplication()
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch.exists, "The globe is the opening view")
        XCTAssertTrue(text(containing: "The whole world", in: app).exists, "The caption still says where the reader is")
        XCTAssertFalse(app.descendants(matching: .any)["activePrototype"].firstMatch.exists, "The plain globe runs no prototype")
        attach("00-plain-globe", in: app)
        openWorkbench(in: app)
        attach("01-workbench", in: app)
        assertLive(Self.plainGlobeLive, in: app, "The plain globe keeps only itself alive")
    }

    /// Additive registry: every existing entry is still listed (a new one only adds a row).
    @MainActor func testEveryPrototypeIsListed() {
        let app = XCUIApplication()
        app.launch()
        defer { app.terminate() }
        openWorkbench(in: app)
        let listed = listedPrototypes(in: app)
        attach("01-workbench", in: app)
        for id in ["plain", "hello-pack", "quiet-names", "quiet-clouds", "night", "quiet-frame"] {
            XCTAssertTrue(listed.contains(id), "The workbench lists \(id): \(listed.sorted())")
        }
    }

    /// A native prototype restyling the globe, then a pack lying on it beside it (runtime 4: starting
    /// another doesn't end the first), each ended from the chip in turn: nothing they made outlives them.
    @MainActor func testTwoDissimilarPrototypesInSuccessionLeaveNothingBehind() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }

        startPrototype("night", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 15))
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("night"), "The night is in front")
        Thread.sleep(forTimeInterval: 2)   // the night's map read, the style crossfaded
        attach("10-night", in: app)

        startPrototype("hello-pack", in: app)
        XCTAssertTrue(text(containing: "First words.", in: app).waitForExistence(timeout: 30), "The pack's script runs beside the night")
        XCTAssertTrue(app.staticTexts["otherPrototypes"].firstMatch.label.contains("1 more"), "The night runs on behind it (several at once)")
        attach("11-night-and-pack", in: app)
        openWorkbench(in: app)
        assertLive(from: "stages 2 · markers ", to: " · globes 1 · films 0 · scripts 1", in: app, "Two stages, one script, the one globe")
        app.buttons["workbenchClose"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10))
        endFront(leaving: "The Earth at night", in: app)
        XCTAssertFalse(app.staticTexts["otherPrototypes"].firstMatch.exists, "Ending the pack leaves the night alone in front")
        endPrototype(in: app)

        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 15))
        XCTAssertFalse(app.descendants(matching: .any)["activePrototype"].firstMatch.exists, "Ending it from its chip ends it")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Back on the plain globe, nothing from either prototype is alive")
        attach("12-live-after", in: app)
    }

    /// The bundled pack: its script runs in the app (no page, no web view), says at the foot which copy
    /// it is, shows its panel from the chip's Open, and leaves nothing when it ends.
    @MainActor func testBundledPackRunsOnTheGlobe() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype("hello-pack", in: app)
        XCTAssertTrue(text(containing: "First words.", in: app).waitForExistence(timeout: 30), "The bundled pack's script runs")
        XCTAssertTrue(text(containing: "Bundled", in: app).exists, "It knows where it came from")
        attach("30-pack-on-globe", in: app)
        let open = app.buttons["openActivePrototype"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 10), "Its script listens for Open")
        open.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanelTitle"].firstMatch.waitForExistence(timeout: 15), "Open shows its panel")
        attach("31-pack-panel", in: app)
        app.buttons["packPanelClose"].firstMatch.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanel"].firstMatch.waitForNonExistence(timeout: 10))
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "The pack's script and everything it asked for are released")
    }
}
