import SwiftUI
import UIKit

enum CamelidAnimal: String, CaseIterable {
    case dromedary, early, camelini, lamini, llama, alpaca
    var name: String {
        switch self {
        case .dromedary: "Dromedary camel"
        case .early: "Early camelid · illustrative reconstruction"
        case .camelini: "Paracamelus-like camelid · illustrative reconstruction"
        case .lamini: "Hemiauchenia-like camelid · illustrative reconstruction"
        case .llama: "Llama"
        case .alpaca: "Alpaca"
        }
    }
}

/// Original art is decoded once. Sprite transforms never allocate textures or redraw the map data.
enum CamelidArt {
    static let images: [CamelidAnimal: UIImage] = {
        var result: [CamelidAnimal: UIImage] = [:]
        let crops: [CamelidAnimal: CGRect] = [
            .dromedary: CGRect(x: 35, y: 20, width: 570, height: 475),
            .early: CGRect(x: 605, y: 185, width: 355, height: 310),
            .camelini: CGRect(x: 990, y: 15, width: 530, height: 480),
            .lamini: CGRect(x: 35, y: 492, width: 485, height: 493),
            .llama: CGRect(x: 560, y: 493, width: 455, height: 500),
            .alpaca: CGRect(x: 1050, y: 550, width: 465, height: 440)
        ]
        if let source = UIImage(named: "CamelidAnimals")?.cgImage {
            for (animal, rect) in crops {
                if let crop = source.cropping(to: rect) { result[animal] = UIImage(cgImage: crop) }
            }
        }
        return result
    }()
    static func image(_ animal: CamelidAnimal) -> UIImage? { images[animal] }
}

struct CamelidAnimalView: View {
    let kind: CamelidAnimal
    var body: some View {
        Group {
            if let image = CamelidArt.image(kind) {
                Image(uiImage: image).resizable().aspectRatio(contentMode: .fit)
            } else {
                Text(kind.name).font(.body)
            }
        }
        .accessibilityLabel(Text(kind.name))
    }
}
