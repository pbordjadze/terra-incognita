import XCTest

/// Packs on the globe (runtime 3 and 4): each starts from the workbench, says at the foot what the
/// reader is looking at, shows its panel from the chip's Open, answers a touch on the world, and leaves
/// nothing behind when it ends. The river study came from a web page; the monsoon's sails and the
/// whales were rebuilt from scratch as packs (painting-company row 176).
final class MovedPackTests: XCTestCase {
    override func setUp() { continueAfterFailure = false }

    @MainActor func testTheMonsoonsSailsLieOnTheGlobe() {
        checkPack("monsoon-sails", about: "The monsoon's sails", shots: "60")
    }

    @MainActor func testTheWhalesLieOnTheGlobe() {
        checkPack("sea-of-whales", about: "Whales in the desert", shots: "62")
    }

    @MainActor func testAGatheringOfWaterLiesOnTheGlobe() {
        checkPack("gathering-of-water", about: "A gathering of water", shots: "65")
    }

    /// `about`: the title of the page the pack's Open shows (not one of its notes, which its touchable sprites open).
    @MainActor private func checkPack(_ id: String, about: String, shots: String, file: StaticString = #filePath, line: UInt = #line) {
        let app = XCUIApplication()
        app.launchArguments = ["-playground-reset"]
        app.launch()
        defer { app.terminate() }
        startPrototype(id, in: app)
        let foot = app.descendants(matching: .any)["globeFootCaption"].firstMatch
        XCTAssertTrue(foot.waitForExistence(timeout: 30), "\(id) says at the foot what this is", file: file, line: line)
        XCTAssertFalse(foot.label.isEmpty, "\(id)'s caption has words", file: file, line: line)
        Thread.sleep(forTimeInterval: 2)
        attach("\(shots)-\(id)-on-globe", in: app)

        // a touch on the world near the middle of the globe: whatever the pack makes of it, it keeps running
        let globe = app.descendants(matching: .any)["mainAtlasGlobe"].firstMatch
        globe.coordinate(withNormalizedOffset: CGVector(dx: 0.5, dy: 0.45)).tap()
        Thread.sleep(forTimeInterval: 1)
        XCTAssertTrue(app.staticTexts["activePrototype"].firstMatch.exists, "\(id) is still running after a touch", file: file, line: line)
        attach("\(shots)-\(id)-touched", in: app)

        let open = app.buttons["openActivePrototype"].firstMatch
        XCTAssertTrue(open.waitForExistence(timeout: 10), "\(id) offers Open", file: file, line: line)
        open.tap()
        let title = app.descendants(matching: .any)["packPanelTitle"].firstMatch
        XCTAssertTrue(title.waitForExistence(timeout: 15), "\(id)'s Open shows its panel", file: file, line: line)
        XCTAssertEqual(title.label, about, "\(id)'s Open shows its own page", file: file, line: line)
        attach("\(shots)-\(id)-panel", in: app)
        app.buttons["packPanelClose"].firstMatch.tap()
        XCTAssertTrue(app.descendants(matching: .any)["packPanel"].firstMatch.waitForNonExistence(timeout: 10), file: file, line: line)

        endPrototype(in: app)
        openWorkbench(in: app)
        assertLive(Self.plainGlobeLive, in: app, "Ending \(id) leaves nothing it made", file: file, line: line)
    }
}
