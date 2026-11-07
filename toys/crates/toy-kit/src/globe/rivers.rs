//! The world's rivers over the ground (painting-company row 258, discovery through scale): HydroSHEDS'
//! HydroRIVERS, every reach with at least 300 km2 upstream (`rivers.bin`, built by
//! tools/toys/terra/rivers.py, whose docstring is its layout). A toy opts in with
//! [`Globe::with_rivers`](super::Globe::with_rivers); without it nothing here is made.
//!
//! What shows is set by the scale: a river is drawn once its upstream area is large for the ground a point
//! of the screen covers (its catchment some reading points across or more, `spacing`), so the density on screen
//! stays the same at every zoom and a pinch in keeps finding more: from afar the great rivers alone, closer
//! their tributaries, and theirs. A river comes in thin and faint and grows with the zoom; the smaller it is
//! the thinner. Nothing is ever a steady half-tone because of where it is: the fade is the zoom's, under the
//! fingers, and a tile's new detail streamed in eases in over half a second.
//!
//! Two layers: the coarse (10-degree tiles, the reaches of 2000 km2 and more, simplified to about 1.3 km)
//! is made at once; the fine (5-degree tiles, every reach, about 220 m) is decoded from the bundled file as
//! the camera comes close, at most two tiles a frame, the least recently seen let go past `KEEP`. Where all
//! four fine tiles of a coarse tile are in, they are drawn instead of it; where some are, the coarse tile is
//! drawn and the fine ones add only their smaller rivers.
//!
//! Drawn in two steps: every segment a capsule into a coverage mask (blended by max, so the joints of a
//! line and the meeting of two rivers never darken), then the mask laid over the ground in one ink.
use crate::assets::Assets;
use crate::gpu::{Gpu, Textures, View};
use std::collections::HashMap;
use std::sync::Mutex;

/// A river is shown once its catchment would be this many reading points across on screen: 22 from
/// afar, closing to 12 at the globe's closest, so the descent grows richer the lower it goes ("a ton of
/// rivers there") rather than holding one density.
fn spacing(km_per_point: f64) -> f64 {
    let u = ((km_per_point.max(1e-3).ln() - 1f64.ln()) / (20f64.ln() - 1f64.ln())).clamp(0.0, 1.0);
    12.0 + 10.0 * u * u * (3.0 - 2.0 * u)
}
/// The fine tiles are wanted once a point of the screen covers this many kilometres or fewer.
const FINE_KMPT: f64 = 4.5;
/// Fine tiles kept (the least recently seen let go beyond).
const KEEP: usize = 64;
const EARTH_KM: f64 = 6371.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Seg {
    /// The ends' longitude and latitude (radians).
    a: [f32; 2],
    b: [f32; 2],
    /// Importance: 8 log2 of the upstream area (km2).
    q: f32,
    /// When the tile came in (the drawn seconds; far past for the coarse layer).
    born: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    /// x the least importance shown (8 log2 km2), y the coarse layer's least, z the ink's strength, w night
    shown: [f32; 4],
}

/// A tile's segments, most important first, and how many have each importance or more.
struct Decoded {
    segs: Vec<Seg>,
    /// (q, segments with importance >= q), q descending.
    cum: Vec<(u8, u32)>,
}

impl Decoded {
    /// How many of the tile's segments have importance >= q.
    fn upto(&self, q: f64) -> u32 {
        let mut n = 0;
        for &(qq, c) in &self.cum {
            if (qq as f64) >= q {
                n = c;
            } else {
                break;
            }
        }
        n
    }
}

fn decode(bytes: &[u8], born: f32) -> Decoded {
    let mut segs = Vec::new();
    let mut cum: Vec<(u8, u32)> = Vec::new();
    let mut i = 0;
    let varint = |i: &mut usize| -> u64 {
        let mut v = 0u64;
        let mut s = 0;
        loop {
            let b = bytes[*i];
            *i += 1;
            v |= ((b & 0x7F) as u64) << s;
            if b & 0x80 == 0 {
                return v;
            }
            s += 7;
        }
    };
    let unzz = |v: u64| -> i64 { (v >> 1) as i64 ^ -((v & 1) as i64) };
    let k = (2e-4f64).to_radians();
    // each reach's first point from the one before's (the tile's first from 0, 0)
    let mut start = (0i64, 0i64);
    while i < bytes.len() {
        let q = bytes[i];
        i += 1;
        let n = varint(&mut i) as usize;
        let (mut x, mut y) = start;
        let mut prev: Option<[f32; 2]> = None;
        for j in 0..n {
            x += unzz(varint(&mut i));
            y += unzz(varint(&mut i));
            if j == 0 {
                start = (x, y);
            }
            let p = [(x as f64 * k) as f32, (y as f64 * k) as f32];
            if let Some(a) = prev {
                segs.push(Seg {
                    a,
                    b: p,
                    q: q as f32,
                    born,
                });
            }
            prev = Some(p);
        }
        match cum.last_mut() {
            Some(last) if last.0 == q => last.1 = segs.len() as u32,
            _ => cum.push((q, segs.len() as u32)),
        }
    }
    Decoded { segs, cum }
}

struct Fine {
    /// None: the tile has no rivers.
    buf: Option<wgpu::Buffer>,
    tile: Decoded,
    /// The first segment smaller than the coarse layer holds.
    small: u32,
    seen: f64,
}

pub struct Rivers {
    data: Vec<u8>,
    fw: usize,
    fh: usize,
    cw: usize,
    ch: usize,
    q_coarse: u8,
    fine_table: Vec<(u32, u32)>,
    coarse_buf: wgpu::Buffer,
    /// Each coarse tile's first segment in `coarse_buf`, and its counts.
    coarse: Vec<(u32, Decoded)>,
    fine: HashMap<usize, Fine>,
    params: wgpu::Buffer,
    params_bind: wgpu::BindGroup,
    mask_pipeline: wgpu::RenderPipeline,
    lay_pipeline: wgpu::RenderPipeline,
    lay_layout: wgpu::BindGroupLayout,
    /// The coast's distance field (the ground's), so a river gives way to the water.
    geo: wgpu::TextureView,
    geo_sampler: wgpu::Sampler,
    /// The coverage mask at the screen's size, and the bind group that lays it.
    mask: Mutex<Option<Mask>>,
    /// The ink's strength: a toy may ease it (0 hides the rivers and skips their drawing).
    pub strength: f32,
    /// The last mask's segments and draws, and the fine tiles held (`stats`).
    last: Mutex<(u32, u32)>,
}

/// The coverage mask, its view, the bind group that lays it, and its size.
type Mask = (wgpu::Texture, wgpu::TextureView, wgpu::BindGroup, [u32; 2]);

const MASK: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;

const WGSL: &str = r#"
@group(0) @binding(0) var<uniform> U: View;
struct P { shown: vec4f };
@group(1) @binding(0) var<uniform> Pr: P;

struct SegOut {
    @builtin(position) pos: vec4f,
    @location(0) @interpolate(flat) ends: vec4f,     // the ends, pixels
    @location(1) @interpolate(flat) look: vec2f,     // half width (pixels), strength
};

fn world(ll: vec2f) -> vec3f {
    return vec3f(cos(ll.y) * sin(ll.x), sin(ll.y), cos(ll.y) * cos(ll.x));
}

@vertex fn vs_seg(@builtin(vertex_index) vi: u32, @location(0) a: vec2f, @location(1) b: vec2f, @location(2) q: f32, @location(3) born: f32) -> SegOut {
    var o: SegOut;
    let ga = world(a); let gb = world(b);
    let va = vec3f(dot(U.rot0.xyz, ga), dot(U.rot1.xyz, ga), dot(U.rot2.xyz, ga));
    let vb = vec3f(dot(U.rot0.xyz, gb), dot(U.rot1.xyz, gb), dot(U.rot2.xyz, gb));
    let k = U.frame.z;
    let R = U.disc.z;
    let pa = vec2f(U.disc.x + va.x * R, U.disc.y - va.y * R) * k;
    let pb = vec2f(U.disc.x + vb.x * R, U.disc.y - vb.y * R) * k;
    // shown by its importance against the scale's (faint a factor of two below, whole a little above),
    // eased in over half a second when its tile has just come, and gone toward the limb
    let rel = (q - Pr.shown.x) / 8.0;
    var alpha = smoothstep(-1.0, 0.6, rel);
    if (q < Pr.shown.y) { alpha *= smoothstep(born, born + 0.6, U.time.x); }
    alpha *= smoothstep(0.02, 0.18, min(va.z, vb.z));
    let wpt = clamp(0.55 + 0.38 * rel, 0.45, 2.3) * U.frame.w;
    let h = wpt * 0.5 * k;
    o.ends = vec4f(pa, pb);
    o.look = vec2f(h, alpha * Pr.shown.z);
    // the capsule's box, a pixel beyond its edge
    var d = pb - pa;
    let len = length(d);
    d = select(vec2f(1.0, 0.0), d / len, len > 1e-4);
    let n = vec2f(-d.y, d.x);
    let e = h + 1.0;
    let corner = array<vec2f, 4>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0), vec2f(1.0, 1.0))[vi];
    let base = select(pa, pb, corner.x > 0.0);
    let p = base + d * corner.x * e + n * corner.y * e;
    o.pos = vec4f(p.x / U.frame.x * 2.0 - 1.0, 1.0 - p.y / U.frame.y * 2.0, 0.0, 1.0);
    if (alpha <= 0.002 || va.z <= 0.0 || vb.z <= 0.0) { o.pos = vec4f(2.0, 2.0, 2.0, 1.0); }
    return o;
}

@fragment fn fs_seg(i: SegOut) -> @location(0) vec4f {
    let p = i.pos.xy;
    let a = i.ends.xy; let b = i.ends.zw;
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-6), 0.0, 1.0);
    let dist = length(p - (a + ab * t));
    // a line under a pixel wide keeps a pixel's width and gives up its strength instead
    let w = max(i.look.x, 0.5);
    let cov = clamp(w + 0.5 - dist, 0.0, 1.0) * min(i.look.x / 0.5, 1.0);
    return vec4f(cov * i.look.y, 0.0, 0.0, 1.0);
}

@group(2) @binding(0) var mask: texture_2d<f32>;
@group(2) @binding(1) var geoT: texture_2d<f32>;
@group(2) @binding(2) var geoS: sampler;

const INK_DAY = vec3f(72.0, 136.0, 178.0) / 255.0;
const INK_NIGHT = vec3f(64.0, 96.0, 132.0) / 255.0;

@fragment fn fs_lay(in: FsIn) -> @location(0) vec4f {
    var m = textureLoad(mask, vec2i(in.position.xy), 0).r;
    // on land only: where a river crosses a lake the map draws, or runs on past the drawn coast, it gives way
    // to the water (HydroSHEDS' lines and Natural Earth's coast are two measurements)
    let G = ground_at(U, in.position.xy);
    let gg = textureSampleLevel(geoT, geoS, G.uv, 0.0);
    let fine = (gg.r * 255.0 - 128.0) / 8.0;
    let coarse = (gg.g * 255.0 - 128.0) * 2.0;
    let d = select(fine, coarse, abs(coarse) > 14.0);
    m = m * smoothstep(-0.3, 0.5, d) * select(0.0, 1.0, G.r2 < 1.0);
    let night = Pr.shown.w;
    let a = m * mix(0.82, 0.5, night);
    return vec4f(mix(INK_DAY, INK_NIGHT, night) * a, a);
}
"#;

impl Rivers {
    pub fn new(
        gpu: &Gpu,
        view_layout: &wgpu::BindGroupLayout,
        assets: &dyn Assets,
        tx: &Textures,
    ) -> anyhow::Result<Rivers> {
        let data = assets.read("rivers.bin")?;
        anyhow::ensure!(
            data.len() > 24 && &data[0..4] == b"TRV1",
            "rivers.bin: not a rivers file"
        );
        let u = |o: usize| u32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        let (fw, fh, cw, ch) = (u(4) as usize, u(8) as usize, u(12) as usize, u(16) as usize);
        let q_coarse = u(20) as u8;
        let table: Vec<(u32, u32)> = (0..cw * ch + fw * fh)
            .map(|i| (u(24 + 8 * i), u(28 + 8 * i)))
            .collect();
        let mut coarse = Vec::with_capacity(cw * ch);
        let mut all: Vec<Seg> = Vec::new();
        for &(off, len) in &table[..cw * ch] {
            let d = decode(&data[off as usize..(off + len) as usize], -1e9);
            coarse.push((
                all.len() as u32,
                Decoded {
                    segs: Vec::new(),
                    cum: d.cum,
                },
            ));
            all.extend_from_slice(&d.segs);
        }
        let fine_table = table[cw * ch..].to_vec();
        let coarse_buf = Self::seg_buffer(gpu, "rivers coarse", &all);
        crate::step(&format!("the rivers: {} coarse segments", all.len()));

        let params = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("rivers"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rivers"),
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
        let params_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rivers"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params.as_entire_binding(),
            }],
        });
        let lay_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("rivers mask"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: false },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    super::ground::tex2d(1),
                    super::ground::sampler(2),
                ],
            });
        let shader = gpu.shader("rivers", &super::wgsl(WGSL));
        let mask_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rivers mask"),
                bind_group_layouts: &[Some(view_layout), Some(&params_layout)],
                immediate_size: 0,
            });
        let max = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Max,
        };
        let mask_pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rivers mask"),
            layout: Some(&mask_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_seg"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Seg>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Float32],
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_seg"),
                targets: &[Some(wgpu::ColorTargetState { format: MASK, blend: Some(wgpu::BlendState { color: max, alpha: max }), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
        let lay_pl = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("rivers lay"),
                bind_group_layouts: &[Some(view_layout), Some(&params_layout), Some(&lay_layout)],
                immediate_size: 0,
            });
        let lay_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("rivers lay"),
                layout: Some(&lay_pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_lay"),
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
        Ok(Rivers {
            data,
            fw,
            fh,
            cw,
            ch,
            q_coarse,
            fine_table,
            coarse_buf,
            coarse,
            fine: HashMap::new(),
            params,
            params_bind,
            mask_pipeline,
            lay_pipeline,
            lay_layout,
            geo: tx.geo.view.clone(),
            geo_sampler: tx.linear_wrap_x.clone(),
            mask: Mutex::new(None),
            strength: 1.0,
            last: Mutex::new((0, 0)),
        })
    }

    fn seg_buffer(gpu: &Gpu, label: &str, segs: &[Seg]) -> wgpu::Buffer {
        let buf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (segs.len().max(1) * std::mem::size_of::<Seg>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if !segs.is_empty() {
            gpu.queue.write_buffer(&buf, 0, bytemuck::cast_slice(segs));
        }
        buf
    }

    /// Kilometres of ground a point of the screen covers at the middle.
    fn km_per_point(view: &View) -> f64 {
        EARTH_KM / view.disc.r.max(1.0)
    }

    /// The least importance shown now (8 log2 of the upstream area in km2).
    fn shown(view: &View) -> f64 {
        let kmpt = Self::km_per_point(view);
        let span = spacing(kmpt) * view.reading * kmpt;
        8.0 * (span * span).log2()
    }

    /// The tiles (of a w x h grid) whose ground may be on screen.
    fn visible(view: &View, w: usize, h: usize) -> Vec<usize> {
        let m = view.camera.rotation();
        let c = m[2];
        let (sw, sh) = (view.width as f64 / view.ppp, view.height as f64 / view.ppp);
        let rho = (sw * 0.5).hypot(sh * 0.5) / view.disc.r.max(1.0);
        let reach = if rho >= 1.0 {
            std::f64::consts::FRAC_PI_2
        } else {
            rho.asin()
        };
        let (dlon, dlat) = (360.0 / w as f64, 180.0 / h as f64);
        let half = (0.5 * dlat).to_radians().hypot((0.5 * dlon).to_radians()) * 1.05;
        let mut out = Vec::new();
        for r in 0..h {
            let lat = (90.0 - (r as f64 + 0.5) * dlat).to_radians();
            // a tile near a pole spans less: its half-diagonal on the sphere by its widest row
            let lat_edge = (90.0 - r as f64 * dlat)
                .abs()
                .min((90.0 - (r + 1) as f64 * dlat).abs());
            let half_r = (0.5 * dlat)
                .to_radians()
                .hypot((0.5 * dlon).to_radians() * (90.0 - lat_edge).to_radians().sin().max(0.05))
                * 1.05;
            let half_r = half_r.min(half);
            for col in 0..w {
                let lon = (-180.0 + (col as f64 + 0.5) * dlon).to_radians();
                let g = [lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos()];
                let d = (g[0] * c[0] + g[1] * c[1] + g[2] * c[2])
                    .clamp(-1.0, 1.0)
                    .acos();
                if d < reach + half_r {
                    out.push(r * w + col);
                }
            }
        }
        out
    }

    /// Streams the fine tiles the view needs (at most two a frame) and lets go of the oldest.
    pub fn update(&mut self, gpu: &Gpu, view: &View, t: f64) {
        if self.strength <= 0.0 || Self::km_per_point(view) > FINE_KMPT {
            return;
        }
        let want = Self::visible(view, self.fw, self.fh);
        let mut loaded = 0;
        for &i in &want {
            if let Some(f) = self.fine.get_mut(&i) {
                f.seen = t;
                continue;
            }
            if loaded >= 2 {
                continue;
            }
            let (off, len) = self.fine_table[i];
            let tile = decode(&self.data[off as usize..(off + len) as usize], t as f32);
            let buf =
                (!tile.segs.is_empty()).then(|| Self::seg_buffer(gpu, "rivers fine", &tile.segs));
            let small = tile.upto(self.q_coarse as f64);
            self.fine.insert(
                i,
                Fine {
                    buf,
                    tile: Decoded {
                        segs: Vec::new(),
                        cum: tile.cum,
                    },
                    small,
                    seen: t,
                },
            );
            loaded += 1;
        }
        if self.fine.len() > KEEP {
            let mut ages: Vec<(f64, usize)> = self.fine.iter().map(|(&i, f)| (f.seen, i)).collect();
            ages.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            for &(_, i) in ages.iter().take(self.fine.len() - KEEP) {
                self.fine.remove(&i);
            }
        }
    }

    /// Draws the coverage mask for the view (its own pass; call before the frame's pass begins).
    pub fn mask(&self, gpu: &Gpu, view: &View, view_bind: &wgpu::BindGroup) {
        let size = [view.width.max(1), view.height.max(1)];
        let mut guard = self.mask.lock().unwrap();
        if guard.as_ref().map(|m| m.3) != Some(size) {
            let tex = gpu.texture(&wgpu::TextureDescriptor {
                label: Some("rivers mask"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: MASK,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let tv = tex.create_view(&Default::default());
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("rivers mask"),
                layout: &self.lay_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&tv),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self.geo),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.geo_sampler),
                    },
                ],
            });
            *guard = Some((tex, tv, bind, size));
        }
        let shown = Self::shown(view);
        let p = Params {
            shown: [
                shown as f32,
                self.q_coarse as f32,
                self.strength,
                view.night,
            ],
        };
        gpu.queue
            .write_buffer(&self.params, 0, bytemuck::bytes_of(&p));
        let (_, tv, _, _) = guard.as_ref().unwrap();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("rivers mask"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: tv,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if self.strength > 0.0 {
                rp.set_pipeline(&self.mask_pipeline);
                rp.set_bind_group(0, view_bind, &[]);
                rp.set_bind_group(1, &self.params_bind, &[]);
                // a factor of two below the scale's least shows faintly
                let q = shown - 8.0;
                let fine_on = Self::km_per_point(view) <= FINE_KMPT * 1.5;
                let per = (self.fw / self.cw).max(1);
                let mut drawn = (0u32, 0u32);
                for ci in Self::visible(view, self.cw, self.ch) {
                    let (cr, cc) = (ci / self.cw, ci % self.cw);
                    let kids: Vec<usize> = (0..per * per)
                        .map(|k| (cr * per + k / per) * self.fw + cc * per + k % per)
                        .collect();
                    let all_in = fine_on && kids.iter().all(|k| self.fine.contains_key(k));
                    if !all_in {
                        let (first, tile) = &self.coarse[ci];
                        let n = tile.upto(q);
                        if n > 0 {
                            rp.set_vertex_buffer(0, self.coarse_buf.slice(..));
                            rp.draw(0..4, *first..*first + n);
                            drawn = (drawn.0 + n, drawn.1 + 1);
                        }
                    }
                    if !fine_on {
                        continue;
                    }
                    for k in kids {
                        let Some(f) = self.fine.get(&k) else { continue };
                        let Some(buf) = &f.buf else { continue };
                        let n = f.tile.upto(q);
                        let from = if all_in { 0 } else { f.small };
                        if n > from {
                            rp.set_vertex_buffer(0, buf.slice(..));
                            rp.draw(0..4, from..n);
                            drawn = (drawn.0 + n - from, drawn.1 + 1);
                        }
                    }
                }
                *self.last.lock().unwrap() = drawn;
                static SAY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
                if *SAY.get_or_init(|| std::env::var("KIT_RIVERS").is_ok()) {
                    crate::log(&format!(
                        "rivers: {} segments in {} draws, {} fine tiles, {:.3} km a point",
                        drawn.0,
                        drawn.1,
                        self.fine.len(),
                        Self::km_per_point(view)
                    ));
                }
            }
        }
        gpu.queue.submit([enc.finish()]);
    }

    /// What the last frame drew: segments, draw calls, and fine tiles held (for a frame-rate watch; the
    /// check prints it with `KIT_RIVERS=1`).
    pub fn stats(&self) -> (u32, u32, usize) {
        let l = *self.last.lock().unwrap();
        (l.0, l.1, self.fine.len())
    }

    /// Lays the mask over what the pass has drawn (the ground).
    pub fn lay(&self, rp: &mut wgpu::RenderPass<'_>, view_bind: &wgpu::BindGroup) {
        if self.strength <= 0.0 {
            return;
        }
        let guard = self.mask.lock().unwrap();
        let Some((_, _, bind, _)) = guard.as_ref() else {
            return;
        };
        rp.set_pipeline(&self.lay_pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.params_bind, &[]);
        rp.set_bind_group(2, bind, &[]);
        rp.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_a_reach() {
        // q 100, two points (2e-4 degree units): (10, -20) then (+3, +4); q 90 from (10, -20): (10, -20), (11, -20)
        let zz = |v: i64| ((v << 1) ^ (v >> 63)) as u8;
        let bytes = [
            100u8,
            2,
            zz(10),
            zz(-20),
            zz(3),
            zz(4),
            90,
            2,
            0,
            0,
            zz(1),
            0,
        ];
        let d = decode(&bytes, 0.0);
        assert_eq!(d.segs.len(), 2);
        assert_eq!(d.cum, vec![(100, 1), (90, 2)]);
        assert_eq!(d.upto(95.0), 1);
        assert_eq!(d.upto(80.0), 2);
        assert!((d.segs[0].b[1] - (-32e-4f64).to_radians() as f32).abs() < 1e-9);
        assert!((d.segs[1].b[0] - (22e-4f64).to_radians() as f32).abs() < 1e-9);
    }
}
