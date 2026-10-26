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

/// The map edge to edge, the caption at its foot, the look menu, and Resume once the reader has
/// taken the map in hand.
struct AtlasScreen: View {
    @State private var model = AtlasModel()
    /// The launch screen's slate over the map: the map fades up from it at launch and dips through it
    /// between looks.
    @State private var veil = 1.0
    /// The look's name, shown for a moment after switching.
    @State private var banner: String?

    var body: some View {
        ZStack(alignment: .bottom) {
            AtlasCanvasView(model: model)
                .ignoresSafeArea()
            if let caption = model.caption {
                CaptionView(caption: caption, style: model.style)
                    .transition(.opacity.combined(with: .offset(y: 8)))
                    .id(caption.title)
            }
        }
        .animation(.easeInOut(duration: 0.6), value: model.caption)
        .overlay(alignment: .topTrailing) {
            if model.looks.count > 1 {
                Menu {
                    Picker(selection: $model.lookIndex) {
                        ForEach(model.looks.indices, id: \.self) { i in Text(model.looks[i].name).tag(i) }
                    } label: { Text("Look") }
                } label: {
                    Image(systemName: "paintpalette")
                        .font(.title3)
                        .padding(6)
                }
                .buttonStyle(.glass)
                .accessibilityLabel(Text("Look"))
                .padding(.trailing, 16)
                .padding(.top, 8)
            }
        }
        .overlay(alignment: .bottom) {
            if model.mode == .free || model.mode == .ended {
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
            withAnimation(.easeOut(duration: 1.4).delay(0.2)) { veil = 0 }
        }
        .onChange(of: model.lookIndex) { _, _ in
            guard let look = model.look else { return }
            Task {
                withAnimation(.easeIn(duration: 0.22)) { veil = 1 }
                try? await Task.sleep(for: .milliseconds(240))
                model.canvas?.show(look)
                banner = look.name
                withAnimation(.easeOut(duration: 0.5)) { veil = 0 }
                try? await Task.sleep(for: .seconds(1.8))
                if banner == look.name { banner = nil }
            }
        }
        .background(Color("LaunchBackground"))
    }
}

private struct CaptionView: View {
    let caption: Plate.Caption
    let style: CaptionStyle?

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            line(caption.eyebrow, style?.eyebrow, fallback: Fallback(size: 12, color: "#dc7354", tracking: 2.4, uppercase: true), relativeTo: .caption)
            line(caption.title, style?.title, fallback: Fallback(size: 28, color: "#f7ebcc"), relativeTo: .title)
            line(caption.text, style?.text, fallback: Fallback(size: 17, color: "#ebdec2", italic: true), relativeTo: .body)
        }
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

    func makeUIView(context: Context) -> AtlasCanvas {
        let canvas = AtlasCanvas(frame: .zero)
        canvas.onCaption = { model.caption = $0 }
        canvas.onMode = { model.mode = $0 }
        model.canvas = canvas
        if let look = model.look { canvas.show(look) }
        return canvas
    }

    func updateUIView(_ canvas: AtlasCanvas, context: Context) {}
}
