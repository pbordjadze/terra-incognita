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
pub mod chrome;
pub mod field;
pub mod film;
pub mod frame;
#[cfg(feature = "globe")]
pub mod globe;
pub mod gpu;
pub mod info;
#[doc(hidden)]
pub mod ios;
pub mod layer;
pub mod model;
pub mod paint;
pub mod shake;
#[cfg(target_os = "ios")]
pub mod shell;
pub mod sound;
pub mod switches;
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub mod web;

pub use assets::{Assets, DirAssets, MemoryAssets};
pub use chrome::{About, Haptic, Senses};
pub use frame::Frame;
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

/// What the phone feels, in the device's own frame held upright (portrait): x to the screen's right, y
/// to its top, z out of the glass toward the reader (iOS's Core Motion frame). Each is None where the
/// device or the shell has no such sense (a desktop browser, the headless check unless a plan gives it), so
/// a toy that reads one has a way to work without it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sensors {
    /// Which way is down, in g (a unit vector at rest): flat on a table face up, [0, 0, -1].
    pub gravity: Option<[f64; 3]>,
    /// The magnetic field through the phone, in microtesla, the phone's own magnetism calibrated out
    /// (the Earth's is 25 to 65 µT; a fridge magnet held near is hundreds or thousands).
    pub field: Option<[f64; 3]>,
    /// Where the top of the phone points, in degrees clockwise from magnetic north (the browser's compass,
    /// where it gives no field).
    pub heading: Option<f64>,
}

impl Sensors {
    /// Which way the screen leans, in screen points' directions (x right, y down the screen), from 0 flat to
    /// 1 on edge: where water on the glass would run. None without gravity.
    pub fn downhill(&self) -> Option<[f64; 2]> {
        self.gravity.map(|g| [g[0], -g[1]])
    }

    /// The field across the glass, in µT, as screen directions (x right, y down the screen): what a compass
    /// needle lying on the screen would follow. From the field, or else the heading as the Earth's
    /// horizontal field (about 20 µT) for a phone held flat. None with neither.
    pub fn field_on_screen(&self) -> Option<[f64; 2]> {
        if let Some(b) = self.field {
            return Some([b[0], -b[1]]);
        }
        self.heading.map(|h| {
            // north is `h` degrees anticlockwise from the phone's top, seen from above
            let a = h.to_radians();
            [-20.0 * a.sin(), -20.0 * a.cos()]
        })
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
    /// What the phone feels: its tilt and the magnetic field through it (`Sensors`), as the shell last said.
    pub sensors: Sensors,
    /// The frame draws the chrome in its platform's own controls (`chrome::In::Frame`): the kit draws no
    /// mark, and a host draws none of its own (Terra's Earth mark: the frame's back button is it).
    pub native_chrome: bool,
    haptics: Vec<Haptic>,
    show_info: bool,
    hide_info: bool,
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
            sensors: Sensors::default(),
            native_chrome: false,
            haptics: Vec::new(),
            show_info: false,
            hide_info: false,
        }
    }

    /// Asks the frame for one light haptic tap (`Haptic::Tick`).
    pub fn haptic(&mut self) {
        self.feel(Haptic::Tick);
    }

    /// Asks the frame for a haptic of this kind (an intent: each platform plays it in its own way). At most
    /// a few wait for the frame; the strongest are kept.
    pub fn feel(&mut self, h: Haptic) {
        self.haptics.push(h);
        if self.haptics.len() > 4 {
            self.haptics.sort_unstable_by(|a, b| b.cmp(a));
            self.haptics.truncate(4);
        }
    }

    /// Asks the host to open the info mark's demo now, as a first visit does (Terra, as a toy it holds
    /// opens for the first time; `Host::first_info`).
    pub fn show_info(&mut self) {
        self.show_info = true;
    }

    /// Asks the host to close the demo or sheet now (Terra, as the toy it holds goes back into its card: the
    /// toy's demo must not play on into the Earth).
    pub fn hide_info(&mut self) {
        self.hide_info = true;
    }

    /// Whether the demo's closing was asked for since the last call, forgetting it.
    pub fn take_hide_info(&mut self) -> bool {
        std::mem::take(&mut self.hide_info)
    }

    /// The haptics asked for since the last call, forgetting them (a host passes on those of a toy it
    /// holds: `toys/terra`).
    pub fn take_haptics(&mut self) -> Vec<Haptic> {
        std::mem::take(&mut self.haptics)
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
    /// The demo (`info`): the app's one gesture as a ghost finger on the live scene, and at most one line,
    /// played on a first visit and by the sheet's "Show me". Asked as it plays, so the strokes may aim at
    /// where things are now (a city, a whale).
    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        None
    }
    /// What the sheet shows first (`info`): the rules the real thing follows, two to four short lines,
    /// the note empty. None (the default): the sheet holds only the switches and buttons, if any.
    fn rules(&self) -> Option<info::Rules> {
        None
    }
    /// The style from the history of animation the toy is drawn in ([`info::Style`]), credited in one line
    /// at the foot of its sheet. None (the default): the toy's own look.
    fn style(&self) -> Option<info::Style> {
        None
    }
    /// What the sheet's "Where this comes from" shows (`info`): where the rules came from (the real
    /// science and history behind them). None (the default) shows no such button.
    fn origins(&self) -> Option<info::Rules> {
        None
    }
    /// The rules the toy's crowd follows, as switches (`switches`): a short label each and whether it is
    /// on now, laid out under the rules on the sheet. Empty (the default): none. Asked every frame.
    fn switches(&self) -> Vec<switches::Switch> {
        Vec::new()
    }
    /// The reader turned rule `i` (of `switches()`) on or off. It stays so after the sheet closes, until
    /// the reader turns it back (the mark wears an amber dot meanwhile); a toy need not save them: opened
    /// afresh, every rule is on.
    fn set_switch(&mut self, _ctx: &mut Ctx, _i: usize, _on: bool) {}
    /// Whether the shell hangs the sheet from the top of the screen, not over the mark at the foot (false, the default), so it keeps clear of what matters in the scene. Asked
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
    /// Words the toy places itself (Terra's postcard: a toy's name, its place and one line), which the
    /// shell sets in the system's text in the box the toy gives, over its frame. None (the default): none.
    /// Asked every frame.
    fn label(&self, _ctx: &Ctx) -> Option<Label> {
        None
    }
    /// What is on screen, in words: its name, its place and one line (Terra: the toy whose card is up or
    /// open). The frame's sheet, a share and the screen reader say it. None (the default): nothing.
    fn about(&self, _ctx: &Ctx) -> Option<About> {
        None
    }
    /// The way back, if there is one now: its words for the screen reader ("Back to the Earth"). The
    /// frame shows its own back button and takes the system's back gesture for it (`go_back`). None (the
    /// default): there is nowhere to go back to, and the system's back leaves the app.
    fn back(&self, _ctx: &Ctx) -> Option<String> {
        None
    }
    /// The reader went back (`back`), at `t` (the frame's seconds).
    fn go_back(&mut self, _ctx: &mut Ctx, _t: f64) {}
    /// The senses the toy reads now (`chrome::Senses`); the frame keeps the others off. The default: a
    /// shake only. A toy that reads `ctx.sensors` asks for its gravity, field or heading here.
    fn senses(&self) -> Senses {
        Senses::default()
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

/// Words a toy places (`Toy::label`): a box in points (left, top, width, height), how shown it is (0..1),
/// and its lines: the first a heading, the second quieter, the rest plain; dark ink (on a light ground)
/// or light.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct Label {
    pub at: [f64; 4],
    pub alpha: f64,
    pub dark: bool,
    pub lines: Vec<String>,
}

impl Label {
    /// As the shell takes it: the box, the alpha and the ink (1 dark) on a first line, tab-separated, then
    /// a line each.
    pub fn text(&self) -> String {
        let mut t = format!(
            "{:.1}\t{:.1}\t{:.1}\t{:.1}\t{:.3}\t{}\n",
            self.at[0],
            self.at[1],
            self.at[2],
            self.at[3],
            self.alpha.clamp(0.0, 1.0),
            self.dark as u8
        );
        for l in &self.lines {
            t.push_str(&l.replace(['\n', '\0'], " "));
            t.push('\n');
        }
        t
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
    /// Whether the sheet hangs from the top (`Toy::card_at_top`, asked as it opened).
    sheet_top: bool,
    /// Shakes told from the device's motion, and the rest after one (`shake`).
    shake: shake::Shake,
}

impl<T: Toy> Host<T> {
    pub fn new(mut ctx: Ctx) -> anyhow::Result<Host<T>> {
        step("the toy starting");
        let mut toy = T::new(&mut ctx)?;
        let synth = toy.synth().map(|s| Arc::new(Mutex::new(s)));
        let mut host = Host {
            ctx,
            toy,
            synth,
            last_t: None,
            info: Default::default(),
            first_info: Default::default(),
            marks: Default::default(),
            sheet_top: false,
            shake: Default::default(),
        };
        host.marks_step();
        Ok(host)
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
            info::Route::Open => self.open_mark(e.t),
        }
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

    /// The toy's named places, and the mark's as `"info"` (and `"rules"`, `"switches"`: one place now).
    pub fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let mut a = self.toy.anchors();
        if let Some(s) = self.info_spot() {
            for name in ["info", "rules", "switches"] {
                a.push((name.into(), s));
            }
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
        // what the mark holds may come and go with what a host holds (Terra: a held toy's, while it is open)
        self.marks_step();
        // the demo's ghost finger: real touches, before the toy's step
        for g in self.info.advance(t, self.ctx.reduce_motion) {
            self.toy.pointer(&mut self.ctx, g);
        }
        self.toy.update(&mut self.ctx, t, dt);
        self.first_info_step(t);
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

    /// The system said the device was shaken at `t` (seconds): the toy's, unless a shake just landed.
    pub fn shaken(&mut self, t: f64) {
        if self.shake.cooled(t) {
            self.toy.shaken(&mut self.ctx);
        }
    }

    /// The device's own acceleration `a` (g, gravity out) at `t` (seconds); a shake is the toy's.
    pub fn motion(&mut self, a: [f64; 3], t: f64) {
        if self.shake.sample(a, t) {
            self.toy.shaken(&mut self.ctx);
        }
    }

    /// What the phone feels now (the shells call it at 60 Hz while a sense is on): the open toy reads
    /// `ctx.sensors`.
    pub fn sense(&mut self, s: Sensors) {
        self.ctx.sensors = s;
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
/// The frame's chrome at a shot (`chrome::Chrome`: the demo's line, the sheet, the postcard, the title), which a
/// frame sets in its platform's own controls and the toy's pixels lack: tools/kit/shell_text.py draws it on the
/// shot as the iPhone's frame lays it out.
pub(crate) fn shell_json<T: Toy>(
    host: &Host<T>,
    path: &std::path::Path,
    size: [u32; 2],
    ppp: f64,
) -> anyhow::Result<()> {
    let c = host.chrome();
    if c.caption.is_none() && c.sheet.is_none() && c.title.is_none() && c.label.is_none() {
        return Ok(());
    }
    let j = serde_json::json!({ "size": size, "ppp": ppp, "chrome": c });
    std::fs::write(path, j.to_string())?;
    Ok(())
}
