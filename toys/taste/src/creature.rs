//! The creature, drawn: one instanced quad a creature (the one in front, and the small ones on the map
//! of where it has been), every pixel worked out from its eight numbers in the fragment shader.
//!
//! The eight traits (`body`, from walk.rs's four numbers): 0 colour (round the wheel), 1 spikes (a smooth round body to a
//! sharp star), 2 arms (three to nine, the count blended between whole numbers), 3 curl (arms swept
//! one way, straight, or swept the other), 4 markings (plain, veins along the arms, rings, spots), 5
//! the second colour (a near neighbour of the first, round to its opposite), 6 glow (matte and
//! shaded, to a lit jelly with a bright rim), 7 temper (slow breathing to a quick shimmer).

use bytemuck::{Pod, Zeroable};
use toy_kit::Gpu;
use wgpu::util::DeviceExt;

/// One creature on screen: its centre and radius in pixels, its opacity, its form, and its life: a
/// clock offset, how much it is touched now (0..1), and where it looks (a unit-ish offset).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct Instance {
    pub c: [f32; 2],
    pub r: f32,
    pub alpha: f32,
    pub f0: [f32; 4],
    pub f1: [f32; 4],
    pub life: [f32; 4],
}

/// The creature's eight traits from the walk's four (colour, spikes, arms, glow): the other four follow
/// smoothly, so a small step is a small change and every corner of the four is a different creature.
pub fn body(f: &crate::walk::Form) -> [f64; 8] {
    let tau = std::f64::consts::TAU;
    let (h, s, a, g) = (f[0], f[1], f[2], f[3]);
    let curl = 0.5 + 0.48 * (tau * (h + 0.6 * a) + 1.0).sin() * (0.3 + 0.7 * s);
    let marks = 0.5 + 0.5 * (tau * (0.7 * s + 0.9 * g + 0.4 * h)).sin();
    let second = 0.5 + 0.5 * (tau * (h * 2.0 + 0.5 * a)).cos();
    let temper = 0.5 + 0.45 * (tau * (0.6 * s - 0.5 * g + 0.25)).sin();
    [h, s, a, curl, marks, second, g, temper]
}

impl Instance {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        c: [f32; 2],
        r: f32,
        alpha: f32,
        form: &crate::walk::Form,
        clock: f32,
        touch: f32,
        look: [f32; 2],
    ) -> Instance {
        let b = body(form);
        let f = |i: usize| b[i] as f32;
        Instance {
            c,
            r,
            alpha,
            f0: [f(0), f(1), f(2), f(3)],
            f1: [f(4), f(5), f(6), f(7)],
            life: [clock, touch, look[0], look[1]],
        }
    }
}

const WGSL: &str = r#"
struct U { size: vec2<f32>, time: f32, still: f32 };
@group(0) @binding(0) var<uniform> u: U;
struct I { @location(0) c: vec2<f32>, @location(1) ra: vec2<f32>, @location(2) f0: vec4<f32>, @location(3) f1: vec4<f32>, @location(4) life: vec4<f32> };
struct O { @builtin(position) pos: vec4<f32>, @location(0) q: vec2<f32>, @location(1) ra: vec2<f32>, @location(2) f0: vec4<f32>, @location(3) f1: vec4<f32>, @location(4) life: vec4<f32> };

const REACH: f32 = 1.75;
const TAU: f32 = 6.2831853;

@vertex fn vs_main(@builtin(vertex_index) vi: u32, i: I) -> O {
  let corner = array<vec2<f32>, 6>(vec2(-1.0,-1.0), vec2(1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,1.0))[vi];
  let p = i.c + corner * i.ra.x * REACH;
  var o: O;
  o.pos = vec4<f32>(p.x / u.size.x * 2.0 - 1.0, 1.0 - p.y / u.size.y * 2.0, 0.0, 1.0);
  o.q = corner * REACH; o.ra = i.ra; o.f0 = i.f0; o.f1 = i.f1; o.life = i.life;
  return o;
}

fn hsv(h: f32, s: f32, v: f32) -> vec3<f32> {
  let k = vec3<f32>(1.0, 2.0 / 3.0, 1.0 / 3.0);
  let p = abs(fract(vec3<f32>(h) + k) * 6.0 - vec3<f32>(3.0));
  return v * mix(vec3<f32>(1.0), clamp(p - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0)), s);
}

// the arms' profile round the body: 1 on an arm's line, falling toward 0 between, sharper with spikes
fn arms(n: f32, a: f32, sharp: f32) -> f32 {
  let c = 0.5 + 0.5 * cos(n * a);
  return pow(c, sharp);
}

fn bump(x: f32, c: f32, w: f32) -> f32 { return max(0.0, 1.0 - abs(x - c) / w); }

@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  let q = i.q;                       // the creature's own units: its radius is 1, y down
  let rpx = i.ra.x;
  let hue = i.f0.x; let spikes = i.f0.y; let nArms = i.f0.z; let curl = i.f0.w;
  let marks = i.f1.x; let second = i.f1.y; let glow = i.f1.z; let temper = i.f1.w;
  let tm = u.time * (1.0 - u.still) + i.life.x;
  let touch = i.life.y;
  let look = i.life.zw;

  let rho = length(q);
  let th = atan2(-q.y, q.x);
  let n = 3.0 + nArms * 6.0;
  let n0 = floor(n);
  let fr = smoothstep(0.0, 1.0, n - n0);
  let cu = (curl * 2.0 - 1.0) * 2.4;
  let speed = mix(0.35, 2.6, temper);
  let spin = tm * mix(0.03, 0.22, temper);
  let tw = th + cu * min(rho, 1.3) + spin;
  let sharp = mix(1.0, 9.0, spikes * spikes);
  let A = mix(0.05, 0.6, spikes);
  let w = mix(arms(n0, tw, sharp), arms(n0 + 1.0, tw, sharp), fr);
  var edge = 1.0 - A + A * w;
  // breathing, and a wave running round the rim (quicker with temper)
  edge = edge * (1.0 + mix(0.010, 0.045, temper) * sin(3.0 * th - tm * speed * 1.7) + 0.022 * sin(tm * speed * 0.8));
  edge = edge * (0.96 + 0.05 * touch);
  let d = rho / max(edge, 0.05);
  let aa = 1.4 / (rpx * max(edge, 0.2));
  let body = 1.0 - smoothstep(1.0 - aa, 1.0 + aa, d);

  // colour: the second colour at the heart, the first toward the rim
  let base = hsv(hue, mix(0.64, 0.78, glow), mix(0.84, 0.97, glow));
  let accent = hsv(hue + mix(0.07, 0.5, second), mix(0.55, 0.72, glow), 0.97);
  var col = mix(accent, base, smoothstep(0.0, 0.8, d));

  // markings
  let nr = n0 + step(0.5, fr);
  let vein = pow(0.5 + 0.5 * cos(nr * tw), 46.0);
  let veins = vein * smoothstep(0.12, 0.35, d) * (1.0 - smoothstep(0.8, 0.97, d));
  let rings = smoothstep(0.45, 0.85, 0.5 + 0.5 * cos(d * 3.14159 * 7.0 - tm * speed * 0.6)) * smoothstep(0.12, 0.3, d) * (1.0 - smoothstep(0.88, 0.99, d));
  let aArm = nr * tw;
  let k = round(aArm / TAU);
  let ang = (aArm - k * TAU) / nr;
  var spot = 0.0;
  for (var j = 0; j < 3; j = j + 1) {
    let di = 0.3 + 0.24 * f32(j);
    let rr = 0.075 * (1.15 - 0.35 * f32(j)) * (0.6 + 0.4 * A * 2.0);
    let dd = length(vec2<f32>(ang * rho, rho - di * edge));
    spot = max(spot, 1.0 - smoothstep(rr - aa, rr + aa, dd));
  }
  let wv = bump(marks, 0.36, 0.26);
  let wr = bump(marks, 0.66, 0.24);
  let ws = smoothstep(0.76, 0.94, marks);
  col = mix(col, mix(accent, vec3<f32>(1.0), 0.55), veins * wv * 0.85);
  col = mix(col, base * 0.5, rings * wr * 0.75);
  col = mix(col, mix(accent, vec3<f32>(0.04, 0.03, 0.06), 0.62), spot * ws);

  // the heart: a soft light that breathes and leans toward the finger
  let hd = length(q - look * 0.16 * d) / 0.2;
  let heart = exp(-hd * hd * 1.6) * (0.75 + 0.25 * sin(tm * speed * 1.1));
  col = mix(col, mix(accent, vec3<f32>(1.0, 0.98, 0.94), 0.6), heart * mix(0.35, 0.7, glow));

  // matte: shaded from the upper left, darker at the rim; jelly: see-through heart, a bright rim
  let lit = (0.74 + 0.36 * clamp(-q.y * 0.45 - q.x * 0.25 + 0.35, 0.0, 1.0)) * mix(1.0, 0.8, smoothstep(0.72, 1.0, d));
  let rim = smoothstep(0.7, 1.0, d);
  let matte = col * lit;
  let jelly = col * 1.04 + base * rim * 0.5;
  col = mix(matte, jelly, glow) * (1.0 + 0.16 * touch);
  let inner = mix(1.0, mix(0.62, 1.0, smoothstep(0.25, 1.0, d)), glow);

  // outside: a glow (more for the jelly), a soft shadow under the matte
  let beyond = max(d - 1.0, 0.0);
  let halo = exp(-beyond * mix(9.0, 3.6, glow)) * (0.12 + 0.42 * glow + 0.18 * touch);
  let shade = exp(-max(length(q - vec2<f32>(0.05, 0.10)) / max(edge, 0.05) - 0.98, 0.0) * 5.0) * 0.38 * (1.0 - glow);
  let a = i.ra.y;
  let fill = body * inner;
  let rgb = col * fill + (1.0 - body) * base * halo;
  let alpha = fill + (1.0 - body) * shade;
  return vec4<f32>(rgb, alpha) * a;
}
"#;

pub struct Creatures {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    pub list: Vec<Instance>,
}

impl Creatures {
    pub fn new(gpu: &Gpu) -> Creatures {
        let dev = &gpu.device;
        let shader = gpu.shader("taste creature", WGSL);
        let uniform = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("creature u"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("creature"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("creature"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("creature"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4];
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("creature"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attrs,
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format,
                    blend: Some(Gpu::over()),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        Creatures {
            pipeline,
            bind,
            uniform,
            list: Vec::new(),
        }
    }

    /// Draws (and forgets) the creatures listed, in order (the last on top).
    pub fn render(
        &mut self,
        gpu: &Gpu,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: [u32; 2],
        time: f32,
        still: bool,
    ) {
        if self.list.is_empty() {
            return;
        }
        gpu.queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::cast_slice(&[
                size[0] as f32,
                size[1] as f32,
                time,
                if still { 1.0 } else { 0.0 },
            ]),
        );
        let buf = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("creatures"),
                contents: bytemuck::cast_slice(&self.list),
                usage: wgpu::BufferUsages::VERTEX,
            });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("creatures"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..6, 0..self.list.len() as u32);
        }
        self.list.clear();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn laid_out_for_the_shader() {
        assert_eq!(std::mem::size_of::<super::Instance>(), 64);
    }
}
