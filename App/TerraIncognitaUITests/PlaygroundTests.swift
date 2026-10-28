import XCTest

/// The playground's promises: the app opens on the plain globe; any prototype can be started, left
/// and followed by another without leaving anything behind; parked work is one tap away; adding a
/// prototype adds a row and changes none of the others.
final class PlaygroundTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    @MainActor func testColdLaunchIsThePlainGlobe() {
        let app = XCUIApplication()
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 45))
        XCTAssertTrue(app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch.exists, "The globe is the opening view")
        XCTAssertTrue(text(containing: "The whole world", in: app).exists, "The caption still says where the reader is")
        for absent in ["activePrototype", "camelidAnimal", "whaleFossil", "oceanSeed", "oceanInvitation", "closeFilm", "filmLook"] {
            XCTAssertFalse(app.descendants(matching: .any)[absent].firstMatch.exists, "The plain globe shows no \(absent)")
        }
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
        for id in ["plain", "camelid", "whale", "ocean", "gathering-of-water", "sea-between", "hello-pack", "sounding-the-deep", "borders-on-old-maps", "persian", "wikipedia", "film"] {
            XCTAssertTrue(listed.contains(id), "The workbench lists \(id): \(listed.sorted())")
        }
    }

    /// A native prototype on the globe and its screen, then a pack lying on the globe, then the atlas
    /// again: each start ends the one before, and nothing they made outlives them.
    @MainActor func testTwoDissimilarPrototypesInSuccessionLeaveNothingBehind() {
        let app = XCUIApplication()
        app.launch()
        defer { app.terminate() }

        startPrototype("camelid", in: app)
        let camel = app.buttons["camelidAnimal"].firstMatch
        XCTAssertTrue(camel.waitForExistence(timeout: 30), "The camelid places its dromedary on the globe")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("Camel"))
        attach("10-camelid-on-globe", in: app)
        camel.tap()
        let close = app.buttons["camelidClose"].firstMatch
        XCTAssertTrue(close.waitForExistence(timeout: 20), "Its marker opens its screen")
        attach("11-camelid-screen", in: app)
        close.tap()
        XCTAssertTrue(camel.waitForExistence(timeout: 15), "Closing its screen leaves it on the globe until ended")

        startPrototype("sea-between", in: app)
        XCTAssertFalse(app.buttons["camelidAnimal"].firstMatch.exists, "Starting another ends the first")
        XCTAssertTrue(app.descendants(matching: .any)["globeFootCaption"].firstMatch.waitForExistence(timeout: 45),
                      "The leaf, a pack now, lies on the globe and says what it is")
        Thread.sleep(forTimeInterval: 1.5)
        attach("12-leaf", in: app)
        endPrototype(in: app)

        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 15))
        XCTAssertFalse(app.descendants(matching: .any)["activePrototype"].firstMatch.exists, "Ending it from its chip ends it")
        XCTAssertFalse(app.buttons["camelidAnimal"].firstMatch.exists)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Back on the plain globe, nothing from either prototype is alive")
        attach("13-live-after", in: app)
    }

    /// The Persian and Wikipedia styles and the films are parked, not gone.
    @MainActor func testParkedDirectionsRemainOneTapAway() {
        let app = XCUIApplication()
        app.launch()
        defer { app.terminate() }

        startPrototype("persian", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 10))
        attach("20-persian", in: app)
        endPrototype(in: app)

        startPrototype("wikipedia", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 10))
        attach("20-wikipedia", in: app)
        endPrototype(in: app)

        startPrototype("film", in: app)
        XCTAssertTrue(app.buttons["closeFilm"].firstMatch.waitForExistence(timeout: 30), "The film opens on its own screen")
        XCTAssertTrue(app.buttons["filmLook"].firstMatch.exists, "Its looks remain")
        Thread.sleep(forTimeInterval: 2)
        attach("21-film", in: app)
        app.buttons["closeFilm"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 15))
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "The film's canvas is released when it closes")
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
