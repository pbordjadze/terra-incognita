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
/// The practice record's name (its title line).
const PRACTICE: &str = "practice record";
/// How long the needle's way back stays on the ring, seconds.
const TRAIL_S: f64 = 1.2;

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
    // what turns with the record, so its spin shows on the whole disc: the grooves' wear (the light
    // catches some arcs more than others), dust and a scratch or two, and a DJ's strip of tape across
    // the grooves marking the cue, smeared along its path as the record runs fast
    let ra = a - ang;
    let q = vec2<f32>(cos(ra), sin(ra)) * rr;
    let wear = noise2(vec2<f32>(rr * 9.0, (ra + 3.14159) * 2.0)) * noise2(vec2<f32>(rr * 31.0 + 5.0, (ra + 3.14159) * 5.0));
    v += vec3<f32>(0.55, 0.52, 0.60) * wear * wear * 0.10 * grooved * (0.35 + sheen * 4.0);
    let scratch = smoothstep(0.0025, 0.0, abs(dot(q - vec2<f32>(0.52, 0.18), vec2<f32>(0.62, -0.78)))) * smoothstep(0.16, 0.0, length(q - vec2<f32>(0.52, 0.18)));
    let scratch2 = smoothstep(0.002, 0.0, abs(length(q - vec2<f32>(-0.9, -0.6)) - 1.18)) * smoothstep(0.22, 0.0, length(q - vec2<f32>(-0.28, 0.26)));
    v += vec3<f32>(0.32) * (scratch + scratch2 * 0.8) * grooved;
    let dust = step(0.9985, hash2(floor(q * 260.0))) * grooved;
    v += vec3<f32>(0.35) * dust;
    // the tape: a short strip across the grooves, a third of the way in from the rim
    let tw = 0.035 + blur * 0.4;
    let along = abs(ra - 1.1 - 6.28318 * round((ra - 1.1) / 6.28318));
    let tape = smoothstep(0.80, 0.79, rr) * smoothstep(0.62, 0.63, rr) * smoothstep(tw, tw * 0.6, along * rr);
    let tone = 1.0 / (1.0 + blur * 6.0);
    v = mix(v, vec3<f32>(0.80, 0.76, 0.66), tape * (0.55 + 0.45 * tone));
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
    /// Where the needle was over the last moments (seconds of the shell's clock, of the song): a
    /// chop's or a rewind's way back is drawn along the ring as a fading streak.
    trail: std::collections::VecDeque<(f64, f64)>,
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
        ([w * 0.5, h * 0.42], r)
    }

    fn sleeve_at(&self) -> [f64; 2] {
        let [_, h] = self.pts;
        [32.0, h - 38.0]
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
        // the practice record is on the turntable from the first launch, so the first touch plays
        // something; the reader's own song (the sleeve) takes its place
        let mut deck = Deck::new();
        let (rate, l, r) = practice::record();
        deck.load(Arc::new(Song::new(PRACTICE, rate, l, r)));
        toy_kit::step("the practice record");
        Ok(Screw {
            deck: Arc::new(Mutex::new(deck)),
            record: Backdrop::new(&ctx.gpu, "record", &body),
            paint: Painter::new(&ctx.gpu),
            finger: None,
            shown: 0.0,
            chops: 0,
            kick: 0.0,
            trail: Default::default(),
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
                let kind = if (p[0] - s[0] - 4.0).hypot(p[1] - s[1]) < 32.0 { Kind::Sleeve } else { Kind::Open };
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
        if d.song.is_some() {
            self.trail.push_back((t, d.pos));
        }
        while self.trail.front().is_some_and(|f| t - f.0 > TRAIL_S) {
            self.trail.pop_front();
        }
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
        let lean = if has { ((0.97 - d.pitch) / 0.3).clamp(0.0, 1.0) * (d.vel.abs() / d.pitch.max(0.1)).min(1.0) } else { 0.0 };
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
                let len = 1.0 + 6.0 * *v as f64;
                let p0 = [(c[0] + cs * (ring - len * 0.5)) as f32 * k, (c[1] + sn * (ring - len * 0.5)) as f32 * k];
                let p1 = [(c[0] + cs * (ring + len * 0.5)) as f32 * k, (c[1] + sn * (ring + len * 0.5)) as f32 * k];
                let played = u <= at;
                let alpha = if played { 0.22 } else { 0.10 };
                self.paint.line(p0, p1, 0.55 * k, 0.8, pm([0.93, 0.88, 0.95, alpha]));
            }
            // the way back: where the needle went backward, a streak along the ring, fading
            let len_s = s.seconds();
            for w in self.trail.iter().collect::<Vec<_>>().windows(2) {
                let ((t0, p0), (_, p1)) = (*w[0], *w[1]);
                if p1 >= p0 - 1e-6 {
                    continue;
                }
                let fade = (1.0 - (self.t - t0) / TRAIL_S).clamp(0.0, 1.0) as f32;
                let (u0, u1) = (p1 / len_s, p0 / len_s);
                let steps = (((u1 - u0) * 720.0).ceil() as usize).clamp(1, 400);
                for i in 0..steps {
                    let ua = u0 + (u1 - u0) * i as f64 / steps as f64;
                    let ub = u0 + (u1 - u0) * (i + 1) as f64 / steps as f64;
                    let pt = |u: f64| {
                        let a = u * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
                        [(c[0] + a.cos() * ring) as f32 * k, (c[1] + a.sin() * ring) as f32 * k]
                    };
                    self.paint.glow(pt(ua), pt(ub), 1.6 * k, 5.0 * k, pm([1.0, 0.72, 0.95, 0.55 * fade]));
                }
            }
            let a = at * std::f64::consts::TAU - std::f64::consts::FRAC_PI_2;
            let np = [(c[0] + a.cos() * ring) as f32 * k, (c[1] + a.sin() * ring) as f32 * k];
            self.paint.disc(np, 2.4 * k, 1.0, pm([1.0, 0.95, 0.85, 0.85]));
            self.paint.glow(np, np, 2.0 * k, 7.0 * k, pm([1.0, 0.75, 0.95, 0.2 + 0.4 * self.kick as f32]));
        }
        // the sleeve: a square jacket with the record peeking out of it, brighter while no song is on
        let s = self.sleeve_at();
        let (sx, sy) = (s[0] as f32 * k, s[1] as f32 * k);
        let half = 14.0 * k;
        let ink = if has { 0.62 } else { 0.7 };
        // a paper record sleeve: a square, its round die-cut hole showing the plum label, the record
        // half out of it
        let peek = [sx + 10.0 * k, sy - 4.0 * k];
        self.paint.disc(peek, 13.0 * k, 1.0, pm([0.07, 0.065, 0.08, 1.0]));
        self.paint.ring(peek, 13.0 * k, 0.9 * k, 1.0, pm([0.9, 0.88, 0.92, ink * 0.6]));
        let (x0, y0, x1, y1) = (sx - half, sy - half, sx + half, sy + half);
        for i in 0..7 {
            let y = y0 + (y1 - y0) * (i as f32 + 0.5) / 7.0;
            self.paint.line([x0 + 1.0 * k, y], [x1 - 1.0 * k, y], (y1 - y0) / 14.0 + 0.6, 0.6, pm([0.50, 0.46, 0.43, 1.0]));
        }
        self.paint.disc([sx, sy], 5.2 * k, 1.0, pm([0.45, 0.17, 0.56, 1.0]));
        self.paint.disc([sx, sy], 0.9 * k, 1.0, pm([0.08, 0.07, 0.09, 1.0]));
        for (a, b) in [([x0, y0], [x1, y0]), ([x1, y0], [x1, y1]), ([x1, y1], [x0, y1]), ([x0, y1], [x0, y0])] {
            self.paint.line(a, b, 0.6 * k, 1.0, pm([0.85, 0.82, 0.78, ink]));
        }
        // a finger on the record: its light, and riding the fader, the way it has come
        if let Some(f) = self.finger {
            let fp = [f.at[0] as f32 * k, f.at[1] as f32 * k];
            let col = match f.kind {
                Kind::Hold => [1.0, 0.7, 0.95, 0.45 + 0.35 * self.kick as f32],
                Kind::Sleeve => [0.9, 0.9, 0.95, 0.35],
                _ => [0.95, 0.85, 1.0, 0.38],
            };
            self.paint.glow(fp, fp, 16.0 * k, 18.0 * k, pm(col));
            self.paint.ring(fp, 22.0 * k, 1.2 * k, 1.5, pm([1.0, 0.9, 1.0, 0.35]));
            // a finger resting on the record: syrup runs down from it, the way to slow it
            if matches!(f.kind, Kind::Open | Kind::Hold) && has && !self.reduce_motion {
                let run = ((self.t - f.t0 - 0.06) * 110.0).clamp(0.0, 60.0) as f32 * k;
                if run > 2.0 * k {
                    let end = [fp[0], fp[1] + 20.0 * k + run];
                    self.paint.glow([fp[0], fp[1] + 14.0 * k], end, 2.2 * k, 5.0 * k, pm([0.62, 0.25, 0.85, 0.55]));
                    self.paint.disc(end, 4.2 * k, 1.5 * k, pm([0.70, 0.32, 0.95, 0.75]));
                }
            }
            if let Kind::Fader { .. } = f.kind {
                let st = [f.start[0] as f32 * k, f.start[1] as f32 * k];
                self.paint.glow(st, fp, 2.0 * k, 8.0 * k, pm([0.80, 0.50, 1.0, 0.45]));
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
                "Put a finger anywhere on the record and push it down: it slows, the voice dropping with it. It stays where you leave it; push up to bring it back.",
                "Tap and it says the last beat again, in time. Hold and it keeps saying it.",
                "Flick it to the left and it spins back two bars.",
                "Touch the label to stop it or start it. The sleeve, bottom left, opens a song of yours from Files.",
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
