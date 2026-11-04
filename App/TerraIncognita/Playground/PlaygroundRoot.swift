import Foundation
import SwiftUI
import UIKit

/// The app: the globe with the atlas's doorways on it, one small workbench button, and while a
/// prototype is active, its chip (what it is, Open, and ✕ back to the atlas) with whatever it has
/// added to the globe, its panel when it shows one, and a lens's chrome while one is open (the
/// experience on the globe itself, which then has the screen to itself).
struct PlaygroundRoot: View {
    @State private var playground = Playground()
    /// Where the controls are, so the globe keeps its names clear of them.
    @State private var controlsFrame = CGRect.zero
    /// The launch screen's slate: the globe fades up from it.
    @State private var veil = 1.0
    /// The info mark's demo, while it shows (`InfoDemo`).
    @State private var infoShown = false
    @Environment(\.scenePhase) private var scenePhase

    private var style: GlobeStyle { playground.style }
    private var covered: Bool { playground.screen != nil }
    private var lens: PlaygroundLens? { playground.stage?.lens }
    /// Inside the Persian page's frame, when that parked style is on.
    private var inset: CGFloat { style == .persian ? PersianCaption.pageRoom : 16 }

    var body: some View {
        // (paused under the workbench too: nothing turns behind a menu, and its list has the main thread)
        GlobeScreen(active: !covered && !playground.workbenchShown && scenePhase == .active, style: style,
                    keepClear: [controlsFrame], invitation: playground.stage?.invitation, foot: foot, quiet: lens != nil,
                    quietFrame: playground.quietFrame,
                    onInvitation: { playground.stage?.acceptInvitation() },
                    onDrawn: { playground.globeDrawn($0) })
            .accessibilityHidden(covered)
            // Top right, all of it: iPadOS keeps the window's top-left corner for its own controls.
            .overlay(alignment: .topTrailing) {
                VStack(alignment: .trailing, spacing: 8) {
                    HStack(alignment: .top, spacing: 10) {
                        if let stage = playground.stage, !covered { chip(stage) }
                        if playground.stage == nil { infoMark }
                        CloudsButton(style: style, experience: playground.veiled)
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
                // (under a lens's chrome they keep their place, so the globe's room stays as it was)
                .unseen(lens != nil)
            }
            .animation(.easeInOut(duration: 0.25), value: playground.stage?.prototype.id)
            // a lens: the experience on the globe itself, its chrome over the globe it drives
            .overlay {
                if let lens {
                    lens.chrome()
                        .id(lens.id)
                        .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.3), value: lens?.id)
            .overlay {
                if infoShown {
                    InfoDemo(globe: playground.globe) { infoShown = false }
                        .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.25), value: infoShown)
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

    /// The info mark: tiny, beside the controls; a tap shows what the app is (`InfoDemo`).
    private var infoMark: some View {
        Button { infoShown = true } label: {
            Image(systemName: "info.circle")
                .font(.system(size: 15, weight: .regular))
                .foregroundStyle(.secondary)
                .frame(width: 44, height: 44)
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("About this app")
        .accessibilityIdentifier("infoMark")
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

    /// The prototype in front: its name (and how many others run), Open if something opens, ✕ to end it
    /// (the next one is then in front; with none left, the plain globe).
    private func chip(_ stage: PrototypeStage) -> some View {
        let others = playground.stages.count - 1
        return HStack(spacing: 6) {
            if stage.loading { ProgressView().controlSize(.small).padding(.leading, 6) }
            Text(stage.prototype.title)
                .font(.subheadline.weight(.semibold))
                .lineLimit(2)
                .padding(.leading, stage.loading ? 0 : 12)
                .accessibilityIdentifier("activePrototype")
            if others > 0 {
                Text(verbatim: "+\(others)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                    .accessibilityIdentifier("otherPrototypes")
                    .accessibilityLabel(Text("and \(others) more running"))
            }
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
            Button { playground.endFront() } label: {
                Image(systemName: "xmark").font(.system(size: 14, weight: .semibold)).frame(width: 44, height: 44)
            }
            .accessibilityLabel(others > 0 ? Text("End \(stage.prototype.title)") : Text("Back to the plain globe"))
            .accessibilityIdentifier("endActivePrototype")
        }
        .foregroundStyle(.primary)
        .glassEffect(.regular, in: Capsule())
        .accessibilityElement(children: .contain)
        .accessibilityLabel(others > 0 ? Text("Active prototype: \(stage.prototype.title), and \(others) more running")
                                       : Text("Active prototype: \(stage.prototype.title)"))
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

/// What the info mark shows for a few seconds: a ghost finger dragging the live globe (the globe turns
/// under it, as under a finger) and one line; a tap anywhere closes it. Under Reduce Motion the finger
/// rests, still, and the globe stays where it is.
struct InfoDemo: View {
    weak var globe: GlobeView?
    let close: () -> Void
    @State private var start = Date()
    private static let line = "The Earth as satellites saw it this month. Drag to turn it; tap a country."
    private static let drag = (from: 0.9, seconds: 1.6)

    var body: some View {
        GeometryReader { geo in
            TimelineView(.animation) { context in
                let el = context.date.timeIntervalSince(start)
                let reduce = UIAccessibility.isReduceMotionEnabled
                let u = reduce ? 0 : min(max((el - Self.drag.from) / Self.drag.seconds, 0), 1)
                let e = u * u * (3 - 2 * u)
                let w = geo.size.width, h = geo.size.height
                let p = CGPoint(x: w * (0.32 + 0.36 * e), y: h * (0.5 - 0.04 * e))
                let shown = reduce ? 1 : min(max((el - 0.4) / 0.3, 0), 1) * (1 - min(max((el - Self.drag.from - Self.drag.seconds - 0.2) / 0.4, 0), 1))
                ZStack {
                    Circle()
                        .fill(Color.white.opacity(0.16))
                        .overlay(Circle().stroke(Color.white.opacity(0.6), lineWidth: 1.4))
                        .frame(width: 38, height: 38)
                        .position(p)
                        .opacity(shown)
                    Text(Self.line)
                        .font(.callout)
                        .foregroundStyle(.white)
                        .shadow(color: .black.opacity(0.9), radius: 6)
                        .lineLimit(1)
                        .minimumScaleFactor(0.7)
                        .frame(maxWidth: min(w - 48, 560))
                        .position(x: w / 2, y: h - 120)
                }
                .onChange(of: el > 5.5) { _, done in if done { close() } }
            }
        }
        .ignoresSafeArea()
        .contentShape(Rectangle())
        .onTapGesture { close() }
        .accessibilityElement()
        .accessibilityLabel(Self.line)
        .accessibilityAddTraits(.isButton)
        .accessibilityHint("Closes")
        .onAppear {
            start = Date()
            UIAccessibility.post(notification: .announcement, argument: Self.line)
            guard !UIAccessibility.isReduceMotionEnabled, let globe else { return }
            // the drag's turn: what's under the finger goes with it, west to east
            Task { @MainActor in
                try? await Task.sleep(for: .seconds(Self.drag.from))
                let c = globe.authoredCamera
                let r = max(globe.stageCamera.radius, 1)
                let degrees = Double(globe.bounds.width) * 0.36 / r * 180 / .pi
                globe.setAuthoredCamera(GlobeCamera(lon: c.lon - degrees, lat: c.lat, zoom: c.zoom), duration: Self.drag.seconds)
            }
        }
    }
}
