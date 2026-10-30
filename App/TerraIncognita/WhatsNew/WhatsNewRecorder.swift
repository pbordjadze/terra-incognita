#if UNRELEASED
import AVFoundation
import Foundation
import Observation
import UIKit

/// A voice note: tap to talk, tap to stop; AAC in an .m4a beside the answer. The microphone is asked
/// for the first time it is used, by the system's own sheet.
@Observable
final class WhatsNewRecorder: NSObject, AVAudioRecorderDelegate {
    private(set) var recording = false
    private(set) var seconds = 0.0
    /// Why it couldn't record, for the reader (the microphone refused, say).
    private(set) var problem: String?
    private var recorder: AVAudioRecorder?
    private var ticker: Task<Void, Never>?
    private var file: URL?

    /// Starts recording into `url`; false if it couldn't (and says why in `problem`).
    func start(to url: URL) async -> Bool {
        problem = nil
        let allowed = await AVAudioApplication.requestRecordPermission()
        guard allowed else {
            problem = "The microphone is off for this app (Settings › Privacy › Microphone)."
            return false
        }
        do {
            let session = AVAudioSession.sharedInstance()
            try session.setCategory(.record, mode: .spokenAudio, options: [])
            try session.setActive(true)
            let settings: [String: Any] = [AVFormatIDKey: Int(kAudioFormatMPEG4AAC), AVSampleRateKey: 22_050,
                                           AVNumberOfChannelsKey: 1, AVEncoderAudioQualityKey: AVAudioQuality.medium.rawValue]
            let recorder = try AVAudioRecorder(url: url, settings: settings)
            recorder.delegate = self
            guard recorder.record() else { throw NSError(domain: "WhatsNew", code: 1) }
            self.recorder = recorder
            file = url
            recording = true
            seconds = 0
            ticker?.cancel()
            ticker = Task { [weak self] in
                while !Task.isCancelled {
                    try? await Task.sleep(for: .milliseconds(250))
                    guard let self, let r = self.recorder, self.recording else { return }
                    self.seconds = r.currentTime
                    // (a note longer than three minutes is a conversation: stop it there)
                    if r.currentTime > 180 { _ = self.stop() }
                }
            }
            return true
        } catch {
            problem = "Could not record: \(error.localizedDescription)"
            return false
        }
    }

    /// Stops, and gives the file (nil if nothing was recorded, or it was too short to mean anything).
    @discardableResult
    func stop() -> URL? {
        ticker?.cancel()
        ticker = nil
        guard let recorder, recording else { return nil }
        let length = recorder.currentTime
        recorder.stop()
        self.recorder = nil
        recording = false
        try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
        let url = file
        file = nil
        if length < 0.4, let url { try? FileManager.default.removeItem(at: url); return nil }
        return url
    }

    /// Stops and throws the note away (the reader changed their mind).
    func discard() {
        if let url = stop() { try? FileManager.default.removeItem(at: url) }
    }

    nonisolated func audioRecorderEncodeErrorDidOccur(_ recorder: AVAudioRecorder, error: Error?) {
        Task { @MainActor in
            self.problem = "Recording failed\(error.map { ": \($0.localizedDescription)" } ?? "")."
            _ = self.stop()
        }
    }
}

/// A picture of the screen as the reader sees it, with the module's own chrome left out, written as
/// a JPEG beside the answer. The host gets a moment first to make what it draws readable (Metal
/// layers drawn straight to the screen aren't, unless asked).
enum WhatsNewSnapshot {
    /// Renders the window after the next screen update.
    static func capture(_ window: UIWindow) -> UIImage? {
        let format = UIGraphicsImageRendererFormat()
        format.scale = min(window.screen.scale, 2)
        format.opaque = true
        let image = UIGraphicsImageRenderer(bounds: window.bounds, format: format).image { _ in
            window.drawHierarchy(in: window.bounds, afterScreenUpdates: true)
        }
        return image
    }

    /// Writes it as a JPEG (about a quarter of a megabyte on an iPad).
    static func write(_ image: UIImage, to url: URL) -> Bool {
        guard let data = image.jpegData(compressionQuality: 0.72) else { return false }
        return (try? data.write(to: url, options: .atomic)) != nil
    }

    /// How much of the picture is near black (0 to 1), by a coarse sample: a capture that missed the
    /// globe's drawing is mostly black, and a check can tell.
    static func darkness(of image: UIImage) -> Double {
        let side = 24
        guard let cg = image.cgImage else { return 1 }
        var pixels = [UInt8](repeating: 0, count: side * side * 4)
        let space = CGColorSpaceCreateDeviceRGB()
        guard let ctx = CGContext(data: &pixels, width: side, height: side, bitsPerComponent: 8, bytesPerRow: side * 4, space: space,
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return 1 }
        ctx.interpolationQuality = .low
        ctx.draw(cg, in: CGRect(x: 0, y: 0, width: side, height: side))
        var dark = 0
        for i in stride(from: 0, to: pixels.count, by: 4) where Int(pixels[i]) + Int(pixels[i + 1]) + Int(pixels[i + 2]) < 60 { dark += 1 }
        return Double(dark) / Double(side * side)
    }
}
#endif
