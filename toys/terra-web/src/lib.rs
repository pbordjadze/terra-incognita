//! Terra on the web: the iPhone's library (`toys/terra`) on a canvas through WebGPU, on the one frame core
//! (toy-kit's frame.rs) every platform shares, with its calls exported to JavaScript, so `index.html` is the
//! web's frame as `main.swift` is the iPhone's: it reads the chrome as data (`chrome`, `events`) and draws it
//! in HTML, and sends back what the reader did (`send`). The data comes in from the page, fetched beside it
//! (`tools/toys/web_build.sh`); the library fetches nothing.
#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;
use terra::Terra;
use toy_kit::{Frame, Host, MemoryAssets, Toy};
use wasm_bindgen::prelude::*;

/// Terra on a canvas: the one frame core (toy-kit's frame.rs) on the browser's surface.
#[wasm_bindgen]
pub struct Shell {
    frame: Frame<Terra>,
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
    // (a toy's warming and Nazaré's swells stop each frame's share of their work by it)
    toy_kit::set_work_clock(|| js_sys::Date::now() / 1000.0);
    let err = |e: &dyn std::fmt::Display| JsValue::from_str(&e.to_string());
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| err(&e))?;
    let data: HashMap<String, Vec<u8>> = names
        .into_iter()
        .zip(files.iter().map(|f| f.to_vec()))
        .collect();
    let mut frame = Frame::<Terra>::start(
        instance,
        surface,
        width,
        height,
        dpr,
        Box::new(MemoryAssets(data)),
    )
    .await
    .map_err(|e| err(&format!("{e:#}")))?;
    let h = &mut frame.host;
    (h.ctx.night, h.ctx.reduce_motion) = (night, reduce_motion);
    h.toy.settings(&mut h.ctx);
    Ok(Shell { frame })
}

#[wasm_bindgen]
impl Shell {
    /// Opens the toy `id` a second after the first frame and stays in it (the page's `#<toy id>`);
    /// false if Terra has no such toy.
    pub fn open_toy(&mut self, id: &str) -> bool {
        self.frame.host.toy.open_toy(id)
    }

    pub fn resize(&mut self, width: u32, height: u32, dpr: f64) {
        self.frame.resize(width, height, dpr);
    }

    /// A frame at `t` seconds (the page's monotonic clock); `now` is its wall clock.
    pub fn frame(&mut self, t: f64, now: f64) {
        toy_kit::set_wall_clock(now);
        self.frame.frame(t);
    }

    /// A finger (or the mouse): phase 0 down, 1 move, 2 up, 3 cancel; points from the canvas's corner.
    pub fn pointer(&mut self, id: u32, phase: u32, x: f64, y: f64, t: f64) {
        self.frame.pointer(id as u64, phase, x, y, t);
    }

    /// The device's own acceleration (g, gravity out; `devicemotion`'s `acceleration`) at `t` seconds: a
    /// few hard jolts are a shake, the open toy's.
    pub fn motion(&mut self, x: f64, y: f64, z: f64, t: f64) {
        self.frame.motion([x, y, z], t);
    }

    /// What the phone feels: which way is down (g, the device's frame: `devicemotion`'s
    /// `accelerationIncludingGravity` turned round and divided by g) and the compass's heading (degrees from
    /// magnetic north); NaN where the browser gives none. A browser has no magnetic field.
    pub fn sense(&mut self, gx: f64, gy: f64, gz: f64, heading: f64) {
        let nan = f64::NAN;
        self.frame.sense([gx, gy, gz], [nan, nan, nan], heading);
    }

    pub fn set_night(&mut self, on: bool) {
        self.frame.host.set_night(on);
    }

    pub fn set_reduce_motion(&mut self, on: bool) {
        self.frame.host.set_reduce_motion(on);
    }

    /// The page was away: the next frame's step is 0, not the time away.
    pub fn pause(&mut self) {
        self.frame.pause();
    }

    /// What Terra keeps between visits (the page puts it in localStorage).
    pub fn save(&self) -> Option<String> {
        self.frame.save()
    }

    pub fn restore(&mut self, s: &str) {
        self.frame.restore(s);
    }

    /// Fills `out` with interleaved stereo at `rate` Hz (silence without a sound).
    pub fn audio(&self, out: &mut [f32], rate: f64) {
        Host::<Terra>::audio(self.frame.host.synth.as_ref(), out, 2, rate);
    }

    /// The chrome as JSON (toy-kit's chrome.rs), when it changed since the last call.
    pub fn chrome(&mut self) -> Option<String> {
        self.frame.chrome_json().map(str::to_string)
    }

    /// What Terra asks of the page (a JSON list of chrome.rs's `Out`), when anything.
    pub fn events(&mut self) -> Option<String> {
        self.frame.events_json()
    }

    /// What the reader did in the page's controls (JSON: chrome.rs's `In`).
    pub fn send(&mut self, json: &str) {
        self.frame.send_json(json);
    }
}
