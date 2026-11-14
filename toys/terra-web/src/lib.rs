//! Terra on the web: the iPhone's library (`toys/terra`) on a canvas through WebGPU, with the iPhone
//! shell's calls (`toys/ios/Sources/toy.h`) exported to JavaScript one for one, so `web/index.html` does
//! what `main.swift` does (the info line, the rules card, the switches panel, the title, the sound) in
//! HTML. The data comes in from the page, fetched beside it (`tools/toys/web_build.sh`); the library
//! fetches nothing.
#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;
use terra::Terra;
use toy_kit::{Ctx, Gpu, Host, MemoryAssets, Phase, PointerEvent, Toy};
use wasm_bindgen::prelude::*;

/// Terra on a canvas.
#[wasm_bindgen]
pub struct Shell {
    /// (kept: in a browser, the device is lost once its instance is gone)
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    host: Host<Terra>,
}

fn text(c: Option<&std::ffi::CStr>) -> Option<String> {
    c.map(|c| c.to_string_lossy().into_owned())
}

fn spot(p: Option<[f64; 2]>) -> Option<Vec<f64>> {
    p.map(|p| p.to_vec())
}

/// Starts Terra on `canvas` (`width` x `height` pixels at `dpr` pixels a point), its data the files
/// `names[i]` = `files[i]`; the page's wall clock is `now` (seconds since 1970).
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)] // (each is one of the iPhone shell's toy_new's, plus the data)
pub async fn start(
    canvas: web_sys::HtmlCanvasElement,
    width: u32,
    height: u32,
    dpr: f64,
    names: Vec<String>,
    files: Vec<js_sys::Uint8Array>,
    now: f64,
    night: bool,
    reduce_motion: bool,
) -> Result<Shell, JsValue> {
    console_error_panic_hook::set_once();
    toy_kit::set_log_sink(|line| web_sys::console::log_1(&format!("toy: {line}").into()));
    toy_kit::set_wall_clock(now);
    let err = |e: &dyn std::fmt::Display| JsValue::from_str(&e.to_string());
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| err(&e))?;
    let gpu = Gpu::new(&instance, Some(&surface))
        .await
        .map_err(|e| err(&format!("{e:#}")))?;
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gpu.format,
        color_space: wgpu::SurfaceColorSpace::Srgb,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);
    let data: HashMap<String, Vec<u8>> = names
        .into_iter()
        .zip(files.iter().map(|f| f.to_vec()))
        .collect();
    let mut ctx = Ctx::new(
        gpu,
        Box::new(MemoryAssets(data)),
        width.max(1),
        height.max(1),
        dpr,
    );
    (ctx.night, ctx.reduce_motion) = (night, reduce_motion);
    let mut host = Host::<Terra>::new(ctx).map_err(|e| err(&format!("{e:#}")))?;
    host.toy.settings(&mut host.ctx);
    Ok(Shell {
        _instance: instance,
        surface,
        config,
        host,
    })
}

#[wasm_bindgen]
impl Shell {
    pub fn resize(&mut self, width: u32, height: u32, dpr: f64) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface
            .configure(&self.host.ctx.gpu.device, &self.config);
        self.host.resize(width, height, dpr);
    }

    /// A frame at `t` seconds (the page's monotonic clock); `now` is its wall clock.
    pub fn frame(&mut self, t: f64, now: f64) {
        toy_kit::set_wall_clock(now);
        self.host.update(t);
        match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                let view = frame.texture.create_view(&Default::default());
                self.host.draw(&view);
                self.host.ctx.gpu.queue.present(frame);
            }
            _ => self
                .surface
                .configure(&self.host.ctx.gpu.device, &self.config),
        }
    }

    /// A finger (or the mouse): phase 0 down, 1 move, 2 up, 3 cancel; points from the canvas's corner.
    pub fn pointer(&mut self, id: u32, phase: u32, x: f64, y: f64, t: f64) {
        let phase = match phase {
            0 => Phase::Down,
            1 => Phase::Move,
            2 => Phase::Up,
            _ => Phase::Cancel,
        };
        self.host.pointer(PointerEvent {
            id: id as u64,
            phase,
            x,
            y,
            t,
        });
    }

    pub fn take_haptic(&mut self) -> bool {
        self.host.take_haptic()
    }

    pub fn set_night(&mut self, on: bool) {
        self.host.set_night(on);
    }

    pub fn set_reduce_motion(&mut self, on: bool) {
        self.host.set_reduce_motion(on);
    }

    /// The page was away: the next frame's step is 0, not the time away.
    pub fn pause(&mut self) {
        self.host.pause();
    }

    /// What Terra keeps between visits (the page puts it in localStorage).
    pub fn save(&self) -> Option<String> {
        self.host.toy.save()
    }

    pub fn restore(&mut self, s: &str) {
        self.host.toy.restore(&mut self.host.ctx, s);
    }

    /// Fills `out` with interleaved stereo at `rate` Hz (silence without a sound).
    pub fn audio(&self, out: &mut [f32], rate: f64) {
        Host::<Terra>::audio(self.host.synth.as_ref(), out, 2, rate);
    }

    pub fn info_spot(&self) -> Option<Vec<f64>> {
        spot(self.host.info_spot())
    }

    pub fn info_line(&self) -> Option<String> {
        text(self.host.info.line())
    }

    pub fn info_open(&mut self, t: f64) {
        self.host.open_info(t);
    }

    pub fn rules_spot(&self) -> Option<Vec<f64>> {
        spot(self.host.rules_spot())
    }

    pub fn rules_text(&self) -> Option<String> {
        text(self.host.info.rules())
    }

    pub fn rules_open(&mut self) {
        self.host.open_rules();
    }

    pub fn origins_spot(&self) -> Option<Vec<f64>> {
        spot(self.host.origins_spot())
    }

    pub fn origins_text(&self) -> Option<String> {
        text(self.host.info.origins())
    }

    pub fn origins_open(&mut self) {
        self.host.open_origins();
    }

    pub fn switches_spot(&self) -> Option<Vec<f64>> {
        spot(self.host.switches_spot())
    }

    pub fn switches_text(&self) -> Option<String> {
        text(self.host.switches_text())
    }

    pub fn switches_open(&mut self) {
        self.host.open_switches();
    }

    pub fn switch_set(&mut self, i: u32, on: bool) {
        self.host.set_switch(i as usize, on);
    }

    pub fn card_at_top(&self) -> bool {
        self.host.card_at_top()
    }

    /// Terra's buttons (its lights, the Earth mark), a line each: label, a tab, x, a tab, y in points.
    pub fn buttons(&mut self) -> String {
        self.host.buttons_text().to_string_lossy().into_owned()
    }

    pub fn press(&mut self, i: u32, t: f64) {
        self.host.press(i as usize, t);
    }

    pub fn title(&mut self) -> Option<String> {
        text(self.host.title_text())
    }
}
