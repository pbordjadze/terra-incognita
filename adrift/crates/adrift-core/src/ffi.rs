//! The device's C ABI (adrift/ios/Sources/Adrift/*.swift calls it): a CAMetalLayer in, frames out.

use crate::app::App;
use crate::assets::DirAssets;
use crate::gpu::Gpu;
use crate::model::{Phase, PointerEvent};
use std::ffi::{c_char, c_void, CStr};

pub struct Shell {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    app: App,
    store: Option<std::path::PathBuf>,
    /// When the frame stats were last printed.
    stats_at: f64,
    /// The offscreen target of an export under way (R9).
    export: Option<crate::gpu::Offscreen>,
}

/// Makes the app over a CAMetalLayer. `scale` is the screen's pixels a point; `assets` the bundle's
/// resource directory. Null on failure (the reason is logged).
#[no_mangle]
pub unsafe extern "C" fn adrift_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut Shell {
    let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::METAL, ..Default::default() });
    let surface = match instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adrift: no surface: {e}");
            return std::ptr::null_mut();
        }
    };
    let gpu = match pollster_lite(Gpu::new(&instance, Some(&surface))) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("adrift: no device: {e}");
            return std::ptr::null_mut();
        }
    };
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gpu.format,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);
    let app = match App::new(gpu, Box::new(DirAssets(dir.into())), width, height, scale) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("adrift: {e}");
            return std::ptr::null_mut();
        }
    };
    Box::into_raw(Box::new(Shell { _instance: instance, surface, config, app, store: None, stats_at: 0.0, export: None }))
}

#[no_mangle]
pub unsafe extern "C" fn adrift_resize(shell: *mut Shell, width: u32, height: u32, scale: f64) {
    let Some(s) = shell.as_mut() else { return };
    s.config.width = width.max(1);
    s.config.height = height.max(1);
    s.surface.configure(&s.app.gpu.device, &s.config);
    s.app.resize(width, height, scale);
}

/// A frame at the display's time `t` (seconds).
#[no_mangle]
pub unsafe extern "C" fn adrift_frame(shell: *mut Shell, t: f64) {
    let Some(s) = shell.as_mut() else { return };
    let frame = match s.surface.get_current_texture() {
        Ok(f) => f,
        Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
            s.surface.configure(&s.app.gpu.device, &s.config);
            return;
        }
        Err(e) => {
            eprintln!("adrift: frame: {e}");
            return;
        }
    };
    let view = frame.texture.create_view(&Default::default());
    s.app.frame(&view, t);
    frame.present();
    // the frame's cost, every five seconds, in the device's log (Console.app, or Xcode's)
    if t - s.stats_at >= 5.0 {
        s.stats_at = t;
        eprintln!("{}", s.app.stats());
    }
}

/// A touch: `phase` 0 down, 1 move, 2 up, 3 cancel; x, y in points; t seconds.
#[no_mangle]
pub unsafe extern "C" fn adrift_pointer(shell: *mut Shell, id: u64, phase: u32, x: f64, y: f64, t: f64) {
    let Some(s) = shell.as_mut() else { return };
    let phase = match phase {
        0 => Phase::Down,
        1 => Phase::Move,
        2 => Phase::Up,
        _ => Phase::Cancel,
    };
    s.app.pointer(PointerEvent { id, phase, x, y, t });
}

/// The device was shaken: undo (touch/VOCABULARY.md).
#[no_mangle]
pub unsafe extern "C" fn adrift_shaken(shell: *mut Shell) {
    if let Some(s) = shell.as_mut() {
        s.app.shaken();
    }
}

/// Whether a Put or a Hold happened since the shell last asked (it plays a light haptic: the beat is
/// felt as well as seen); cleared by the asking.
#[no_mangle]
pub unsafe extern "C" fn adrift_take_haptic(shell: *mut Shell) -> bool {
    match shell.as_mut() {
        Some(s) => std::mem::take(&mut s.app.haptic),
        None => false,
    }
}

/// Where the state is kept between launches (a file in the app's documents): read now, written by
/// `adrift_pause`.
#[no_mangle]
pub unsafe extern "C" fn adrift_store(shell: *mut Shell, path: *const c_char) {
    let Some(s) = shell.as_mut() else { return };
    let p = std::path::PathBuf::from(CStr::from_ptr(path).to_string_lossy().into_owned());
    if let Ok(json) = std::fs::read_to_string(&p) {
        s.app.restore(&json);
    }
    s.store = Some(p);
}

/// The app is going to the background: the state is written.
#[no_mangle]
pub unsafe extern "C" fn adrift_pause(shell: *mut Shell) {
    let Some(s) = shell.as_mut() else { return };
    if let Some(p) = &s.store {
        if let Err(e) = std::fs::write(p, s.app.snapshot()) {
            eprintln!("adrift: store: {e}");
        }
    }
}

/// The system's appearance: dark is the look's night.
#[no_mangle]
pub unsafe extern "C" fn adrift_set_night(shell: *mut Shell, on: bool) {
    if let Some(s) = shell.as_mut() {
        s.app.night = on;
    }
}

#[no_mangle]
pub unsafe extern "C" fn adrift_set_reduce_motion(shell: *mut Shell, on: bool) {
    if let Some(s) = shell.as_mut() {
        s.app.reduce_motion = on;
    }
}

/// Whether the reader asked to export the newest take since the shell last asked (R9); cleared by
/// the asking. The shell then runs `adrift_export_begin`, `_frame` until false, `_end`.
#[no_mangle]
pub unsafe extern "C" fn adrift_take_export(shell: *mut Shell) -> bool {
    match shell.as_mut() {
        Some(s) => s.app.take_export_wanted(),
        None => false,
    }
}

/// Begins exporting the newest take at `fps`: the frames to come (0: nothing to export), and the
/// video's size in pixels written to `width`, `height` (even sides).
#[no_mangle]
pub unsafe extern "C" fn adrift_export_begin(shell: *mut Shell, fps: f64, width: *mut u32, height: *mut u32) -> u32 {
    let Some(s) = shell.as_mut() else { return 0 };
    let frames = s.app.export_begin(None, fps);
    if frames == 0 {
        return 0;
    }
    let (tw, th) = s.app.export_target_size().unwrap_or((2, 2));
    s.export = Some(crate::gpu::Offscreen::new(&s.app.gpu, tw, th));
    let (w, h) = s.app.export_size().unwrap_or((2, 2));
    if let Some(p) = width.as_mut() {
        *p = w;
    }
    if let Some(p) = height.as_mut() {
        *p = h;
    }
    frames
}

/// Renders the export's next frame into `out`: BGRA, rows top down, `stride` bytes apart (a
/// CVPixelBuffer's bytes per row). False when the export is done.
#[no_mangle]
pub unsafe extern "C" fn adrift_export_frame(shell: *mut Shell, out: *mut u8, stride: usize) -> bool {
    let Some(s) = shell.as_mut() else { return false };
    let Some(target) = s.export.as_ref() else { return false };
    let Some((w, h)) = s.app.export_size() else { return false };
    let Some(px) = s.app.export_frame(target) else { return false };
    let row = w as usize * 4;
    for y in 0..h as usize {
        std::ptr::copy_nonoverlapping(px.as_ptr().add(y * row), out.add(y * stride), row);
    }
    true
}

/// Ends an export: the live app put back as it was.
#[no_mangle]
pub unsafe extern "C" fn adrift_export_end(shell: *mut Shell) {
    let Some(s) = shell.as_mut() else { return };
    s.app.export_end();
    s.export = None;
}

/// The panel's marks for VoiceOver (R10): how many there are now.
#[no_mangle]
pub unsafe extern "C" fn adrift_panel_count(shell: *mut Shell) -> u32 {
    match shell.as_ref() {
        Some(s) => s.app.panel_bounds().len() as u32,
        None => 0,
    }
}

/// The panel's `i`th mark: its rect in points written to `rect` (x, y, w, h) and its words, UTF-8 and
/// NUL-terminated, into `words` (at most `len` bytes). False past the last.
#[no_mangle]
pub unsafe extern "C" fn adrift_panel_item(shell: *mut Shell, i: u32, rect: *mut f32, words: *mut u8, len: usize) -> bool {
    let Some(s) = shell.as_ref() else { return false };
    let bounds = s.app.panel_bounds();
    let Some((id, r)) = bounds.get(i as usize) else { return false };
    if !rect.is_null() {
        std::ptr::copy_nonoverlapping(r.as_ptr(), rect, 4);
    }
    if !words.is_null() && len > 0 {
        let text = s.app.panel_words().into_iter().find(|(w, _)| w == id).map(|(_, t)| t).unwrap_or_else(|| format!("Lamp {id}"));
        let b = text.as_bytes();
        let n = b.len().min(len - 1);
        std::ptr::copy_nonoverlapping(b.as_ptr(), words, n);
        *words.add(n) = 0;
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn adrift_free(shell: *mut Shell) {
    if !shell.is_null() {
        drop(Box::from_raw(shell));
    }
}

/// Runs a future that is ready at once or after the Metal backend's synchronous work (wgpu's Metal
/// adapter and device requests complete without a reactor).
fn pollster_lite<F: std::future::Future>(f: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}
