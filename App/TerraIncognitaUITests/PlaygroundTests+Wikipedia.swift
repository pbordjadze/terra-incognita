import XCTest

/// Wikipedia's words (`App/TerraIncognita/Wikipedia/`): the switch beside the clouds' puts Wikipedia's
/// own words where the atlas's paragraphs were, at the foot of a pack and in its panel, and gives the
/// atlas's back when turned off, with the Live counts untouched. Part of `PlaygroundTests`, so
/// `[playground-check]` runs it.
extension PlaygroundTests {
    @MainActor @objc func testWikipediasWordsStandWhereTheAtlassWere() {
        let app = XCUIApplication()
        // the switch off and unremembered, whatever a run before left
        app.launchArguments = ["-playground-reset", "-wikipedia-words", "off"]
        app.launch()
        defer { app.terminate() }

        XCTAssertTrue(app.buttons["workbenchButton"].waitForExistence(timeout: 45))
        // (a toggle: VoiceOver and the tests see it as a switch, so it is found by its identifier alone)
        let words = app.descendants(matching: .any)["wikipediaButton"].firstMatch
        XCTAssertTrue(words.waitForExistence(timeout: 10), "The switch is beside the clouds'")
        func says(_ value: String) -> Bool {
            let said = XCTNSPredicateExpectation(predicate: NSPredicate(format: "value == %@", value), object: words)
            return XCTWaiter().wait(for: [said], timeout: 5) == .completed
        }
        XCTAssertTrue(says("Off"), "Off until the reader turns it on")
        XCTAssertTrue(text(containing: "The whole world", in: app).exists, "The plain globe's own caption")

        // the sea between, in the atlas's own words
        startPrototype("sea-between", in: app)
        let own = text(containing: "engraved chart", in: app)
        XCTAssertTrue(own.waitForExistence(timeout: 30), "The pack's own words at the foot")
        XCTAssertFalse(app.descendants(matching: .any)["wikipediaLink"].firstMatch.exists, "No link while the switch is off")
        attach("50-wikipedia-off-foot", in: app)

        // on: Wikipedia's first sentence on the Aegean, and the link to the article
        words.tap()
        XCTAssertTrue(says("On"), "The switch turns Wikipedia's words on")
        XCTAssertTrue(text(containing: "Aegean Sea is", in: app).waitForExistence(timeout: 10), "Wikipedia's sentence at the foot")
        XCTAssertTrue(app.descendants(matching: .any)["wikipediaLink"].firstMatch.waitForExistence(timeout: 5), "A link to the article under it")
        XCTAssertFalse(own.exists, "The atlas's paragraph is gone")
        attach("51-wikipedia-on-foot", in: app)

        // its panel: Wikipedia's cards where the paragraphs were, and none of the atlas's paragraphs
        let open = app.buttons["openActivePrototype"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 10))
        open.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanelTitle"].firstMatch.waitForExistence(timeout: 15), "Open shows its panel")
        XCTAssertTrue(app.descendants(matching: .any)["wikipediaCard"].firstMatch.waitForExistence(timeout: 5), "A card where the paragraph was")
        XCTAssertFalse(app.descendants(matching: .any)["packPanelParagraph"].firstMatch.exists, "No paragraph of the atlas's own")
        attach("52-wikipedia-on-panel", in: app)
        app.buttons["packPanelClose"].firstMatch.tap()

        // off again: the atlas's words back, nothing else changed
        words.tap()
        XCTAssertTrue(says("Off"), "The switch turns them off")
        XCTAssertTrue(own.waitForExistence(timeout: 10), "The pack's own words are back")
        XCTAssertFalse(app.descendants(matching: .any)["wikipediaLink"].firstMatch.exists, "And the link is gone")
        open.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanelParagraph"].firstMatch.waitForExistence(timeout: 15), "Its paragraphs are back")
        XCTAssertFalse(app.descendants(matching: .any)["wikipediaCard"].firstMatch.exists)
        app.buttons["packPanelClose"].firstMatch.tap()
        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "The switch belongs to the reader, not the stage: nothing more live")
    }
}
