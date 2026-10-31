//! The look's own bench: draws the marks (adrift-core's `look::Look`) with wgpu over a picture of
//! the sea (Terra's Metal stand-in's, or any PNG) from a scene that `tools/adrift/study.py --scene`
//! writes, headless on Linux (Mesa's lavapipe: `bash tools/adrift/linux-setup.sh`). It is for trying
//! a colour or a size with the maybes and the drifter in place before Truth's data exists; the
//! whole app, ground and sea and all, is `adrift-check`.
//!
//!     python3 tools/adrift/study.py held phone --at=-68,36,2.6 --put=-74.5,35.3 --scene build/adrift/scene.json
//!     cargo run --release --manifest-path tools/adrift/lookbench/Cargo.toml -- build/adrift/scene.json out.png
//!
//! The scene names the base picture, the View (frame, disc, rotation, time, tone), the floats, the
//! wake, the maybes, the silver track and the words.
use adrift_core::gpu::{Gpu, ViewUniform};
use adrift_core::look::{self, Look, LookFloat, LookState};
use adrift_core::model::LonLat;
use serde::Deserialize;

#[derive(Deserialize)]
struct Scene {
    base: String,
    frame: [f32; 4],
    disc: [f32; 4],
    rot: [[f32; 4]; 3],
    time: [f32; 4],
    tone: [f32; 4],
    night: bool,
    floats: Vec<SceneFloat>,
    wake: Vec<[f64; 2]>,
    maybes: Vec<[f64; 2]>,
    maybes_bloom: f32,
    silver: Vec<[f64; 2]>,
    silver_rise: f32,
    date: String,
    credits: String,
    #[serde(default)]
    resting: Option<(f64, f64, f32)>,
}

#[derive(Deserialize)]
struct SceneFloat {
    lon: f64,
    lat: f64,
    #[allow(dead_code)]
    beat: f32,
    lit: f32,
    held: f32,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (scene_path, out) = (&args[1], &args[2]);
    let scene: Scene = serde_json::from_str(&std::fs::read_to_string(scene_path).expect("scene")).expect("scene json");
    let base = image::open(&scene.base).expect("base png").to_rgba8();
    let (w, h) = base.dimensions();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
    let gpu = pollster::block_on(Gpu::new(&instance, None)).expect("a device (install mesa-vulkan-drivers)");
    eprintln!("adapter: {}", gpu.adapter_info.name);
    // the target, with the sea (the base picture) as its first contents
    let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gpu.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let target_view = target.create_view(&Default::default());
    gpu.queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &base,
        wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );

    let view_layout = look::view_layout(&gpu);
    let view_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("view"),
        size: std::mem::size_of::<ViewUniform>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let view_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("view"),
        layout: &view_layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: view_buffer.as_entire_binding() }],
    });
    let u = ViewUniform { frame: scene.frame, disc: scene.disc, rot0: scene.rot[0], rot1: scene.rot[1], rot2: scene.rot[2], time: scene.time, tone: scene.tone, _pad: [0.0; 4] };
    gpu.queue.write_buffer(&view_buffer, 0, bytemuck::bytes_of(&u));

    let mut look = Look::new(&gpu, &view_layout);
    let wake: Vec<LonLat> = scene.wake.iter().map(|p| LonLat::new(p[0], p[1])).collect();
    let state = LookState {
        floats: scene.floats.iter().enumerate().map(|(i, f)| LookFloat { at: LonLat::new(f.lon, f.lat), lit: f.lit, held: f.held > 0.5, wake: if i == 0 { wake.clone() } else { Vec::new() } }).collect(),
        resting: scene.resting.map(|(lon, lat, beat)| (LonLat::new(lon, lat), beat)),
        maybes: scene.maybes.iter().map(|p| LonLat::new(p[0], p[1])).collect(),
        bloom: scene.maybes_bloom,
        drifter: scene.silver.iter().map(|p| LonLat::new(p[0], p[1])).collect(),
        rise: scene.silver_rise,
        night: scene.night,
        date: scene.date.clone(),
        credits: scene.credits.clone(),
    };
    look.prepare(&gpu, &state, scene.frame[0] / scene.frame[2], scene.frame[1] / scene.frame[2], scene.frame[3], [scene.disc[0], scene.disc[1], scene.disc[2]]);

    let mut enc = gpu.device.create_command_encoder(&Default::default());
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("look"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &target_view, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }, depth_slice: None })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        look.draw(&mut rp, &view_bind);
    }
    // read back
    let bpr = (w * 4 + 255) / 256 * 256;
    let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("readback"), size: (bpr * h) as u64, usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: None } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
    );
    gpu.queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::wait()).expect("poll");
    let data = slice.get_mapped_range();
    let mut px = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        px.extend_from_slice(&data[(y * bpr) as usize..(y * bpr + w * 4) as usize]);
    }
    image::save_buffer(out, &px, w, h, image::ColorType::Rgba8).expect("out");
    println!("{out}");
}
