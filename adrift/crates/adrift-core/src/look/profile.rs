//! (R24) The export frame's GPU work, pass by pass, behind `ADRIFT_PROFILE=1` (Visual's). With it
//! set, the app draws each pass in a render pass of its own (loading what the last left, so the
//! frame is the same) and waits for the GPU between them (`device.poll(wait)`: llvmpipe has no
//! timestamp queries worth trusting); the times add up here and `adrift-check journey` prints them.
//! Without it nothing changes. The phone's absolute times differ; the shares are what to cut.

use crate::gpu::Gpu;
use std::sync::Mutex;

static TIMES: Mutex<Vec<(&'static str, f64, u32)>> = Mutex::new(Vec::new());

/// Whether the passes are timed (`ADRIFT_PROFILE=1`).
pub fn on() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var("ADRIFT_PROFILE").map_or(false, |v| v == "1"))
}

fn wait(gpu: &Gpu) {
    let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
}

/// A pass begun alone: the GPU waited for, the clock started, an encoder.
pub fn begin(gpu: &Gpu) -> (wgpu::CommandEncoder, std::time::Instant) {
    wait(gpu);
    (gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("profiled") }), std::time::Instant::now())
}

/// Its render pass into `target`, cleared to `clear` if given, else loading what is there.
pub fn render_pass<'e>(enc: &'e mut wgpu::CommandEncoder, target: &'e wgpu::TextureView, clear: Option<wgpu::Color>) -> wgpu::RenderPass<'e> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("profiled"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            resolve_target: None,
            ops: wgpu::Operations { load: clear.map_or(wgpu::LoadOp::Load, wgpu::LoadOp::Clear), store: wgpu::StoreOp::Store },
            depth_slice: None,
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}

/// Submitted, waited for, and its time added to `name`'s.
pub fn end(gpu: &Gpu, name: &'static str, (enc, t): (wgpu::CommandEncoder, std::time::Instant)) {
    gpu.queue.submit([enc.finish()]);
    wait(gpu);
    record(name, t.elapsed().as_secs_f64());
}

/// `timed!(gpu, "name", target, clear, |rp| draws...)`: one pass drawn alone and timed.
#[macro_export]
macro_rules! timed {
    ($gpu:expr, $name:expr, $target:expr, $clear:expr, |$rp:ident| $body:expr) => {{
        let mut b = $crate::look::profile::begin($gpu);
        {
            let mut $rp = $crate::look::profile::render_pass(&mut b.0, $target, $clear);
            let $rp = &mut $rp;
            $body;
        }
        $crate::look::profile::end($gpu, $name, b);
    }};
}

/// Adds a time to a pass's sum.
pub fn record(name: &'static str, secs: f64) {
    let mut t = TIMES.lock().unwrap();
    match t.iter_mut().find(|e| e.0 == name) {
        Some(e) => {
            e.1 += secs;
            e.2 += 1;
        }
        None => t.push((name, secs, 1)),
    }
}

/// The table so far (each pass's mean ms a frame and its share), and the sums cleared.
pub fn report() -> String {
    let mut t = TIMES.lock().unwrap();
    let total: f64 = t.iter().map(|e| e.1 / e.2.max(1) as f64).sum();
    let mut s = format!("{:<10} {:>9} {:>7}\n", "pass", "ms/frame", "share");
    for (name, secs, n) in t.iter() {
        let ms = secs / (*n).max(1) as f64 * 1e3;
        s += &format!("{:<10} {:>9.2} {:>6.1}%\n", name, ms, ms / (total * 1e3).max(1e-9) * 100.0);
    }
    s += &format!("{:<10} {:>9.2}\n", "total", total * 1e3);
    t.clear();
    s
}
