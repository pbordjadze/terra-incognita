import UIKit

/// A picture a prototype places at a point of the globe (`GlobeView.add`): it follows the geography,
/// can be tapped, keeps the names clear of it and is one of the map's accessibility elements. It
/// shows only while it faces the reader and the zoom is within `minZoom`...`maxZoom`; the prototype's
/// stage takes it away again when the prototype ends.
final class GlobeMarker {
    enum Look {
        /// The picture as drawn, with no ground (an animal, a fossil, a pack's image).
        case picture(UIImage)
        /// A small paper seal with a symbol, ruled in the margin's blue ink.
        case seal(symbol: String)
    }

    let lon: Double
    let lat: Double
    let label: String
    let minZoom: Double
    let maxZoom: Double
    /// A slow breath, for something alive (still under Reduce Motion).
    let breathes: Bool
    /// VoiceOver's name for opening it directly from the map (beside "Go to …").
    let actionName: String?
    let onOpen: (() -> Void)?
    let view = UIButton(type: .custom)
    private let live = LiveCount.Token(.markers)

    init(id: String, lon: Double, lat: Double, look: Look, size: CGSize, label: String, hint: String? = nil,
         minZoom: Double = 0, maxZoom: Double = .infinity, breathes: Bool = false, actionName: String? = nil,
         onOpen: (() -> Void)?) {
        self.lon = lon
        self.lat = lat
        self.label = label
        self.minZoom = minZoom
        self.maxZoom = maxZoom
        self.breathes = breathes
        self.actionName = actionName
        self.onOpen = onOpen
        switch look {
        case .picture(let image):
            view.setImage(image.withRenderingMode(.alwaysOriginal), for: .normal)
            view.imageView?.contentMode = .scaleAspectFit
        case .seal(let symbol):
            view.setImage(UIImage(systemName: symbol), for: .normal)
            view.tintColor = UIColor(hex: "#3B2C16")
            view.backgroundColor = UIColor(hex: "#F2E4C8")
            view.layer.cornerRadius = min(size.width, size.height) / 2
            view.layer.borderWidth = 1.5
            view.layer.borderColor = UIColor(hex: "#6D6793")?.cgColor
        }
        view.bounds = CGRect(origin: .zero, size: size)
        view.accessibilityIdentifier = id
        view.accessibilityLabel = label
        view.accessibilityHint = hint
        view.isUserInteractionEnabled = onOpen != nil
        if let onOpen { view.addAction(UIAction { _ in onOpen() }, for: .touchUpInside) }
    }
}
