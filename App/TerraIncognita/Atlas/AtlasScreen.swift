import Observation
import SwiftUI
import UIKit

/// What the screen around the map shows: which look, the caption of the moment, and whether the film
/// is playing or the reader has the map in hand.
@Observable
final class AtlasModel {
    let looks = Atlas.looks()
    var lookIndex = 0
    var caption: Plate.Caption?
    var mode: AtlasCanvas.Mode = .film
    @ObservationIgnored weak var canvas: AtlasCanvas?

    var look: Look? { looks.indices.contains(lookIndex) ? looks[lookIndex] : nil }
}

/// The map edge to edge, the caption at its foot, the look menu, and Resume once the reader has
/// taken the map in hand.
struct AtlasScreen: View {
    @State private var model = AtlasModel()

    var body: some View {
        ZStack(alignment: .bottom) {
            AtlasCanvasView(model: model)
                .ignoresSafeArea()
            if let caption = model.caption {
                CaptionView(caption: caption)
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
                Button(model.mode == .ended ? "Play again" : "Resume the tour") { model.canvas?.resume() }
                    .buttonStyle(.glassProminent)
                    .tint(Color(red: 0.62, green: 0.29, blue: 0.20))
                    .padding(.bottom, 28)
                    .transition(.opacity)
            }
        }
        .animation(.easeInOut(duration: 0.3), value: model.mode)
        .onChange(of: model.lookIndex) { _, _ in
            if let look = model.look { model.canvas?.show(look) }
        }
        .background(Color.black)
    }
}

private struct CaptionView: View {
    let caption: Plate.Caption

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(verbatim: caption.eyebrow.uppercased())
                .font(.system(.caption, design: .serif).weight(.semibold))
                .tracking(2.4)
                .foregroundStyle(Color(red: 0.86, green: 0.45, blue: 0.33))
            Text(verbatim: caption.title)
                .font(.system(.title, design: .serif))
                .foregroundStyle(Color(red: 0.97, green: 0.92, blue: 0.80))
            Text(verbatim: caption.text)
                .font(.system(.body, design: .serif).italic())
                .foregroundStyle(Color(red: 0.92, green: 0.87, blue: 0.76))
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 24)
        .padding(.top, 60)
        .padding(.bottom, 36)
        .background(LinearGradient(colors: [.clear, .black.opacity(0.55)], startPoint: .top, endPoint: .bottom))
        .allowsHitTesting(false)
        .accessibilityElement(children: .combine)
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
