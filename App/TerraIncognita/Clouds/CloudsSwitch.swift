import Foundation
import Observation
import SwiftUI

/// Whether the reader wants the clouds: on until they turn them off, and remembered. And whether
/// they give way to an experience on the globe (`veiled`): while one runs they thin to a veil, so
/// what it shows comes first.
@Observable
final class CloudsSwitch {
    static let shared = CloudsSwitch()
    nonisolated private static let key = "clouds.shown"

    private(set) var on = UserDefaults.standard.object(forKey: CloudsSwitch.key) as? Bool ?? true
    var veiled = false

    private init() {}

    func toggle() {
        on.toggle()
        UserDefaults.standard.set(on, forKey: Self.key)
    }
}

/// The clouds' quiet control, beside the workbench button: a cloud, filled while they show. Not on
/// the Persian page, which has none. It also tells the clouds when an experience is running.
struct CloudsButton: View {
    let style: GlobeStyle
    /// An experience is running on the globe: the clouds give way to it.
    var experience = false
    private var clouds: CloudsSwitch { .shared }

    var body: some View {
        Group { button }
            .onChange(of: experience, initial: true) { clouds.veiled = experience }
    }

    @ViewBuilder private var button: some View {
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
