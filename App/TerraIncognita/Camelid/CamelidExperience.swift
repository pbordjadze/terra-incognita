import SwiftUI

/// A quiet authored lens on the atlas. All reading is optional and leaves the playback paused.
struct CamelidExperience: View {
    let style: GlobeStyle
    @Environment(\.dismiss) private var dismiss
    @Environment(\.scenePhase) private var scenePhase
    @Environment(\.accessibilityReduceMotion) private var systemReduceMotion
    @Environment(\.dynamicTypeSize) private var typeSize
    @State private var story = CamelidStory()
    private let ink = Color(red: 0.23, green: 0.17, blue: 0.09)
    private let paper = Color(red: 0.95, green: 0.89, blue: 0.78)
    private let blue = Color(red: 0.31, green: 0.30, blue: 0.47)
    private var reduceMotion: Bool {
        #if DEBUG
        systemReduceMotion || ProcessInfo.processInfo.arguments.contains("-camelid-reduce-motion")
        #else
        systemReduceMotion
        #endif
    }

    var body: some View {
        ZStack {
            paper.ignoresSafeArea()
            CamelidCanvasView(story: story, style: style,
                              active: scenePhase == .active && story.reading == nil,
                              reduceMotion: reduceMotion)
                .ignoresSafeArea()
            VStack(spacing: 0) {
            VStack(spacing: 6) {
                HStack {
                    Button { story.pause(); dismiss() } label: {
                        Image(systemName: "xmark").frame(width: 44, height: 44)
                    }
                    .accessibilityLabel("Return to atlas")
                    .accessibilityIdentifier("camelidClose")
                    Spacer(minLength: 4)
                    Text(story.date)
                        .font(.custom("LibertinusSerif-Regular", size: 30, relativeTo: .title))
                        .monospacedDigit()
                        .accessibilityIdentifier("camelidDate")
                    Spacer(minLength: 4)
                    Button { story.open(.evidence) } label: {
                        Image(systemName: "info.circle").frame(width: 44, height: 44)
                    }
                    .accessibilityLabel("The evidence")
                    .accessibilityIdentifier("camelidEvidence")
                }
                Text(story.place)
                    .font(.custom("LibertinusSerif-Regular", size: 16, relativeTo: .callout))
                Text(story.title.isEmpty ? " " : story.title)
                    .font(.custom("LibertinusSerif-Regular", size: 25, relativeTo: .title2))
                    .multilineTextAlignment(.center)
                    .accessibilityIdentifier("camelidScene")
                if story.historical {
                    Text("Modern geography · historical ranges illustrative")
                        .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
                        .multilineTextAlignment(.center)
                }
                if story.followed && !story.exploring && (story.scene == 3 || story.scene == 4) {
                    Text("Broad dispersal · many generations")
                        .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
                }
            }
            .padding(.horizontal, 22)
            .padding(.top, 6)
            .padding(.bottom, 10)
            .background(paper.opacity(0.97))
                Spacer(minLength: 0)
            }
            if !story.followed && !story.exploring {
                Button { story.canvas?.attend() } label: { Color.clear }
                    .frame(width: 220, height: 220)
                    .accessibilityLabel("Dromedary camel")
                    .accessibilityHint("Responds to your touch")
                    .accessibilityIdentifier("camelidInspectAnimal")
            }
        }
        .foregroundStyle(ink)
        .tint(blue)
        .buttonStyle(.plain)
        .safeAreaInset(edge: .bottom, spacing: 0) {
            transport.foregroundStyle(ink).tint(blue).buttonStyle(.plain)
        }
        .sheet(item: $story.reading) { choice in
            CamelidEvidence(reading: choice)
                .preferredColorScheme(.light)
        }
        .onChange(of: scenePhase) { _, phase in if phase != .active { story.pause() } }
        .onDisappear { story.pause(); story.canvas?.setActive(false) }
        .statusBarHidden()
    }

    private var transport: some View {
        VStack(spacing: 12) {
            if !story.followed {
                Button { story.follow() } label: {
                    HStack(spacing: 8) {
                        Image(systemName: "arrow.triangle.branch")
                        Text("Follow lineage")
                    }
                    .font(.custom("LibertinusSerif-Regular", size: 21, relativeTo: .title3))
                    .padding(.horizontal, 20)
                    .frame(minHeight: 48)
                }
                .accessibilityIdentifier("camelidFollow")
            } else {
                ViewThatFits(in: .horizontal) {
                    HStack(spacing: 20) { playbackButtons }
                    VStack(spacing: 4) { playbackButtons }
                }
                .font(.custom("LibertinusSerif-Regular", size: 17, relativeTo: .body))
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 1) {
                        ForEach(0..<6) { index in
                            Button { story.select(index) } label: {
                                VStack(spacing: 7) {
                                    Circle().fill(story.scene == index ? blue : blue.opacity(0.35)).frame(width: 5, height: 5)
                                    Text(["Today", "44 Ma", "17 Ma", "7 Ma", "3 Ma", "Today"][index])
                                }
                                .frame(minWidth: 46, minHeight: 44)
                                .padding(.horizontal, 3)
                            }
                            .accessibilityLabel(["Today, follow the ancestry", "44 million years ago, North American origin", "17 million years ago, the family branches", "7 million years ago, toward Eurasia", "3 million years ago, toward South America", "Today, the cousins"][index])
                            .accessibilityIdentifier("camelidMilestone\(index)")
                            .accessibilityAddTraits(story.scene == index ? .isSelected : [])
                        }
                    }
                    .font(.custom("LibertinusSerif-Regular", size: 13, relativeTo: .caption))
                    .padding(.horizontal, 10)
                }
                .fixedSize(horizontal: false, vertical: true)
                if story.scene == 1 && !story.exploring {
                    Text("Early camelid · illustrative reconstruction")
                        .font(.custom("LibertinusSerif-Regular", size: 12, relativeTo: .caption))
                }
            }
        }
        .padding(.horizontal, 18)
        .padding(.top, 8)
        .padding(.bottom, 12)
        .frame(maxWidth: .infinity)
        .background(paper.opacity(0.96))
    }

    @ViewBuilder private var playbackButtons: some View {
                    Button {
                        if story.playing { story.pause() } else { story.resume() }
                    } label: {
                        Label(story.playing ? "Pause" : "Resume", systemImage: story.playing ? "pause" : "play")
                            .frame(minHeight: 44)
                    }
                    .accessibilityIdentifier(story.playing ? "camelidPause" : "camelidResume")
                    .accessibilityValue("\(Int(story.elapsed)) seconds")
                    Menu {
                        Button("The visual story, in words") { story.open(.story) }
                            .accessibilityIdentifier("camelidStory")
                        Button("The evidence") { story.open(.evidence) }
                        Button("Return to the camel") { story.inspectCamel() }
                        Button("Return to atlas") { story.pause(); dismiss() }
                    } label: {
                        Image(systemName: "ellipsis").frame(width: 44, height: 44)
                    }
                    .accessibilityLabel("More ways to explore")
                    .accessibilityIdentifier("camelidMore")
                    if story.scene == 5 {
                        Button("Llama & alpaca") { story.open(.family) }
                            .frame(minHeight: 44)
                            .accessibilityIdentifier("camelidFamily")
                    }
    }
}
