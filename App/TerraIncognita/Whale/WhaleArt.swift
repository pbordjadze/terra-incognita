import UIKit

enum WhaleArtwork: String, CaseIterable {
    case living, fossil, limb, desert, sea

    var description: String {
        switch self {
        case .living: "Dorudon · illustrative reconstruction"
        case .fossil: "Whale fossil · illustrative skeleton"
        case .limb: "Tiny hind limb · enlarged illustration"
        case .desert: "Wadi Al-Hitan today · landscape illustration"
        case .sea: "Ancient shallow sea · landscape illustration"
        }
    }
}

/// Original illustrations are decoded/cropped once. Animated native layers share their CGImages.
enum WhaleArt {
    private static let images: [WhaleArtwork: UIImage] = {
        var result: [WhaleArtwork: UIImage] = [:]
        let sources: [(WhaleArtwork, String, CGRect?)] = [
            (.living, "WhaleLiving", CGRect(x: 12, y: 203, width: 1917, height: 439)),
            (.fossil, "WhaleFossil", CGRect(x: 19, y: 98, width: 2133, height: 524)),
            (.limb, "WhaleLimb", CGRect(x: 12, y: 91, width: 1912, height: 602)),
            (.sea, "WhaleHabitats", CGRect(x: 11, y: 118, width: 1001, height: 575)),
            (.desert, "WhaleHabitats", CGRect(x: 13, y: 936, width: 993, height: 458))
        ]
        var decoded: [String: CGImage] = [:]
        for (kind, name, rectangle) in sources {
            let source: CGImage?
            if let cached = decoded[name] { source = cached }
            else {
                source = UIImage(named: name)?.cgImage
                decoded[name] = source
            }
            guard let source else { continue }
            let cropped = rectangle.flatMap { source.cropping(to: $0) } ?? source
            result[kind] = UIImage(cgImage: cropped)
        }
        return result
    }()

    static func image(_ kind: WhaleArtwork) -> UIImage? { images[kind] }
}
