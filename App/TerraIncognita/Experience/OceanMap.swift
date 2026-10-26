import SwiftUI

/// A movable manuscript glass makes one patch of geography readable in a different register.
/// Its evidence illustrations are not artifact photographs, a distribution census or a voyage map.
struct OceanMap: View {
    let lens: String
    let time: Double
    var onPlace: (String) -> Void = { _ in }
    @State private var focus = CGPoint(x: 0.24, y: 0.65)
    @State private var handleOrigin: CGPoint?

    private let sea = Color(red: 0.875, green: 0.918, blue: 0.949)
    private let land = Color(red: 0.867, green: 0.906, blue: 0.851)
    private let coast = Color(red: 0.427, green: 0.404, blue: 0.576)
    private let blueInk = Color(red: 0.30, green: 0.29, blue: 0.45)
    private let ink = Color(red: 0.231, green: 0.173, blue: 0.086)
    private let paper = Color(red: 0.949, green: 0.894, blue: 0.784)
    private let gold = Color(red: 0.773, green: 0.667, blue: 0.431)

    private struct Place {
        let id: String
        let name: String
        let coordinate: CGPoint
        let offset: CGSize
    }
    // Labels are separate 44-point controls, deliberately displaced from nearby island anchors.
    // The Madagascar and Comoros controls never share the small gap between the islands.
    private let places = [
        Place(id: "madagascar", name: "Madagascar", coordinate: CGPoint(x: 47, y: -19), offset: CGSize(width: 60, height: 66)),
        Place(id: "borneo", name: "Borneo", coordinate: CGPoint(x: 114, y: -2), offset: CGSize(width: -22, height: -33)),
        Place(id: "comoros", name: "Comoros", coordinate: CGPoint(x: 43.5, y: -11.7), offset: CGSize(width: 78, height: -2)),
        Place(id: "africa", name: "East Africa", coordinate: CGPoint(x: 37, y: -5), offset: CGSize(width: 0, height: -32))
    ]

    var body: some View {
        GeometryReader { g in
            let chart = Chart(size: g.size)
            let centre = constrained(focus, in: g.size)
            let radius = min(64.0, g.size.width * 0.19)
            let nearby = nearest(to: centre, chart: chart, reach: radius * 1.35)
            ZStack {
                Canvas { context, size in
                    let rect = CGRect(origin: .zero, size: size)
                    context.fill(Path(rect), with: .color(sea))
                    terrain(context, chart: chart)
                    label("INDIAN OCEAN", at: chart.point(CGPoint(x: 79, y: -17)), context: context,
                          size: min(max(size.width / 28, 12), 18), tracking: 1.8, color: blueInk)
                    label("India", at: chart.point(CGPoint(x: 78, y: 18)), context: context, size: 12, color: ink)
                    label("Arabia", at: chart.point(CGPoint(x: 48, y: 24)), context: context, size: 12, color: ink)
                    label("Sumatra", at: chart.point(CGPoint(x: 100, y: -6)), context: context, size: 11, color: ink)
                    for place in places {
                        let p = chart.point(place.coordinate)
                        let destination = labelPoint(place, chart: chart)
                        var leader = Path()
                        leader.move(to: p)
                        leader.addLine(to: destination)
                        context.stroke(leader, with: .color(coast.opacity(0.55)), lineWidth: 0.65)
                        context.fill(Path(ellipseIn: CGRect(x: p.x - 2, y: p.y - 2, width: 4, height: 4)), with: .color(ink))
                    }
                    // The glass magnifies the same coast geometry, with a genuine affine transform.
                    let aperture = CGRect(x: centre.x - radius, y: centre.y - radius, width: radius * 2, height: radius * 2)
                    var glass = context
                    glass.clip(to: Path(ellipseIn: aperture))
                    glass.fill(Path(aperture), with: .color(sea))
                    var magnified = glass
                    magnified.translateBy(x: centre.x, y: centre.y)
                    magnified.scaleBy(x: 2.1, y: 2.1)
                    magnified.translateBy(x: -centre.x, y: -centre.y)
                    terrain(magnified, chart: chart)
                    if let nearby {
                        evidence(nearby.id, context: glass, centre: centre, radius: radius)
                    } else {
                        label("the open sea", at: centre, context: glass, size: 16, color: blueInk)
                    }
                    for inset in [0.0, 3.0, 5.0] {
                        context.stroke(Path(ellipseIn: aperture.insetBy(dx: inset, dy: inset)),
                                       with: .color(inset == 3 ? paper : coast), lineWidth: inset == 3 ? 3.5 : 0.9)
                    }
                    context.stroke(Path(rect.insetBy(dx: 0.8, dy: 0.8)), with: .color(gold), lineWidth: 1.5)
                    context.stroke(Path(rect.insetBy(dx: 4, dy: 4)), with: .color(coast.opacity(0.55)), lineWidth: 0.5)
                }
                .contentShape(Rectangle())
                .gesture(DragGesture(minimumDistance: 0).onChanged { value in
                    focus = normalized(value.location, size: g.size)
                })
                .accessibilityHidden(true)

                // The handle is a real adjustable control and an alternative to dragging the field.
                Button {
                    advance(from: nearby?.id, direction: 1, chart: chart)
                } label: {
                    Image(systemName: "hand.draw")
                        .font(.system(size: 13))
                        .foregroundStyle(ink)
                        .padding(5).background(paper, in: Circle())
                        .frame(width: 44, height: 44).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .highPriorityGesture(DragGesture(minimumDistance: 5, coordinateSpace: .named("oceanField")).onChanged { value in
                    if handleOrigin == nil { handleOrigin = constrained(focus, in: g.size) }
                    if let origin = handleOrigin {
                        focus = normalized(CGPoint(x: origin.x + value.translation.width, y: origin.y + value.translation.height), size: g.size)
                    }
                }.onEnded { _ in handleOrigin = nil })
                .accessibilityLabel("Inspection glass")
                .accessibilityValue(nearby?.name ?? "Open sea")
                .accessibilityHint("Adjust to move between shores, or drag across the map")
                .accessibilityIdentifier("oceanLoupe")
                .accessibilityAdjustableAction { direction in
                    advance(from: nearby?.id, direction: direction == .decrement ? -1 : 1, chart: chart)
                }
                .position(x: centre.x - radius * 0.70, y: centre.y + radius * 0.70)
                ForEach(places, id: \.id) { place in
                    Button {
                        focus = normalized(chart.point(place.coordinate), size: g.size)
                        onPlace(place.id)
                    } label: {
                        Text(verbatim: place.name)
                            .font(.custom("LibertinusSerif-Semibold", size: 12))
                            .foregroundStyle(ink)
                            .padding(.horizontal, 5).padding(.vertical, 2)
                            .background(paper.opacity(0.95))
                            .frame(minWidth: 60, minHeight: 44)
                            .contentShape(Rectangle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Explore \(place.name)")
                    .accessibilityIdentifier("place_\(place.id)")
                    .position(labelPoint(place, chart: chart))
                }
                VStack {
                    Spacer()
                    Text("Illustrative traces · modern coasts · move the glass")
                        .font(.system(size: 10, design: .serif))
                        .foregroundStyle(ink)
                        .padding(.horizontal, 8).padding(.vertical, 3)
                        .background(sea.opacity(0.94)).padding(.bottom, 7)
                }
                .allowsHitTesting(false)
            }
            .coordinateSpace(name: "oceanField")
            .clipped()
            .accessibilityIdentifier("oceanMapSurface")
            .accessibilityElement(children: .contain)
            .accessibilityLabel("Indian Ocean viewing table")

        }
    }

    private func terrain(_ context: GraphicsContext, chart: Chart) {
        var map = context
        map.clip(to: Path(chart.room))
        for longitude in stride(from: 30.0, through: 120, by: 15) {
            var p = Path()
            p.move(to: chart.point(CGPoint(x: longitude, y: -31)))
            p.addLine(to: chart.point(CGPoint(x: longitude, y: 27)))
            map.stroke(p, with: .color(coast.opacity(0.12)), lineWidth: 0.5)
        }
        for latitude in stride(from: -30.0, through: 15, by: 15) {
            var p = Path()
            p.move(to: chart.point(CGPoint(x: 27, y: latitude)))
            p.addLine(to: chart.point(CGPoint(x: 123, y: latitude)))
            map.stroke(p, with: .color(coast.opacity(latitude == 0 ? 0.23 : 0.12)), lineWidth: 0.5)
        }
        for ring in OceanCoasts.land { map.fill(chart.path(ring, closed: true), with: .color(land)) }
        for shore in OceanCoasts.shores {
            map.stroke(chart.path(shore, closed: false), with: .color(coast.opacity(0.8)), lineWidth: 0.7)
        }
    }

    private func evidence(_ place: String, context: GraphicsContext, centre: CGPoint, radius: CGFloat) {
        let rice = lens == "seeds" && (place == "madagascar" || place == "comoros")
        // A quiet sheet under the evidence makes clear it is an interpretation above the map.
        let panel = CGRect(x: centre.x - radius + 8, y: centre.y - radius + 10, width: radius * 2 - 16, height: radius * 2 - 20)
        context.fill(Path(ellipseIn: panel), with: .color(paper.opacity(0.91)))
        if rice {
            let reveal = min(max(time, 0), 1)
            let grain = CGRect(x: centre.x - 8, y: centre.y - 31, width: 16, height: 45)
            let shape = SeedStudy.grain(in: grain)
            // Surface: a pale botanical outline. Beneath: a blackened, preserved grain.
            context.fill(shape, with: .color(reveal < 0.5 ? gold.opacity(0.2) : ink.opacity(0.95)))
            context.stroke(shape, with: .color(ink.opacity(0.8)), lineWidth: 0.9)
            var furrow = Path()
            furrow.move(to: CGPoint(x: centre.x, y: grain.minY + 4))
            furrow.addQuadCurve(to: CGPoint(x: centre.x, y: grain.maxY - 4), control: CGPoint(x: centre.x + 3, y: centre.y))
            context.stroke(furrow, with: .color(reveal < 0.5 ? ink.opacity(0.3) : gold.opacity(0.8)), lineWidth: 0.7)
            label(reveal < 0.5 ? "a trace below" : "charred rice", at: CGPoint(x: centre.x, y: centre.y + 29), context: context, size: 14, color: ink)
            label(reveal < 0.5 ? "draw back the layer" : "c. 700–1000 CE", at: CGPoint(x: centre.x, y: centre.y + 44), context: context, size: 10, color: blueInk)
        } else if lens == "words" && (place == "borneo" || place == "madagascar") {
            label(place == "borneo" ? "jatuh" : "zato", at: CGPoint(x: centre.x, y: centre.y - 11), context: context, size: 29, color: ink)
            label(place == "borneo" ? "Ma’anyan" : "Malagasy", at: CGPoint(x: centre.x, y: centre.y + 16), context: context, size: 12, color: blueInk)
            label("hundred", at: CGPoint(x: centre.x, y: centre.y + 35), context: context, size: 14, color: ink)
        } else {
            let name = place == "borneo" ? "Borneo" : place == "comoros" ? "Comoros" : place == "africa" ? "East Africa" : "Madagascar"
            label(name, at: CGPoint(x: centre.x, y: centre.y - 9), context: context, size: 18, color: ink)
            label(lens == "people" ? "a place, not a route" : "another shore", at: CGPoint(x: centre.x, y: centre.y + 16), context: context, size: 11, color: blueInk)
        }
    }

    private func advance(from id: String?, direction: Int, chart: Chart) {
        let current = places.firstIndex { $0.id == id } ?? 0
        let next = places[(current + direction + places.count) % places.count]
        focus = normalized(chart.point(next.coordinate), size: chart.size)
    }
    private func labelPoint(_ place: Place, chart: Chart) -> CGPoint {
        let p = chart.point(place.coordinate)
        return CGPoint(x: min(max(p.x + place.offset.width, 44), chart.size.width - 44),
                       y: min(max(p.y + place.offset.height, 30), chart.size.height - 41))
    }
    private func label(_ value: String, at point: CGPoint, context: GraphicsContext, size: CGFloat, tracking: CGFloat = 0, color: Color) {
        let text = Text(verbatim: value).font(.custom("LibertinusSerif-Regular", size: size)).tracking(tracking).foregroundColor(color)
        context.draw(text, at: point)
    }
    private func normalized(_ point: CGPoint, size: CGSize) -> CGPoint {
        CGPoint(x: point.x / max(size.width, 1), y: point.y / max(size.height, 1))
    }
    private func constrained(_ point: CGPoint, in size: CGSize) -> CGPoint {
        let inset = min(65.0, size.width * 0.20)
        return CGPoint(x: min(max(point.x * size.width, inset), size.width - inset),
                       y: min(max(point.y * size.height, inset), size.height - inset - 14))
    }
    private func nearest(to point: CGPoint, chart: Chart, reach: CGFloat) -> Place? {
        let sorted = places.sorted { a, b in
            let pa = chart.point(a.coordinate), pb = chart.point(b.coordinate)
            return hypot(pa.x - point.x, pa.y - point.y) < hypot(pb.x - point.x, pb.y - point.y)
        }
        guard let first = sorted.first else { return nil }
        let p = chart.point(first.coordinate)
        return hypot(p.x - point.x, p.y - point.y) < reach ? first : nil
    }
    private struct Chart {
        let size: CGSize
        var scale: CGFloat { min((size.width - 28) / 96, (size.height - 45) / 58) }
        var room: CGRect { CGRect(x: (size.width - 96 * scale) / 2, y: (size.height - 58 * scale) / 2 - 8, width: 96 * scale, height: 58 * scale) }
        func point(_ coordinate: CGPoint) -> CGPoint {
            CGPoint(x: room.minX + (coordinate.x - 27) * scale, y: room.minY + (27 - coordinate.y) * scale)
        }
        func path(_ coordinates: [CGPoint], closed: Bool) -> Path {
            var p = Path()
            guard let first = coordinates.first else { return p }
            p.move(to: point(first))
            for coordinate in coordinates.dropFirst() { p.addLine(to: point(coordinate)) }
            if closed { p.closeSubpath() }
            return p
        }
    }
}

/// An original botanical line study. It is intentionally not a likeness of a particular find.
/// Its central furrow and paired veins follow the length of an unhusked grain of rice.
struct SeedStudy: View {
    var progress: Double = 1

    var body: some View {
        Canvas { context, size in
            let w = min(size.width * 0.30, size.height * 0.40)
            let h = min(size.height * 0.80, w * 2.75)
            let r = CGRect(x: (size.width - w) / 2, y: (size.height - h) / 2, width: w, height: h)
            let ink = Color(red: 0.231, green: 0.173, blue: 0.086)
            let gold = Color(red: 0.773, green: 0.667, blue: 0.431)
            let t = min(max(progress, 0), 1)
            context.fill(Self.grain(in: r), with: .color(gold.opacity(0.20 + t * 0.35)))
            context.stroke(Self.grain(in: r), with: .color(ink), lineWidth: 1.2)
            for n in -2...2 {
                let x = r.midX + CGFloat(n) * r.width * 0.115
                var p = Path()
                p.move(to: CGPoint(x: r.midX, y: r.minY + r.height * 0.04))
                p.addCurve(to: CGPoint(x: r.midX, y: r.maxY - r.height * 0.04),
                           control1: CGPoint(x: x, y: r.minY + r.height * 0.30),
                           control2: CGPoint(x: x, y: r.minY + r.height * 0.75))
                context.stroke(p, with: .color(ink.opacity(n == 0 ? 0.8 : 0.38)), lineWidth: n == 0 ? 0.9 : 0.55)
            }
            for n in 0..<9 {
                let y = r.minY + r.height * CGFloat(0.20 + Double(n) * 0.068)
                let ww = r.width * CGFloat(0.20 + 0.15 * sin(Double(n) / 8 * .pi))
                var p = Path()
                p.move(to: CGPoint(x: r.midX - ww, y: y - 2))
                p.addQuadCurve(to: CGPoint(x: r.midX + ww, y: y + 2), control: CGPoint(x: r.midX, y: y + 6))
                context.stroke(p, with: .color(ink.opacity(0.14)), lineWidth: 0.6)
            }
            // One ruled leader sets the specimen apart from the map's geographic marks.
            var leader = Path()
            leader.move(to: CGPoint(x: r.maxX + 10, y: r.midY))
            leader.addLine(to: CGPoint(x: min(size.width - 12, r.maxX + 40), y: r.midY))
            context.stroke(leader, with: .color(ink.opacity(0.4)), lineWidth: 0.6)
        }
        .accessibilityLabel("Botanical drawing of a rice grain; stylised, not an excavation photograph")
    }

    static func grain(in r: CGRect) -> Path {
        var p = Path()
        p.move(to: CGPoint(x: r.midX, y: r.minY))
        p.addCurve(to: CGPoint(x: r.midX, y: r.maxY),
                   control1: CGPoint(x: r.maxX + r.width * 0.2, y: r.minY + r.height * 0.21),
                   control2: CGPoint(x: r.maxX, y: r.minY + r.height * 0.78))
        p.addCurve(to: CGPoint(x: r.midX, y: r.minY),
                   control1: CGPoint(x: r.minX, y: r.minY + r.height * 0.78),
                   control2: CGPoint(x: r.minX - r.width * 0.2, y: r.minY + r.height * 0.21))
        p.closeSubpath()
        return p
    }
}
