import Foundation
import SwiftUI

/// The globe's date while a time-aware prototype runs (terra.js runtime 2, `terra.time`): a value the
/// reader drags along a strip at the foot of the screen, with ticks where the prototype's evidence is.
/// Years, or days since 1 January 1970 (UTC). It belongs to the stage and ends with it.
struct GlobeTime: Equatable {
    enum Unit: String { case year, day }
    struct Tick: Equatable {
        let at: Double
        let hollow: Bool
    }
    /// Who moved it, as the page hears it.
    enum Mover: String { case reader, play, host }

    var unit: Unit
    var min: Double
    var max: Double
    var value: Double
    var ticks: [Tick]
    var playable: Bool
    /// Units a second while playing.
    var rate: Double
    var playing = false

    /// "1945", "300 BCE", "26 February 1873".
    var label: String {
        switch unit {
        case .year:
            // (the range is clamped to ten million years either way, so this can't overflow)
            let y = Int(value.rounded())
            return y < 1 ? String(localized: "\(1 - y) BCE") : String(y)
        case .day:
            return Self.days.string(from: Date(timeIntervalSince1970: value.rounded(.down) * 86_400))
        }
    }

    /// As a year with its fraction, so a date can pass between prototypes counted in other units.
    var year: Double { unit == .year ? value : 1970 + value / 365.2425 }

    /// The value in this unit for a year with its fraction.
    func value(forYear y: Double) -> Double { unit == .year ? y : (y - 1970) * 365.2425 }

    func clamped(_ v: Double) -> Double { Swift.min(Swift.max(v, min), max) }

    /// The next tick past the value (VoiceOver's increment), or a hundredth of the span, never less
    /// than the one year or day the label shows.
    func step(_ direction: Double) -> Double {
        let ahead = direction > 0 ? ticks.map(\.at).filter { $0 > value + 1e-6 }.min() : ticks.map(\.at).filter { $0 < value - 1e-6 }.max()
        return clamped(ahead ?? value + direction * Swift.max(1, (max - min) / 100))
    }

    private static let days: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_GB")
        f.timeZone = TimeZone(identifier: "UTC")
        f.dateFormat = "d MMMM y"
        return f
    }()

    /// From `terra.time.show(options)`; nil if its range is not a range.
    static func from(_ o: [String: Any]) -> GlobeTime? {
        func number(_ x: Any?) -> Double? { (x as? NSNumber).map(\.doubleValue).flatMap { $0.isFinite ? $0 : nil } }
        // ten million years either way (in days for a day unit) is room enough, and keeps every label finite
        let unit = Unit(rawValue: (o["unit"] as? String) ?? "year") ?? .year
        let limit = unit == .year ? 1e7 : 3.66e9
        guard let lo = number(o["min"]).map({ Swift.max($0, -limit) }), let hi = number(o["max"]).map({ Swift.min($0, limit) }), hi > lo else { return nil }
        let ticks: [Tick] = ((o["ticks"] as? [Any]) ?? []).prefix(2000).compactMap { t in
            if let at = number(t) { return Tick(at: at, hollow: false) }
            if let d = t as? [String: Any], let at = number(d["at"]) { return Tick(at: at, hollow: (d["hollow"] as? Bool) ?? false) }
            return nil
        }.filter { $0.at >= lo && $0.at <= hi }
        return GlobeTime(unit: unit, min: lo, max: hi, value: Swift.min(Swift.max(number(o["value"]) ?? lo, lo), hi),
                         ticks: ticks, playable: (o["playable"] as? Bool) ?? false,
                         rate: Swift.min(Swift.max(number(o["rate"]) ?? 1, 0.001), (hi - lo) / 2))
    }
}

/// What a prototype says at the foot of the screen: what this is (`title`, above the date) and a line
/// or two more (`text`, below it).
struct StageCaption: Equatable {
    let title: String
    let text: String
}

/// The foot of the screen while a prototype gives it words or a date: the globe's caption type, and
/// the date's strip under it. It replaces the globe's own caption, in the same place.
struct GlobeFoot: View {
    let caption: StageCaption?
    let time: GlobeTime?
    let onTime: (Double) -> Void
    let onPlay: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            CaptionLines(caption: Plate.Caption(eyebrow: caption?.title ?? "", title: time?.label ?? "", text: caption?.text ?? ""),
                         style: GlobeScreen.style)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("globeFootCaption")
            if let time {
                DateStrip(time: time, onTime: onTime, onPlay: onPlay)
            }
        }
        .frame(maxWidth: CaptionLines.widest, alignment: .leading)
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 24)
        .padding(.top, 70)
        .padding(.bottom, 14)
        // (a shape style, so the paper reaches the screen's edge below the date, as the caption's does)
        .background(LinearGradient(stops: [.init(color: .clear, location: 0), .init(color: scrim.opacity(0.85), location: 0.3),
                                           .init(color: scrim, location: 0.55)], startPoint: .top, endPoint: .bottom),
                    ignoresSafeAreaEdges: .bottom)
    }

    private var scrim: Color {
        Color(uiColor: GlobeScreen.style?.scrim.flatMap { UIColor(hex: $0) } ?? UIColor(red: 0.965, green: 0.949, blue: 0.906, alpha: 0.9))
    }
}

/// The date's strip: a hairline, the ticks, a needle at the date; drag or touch anywhere along it.
private struct DateStrip: View {
    let time: GlobeTime
    let onTime: (Double) -> Void
    let onPlay: (Bool) -> Void

    private static let ink = Color(red: 0.145, green: 0.204, blue: 0.251)
    private static let needle = Color(red: 0.69, green: 0.259, blue: 0.204)

    var body: some View {
        HStack(spacing: 6) {
            if time.playable {
                Button { onPlay(!time.playing) } label: {
                    Image(systemName: time.playing ? "pause.fill" : "play.fill")
                        .font(.system(size: 15, weight: .semibold))
                        .frame(width: 44, height: 44)
                        .contentShape(Rectangle())
                }
                .foregroundStyle(Self.ink)
                .accessibilityLabel(time.playing ? Text("Pause") : Text("Play"))
                .accessibilityIdentifier("globeDatePlay")
            }
            GeometryReader { g in
                Canvas { context, size in
                    let y = size.height / 2
                    var line = Path()
                    line.move(to: CGPoint(x: 0, y: y))
                    line.addLine(to: CGPoint(x: size.width, y: y))
                    context.stroke(line, with: .color(Self.ink.opacity(0.35)), lineWidth: 1)
                    // (many ticks, closer than a dot's width, read as fine strokes rather than a blur of dots)
                    let fine = Double(time.ticks.count) * 7 > Double(size.width)
                    for t in time.ticks {
                        let x = position(t.at, width: size.width)
                        if fine {
                            var stroke = Path()
                            stroke.move(to: CGPoint(x: x, y: y - 4))
                            stroke.addLine(to: CGPoint(x: x, y: y + 4))
                            context.stroke(stroke, with: .color(Self.ink.opacity(t.hollow ? 0.35 : 0.8)), lineWidth: 1)
                            continue
                        }
                        let dot = Path(ellipseIn: CGRect(x: x - 2.6, y: y - 2.6, width: 5.2, height: 5.2))
                        if t.hollow {
                            context.fill(dot, with: .color(Color(red: 0.965, green: 0.949, blue: 0.906)))
                            context.stroke(dot, with: .color(Self.ink), lineWidth: 1)
                        } else {
                            context.fill(dot, with: .color(Self.ink))
                        }
                    }
                    let x = position(time.value, width: size.width)
                    var mark = Path()
                    mark.move(to: CGPoint(x: x, y: y - 12))
                    mark.addLine(to: CGPoint(x: x, y: y + 8))
                    context.stroke(mark, with: .color(Self.needle), style: StrokeStyle(lineWidth: 2, lineCap: .round))
                }
                .contentShape(Rectangle())
                .gesture(DragGesture(minimumDistance: 0).onChanged { drag in
                    let x = drag.location.x
                    // a touch within a fingertip of a tick lands on it: the dated map, the station
                    let near = time.ticks.min { abs(position($0.at, width: g.size.width) - x) < abs(position($1.at, width: g.size.width) - x) }
                    if let near, abs(position(near.at, width: g.size.width) - x) <= 10 {
                        onTime(near.at)
                    } else {
                        let f = Double(min(max(x / max(g.size.width, 1), 0), 1))
                        onTime(time.min + f * (time.max - time.min))
                    }
                })
            }
            .frame(height: 44)
            .accessibilityElement()
            .accessibilityAddTraits(time.playing ? .updatesFrequently : [])
            .accessibilityLabel(Text("Date"))
            .accessibilityValue(Text(time.label))
            .accessibilityHint(Text("Swipe up or down to move through time"))
            .accessibilityAdjustableAction { direction in
                switch direction {
                case .increment: onTime(time.step(1))
                case .decrement: onTime(time.step(-1))
                @unknown default: break
                }
            }
            .accessibilityIdentifier("globeDate")
        }
    }

    private func position(_ v: Double, width: CGFloat) -> CGFloat {
        CGFloat((v - time.min) / (time.max - time.min)) * width
    }
}
