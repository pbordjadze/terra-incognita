import SwiftUI
import UIKit

/// Terra Incognita: a medieval map that plays as a film, from the whole world down to a street. The
/// pictures are each look's plates (`Atlas/`, made by tools/plates); `AtlasScreen` plays them.
@main
struct TerraIncognitaApp: App {
    var body: some Scene {
        WindowGroup {
            AtlasScreen()
                .statusBarHidden()
                .persistentSystemOverlays(.hidden)
                .preferredColorScheme(.dark)
                // The film plays by itself: the screen stays awake while it is open.
                .onAppear { UIApplication.shared.isIdleTimerDisabled = true }
                .onDisappear { UIApplication.shared.isIdleTimerDisabled = false }
        }
    }
}
