//! Strokes (`wake.wgsl`, R10): continuous tapering lines. The wake of every lamp is one drawn line
//! through its daily places, softened and smoothed, ending at the lamp; the screen's marks (the
//! panel's journeys and lamps, the record control's ring) are the same strokes in points.
use super::{mark_pipeline, marks_wgsl, Instances, Uniform};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

/// One segment of a line, a to b, with its neighbours so it can cut its piece at each turn's bisector.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct StrokeInstance {
    /// the point before a (a itself at a cap), and a's width in reading points
    pub prev: [f32; 4],
    /// a, and its alpha
    pub a: [f32; 4],
    /// b, and its width in reading points
    pub b: [f32; 4],
    /// the point after b (b itself at a cap), and b's alpha
    pub next: [f32; 4],
    /// the hue; w 1 for points on the screen (x, y in points), 0 for unit vectors on the globe
    pub hue: [f32; 4],
    /// a round cap at a, at b, night, hollow (0..1)
    pub flags: [f32; 4],
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<StrokeInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WakeUniform {
    /// 0 this draw lays the edges, 1 the lines
    pub layer: f32,
    /// 1 a wind line (R21): no edge, paler, feathered
    pub airy: f32,
    pub _pad: [f32; 2],
}

pub struct Wake {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<StrokeInstance>,
    /// the edges' draw, then the lines'
    uniforms: [Uniform<WakeUniform>; 2],
}

impl Wake {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        Self::with(gpu, view, false)
    }

    /// The wind's lines (R21): the same strokes drawn light and feathered, with no dark edge.
    pub fn airy(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        Self::with(gpu, view, true)
    }

    fn with(gpu: &Gpu, view: &wgpu::BindGroupLayout, airy: bool) -> Self {
        let module = gpu.shader("wake", &marks_wgsl(include_str!("wake.wgsl")));
        let uniforms = [
            Uniform::new(gpu, "wake edges"),
            Uniform::new(gpu, "wake lines"),
        ];
        for (i, u) in uniforms.iter().enumerate() {
            u.write(
                gpu,
                &WakeUniform {
                    layer: i as f32,
                    airy: airy as u8 as f32,
                    _pad: [0.0; 2],
                },
            );
        }
        Self {
            pipeline: mark_pipeline(
                gpu,
                "wake",
                &module,
                LAYOUT,
                view,
                Some(&uniforms[0].layout),
            ),
            instances: Instances::new(gpu, "wake", 1024),
            uniforms,
        }
    }

    pub fn prepare(&mut self, gpu: &Gpu, segments: &[StrokeInstance]) {
        self.instances.write(gpu, segments);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        for u in &self.uniforms {
            pass.set_bind_group(1, &u.group, &[]);
            pass.draw(0..6, 0..self.instances.count);
        }
    }
}

/// A point of a line: where (a unit vector, or x, y points on the screen), its width in reading
/// points, its alpha, how hollow (0..1).
#[derive(Clone, Copy, Debug)]
pub struct Pt {
    pub p: [f32; 3],
    pub w: f32,
    pub a: f32,
    pub hollow: f32,
}

/// The segments of one line through `pts`: open with round caps at its ends, or `closed` (a ring).
/// One point is a dot of its width.
pub fn line(
    out: &mut Vec<StrokeInstance>,
    pts: &[Pt],
    hue: [f32; 3],
    screen: bool,
    night: bool,
    closed: bool,
) {
    let s = screen as u8 as f32;
    let nf = night as u8 as f32;
    let h4 = [hue[0], hue[1], hue[2], s];
    let seg =
        |a: &Pt, b: &Pt, prev: [f32; 3], next: [f32; 3], cap_a: bool, cap_b: bool| StrokeInstance {
            prev: [prev[0], prev[1], prev[2], a.w],
            a: [a.p[0], a.p[1], a.p[2], a.a],
            b: [b.p[0], b.p[1], b.p[2], b.w],
            next: [next[0], next[1], next[2], b.a],
            hue: h4,
            flags: [
                cap_a as u8 as f32,
                cap_b as u8 as f32,
                nf,
                0.5 * (a.hollow + b.hollow),
            ],
        };
    match pts.len() {
        0 => {}
        1 => {
            let a = pts[0];
            let mut b = a;
            // (a dot: a segment too short to see, round at both ends)
            if screen {
                b.p[0] += 0.01
            } else {
                b.p[0] += 1e-6
            }
            out.push(seg(&a, &b, a.p, b.p, true, true));
        }
        n => {
            let segs = if closed { n } else { n - 1 };
            for i in 0..segs {
                let (a, b) = (&pts[i], &pts[(i + 1) % n]);
                let prev = if i > 0 {
                    pts[i - 1].p
                } else if closed {
                    pts[n - 1].p
                } else {
                    a.p
                };
                let next = if i + 2 < n {
                    pts[i + 2].p
                } else if closed {
                    pts[(i + 2) % n].p
                } else {
                    b.p
                };
                out.push(seg(
                    a,
                    b,
                    prev,
                    next,
                    !closed && i == 0,
                    !closed && i + 1 == segs,
                ));
            }
        }
    }
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// A path's places softened and smoothed into one drawn line: the day-to-day wander of an eddy (real,
/// Truth's rules) eased by a light low-pass with the ends held (so the line still starts where the
/// path did and ends at its lamp), then a Catmull-Rom curve through what remains, `sub` points a step.
/// Each place carries a value (its measuredness) that is smoothed along with it.
pub fn smooth(places: &[([f32; 3], f32)], passes: usize, sub: usize) -> Vec<([f32; 3], f32, f32)> {
    smooth_with(places, passes, sub, true)
}

/// The same for points on a plane (the panel's journeys): each point and how far along, 0..1.
pub fn smooth_flat(places: &[([f32; 3], f32)], passes: usize, sub: usize) -> Vec<([f32; 3], f32)> {
    smooth_with(places, passes, sub, false)
        .into_iter()
        .map(|(p, _, f)| (p, f))
        .collect()
}

fn smooth_with(
    places: &[([f32; 3], f32)],
    passes: usize,
    sub: usize,
    unit: bool,
) -> Vec<([f32; 3], f32, f32)> {
    let norm = |v: [f32; 3]| if unit { norm(v) } else { v };
    // (points closer than a few hundred metres are one: a float held at a coast for days)
    let mut p: Vec<([f32; 3], f32)> = Vec::with_capacity(places.len());
    for &q in places {
        if let Some(l) = p.last() {
            let d =
                ((l.0[0] - q.0[0]).powi(2) + (l.0[1] - q.0[1]).powi(2) + (l.0[2] - q.0[2]).powi(2))
                    .sqrt();
            if d < if unit { 5e-5 } else { 0.03 } {
                continue;
            }
        }
        p.push(q);
    }
    if p.len() < 3 {
        let n = p.len().saturating_sub(1).max(1) as f32;
        return p
            .iter()
            .enumerate()
            .map(|(i, q)| (q.0, q.1, i as f32 / n))
            .collect();
    }
    for _ in 0..passes {
        let q = p.clone();
        for i in 1..q.len() - 1 {
            let v =
                [0, 1, 2].map(|k| 0.25 * q[i - 1].0[k] + 0.5 * q[i].0[k] + 0.25 * q[i + 1].0[k]);
            p[i] = (
                norm(v),
                0.25 * q[i - 1].1 + 0.5 * q[i].1 + 0.25 * q[i + 1].1,
            );
        }
    }
    let n = p.len();
    let mut out = Vec::with_capacity((n - 1) * sub + 1);
    let last = (n - 1) as f32;
    for i in 0..n - 1 {
        let p0 = if i > 0 { p[i - 1].0 } else { p[i].0 };
        let (p1, p2) = (p[i].0, p[i + 1].0);
        let p3 = if i + 2 < n { p[i + 2].0 } else { p[i + 1].0 };
        for j in 0..sub {
            let t = j as f32 / sub as f32;
            let (t2, t3) = (t * t, t * t * t);
            let v = [0, 1, 2].map(|k| {
                0.5 * (2.0 * p1[k]
                    + (-p0[k] + p2[k]) * t
                    + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2
                    + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3)
            });
            out.push((
                norm(v),
                p[i].1 + (p[i + 1].1 - p[i].1) * t,
                (i as f32 + t) / last,
            ));
        }
    }
    out.push((p[n - 1].0, p[n - 1].1, 1.0));
    out
}

/// A lamp's wake as one line: its daily places oldest first and where it is now, a ribbon that is
/// widest and fullest at the lamp and tapers and fades with age to nothing at its oldest day. A lamp
/// chosen as itself (`follow`, R8) carries its whole journey, fuller and never fading below a third,
/// so it reads as a line of its colour at any zoom. `fade` dims the whole line (0..1); `heavy` widens
/// it (2 in a clip, R12, where the line must carry the frame).
#[allow(clippy::too_many_arguments)] // each argument is one thing the call draws or reads
pub fn wake(
    out: &mut Vec<StrokeInstance>,
    path: &[super::LookBead],
    now: crate::model::LonLat,
    night: bool,
    hue: [f32; 3],
    follow: bool,
    fade: f32,
    heavy: f32,
    rad_per_pt: f32,
) {
    let mut places: Vec<([f32; 3], f32)> = path
        .iter()
        .map(|b| (super::unit(b.at), b.measured))
        .collect();
    // (R15: steps kept a little ahead of where the lamp is drawn would send the line past the lamp
    // and back, a hook at its head: the line is cut at the step nearest the lamp, among the last few)
    let here = super::unit(now);
    let d2 = |p: [f32; 3]| {
        (p[0] - here[0]).powi(2) + (p[1] - here[1]).powi(2) + (p[2] - here[2]).powi(2)
    };
    let tail = places.len().saturating_sub(16);
    if let Some(k) =
        (tail..places.len()).min_by(|&i, &j| d2(places[i].0).partial_cmp(&d2(places[j].0)).unwrap())
    {
        places.truncate(k + 1);
    }
    places.push((here, path.last().map_or(1.0, |b| b.measured)));
    // (R17: a centripetal Catmull-Rom through the steps, cut into pieces of about two reading points,
    // so however far apart the steps are on the screen the line is a curve, never a polyline)
    let line_pts: Vec<Pt> = round_corners(
        spline(&places, 1, 2.0 * rad_per_pt),
        10.0 * heavy * rad_per_pt,
    )
    .into_iter()
    .map(|(p, measured, f)| {
        let age = 1.0 - f;
        let (w, a) = if follow && heavy > 1.0 {
            // (a journey clip, R14: the whole journey drawn and held, the dance's shape
            // accumulating; the newest part brightest and widest, nothing fading off the tail,
            // and the line nearly opaque so two lamps' lines cross cleanly, each in its own hue
            // on its own dark edge, the later over the earlier, never a muddy mix)
            (2.3 + 1.0 * (1.0 - age), 0.82 + 0.18 * (1.0 - age).powf(2.0))
        } else if follow {
            (3.0 - 1.5 * age, 1.0 - 0.62 * age.powf(1.5))
        } else if heavy > 1.0 {
            // (in a clip, R12: the tail held longer, so a take's whole line is there)
            (3.2 - 2.0 * age, 0.95 * (1.0 - age).powf(0.7))
        } else {
            (3.2 - 2.4 * age, 0.92 * (1.0 - age).powf(1.3))
        };
        Pt {
            p,
            w: w * heavy,
            a: a * fade,
            hollow: if measured < 0.5 { 1.0 } else { 0.0 },
        }
    })
    .collect();
    line(out, &line_pts, hue, false, night, false);
}

/// A line's sharp turns rounded at the drawing's own scale (R15): the path resampled every third of
/// `radius` (radians on the globe: the reading points wanted over the globe's radius in points) and
/// eased by six soft passes, ends held, so a turn tighter than about `radius` (a lamp held at a
/// coast, turning back on itself) is drawn as a bend and not an elbow. The path data is untouched;
/// at a closer zoom the same turn rounds less, so the line stays true where it can be seen.
pub fn round_corners(pts: Vec<([f32; 3], f32, f32)>, radius: f32) -> Vec<([f32; 3], f32, f32)> {
    if pts.len() < 3 || !(radius > 0.0) {
        return pts;
    }
    let step = radius / 3.0;
    let dist = |a: [f32; 3], b: [f32; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    let total: f32 = pts.windows(2).map(|w| dist(w[0].0, w[1].0)).sum();
    // (never more than a few thousand points a line: from afar the step is coarse, close in the
    // line is shorter on the screen than it is long)
    let step = step.max(total / 4000.0);
    if total < step * 3.0 {
        return pts;
    }
    let mut out: Vec<([f32; 3], f32, f32)> = vec![pts[0]];
    let mut carry = 0.0;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let d = dist(a.0, b.0);
        let mut t = step - carry;
        while t < d {
            let u = t / d;
            out.push((
                norm([0, 1, 2].map(|k| a.0[k] + (b.0[k] - a.0[k]) * u)),
                a.1 + (b.1 - a.1) * u,
                a.2 + (b.2 - a.2) * u,
            ));
            t += step;
        }
        carry = d - (t - step);
    }
    let last = *pts.last().unwrap();
    if dist(out.last().unwrap().0, last.0) > step * 0.3 {
        out.push(last);
    } else {
        *out.last_mut().unwrap() = last;
    }
    for _ in 0..6 {
        let q = out.clone();
        for i in 1..q.len() - 1 {
            out[i].0 = norm(
                [0, 1, 2].map(|k| 0.25 * q[i - 1].0[k] + 0.5 * q[i].0[k] + 0.25 * q[i + 1].0[k]),
            );
        }
    }
    // (R17) a turn sharper than the soft passes can round (a lamp reversing within one step, its legs
    // long and straight) is found where the heading swings past sixty degrees over a few samples, and
    // only there the line is eased further, over about eight samples each side, so it bends through
    // the turn instead of meeting it in a V; the even curls of a loop are below the threshold and keep
    // their shape
    let m = out.len();
    if m > 8 {
        let dir = |a: [f32; 3], b: [f32; 3]| {
            let v = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
            [v[0] / l, v[1] / l, v[2] / l]
        };
        let mut corner = vec![0.0f32; m];
        for i in 2..m - 2 {
            let (a, b) = (dir(out[i - 2].0, out[i].0), dir(out[i].0, out[i + 2].0));
            let ang = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            corner[i] = ((ang - 60.0) / 40.0).clamp(0.0, 1.0);
        }
        let reach = 8isize;
        let mut w = vec![0.0f32; m];
        for i in 0..m {
            for d in -reach..=reach {
                let j = i as isize + d;
                if j >= 0 && (j as usize) < m {
                    w[i] = w[i].max(
                        corner[j as usize] * (1.0 - d.unsigned_abs() as f32 / (reach as f32 + 1.0)),
                    );
                }
            }
        }
        if w.iter().any(|&x| x > 0.0) {
            for _ in 0..16 {
                let q = out.clone();
                for i in 1..m - 1 {
                    if w[i] > 0.0 {
                        let avg = [0, 1, 2].map(|k| 0.5 * (q[i - 1].0[k] + q[i + 1].0[k]));
                        out[i].0 = norm([0, 1, 2].map(|k| q[i].0[k] + (avg[k] - q[i].0[k]) * w[i]));
                    }
                }
            }
        }
    }
    out
}

/// A path's places softened (`passes` of the light low-pass, ends held) and drawn as a centripetal
/// Catmull-Rom curve (R17: alpha one half, which neither overshoots nor makes cusps where the steps
/// bunch, as a float's do where it stalls), each span cut into pieces of at most `piece` (radians on
/// the globe), so the curve stays a curve at any zoom. Returns each point, its value, and how far
/// along the places it is, 0..1.
pub fn spline(places: &[([f32; 3], f32)], passes: usize, piece: f32) -> Vec<([f32; 3], f32, f32)> {
    // the softening and the merging of near-duplicates, as `smooth` does them, with no subdivision
    let p: Vec<([f32; 3], f32)> = smooth_with(places, passes, 1, true)
        .into_iter()
        .map(|(q, m, _)| (q, m))
        .collect();
    let n = p.len();
    if n < 3 {
        return smooth_with(places, passes, 1, true);
    }
    let sub3 = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let len3 = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let lerp = |a: [f32; 3], b: [f32; 3], ta: f32, tb: f32, t: f32| {
        let w = if (tb - ta).abs() < 1e-12 {
            0.0
        } else {
            (t - ta) / (tb - ta)
        };
        [
            a[0] + (b[0] - a[0]) * w,
            a[1] + (b[1] - a[1]) * w,
            a[2] + (b[2] - a[2]) * w,
        ]
    };
    let last = (n - 1) as f32;
    let mut out = Vec::new();
    for i in 0..n - 1 {
        let p1 = p[i].0;
        let p2 = p[i + 1].0;
        // (past an end, the end reflected: the curve leaves and arrives straight)
        let p0 = if i > 0 {
            p[i - 1].0
        } else {
            sub3([2.0 * p1[0], 2.0 * p1[1], 2.0 * p1[2]], p2)
        };
        let p3 = if i + 2 < n {
            p[i + 2].0
        } else {
            sub3([2.0 * p2[0], 2.0 * p2[1], 2.0 * p2[2]], p1)
        };
        let knot = |a: [f32; 3], b: [f32; 3]| len3(sub3(b, a)).max(1e-9).sqrt();
        let t0 = 0.0;
        let t1 = t0 + knot(p0, p1);
        let t2 = t1 + knot(p1, p2);
        let t3 = t2 + knot(p2, p3);
        let pieces = ((len3(sub3(p2, p1)) / piece.max(1e-9)).ceil() as usize).clamp(1, 48);
        for j in 0..pieces {
            let u = j as f32 / pieces as f32;
            let t = t1 + (t2 - t1) * u;
            let a1 = lerp(p0, p1, t0, t1, t);
            let a2 = lerp(p1, p2, t1, t2, t);
            let a3 = lerp(p2, p3, t2, t3, t);
            let b1 = lerp(a1, a2, t0, t2, t);
            let b2 = lerp(a2, a3, t1, t3, t);
            let c = lerp(b1, b2, t1, t2, t);
            out.push((
                norm(c),
                p[i].1 + (p[i + 1].1 - p[i].1) * u,
                (i as f32 + u) / last,
            ));
        }
    }
    out.push((p[n - 1].0, p[n - 1].1, 1.0));
    out
}

/// A thing on the wind's line (R21): its whole flight since it was lifted, a centripetal Catmull-Rom
/// through its true steps (no softening: its kinks are the wind's, 0.7 sigma a step) to its head on
/// the same curve; long and light, a wind line not a wake: thin, widest at the head, fading along its
/// length but never to nothing within its short life. Drawn by the airy strokes (`Wake::airy`). `fade`
/// dims it (a seed come down lets its line go quiet).
#[allow(clippy::too_many_arguments)] // each argument is one thing the call draws or reads
pub fn air_line(
    out: &mut Vec<StrokeInstance>,
    path: &[super::LookBead],
    head: crate::model::LonLat,
    night: bool,
    hue: [f32; 3],
    fade: f32,
    heavy: f32,
    rad_per_pt: f32,
) {
    let mut places: Vec<([f32; 3], f32)> = path.iter().map(|b| (super::unit(b.at), 1.0)).collect();
    places.push((super::unit(head), 1.0));
    let pts: Vec<Pt> = spline(&places, 0, 2.0 * rad_per_pt)
        .into_iter()
        .map(|(p, _, f)| {
            let age = 1.0 - f;
            let (w, a) = (2.0 - 1.2 * age, 0.85 * (1.0 - 0.75 * age.powf(0.8)));
            Pt {
                p,
                w: w * heavy.sqrt(),
                a: a * fade,
                hollow: 0.0,
            }
        })
        .collect();
    line(out, &pts, hue, false, night, false);
}
