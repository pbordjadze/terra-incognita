import XCTest
import UIKit

/// Verify the actual bundled WKWebView and its return route; the browser harness covers the camera.
final class LeafTests: XCTestCase {
    override func setUpWithError() throws { continueAfterFailure = false }

    @MainActor private func capture(_ name: String) {
        Thread.sleep(forTimeInterval: 0.7)
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    @MainActor private func geographyIsVisible() -> Bool {
        // Inspect the actual screen, independently of DOM readiness or tappable controls.
        guard let image = XCUIScreen.main.screenshot().image.cgImage else { return false }
        let width = 160, height = max(1, image.height * 160 / image.width)
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        let drawn = pixels.withUnsafeMutableBytes { bytes -> Bool in
            guard let context = CGContext(data: bytes.baseAddress, width: width, height: height,
                bitsPerComponent: 8, bytesPerRow: width * 4, space: CGColorSpaceCreateDeviceRGB(),
                bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue | CGBitmapInfo.byteOrder32Big.rawValue) else { return false }
            context.draw(image, in: CGRect(x: 0, y: 0, width: width, height: height))
            return true
        }
        guard drawn else { return false }
        var land = 0, total = 0
        for y in (height / 5)..<(height * 4 / 5) {
            for x in (width / 10)..<(width * 9 / 10) {
                let i = (y * width + x) * 4
                let r = Int(pixels[i]), g = Int(pixels[i + 1]), b = Int(pixels[i + 2])
                // Warm land ink/wash is distinct from the cool water and the dark lettering.
                if r > 170 && g > 160 && r - b > 12 && g - b > 6 { land += 1 }
                total += 1
            }
        }
        return Double(land) / Double(max(1, total)) > 0.03
    }

    @MainActor private func exercise(largeText: Bool) {
        let app = XCUIApplication()
        app.launchArguments = ["-globe", "-globeStyle", "persian"]
        if largeText {
            app.launchArguments += ["-UIPreferredContentSizeCategoryName", "UICTContentSizeCategoryAccessibilityXXXL"]
        }
        app.launch()
        defer { app.terminate() }
        let door = app.buttons["An Aegean leaf"].firstMatch
        XCTAssertTrue(door.waitForExistence(timeout: 45), "The separate leaf must remain reachable beside the globe")
        XCTAssertTrue(door.isHittable)
        XCTAssertTrue(app.frame.contains(door.frame), "The combined atlas entries must fit the screen at this text size: entry \(door.frame), screen \(app.frame)")
        door.tap()
        let menu = app.buttons["Open the leaf menu"].firstMatch
        XCTAssertTrue(menu.waitForExistence(timeout: 45), "The bundled leaf menu must remain accessible")
        var painted = false
        for _ in 0..<20 {
            if geographyIsVisible() { painted = true; break }
            Thread.sleep(forTimeInterval: 1)
        }
        XCTAssertTrue(painted, "The actual screen must show the map, not merely its controls")
        capture(largeText ? "leaf-large-opening" : "leaf-opening")
        menu.tap()
        let place = app.buttons["Read note: Santorini"].firstMatch
        XCTAssertTrue(place.waitForExistence(timeout: 10))
        let sheet = app.otherElements["An Aegean leaf, web dialog"].firstMatch
        XCTAssertTrue(sheet.waitForExistence(timeout: 10))
        let menuClose = app.buttons["Close the leaf menu"].firstMatch
        func visiblyInsideSheet() -> Bool {
            // WebKit can report hittable for an element below a nested scroll clip.
            let visible = sheet.frame.insetBy(dx: 8, dy: 8)
            return place.isHittable && visible.contains(place.frame)
                && place.frame.minY > menuClose.frame.maxY + 8
        }
        for _ in 0..<12 where !visiblyInsideSheet() {
            let bounds = sheet.frame
            let origin = app.coordinate(withNormalizedOffset: .zero)
            let start = origin.withOffset(CGVector(dx: bounds.midX, dy: bounds.minY + bounds.height * 0.78))
            let end = origin.withOffset(CGVector(dx: bounds.midX, dy: bounds.minY + bounds.height * 0.38))
            start.press(forDuration: 0.05, thenDragTo: end)
        }
        XCTAssertTrue(visiblyInsideSheet(), "Optional notes remain visible inside their sheet at the reader's text size. \(app.debugDescription)")
        place.tap()
        let close = app.buttons["Close the note"].firstMatch
        XCTAssertTrue(close.waitForExistence(timeout: 10))
        capture(largeText ? "leaf-large-note" : "leaf-note")
        XCTAssertTrue(close.isHittable, "Leaving a note must not require reading to its end")
        close.tap()
        let back = app.buttons["Return to the atlas"].firstMatch
        XCTAssertTrue(back.waitForExistence(timeout: 10))
        back.tap()
        XCTAssertTrue(door.waitForExistence(timeout: 15), "The existing globe must still be present after leaving the leaf")
    }

    @MainActor func testBundledLeafAndReturn() { exercise(largeText: false) }
    @MainActor func testLeafAtLargestText() { exercise(largeText: true) }
}
