//! toy-kit: what every toy stands on (toys/README.md). A toy is one type implementing [`Toy`]; the
//! kit runs it on the device (`ios_shell!`, the C ABI the generic Swift shell in `toys/ios` calls),
//! headless on Linux (`check::main`, plans played to PNG and WAV on Mesa's software Vulkan), in a
//! browser (`web::run`, WebGPU; the `web` feature, wasm32), and gives it the GPU, the data, the real
//! globe by day or night (`globe`; the `globe` feature, on by default), a painter of soft 2D shapes
//! (`paint`), a full-screen backdrop (`backdrop`), persistent layers (`layer`) and a sound out
//! (`Synth`, `sound::Bells`).
//!
//! The GPU, the geometry and the ground were copied from adrift-core (Nov 2, 2026) so that Adrift's
//! rounds never break a toy, and a toy's never break Adrift: change them here freely, for every toy.
//! `paint`, `backdrop`, `layer`, `sound`, `web` and `check::main_with` came from pbordjadze/busquets'
//! copy of the kit (Nov 3, 2026; same author, MIT). Other repositories take the kit as a git
//! dependency pinned to a `toy-kit-v*` tag (CHANGES.md).

pub mod assets;
pub mod backdrop;
pub mod budget;
#[cfg(not(target_arch = "wasm32"))]
pub mod check;
#[cfg(feature = "globe")]
pub mod globe;
pub mod gpu;
pub mod info;
pub mod layer;
pub mod model;
pub mod paint;
#[cfg(target_os = "ios")]
pub mod shell;
pub mod sound;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;

pub use assets::{Assets, DirAssets, MemoryAssets};
#[cfg(feature = "globe")]
pub use globe::Globe;
pub use gpu::{Gpu, Offscreen};
pub use model::*;

use std::sync::{Arc, Mutex, OnceLock};

static LOG: OnceLock<std::path::PathBuf> = OnceLock::new();

/// Where the device keeps the toy's log (`<Documents>/toy-log.txt`, visible in the Files app): the
/// shell sets it before the toy starts. A panic's message and place, a GPU error, a toy that could not
/// start are written there, so a crash on a phone can be read without a Mac.
pub fn set_log(path: std::path::PathBuf) {
    if LOG.set(path).is_ok() {
        std::panic::set_hook(Box::new(|info| {
            let at = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
            let msg = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_default();
            log(&format!("panic at {at}: {msg}"));
        }));
    }
}

/// One line in the device's log (and stderr), stamped with seconds since 1970.
pub fn log(line: &str) {
    eprintln!("toy: {line}");
    if let Some(p) = LOG.get() {
        use std::io::Write;
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
            let _ = writeln!(f, "{t} {line}");
        }
    }
}

/// A launch step in the device's log, with the memory the app may still take there (iOS's
/// `os_proc_available_memory`, what the system kills on), so a toy that closes while it starts says
/// how far it got and how much room was left: the last line in `toy-log.txt` is the step it died in.
/// A toy may call it between its own heavy steps. Off the device it is a line on stderr only when
/// `KIT_STEPS=1`.
pub fn step(what: &str) {
    #[cfg(target_os = "ios")]
    log(&format!("launch: {what}; {} MB available", shell::available_mb()));
    #[cfg(not(target_os = "ios"))]
    if std::env::var("KIT_STEPS").is_ok_and(|v| v == "1") {
        eprintln!("toy: launch: {what}");
    }
}

/// What a toy is given: the device, its data, the screen, the reader's settings, and a way back to
/// the shell (a haptic tap).
pub struct Ctx {
    pub gpu: Gpu,
    pub assets: Box<dyn Assets>,
    /// The target's size in pixels, and pixels a point.
    pub width: u32,
    pub height: u32,
    pub ppp: f64,
    /// The system's dark appearance.
    pub night: bool,
    /// The reader's Reduce Motion.
    pub reduce_motion: bool,
    haptic: bool,
}

impl Ctx {
    pub fn new(gpu: Gpu, assets: Box<dyn Assets>, width: u32, height: u32, ppp: f64) -> Ctx {
        Ctx { gpu, assets, width, height, ppp, night: false, reduce_motion: false, haptic: false }
    }

    /// Asks the shell for one light haptic tap this frame.
    pub fn haptic(&mut self) {
        self.haptic = true;
    }

    /// The screen's size in points.
    pub fn points(&self) -> [f64; 2] {
        [self.width as f64 / self.ppp, self.height as f64 / self.ppp]
    }
}

/// One toy. Everything it shows it draws itself into the target, every pixel (there is no chrome
/// around it); everything it hears is a finger.
pub trait Toy: 'static {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self>
    where
        Self: Sized;
    /// The screen changed size (`ctx.width`, `height`, `ppp` are the new ones).
    fn resize(&mut self, _ctx: &mut Ctx) {}
    /// A finger, in points from the top left.
    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent);
    /// Time moves: `t` the shell's seconds, `dt` since the last frame (capped at 0.1 s). Simulation
    /// lives here (GPU compute too), never in `draw`, so a headless check that draws only some frames
    /// sees the same world.
    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64);
    /// Draws the frame into `target` (format `ctx.gpu.format`), every pixel.
    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView);
    /// The toy's sound, once, at start: it runs on the audio thread from then on (talk to it through
    /// an `Arc` of atomics or a `Mutex` the toy keeps a clone of). None: a silent toy.
    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        None
    }
    /// The device was shaken.
    fn shaken(&mut self, _ctx: &mut Ctx) {}
    /// The appearance or Reduce Motion changed (read `ctx.night`, `ctx.reduce_motion`).
    fn settings(&mut self, _ctx: &mut Ctx) {}
    /// What to keep when the app goes to the background (any text; JSON is usual), and to take back.
    fn save(&self) -> Option<String> {
        None
    }
    fn restore(&mut self, _ctx: &mut Ctx, _saved: &str) {}
    /// The places the toy names, in points from the top left, as they are now (a plan may tap
    /// `"whale"`: check.rs). None by default.
    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        Vec::new()
    }
    /// Where a longitude and latitude are on screen now, in points, if the toy has a globe and the
    /// place faces the reader (a plan may tap `{"lonlat": [lon, lat]}`). A toy with a globe returns
    /// `self.globe.point(lonlat)`. None by default.
    fn project(&self, _lonlat: [f64; 2]) -> Option<[f64; 2]> {
        None
    }
    /// What the info mark shows (`info`): the app's one gesture as a ghost finger on the live scene,
    /// and at most one line. None (the default) shows no mark. Asked when the mark is tapped, so the
    /// strokes may aim at where things are now (a city, a whale).
    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        None
    }
    /// What the rules mark shows (`info`): the few rules that make what is on screen, and one line of
    /// what is measured and what is drawn. None (the default) shows no rules mark.
    fn rules(&self) -> Option<info::Rules> {
        None
    }
}

/// A toy's sound: fills interleaved samples (`channels` of them a frame, at `rate` Hz), -1..1.
pub trait Synth: Send {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64);
}

pub type SharedSynth = Arc<Mutex<Box<dyn Synth>>>;

/// A toy and its context, as every shell drives it.
pub struct Host<T: Toy> {
    pub ctx: Ctx,
    pub toy: T,
    pub synth: Option<SharedSynth>,
    last_t: Option<f64>,
    /// The info mark and its demo.
    pub info: info::Info,
    has_demo: bool,
    has_rules: bool,
}

impl<T: Toy> Host<T> {
    pub fn new(mut ctx: Ctx) -> anyhow::Result<Host<T>> {
        step("the toy starting");
        let mut toy = T::new(&mut ctx)?;
        let synth = toy.synth().map(|s| Arc::new(Mutex::new(s)));
        let has_demo = toy.demo(&ctx).is_some();
        let has_rules = toy.rules().is_some();
        Ok(Host { ctx, toy, synth, last_t: None, info: Default::default(), has_demo, has_rules })
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.ctx.width, self.ctx.height, self.ctx.ppp) = (width.max(1), height.max(1), ppp);
        self.toy.resize(&mut self.ctx);
    }

    /// A real finger: the info mark's (and the tap that closes its demo), or the toy's.
    pub fn pointer(&mut self, e: PointerEvent) {
        match self.info.route(&e, self.ctx.points(), self.has_demo, self.has_rules) {
            info::Route::Toy => self.toy.pointer(&mut self.ctx, e),
            info::Route::Taken => {}
            info::Route::Open => self.open_info(e.t),
            info::Route::Rules => self.open_rules(),
        }
    }

    /// Shows the rules (a tap on the rules mark, or VoiceOver's activation of it), or, if they show,
    /// hides them.
    pub fn open_rules(&mut self) {
        let r = if self.info.rules().is_some() { None } else { self.toy.rules() };
        self.info.show_rules(r);
    }

    /// The rules mark's place in points, if the toy has one.
    pub fn rules_spot(&self) -> Option<[f64; 2]> {
        self.has_rules.then(|| info::Info::rules_spot(self.ctx.points()))
    }

    /// Opens the info mark's demo (a tap on the mark, or VoiceOver's activation of it).
    pub fn open_info(&mut self, t: f64) {
        if let Some(d) = self.toy.demo(&self.ctx) {
            self.info.open(d, t);
        }
    }

    /// The info mark's place in points, if the toy has one.
    pub fn info_spot(&self) -> Option<[f64; 2]> {
        self.has_demo.then(|| info::Info::spot(self.ctx.points()))
    }

    /// The toy's named places, and the info mark's as `"info"`.
    pub fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let mut a = self.toy.anchors();
        if let Some(s) = self.info_spot() {
            a.push(("info".into(), s));
        }
        if let Some(s) = self.rules_spot() {
            a.push(("rules".into(), s));
        }
        a
    }

    /// Time to `t`; the first call only sets the clock.
    pub fn update(&mut self, t: f64) {
        let dt = match self.last_t {
            Some(l) => (t - l).clamp(0.0, 0.1),
            None => 0.0,
        };
        self.last_t = Some(t);
        // the demo's ghost finger: real touches, before the toy's step
        for g in self.info.advance(t, self.ctx.reduce_motion) {
            self.toy.pointer(&mut self.ctx, g);
        }
        self.toy.update(&mut self.ctx, t, dt);
    }

    pub fn draw(&mut self, target: &wgpu::TextureView) {
        self.toy.draw(&mut self.ctx, target);
        self.info.draw(&self.ctx.gpu, target, [self.ctx.width, self.ctx.height], self.ctx.ppp, self.has_demo, self.has_rules, self.ctx.reduce_motion);
    }

    /// Forgets the clock (after the app was away: the next frame's dt is 0, not the time away).
    pub fn pause(&mut self) {
        self.last_t = None;
    }

    pub fn set_night(&mut self, on: bool) {
        self.ctx.night = on;
        self.toy.settings(&mut self.ctx);
    }

    pub fn set_reduce_motion(&mut self, on: bool) {
        self.ctx.reduce_motion = on;
        self.toy.settings(&mut self.ctx);
    }

    /// Whether the toy asked for a haptic tap since the last call.
    pub fn take_haptic(&mut self) -> bool {
        std::mem::take(&mut self.ctx.haptic)
    }

    /// Sound for the next `out.len() / channels` frames (silence without a synth or while locked).
    pub fn audio(synth: Option<&SharedSynth>, out: &mut [f32], channels: usize, rate: f64) {
        out.fill(0.0);
        if let Some(s) = synth {
            if let Ok(mut s) = s.try_lock() {
                s.render(out, channels, rate);
            }
        }
    }
}

/// A small executor for wgpu's futures (they are ready at once, or nearly).
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The C ABI over a toy type: `toy_kit::ios_shell!(MyToy);` in the toy's `lib.rs`. Compiled only for
/// iOS (the static library the shell links).
#[macro_export]
macro_rules! ios_shell {
    ($toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            use super::*;
            use std::ffi::{c_char, c_void, CStr};
            type S = $crate::shell::Shell<$toy>;

            /// Where to keep the log (call before `toy_new`).
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_log(path: *const c_char) {
                $crate::set_log(CStr::from_ptr(path).to_string_lossy().into_owned().into());
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut S {
                let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
                match S::new(layer, width, height, scale, &dir) {
                    Some(s) => Box::into_raw(Box::new(s)),
                    None => std::ptr::null_mut(),
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_resize(s: *mut S, width: u32, height: u32, scale: f64) {
                if let Some(s) = s.as_mut() { s.resize(width, height, scale) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_frame(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.frame(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pointer(s: *mut S, id: u64, phase: u32, x: f64, y: f64, t: f64) {
                if let Some(s) = s.as_mut() { s.pointer(id, phase, x, y, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_shaken(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.toy.shaken(&mut s.host.ctx) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_take_haptic(s: *mut S) -> bool {
                s.as_mut().map(|s| s.host.take_haptic()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_night(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_night(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_set_reduce_motion(s: *mut S, on: bool) {
                if let Some(s) = s.as_mut() { s.host.set_reduce_motion(on) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_store(s: *mut S, path: *const c_char) {
                if let Some(s) = s.as_mut() { s.store(&CStr::from_ptr(path).to_string_lossy()) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_pause(s: *mut S) {
                if let Some(s) = s.as_mut() { s.pause() }
            }
            /// The toy's sound, for the audio thread (null: silent). Freed by `toy_synth_free`.
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth(s: *mut S) -> *mut $crate::SharedSynth {
                match s.as_ref().and_then(|s| s.synth()) {
                    Some(a) => Box::into_raw(Box::new(a)),
                    None => std::ptr::null_mut(),
                }
            }
            /// Fills `frames` interleaved frames of `channels` floats at `rate` Hz (the audio thread).
            #[no_mangle]
            pub unsafe extern "C" fn toy_audio(synth: *mut $crate::SharedSynth, out: *mut f32, frames: u32, channels: u32, rate: f64) {
                let out = std::slice::from_raw_parts_mut(out, (frames * channels) as usize);
                $crate::Host::<$toy>::audio(synth.as_ref(), out, channels as usize, rate);
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_synth_free(synth: *mut $crate::SharedSynth) {
                if !synth.is_null() { drop(Box::from_raw(synth)) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.info_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_line(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.line()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_info_open(s: *mut S, t: f64) {
                if let Some(s) = s.as_mut() { s.host.open_info(t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.rules_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_text(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.rules()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_rules_open(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.open_rules() }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_free(s: *mut S) {
                if !s.is_null() { drop(Box::from_raw(s)) }
            }
        }
    };
}
