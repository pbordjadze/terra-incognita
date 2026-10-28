import Foundation
import SwiftUI
import UIKit

/// The app: the Wikipedia globe with the atlas's doorways on it, one small workbench button, and while
/// a prototype is active, its chip (what it is, Open, and ✕ back to the atlas) with whatever it has
/// added to the globe, and its panel when it shows one.
struct PlaygroundRoot: View {
    @State private var playground = Playground()
    /// Where the controls are, so the globe keeps its names clear of them.
    @State private var controlsFrame = CGRect.zero
    /// The launch screen's slate: the globe fades up from it.
    @State private var veil = 1.0
    @Environment(\.scenePhase) private var scenePhase

    private var style: GlobeStyle { playground.stage?.style ?? .base }
    private var covered: Bool { playground.screen != nil }
    /// Inside the Persian page's frame, when that parked style is on.
    private var inset: CGFloat { style == .persian ? PersianCaption.pageRoom : 16 }

    var body: some View {
        // (paused under the workbench too: nothing turns behind a menu, and its list has the main thread)
        GlobeScreen(active: !covered && !playground.workbenchShown && scenePhase == .active, style: style,
                    keepClear: [controlsFrame], invitation: playground.stage?.invitation, foot: foot,
                    onInvitation: { playground.stage?.acceptInvitation() },
                    onDrawn: { playground.globeDrawn($0) })
            .accessibilityHidden(covered)
            // Top right, all of it: iPadOS keeps the window's top-left corner for its own controls.
            .overlay(alignment: .topTrailing) {
                VStack(alignment: .trailing, spacing: 8) {
                    HStack(alignment: .top, spacing: 10) {
                        if let stage = playground.stage, !covered { chip(stage) }
                        CloudsButton(style: style)
                        // (a store build would leave WORKBENCH out of the build settings, and the workbench with it)
                        #if WORKBENCH
                        workbenchButton
                        #endif
                    }
                    if let problem = playground.problem { problemNote(problem) }
                }
                .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { controlsFrame = $0 }
                .padding(.leading, 24)
                .padding(.trailing, inset)
                .padding(.top, style == .persian ? PersianCaption.pageRoom : 8)
            }
            .animation(.easeInOut(duration: 0.25), value: playground.stage?.prototype.id)
            .overlay {
                Color("LaunchBackground").opacity(veil).ignoresSafeArea().allowsHitTesting(false)
            }
            .onAppear { withAnimation(.easeOut(duration: 1.4).delay(0.2)) { veil = 0 } }
            .sheet(isPresented: $playground.workbenchShown, onDismiss: playground.workbenchDismissed) {
                WorkbenchView(playground: playground)
                    .presentationDetents([.large])
                    .presentationDragIndicator(.visible)
                    .preferredColorScheme(.light)
            }
            .fullScreenCover(item: $playground.screen, onDismiss: playground.screenClosed) { screen in
                screen.content()
            }
            // a pack's panel: half height, the globe live above it (on its own view, beside the workbench's sheet)
            .background {
                Color.clear.sheet(item: $playground.panel, onDismiss: playground.panelDismissed) { panel in
                    PackPanelView(panel: panel)
                        .presentationDetents([.medium, .large])
                        .presentationDragIndicator(.visible)
                        .presentationBackgroundInteraction(.enabled(upThrough: .medium))
                        .preferredColorScheme(.light)
                }
            }
            .alert("Add this source?", isPresented: Binding(get: { playground.proposedSource != nil },
                                                             set: { if !$0 { playground.proposedSource = nil } }),
                   presenting: playground.proposedSource) { url in
                Button("Add") {
                    do {
                        try playground.packs.addSource(url)
                        playground.workbenchShown = true
                    } catch {
                        playground.problem = error.localizedDescription
                    }
                    playground.proposedSource = nil
                }
                Button("Cancel", role: .cancel) { playground.proposedSource = nil }
            } message: { url in
                Text("The workbench will look for prototypes at \(url.absoluteString). Add only sources you trust.")
            }
            .onOpenURL { playground.handle($0) }
            .background(Color("LaunchBackground"))
    }

    /// A prototype's words and date at the foot of the screen (runtime 2), in the caption's place.
    private var foot: AnyView? {
        guard let stage = playground.stage, stage.time != nil || stage.caption != nil else { return nil }
        return AnyView(GlobeFoot(caption: stage.caption, time: stage.time,
                                 onTime: { [weak stage] in stage?.setTime($0, by: .reader) },
                                 onPlay: { [weak stage] in stage?.play($0) }))
    }

    private var workbenchButton: some View {
        Button { playground.workbenchShown = true } label: {
            Image(systemName: "square.grid.2x2")
                .font(.system(size: 18))
                .frame(width: 44, height: 44)
                .contentShape(Rectangle())
        }
        .buttonStyle(.glass)
        .accessibilityLabel("Workbench")
        .accessibilityHint("Prototypes to try on the globe")
        .accessibilityIdentifier("workbenchButton")
    }

    /// The active prototype: its name, Open if something opens, ✕ back to the plain globe.
    private func chip(_ stage: PrototypeStage) -> some View {
        HStack(spacing: 6) {
            if stage.loading { ProgressView().controlSize(.small).padding(.leading, 6) }
            Text(stage.prototype.title)
                .font(.subheadline.weight(.semibold))
                .lineLimit(2)
                .padding(.leading, stage.loading ? 0 : 12)
                .accessibilityIdentifier("activePrototype")
            if stage.canOpen {
                Button { stage.open() } label: {
                    Text("Open")
                        .font(.subheadline)
                        .padding(.horizontal, 6)
                        .frame(minHeight: 44)
                        .contentShape(Rectangle())
                }
                .accessibilityIdentifier("openActivePrototype")
            }
            Button { playground.choosePlainGlobe() } label: {
                Image(systemName: "xmark").font(.system(size: 14, weight: .semibold)).frame(width: 44, height: 44)
            }
            .accessibilityLabel("Back to the plain globe")
            .accessibilityIdentifier("endActivePrototype")
        }
        .foregroundStyle(.primary)
        .glassEffect(.regular, in: Capsule())
        .accessibilityElement(children: .contain)
        .accessibilityLabel(Text("Active prototype: \(stage.prototype.title)"))
    }

    private func problemNote(_ problem: String) -> some View {
        HStack(alignment: .top, spacing: 6) {
            Text(problem)
                .font(.footnote)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.leading, 12)
                .padding(.vertical, 12)
            Button { playground.problem = nil } label: {
                Image(systemName: "xmark").font(.system(size: 12, weight: .semibold)).frame(width: 44, height: 44)
            }
            .accessibilityLabel("Dismiss")
        }
        .frame(maxWidth: 420, alignment: .leading)
        .glassEffect(.regular, in: RoundedRectangle(cornerRadius: 18))
        .accessibilityIdentifier("playgroundProblem")
    }
}
