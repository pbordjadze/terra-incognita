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
    /// Its frames in flight (R24).
    readback: Option<crate::gpu::Readback>,
}

/// Makes the app over a CAMetalLayer. `scale` is the screen's pixels a point; `assets` the bundle's
/// resource directory. Null on failure (the reason is logged).
#[no_mangle]
pub unsafe extern "C" fn adrift_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut Shell {
    let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::METAL, ..wgpu::InstanceDescriptor::new_without_display_handle() });
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
        color_space: wgpu::SurfaceColorSpace::Srgb,
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
    Box::into_raw(Box::new(Shell { _instance: instance, surface, config, app, store: None, stats_at: 0.0, export: None, readback: None }))
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
        wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            s.surface.configure(&s.app.gpu.device, &s.config);
            return;
        }
        other => {
            eprintln!("adrift: frame: {other:?}");
            return;
        }
    };
    let view = frame.texture.create_view(&Default::default());
    s.app.frame(&view, t);
    s.app.gpu.queue.present(frame);
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
    // (R24) while an export renders, a touch does nothing to the world (the clip is the take's)
    if s.export.is_some() {
        return;
    }
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
    s.readback = None;
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
    // (R24) two frames in flight: the next is drawn while the last is read back, straight into the
    // encoder's buffer, BGRA as it is drawn (no allocation, no swizzle on the phone)
    let rb = s.readback.get_or_insert_with(|| crate::gpu::Readback::new(&s.app.gpu, target, 2));
    while rb.has_room() {
        match s.app.export_render(target) {
            Some(true) => rb.submit(&s.app.gpu, target),
            Some(false) => {}
            None => break,
        }
    }
    let _ = w;
    let rows = std::slice::from_raw_parts_mut(out, stride * h as usize);
    rb.take(&s.app.gpu, rows, stride, true)
}

/// Ends an export: the live app put back as it was.
#[no_mangle]
pub unsafe extern "C" fn adrift_export_end(shell: *mut Shell) {
    let Some(s) = shell.as_mut() else { return };
    s.app.export_end();
    s.export = None;
    s.readback = None;
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

/// The song the shell plays from its start while a take records (R12), by a name (UTF-8,
/// NUL-terminated), or none (null).
#[no_mangle]
pub unsafe extern "C" fn adrift_set_song(shell: *mut Shell, name: *const std::os::raw::c_char) {
    let Some(s) = shell.as_mut() else { return };
    let name = if name.is_null() { None } else { Some(std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned()) };
    s.app.set_song(name);
}

/// Whether a take is being recorded now (R12: the shell starts its song when this turns true and
/// stops it when it turns false).
#[no_mangle]
pub unsafe extern "C" fn adrift_recording(shell: *mut Shell) -> bool {
    shell.as_ref().is_some_and(|s| s.app.recording())
}

/// While exporting: whether the take had a song, and if so where it begins in the clip and how long
/// it runs, seconds, written to `start` and `length` (R12).
#[no_mangle]
pub unsafe extern "C" fn adrift_export_song(shell: *mut Shell, start: *mut f64, length: *mut f64) -> bool {
    let Some(s) = shell.as_ref() else { return false };
    let Some((a, n)) = s.app.export_song() else { return false };
    if let Some(p) = start.as_mut() {
        *p = a;
    }
    if let Some(p) = length.as_mut() {
        *p = n;
    }
    true
}

/// Whether the studio's menu was asked for since last asked (R13), and what it can offer: how many
/// lamps a journey clip or a copy would take (`picked`) and how many takes are kept (`takes`).
#[no_mangle]
pub unsafe extern "C" fn adrift_menu(shell: *mut Shell, picked: *mut u32, takes: *mut u32, chosen: *mut u32, kept: *mut u32) -> bool {
    let Some(s) = shell.as_mut() else { return false };
    if !s.app.menu_wanted() {
        return false;
    }
    let (c, k) = s.app.chosen_kept();
    if let Some(p) = chosen.as_mut() {
        *p = c as u32;
    }
    if let Some(p) = kept.as_mut() {
        *p = k as u32;
    }
    if let Some(p) = picked.as_mut() {
        *p = s.app.picked_lamps().len() as u32;
    }
    if let Some(p) = takes.as_mut() {
        *p = s.app.studio.takes.len() as u32;
    }
    true
}

/// What a put makes (R21): 0 a lamp on the sea, 1 a seed on the wind.
#[no_mangle]
pub unsafe extern "C" fn adrift_medium(shell: *mut Shell) -> u32 {
    shell.as_mut().map_or(0, |s| match s.app.medium {
        crate::model::Medium::Sea => 0,
        crate::model::Medium::Wind => 1,
    })
}

/// Sets what a put makes (R21; 0 the sea, 1 the wind), as an intent (a take records it).
#[no_mangle]
pub unsafe extern "C" fn adrift_set_medium(shell: *mut Shell, medium: u32) {
    if let Some(s) = shell.as_mut() {
        let medium = if medium == 1 { crate::model::Medium::Wind } else { crate::model::Medium::Sea };
        s.app.command(crate::model::Intent::Medium { medium });
    }
}

/// The chosen lamps kept for good (`on`) or let go (R13); how many changed.
#[no_mangle]
pub unsafe extern "C" fn adrift_keep(shell: *mut Shell, on: bool) -> u32 {
    shell.as_mut().map_or(0, |s| s.app.keep_chosen(on) as u32)
}

/// Begins exporting a journey clip of the picked lamps, `seconds` long, at `fps` (R13); the frames
/// to come (0: nothing to make), the video's size written to `width`, `height`. Then
/// `adrift_export_frame` until false and `adrift_export_end`, as for a take.
#[no_mangle]
pub unsafe extern "C" fn adrift_journey_begin(shell: *mut Shell, seconds: f64, fps: f64, width: *mut u32, height: *mut u32) -> u32 {
    adrift_journey_begin_edit(shell, seconds, f64::NAN, f64::NAN, f64::NAN, fps, width, height)
}

/// The edit view's days (R23): the first day the picked lamps are all out, the app's date, and the
/// days a journey of `seconds` shows by default; false when no lamp is picked.
#[no_mangle]
pub unsafe extern "C" fn adrift_journey_bounds(shell: *mut Shell, seconds: f64, born: *mut f64, now: *mut f64, from: *mut f64, to: *mut f64) -> bool {
    let Some(s) = shell.as_mut() else { return false };
    let Some((b, n, f, t)) = s.app.journey_bounds(seconds) else { return false };
    for (p, v) in [(born, b), (now, n), (from, f), (to, t)] {
        if let Some(p) = p.as_mut() {
            *p = v;
        }
    }
    true
}

/// The edit view's preview (R23): begin (false if nothing to show), show a day, play between days at
/// a pace, and end (the live state put back).
#[no_mangle]
pub unsafe extern "C" fn adrift_preview_begin(shell: *mut Shell) -> bool {
    shell.as_mut().is_some_and(|s| s.app.preview_begin())
}

#[no_mangle]
pub unsafe extern "C" fn adrift_preview_at(shell: *mut Shell, day: f64) {
    if let Some(s) = shell.as_mut() {
        s.app.preview_at(day);
    }
}

#[no_mangle]
pub unsafe extern "C" fn adrift_preview_play(shell: *mut Shell, from: f64, to: f64, pace: f64) {
    if let Some(s) = shell.as_mut() {
        s.app.preview_play(from, to, pace);
    }
}

#[no_mangle]
pub unsafe extern "C" fn adrift_preview_end(shell: *mut Shell) {
    if let Some(s) = shell.as_mut() {
        s.app.preview_end();
    }
}

/// The app's date (R23: the edit view's playhead while a preview plays).
#[no_mangle]
pub unsafe extern "C" fn adrift_day(shell: *mut Shell) -> f64 {
    shell.as_mut().map_or(0.0, |s| s.app.clock.day)
}

/// Begins exporting a journey clip as the edit view sets it (R22): its first and last days and its
/// pace (days a second), each NaN for the app's own choice; the frames to come, the size as above.
#[no_mangle]
pub unsafe extern "C" fn adrift_journey_begin_edit(shell: *mut Shell, seconds: f64, from: f64, to: f64, pace: f64, fps: f64, width: *mut u32, height: *mut u32) -> u32 {
    let Some(s) = shell.as_mut() else { return 0 };
    let some = |x: f64| (!x.is_nan()).then_some(x);
    let edit = crate::app::JourneyEdit { seconds, from: some(from), to: some(to), pace: some(pace) };
    let frames = s.app.journey_begin_edit(&edit, fps);
    if frames == 0 {
        return 0;
    }
    let (tw, th) = s.app.export_target_size().unwrap_or((2, 2));
    s.export = Some(crate::gpu::Offscreen::new(&s.app.gpu, tw, th));
    s.readback = None;
    let (w, h) = s.app.export_size().unwrap_or((2, 2));
    if let Some(p) = width.as_mut() {
        *p = w;
    }
    if let Some(p) = height.as_mut() {
        *p = h;
    }
    frames
}

/// The picked lamps as text to copy (R13), UTF-8 and NUL-terminated into `out` (at most `len`
/// bytes); returns the bytes the whole text needs (without the NUL), 0 for no lamps. Call with a
/// null `out` to learn the size.
#[no_mangle]
pub unsafe extern "C" fn adrift_lamps_text(shell: *mut Shell, out: *mut u8, len: usize) -> usize {
    let Some(s) = shell.as_ref() else { return 0 };
    let Some(text) = s.app.lamps_text() else { return 0 };
    let b = text.as_bytes();
    if !out.is_null() && len > b.len() {
        std::ptr::copy_nonoverlapping(b.as_ptr(), out, b.len());
        *out.add(b.len()) = 0;
    }
    b.len()
}

/// Lamps pasted from copied text (R13); how many came (0: the text isn't Adrift's lamps). The date
/// goes to the day they were copied on, so they are where they were.
#[no_mangle]
pub unsafe extern "C" fn adrift_lamps_paste(shell: *mut Shell, text: *const std::os::raw::c_char) -> u32 {
    let Some(s) = shell.as_mut() else { return 0 };
    if text.is_null() {
        return 0;
    }
    let t = std::ffi::CStr::from_ptr(text).to_string_lossy();
    s.app.paste_lamps(&t, true) as u32
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
