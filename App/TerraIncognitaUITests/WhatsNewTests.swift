import XCTest

/// What's new (painting-company row 187), used as the owner would: a build with pieces due offers its
/// pill; the round plays each piece from its start with the bar above and the taps below; a tap
/// answers and the next comes; a picture of the screen is a picture (the globe in it, not black); the
/// round ends on its card; the workbench keeps the way back to any piece, which plays alone; leaving
/// puts the plain globe back with nothing alive. Screenshots at each step are the round log's eyes.
final class WhatsNewTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    /// The system's microphone sheet, if it comes: allowed, so the round goes on.
    @MainActor private func allowMicrophone() {
        addUIInterruptionMonitor(withDescription: "microphone") { alert in
            for name in ["Allow", "OK"] where alert.buttons[name].exists { alert.buttons[name].tap(); return true }
            return false
        }
    }

    @MainActor private func progress(in app: XCUIApplication) -> String {
        let p = app.staticTexts["whatsNewProgress"].firstMatch
        return p.exists ? p.label : ""
    }

    @MainActor private func waitForProgress(_ prefix: String, in app: XCUIApplication, _ why: String) {
        let p = app.staticTexts["whatsNewProgress"].firstMatch
        let ok = XCTNSPredicateExpectation(predicate: NSPredicate(format: "label BEGINSWITH %@", prefix), object: p)
        XCTAssertEqual(XCTWaiter().wait(for: [ok], timeout: 20), .completed, "\(why): \(p.exists ? p.label : "no progress")")
    }

    @MainActor func testTheRoundOnThisBuild() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset", "-whatsnew-reset", "-whatsnew"]
        allowMicrophone()
        app.launch()
        defer { app.terminate() }

        let bar = app.otherElements["whatsNewBar"].firstMatch
        XCTAssertTrue(bar.waitForExistence(timeout: 60), "Launched into the round, the first piece's bar is over the globe")
        XCTAssertTrue(progress(in: app).hasPrefix("1 of "), "The bar counts from one: \(progress(in: app))")
        let total = Int(progress(in: app).split(separator: " ").last ?? "") ?? 0
        XCTAssertEqual(total, 5, "A round is five pieces at a time (the twelve seeded wait their turn)")
        XCTAssertTrue(app.buttons["react_wow"].firstMatch.waitForExistence(timeout: 10), "The taps are under the piece")
        XCTAssertFalse(app.buttons["workbenchButton"].firstMatch.isHittable, "The playground's own controls stand back while a piece plays")
        Thread.sleep(forTimeInterval: 2.5)
        attach("20-first-piece", in: app)

        // one tap answers and the next piece comes
        app.buttons["react_wow"].firstMatch.tap()
        waitForProgress("2 of", in: app, "A tap moves to the second piece")
        Thread.sleep(forTimeInterval: 3)
        attach("21-second-piece", in: app)

        // a picture of the screen is a picture of the globe, not of nothing
        app.buttons["whatsNewCamera"].firstMatch.tap()
        let picture = app.images["whatsNewPicture"].firstMatch
        XCTAssertTrue(picture.waitForExistence(timeout: 10), "Taking a picture shows it a moment")
        let label = picture.label
        let dark = Int(label.split(separator: " ").compactMap { Int($0) }.first ?? 100)
        XCTAssertLessThan(dark, 70, "The picture has the globe in it (\(label))")
        attach("22-picture-taken", in: app)

        // a voice note: tapped on, tapped off (the simulator may have no microphone; the round goes on either way)
        app.buttons["whatsNewMic"].firstMatch.tap()
        // (an inert touch, so the system's microphone sheet, if it came, is answered by the monitor)
        app.staticTexts["whatsNewTitle"].firstMatch.tap()
        Thread.sleep(forTimeInterval: 1.5)
        attach("23-recording", in: app)
        let mic = app.buttons["whatsNewMic"].firstMatch
        if mic.exists, mic.label.contains("Stop") { mic.tap() }

        // the way back within the round
        app.buttons["whatsNewBack"].firstMatch.tap()
        waitForProgress("1 of", in: app, "Back shows the piece before")
        app.buttons["react_nice"].firstMatch.tap()
        waitForProgress("2 of", in: app, "Answering again moves on")

        // skip through to the end (the taps wait while the next piece is put in place)
        var at = 2
        while at < total {
            let skip = app.buttons["react_skip"].firstMatch
            XCTAssertTrue(skip.waitForExistence(timeout: 15), "Skip is there under piece \(at)")
            let ready = XCTNSPredicateExpectation(predicate: NSPredicate(format: "isEnabled == true"), object: skip)
            _ = XCTWaiter().wait(for: [ready], timeout: 15)
            skip.tap()
            at += 1
            waitForProgress("\(at) of", in: app, "Skip moves on to piece \(at)")
            if at == 3 { Thread.sleep(forTimeInterval: 2); attach("23b-third-piece", in: app) }
        }
        let last = app.buttons["react_skip"].firstMatch
        let lastReady = XCTNSPredicateExpectation(predicate: NSPredicate(format: "isEnabled == true"), object: last)
        _ = XCTWaiter().wait(for: [lastReady], timeout: 15)
        last.tap()
        let finish = app.otherElements["whatsNewFinish"].firstMatch
        XCTAssertTrue(finish.waitForExistence(timeout: 20), "After the last piece, the card")
        let tally = app.staticTexts["whatsNewTally"].firstMatch
        XCTAssertTrue(tally.label.contains("nice") && tally.label.contains("waiting"), "The card says what was said and what waits: \(tally.label)")
        XCTAssertTrue(app.buttons["whatsNewKeepGoing"].firstMatch.exists, "More pieces wait: the card offers to keep going")
        XCTAssertTrue(app.buttons["whatsNewSetUp"].firstMatch.exists || app.buttons["whatsNewSend"].firstMatch.exists, "The card offers the way to the company")
        attach("24-finish", in: app)
        app.buttons["whatsNewDone"].firstMatch.tap()
        XCTAssertTrue(finish.waitForNonExistence(timeout: 10))
        XCTAssertTrue(app.buttons["whatsNewPill"].firstMatch.waitForExistence(timeout: 10), "Pieces still wait: the pill is back with their count")

        // the plain globe again, nothing alive, and the way back in the workbench
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "After the round the plain globe keeps only itself alive")
        let row = app.buttons["whatsNewRow"].firstMatch
        revealInWorkbench(row, in: app)
        XCTAssertTrue(row.exists, "The workbench has the way back")
        row.tap()
        XCTAssertTrue(app.collectionViews["whatsNewList"].firstMatch.waitForExistence(timeout: 10) || app.otherElements["whatsNewList"].firstMatch.waitForExistence(timeout: 5), "The list of every piece")
        attach("25-list", in: app)
        let herds = app.buttons["piece_herds"].firstMatch
        if !herds.exists { app.swipeUp() }
        XCTAssertTrue(herds.waitForExistence(timeout: 10), "The herds are a piece")
        herds.tap()
        XCTAssertTrue(bar.waitForExistence(timeout: 30), "A piece from the list plays alone")
        XCTAssertEqual(progress(in: app), "1 of 1")
        Thread.sleep(forTimeInterval: 4)
        attach("26-single-piece", in: app)
        app.buttons["whatsNewLeave"].firstMatch.tap()
        XCTAssertTrue(bar.waitForNonExistence(timeout: 10), "Leaving takes the bar away")
        XCTAssertTrue(app.buttons["workbenchButton"].firstMatch.waitForExistence(timeout: 10), "and gives the playground's controls back")
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Leaving a piece leaves nothing alive")
    }

    @MainActor func testThePillOnLaunch() {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset", "-whatsnew-reset"]
        app.launch()
        defer { app.terminate() }
        let pill = app.buttons["whatsNewPill"].firstMatch
        XCTAssertTrue(pill.waitForExistence(timeout: 60), "A build with pieces due says so on the globe")
        XCTAssertTrue(pill.label.contains("new"), pill.label)
        Thread.sleep(forTimeInterval: 2)
        attach("27-pill", in: app)
        pill.tap()
        let bar = app.otherElements["whatsNewBar"].firstMatch
        XCTAssertTrue(bar.waitForExistence(timeout: 30), "The pill starts the round")
        XCTAssertFalse(pill.exists, "and the pill steps aside")
        app.buttons["whatsNewLeave"].firstMatch.tap()
        XCTAssertTrue(bar.waitForNonExistence(timeout: 10))
        XCTAssertTrue(pill.waitForExistence(timeout: 10), "Nothing answered: the pill is back")
    }
}
