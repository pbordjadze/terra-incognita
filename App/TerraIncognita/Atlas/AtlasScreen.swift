import Foundation
import Observation
import SwiftUI
import UIKit

/// What the screen around the map shows: which look, the caption of the moment, and whether the film
/// is playing or the reader has the map in hand.
@Observable
final class AtlasModel {
    let looks = Atlas.looks()
    var lookIndex = 0 { didSet { loadStyle() } }
    var caption: Plate.Caption?
    var mode: AtlasCanvas.Mode = .film
    /// The look's captions: its fonts, sizes and colours.
    private(set) var style: CaptionStyle?
    @ObservationIgnored weak var canvas: AtlasCanvas?

    var look: Look? { looks.indices.contains(lookIndex) ? looks[lookIndex] : nil }

    init() {
        #if DEBUG
        // `-look sampler` (CI's screenshots): start on that look
        let args = ProcessInfo.processInfo.arguments
        if let i = args.firstIndex(of: "-look"), i + 1 < args.count, let k = looks.firstIndex(where: { $0.id == args[i + 1] }) {
            lookIndex = k
        }
        #endif
        loadStyle()
    }

    private func loadStyle() {
        guard let look, let set = Atlas.plates(of: look) else { style = nil; return }
        if let s = set.style { Atlas.registerFonts(of: look, s) }
        style = set.style
    }
}

/// A single presentation route keeps the atlas's demos from stacking full-screen hosts.
enum AtlasDemo: String, Identifiable {
    case camelid, whale, ocean, water, leaf
    var id: String { rawValue }
}

/// The atlas stays home; Demos is the one deliberate doorway to the existing studies.
struct AtlasScreen: View {
    @State private var model = AtlasModel()
    @State private var demo: AtlasDemo? = ProcessInfo.processInfo.arguments.contains("-leaf") ? .leaf : nil
    @State private var demoCoverActive = ProcessInfo.processInfo.arguments.contains("-leaf")
    @State private var veil = 1.0
    @State private var banner: String?
    @State private var globe = AtlasScreen.startsOnGlobe
    @State private var globeMade = AtlasScreen.startsOnGlobe
    @State private var switching = false
    @State private var navigationTask: Task<Void, Never>?
    @State private var globeDrawn = false
    @AppStorage("globeStyle") private var globeStyle = GlobeStyle.persian
    @State private var buttonsFrame = CGRect.zero
    @Environment(\.scenePhase) private var scenePhase

    private static var startsOnGlobe: Bool {
        #if DEBUG
        let args = ProcessInfo.processInfo.arguments
        return args.contains("-globe") || (!args.contains("-film") && !args.contains("-look"))
        #else
        return true
        #endif
    }

    var body: some View {
        ZStack(alignment: .bottom) {
            AtlasCanvasView(model: model, active: !globe && !demoCoverActive && scenePhase == .active)
                .ignoresSafeArea()
                .accessibilityHidden(globe || demoCoverActive)
            if let caption = model.caption {
                CaptionView(caption: caption, style: model.style)
                    .transition(.opacity.combined(with: .offset(y: 8)))
                    .id(caption.title)
                    .accessibilityHidden(globe || demoCoverActive)
            }
            if globeMade {
                GlobeScreen(active: globe && !demoCoverActive && scenePhase == .active,
                            style: globeStyle, keepClear: [buttonsFrame],
                            onDrawn: { globeDrawn = true }, onDemo: openDemo)
                    .opacity(globe ? 1 : 0)
                    .allowsHitTesting(globe && !demoCoverActive)
                    .accessibilityHidden(!globe || demoCoverActive)
            }
        }
        .animation(.easeInOut(duration: 0.6), value: model.caption)
        .overlay(alignment: .topTrailing) {
            atlasControls
                .allowsHitTesting(!demoCoverActive)
                .accessibilityHidden(demoCoverActive)
                .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { buttonsFrame = $0 }
                .padding(.trailing, globe && globeStyle == .persian ? PersianCaption.pageRoom : 16)
                .padding(.top, globe && globeStyle == .persian ? PersianCaption.pageRoom : 8)
        }
        .overlay(alignment: .bottom) {
            if !globe && !demoCoverActive && (model.mode == .free || model.mode == .ended) {
                Button(model.mode == .ended || model.canvas?.atEnd == true ? "Play again" : "Resume the tour") { model.canvas?.resume() }
                    .buttonStyle(.glassProminent)
                    .tint(Color(red: 0.62, green: 0.29, blue: 0.20))
                    .padding(.bottom, 28)
                    .transition(.opacity)
            }
        }
        .animation(.easeInOut(duration: 0.3), value: model.mode)
        .overlay {
            Color("LaunchBackground").opacity(veil).ignoresSafeArea().allowsHitTesting(false)
        }
        .overlay(alignment: .top) {
            if let banner {
                Text(verbatim: banner)
                    .font(.system(.headline, design: .serif))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 18)
                    .padding(.vertical, 10)
                    .glassEffect()
                    .padding(.top, 60)
                    .transition(.opacity)
                    .accessibilityAddTraits(.isHeader)
            }
        }
        .animation(.easeInOut(duration: 0.4), value: banner)
        .onAppear {
            updatePlayback()
            withAnimation(.easeOut(duration: 1.4).delay(0.2)) { veil = 0 }
        }
        .onChange(of: scenePhase) { _, _ in updatePlayback() }
        .fullScreenCover(item: $demo, onDismiss: {
            demoCoverActive = false
            updatePlayback()
        }) { destination in
            switch destination {
            case .camelid: CamelidExperience(style: globeStyle)
            case .whale: WhaleExperience(style: globeStyle)
            case .ocean: OceanExperience()
            case .leaf: LeafScreen()
            case .water:
                NavigationStack {
                    WaterScreen()
                        .navigationTitle("Terra Incognita")
                        .navigationBarTitleDisplayMode(.inline)
                        .toolbar {
                            ToolbarItem(placement: .topBarLeading) {
                                Button("Back to atlas") { demo = nil }
                                    .frame(minHeight: 44)
                                    .accessibilityIdentifier("closeWaterStudy")
                            }
                        }
                        .toolbarBackground(.visible, for: .navigationBar)
                }
                .preferredColorScheme(.light)
            }
        }
        .background(Color("LaunchBackground"))
    }

    private var atlasControls: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 10) { demosMenu; styleControl }
                .fixedSize(horizontal: true, vertical: false)
            VStack(alignment: .trailing, spacing: 10) { demosMenu; styleControl }
        }
    }

    private var demosMenu: some View {
        Menu {
            Section("Experiences") {
                Button("Camel and Llama") { openDemo(.camelid) }.accessibilityIdentifier("demoCamelid")
                Button("Whale and Desert") { openDemo(.whale) }.accessibilityIdentifier("demoWhale")
                Button("Ocean encounter") { openDemo(.ocean) }.accessibilityIdentifier("demoOcean")
                Button("A gathering of water") { openDemo(.water) }.accessibilityIdentifier("demoWater")
                Button("An Aegean leaf") { openDemo(.leaf) }.accessibilityIdentifier("demoLeaf")
            }
            Section("Atlas and film") {
                Button("The globe") { showAtlas(globe: true) }.accessibilityIdentifier("demoGlobe")
                Button("Play current film") { showAtlas(globe: false, filmIndex: model.lookIndex) }.accessibilityIdentifier("demoFilm")
                Menu {
                    ForEach(model.looks.indices, id: \.self) { index in
                        Button(model.looks[index].name) { showAtlas(globe: false, filmIndex: index) }
                            .accessibilityIdentifier("demoLook_" + model.looks[index].id)
                    }
                } label: { Text("Film and look studies") }
                .accessibilityIdentifier("demoFilmLooks")
            }
        } label: {
            Label("Demos", systemImage: "square.grid.2x2")
                .font(.body)
                .padding(.horizontal, 12)
                .frame(minHeight: 44)
                .contentShape(Rectangle())
        }
        .buttonStyle(.glass)
        .accessibilityLabel("Demos")
        .accessibilityValue(globe ? "Globe" : "Film · " + (model.look?.name ?? ""))
        .accessibilityIdentifier("demosMenu")
    }

    @ViewBuilder private var styleControl: some View {
        if globe {
            Menu {
                Picker(selection: $globeStyle) {
                    ForEach(GlobeStyle.allCases) { style in Text(style.name).tag(style) }
                } label: { Text("Style") }
            } label: {
                Image(systemName: "paintpalette").font(.system(size: 20)).frame(width: 44, height: 44)
            }
            .buttonStyle(.glass)
            .accessibilityLabel("Style")
            .accessibilityValue(Text(globeStyle.name))
        } else if model.looks.count > 1 {
            Menu {
                Picker(selection: Binding(get: { model.lookIndex }, set: { showAtlas(globe: false, filmIndex: $0) })) {
                    ForEach(model.looks.indices, id: \.self) { index in Text(model.looks[index].name).tag(index) }
                } label: { Text("Look") }
            } label: {
                Image(systemName: "paintpalette").font(.system(size: 20)).frame(width: 44, height: 44)
            }
            .buttonStyle(.glass)
            .accessibilityLabel("Look")
            .accessibilityValue(Text(model.look?.name ?? ""))
        }
    }

    private func openDemo(_ destination: AtlasDemo) {
        guard !demoCoverActive else { return }
        navigationTask?.cancel()
        switching = false
        veil = 0
        banner = nil
        demoCoverActive = true
        updatePlayback()
        demo = destination
    }

    private func updatePlayback() {
        let shouldSuspend = globe || demoCoverActive || scenePhase != .active
        if model.canvas?.suspended != shouldSuspend { model.canvas?.suspended = shouldSuspend }
    }

    /// One cancellable transition owns film/look changes and globe changes. Selecting an immersive
    /// demo cancels it before presenting its host, so no delayed callback restarts a hidden player.
    private func showAtlas(globe target: Bool, filmIndex: Int? = nil) {
        guard !demoCoverActive else { return }
        if target && globe && !switching { return }
        navigationTask?.cancel()
        switching = true
        navigationTask = Task { @MainActor in
            withAnimation(.easeIn(duration: 0.22)) { veil = 1 }
            do { try await Task.sleep(for: .milliseconds(240)) } catch { return }
            guard !Task.isCancelled else { return }
            if let index = filmIndex, model.looks.indices.contains(index) {
                model.lookIndex = index
                model.canvas?.show(model.looks[index])
                model.canvas?.resume()
                banner = model.looks[index].name
            }
            globe = target
            globeMade = globeMade || target
            updatePlayback()
            for _ in 0..<40 where target && !globeDrawn {
                do { try await Task.sleep(for: .milliseconds(25)) } catch { return }
            }
            guard !Task.isCancelled else { return }
            withAnimation(.easeOut(duration: 0.5)) { veil = 0 }
            switching = false
            do { try await Task.sleep(for: .seconds(1.8)) } catch { return }
            guard !Task.isCancelled else { return }
            banner = nil
        }
    }
}

/// A caption in a look's type: the film's, and the globe's.
struct CaptionView: View {
    let caption: Plate.Caption
    let style: CaptionStyle?

    var body: some View {
        CaptionLines(caption: caption, style: style)
            .frame(maxWidth: CaptionLines.widest, alignment: .leading)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, 24)
            .padding(.top, 70)
            .padding(.bottom, 36)
            // nearly full by the eyebrow's line (about a third of the way down), so the small type stays
            // clear over a busy map
            .background(LinearGradient(stops: [.init(color: .clear, location: 0), .init(color: scrim.opacity(0.85), location: 0.3),
                                               .init(color: scrim, location: 0.55)], startPoint: .top, endPoint: .bottom))
            .allowsHitTesting(false)
            .accessibilityElement(children: .combine)
    }

    private var scrim: Color { Color(uiColor: style?.scrim.flatMap { UIColor(hex: $0) } ?? UIColor(white: 0, alpha: 0.55)) }
}

/// A caption's three lines (eyebrow, title, text) in a look's type.
struct CaptionLines: View {
    let caption: Plate.Caption
    let style: CaptionStyle?
    /// The widest a caption runs, in points: on an iPad a line any longer is hard to read.
    static let widest: CGFloat = 560

    var body: some View {
        // (a line left empty takes no room)
        VStack(alignment: .leading, spacing: 6) {
            if !caption.eyebrow.isEmpty {
                line(caption.eyebrow, style?.eyebrow, fallback: Fallback(size: 12, color: "#dc7354", tracking: 2.4, uppercase: true), relativeTo: .caption)
            }
            if !caption.title.isEmpty {
                line(caption.title, style?.title, fallback: Fallback(size: 28, color: "#f7ebcc"), relativeTo: .title)
            }
            if !caption.text.isEmpty {
                line(caption.text, style?.text, fallback: Fallback(size: 17, color: "#ebdec2", italic: true), relativeTo: .body)
            }
        }
    }

    private struct Fallback {
        var size: Double, color: String, tracking = 0.0, uppercase = false, italic = false
    }

    /// One line of the caption in the look's type, or the app's serif when the look sets none. A look
    /// that styles the line means what it leaves out: no italic, no capitals, no tracking.
    private func line(_ string: String, _ t: CaptionStyle.Text?, fallback f: Fallback, relativeTo textStyle: Font.TextStyle) -> some View {
        let size = t?.size ?? f.size
        var font: Font = t.map { $0.font == "serif" ? .system(size: size, design: .serif) : .custom($0.font, size: size, relativeTo: textStyle) }
            ?? .system(size: size, design: .serif)
        if t.map({ $0.italic ?? false }) ?? f.italic { font = font.italic() }
        let upper = t.map { $0.uppercase ?? false } ?? f.uppercase
        return Text(verbatim: upper ? string.uppercased() : string)
            .font(font)
            .tracking(t.map { $0.tracking ?? 0 } ?? f.tracking)
            .foregroundStyle(Color(uiColor: UIColor(hex: t?.color ?? f.color) ?? .white))
            .fixedSize(horizontal: false, vertical: true)
    }
}

private struct AtlasCanvasView: UIViewRepresentable {
    let model: AtlasModel
    let active: Bool

    func makeUIView(context: Context) -> AtlasCanvas {
        let canvas = AtlasCanvas(frame: .zero)
        canvas.accessibilityIdentifier = "mainAtlasFilm"
        canvas.suspended = !active
        canvas.onCaption = { model.caption = $0 }
        canvas.onMode = { model.mode = $0 }
        model.canvas = canvas
        if let look = model.look { canvas.show(look) }
        return canvas
    }

    func updateUIView(_ canvas: AtlasCanvas, context: Context) {
        if canvas.suspended == active { canvas.suspended = !active }
    }
}
