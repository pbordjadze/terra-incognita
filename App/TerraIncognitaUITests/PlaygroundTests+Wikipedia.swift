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
        func gone(_ element: XCUIElement) -> Bool {
            let away = XCTNSPredicateExpectation(predicate: NSPredicate(format: "exists == false"), object: element)
            return XCTWaiter().wait(for: [away], timeout: 10) == .completed
        }
        XCTAssertTrue(says("Off"), "Off until the reader turns it on")
        XCTAssertTrue(text(containing: "The whole world", in: app).exists, "The plain globe's own caption")

        // the whales in the desert: a pack that writes nothing at the foot but its name, and keeps
        // its own paragraphs for its panel
        startPrototype("sea-of-whales", in: app)
        let foot = app.descendants(matching: .any)["globeFootCaption"].firstMatch
        XCTAssertTrue(foot.waitForExistence(timeout: 30), "The pack names itself at the foot")
        XCTAssertTrue(foot.label.lowercased().contains("whales in the desert"), "Its name at the foot (got “\(foot.label)”)")
        // (Wikipedia's first sentence on Wadi Al-Hitan, from the pack's own wikipedia.json)
        let sentence = text(containing: "paleontological site", in: app)
        XCTAssertFalse(sentence.exists, "Nothing of Wikipedia's while the switch is off")
        XCTAssertFalse(app.descendants(matching: .any)["wikipediaLink"].firstMatch.exists, "No link while the switch is off")
        let open = app.buttons["openActivePrototype"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 10))
        open.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanelTitle"].firstMatch.waitForExistence(timeout: 15), "Open shows its panel")
        XCTAssertTrue(app.descendants(matching: .any)["packPanelParagraph"].firstMatch.waitForExistence(timeout: 5), "The pack's own paragraphs")
        XCTAssertFalse(app.descendants(matching: .any)["wikipediaCard"].firstMatch.exists, "And no card of Wikipedia's")
        attach("50-wikipedia-off-panel", in: app)
        app.buttons["packPanelClose"].firstMatch.tap()

        // on: Wikipedia's first sentence on the valley at the foot, and the link to the article
        words.tap()
        XCTAssertTrue(says("On"), "The switch turns Wikipedia's words on")
        XCTAssertTrue(sentence.waitForExistence(timeout: 10), "Wikipedia's sentence at the foot")
        XCTAssertTrue(app.descendants(matching: .any)["wikipediaLink"].firstMatch.waitForExistence(timeout: 5), "A link to the article under it")
        attach("51-wikipedia-on-foot", in: app)

        // its panel: Wikipedia's cards where the paragraphs were, and none of the pack's own
        open.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanelTitle"].firstMatch.waitForExistence(timeout: 15), "Open shows its panel")
        XCTAssertTrue(app.descendants(matching: .any)["wikipediaCard"].firstMatch.waitForExistence(timeout: 5), "A card where the paragraph was")
        XCTAssertFalse(app.descendants(matching: .any)["packPanelParagraph"].firstMatch.exists, "No paragraph of the pack's own")
        attach("52-wikipedia-on-panel", in: app)
        app.buttons["packPanelClose"].firstMatch.tap()

        // off again: the pack's words back, nothing else changed
        words.tap()
        XCTAssertTrue(says("Off"), "The switch turns them off")
        XCTAssertTrue(gone(sentence), "Wikipedia's sentence leaves the foot")
        XCTAssertTrue(foot.label.lowercased().contains("whales in the desert"), "The pack's name is still there")
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
