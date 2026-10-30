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
    /// A date and the seconds of playback at which it is reached (runtime 5, `pace`).
    struct Stop: Equatable {
        let date: Double
        let seconds: Double
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
    /// The date passing unevenly (runtime 5): playback reaches each stop's date that many seconds after
    /// the first, and the strip is laid out in those seconds, so an era given more of them has more of
    /// the strip. Empty: evenly, at `rate`.
    var pace: [Stop] = []
    /// How fast playback runs, 1 as written (runtime 5, `terra.time.speed`): a pack may slow its date
    /// as the reader comes close, so what moves with it walks rather than races.
    var speed: Double = 1

    /// "1945", "300 BCE", "26 February 1873"; before 10,000 BCE, years ago: "45 million years ago".
    var label: String {
        switch unit {
        case .year:
            // (the range is clamped to five billion years either way, so this can't overflow)
            let y = Int(value.rounded())
            if y <= -10_000 { return Self.yearsAgo(Double(Self.thisYear - y)) }
            return y < 1 ? String(localized: "\(1 - y) BCE") : String(y)
        case .day:
            return Self.days.string(from: Date(timeIntervalSince1970: value.rounded(.down) * 86_400))
        }
    }

    /// "45 million years ago", "3.4 million years ago", "850,000 years ago", "14,000 years ago": two
    /// significant figures, as deep time is told.
    static func yearsAgo(_ ago: Double) -> String {
        if ago >= 9.95e8 {
            let billions = (ago / 1e8).rounded() / 10
            return String(localized: "\(billions.formatted(.number.precision(.fractionLength(0...1)))) billion years ago")
        }
        if ago >= 9.95e5 {
            let millions = ago >= 9.95e6 ? (ago / 1e6).rounded() : (ago / 1e5).rounded() / 10
            return String(localized: "\(millions.formatted(.number.precision(.fractionLength(0...1)))) million years ago")
        }
        let magnitude = pow(10, floor(log10(Swift.max(ago, 1))) - 1)
        let rounded = Int((ago / magnitude).rounded() * magnitude)
        return String(localized: "\(rounded.formatted(.number.grouping(.automatic))) years ago")
    }

    private static let thisYear = Calendar(identifier: .gregorian).component(.year, from: Date())

    /// As a year with its fraction, so a date can pass between prototypes counted in other units.
    var year: Double { unit == .year ? value : 1970 + value / 365.2425 }

    /// The value in this unit for a year with its fraction.
    func value(forYear y: Double) -> Double { unit == .year ? y : (y - 1970) * 365.2425 }

    func clamped(_ v: Double) -> Double { Swift.min(Swift.max(v, min), max) }

    /// Where a date lies along the strip, 0 at its start and 1 at its end: evenly, or by the pace.
    func fraction(_ v: Double) -> Double {
        guard let total = pace.last?.seconds, total > 0 else { return (clamped(v) - min) / (max - min) }
        return seconds(at: v) / total
    }

    /// The date at a fraction of the strip.
    func value(atFraction f: Double) -> Double {
        let f = Swift.min(Swift.max(f, 0), 1)
        guard let total = pace.last?.seconds, total > 0 else { return min + f * (max - min) }
        return date(atSeconds: f * total)
    }

    /// The date `dt` seconds of playback after `v` (at its speed), at most the end.
    func advanced(_ v: Double, by dt: Double) -> Double {
        guard !pace.isEmpty else { return Swift.min(max, v + rate * speed * dt) }
        return date(atSeconds: seconds(at: v) + dt * speed)
    }

    /// Seconds of playback at which the pace reaches a date.
    private func seconds(at v: Double) -> Double {
        guard let first = pace.first, let last = pace.last else { return 0 }
        if v <= first.date { return first.seconds }
        if v >= last.date { return last.seconds }
        let i = pace.firstIndex { $0.date >= v } ?? pace.count - 1
        let a = pace[i - 1], b = pace[i]
        return a.seconds + (v - a.date) / (b.date - a.date) * (b.seconds - a.seconds)
    }

    private func date(atSeconds s: Double) -> Double {
        guard let first = pace.first, let last = pace.last else { return min }
        if s <= first.seconds { return first.date }
        if s >= last.seconds { return last.date }
        let i = pace.firstIndex { $0.seconds >= s } ?? pace.count - 1
        let a = pace[i - 1], b = pace[i]
        return a.date + (s - a.seconds) / (b.seconds - a.seconds) * (b.date - a.date)
    }

    /// The next tick past the value (VoiceOver's increment), or a hundredth of the strip, never less
    /// than the one year or day the label shows.
    func step(_ direction: Double) -> Double {
        let ahead = direction > 0 ? ticks.map(\.at).filter { $0 > value + 1e-6 }.min() : ticks.map(\.at).filter { $0 < value - 1e-6 }.max()
        if let ahead { return clamped(ahead) }
        let next = value(atFraction: fraction(value) + direction / 100)
        return clamped(abs(next - value) >= 1 ? next : value + direction)
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
        // five billion years either way (ten million in days for a day unit) is room enough for the
        // Earth's age, and keeps every label finite
        let unit = Unit(rawValue: (o["unit"] as? String) ?? "year") ?? .year
        let limit = unit == .year ? 5e9 : 3.66e9
        // a pace (runtime 5): dates and seconds both rising; its first and last dates are the range
        let pace: [Stop] = ((o["pace"] as? [Any]) ?? []).prefix(200).compactMap { s in
            guard let pair = (s as? [Any])?.compactMap(number), pair.count == 2, abs(pair[0]) <= limit, pair[1] >= 0 else { return nil }
            return Stop(date: pair[0], seconds: pair[1])
        }
        let paced = pace.count >= 2 && zip(pace, pace.dropFirst()).allSatisfy { $1.date > $0.date && $1.seconds > $0.seconds }
        guard let lo = paced ? pace.first?.date : number(o["min"]).map({ Swift.max($0, -limit) }),
              let hi = paced ? pace.last?.date : number(o["max"]).map({ Swift.min($0, limit) }), hi > lo else { return nil }
        let ticks: [Tick] = ((o["ticks"] as? [Any]) ?? []).prefix(2000).compactMap { t in
            if let at = number(t) { return Tick(at: at, hollow: false) }
            if let d = t as? [String: Any], let at = number(d["at"]) { return Tick(at: at, hollow: (d["hollow"] as? Bool) ?? false) }
            return nil
        }.filter { $0.at >= lo && $0.at <= hi }
        return GlobeTime(unit: unit, min: lo, max: hi, value: Swift.min(Swift.max(number(o["value"]) ?? lo, lo), hi),
                         ticks: ticks, playable: (o["playable"] as? Bool) ?? false,
                         rate: Swift.min(Swift.max(number(o["rate"]) ?? 1, 0.001), (hi - lo) / 2),
                         pace: paced ? pace.map { Stop(date: $0.date, seconds: $0.seconds - pace[0].seconds) } : [])
    }
}

/// The date a stage shows, as its dated sprites read it every frame (runtime 5): playing, it runs on
/// from where playback last set it, so a sprite moves smoothly between playback's steps.
struct DateClock {
    let time: GlobeTime
    let stamp: CFTimeInterval

    func value(at now: CFTimeInterval) -> Double {
        guard time.playing else { return time.value }
        // (playback steps about thirty times a second; past a tenth of a second it is held, and so is this)
        return time.advanced(time.value, by: Swift.min(0.1, Swift.max(0, now - stamp)))
    }
}

/// What a prototype says at the foot of the screen: what this is (`title`, above the date) and a line
/// or two more (`text`, below it).
struct StageCaption: Equatable {
    let title: String
    let text: String
    /// The Wikipedia article on its subject, when the prototype names one: with Wikipedia's words on, its
    /// first sentence is the text, and a link to it follows.
    var article: WikipediaArticle? = nil
}

/// The foot of the screen while a prototype gives it words or a date: the globe's caption type, and
/// the date's strip under it. It replaces the globe's own caption, in the same place.
struct GlobeFoot: View {
    let caption: StageCaption?
    let time: GlobeTime?
    let onTime: (Double) -> Void
    let onPlay: (Bool) -> Void

    private var wikipedia: Bool { WikipediaWords.shared.on }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            CaptionLines(caption: Plate.Caption(eyebrow: caption?.title ?? "", title: time?.label ?? "",
                                                text: WordsRule.line(caption?.text ?? "", article: caption?.article)),
                         style: GlobeScreen.style)
                .accessibilityElement(children: .combine)
                .accessibilityIdentifier("globeFootCaption")
            if wikipedia, let article = caption?.article {
                WikipediaLink(url: article.url, color: Color(uiColor: (GlobeScreen.style?.eyebrow?.color).flatMap { UIColor(hex: $0) } ?? .systemBlue))
                    .padding(.top, -8)
            }
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
                        onTime(time.value(atFraction: Double(x / max(g.size.width, 1))))
                    }
                })
            }
            .frame(height: 44)
            .accessibilityElement()
            .accessibilityAddTraits(time.playing ? .updatesFrequently : [])
            .accessibilityLabel(Text("Date"))
            .accessibilityValue(Text(spoken))
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
        CGFloat(time.fraction(v)) * width
    }

    /// What VoiceOver reads as the value: the label, and where along the strip when a step would leave
    /// the label as it is (a night told in hours under one year's name: the position is the progress).
    private var spoken: String {
        var next = time
        next.value = time.step(1)
        var back = time
        back.value = time.step(-1)
        guard next.label == time.label, back.label == time.label, time.max > time.min else { return time.label }
        let percent = Int((time.fraction(time.value) * 100).rounded())
        return String(localized: "\(time.label), \(percent) per cent along")
    }
}

extension GlobeTime {
    /// The month (1 to 12) of a date to the day, for the globe's season; a year has none.
    var month: Int? {
        guard unit == .day, value.isFinite else { return nil }
        var utc = Calendar(identifier: .gregorian)
        utc.timeZone = TimeZone(identifier: "UTC") ?? .current
        return utc.component(.month, from: Date(timeIntervalSince1970: value.rounded(.down) * 86_400))
    }
}
