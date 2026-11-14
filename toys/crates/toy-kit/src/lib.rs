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
pub mod field;
#[cfg(feature = "globe")]
pub mod globe;
pub mod gpu;
pub mod info;
#[doc(hidden)]
pub mod ios;
pub mod layer;
pub mod model;
pub mod paint;
#[cfg(target_os = "ios")]
pub mod shell;
pub mod sound;
pub mod switches;
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

/// Seconds since 1970 by the wall clock. A browser's WebAssembly has no clock of its own (std's
/// `SystemTime::now` panics there), so on wasm32 the web shell sets it each frame (`set_wall_clock`).
pub fn wall_clock() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        f64::from_bits(WALL.load(std::sync::atomic::Ordering::Relaxed))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

#[cfg(target_arch = "wasm32")]
static WALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The page's clock (JavaScript's `Date.now() / 1000`), for `wall_clock` on wasm32.
#[cfg(target_arch = "wasm32")]
pub fn set_wall_clock(secs: f64) {
    WALL.store(secs.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// Where else the log's lines go: a browser's console (stderr goes nowhere in a browser's
/// WebAssembly; the web shell sets it).
static SINK: OnceLock<fn(&str)> = OnceLock::new();

/// Sends the log's lines to `sink` too (the web shell: the browser's console).
pub fn set_log_sink(sink: fn(&str)) {
    let _ = SINK.set(sink);
}

/// One line in the device's log (and stderr), stamped with seconds since 1970.
pub fn log(line: &str) {
    eprintln!("toy: {line}");
    if let Some(sink) = SINK.get() {
        sink(line);
    }
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
/// A toy may call it between its own heavy steps. Off the device: a line on stderr if `KIT_STEPS=1`.
pub fn step(what: &str) {
    #[cfg(target_os = "ios")]
    log(&format!(
        "launch: {what}; {} MB available",
        shell::available_mb()
    ));
    // (in a browser: its console, through the web shell's sink)
    #[cfg(target_arch = "wasm32")]
    log(&format!("launch: {what}"));
    #[cfg(not(any(target_os = "ios", target_arch = "wasm32")))]
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
    show_info: bool,
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
            show_info: false,
        }
    }

    /// Asks the shell for one light haptic tap this frame.
    pub fn haptic(&mut self) {
        self.haptic = true;
    }

    /// Asks the host to open the info mark's demo now, as a first visit does (Terra, as a toy it holds
    /// opens for the first time; `Host::first_info`).
    pub fn show_info(&mut self) {
        self.show_info = true;
    }

    /// Whether a haptic tap was asked for since the last call, forgetting it (a host passes on the
    /// taps of a toy it holds: `toys/terra`).
    pub fn take_haptic(&mut self) -> bool {
        std::mem::take(&mut self.haptic)
    }

    /// Whether the demo was asked for since the last call, forgetting it.
    pub fn take_show_info(&mut self) -> bool {
        std::mem::take(&mut self.show_info)
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
    /// What the rules mark shows (`info`): the rules the real thing follows, two to four short lines,
    /// the note empty. None (the default) shows no rules mark.
    fn rules(&self) -> Option<info::Rules> {
        None
    }
    /// What the origins mark shows (`info`): where the rules came from (the real science and
    /// history behind them). None (the default) shows no origins mark.
    fn origins(&self) -> Option<info::Rules> {
        None
    }
    /// The rules the toy's crowd follows, as switches (`switches`): a short label each and whether it is
    /// on now. Empty (the default) shows no switches mark. Asked every frame while the panel shows.
    fn switches(&self) -> Vec<switches::Switch> {
        Vec::new()
    }
    /// The reader turned rule `i` (of `switches()`) on or off. The kit turns every rule back on as the
    /// panel closes, so the toy needs no reset of its own; it need not save them.
    fn set_switch(&mut self, _ctx: &mut Ctx, _i: usize, _on: bool) {}
    /// Whether the shell hangs the rules' and origins' card from the top of the screen, not over the
    /// marks at the foot (false, the default), so it keeps clear of what matters in the scene. Asked
    /// as a card opens, so it may look at where things are now.
    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        false
    }
    /// Whether the info mark's demo opens by itself a moment after the toy starts (true, the default:
    /// a first visit shows what the toy is). Terra says no for its Earth and asks for each toy it opens
    /// instead (`Ctx::show_info`).
    fn info_at_start(&self) -> bool {
        true
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
    /// A toy whose picture is the ground seen from straight above, at a true scale, says where it lies
    /// ([`Above`]), so a globe holding it (Terra) can come down onto it under the fingers and hand over
    /// exactly: the last frame of the Earth and the toy's first are the same place at the same scale
    /// (painting-company row 258). None (the default): not a view from above.
    fn seen_from_above(&self, _ctx: &Ctx) -> Option<Above> {
        None
    }
}

/// Where a toy seen from straight above lies on the Earth (`Toy::seen_from_above`).
#[derive(Clone, Debug, PartialEq)]
pub struct Above {
    /// The place at the middle of the toy's screen (longitude, latitude, degrees).
    pub centre: [f64; 2],
    /// Metres of ground a point of the screen covers.
    pub metres_per_point: f64,
    /// How the toy's picture is turned: its up is this many degrees east of north.
    pub turn: f64,
    /// The picture of the ground the toy lies on, if it has one, for the globe to lay at its place as the
    /// camera comes down (`Globe::set_sheet`).
    pub sheet: Option<Sheet>,
}

/// A picture of the ground laid on the globe at its true place and turn (`globe::sheet`).
#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    /// The image, by its name in the toy's data (a square picture, north-up before `turn`).
    pub image: String,
    /// Its middle (longitude, latitude, degrees), metres a side, and turn (its up, degrees east of north).
    pub centre: [f64; 2],
    pub side_m: f64,
    pub turn: f64,
    /// The colour as the toy shows it: `(c - lift[0]) * lift[1]`, then saturation by `lift[2]` (1 none).
    pub lift: [f32; 3],
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
    /// The info demo's opening on a first visit (`info::First`).
    pub first_info: info::First,
    /// The switches mark and its panel (`switches`).
    switches: switches::Panel,
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
            first_info: Default::default(),
            switches: Default::default(),
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
        self.first_info.finger(&e);
        if self.switches_pointer(&e) {
            return;
        }
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
        if let Some(s) = self.switches_spot() {
            a.push(("switches".into(), s));
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
        // the origins mark may come and go with what a host holds (Terra: a held toy's, while it is open)
        self.marks.origins = self.toy.origins().is_some();
        // the demo's ghost finger: real touches, before the toy's step
        for g in self.info.advance(t, self.ctx.reduce_motion) {
            self.toy.pointer(&mut self.ctx, g);
        }
        self.toy.update(&mut self.ctx, t, dt);
        self.first_info_step(t);
        self.switches_step();
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
        self.draw_switches_mark(target);
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

#[cfg(not(target_arch = "wasm32"))]
/// The shell's text showing at a shot (the info line, the rules, the switches panel, the title), which the
/// shell sets in the system's text and the frame lacks: tools/kit/shell_text.py draws it on the shot.
pub(crate) fn shell_json<T: Toy>(
    host: &Host<T>,
    path: &std::path::Path,
    size: [u32; 2],
    ppp: f64,
) -> anyhow::Result<()> {
    let title = host.toy.title();
    let (line, rules, origins) = (host.info.line(), host.info.rules(), host.info.origins());
    let switches = host.switches_text();
    if line.is_none()
        && rules.is_none()
        && origins.is_none()
        && switches.is_none()
        && title.is_none()
    {
        return Ok(());
    }
    let s = |c: Option<&std::ffi::CStr>| {
        serde_json::Value::from(c.map(|c| c.to_string_lossy().into_owned()))
    };
    let j = serde_json::json!({ "size": size, "ppp": ppp, "line": s(line), "rules": s(rules),
        "origins": s(origins), "switches": s(switches), "top": host.card_at_top(), "title": title });
    std::fs::write(path, j.to_string())?;
    Ok(())
}
