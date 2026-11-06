//! Screw (a working title): a record turning on a dark page (EXPERIENCE.md; painting-company row 250).
//! A tap plays it. A finger pushed down the record slows it, voice and all, as a turntable's pitch
//! does (the screw), and it stays there. A tap says the last beat again (the chop); held, it keeps
//! saying it. A flick to the left spins it back two bars (bring it back). A tap on the label is the
//! brake. The song is the reader's own, opened from Files through the sleeve (bottom left); nothing
//! is bundled or fetched.
//!
//! `deck.rs` is the turntable (the audio thread), `tempo.rs` finds a song's beat, `practice.rs` is
//! the checks' practice record. Here: the hand, and the picture (the record by a backdrop shader, the
//! song's shape around it and the needle's place on it, the sleeve, by the painter).

pub mod deck;
pub mod practice;
pub mod tempo;

use deck::{Deck, DeckSynth, Song};
use std::sync::{Arc, Mutex};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::info::{Demo, Rules, Stroke};
use toy_kit::paint::{pm, Painter};
use toy_kit::{AudioFile, Ctx, Phase, PointerEvent, Toy};

/// A finger moved this far (points) is a drag, not a tap.
const SLOP: f64 = 10.0;
/// A finger resting this long on a turning record holds the chop.
const HOLD_S: f64 = 0.32;
/// Points of drag down for the pitch fader to fall by 1.0 (120 points down: about 0.7, a screw).
const FADER: f64 = 400.0;
const PITCH_MIN: f64 = 0.5;
const PITCH_MAX: f64 = 1.08;
/// A flick to the left this fast (points a second), or this far, spins it back.
const FLICK_V: f64 = 350.0;
const FLICK_D: f64 = 70.0;
/// The label's share of the record's radius.
const LABEL: f64 = 0.34;

const RECORD: &str = r#"
fn hash1(x: f32) -> f32 { return fract(sin(x * 91.3458) * 47453.5453); }
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let c = u.p0.xy; let R = u.p0.z; let ang = u.p0.w;
  let lean = u.p1.x; let has = u.p1.y; let blur = u.p1.z; let kick = u.p1.w;
  let d = px - c; let r = length(d); let rr = r / R;
  let a = atan2(d.y, d.x);
  // the page: warm near-black, and, as the record slows, the faint purple of the syrup round it
  var col = vec3<f32>(0.030, 0.027, 0.034) * (1.0 - 0.25 * smoothstep(0.6, 1.6, length((px - u.size * 0.5) / u.size.y) * 2.0));
  let halo = exp(-pow(max(rr - 1.0, 0.0) * 2.2, 2.0)) * smoothstep(0.98, 1.02, rr);
  col += vec3<f32>(0.20, 0.07, 0.30) * halo * lean * 0.55;
  if (rr < 1.0) {
    let aa = 1.2 / R;
    // the vinyl: grooves, each its own shade, a smooth lead-in at the rim and run-out by the label
    let g = rr * 150.0;
    let groove = hash1(floor(g)) * 0.5 + 0.5 * sin(g * 6.2831);
    let grooved = smoothstep(LABEL + 0.03, LABEL + 0.05, rr) * (1.0 - smoothstep(0.955, 0.965, rr));
    // a track gap or two (rotationally even, as cut)
    let gap = 1.0 - 0.7 * (smoothstep(0.004, 0.0, abs(rr - 0.71)) + smoothstep(0.004, 0.0, abs(rr - 0.55)));
    var v = vec3<f32>(0.045, 0.043, 0.048) + vec3<f32>(0.018) * groove * grooved * gap;
    // the light on the grooves: two wedges that stay where the lamp is while the record turns
    let L = -2.25;
    let sheen = pow(max(cos(2.0 * (a - L)), 0.0), 18.0) * (0.10 + 0.08 * groove) * grooved * gap;
    let sheen2 = pow(max(cos(2.0 * (a - L)), 0.0), 220.0) * 0.12 * grooved;
    v += vec3<f32>(0.85, 0.82, 0.90) * (sheen + sheen2);
    // a chop's kick: the grooves catch the light for a moment
    v += vec3<f32>(0.30, 0.22, 0.38) * kick * grooved * 0.18;
    // the rim
    v += vec3<f32>(0.07) * smoothstep(0.975, 0.995, rr) * (1.0 - smoothstep(0.995, 1.0, rr));
    // the label, turning with the record
    let la = a - ang;
    if (rr < LABEL + 0.004) {
      let plum = mix(vec3<f32>(0.10, 0.10, 0.11), vec3<f32>(0.36, 0.13, 0.44), has);
      var lc = plum * (0.92 + 0.08 * sin(rr * 40.0));
      // what is printed on it: an arc, a bar and a dot, smeared as it spins fast
      // (in the label's own turning frame; the edges soften as it spins, a blur's worth)
      let w = 0.04 + blur;
      let arc = smoothstep(0.010, 0.0, abs(rr - 0.25)) * smoothstep(0.35 - w, 0.35 + w, cos(la - 0.6));
      let barr = smoothstep(0.018, 0.0, abs(rr - 0.16)) * smoothstep(0.80 - w, 0.80 + w, cos(la + 2.4));
      let q = vec2<f32>(cos(la), sin(la)) * rr;
      let dot = smoothstep(0.028, 0.018, length(q - vec2<f32>(-0.27, 0.0)));
      lc += vec3<f32>(0.95, 0.80, 0.55) * (arc * 0.55 + barr * 0.45 + dot * 0.6) * has;
      lc *= 1.0 - 0.35 * smoothstep(LABEL - 0.02, LABEL, rr);
      // the spindle
      lc = mix(lc, vec3<f32>(0.55, 0.55, 0.58), smoothstep(0.03, 0.022, rr));
      lc = mix(lc, vec3<f32>(0.10, 0.10, 0.11), smoothstep(0.014, 0.010, rr));
      v = mix(v, lc, smoothstep(LABEL + 0.004, LABEL - 0.002, rr));
    }
    col = mix(col, v, smoothstep(1.0, 1.0 - aa, rr));
  }
  return vec4<f32>(col, 1.0);
}
"#;

/// A finger on the glass and what it is doing.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// Not yet a drag, a tap or a hold.
    Open,
    /// Riding the pitch fader: the pitch and the finger's height it began from.
    Fader { p0: f64, y0: f64 },
    /// Sideways: a flick, maybe.
    Spin,
    /// Resting on a turning record: the chop held.
    Hold,
    /// On the sleeve.
    Sleeve,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: u64,
    at: [f64; 2],
    start: [f64; 2],
    t0: f64,
    /// Sideways speed, points a second (smoothed), and when it was last measured.
    vx: f64,
    tl: f64,
    kind: Kind,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Saved {
    pitch: f64,
    song: String,
    pos: f64,
}

pub struct Screw {
    deck: Arc<Mutex<Deck>>,
    record: Backdrop,
    paint: Painter,
    finger: Option<Finger>,
    /// The record's angle as shown (turns), following the deck's.
    shown: f64,
    /// The deck's chop and rewind counts as last seen, and the kick's glow.
    chops: u32,
    kick: f64,
    /// The deck's sound clock as last seen, and how long it has stood still while the record turns.
    rendered: u64,
    stalled: f64,
    /// The sleeve was touched: ask the shell for a song.
    ask: bool,
    /// Where the song was when the app last went away (taken back when it arrives again).
    resume: Option<(String, f64)>,
    /// The fader's last side of each detent (for a haptic tick as it crosses).
    side: [bool; 2],
    t: f64,
    reduce_motion: bool,
    /// The screen, points.
    pts: [f64; 2],
}

impl Screw {
    /// The record's centre and radius, points.
    fn geometry(&self) -> ([f64; 2], f64) {
        let [w, h] = self.pts;
        let r = (w * 0.40).min(h * 0.28).min(330.0);
        ([w * 0.5, h * 0.45], r)
    }

    fn sleeve_at(&self) -> [f64; 2] {
        let [_, h] = self.pts;
        [30.0, h - 36.0]
    }

    fn has_song(&self) -> bool {
        self.deck.lock().map(|d| d.song.is_some()).unwrap_or(false)
    }

    fn tap(&mut self, at: [f64; 2]) {
        let (c, r) = self.geometry();
        let mut d = self.deck.lock().unwrap();
        if d.song.is_none() {
            self.ask = true;
            return;
        }
        let on_label = (at[0] - c[0]).hypot(at[1] - c[1]) < r * LABEL;
        if !d.motor {
            d.start();
        } else if on_label {
            d.stop();
        } else {
            d.chop();
        }
    }
}

/// A file's name as a song's: no track number in front, no underscores.
fn song_name(file: &str) -> String {
    let s = file.replace('_', " ");
    let t = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let t = if t.len() < s.len() { t.trim_start_matches([' ', '-', '.', '–']).trim_start() } else { s.as_str() };
    if t.is_empty() { s.trim().to_string() } else { t.trim().to_string() }
}

impl Toy for Screw {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let body = RECORD.replace("LABEL", &format!("{LABEL:.3}"));
        Ok(Screw {
            deck: Arc::new(Mutex::new(Deck::new())),
            record: Backdrop::new(&ctx.gpu, "record", &body),
            paint: Painter::new(&ctx.gpu),
            finger: None,
            shown: 0.0,
            chops: 0,
            kick: 0.0,
            rendered: 0,
            stalled: 0.0,
            ask: false,
            resume: None,
            side: [true, true],
            t: 0.0,
            reduce_motion: ctx.reduce_motion,
            pts: ctx.points(),
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.pts = ctx.points();
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(DeckSynth(self.deck.clone())))
    }

    fn plays_music(&self) -> bool {
        true
    }

    fn wants_file(&mut self) -> bool {
        std::mem::take(&mut self.ask)
    }

    fn opened(&mut self, _ctx: &mut Ctx, file: AudioFile) {
        let name = song_name(&file.name);
        let song = Arc::new(Song::new(&name, file.rate, file.left, file.right));
        toy_kit::log(&format!("screw: {name}: {:.1} s, a beat of {:.3} s ({:.1} BPM)", song.seconds(), song.beat, 60.0 / song.beat));
        let mut d = self.deck.lock().unwrap();
        d.load(song);
        if let Some((n, pos)) = self.resume.take() {
            if n == name {
                d.pos = pos;
            }
        }
    }

    fn title(&self) -> Option<String> {
        self.deck.lock().ok()?.song.as_ref().map(|s| s.name.clone())
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                if self.finger.is_some() {
                    return;
                }
                let s = self.sleeve_at();
                let kind = if (p[0] - s[0]).hypot(p[1] - s[1]) < 30.0 { Kind::Sleeve } else { Kind::Open };
                self.finger = Some(Finger { id: e.id, at: p, start: p, t0: e.t, vx: 0.0, tl: e.t, kind });
            }
            Phase::Move => {
                let Some(mut f) = self.finger.filter(|f| f.id == e.id) else { return };
                let dt = (e.t - f.tl).max(1e-3);
                f.vx += ((e.x - f.at[0]) / dt - f.vx) * (dt / 0.05).min(1.0);
                f.tl = e.t;
                f.at = p;
                let (dx, dy) = (p[0] - f.start[0], p[1] - f.start[1]);
                if f.kind == Kind::Open && dx.hypot(dy) > SLOP {
                    let pitch = self.deck.lock().unwrap().pitch;
                    f.kind = if dy.abs() >= dx.abs() { Kind::Fader { p0: pitch, y0: p[1] } } else { Kind::Spin };
                }
                if let Kind::Fader { p0, y0 } = f.kind {
                    let pitch = (p0 - (p[1] - y0) / FADER).clamp(PITCH_MIN, PITCH_MAX);
                    self.deck.lock().unwrap().pitch = pitch;
                    // a tick as the fader crosses as-recorded and the screw's three quarters
                    for (i, at) in [0.995, 0.75].into_iter().enumerate() {
                        let side = pitch >= at;
                        if side != self.side[i] {
                            self.side[i] = side;
                            ctx.haptic();
                        }
                    }
                }
                self.finger = Some(f);
            }
            Phase::Up | Phase::Cancel => {
                let Some(f) = self.finger.filter(|f| f.id == e.id) else { return };
                self.finger = None;
                if e.phase == Phase::Cancel {
                    self.deck.lock().unwrap().hold(false);
                    return;
                }
                match f.kind {
                    Kind::Sleeve => self.ask = true,
                    Kind::Open => self.tap(p),
                    Kind::Hold => self.deck.lock().unwrap().hold(false),
                    Kind::Spin => {
                        let dx = p[0] - f.start[0];
                        if f.vx < -FLICK_V || dx < -FLICK_D {
                            let mut d = self.deck.lock().unwrap();
                            if d.song.is_some() {
                                d.rewind();
                                ctx.haptic();
                            }
                        } else if !self.has_song() {
                            self.ask = true;
                        }
                    }
                    Kind::Fader { .. } => {}
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        // a finger resting on a turning record: the chop held
        if let Some(mut f) = self.finger {
            if f.kind == Kind::Open && t - f.t0 >= HOLD_S {
                let mut d = self.deck.lock().unwrap();
                if d.motor && d.song.is_some() && !d.rewinding() {
                    d.hold(true);
                    f.kind = Kind::Hold;
                    self.finger = Some(f);
                }
            }
        }
        let mut d = self.deck.lock().unwrap();
        // no sound running (an audio engine that didn't start): the record still turns
        if d.rendered == self.rendered && (d.motor || d.vel != 0.0 || d.rewinding()) {
            self.stalled += dt;
            if self.stalled > 0.25 {
                d.advance(dt);
            }
        } else {
            self.stalled = 0.0;
        }
        self.rendered = d.rendered;
        if d.chops != self.chops {
            self.chops = d.chops;
            self.kick = 1.0;
            drop(d);
            ctx.haptic();
            d = self.deck.lock().unwrap();
        }
        self.kick *= (-dt / 0.18).exp();
        let target = d.turns;
        drop(d);
        // the angle follows the deck's quickly, so a chop's step back is a visible kick, not a jump
        self.shown += (target - self.shown) * (dt / 0.045).min(1.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let k = ctx.ppp as f32;
        let (c, r) = self.geometry();
        let d = self.deck.lock().unwrap();
        let has = d.song.is_some();
        let speed = d.vel.abs();
        let lean = if has { ((0.97 - d.pitch) / 0.3).clamp(0.0, 1.0) * if d.motor { 1.0 } else { 0.4 } } else { 0.0 };
        let blur = ((speed - 0.2) * 0.05).clamp(0.0, 0.5) as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [(c[0] * ctx.ppp) as f32, (c[1] * ctx.ppp) as f32, (r * ctx.ppp) as f32, (self.shown * std::f64::consts::TAU) as f32, lean as f32, has as u8 as f32, blur, self.kick as f32],
        };
        // the song's shape round the record, from the top, clockwise; the needle's place bright on it
        self.paint.shapes_over.clear();
        self.paint.shapes_add.clear();
        let ring = r * 1.115;
        if let Some(s) = &d.song {
            let n = s.shape.len();
            let at = (d.pos / s.seconds()).clamp(0.0, 1.0);
            for (i, v) in s.shape.iter().enumerate() {
                let u = i as f64 / n as f64;
                let a = u * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                let (cs, sn) = (a.cos(), a.sin());
                let len = 1.0 + 8.0 * *v as f64;
                let p0 = [(c[0] + cs * (ring - len * 0.5)) as f32 * k, (c[1] + sn * (ring - len * 0.5)) as f32 * k];
                let p1 = [(c[0] + cs * (ring + len * 0.5)) as f32 * k, (c[1] + sn * (ring + len * 0.5)) as f32 * k];
                let played = u <= at;
                let alpha = if played { 0.36 } else { 0.2 };
                self.paint.line(p0, p1, 0.55 * k, 0.8, pm([0.93, 0.88, 0.95, alpha]));
            }
            let a = at * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
            let np = [(c[0] + a.cos() * ring) as f32 * k, (c[1] + a.sin() * ring) as f32 * k];
            self.paint.disc(np, 3.2 * k, 1.0, pm([1.0, 0.95, 0.85, 0.95]));
            self.paint.glow(np, np, 3.0 * k, 9.0 * k, pm([1.0, 0.75, 0.95, 0.35 + 0.4 * self.kick as f32]));
        }
        // the sleeve: a square jacket with the record peeking out of it, brighter while no song is on
        let s = self.sleeve_at();
        let (sx, sy) = (s[0] as f32 * k, s[1] as f32 * k);
        let half = 10.0 * k;
        let ink = if has { 0.42 } else { 0.55 + 0.25 * (0.5 + 0.5 * (self.t * 2.4).sin() as f32) * if self.reduce_motion { 0.0 } else { 1.0 } };
        self.paint.disc([sx + 4.0 * k, sy - 4.0 * k], 8.0 * k, 1.0, pm([0.12, 0.11, 0.13, 1.0]));
        self.paint.ring([sx + 4.0 * k, sy - 4.0 * k], 8.0 * k, 0.9 * k, 1.0, pm([0.9, 0.88, 0.92, ink * 0.8]));
        self.paint.disc([sx - 1.0 * k, sy + 1.0 * k], half, 1.0, pm([0.0, 0.0, 0.0, 0.0]));
        let (x0, y0, x1, y1) = (sx - half - 1.0 * k, sy - half + 1.0 * k, sx + half - 1.0 * k, sy + half + 1.0 * k);
        // (the jacket's face covers the record behind it)
        for i in 0..5 {
            let y = y0 + (y1 - y0) * (i as f32 + 0.5) / 5.0;
            self.paint.line([x0 + 1.0 * k, y], [x1 - 1.0 * k, y], (y1 - y0) / 10.0 + 0.5, 0.6, pm([0.06, 0.055, 0.065, 1.0]));
        }
        for (a, b) in [([x0, y0], [x1, y0]), ([x1, y0], [x1, y1]), ([x1, y1], [x0, y1]), ([x0, y1], [x0, y0])] {
            self.paint.line(a, b, 0.75 * k, 1.0, pm([0.9, 0.88, 0.92, ink]));
        }
        // a finger on the record: its light, and riding the fader, the way it has come
        if let Some(f) = self.finger {
            let fp = [f.at[0] as f32 * k, f.at[1] as f32 * k];
            let col = match f.kind {
                Kind::Hold => [1.0, 0.7, 0.95, 0.30 + 0.25 * self.kick as f32],
                Kind::Sleeve => [0.9, 0.9, 0.95, 0.25],
                _ => [0.95, 0.85, 1.0, 0.22],
            };
            self.paint.glow(fp, fp, 14.0 * k, 16.0 * k, pm(col));
            if let Kind::Fader { .. } = f.kind {
                let st = [f.start[0] as f32 * k, f.start[1] as f32 * k];
                self.paint.glow(st, fp, 1.0 * k, 6.0 * k, pm([0.75, 0.45, 0.95, 0.25]));
            }
        }
        drop(d);
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.record.render(&ctx.gpu, &mut enc, target, size, &params);
        self.paint.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn save(&self) -> Option<String> {
        let d = self.deck.lock().ok()?;
        let song = d.song.as_ref().map(|s| s.name.clone()).unwrap_or_default();
        serde_json::to_string(&Saved { pitch: d.pitch, song, pos: d.pos }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            let mut d = self.deck.lock().unwrap();
            if s.pitch > 0.0 {
                d.pitch = s.pitch.clamp(PITCH_MIN, PITCH_MAX);
            }
            if d.song.as_ref().is_some_and(|x| x.name == s.song) {
                d.pos = s.pos;
            } else if !s.song.is_empty() {
                self.resume = Some((s.song, s.pos));
            }
        }
    }

    /// For plans: the record off its label, its label, a hand's width below the record's spot, the sleeve.
    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let (c, r) = self.geometry();
        vec![
            ("record".into(), [c[0] + r * 0.62, c[1] - r * 0.3]),
            ("label".into(), c),
            ("below".into(), [c[0] + r * 0.62, c[1] - r * 0.3 + 120.0]),
            ("left".into(), [c[0] - r * 0.1, c[1] + r * 0.6]),
            ("sleeve".into(), self.sleeve_at()),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<Demo> {
        let (c, r) = self.geometry();
        let d = self.deck.lock().ok()?;
        if d.song.is_none() {
            return Some(Demo::new("Open a song of yours from the sleeve.", vec![]));
        }
        let on = [c[0] + r * 0.62, c[1] - r * 0.35];
        let mut strokes = Vec::new();
        let mut t = 0.4;
        if !d.motor {
            strokes.push(Stroke::tap(t, [c[0] - r * 0.6, c[1] + r * 0.4]));
            t += 1.4;
        }
        strokes.push(Stroke::drag(t, 2.2, on, [on[0], on[1] + 120.0]));
        t += 3.2;
        strokes.push(Stroke::tap(t, [c[0] - r * 0.55, c[1] + r * 0.45]));
        strokes.push(Stroke::tap(t + 1.6, [c[0] - r * 0.55, c[1] + r * 0.45]));
        Some(Demo::new("Push it down to slow it. Tap to say the beat again.", strokes))
    }

    fn rules(&self) -> Option<Rules> {
        Some(Rules::new(
            &[
                "Push down on the record and it slows, the voice dropping with it. It stays where you leave it.",
                "Tap and it says the last beat again, in time. Hold and it keeps saying it.",
                "Flick it to the left and it spins back two bars.",
                "Touch the label to stop it or start it. The sleeve opens a song from Files.",
            ],
            "",
        ))
    }

    fn origins(&self) -> Option<Rules> {
        Some(Rules::new(
            &[
                "Houston, the 1990s: DJ Screw slowed records on a turntable's pitch, so the voices came out low and thick. That is the screw; a record slowed this way drops in pitch as it slows, as here.",
                "His chops were two copies of the same record a beat apart, the crossfader cut between them on the beat, so a line says itself again and the song never loses time.",
            ],
            "Your song plays as recorded, only slower or faster; nothing is time-stretched. It stays on this phone.",
        ))
    }
}

toy_kit::ios_shell!(Screw);

#[cfg(test)]
mod tests {
    #[test]
    fn a_files_name_becomes_the_songs() {
        assert_eq!(super::song_name("16 - Houston Ft Slim Thug Z-Ro"), "Houston Ft Slim Thug Z-Ro");
        assert_eq!(super::song_name("Paul_Wall-Houston"), "Paul Wall-Houston");
        assert_eq!(super::song_name("practice"), "practice");
        assert_eq!(super::song_name("1979"), "1979");
    }
}
