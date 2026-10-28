import Foundation
import Observation
import SwiftUI

/// Whether the reader wants the clouds: on until they turn them off, and remembered.
@Observable
final class CloudsSwitch {
    static let shared = CloudsSwitch()
    nonisolated private static let key = "clouds.shown"

    private(set) var on = UserDefaults.standard.object(forKey: CloudsSwitch.key) as? Bool ?? true

    private init() {}

    func toggle() {
        on.toggle()
        UserDefaults.standard.set(on, forKey: Self.key)
    }
}

/// The clouds' quiet control, beside the workbench button: a cloud, filled while they show. Not on
/// the Persian page, which has none.
struct CloudsButton: View {
    let style: GlobeStyle
    private var clouds: CloudsSwitch { .shared }

    var body: some View {
        if CloudsLook.on(style) != nil {
            Button { clouds.toggle() } label: {
                Image(systemName: clouds.on ? "cloud.fill" : "cloud")
                    .font(.system(size: 18))
                    .frame(width: 44, height: 44)
                    .contentShape(Rectangle())
            }
            .buttonStyle(.glass)
            .accessibilityLabel("Clouds")
            .accessibilityValue(clouds.on ? "On" : "Off")
            .accessibilityHint("Shows or hides this month's clouds")
            .accessibilityAddTraits(.isToggle)
            .accessibilityIdentifier("cloudsButton")
        }
    }
}
