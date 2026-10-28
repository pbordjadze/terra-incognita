import SwiftUI
import UIKit

/// A voluntary visit to one place's ancient marine setting, on the atlas the reader already knows: it
/// composes on the playground's one globe (`WhaleCanvas`), and this chrome lies over it.
struct WhaleExperience: View {
    @Bindable var story: WhaleStory
    let canvas: WhaleCanvas
    /// Back to the atlas: the stage closes the lens and gives the globe back as it was.
    let close: () -> Void
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @State private var headerFrame = CGRect.zero
    @State private var transportFrame = CGRect.zero
    private let ink = Color(red: 0.20, green: 0.18, blue: 0.15)
    private let paper = Color(red: 0.95, green: 0.90, blue: 0.80)
    private let sea = Color(red: 0.23, green: 0.37, blue: 0.43)

    private var reduceMotion: Bool {
        #if DEBUG
        systemReduceMotion || ProcessInfo.processInfo.arguments.contains("-whale-reduce-motion")
        #else
        systemReduceMotion
        #endif
    }

    /// The visit on the globe, made for it, and its chrome (`PrototypeStage.openLens`).
    static func lens(on globe: GlobeView, close: @escaping () -> Void) -> PlaygroundLens {
        let story = WhaleStory()
        let canvas = WhaleCanvas(story: story, globe: globe)
        return PlaygroundLens(id: "whale", canvas: canvas) {
            AnyView(WhaleExperience(story: story, canvas: canvas, close: close))
        }
    }

    var body: some View {
        // (the globe beneath is the playground's: the empty middle leaves it to the reader's hands)
        GeometryReader { viewport in
            let width = max(viewport.size.width, 1)
            ZStack {
                VStack(spacing: 0) {
                    header(width: width)
                        .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { headerFrame = $0 }
                    Spacer(minLength: 0)
                }
                .frame(width: width)
            }
            .frame(width: width)
            .safeAreaInset(edge: .bottom, spacing: 0) {
                transport(width: width)
                    .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { transportFrame = $0 }
                    .foregroundStyle(ink)
                    .tint(sea)
                    .buttonStyle(.plain)
            }
        }
        .foregroundStyle(ink)
        .tint(sea)
        .buttonStyle(.plain)
        .sheet(item: $story.reading) { choice in
            WhaleEvidence(reading: choice)
                .preferredColorScheme(.light)
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { story.pause() } }
        .onChange(of: scenePhase == .active && story.reading == nil, initial: true) { _, active in canvas.setActive(active) }
        .onChange(of: reduceMotion, initial: true) { _, reduced in canvas.reduceMotion = reduced }
        .onChange(of: [headerFrame, transportFrame], initial: true) { _, frames in canvas.setKeepClear(frames.filter { !$0.isEmpty }) }
        .onDisappear { story.pause(); canvas.setActive(false) }
        .statusBarHidden()
    }

    private func header(width: CGFloat) -> some View {
        let readingWidth = max(width - 40, 1)
        return VStack(spacing: 6) {
            HStack(spacing: 4) {
                Button { story.pause(); close() } label: {
                    Image(systemName: "xmark")
                        .frame(width: 44, height: 44)
                        .contentShape(Rectangle())
                }
                .frame(width: 44)
                .accessibilityLabel("Return to atlas")
                .accessibilityIdentifier("whaleClose")
                Text(story.date)
                    .font(.custom("LibertinusSerif-Regular", size: 28, relativeTo: .title))
                    .lineLimit(1)
                    .minimumScaleFactor(0.55)
                    .frame(maxWidth: .infinity)
                    .accessibilityIdentifier("whaleDate")
                Button { story.open(.evidence) } label: {
                    Image(systemName: "info.circle")
                        .frame(width: 44, height: 44)
                        .contentShape(Rectangle())
                }
                .frame(width: 44)
                .accessibilityLabel("The evidence")
                .accessibilityIdentifier("whaleEvidence")
            }
            .frame(width: readingWidth)
            // Keep these quiet rows in the composition even during the temporal cut and today.
            // Their measured height also reserves the full readable text at larger sizes.
            Text("Wadi Al-Hitan · Egypt")
                .font(.custom("LibertinusSerif-Regular", size: 16, relativeTo: .callout))
                .multilineTextAlignment(.center)
                .frame(width: readingWidth)
                .fixedSize(horizontal: false, vertical: true)
                .opacity(story.place.isEmpty ? 0 : 1)
                .accessibilityHidden(story.place.isEmpty)
            Text(story.title.isEmpty ? " " : story.title)
                .font(.custom("LibertinusSerif-Regular", size: 26, relativeTo: .title2))
                .multilineTextAlignment(.center)
                .frame(width: readingWidth)
                .fixedSize(horizontal: false, vertical: true)
                .accessibilityIdentifier("whaleScene")
            ZStack(alignment: .top) {
                // Reserve the longest status at its actual reading size, while exposing only the
                // label that qualifies the visible reconstruction to accessibility.
                Text("Modern geography · reconstructed sea and whale")
                    .hidden()
                    .accessibilityHidden(true)
                Text(story.historical ? "Modern geography · reconstructed sea and whale" : "Illustrative fossil reconstruction")
                    .accessibilityIdentifier("whaleReconstructionStatus")
            }
            .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
            .multilineTextAlignment(.center)
            .frame(width: readingWidth)
            .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.top, 5)
        .padding(.bottom, 10)
        .frame(width: width)
        .background(paper.opacity(0.97))
    }

    private func transport(width: CGFloat) -> some View {
        VStack(spacing: 8) {
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 20) { playbackButtons }
                VStack(spacing: 4) { playbackButtons }
            }
            .font(.custom("LibertinusSerif-Regular", size: 18, relativeTo: .body))
            if story.followed {
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 18) {
                        milestone(index: 0, label: "Today", name: "Today, the fossil in Egypt")
                        milestone(index: 1, label: "≈37 Ma", name: "About 37 million years ago, a shallow sea")
                        milestone(index: 5, label: "Today", name: "Today, whales swam here")
                    }
                    .padding(.horizontal, 20)
                    .frame(minWidth: width)
                }
                .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(.vertical, 10)
        .frame(width: width)
        .background(paper.opacity(0.97))
    }

    private func milestone(index: Int, label: String, name: String) -> some View {
        let selected = index == 1 ? (1...3).contains(story.scene) : story.scene == index
        return Button { story.select(index) } label: {
            VStack(spacing: 5) {
                Circle().fill(selected ? sea : sea.opacity(0.35)).frame(width: selected ? 6 : 4, height: 6)
                Text(label)
                    .font(.custom("LibertinusSerif-Regular", size: 14, relativeTo: .caption))
            }
            .frame(minWidth: 65, minHeight: 44)
            .contentShape(Rectangle())
        }
        .accessibilityLabel(name)
        .accessibilityIdentifier("whaleMilestone\(index)")
        .accessibilityAddTraits(selected ? .isSelected : [])
    }

    @ViewBuilder private var playbackButtons: some View {
        if story.followed {
            Button {
                if story.playing { story.pause() } else { story.resume() }
            } label: {
                Label(story.playing ? "Pause" : "Resume", systemImage: story.playing ? "pause" : "play")
                    .frame(minHeight: 44)
                    .contentShape(Rectangle())
            }
            .accessibilityIdentifier(story.playing ? "whalePause" : "whaleResume")
            .accessibilityValue("\(Int(story.elapsed)) seconds")
        } else {
            Button { story.follow() } label: {
                Label("Visit the past", systemImage: "arrow.uturn.backward")
                    .frame(minHeight: 44)
                    .contentShape(Rectangle())
            }
            .accessibilityIdentifier("whaleFollow")
        }
        Menu {
            Button("The story, in words") { story.open(.story) }
                .accessibilityIdentifier("whaleStory")
            Button("Tiny hind limbs") { story.open(.limbs) }
                .accessibilityIdentifier("whaleLimbsMenu")
            Button("The evidence") { story.open(.evidence) }
            Button("Return to the fossil") { story.inspectFossil() }
            Button("Return to atlas") { story.pause(); close() }
        } label: {
            Image(systemName: "ellipsis")
                .frame(width: 44, height: 44)
                .contentShape(Rectangle())
        }
        .accessibilityLabel("More ways to explore")
        .accessibilityIdentifier("whaleMore")
        if story.followed && (story.scene == 3 || story.scene == 5) {
            Button("Tiny hind limbs") { story.open(.limbs) }
                .frame(minHeight: 44)
                .contentShape(Rectangle())
                .accessibilityIdentifier("whaleLimbs")
        }
    }
}
