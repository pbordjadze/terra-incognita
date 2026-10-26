import CoreGraphics
import Foundation

/// Where the camera looks: the point (x, y) of plate `plate`, in that plate's pixels, sits at the
/// screen's centre, and one of its pixels covers `zoom` screen points.
nonisolated struct Camera: Equatable, Sendable {
    var plate: Int
    var x: Double
    var y: Double
    var zoom: Double
}

/// A look's plates as one continuous zoom: converting a camera between nested plates, the zoom at which
/// each plate is at rest, and how opaque each is drawn.
nonisolated struct Ladder: Sendable {
    let plates: [Plate]
    /// The screen, in points.
    var screen: CGSize

    var count: Int { plates.count }

    /// At rest a plate covers the screen 1.4 times over, so the camera can drift without an edge showing.
    func restZoom(_ k: Int) -> Double { 1.4 * coverZoom(k) }
    /// The zoom at which plate k just covers the screen.
    func coverZoom(_ k: Int) -> Double {
        max(Double(screen.width) / plates[k].width, Double(screen.height) / plates[k].height)
    }

    /// Plate k's pixels per pixel of its child, plate k + 1.
    private func ratio(_ k: Int) -> Double { plates[k].child![2] / plates[k + 1].width }
    private func origin(_ k: Int) -> (Double, Double) { (plates[k].child![0], plates[k].child![1]) }

    /// The same view, expressed in the parent plate.
    func up(_ c: Camera) -> Camera {
        let k = c.plate - 1, f = ratio(k), (ox, oy) = origin(k)
        return Camera(plate: k, x: ox + c.x * f, y: oy + c.y * f, zoom: c.zoom / f)
    }

    /// The same view, expressed in the child plate.
    func down(_ c: Camera) -> Camera {
        let k = c.plate, f = ratio(k), (ox, oy) = origin(k)
        return Camera(plate: k + 1, x: (c.x - ox) / f, y: (c.y - oy) / f, zoom: c.zoom * f)
    }

    func express(_ c: Camera, in k: Int) -> Camera {
        var c = c
        while c.plate > k { c = up(c) }
        while c.plate < k { c = down(c) }
        return c
    }

    /// The same view in the deepest plate that is drawn opaque (with a little hysteresis, so a camera
    /// resting on a threshold doesn't flip between plates): numbers stay small however deep the dive.
    func normalized(_ c: Camera) -> Camera {
        var c = c
        while c.plate < count - 1, down(c).zoom >= 0.6 * restZoom(c.plate + 1) { c = down(c) }
        while c.plate > 0, c.zoom < 0.55 * restZoom(c.plate) { c = up(c) }
        return c
    }

    /// How opaque plate k is drawn when the camera puts it at `zoom`: the first plate always, each
    /// later one fading up while it is still drawn smaller than its pixels (sharp), so the wider plate
    /// behind is never blown up much past its own; but only once the camera has pushed in past the
    /// wider plate's rest, so a plate never shows through while the film rests on the one before.
    func opacity(_ k: Int, zoom: Double) -> Double {
        guard k > 0 else { return 1 }
        let atParentRest = restZoom(k - 1) * ratio(k - 1)
        let start = max(0.25 * restZoom(k), 1.3 * atParentRest)
        return smoothstep(start, max(0.6 * restZoom(k), start * 1.25), zoom)
    }

    /// Plate k centred, at rest.
    func rest(_ k: Int) -> Camera {
        Camera(plate: k, x: plates[k].width / 2, y: plates[k].height / 2, zoom: restZoom(k))
    }

    /// Keeps the camera on the pictures: the first plate may be pulled back until it fits the screen,
    /// the last pushed in to three times its rest, and the centre stays where the plate covers the
    /// screen.
    func clamped(_ c: Camera) -> Camera {
        var c = c
        let p = plates[c.plate]
        if c.plate == 0 {
            let fit = min(Double(screen.width) / p.width, Double(screen.height) / p.height)
            c.zoom = max(c.zoom, fit)
        }
        if c.plate == count - 1 { c.zoom = min(c.zoom, 3 * restZoom(c.plate)) }
        let hw = Double(screen.width) / 2 / c.zoom, hh = Double(screen.height) / 2 / c.zoom
        c.x = hw * 2 >= p.width ? p.width / 2 : min(max(c.x, hw), p.width - hw)
        c.y = hh * 2 >= p.height ? p.height / 2 : min(max(c.y, hh), p.height - hh)
        return c
    }
}

nonisolated func smoothstep(_ a: Double, _ b: Double, _ x: Double) -> Double {
    let t = min(max((x - a) / (b - a), 0), 1)
    return t * t * (3 - 2 * t)
}

nonisolated func easeInOut(_ t: Double) -> Double {
    let t = min(max(t, 0), 1)
    return t < 0.5 ? 4 * t * t * t : 1 - pow(-2 * t + 2, 3) / 2
}

/// A move between two views of the same plate: the zoom goes evenly in its logarithm, and the centre
/// keeps pace with how far the zoom has come, so a point the camera dives towards stays in view.
nonisolated func glide(_ a: Camera, _ b: Camera, _ e: Double) -> Camera {
    let zoom = exp(log(a.zoom) + (log(b.zoom) - log(a.zoom)) * e)
    let w = abs(b.zoom - a.zoom) < 1e-9 ? e : (1 / a.zoom - 1 / zoom) / (1 / a.zoom - 1 / b.zoom)
    return Camera(plate: a.plate, x: a.x + (b.x - a.x) * w, y: a.y + (b.y - a.y) * w, zoom: zoom)
}

/// The film: on each plate a slow drift towards the next, its caption showing, then a dive into it,
/// straight through the plates that only pass; on the last a longer drift, and the end.
nonisolated struct Film: Sendable {
    nonisolated struct Shot: Sendable {
        let start: Double, end: Double
        let from: Camera, to: Camera
        /// A drift shows plate `plate`'s caption; a dive doesn't.
        let drift: Bool
    }

    let shots: [Shot]
    var duration: Double { shots.last?.end ?? 0 }

    init(_ ladder: Ladder) {
        var shots: [Shot] = [], t = 0.0
        let stops = ladder.plates.indices.filter { !ladder.plates[$0].passes }
        for (i, k) in stops.enumerated() {
            let last = i == stops.count - 1
            let start = ladder.rest(k)
            // the next stop at rest, seen from this plate
            let target = last ? start : ladder.express(ladder.rest(stops[i + 1]), in: k)
            var drift = start
            if last {
                drift.zoom *= 1.18
                drift.y -= ladder.plates[k].height * 0.04
            } else {
                drift.x += (target.x - start.x) * 0.3
                drift.y += (target.y - start.y) * 0.3
                drift.zoom *= 1.12
            }
            let dwell = i == 0 ? 8.0 : last ? 11.0 : 7.0
            shots.append(Shot(start: t, end: t + dwell, from: start, to: drift, drift: true))
            t += dwell
            if !last {
                let dive = 2.4 + 0.8 * log(target.zoom / drift.zoom)
                shots.append(Shot(start: t, end: t + dive, from: drift, to: target, drift: false))
                t += dive
            }
        }
        self.shots = shots
    }

    func camera(at t: Double) -> Camera {
        guard let shot = shots.first(where: { t < $0.end }) ?? shots.last else { return Camera(plate: 0, x: 0, y: 0, zoom: 1) }
        let u = (t - shot.start) / (shot.end - shot.start)
        return glide(shot.from, shot.to, easeInOut(u))
    }

    /// The plate whose caption shows at t, if any.
    func caption(at t: Double) -> Int? {
        guard let shot = shots.first(where: { t >= $0.start && t < $0.end }), shot.drift,
              t > shot.start + 0.6, t < shot.end - 0.4 else { return nil }
        return shot.from.plate
    }
}
