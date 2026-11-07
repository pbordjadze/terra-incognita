//! Night Walk (a working title; painting-company row 224): a walker crosses a land at night that no one
//! can see. Each of its tries is one rule (`land.rs`): pick a spot within its stride, go there if it is
//! higher, sometimes if it is lower, otherwise stay. Every try leaves a footprint, and where they pile up
//! thickest the map in the dark rises: its height is the log of the pile, so it is a measurement of the
//! land, not a picture of it. At dawn the real land comes up under the map. The one control is the
//! stride: a touch sets how far each step may go (the ring round the walker reaches the finger).

pub mod land;
mod sound;

use land::{Land, Try, Walker, GW, K};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// Tries a second.
pub const PACE: f64 = 120.0;
/// A night's walk, the dawn coming up, the day held, the dusk (seconds).
const NIGHT: f64 = 42.0;
/// The first night, while untouched, is shorter: its dawn comes soon.
const FIRST_NIGHT: f64 = 27.0;
/// A night lasts at least this long after the stride last changed (so a new stride has its time), and
/// never longer than LONGEST.
const AFTER: f64 = 30.0;
const LONGEST: f64 = 80.0;
const RISE: f64 = 6.0;
const DAY: f64 = 13.0;
const DUSK: f64 = 3.0;
/// The stride's bounds and its first value (screen widths).
const MIN_STRIDE: f64 = 0.006;
const FIRST_STRIDE: f64 = 0.05;
/// The map's contours, every this many of its units (a contour step is a factor of e^0.7 ≈ 2 in footprints).
const CONTOUR: f32 = 0.7;
const BUMPS: usize = 40;

pub struct Walk {
    land: Land,
    walker: Walker,
    night: u64,
    /// When this night began (the shell's clock).
    began: f64,
    /// This night's length (NIGHT, longer if the stride changed late).
    length: f64,
    t: f64,
    acc: f64,
    points: [f64; 2],
    /// The lantern as drawn (it glides after the walker's real place).
    lamp: [f64; 2],
    /// The latest footprints (where, when, and whether it was a step or a stay), and the spots it
    /// looked at and didn't go to (where, when).
    trail: std::collections::VecDeque<([f64; 2], f64, bool)>,
    glances: std::collections::VecDeque<([f64; 2], f64)>,
    holding: bool,
    /// The walk's pace (1 at full; slow while a finger sets the stride, so single tries can be seen).
    pace: f64,
    /// Where the lantern was when the finger came down: the stride is measured from there while it stays.
    anchor: [f64; 2],
    touched: bool,
    still: bool,
    ground: Ground,
    painter: Painter,
    sound: sound::Night,
    stride_seen: f64,
}

struct Ground {
    pipe: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    buf: wgpu::Buffer,
    tex: wgpu::Texture,
    gh: usize,
    half: Vec<u16>,
}

fn half(f: f32) -> u16 {
    let b = f.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp <= 0 {
        return sign;
    }
    if exp >= 31 {
        return sign | 0x7bff;
    }
    let m = (man + 0x1000) >> 13;
    if m >= 0x400 {
        return sign | (((exp + 1).min(30) as u16) << 10);
    }
    sign | ((exp as u16) << 10) | m as u16
}

impl Ground {
    fn new(ctx: &Ctx, gh: usize) -> Ground {
        let dev = &ctx.gpu.device;
        let tex = dev.create_texture(&wgpu::TextureDescriptor {
            label: Some("walk prints"),
            size: wgpu::Extent3d {
                width: GW as u32,
                height: gh as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("walk ground"),
            size: (16 * (3 + BUMPS)) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("walk ground"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("walk ground"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let src = GROUND_WGSL
            .replace("BUMPS", &BUMPS.to_string())
            .replace("CONTOUR", &format!("{CONTOUR:.3}"));
        let shader = ctx.gpu.shader("walk ground", &src);
        let pipe = ctx
            .gpu
            .fullscreen_pipeline("walk ground", &shader, &[&bgl], None);
        Ground {
            pipe,
            bind,
            buf,
            tex,
            gh,
            half: vec![0; GW * gh],
        }
    }
}

impl Walk {
    fn begin_night(&mut self, ctx: &Ctx, t: f64) {
        self.night += 1;
        let aspect = self.points[1] / self.points[0].max(1.0);
        self.land = Land::new(0x5eed_0000 + self.night, aspect);
        self.walker = Walker::new(&self.land, 0xf007 + self.night * 31, self.walker.stride);
        self.began = t;
        self.length = NIGHT;
        self.lamp = self.walker.at;
        self.trail.clear();
        self.glances.clear();
        if self.ground.gh != self.walker.gh {
            self.ground = Ground::new(ctx, self.walker.gh);
        }
    }

    /// Seconds into this night's cycle.
    fn clock(&self) -> f64 {
        self.t - self.began
    }

    /// How far the dawn has come (0 night, 1 day), and the light (0 at the dusk's end).
    fn dawn(&self) -> (f64, f64) {
        let c = self.clock();
        let up = ((c - self.length) / RISE).clamp(0.0, 1.0);
        let up = up * up * (3.0 - 2.0 * up);
        let light = if c < 2.0 && self.night > 1 {
            c / 2.0
        } else {
            1.0 - ((c - self.length - DAY) / DUSK).clamp(0.0, 1.0)
        };
        (up, light)
    }

    fn walking(&self) -> bool {
        self.clock() < self.length
    }

    /// A place on the land (screen widths) on screen (points), and back.
    fn pt(&self, p: [f64; 2]) -> [f64; 2] {
        [p[0] * self.points[0], p[1] * self.points[0]]
    }
    fn from_pt(&self, p: [f64; 2]) -> [f64; 2] {
        [p[0] / self.points[0], p[1] / self.points[0]]
    }

    fn reach_to(&mut self, p: [f64; 2]) {
        let q = self.from_pt(p);
        let d = (q[0] - self.anchor[0]).hypot(q[1] - self.anchor[1]);
        self.walker.stride = d.max(MIN_STRIDE);
        if self.walking() {
            self.length = self
                .length
                .max(NIGHT)
                .max(self.clock() + AFTER)
                .min(LONGEST);
        }
    }
}

impl Toy for Walk {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let points = ctx.points();
        let aspect = points[1] / points[0].max(1.0);
        let land = Land::new(0x5eed_0001, aspect);
        let mut walker = Walker::new(&land, 0xf007 + 31, FIRST_STRIDE);
        // the first night begins on the middle hill, the others above and below it
        let mut r = land::Rng::new(0xf1257);
        walker.at = land.start_on(land.middle_hill(), &mut r);
        walker.h = land.h(walker.at);
        let ground = Ground::new(ctx, walker.gh);
        let lamp = walker.at;
        Ok(Walk {
            land,
            walker,
            night: 1,
            began: 0.0,
            length: FIRST_NIGHT,
            t: 0.0,
            acc: 0.0,
            points,
            lamp,
            trail: Default::default(),
            glances: Default::default(),
            holding: false,
            pace: 1.0,
            anchor: [0.0, 0.0],
            touched: false,
            still: ctx.reduce_motion,
            ground,
            painter: Painter::new(&ctx.gpu),
            sound: sound::Night::new(),
            stride_seen: FIRST_STRIDE,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.points = ctx.points();
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                self.touched = true;
                self.holding = true;
                self.anchor = self.lamp;
                self.reach_to([e.x, e.y]);
                ctx.haptic();
            }
            Phase::Move => {
                if self.holding {
                    self.reach_to([e.x, e.y]);
                }
            }
            Phase::Up | Phase::Cancel => self.holding = false,
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        if self.clock() >= self.length + DAY + DUSK {
            self.begin_night(ctx, t);
        }
        let mut steps: Vec<(f32, f32)> = Vec::new();
        if self.walking() {
            let want = if self.holding { 0.1 } else { 1.0 };
            self.pace +=
                (want - self.pace) * (1.0 - (-dt / if self.holding { 0.15 } else { 0.6 }).exp());
            self.acc += dt * PACE * self.pace;
            let n = self.acc.floor() as usize;
            self.acc -= n as f64;
            for k in 0..n.min(40) {
                let at = t - dt * (1.0 - (k as f64 + 1.0) / n as f64);
                let Try { from, to, went } = self.walker.step(&self.land);
                if went {
                    let len = (to[0] - from[0]).hypot(to[1] - from[1]);
                    steps.push((len as f32, (to[0] * 2.0 - 1.0) as f32));
                } else {
                    self.glances.push_back((to, at));
                }
                self.trail.push_back((self.walker.at, at, went));
            }
            while self.trail.len() > 260 {
                self.trail.pop_front();
            }
            self.glances.retain(|g| t - g.1 < 0.45);
            while self.glances.len() > 90 {
                self.glances.pop_front();
            }
        }
        // the lantern glides after the walker
        let k = 1.0 - (-dt / 0.05).exp();
        let jump = (self.walker.at[0] - self.lamp[0]).hypot(self.walker.at[1] - self.lamp[1]);
        if jump > 0.6 {
            self.lamp = self.walker.at;
        } else {
            self.lamp[0] += (self.walker.at[0] - self.lamp[0]) * k;
            self.lamp[1] += (self.walker.at[1] - self.lamp[1]) * k;
        }
        self.stride_seen += (self.walker.stride - self.stride_seen) * (1.0 - (-dt / 0.08).exp());
        let (dawn, light) = self.dawn();
        self.sound
            .set(&steps, self.walker.stride as f32, dawn as f32, light as f32);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp;
        let size = [ctx.width, ctx.height];
        let (dawn, light) = self.dawn();
        // the footprints, to the map's texture
        for (h, &c) in self.ground.half.iter_mut().zip(&self.walker.prints) {
            *h = half(c);
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.ground.tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.ground.half),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(GW as u32 * 2),
                rows_per_image: Some(self.ground.gh as u32),
            },
            wgpu::Extent3d {
                width: GW as u32,
                height: self.ground.gh as u32,
                depth_or_array_layers: 1,
            },
        );
        // the map's height is log(footprints / (count · cell area)) less the land's highest log-chance
        // (placed by the pile's own deepest point: the map knows nothing of the land)
        let deepest = self.walker.prints.iter().fold(1e-6f32, |a, &b| a.max(b));
        let off = (deepest as f64).ln();
        let mut u: Vec<f32> = vec![
            size[0] as f32,
            size[1] as f32,
            self.t as f32,
            k as f32,
            dawn as f32,
            light as f32,
            off as f32,
            K as f32,
            self.land.top as f32,
            self.land.aspect as f32,
            self.land.bumps.len().min(BUMPS) as f32,
            self.ground.gh as f32,
        ];
        for i in 0..BUMPS {
            let b = self.land.bumps.get(i).copied().unwrap_or(land::Bump {
                c: [0.0, 0.0],
                sigma: 1.0,
                amp: 0.0,
            });
            u.extend_from_slice(&[b.c[0] as f32, b.c[1] as f32, b.sigma as f32, b.amp as f32]);
        }
        gpu.queue
            .write_buffer(&self.ground.buf, 0, bytemuck::cast_slice(&u));
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("walk ground"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.ground.pipe);
            rp.set_bind_group(0, &self.ground.bind, &[]);
            rp.draw(0..3, 0..1);
        }

        let w = self.points[0];
        let px = |p: [f64; 2]| [(p[0] * w * k) as f32, (p[1] * w * k) as f32];
        let pk = k as f32; // pixels a point
        let mut lamp = px(self.lamp);
        // the lantern flinches toward a spot it has just refused
        if let (false, Some(&(g, at))) = (self.still, self.glances.back()) {
            let a = (1.0 - (self.t - at) / 0.1).clamp(0.0, 1.0) as f32;
            let to = px(g);
            let (dx, dy) = (to[0] - lamp[0], to[1] - lamp[1]);
            let d = dx.hypot(dy).max(1.0);
            lamp = [
                lamp[0] + dx / d * 1.6 * pk * a,
                lamp[1] + dy / d * 1.6 * pk * a,
            ];
        }
        let night = (1.0 - dawn) as f32 * light as f32;
        let lit = light as f32;

        // the spots it looked at and didn't go to: a faint glance each, gone in a moment
        if !self.still {
            for &(p, at) in &self.glances {
                let a = (1.0 - (self.t - at) / 0.45).clamp(0.0, 1.0) as f32;
                // a try off the edge is shown at the edge, where it left the land
                let e = 0.006;
                let q = [p[0].clamp(e, 1.0 - e), p[1].clamp(e, self.land.aspect - e)];
                self.painter.disc(
                    px(q),
                    1.6 * pk,
                    1.0 * pk,
                    [0.80, 0.86, 1.0, 0.6 * a * a * night],
                );
            }
        }
        // the latest footprints: warm while fresh, cooling into the map
        let n = self.trail.len();
        for (i, &(p, at, _)) in self.trail.iter().enumerate() {
            let age = (self.t - at).max(0.0);
            let a = (1.0 - i as f32 / n.max(1) as f32).powf(0.0); // (kept even; the age fades them)
            let fade = (1.0 - age / 2.2).clamp(0.0, 1.0) as f32;
            let _ = a;
            self.painter.disc(
                px(p),
                1.1 * pk,
                0.8 * pk,
                [
                    1.0,
                    0.86,
                    0.62,
                    0.55 * fade * night + 0.25 * fade * (1.0 - night) * lit,
                ],
            );
        }
        // the stride: a ring round the walker that reaches as far as a step may go
        let r = (self.stride_seen * w * k) as f32;
        let since = self.clock();
        let invite = if self.touched || !self.walking() || (self.night == 1 && since < 2.5) {
            0.0
        } else {
            1.0
        };
        if invite > 0.0 {
            // every 3.2 s a faint ring swells from the stride's out toward a third of the screen and fades
            let ph = ((self.t - 2.5).max(0.0) % 3.2 / 2.2).min(1.0) as f32;
            let ease = 1.0 - (1.0 - ph) * (1.0 - ph);
            let grow = if self.still { 0.5 } else { ease };
            let rr = r + (0.30 * w * k) as f32 * grow;
            let a = if self.still {
                0.25
            } else {
                0.35 * (1.0 - ph) * (ph * 6.0).min(1.0)
            };
            self.painter
                .glow_ring(lamp, rr, 0.8 * pk, 1.2 * pk, [1.0, 0.90, 0.70, a * lit]);
        }
        let ring_a = (0.28 + 0.30 * self.holding as i32 as f32 + 0.1 * invite)
            * lit
            * (1.0 - 0.6 * dawn as f32);
        self.painter.glow_ring(
            lamp,
            r.max(2.5 * pk),
            0.7 * pk,
            0.9 * pk,
            [1.0, 0.90, 0.70, ring_a],
        );
        // a stride longer than the land: its edges glow faintly, where the far tries end
        let corner = [
            [0.0, 0.0],
            [1.0, 0.0],
            [0.0, self.land.aspect],
            [1.0, self.land.aspect],
        ]
        .iter()
        .map(|c: &[f64; 2]| (c[0] - self.lamp[0]).hypot(c[1] - self.lamp[1]))
        .fold(0.0, f64::max);
        let beyond = ((self.stride_seen - 0.5 * corner) / (0.5 * corner)).clamp(0.0, 1.0) as f32;
        if beyond > 0.0 {
            let (sw, sh) = (size[0] as f32, size[1] as f32);
            let e = 1.5 * pk;
            let c = [1.0, 0.88, 0.65, 0.22 * beyond * night];
            for (a, b) in [
                ([e, e], [sw - e, e]),
                ([sw - e, e], [sw - e, sh - e]),
                ([sw - e, sh - e], [e, sh - e]),
                ([e, sh - e], [e, e]),
            ] {
                self.painter.glow(a, b, 0.6 * pk, 10.0 * pk, c);
            }
        }
        // standing: the share of the latest tries refused, as a ring stamped round the lantern
        let recent = self.trail.iter().rev().take(36);
        let (mut stays, mut all) = (0.0f32, 0.0f32);
        for &(_, _, went) in recent {
            all += 1.0;
            if !went {
                stays += 1.0;
            }
        }
        let stay = if all > 0.0 { stays / all } else { 0.0 };
        if stay > 0.3 {
            let beat = if self.still {
                0.5
            } else {
                (self.t * 9.0).sin().abs() as f32
            };
            self.painter.glow_ring(
                lamp,
                (5.5 + 1.5 * beat) * pk,
                0.8 * pk,
                1.0 * pk,
                [1.0, 0.88, 0.65, 0.5 * (stay - 0.3) / 0.7 * night],
            );
        }
        // the lantern
        let sway = if self.still {
            0.0
        } else {
            (self.t * 7.3).sin() as f32 * 0.06
        };
        self.painter.glow(
            lamp,
            lamp,
            2.0 * pk,
            24.0 * pk,
            [1.0, 0.78, 0.45, (0.22 + sway * 0.3) * night + 0.05 * lit],
        );
        self.painter.glow(
            lamp,
            lamp,
            1.5 * pk,
            6.0 * pk,
            [1.0, 0.85, 0.6, 0.6 * night + 0.25 * lit],
        );
        self.painter.disc(
            lamp,
            2.4 * pk,
            0.8 * pk,
            [1.0, 0.94, 0.80, (0.95 * night + 0.6 * (1.0 - night)) * lit],
        );
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.still = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.sound.synth())
    }

    fn save(&self) -> Option<String> {
        Some(format!("{{\"stride\":{}}}", self.walker.stride))
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(saved) {
            if let Some(s) = v.get("stride").and_then(|s| s.as_f64()) {
                self.walker.stride = s.clamp(MIN_STRIDE, 4.0);
                self.stride_seen = self.walker.stride;
            }
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let w = self.points[0];
        let lamp = self.pt(self.lamp);
        let clampi = |p: [f64; 2]| {
            [
                p[0].clamp(8.0, self.points[0] - 8.0),
                p[1].clamp(8.0, self.points[1] - 8.0),
            ]
        };
        let centre = [self.points[0] * 0.5, self.points[1] * 0.5];
        // toward the screen's middle, a third of the width away: a stride that finds the other hills
        let (dx, dy) = (centre[0] - lamp[0], centre[1] - lamp[1]);
        let d = dx.hypot(dy).max(1.0);
        let good = clampi([lamp[0] + dx / d * 0.3 * w, lamp[1] + dy / d * 0.3 * w]);
        let near = clampi([lamp[0] + 0.012 * w, lamp[1]]);
        // the screen's corner farthest from the walker: a stride longer than the land
        let far = [
            [8.0, 8.0],
            [self.points[0] - 8.0, 8.0],
            [8.0, self.points[1] - 8.0],
            [self.points[0] - 8.0, self.points[1] - 8.0],
        ]
        .into_iter()
        .max_by(|a, b| {
            (a[0] - lamp[0])
                .hypot(a[1] - lamp[1])
                .total_cmp(&(b[0] - lamp[0]).hypot(b[1] - lamp[1]))
        })
        .unwrap();
        let mut a = vec![
            ("walker".to_string(), lamp),
            ("near".into(), near),
            ("good".into(), good),
            ("far".into(), far),
            ("centre".into(), centre),
        ];
        for (i, &h) in self.land.hills.iter().enumerate() {
            a.push((format!("hill{i}"), self.pt(h)));
        }
        a
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let w = self.points[0];
        let lamp = self.pt(self.lamp);
        let r = self.walker.stride * w;
        let centre = [self.points[0] * 0.5, self.points[1] * 0.5];
        let (dx, dy) = (centre[0] - lamp[0], centre[1] - lamp[1]);
        let d = dx.hypot(dy).max(1.0);
        let (ux, uy) = (dx / d, dy / d);
        let from = [lamp[0] + ux * (r + 10.0), lamp[1] + uy * (r + 10.0)];
        let to = [lamp[0] + ux * 0.28 * w, lamp[1] + uy * 0.28 * w];
        Some(toy_kit::info::Demo::new(
            "Touch near the walker for short steps, farther away for long ones.",
            vec![toy_kit::info::Stroke::drag(0.5, 1.8, from, to)],
        ))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "The walker picks a spot at random, no farther than its ring.",
                "Higher ground: it always goes. Lower: only sometimes, and more rarely the farther down it is.",
                "When it doesn't go, it stays put and leaves another footprint where it stands. Past the edge there is no ground: it never goes there.",
                "Where footprints pile up thickest, the map in the dark rises highest. At dawn, the land itself.",
            ],
            "Real: every step is that rule, on the land you see at dawn; the map in the dark is made from the footprints alone. Drawn: the land (a new one each night), the walker, the night.",
        ))
    }
}

/// The ground: the night, the footprints' map in moonlit ink, and at dawn the land itself under it.
/// `p0` (size px, time, ppp), `p1` (dawn, light, the map's offset, K), `p2` (the land's top, aspect,
/// bumps, the map's rows); `b` the land's bumps (centre, sigma, height).
const GROUND_WGSL: &str = r#"
struct U { p0: vec4<f32>, p1: vec4<f32>, p2: vec4<f32>, b: array<vec4<f32>, BUMPS> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var prints: texture_2d<f32>;
@group(0) @binding(2) var smp: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise2(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), w.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), w.x), w.y);
}
// the land's height and its slope at q (screen widths)
fn land(q: vec2<f32>) -> vec3<f32> {
  var h = 0.0; var g = vec2<f32>(0.0);
  let n = i32(u.p2.z);
  for (var i = 0; i < BUMPS; i = i + 1) {
    if (i >= n) { break; }
    let b = u.b[i];
    let d = q - b.xy;
    let e = b.w * exp(-dot(d, d) / (2.0 * b.z * b.z));
    h = h + e;
    g = g - e * d / (b.z * b.z);
  }
  return vec3<f32>(h, g);
}
// a contour line's strength where f crosses a whole number (f's own screen rate given)
fn contour(f: f32, w: f32, px: f32) -> f32 {
  let d = abs(fract(f + 0.5) - 0.5) / max(w, 1e-5);
  return (1.0 - smoothstep(0.35 * px, 0.35 * px + 1.0, d)) * (1.0 - smoothstep(0.25, 0.6, w));
}
fn relief(g: vec2<f32>, k: f32) -> f32 {
  let n = normalize(vec3<f32>(-g * k, 1.0));
  return clamp(dot(n, normalize(vec3<f32>(-0.55, -0.65, 0.55))), 0.0, 1.0);
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let size = u.p0.xy;
  let px = i.uv * size;
  let q = px / size.x;
  let ppp = u.p0.w;
  let dawn = u.p1.x;
  let light = u.p1.y;
  // the night's ground: unseen, a faint grain
  let grain = noise2(px / (2.5 * ppp)) * 0.5 + noise2(q * 40.0) * 0.5;
  var night = vec3<f32>(0.030, 0.038, 0.066) * (0.85 + 0.3 * grain);
  // the footprints' map: log of the pile, against the land's top
  let c = textureSample(prints, smp, vec2<f32>(q.x, q.y * 144.0 / u.p2.w));
  let pile = c.r;
  let L = log(max(pile, 1e-6)) - u.p1.z;
  let seen = smoothstep(0.02, 0.12, pile);
  // the map's slope from its neighbours a cell away (smooth, unlike the screen's 2x2 derivatives)
  let tw = 1.0 / 144.0;
  let tv = vec2<f32>(q.x, q.y * 144.0 / u.p2.w);
  let th = 1.0 / u.p2.w;
  let lx = log(max(textureSample(prints, smp, tv + vec2<f32>(tw, 0.0)).r, 1e-6)) - log(max(textureSample(prints, smp, tv - vec2<f32>(tw, 0.0)).r, 1e-6));
  let ly = log(max(textureSample(prints, smp, tv + vec2<f32>(0.0, th)).r, 1e-6)) - log(max(textureSample(prints, smp, tv - vec2<f32>(0.0, th)).r, 1e-6));
  let gL = vec2<f32>(lx, ly) / (2.0 * tw);
  let t = clamp(1.0 + L / 6.5, 0.0, 1.0);
  let shade = relief(gL, 0.022);
  var ink = mix(vec3<f32>(0.07, 0.09, 0.17), vec3<f32>(0.62, 0.68, 0.82), t * t) * (0.55 + 0.65 * shade);
  let fL = L / CONTOUR;
  let wL = fwidth(fL);
  let thick = smoothstep(0.15, 0.5, pile);
  let lineN = contour(fL, wL, ppp) * thick;
  ink = mix(ink, vec3<f32>(0.80, 0.85, 0.95), 0.45 * lineN);
  let nightMap = mix(night, ink, seen * (0.45 + 0.55 * thick));
  // the land itself, by day
  let hg = land(q);
  let Lt = u.p1.w * (hg.x - u.p2.x);
  let tt = clamp(1.0 + Lt / 6.5, 0.0, 1.0);
  let low = vec3<f32>(0.40, 0.50, 0.36);
  let mid = vec3<f32>(0.70, 0.63, 0.43);
  let high = vec3<f32>(0.94, 0.90, 0.80);
  var day = mix(low, mid, smoothstep(0.0, 0.6, tt));
  day = mix(day, high, smoothstep(0.55, 1.0, tt));
  day = day * (0.45 + 0.75 * relief(u.p1.w * hg.yz, 0.05));
  day = day * (0.96 + 0.08 * grain);
  // by day both sets of contours every second level, at the same heights, so they can be compared
  // the night's own map, coloured as land: the first light shows what the walker found
  let tm = clamp(1.0 + L / 6.5, 0.0, 1.0);
  var mapDay = mix(low, mid, smoothstep(0.0, 0.6, tm));
  mapDay = mix(mapDay, high, smoothstep(0.55, 1.0, tm));
  mapDay = mapDay * (0.45 + 0.75 * relief(gL, 0.03));
  mapDay = mix(low * 0.55, mapDay, seen * (0.45 + 0.55 * thick));
  // only the upper contours are compared, and only where footprints are thick
  let fT = Lt / (2.0 * CONTOUR);
  let lineT = contour(fT, fwidth(fT), 1.6 * ppp) * smoothstep(-4.8, -4.0, Lt);
  day = mix(day, vec3<f32>(0.36, 0.24, 0.12), 0.75 * lineT);
  let fD = L / (2.0 * CONTOUR);
  let lineD = contour(fD, fwidth(fD), 1.6 * ppp) * smoothstep(0.3, 0.8, pile) * smoothstep(-4.8, -4.0, L);
  // the night's map stays on the day as ink: its contours, and a cool wash where it walked
  day = mix(day, day * vec3<f32>(0.86, 0.90, 1.0), 0.30 * seen * thick);
  day = mix(day, vec3<f32>(0.10, 0.12, 0.30), 0.75 * lineD);
  // the dawn's light from the top of the screen
  let uv = i.uv;
  day = day * (1.0 + 0.10 * (1.0 - uv.y)) * vec3<f32>(1.04, 1.0, 0.95);
  // dawn in two steps: the night's map warms into land where it lies, then the real land comes up under it
  let a = smoothstep(0.0, 0.45, dawn);
  let b = smoothstep(0.45, 1.0, dawn);
  var col = mix(nightMap, mix(mapDay, day, b), a);
  let v = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, size.y / size.x));
  col = col * (1.0 - (0.5 - 0.3 * dawn) * smoothstep(0.45, 1.3, v));
  col = col * light;
  return vec4<f32>(col, 1.0);
}
"#;

toy_kit::ios_shell!(Walk);
