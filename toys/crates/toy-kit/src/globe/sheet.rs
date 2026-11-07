//! A picture of the ground laid on the globe at its true place, size and turn (painting-company row 258):
//! the toy a globe holds says it (`Toy::seen_from_above`'s `Sheet`), and as the camera comes down onto that
//! place the satellite picture sharpens out of the month's ground, so the descent finds the toy's own ground
//! where it really is. One full-screen pass over the ground and its rivers; its edges dissolve into the
//! ground over the outer twentieth, broken by a noise in the ground, so it is never a drawn square.
use crate::gpu::textures::{decode, Texture};
use crate::gpu::{Gpu, Textures};
use crate::{Assets, Sheet};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct U {
    /// longitude, latitude of the middle (radians), metres a side, turn (radians)
    place: [f32; 4],
    /// the toy's lift (offset, gain, saturation), how much shows
    look: [f32; 4],
    /// close in, the camera's middle in metres east and north of the sheet's, metres a pixel, the roll
    /// (radians): the picture placed from the screen's own pixels (exact to a millimetre, where the
    /// globe's longitude and latitude in f32 step by most of a metre)
    local: [f32; 4],
    /// x 1 close in (the local placement)
    mode: [f32; 4],
}

pub struct SheetPass {
    pub sheet: Sheet,
    _tex: Texture,
    buf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    /// How much shows (0..1); a toy may ease it.
    pub alpha: f32,
}

const WGSL: &str = r#"
@group(0) @binding(0) var<uniform> U: View;
struct S { place: vec4f, look: vec4f, local: vec4f, mode: vec4f };
@group(1) @binding(0) var pic: texture_2d<f32>;
@group(1) @binding(1) var smp: sampler;
@group(1) @binding(2) var<uniform> Sh: S;
@group(1) @binding(3) var geoT: texture_2d<f32>;
@group(1) @binding(4) var landA: texture_2d<f32>;
@group(1) @binding(5) var landB: texture_2d<f32>;

fn lifted(c0: vec3f) -> vec3f {
    let l = dot(c0, vec3f(0.3, 0.59, 0.11));
    let c = clamp((c0 - Sh.look.x) * Sh.look.y, vec3f(0.0), vec3f(1.0));
    return clamp(mix(vec3f(l), c, Sh.look.z), vec3f(0.0), vec3f(1.0));
}

@fragment fn fs_main(in: FsIn) -> @location(0) vec4f {
    let G = ground_at(U, in.position.xy);
    // metres east and north of the sheet's middle (a tangent plane: the sheet is a few tens of km)
    var dlon = G.lon - Sh.place.x;
    dlon = dlon - 6.2831853 * round(dlon / 6.2831853);
    let e = 6371000.0 * cos(Sh.place.y) * dlon;
    let n = 6371000.0 * (G.lat - Sh.place.y);
    // into the picture: its right is the bearing turn + 90 degrees, its down the bearing turn + 180
    let t = Sh.place.w;
    var en = vec2f(e, n);
    if (Sh.mode.x > 0.5) {
        // close in: from the screen's own pixels about its middle, turned by the roll (the screen's up is
        // the bearing `roll`)
        let d = (in.position.xy - vec2f(U.disc.x, U.disc.y) * U.frame.z) * Sh.local.z;
        let r = Sh.local.w;
        en = Sh.local.xy + vec2f(d.x * cos(r) - d.y * sin(r), -d.x * sin(r) - d.y * cos(r));
    }
    let x = en.x * cos(t) - en.y * sin(t);
    let y = -(en.x * sin(t) + en.y * cos(t));
    let st = vec2f(x, y) / Sh.place.z + 0.5;
    let stc = clamp(st, vec2f(0.0), vec2f(1.0));
    var c = lifted(textureSample(pic, smp, stc).rgb);
    // the colour ripening with the depth: while a point of the screen covers a kilometre or more the
    // picture's broad colour is the month's ground's as the Earth is drawn (its light key), only its detail
    // the satellite's, so it comes in as more of the same ground rather than a darker square; by 50 m a point
    // it is the picture's own colour (the toy's, at the hand-off)
    let kmpt = 6371.0 / max(U.disc.z, 1.0);
    let key = smoothstep(-3.0, 0.0, log(kmpt));
    let px_m = Sh.place.z / f32(textureDimensions(pic).x);
    let broad = lifted(textureSampleLevel(pic, smp, stc, log2(12000.0 / px_m)).rgb);
    let month = mix(textureSampleLevel(landA, smp, G.uv, 1.0).rgb, textureSampleLevel(landB, smp, G.uv, 1.0).rgb, U.time.w);
    c = clamp(c + key * (month - broad), vec3f(0.0), vec3f(1.0));
    let l = dot(c, vec3f(0.3, 0.59, 0.11));
    // the edge: the outer twentieth dissolving, unevenly (a noise lying on the ground, so it holds still)
    let edge = min(min(st.x, 1.0 - st.x), min(st.y, 1.0 - st.y));
    let wob = vnoise(st * 9.0) * 0.6 + vnoise(st * 23.0 + 7.0) * 0.4;
    var a = smoothstep(0.0, 0.05, edge - 0.025 * (wob - 0.5) - 0.012);
    // on land only: the map's own sea where the picture has water past the drawn coast (a satellite's sea is
    // another colour from the map's), as the rivers give way to it
    let gg = textureSampleLevel(geoT, smp, G.uv, 0.0);
    let fine = (gg.r * 255.0 - 128.0) / 8.0;
    let coarse = (gg.g * 255.0 - 128.0) * 2.0;
    a = a * smoothstep(-0.6, 0.6, select(fine, coarse, abs(coarse) > 14.0));
    // by night (or a dusk): the picture's light in the night's tones, the bright ground pale by moonlight and the
    // dark darker still, as moonlight leaves it
    let night = U.pad.x;
    let nc = pow(l, 1.6) * vec3f(0.17, 0.185, 0.23) * 0.85 + vec3f(0.008, 0.010, 0.018);
    c = mix(c, nc, night);
    a = a * Sh.look.w * select(0.0, 1.0, G.r2 < 1.0);
    return vec4f(c * a, a);
}
"#;

impl SheetPass {
    pub fn new(
        gpu: &Gpu,
        view_layout: &wgpu::BindGroupLayout,
        assets: &dyn Assets,
        tx: &Textures,
        sheet: Sheet,
    ) -> anyhow::Result<SheetPass> {
        let tex = Texture::from_pixels(
            gpu,
            "ground sheet",
            &decode(&assets.read(&sheet.image)?, 4)?,
            false,
        );
        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ground sheet"),
                entries: &[
                    super::ground::tex2d(0),
                    super::ground::sampler(1),
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    super::ground::tex2d(3),
                    super::ground::tex2d(4),
                    super::ground::tex2d(5),
                ],
            });
        let buf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("ground sheet"),
            size: std::mem::size_of::<U>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let smp = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground sheet"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tex.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&smp),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&tx.geo.view),
                },
                // (the months when the sheet was laid: a toy that runs the date keeps laying it again)
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&tx.land_a.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(&tx.land_b.view),
                },
            ],
        });
        let shader = gpu.shader("ground sheet", &super::wgsl(WGSL));
        let pipeline = gpu.fullscreen_pipeline(
            "ground sheet",
            &shader,
            &[view_layout, &layout],
            Some(Gpu::over()),
        );
        Ok(SheetPass {
            sheet,
            _tex: tex,
            buf,
            bind,
            pipeline,
            alpha: 1.0,
        })
    }

    pub fn upload(&self, gpu: &Gpu, view: &crate::gpu::View) {
        let s = &self.sheet;
        let cam = view.camera;
        let r = 6_371_000.0;
        let mut dlon = (cam.lon - s.centre[0]).to_radians();
        dlon -= std::f64::consts::TAU * (dlon / std::f64::consts::TAU).round();
        let e = r * s.centre[1].to_radians().cos() * dlon;
        let n = r * (cam.lat - s.centre[1]).to_radians();
        let m_px = r / (view.disc.r * view.ppp).max(1.0);
        // close in (under a few hundred metres a point) the screen's own pixels place it
        let local = (m_px * view.ppp) < 300.0;
        let u = U {
            place: [
                s.centre[0].to_radians() as f32,
                s.centre[1].to_radians() as f32,
                s.side_m as f32,
                s.turn.to_radians() as f32,
            ],
            look: [s.lift[0], s.lift[1], s.lift[2], self.alpha],
            local: [
                e as f32,
                n as f32,
                m_px as f32,
                cam.roll.to_radians() as f32,
            ],
            mode: [if local { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        };
        gpu.queue.write_buffer(&self.buf, 0, bytemuck::bytes_of(&u));
    }

    pub fn draw(&self, rp: &mut wgpu::RenderPass<'_>, view_bind: &wgpu::BindGroup) {
        if self.alpha <= 0.0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.draw(0..3, 0..1);
    }
}
