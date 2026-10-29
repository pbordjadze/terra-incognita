import XCTest

/// A lens: the camel's lineage composes on the playground's one globe. While it is open the globe is the
/// sequence's (it answers to the sequence's name, and the playground's buttons give it the screen);
/// closing it gives the globe back as the reader had it: its own name and words, the camera where it
/// was, the doorway's camel where it stood, and no second globe alive. Part of `PlaygroundTests`, so
/// `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testLensBorrowsTheOneGlobeAndGivesItBack() {
        let app = XCUIApplication()
        app.launchArguments = ["-prototype", "camelid", "-globe-at", "46", "24", "3.2"]
        app.launch()
        defer { app.terminate() }

        let camel = app.descendants(matching: .any)["camelidAnimal"].firstMatch
        XCTAssertTrue(camel.waitForExistence(timeout: 45), "The camel is on the globe")
        let atlas = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        XCTAssertTrue(atlas.exists, "The playground's globe")
        let words = atlas.label
        let place = camel.frame
        camel.tap()

        let lent = app.descendants(matching: .any)["camelidGlobe"].firstMatch
        let follow = app.buttons["camelidFollow"].firstMatch
        XCTAssertTrue(follow.waitForExistence(timeout: 15), "The camel opens its sequence")
        XCTAssertTrue(lent.exists, "The sequence describes the one globe while it is open")
        XCTAssertFalse(atlas.exists, "No other globe answers meanwhile")
        XCTAssertFalse(app.buttons["workbenchButton"].firstMatch.exists, "The sequence has the screen to itself")
        follow.tap()
        let pause = app.buttons["camelidPause"].firstMatch
        XCTAssertTrue(pause.waitForExistence(timeout: 10))
        Thread.sleep(forTimeInterval: 3)   // the sequence's camera on its way
        attach("60-lens-camelid", in: app)
        pause.tap()
        app.buttons["camelidClose"].firstMatch.tap()

        XCTAssertTrue(atlas.waitForExistence(timeout: 15), "Closing gives the globe back its own name")
        XCTAssertFalse(lent.exists)
        XCTAssertEqual(atlas.label, words, "and its own words")
        XCTAssertTrue(camel.waitForExistence(timeout: 15), "The doorway's camel shows again")
        let back = camel.frame
        XCTAssertEqual(back.midX, place.midX, accuracy: 2, "The camera is back where the reader had it")
        XCTAssertEqual(back.midY, place.midY, accuracy: 2, "The camera is back where the reader had it")
        attach("61-lens-closed", in: app)
        openWorkbench(in: app)
        assertLive("stages 1 · markers 1 · globes 1 · films 0 · scripts 0", in: app,
                   "The camel still runs on the globe, and the sequence left no globe of its own")
        app.buttons["workbenchClose"].firstMatch.tap()
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending it leaves only the plain globe")
    }
}
