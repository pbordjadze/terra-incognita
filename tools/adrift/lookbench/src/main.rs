//! The look's own bench: draws every look pass with wgpu over a picture of the sea (Terra's Metal
//! stand-in's, or any PNG) from a scene that `tools/adrift/study.py --scene` writes, headless on
//! Linux (Mesa's lavapipe: `bash tools/adrift/linux-setup.sh`, or `apt-get install mesa-vulkan-drivers`).
//!
//!     python3 tools/adrift/study.py held phone --at=-68,36,2.6 --put=-74.5,35.3 --scene build/adrift/scene.json
//!     cargo run --release --manifest-path tools/adrift/lookbench/Cargo.toml -- build/adrift/scene.json out.png
//!
//! The scene names the base picture, the View (frame, disc, rotation, time, tone), the floats, the
//! wake, the maybes, the silver track and the words. The look's Rust is included from the crate by
//! path, so what this draws is what the app draws; until Engineering's `adrift-check` renders the
//! look, this is the running build the critics judge.
#[path = "../../../../adrift/crates/adrift-core/src/look/mod.rs"]
mod look;

use look::{floats::FloatInstance, maybes::SpeckInstance, words::Line};
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
    foot: f32,
}

#[derive(Deserialize)]
struct SceneFloat { lon: f64, lat: f64, beat: f32, lit: f32, held: f32 }

fn read_png(path: &str) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::fs::File::open(path).expect("base png"));
    let mut reader = decoder.read_info().expect("png");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        _ => panic!("the base must be RGB or RGBA"),
    };
    (info.width, info.height, rgba)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (scene_path, out) = (&args[1], &args[2]);
    let scene: Scene = serde_json::from_str(&std::fs::read_to_string(scene_path).expect("scene")).expect("scene json");
    let (w, h, base) = read_png(&scene.base);

    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).expect("an adapter (install mesa-vulkan-drivers)");
    eprintln!("adapter: {} ({:?})", adapter.get_info().name, adapter.get_info().backend);
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default(), None)).expect("a device");
    let format = wgpu::TextureFormat::Rgba8Unorm;

    // the sea: the base picture as the target's first contents
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"), size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: 1, sample_count: 1, dimension: wgpu::TextureDimension::D2, format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::COPY_DST, view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        &base, wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w * 4), rows_per_image: Some(h) },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });

    let view = look::ViewBinding::new(&device);
    view.write(&queue, &look::ViewRaw { frame: scene.frame, disc: scene.disc, rot0: scene.rot[0], rot1: scene.rot[1], rot2: scene.rot[2], time: scene.time, tone: scene.tone });
    let night = scene.night;
    let n = night as u8 as f32;

    let mut wake = look::wake::Wake::new(&device, format, &view.layout);
    let mut maybes = look::maybes::Maybes::new(&device, format, &view.layout);
    let mut silver = look::silver::Silver::new(&device, format, &view.layout);
    let mut floats = look::floats::Floats::new(&device, format, &view.layout);
    let mut words = look::words::Words::new(&device, &queue, format, &view.layout);

    let path: Vec<[f32; 3]> = scene.wake.iter().map(|p| look::unit(p[0], p[1])).collect();
    wake.prepare(&device, &queue, &look::wake::beads(&path, night));
    let specks: Vec<SpeckInstance> = scene.maybes.iter().map(|p| SpeckInstance { g: look::unit(p[0], p[1]) }).collect();
    maybes.prepare(&device, &queue, &specks, scene.maybes_bloom, night);
    let track: Vec<[f32; 3]> = scene.silver.iter().map(|p| look::unit(p[0], p[1])).collect();
    silver.prepare(&device, &queue, &track, scene.silver_rise, night);
    let fl: Vec<FloatInstance> = scene.floats.iter().map(|f| FloatInstance { g: look::unit(f.lon, f.lat), beat: f.beat, lit: f.lit, held: f.held, night: n }).collect();
    floats.prepare(&device, &queue, &fl);

    // the words: the date 17 pt and the credits 10 pt, centred at the foot (palette.json)
    let reading = scene.frame[3];
    let (date_ink, credit_ink, halo) = if night {
        ([0.894, 0.925, 0.957, 1.0], [0.722, 0.776, 0.831, 1.0], [0.063, 0.094, 0.157, 0.92])
    } else {
        ([0.149, 0.188, 0.227, 1.0], [0.243, 0.298, 0.353, 1.0], [0.761, 0.894, 0.957, 0.92])
    };
    let cx = scene.frame[0] / scene.frame[2] / 2.0;
    let mut glyphs = words.face.layout(&Line { text: &scene.date, size: 17.0 * reading, ink: date_ink, tracking: 0.0 }, cx, scene.foot);
    glyphs.extend(words.face.layout(&Line { text: &scene.credits, size: 10.0 * reading, ink: credit_ink, tracking: 0.0 }, cx, scene.foot + 22.0 * reading));
    words.prepare(&device, &queue, &glyphs, halo);

    let tv = target.create_view(&Default::default());
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("look"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &tv, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            ..Default::default()
        });
        // the draw order (DECISIONS.md §3): wakes, maybes, drifter, floats, chrome
        wake.draw(&mut pass, &view.group);
        maybes.draw(&mut pass, &view.group);
        silver.draw(&mut pass, &view.group);
        floats.draw(&mut pass, &view.group);
        words.draw(&mut pass, &view.group);
    }
    // read back
    let bpr = (w * 4 + 255) / 256 * 256;
    let staging = device.create_buffer(&wgpu::BufferDescriptor { label: Some("staging"), size: (bpr * h) as u64, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &target, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &staging, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: Some(h) } },
        wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 });
    queue.submit([encoder.finish()]);
    let slice = staging.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::Maintain::Wait);
    let data = slice.get_mapped_range();
    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h { pixels.extend_from_slice(&data[(y * bpr) as usize..(y * bpr + w * 4) as usize]); }
    let file = std::fs::File::create(out).expect("out");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&pixels).unwrap();
    println!("{out}");
}
