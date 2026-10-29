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
        for absent in ["activePrototype", "camelidAnimal", "closeFilm", "filmLook"] {
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
        for id in ["plain", "camelid", "gathering-of-water", "hello-pack", "sea-of-whales", "monsoon-sails", "fires-across-the-sea", "lead-line", "low-sea", "vine", "quiet-names", "quiet-clouds", "night", "quiet-frame", "persian", "wikipedia", "still-sea", "film"] {
            XCTAssertTrue(listed.contains(id), "The workbench lists \(id): \(listed.sorted())")
        }
    }

    /// A native prototype on the globe and its sequence on the same globe, then a pack lying on the
    /// globe beside it (runtime 4: starting another no longer ends the first), each ended from the chip
    /// in turn, then the atlas again: nothing they made outlives them.
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
        XCTAssertTrue(close.waitForExistence(timeout: 20), "Its marker opens its sequence, on the one globe")
        XCTAssertTrue(app.descendants(matching: .any)["camelidGlobe"].firstMatch.exists, "The sequence has the globe meanwhile")
        attach("11-camelid-screen", in: app)
        close.tap()
        XCTAssertTrue(camel.waitForExistence(timeout: 15), "Closing the sequence leaves it on the globe until ended")
        XCTAssertTrue(app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch.exists, "and gives the globe back")

        startPrototype("lead-line", in: app)
        XCTAssertTrue(app.descendants(matching: .any)["globeFootCaption"].firstMatch.waitForExistence(timeout: 45),
                      "The lead line, a pack, lies on the globe and says what it is")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("lead line"), "The lead line is in front")
        XCTAssertTrue(app.staticTexts["otherPrototypes"].firstMatch.label.contains("1 more"), "The camel runs on behind it (several at once)")
        Thread.sleep(forTimeInterval: 1.5)
        attach("12-lead-line", in: app)
        endFront(leaving: "Camel", in: app)
        XCTAssertFalse(app.staticTexts["otherPrototypes"].firstMatch.exists, "Ending the lead line leaves the camel alone in front")
        endPrototype(in: app)

        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 15))
        XCTAssertFalse(app.descendants(matching: .any)["activePrototype"].firstMatch.exists, "Ending it from its chip ends it")
        XCTAssertFalse(app.buttons["camelidAnimal"].firstMatch.exists)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Back on the plain globe, nothing from either prototype is alive")
        attach("13-live-after", in: app)
    }

    /// The living globe (runtime 4): the doorways alive on the globe at once, three dissimilar
    /// experiences entered by touching them (a lens, another lens, a pack), the chip naming the one in
    /// front and counting the others, and each ended from the chip in turn with the Live counts back
    /// exactly to what they were before it.
    @MainActor func testSeveralExperiencesLiveTogetherByTouch() {
        let app = XCUIApplication()
        // held where the camel's, the whales' and the fires' doorways are all in view on the 13-inch iPad
        app.launchArguments = ["-playground-reset", "-globe-at", "33", "30", "2"]
        app.launch()
        defer { app.terminate() }
        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 45))
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Before anything: the plain globe")
        app.buttons["workbenchClose"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10))

        // the camel, by its doorway: a native one opens straight away, on the one globe; closing its
        // sequence leaves it running on the globe, and the camera where the reader had it
        let camelDoor = app.buttons["doorway.camelid"].firstMatch
        XCTAssertTrue(camelDoor.waitForExistence(timeout: 20), "The camel's doorway is alive on the globe")
        attach("70-doorways-alive", in: app)
        camelDoor.tap()
        let camelClose = app.buttons["camelidClose"].firstMatch
        XCTAssertTrue(camelClose.waitForExistence(timeout: 20), "Touching the camel's doorway opens its sequence")
        camelClose.tap()
        XCTAssertTrue(app.buttons["camelidAnimal"].firstMatch.waitForExistence(timeout: 15), "The camel stays on the globe")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("Camel"))

        // the whales, a pack, by their doorway at Wadi Al-Hitan: the script runs beside the camel
        let whalesDoor = app.buttons["doorway.sea-of-whales"].firstMatch
        XCTAssertTrue(whalesDoor.waitForExistence(timeout: 15), "The whales' doorway is still there, in view")
        whalesDoor.tap()
        XCTAssertTrue(app.descendants(matching: .any)["globeFootCaption"].firstMatch.waitForExistence(timeout: 45), "The whales start and say what they are")
        XCTAssertTrue(app.buttons["camelidAnimal"].firstMatch.exists, "The camel is still there: nothing was reset")
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("Whales"), "The whales are in front")
        XCTAssertTrue(app.staticTexts["otherPrototypes"].firstMatch.label.contains("1 more"), "The camel runs on behind")
        attach("71-two-alive", in: app)

        // the fires, a pack, by their doorway in the Aegean: a second script runs beside both
        let firesDoor = app.buttons["doorway.fires-across-the-sea"].firstMatch
        XCTAssertTrue(firesDoor.waitForExistence(timeout: 15), "The fires' doorway is in view")
        firesDoor.tap()
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.label.contains("Fires"), "The fires are in front")
        XCTAssertTrue(app.staticTexts["otherPrototypes"].firstMatch.label.contains("2 more"), "The camel and the whales run on behind")
        Thread.sleep(forTimeInterval: 1.5)
        attach("72-three-alive", in: app)
        openWorkbench(in: app)
        assertLive(from: "stages 3 · markers ", to: " · globes 1 · films 0 · scripts 2", in: app,
                   "Three stages, two scripts, the one globe (the packs' own sprites are their own count)")
        app.buttons["workbenchClose"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10))

        // ending each from the chip, front first: the counts come back exactly
        endFront(leaving: "Whales", in: app)
        openWorkbench(in: app)
        assertLive(from: "stages 2 · markers ", to: " · globes 1 · films 0 · scripts 1", in: app, "Ending the fires leaves the camel and the whales as they were")
        app.buttons["workbenchClose"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10))
        endFront(leaving: "Camel", in: app)
        openWorkbench(in: app)
        assertLive("stages 1 · markers 1 · globes 1 · films 0 · scripts 0", in: app, "Ending the whales leaves the camel as it was")
        app.buttons["workbenchClose"].firstMatch.tap()
        XCTAssertTrue(app.buttons["workbenchClose"].firstMatch.waitForNonExistence(timeout: 10))
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending the camel leaves the plain globe")
        attach("73-plain-again", in: app)
    }

    /// The Persian and Wikipedia styles, the still sea and the films are parked, not gone.
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

        startPrototype("still-sea", in: app)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.waitForExistence(timeout: 10))
        Thread.sleep(forTimeInterval: 1)
        attach("20-still-sea", in: app)
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
