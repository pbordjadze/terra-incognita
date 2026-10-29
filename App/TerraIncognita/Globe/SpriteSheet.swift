import UIKit

// Sprite sheets (runtime 6): many frames in one picture, so a pack's frames don't each cost a file. A
// frame of a sheet is named by the sheet's path and its rectangle in the sheet's pixels,
// `art/early.png#0,0,375,334` (x, y, width, height); any other name is a picture of its own. The sheets
// are written by tools/packart/render.cjs (a picture's `sheet`), which gives each frame its name. Packs
// reach this through their sprites' and doorways' looks (PackScript, Playground); a native prototype
// reads a bundled sheet's frames the same way, with the bundle's resources as the root.

enum SpriteSheet {
    /// The sheet's path and the frame's rectangle, if the name is a frame of a sheet.
    static func split(_ name: String) -> (sheet: String, rect: CGRect)? {
        guard let hash = name.firstIndex(of: "#") else { return nil }
        let numbers = name[name.index(after: hash)...].split(separator: ",", omittingEmptySubsequences: false).compactMap { Int($0) }
        guard numbers.count == 4, numbers[0] >= 0, numbers[1] >= 0, numbers[2] > 0, numbers[3] > 0 else { return nil }
        return (String(name[..<hash]), CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3]))
    }

    /// A picture by name under `root`: a file, or a frame cut from a sheet. A sheet is decoded once and
    /// kept in `sheets` (its frames share its pixels, so cutting one copies nothing); `safe` says
    /// whether a path may be read (a pack's rule).
    static func picture(_ name: String, in root: URL, sheets: inout [String: UIImage],
                        safe: (String) -> Bool = { _ in true }) -> UIImage? {
        guard let frame = split(name) else {
            guard !name.contains("#"), safe(name) else { return nil }
            return UIImage(contentsOfFile: root.appending(path: name).path(percentEncoded: false))
        }
        let path = frame.sheet, rect = frame.rect
        guard safe(path) else { return nil }
        let sheet: UIImage
        if let known = sheets[path] {
            sheet = known
        } else {
            // (decoded now, once: a frame cut from an undecoded picture would decode the whole sheet each time it is drawn)
            guard let read = UIImage(contentsOfFile: root.appending(path: path).path(percentEncoded: false)),
                  let decoded = read.preparingForDisplay() else { return nil }
            sheets[path] = decoded
            sheet = decoded
        }
        guard let image = sheet.cgImage, CGRect(x: 0, y: 0, width: image.width, height: image.height).contains(rect),
              let cut = image.cropping(to: rect) else { return nil }
        return UIImage(cgImage: cut, scale: sheet.scale, orientation: .up)
    }
}
