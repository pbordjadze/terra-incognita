import SwiftUI

/// Terra Incognita: a playground for a wandering atlas. It opens on the plain Wikipedia-style globe;
/// the workbench starts one prototype at a time on it or beside it (Playground/, docs/playground.md).
@main
struct TerraIncognitaApp: App {
    var body: some Scene {
        WindowGroup {
            PlaygroundRoot()
                .statusBarHidden()
                .persistentSystemOverlays(.hidden)
                .preferredColorScheme(.dark)
        }
    }
}
