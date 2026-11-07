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
            let at = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default();
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
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
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
    log(&format!(
        "launch: {what}; {} MB available",
        shell::available_mb()
    ));
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
        Ctx {
            gpu,
            assets,
            width,
            height,
            ppp,
            night: false,
            reduce_motion: false,
            haptic: false,
        }
    }

    /// Asks the shell for one light haptic tap this frame.
    pub fn haptic(&mut self) {
        self.haptic = true;
    }

    /// Whether a haptic tap was asked for since the last call, forgetting it (a host passes on the
    /// taps of a toy it holds: `toys/terra`).
    pub fn take_haptic(&mut self) -> bool {
        std::mem::take(&mut self.haptic)
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
    /// What the origins mark shows (`info`): where the rules came from (the real science and
    /// history behind them), then a dimmer note of what is real and what is drawn. None (the default)
    /// shows no origins mark.
    fn origins(&self) -> Option<info::Rules> {
        None
    }
    /// Whether the shell hangs the rules' and origins' card from the top of the screen, not over the
    /// marks at the foot (false, the default), so it keeps clear of what matters in the scene. Asked
    /// as a card opens, so it may look at where things are now.
    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        false
    }
    /// Things on screen a finger can touch that VoiceOver should offer as buttons: a label and where
    /// it is, in points (Terra's lights, each opening its toy). None by default: a toy's scene is
    /// touched, not pressed.
    fn buttons(&self, _ctx: &Ctx) -> Vec<(String, [f64; 2])> {
        Vec::new()
    }
    /// VoiceOver activated `buttons()[i]` (as it was this frame); `t` the shell's seconds.
    fn press(&mut self, _ctx: &mut Ctx, _i: usize, _t: f64) {}
    /// Whether the toy asks the reader for a sound file of their own now (Screw's sleeve was touched):
    /// asked once a frame, so return true once per ask. The shell shows the Files picker, decodes what is
    /// chosen and hands it to `opened`; it keeps the file in the app's own storage and hands it again at
    /// every launch, until another is chosen. Nothing is fetched or bundled: the file is the reader's.
    fn wants_file(&mut self) -> bool {
        false
    }
    /// A sound file the reader chose (`wants_file`), decoded: on the device by AVAudioFile (any format
    /// iOS plays), in a plan by `"open": "<file.wav>"` (check.rs).
    fn opened(&mut self, _ctx: &mut Ctx, _file: AudioFile) {}
    /// Whether the toy's sound is music the reader asked for (Screw's record), so the shell plays it in the
    /// playback category: heard with the silent switch on, other apps' audio paused. False (the default):
    /// the ambient category, which the silent switch silences and other apps' audio plays over.
    fn plays_music(&self) -> bool {
        false
    }
    /// One quiet line the shell sets at the top in the system's text (VoiceOver reads it): Screw's
    /// song's name. None (the default): nothing.
    fn title(&self) -> Option<String> {
        None
    }
}

/// A decoded sound file (`Toy::opened`): its name (the file's, without its extension), its sample rate
/// and its two channels (a mono file's are the same).
pub struct AudioFile {
    pub name: String,
    pub rate: f64,
    pub left: Vec<f32>,
    pub right: Vec<f32>,
}

impl AudioFile {
    /// A WAV file (PCM 8, 16, 24 or 32 bit, or 32-bit float; any channels, the first two kept).
    pub fn wav(path: &std::path::Path) -> anyhow::Result<AudioFile> {
        let b = std::fs::read(path)?;
        anyhow::ensure!(
            b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WAVE",
            "{}: not a WAV file",
            path.display()
        );
        let (mut fmt, mut data) = (None, None);
        let mut i = 12;
        while i + 8 <= b.len() {
            let n = u32::from_le_bytes(b[i + 4..i + 8].try_into().unwrap()) as usize;
            let body = &b[i + 8..(i + 8 + n).min(b.len())];
            match &b[i..i + 4] {
                b"fmt " => fmt = Some(body),
                b"data" => data = Some(body),
                _ => {}
            }
            i += 8 + n + (n & 1);
        }
        let (f, d) = (
            fmt.ok_or_else(|| anyhow::anyhow!("no fmt chunk"))?,
            data.ok_or_else(|| anyhow::anyhow!("no data chunk"))?,
        );
        anyhow::ensure!(f.len() >= 16, "short fmt chunk");
        let tag = u16::from_le_bytes([f[0], f[1]]);
        let ch = u16::from_le_bytes([f[2], f[3]]).max(1) as usize;
        let rate = u32::from_le_bytes(f[4..8].try_into().unwrap()) as f64;
        let bits = u16::from_le_bytes([f[14], f[15]]) as usize;
        let tag = if tag == 0xFFFE && f.len() >= 26 {
            u16::from_le_bytes([f[24], f[25]])
        } else {
            tag
        };
        let w = bits / 8;
        anyhow::ensure!(w > 0, "zero-width samples");
        let sample = |s: &[u8]| -> f32 {
            match (tag, bits) {
                (3, 32) => f32::from_le_bytes(s.try_into().unwrap()),
                (_, 8) => (s[0] as f32 - 128.0) / 128.0,
                (_, 16) => i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0,
                (_, 24) => (i32::from_le_bytes([0, s[0], s[1], s[2]]) >> 8) as f32 / 8388608.0,
                (_, 32) => i32::from_le_bytes(s.try_into().unwrap()) as f32 / 2147483648.0,
                _ => 0.0,
            }
        };
        let frames = d.len() / (w * ch);
        let (mut left, mut right) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
        for k in 0..frames {
            let at = k * w * ch;
            left.push(sample(&d[at..at + w]));
            right.push(if ch > 1 {
                sample(&d[at + w..at + 2 * w])
            } else {
                sample(&d[at..at + w])
            });
        }
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(AudioFile {
            name,
            rate,
            left,
            right,
        })
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
    marks: info::Marks,
    /// The toy's buttons for VoiceOver, as the shell reads them (`buttons_text`).
    buttons: std::ffi::CString,
    /// The toy's title as the shell reads it (`title_text`).
    title: std::ffi::CString,
}

impl<T: Toy> Host<T> {
    pub fn new(mut ctx: Ctx) -> anyhow::Result<Host<T>> {
        step("the toy starting");
        let mut toy = T::new(&mut ctx)?;
        let synth = toy.synth().map(|s| Arc::new(Mutex::new(s)));
        let marks = info::Marks {
            demo: toy.demo(&ctx).is_some(),
            rules: toy.rules().is_some(),
            origins: toy.origins().is_some(),
        };
        Ok(Host {
            ctx,
            toy,
            synth,
            last_t: None,
            info: Default::default(),
            marks,
            buttons: Default::default(),
            title: Default::default(),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.ctx.width, self.ctx.height, self.ctx.ppp) = (width.max(1), height.max(1), ppp);
        self.toy.resize(&mut self.ctx);
    }

    /// A real finger: the info mark's (and the tap that closes its demo), or the toy's.
    pub fn pointer(&mut self, e: PointerEvent) {
        // a finger going down on one of the toy's own buttons (Terra's Earth mark, its lights) while a card
        // shows closes the card and is the button's too: one tap, not two (painting-company row 246: five
        // of six newcomers tapped Terra's Earth mark with the rules up, and it only closed them). A toy
        // without buttons keeps the rule that a closing tap is nobody's.
        if e.phase == Phase::Down && self.info.is_open() {
            let pts = self.toy.buttons(&self.ctx);
            if pts
                .iter()
                .any(|(_, p)| (e.x - p[0]).hypot(e.y - p[1]) <= info::Info::REACH)
            {
                self.info.close();
                self.toy.pointer(&mut self.ctx, e);
                return;
            }
        }
        match self.info.route(&e, self.ctx.points(), self.marks) {
            info::Route::Toy => self.toy.pointer(&mut self.ctx, e),
            info::Route::Taken => {}
            info::Route::Open => self.open_info(e.t),
            info::Route::Rules => self.open_rules(),
            info::Route::Origins => self.open_origins(),
        }
    }

    /// Shows where the rules came from (a tap on the origins mark, or VoiceOver's activation of it),
    /// or, if that shows, hides it.
    pub fn open_origins(&mut self) {
        let r = if self.info.origins().is_some() {
            None
        } else {
            self.toy.origins()
        };
        self.info.show_origins(r);
    }

    /// The origins mark's place in points, if the toy has one.
    pub fn origins_spot(&self) -> Option<[f64; 2]> {
        self.marks
            .origins
            .then(|| info::Info::origins_spot(self.ctx.points()))
    }

    /// Whether the shell hangs the card from the top (`Toy::card_at_top`).
    pub fn card_at_top(&self) -> bool {
        self.toy.card_at_top(&self.ctx)
    }

    /// Shows the rules (a tap on the rules mark, or VoiceOver's activation of it), or, if they show,
    /// hides them.
    pub fn open_rules(&mut self) {
        let r = if self.info.rules().is_some() {
            None
        } else {
            self.toy.rules()
        };
        self.info.show_rules(r);
    }

    /// The rules mark's place in points, if the toy has one.
    pub fn rules_spot(&self) -> Option<[f64; 2]> {
        self.marks
            .rules
            .then(|| info::Info::rules_spot(self.ctx.points()))
    }

    /// Opens the info mark's demo (a tap on the mark, or VoiceOver's activation of it).
    pub fn open_info(&mut self, t: f64) {
        if let Some(d) = self.toy.demo(&self.ctx) {
            self.info.open(d, t);
        }
    }

    /// The info mark's place in points, if the toy has one.
    pub fn info_spot(&self) -> Option<[f64; 2]> {
        self.marks.demo.then(|| info::Info::spot(self.ctx.points()))
    }

    /// The toy's buttons, a line each: its label, a tab, x, a tab, y (points); valid until the next call.
    pub fn buttons_text(&mut self) -> &std::ffi::CStr {
        let text: String = self
            .toy
            .buttons(&self.ctx)
            .into_iter()
            .map(|(label, p)| {
                format!(
                    "{}\t{:.1}\t{:.1}\n",
                    label.replace(['\t', '\n', '\0'], " "),
                    p[0],
                    p[1]
                )
            })
            .collect();
        self.buttons = std::ffi::CString::new(text).unwrap_or_default();
        &self.buttons
    }

    /// VoiceOver pressed button `i`.
    pub fn press(&mut self, i: usize, t: f64) {
        self.toy.press(&mut self.ctx, i, t);
    }

    /// Whether the toy asks for a sound file now (`Toy::wants_file`).
    pub fn wants_file(&mut self) -> bool {
        self.toy.wants_file()
    }

    /// Hands the toy the sound file the reader chose.
    pub fn opened(&mut self, file: AudioFile) {
        self.toy.opened(&mut self.ctx, file);
    }

    /// The toy's quiet line (`Toy::title`), valid until the next call.
    pub fn title_text(&mut self) -> Option<&std::ffi::CStr> {
        let t = self.toy.title()?;
        self.title = std::ffi::CString::new(t.replace('\0', " ")).unwrap_or_default();
        Some(&self.title)
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
        if let Some(s) = self.origins_spot() {
            a.push(("origins".into(), s));
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
        // the origins mark may come and go with what a host holds (Terra: Mold's, while Mold is open)
        self.marks.origins = self.toy.origins().is_some();
        // the demo's ghost finger: real touches, before the toy's step
        for g in self.info.advance(t, self.ctx.reduce_motion) {
            self.toy.pointer(&mut self.ctx, g);
        }
        self.toy.update(&mut self.ctx, t, dt);
    }

    pub fn draw(&mut self, target: &wgpu::TextureView) {
        self.toy.draw(&mut self.ctx, target);
        self.info.draw(
            &self.ctx.gpu,
            target,
            [self.ctx.width, self.ctx.height],
            self.ctx.ppp,
            self.marks,
            self.ctx.reduce_motion,
        );
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
///
/// A host links several toys into one library (Terra holds them all: `toys/terra`), so each toy's own
/// C ABI is left out when the kit's `hosted` feature is on (the host's crate turns it on; cargo then
/// builds the kit hosted for every toy in that build), and the host exports its own with
/// `ios_shell!(host MyHost)`. A toy's own app is built without it, as before.
#[macro_export]
macro_rules! ios_shell {
    (host $toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            $crate::ios_shell!(@body $toy);
        }
    };
    (@body $toy:ty) => {
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
            pub unsafe extern "C" fn toy_origins_spot(s: *mut S, x: *mut f64, y: *mut f64) -> bool {
                match s.as_ref().and_then(|s| s.host.origins_spot()) {
                    Some(p) => { *x = p[0]; *y = p[1]; true }
                    None => false,
                }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_origins_text(s: *mut S) -> *const c_char {
                s.as_ref().and_then(|s| s.host.info.origins()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_origins_open(s: *mut S) {
                if let Some(s) = s.as_mut() { s.host.open_origins() }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_card_at_top(s: *mut S) -> bool {
                s.as_ref().map(|s| s.host.card_at_top()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_buttons(s: *mut S) -> *const c_char {
                s.as_mut().map(|s| s.host.buttons_text().as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_button_press(s: *mut S, i: u32, t: f64) {
                if let Some(s) = s.as_mut() { s.host.press(i as usize, t) }
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_plays_music(s: *mut S) -> bool {
                s.as_ref().map(|s| s.host.toy.plays_music()).unwrap_or(false)
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_wants_file(s: *mut S) -> bool {
                s.as_mut().map(|s| s.host.wants_file()).unwrap_or(false)
            }
            /// A sound file the reader chose, decoded: `frames` samples a channel (`right` may be null
            /// for a mono file), at `rate` Hz; copied, so the shell frees its buffers after.
            #[no_mangle]
            pub unsafe extern "C" fn toy_opened_audio(s: *mut S, name: *const c_char, left: *const f32, right: *const f32, frames: u64, rate: f64) {
                let Some(s) = s.as_mut() else { return };
                if left.is_null() { return }
                let l = std::slice::from_raw_parts(left, frames as usize).to_vec();
                let r = if right.is_null() { l.clone() } else { std::slice::from_raw_parts(right, frames as usize).to_vec() };
                let name = CStr::from_ptr(name).to_string_lossy().into_owned();
                s.host.opened($crate::AudioFile { name, rate, left: l, right: r });
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_title(s: *mut S) -> *const c_char {
                s.as_mut().and_then(|s| s.host.title_text()).map(|l| l.as_ptr()).unwrap_or(std::ptr::null())
            }
            #[no_mangle]
            pub unsafe extern "C" fn toy_free(s: *mut S) {
                if !s.is_null() { drop(Box::from_raw(s)) }
            }
    };
    ($toy:ty) => {
        $crate::__own_shell!($toy);
    };
}

/// A toy's own C ABI (`ios_shell!(MyToy)`), unless the kit is built `hosted`.
#[cfg(not(feature = "hosted"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __own_shell {
    ($toy:ty) => {
        #[cfg(target_os = "ios")]
        pub mod ffi {
            $crate::ios_shell!(@body $toy);
        }
    };
}

/// Built `hosted` (a host such as Terra links several toys: their own C ABIs would collide), a toy's
/// `ios_shell!` exports nothing; the host's `ios_shell!(host MyHost)` does.
#[cfg(feature = "hosted")]
#[doc(hidden)]
#[macro_export]
macro_rules! __own_shell {
    ($toy:ty) => {};
}
