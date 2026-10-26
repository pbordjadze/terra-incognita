import SwiftUI
import UIKit

/// Terra Incognita: an atlas drawn in letters. The map and its film live in the bundled page
/// (`Web/`, see its `render.js`); this is the window that holds it, edge to edge.
@main
struct TerraIncognitaApp: App {
    @State private var status = PageStatus()

    var body: some Scene {
        WindowGroup {
            MapView(status: status)
                .ignoresSafeArea()
                .background(Color.black)
                .overlay(alignment: .bottom) { StartLine(status: status) }
                .statusBarHidden()
                .persistentSystemOverlays(.hidden)
                .preferredColorScheme(.dark)
                // The film plays by itself for six minutes: the screen stays awake while it is open.
                .onAppear { UIApplication.shared.isIdleTimerDisabled = true }
                .onDisappear { UIApplication.shared.isIdleTimerDisabled = false }
        }
    }
}
