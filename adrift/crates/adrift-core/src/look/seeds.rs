//! The things on the wind (R21, `seeds.wgsl`): a down seed, its pappus seen from above, carried by
//! the air over its own shadow on the ground, and its quiet ending when it comes down (settled on
//! land, a ring on the sea). A lamp is glass on the water; a seed is gossamer in the air. Its line is
//! the airy stroke (`wake::air_line`); its head glides on the same spline as its line (`spline_head`).
use super::{mark_pipeline, marks_wgsl, Instances};
use crate::gpu::Gpu;
use crate::model::LonLat;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SeedInstance {
    pub g: [f32; 3],
    /// 0..1 while a finger rests and the ring gathers into a seed; 1 a seed
    pub beat: f32,
    /// its hue (`look::lamp_hue`, as the lamps')
    pub hue: [f32; 3],
    /// 1 by night
    pub night: f32,
    /// how high it rides, 0..1: its shadow's offset on the ground (1 aloft, 0 down)
    pub lift: f32,
    /// 0 flying, 1 come down on land, 2 come down on the sea
    pub down: f32,
    /// 0..1 through its ending after it settles: the puff fades to a mark (and a ring spreads on the sea)
    pub after: f32,
    /// its own phase (by id), so no two turn together
    pub spin: f32,
    /// 1 while it is what you just did, fading in time
    pub lit: f32,
    /// 1 while held, chosen or followed
    pub held: f32,
    /// 1 in a clip: the line's head, a size up with a soft glow
    pub head: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SeedInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32, 4 => Float32, 5 => Float32, 6 => Float32, 7 => Float32, 8 => Float32, 9 => Float32, 10 => Float32, 11 => Float32],
};

pub struct Seeds {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<SeedInstance>,
}

impl Seeds {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("seeds", &marks_wgsl(include_str!("seeds.wgsl")));
        Self { pipeline: mark_pipeline(gpu, "seeds", &module, LAYOUT, view, None), instances: Instances::new(gpu, "seeds", 16) }
    }

    pub fn prepare(&mut self, gpu: &Gpu, seeds: &[SeedInstance]) {
        self.instances.write(gpu, seeds);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}

/// The days a seed takes to settle once it is down (its shadow meets it, the pappus folds a little),
/// and then to fade to its mark: a small quiet ending, timed in the date's days so a scrub back
/// lifts it again.
pub const SETTLE_DAYS: f32 = 0.12;
pub const FADE_DAYS: f32 = 0.5;

/// Where a thing on the wind is drawn between its steps (R21): on the centripetal Catmull-Rom through
/// the steps either side (`p` the step before the one before now, the one before now, the next, the
/// one after: ends repeated where there are none), `fr` of the way from `p[1]` to `p[2]`. The same
/// curve its line is drawn on, so the head glides along its line and never cuts a corner of it.
pub fn spline_head(p: [LonLat; 4], fr: f64) -> LonLat {
    let v: Vec<[f64; 3]> = p.iter().map(|q| q.to_xyz()).collect();
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let len = |a: [f64; 3]| (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    let (mut p0, p1, p2, mut p3) = (v[0], v[1], v[2], v[3]);
    // (a missing neighbour, the end reflected, as `wake::spline` does)
    if len(sub(p0, p1)) < 1e-12 {
        p0 = sub([2.0 * p1[0], 2.0 * p1[1], 2.0 * p1[2]], p2);
    }
    if len(sub(p3, p2)) < 1e-12 {
        p3 = sub([2.0 * p2[0], 2.0 * p2[1], 2.0 * p2[2]], p1);
    }
    let knot = |a: [f64; 3], b: [f64; 3]| len(sub(b, a)).max(1e-12).sqrt();
    let t1 = knot(p0, p1);
    let t2 = t1 + knot(p1, p2);
    let t3 = t2 + knot(p2, p3);
    let lerp = |a: [f64; 3], b: [f64; 3], ta: f64, tb: f64, t: f64| {
        let w = if (tb - ta).abs() < 1e-15 { 0.0 } else { (t - ta) / (tb - ta) };
        [a[0] + (b[0] - a[0]) * w, a[1] + (b[1] - a[1]) * w, a[2] + (b[2] - a[2]) * w]
    };
    let t = t1 + (t2 - t1) * fr.clamp(0.0, 1.0);
    let a1 = lerp(p0, p1, 0.0, t1, t);
    let a2 = lerp(p1, p2, t1, t2, t);
    let a3 = lerp(p2, p3, t2, t3, t);
    let b1 = lerp(a1, a2, 0.0, t2, t);
    let b2 = lerp(a2, a3, t1, t3, t);
    let c = lerp(b1, b2, t1, t2, t);
    let l = len(c).max(1e-12);
    LonLat::from_xyz([c[0] / l, c[1] / l, c[2] / l])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spline_head_passes_through_the_steps() {
        let p = [LonLat { lon: -80.0, lat: 38.0 }, LonLat { lon: -79.0, lat: 38.5 }, LonLat { lon: -77.5, lat: 38.2 }, LonLat { lon: -76.0, lat: 39.0 }];
        for (fr, want) in [(0.0, p[1]), (1.0, p[2])] {
            let h = spline_head(p, fr);
            assert!(h.distance_m(want) < 1.0, "{fr}: {h:?}");
        }
        // between, near the chord and not past either end
        let m = spline_head(p, 0.5);
        assert!(m.distance_m(p[1]) < p[1].distance_m(p[2]) && m.distance_m(p[2]) < p[1].distance_m(p[2]));
    }
}
