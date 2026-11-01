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
    pub _pad: [f32; 3],
}

pub struct Wake {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<StrokeInstance>,
    /// the edges' draw, then the lines'
    uniforms: [Uniform<WakeUniform>; 2],
}

impl Wake {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("wake", &marks_wgsl(include_str!("wake.wgsl")));
        let uniforms = [Uniform::new(gpu, "wake edges"), Uniform::new(gpu, "wake lines")];
        for (i, u) in uniforms.iter().enumerate() {
            u.write(gpu, &WakeUniform { layer: i as f32, _pad: [0.0; 3] });
        }
        Self { pipeline: mark_pipeline(gpu, "wake", &module, LAYOUT, view, Some(&uniforms[0].layout)), instances: Instances::new(gpu, "wake", 1024), uniforms }
    }

    pub fn prepare(&mut self, gpu: &Gpu, segments: &[StrokeInstance]) {
        self.instances.write(gpu, segments);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
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
pub fn line(out: &mut Vec<StrokeInstance>, pts: &[Pt], hue: [f32; 3], screen: bool, night: bool, closed: bool) {
    let s = screen as u8 as f32;
    let nf = night as u8 as f32;
    let h4 = [hue[0], hue[1], hue[2], s];
    let seg = |a: &Pt, b: &Pt, prev: [f32; 3], next: [f32; 3], cap_a: bool, cap_b: bool| StrokeInstance {
        prev: [prev[0], prev[1], prev[2], a.w],
        a: [a.p[0], a.p[1], a.p[2], a.a],
        b: [b.p[0], b.p[1], b.p[2], b.w],
        next: [next[0], next[1], next[2], b.a],
        hue: h4,
        flags: [cap_a as u8 as f32, cap_b as u8 as f32, nf, 0.5 * (a.hollow + b.hollow)],
    };
    match pts.len() {
        0 => {}
        1 => {
            let a = pts[0];
            let mut b = a;
            // (a dot: a segment too short to see, round at both ends)
            if screen { b.p[0] += 0.01 } else { b.p[0] += 1e-6 }
            out.push(seg(&a, &b, a.p, b.p, true, true));
        }
        n => {
            let segs = if closed { n } else { n - 1 };
            for i in 0..segs {
                let (a, b) = (&pts[i], &pts[(i + 1) % n]);
                let prev = if i > 0 { pts[i - 1].p } else if closed { pts[n - 1].p } else { a.p };
                let next = if i + 2 < n { pts[i + 2].p } else if closed { pts[(i + 2) % n].p } else { b.p };
                out.push(seg(a, b, prev, next, !closed && i == 0, !closed && i + 1 == segs));
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
    smooth_with(places, passes, sub, false).into_iter().map(|(p, _, f)| (p, f)).collect()
}

fn smooth_with(places: &[([f32; 3], f32)], passes: usize, sub: usize, unit: bool) -> Vec<([f32; 3], f32, f32)> {
    let norm = |v: [f32; 3]| if unit { norm(v) } else { v };
    // (points closer than a few hundred metres are one: a float held at a coast for days)
    let mut p: Vec<([f32; 3], f32)> = Vec::with_capacity(places.len());
    for &q in places {
        if let Some(l) = p.last() {
            let d = ((l.0[0] - q.0[0]).powi(2) + (l.0[1] - q.0[1]).powi(2) + (l.0[2] - q.0[2]).powi(2)).sqrt();
            if d < if unit { 5e-5 } else { 0.03 } {
                continue;
            }
        }
        p.push(q);
    }
    if p.len() < 3 {
        let n = p.len().saturating_sub(1).max(1) as f32;
        return p.iter().enumerate().map(|(i, q)| (q.0, q.1, i as f32 / n)).collect();
    }
    for _ in 0..passes {
        let q = p.clone();
        for i in 1..q.len() - 1 {
            let v = [0, 1, 2].map(|k| 0.25 * q[i - 1].0[k] + 0.5 * q[i].0[k] + 0.25 * q[i + 1].0[k]);
            p[i] = (norm(v), 0.25 * q[i - 1].1 + 0.5 * q[i].1 + 0.25 * q[i + 1].1);
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
            let v = [0, 1, 2].map(|k| 0.5 * (2.0 * p1[k] + (-p0[k] + p2[k]) * t + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2 + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3));
            out.push((norm(v), p[i].1 + (p[i + 1].1 - p[i].1) * t, (i as f32 + t) / last));
        }
    }
    out.push((p[n - 1].0, p[n - 1].1, 1.0));
    out
}

/// A lamp's wake as one line: its daily places oldest first and where it is now, a ribbon that is
/// widest and fullest at the lamp and tapers and fades with age to nothing at its oldest day. A lamp
/// chosen as itself (`follow`, R8) carries its whole journey, fuller and never fading below a third,
/// so it reads as a line of its colour at any zoom. `fade` dims the whole line (0..1).
pub fn wake(out: &mut Vec<StrokeInstance>, path: &[super::LookBead], now: crate::model::LonLat, night: bool, hue: [f32; 3], follow: bool, fade: f32) {
    let mut places: Vec<([f32; 3], f32)> = path.iter().map(|b| (super::unit(b.at), b.measured)).collect();
    places.push((super::unit(now), path.last().map_or(1.0, |b| b.measured)));
    let line_pts: Vec<Pt> = smooth(&places, 1, 2)
        .into_iter()
        .map(|(p, measured, f)| {
            let age = 1.0 - f;
            let (w, a) = if follow {
                (3.0 - 1.5 * age, 1.0 - 0.62 * age.powf(1.5))
            } else {
                (3.2 - 2.4 * age, 0.92 * (1.0 - age).powf(1.3))
            };
            Pt { p, w, a: a * fade, hollow: if measured < 0.5 { 1.0 } else { 0.0 } }
        })
        .collect();
    line(out, &line_pts, hue, false, night, false);
}
