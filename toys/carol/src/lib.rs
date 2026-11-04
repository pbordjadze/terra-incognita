//! Carol: a toy chess trainer (painting-company row 215). A wooden board; tap a piece, then where it
//! goes. The right move slides home with a note, the other side answers by itself, a wrong one bumps
//! back with a red square. A bulb lights the piece to move (a miss), a play button makes the move
//! (a miss). Twenty-four drills, beads along the top: dark beads lines played as Black, light beads
//! as White, amber puzzles; a hollow bead is new, a filled one done, a red pip under it last time
//! missed. Finishing one picks the next by the mix: new ones first, then the missed, then the least
//! played.
//!
//! No move generation: every position after every ply is in `drills.json` (`after`), and a move is
//! right when its squares match the line's (castling also by the king onto its rook).

pub mod chess;

use chess::{Board, Drill, Sq};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::gpu::textures::{decode, Texture};
use toy_kit::info::{Demo, Stroke};
use toy_kit::paint::Painter;
use toy_kit::sound::{Bells, Note};
use toy_kit::{Ctx, Gpu, Phase, PointerEvent, Synth, Toy};

toy_kit::ios_shell!(Carol);

/// How long the other side waits before it answers, seconds.
const ANSWER: f64 = 0.65;
/// How long a finished drill lingers before the next comes, seconds.
const LINGER: f64 = 2.0;

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Stat {
    pub done: u32,
    /// Misses the last time it was finished.
    pub last: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Turn {
    /// The other side answers at this time.
    Answer(f64),
    Yours,
    /// Finished at this time.
    Done(f64),
}

/// A piece as drawn: where it is now, easing toward its square.
#[derive(Clone, Copy)]
struct Shown {
    piece: char,
    sq: Sq,
    pos: [f64; 2],
    born: f64,
    /// Captured: fading from this time.
    gone: Option<f64>,
    /// Bumped back (a wrong move) at this time.
    bump: f64,
}

struct Spark {
    origin: Option<Sq>,
    placed: bool,
    p: [f64; 2],
    v: [f64; 2],
    born: f64,
    color: [f32; 4],
    r: f32,
}

struct Ripple {
    p: [f64; 2],
    born: f64,
    color: [f32; 4],
}

/// The layout in points.
#[derive(Clone, Copy)]
struct Layout {
    /// The board's top left and its size (the squares only).
    x0: f64,
    y0: f64,
    s: f64,
    beads_y: f64,
    beads_x0: f64,
    beads_dx: f64,
    buttons_y: f64,
    hint_x: f64,
    show_x: f64,
    button_r: f64,
    /// The controls' scale: 1 on a phone, larger on an iPad (a reading scale, as Terra's).
    ui: f64,
}

pub struct Carol {
    drills: Vec<Drill>,
    stats: Vec<Stat>,
    current: usize,
    ply: usize,
    board: Board,
    turn: Turn,
    misses: u32,
    hinted: bool,
    hint_at: Option<f64>,
    selected: Option<Sq>,
    /// A finger on a piece: its id, where it went down, whether it has moved enough to be a drag.
    drag: Option<(u64, [f64; 2], bool)>,
    finger: [f64; 2],
    shown: Vec<Shown>,
    last_move: Option<(Sq, Sq)>,
    red: Option<(Sq, f64)>,
    sparks: Vec<Spark>,
    ripples: Vec<Ripple>,
    pressed: Option<(u8, f64)>,
    pending_notes: Vec<(f64, Note)>,
    now: f64,
    seed: u64,
    reduce_motion: bool,
    bells: Bells,
    // drawing
    backdrop: Backdrop,
    painter: Painter,
    pieces: PiecePass,
}

impl Carol {
    fn layout(&self, points: [f64; 2]) -> Layout {
        let [w, h] = points;
        let ui = (w.min(h) / 390.0).clamp(1.0, 1.5);
        // the group (beads, board, buttons) centred in the safe area (a status bar above, the home
        // indicator below)
        let (top, bottom) = (54.0, 30.0);
        let above = 44.0 * ui;
        let below = 24.0 * ui + 2.0 * 30.0 * ui;
        let s = (w - 28.0).min(h - top - bottom - above - below - 24.0).min(820.0).max(200.0);
        let x0 = (w - s) / 2.0;
        let y0 = top + ((h - top - bottom) - (above + s + below)) / 2.0 + above;
        let span = (s - 8.0).min(w - 40.0);
        let gaps = self.drills.windows(2).filter(|p| p[0].group != p[1].group).count();
        let slots = (self.drills.len() + gaps).max(2) as f64;
        let beads_dx = span / (slots - 1.0);
        let beads_x0 = (w - span) / 2.0;
        let button_r = 30.0 * ui;
        Layout {
            ui,
            x0,
            y0,
            s,
            beads_y: y0 - 30.0 * ui,
            beads_x0,
            beads_dx,
            buttons_y: y0 + s + 24.0 * ui + button_r,
            hint_x: w / 2.0 - 58.0 * ui,
            show_x: w / 2.0 + 58.0 * ui,
            button_r,
        }
    }

    fn drill(&self) -> &Drill {
        &self.drills[self.current]
    }

    /// Where a square's centre is on screen.
    fn sq_pos(&self, l: &Layout, sq: Sq) -> [f64; 2] {
        let (c, r) = self.drill().screen(sq);
        let q = l.s / 8.0;
        [l.x0 + (c as f64 + 0.5) * q, l.y0 + (r as f64 + 0.5) * q]
    }

    fn sq_at(&self, l: &Layout, p: [f64; 2]) -> Option<Sq> {
        let q = l.s / 8.0;
        let c = ((p[0] - l.x0) / q).floor();
        let r = ((p[1] - l.y0) / q).floor();
        if (0.0..8.0).contains(&c) && (0.0..8.0).contains(&r) {
            Some(self.drill().from_screen(c as u8, r as u8))
        } else {
            None
        }
    }

    /// Where each bead is (the groups a unit apart).
    fn bead_pos(&self, l: &Layout, i: usize) -> [f64; 2] {
        let mut gaps = 0;
        for j in 1..=i {
            if self.drills[j].group != self.drills[j - 1].group {
                gaps += 1;
            }
        }
        [l.beads_x0 + (i + gaps) as f64 * l.beads_dx, l.beads_y]
    }

    fn rnd(&mut self) -> f64 {
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.seed >> 33) as f64) / (1u64 << 31) as f64
    }

    /// The mix: new drills first, then the missed (most first), then the least done; never the one
    /// just played when another is there.
    pub fn next_in_mix(stats: &[Stat], current: usize) -> usize {
        let mut order: Vec<usize> = (0..stats.len()).collect();
        order.sort_by_key(|&i| {
            let s = stats[i];
            (if s.done == 0 { 0 } else if s.last > 0 { 1 } else { 2 }, std::cmp::Reverse(if s.done > 0 { s.last } else { 0 }), s.done, i)
        });
        order.into_iter().find(|&i| i != current || stats.len() == 1).unwrap_or(0)
    }

    fn start(&mut self, d: usize, ctx: &Ctx) {
        self.current = d;
        let drill = &self.drills[d];
        let mut board = drill.start.clone();
        let auto = drill.auto.min(drill.plies.len());
        if auto > 0 {
            board = drill.plies[auto - 1].after.clone();
        }
        self.ply = auto;
        self.board = board;
        self.misses = 0;
        self.hinted = false;
        self.hint_at = None;
        self.selected = None;
        self.drag = None;
        self.last_move = if auto > 0 { Some(drill.plies[auto - 1].squares()) } else { None };
        self.red = None;
        // the pieces drop in, a wave from the reader's side
        let l = self.layout(ctx.points());
        self.shown.clear();
        for sq in 0..64u8 {
            if let Some(p) = self.board.at(sq) {
                let (_, r) = self.drills[d].screen(sq);
                let delay = if self.reduce_motion { 0.0 } else { (7 - r) as f64 * 0.045 + 0.05 };
                self.shown.push(Shown { piece: p, sq, pos: self.sq_pos(&l, sq), born: self.now + delay, gone: None, bump: -9.0 });
            }
        }
        self.after_ply();
    }

    /// Whose turn it is now, after the ply count changed.
    fn after_ply(&mut self) {
        let drill = &self.drills[self.current];
        if self.ply >= drill.plies.len() {
            self.turn = Turn::Done(self.now);
            self.finish();
        } else if self.board.white_to_move == drill.white {
            self.turn = Turn::Yours;
        } else {
            self.turn = Turn::Answer(self.now + ANSWER + if self.ply == drill.auto { 0.5 } else { 0.0 });
        }
        self.hinted = false;
        self.hint_at = None;
    }

    fn finish(&mut self) {
        let s = &mut self.stats[self.current];
        s.done += 1;
        s.last = self.misses;
        let clean = self.misses == 0;
        // a chord climbing, brighter when clean
        let notes: &[f32] = if clean { &[523.25, 659.25, 783.99, 1046.5] } else { &[440.0, 523.25, 659.25] };
        for (i, f) in notes.iter().enumerate() {
            self.pending_notes.push((self.now + 0.12 * i as f64, Note { freq: *f, amp: 0.22, decay: 1.6, pan: -0.3 + 0.2 * i as f32, bright: 0.8 }));
        }
        if !self.reduce_motion {
            let centre = self.last_move.map(|(_, to)| to);
            self.burst(centre, clean);
        }
    }

    fn burst(&mut self, at: Option<Sq>, clean: bool) {
        let n = if clean { 70 } else { 28 };
        let palette: [[f32; 4]; 5] = if clean {
            [[0.98, 0.78, 0.30, 1.0], [0.95, 0.45, 0.38, 1.0], [0.45, 0.75, 0.62, 1.0], [0.99, 0.95, 0.85, 1.0], [0.55, 0.65, 0.95, 1.0]]
        } else {
            [[0.99, 0.95, 0.85, 1.0], [0.98, 0.78, 0.30, 1.0], [0.99, 0.95, 0.85, 1.0], [0.85, 0.80, 0.70, 1.0], [0.98, 0.78, 0.30, 1.0]]
        };
        for i in 0..n {
            let a = self.rnd() * std::f64::consts::TAU;
            let sp = 120.0 + 380.0 * self.rnd();
            let r = 2.5 + 3.5 * self.rnd() as f32;
            self.sparks.push(Spark { origin: at, placed: false, p: [0.0, 0.0], v: [a.cos() * sp, a.sin() * sp - 260.0], born: self.now + 0.004 * i as f64, color: palette[i % 5], r });
        }
    }

    fn strike(&self, freq: f32, amp: f32, decay: f32, bright: f32) {
        self.bells.strike(Note { freq, amp, decay, pan: 0.0, bright });
    }

    /// Plays a ply onto the board (the user's, the other side's, or shown).
    fn play_ply(&mut self, ctx: &mut Ctx, yours: bool) {
        let ply = self.drills[self.current].plies[self.ply].clone();
        self.board = ply.after.clone();
        self.last_move = Some(ply.squares());
        self.ply += 1;
        self.selected = None;
        self.retarget(ctx);
        let scale = [523.25f32, 587.33, 659.25, 783.99, 880.0, 1046.5, 1174.7, 1318.5];
        if yours {
            let k = (self.ply / 2).min(7);
            self.strike(scale[k], 0.28, 0.9, 1.2);
            ctx.haptic();
        } else {
            // the other side: a soft wooden knock
            self.strike(261.63, 0.22, 0.35, 0.4);
        }
        if ply.capture {
            self.strike(130.8, 0.18, 0.25, 2.5);
        }
        self.after_ply();
    }

    /// The shown pieces follow the board: kept where they stand, moved from where one went, faded
    /// where captured, born where new (a promotion).
    fn retarget(&mut self, ctx: &Ctx) {
        let l = self.layout(ctx.points());
        let mut used = vec![false; self.shown.len()];
        let mut placed: Vec<(usize, Sq)> = Vec::new();
        let mut unmatched: Vec<(Sq, char)> = Vec::new();
        for sq in 0..64u8 {
            if let Some(p) = self.board.at(sq) {
                if let Some(i) = self.shown.iter().position(|s| s.gone.is_none() && s.sq == sq && s.piece == p) {
                    used[i] = true;
                    placed.push((i, sq));
                } else {
                    unmatched.push((sq, p));
                }
            }
        }
        for (sq, p) in unmatched {
            // the nearest piece of its kind that has left its square
            let to = self.sq_pos(&l, sq);
            let best = self
                .shown
                .iter()
                .enumerate()
                .filter(|(i, s)| !used[*i] && s.gone.is_none() && s.piece == p && self.board.at(s.sq) != Some(p))
                .min_by(|a, b| dist(a.1.pos, to).total_cmp(&dist(b.1.pos, to)))
                .map(|(i, _)| i);
            match best {
                Some(i) => {
                    used[i] = true;
                    self.shown[i].sq = sq;
                    if self.reduce_motion {
                        self.shown[i].pos = to;
                    }
                }
                None => {
                    self.shown.push(Shown { piece: p, sq, pos: to, born: self.now, gone: None, bump: -9.0 });
                    used.push(true);
                }
            }
        }
        for (i, s) in self.shown.iter_mut().enumerate() {
            if !used[i] && s.gone.is_none() {
                s.gone = Some(self.now);
            }
        }
    }

    /// The user's move from `from` to `to`: right, or a miss.
    fn try_move(&mut self, ctx: &mut Ctx, from: Sq, to: Sq) {
        let ply = &self.drills[self.current].plies[self.ply];
        if ply.accepts(&self.board, from, to) {
            self.play_ply(ctx, true);
        } else {
            self.misses += 1;
            self.red = Some((to, self.now));
            if let Some(s) = self.shown.iter_mut().find(|s| s.gone.is_none() && s.sq == from) {
                s.bump = self.now;
            }
            self.selected = None;
            self.strike(155.56, 0.3, 0.45, 2.2);
            ctx.haptic();
        }
    }

    fn hint(&mut self, ctx: &mut Ctx) {
        if self.turn != Turn::Yours {
            return;
        }
        if !self.hinted {
            self.hinted = true;
            self.misses += 1;
        }
        self.hint_at = Some(self.now);
        self.strike(1318.5, 0.16, 1.2, 0.3);
        ctx.haptic();
    }

    fn show(&mut self, ctx: &mut Ctx) {
        if self.turn != Turn::Yours {
            return;
        }
        self.misses += 1;
        self.drag = None;
        self.play_ply(ctx, true);
    }

    fn ripple(&mut self, p: [f64; 2], color: [f32; 4]) {
        if self.ripples.len() < 12 {
            self.ripples.push(Ripple { p, born: self.now, color });
        }
    }

    fn own(&self, sq: Sq) -> bool {
        self.board.at(sq).is_some_and(|p| p.is_ascii_uppercase() == self.drill().white)
    }
}

fn dist(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// A spring's overshoot from 0 to 1 (a drop that bounces once).
fn pop(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    1.0 - (-6.0 * t).exp() * (9.0 * t).cos()
}

impl Toy for Carol {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let drills = chess::load(&ctx.assets.read("drills.json")?)?;
        let atlas = decode(&ctx.assets.read("pieces.png")?, 4)?;
        let gpu = &ctx.gpu;
        let pieces = PiecePass::new(gpu, &Texture::from_pixels(gpu, "pieces", &atlas, false));
        let n = drills.len();
        let mut toy = Carol {
            stats: vec![Stat::default(); n],
            drills,
            current: 0,
            ply: 0,
            board: Board::empty(),
            turn: Turn::Yours,
            misses: 0,
            hinted: false,
            hint_at: None,
            selected: None,
            drag: None,
            finger: [0.0, 0.0],
            shown: Vec::new(),
            last_move: None,
            red: None,
            sparks: Vec::new(),
            ripples: Vec::new(),
            pressed: None,
            pending_notes: Vec::new(),
            now: 0.0,
            seed: 0x5eed_c4e5,
            reduce_motion: ctx.reduce_motion,
            bells: Bells::new(),
            backdrop: Backdrop::new(gpu, "carol board", BOARD_WGSL),
            painter: Painter::new(gpu),
            pieces,
        };
        let first = Carol::next_in_mix(&toy.stats, usize::MAX);
        toy.start(first, ctx);
        Ok(toy)
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let l = self.layout(ctx.points());
        let p = [e.x, e.y];
        self.finger = p;
        match e.phase {
            Phase::Down => {
                // the buttons
                for (k, x) in [(0u8, l.hint_x), (1u8, l.show_x)] {
                    if dist(p, [x, l.buttons_y]) < l.button_r + 12.0 {
                        self.pressed = Some((k, self.now));
                        if self.turn == Turn::Yours {
                            if k == 0 { self.hint(ctx) } else { self.show(ctx) }
                        } else {
                            self.ripple([x, l.buttons_y], [1.0, 0.95, 0.85, 0.5]);
                        }
                        return;
                    }
                }
                // the beads
                if (p[1] - l.beads_y).abs() < 22.0 * l.ui {
                    let near = (0..self.drills.len()).map(|i| (i, dist(p, self.bead_pos(&l, i)))).min_by(|a, b| a.1.total_cmp(&b.1));
                    if let Some((i, d)) = near {
                        if d < l.beads_dx.max(14.0) {
                            self.strike(987.8, 0.15, 0.5, 0.6);
                            ctx.haptic();
                            self.start(i, ctx);
                            return;
                        }
                    }
                }
                let Some(sq) = self.sq_at(&l, p) else {
                    self.ripple(p, [1.0, 0.95, 0.85, 0.35]);
                    return;
                };
                if self.turn != Turn::Yours {
                    self.ripple(p, [1.0, 0.95, 0.85, 0.45]);
                    return;
                }
                if self.own(sq) {
                    if self.selected == Some(sq) {
                        self.drag = Some((e.id, p, false));
                    } else {
                        self.selected = Some(sq);
                        self.drag = Some((e.id, p, false));
                        self.strike(698.5, 0.1, 0.25, 0.5);
                    }
                } else if let Some(from) = self.selected {
                    self.try_move(ctx, from, sq);
                } else {
                    self.ripple(p, [1.0, 0.95, 0.85, 0.45]);
                }
            }
            Phase::Move => {
                if let Some((id, at, moved)) = self.drag {
                    if id == e.id && !moved && dist(at, p) > 8.0 {
                        self.drag = Some((id, at, true));
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if let Some((id, _, moved)) = self.drag {
                    if id == e.id {
                        self.drag = None;
                        if moved && e.phase == Phase::Up && self.turn == Turn::Yours {
                            if let (Some(from), Some(to)) = (self.selected, self.sq_at(&l, p)) {
                                if to != from {
                                    self.try_move(ctx, from, to);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.now = t;
        self.reduce_motion = ctx.reduce_motion;
        keep_points(ctx.points());
        let mut i = 0;
        while i < self.pending_notes.len() {
            if self.pending_notes[i].0 <= t {
                let (_, n) = self.pending_notes.remove(i);
                self.bells.strike(n);
            } else {
                i += 1;
            }
        }
        match self.turn {
            Turn::Answer(at) if t >= at => self.play_ply(ctx, false),
            Turn::Done(at) if t >= at + LINGER => {
                let next = Carol::next_in_mix(&self.stats, self.current);
                self.start(next, ctx);
            }
            _ => {}
        }
        // the pieces ease toward their squares
        let l = self.layout(ctx.points());
        let k = if self.reduce_motion { 1.0 } else { 1.0 - (-dt * 16.0).exp() };
        let targets: Vec<[f64; 2]> = self.shown.iter().map(|s| self.sq_pos(&l, s.sq)).collect();
        for (s, to) in self.shown.iter_mut().zip(targets) {
            s.pos = [s.pos[0] + (to[0] - s.pos[0]) * k, s.pos[1] + (to[1] - s.pos[1]) * k];
        }
        self.shown.retain(|s| s.gone.is_none_or(|g| t - g < 0.5));
        let centre = [l.x0 + l.s / 2.0, l.y0 + l.s / 2.0];
        for i in 0..self.sparks.len() {
            if !self.sparks[i].placed {
                self.sparks[i].p = self.sparks[i].origin.map(|q| self.sq_pos(&l, q)).unwrap_or(centre);
                self.sparks[i].placed = true;
            }
            let s = &mut self.sparks[i];
            if t >= s.born {
                s.v[1] += 900.0 * dt;
                s.v[0] *= (-1.2 * dt).exp();
                s.p = [s.p[0] + s.v[0] * dt, s.p[1] + s.v[1] * dt];
            }
        }
        self.sparks.retain(|s| t - s.born < 1.8);
        self.ripples.retain(|r| t - r.born < 0.6);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let ppp = ctx.ppp;
        let size = [ctx.width, ctx.height];
        let l = self.layout(ctx.points());
        let t = self.now;
        let k = ppp as f32;
        let px = |p: [f64; 2]| [(p[0] * ppp) as f32, (p[1] * ppp) as f32];
        let drill = &self.drills[self.current];
        let idx = |sq: Option<Sq>| sq.map(|s| { let (c, r) = drill.screen(s); (r as u32 * 8 + c as u32) as f32 }).unwrap_or(-1.0);
        let (red_sq, red_k) = match self.red {
            Some((s, at)) if t - at < 0.7 => (idx(Some(s)), (1.0 - (t - at) / 0.7) as f32),
            _ => (-1.0, 0.0),
        };
        let glow = match self.turn {
            Turn::Done(at) => (((t - at) * 3.0).min(1.0) * (1.0 - ((t - at - LINGER + 0.5) * 2.0).clamp(0.0, 1.0))) as f32 * if self.stats[self.current].last == 0 { 1.0 } else { 0.4 },
            _ => 0.0,
        };
        let params = BackdropParams {
            time: t as f32,
            scale: k,
            center: [glow, 0.0],
            p: [
                (l.x0 * ppp) as f32,
                (l.y0 * ppp) as f32,
                (l.s * ppp) as f32,
                (10.0 * ppp) as f32,
                idx(self.last_move.map(|m| m.0)),
                idx(self.last_move.map(|m| m.1)),
                red_sq,
                red_k,
            ],
        };
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        self.backdrop.render(gpu, &mut enc, target, size, &params);

        // under the pieces: the selection, the hint
        let q = (l.s / 8.0) as f32 * k;
        if let Some(sq) = self.selected {
            let c = px(self.sq_pos(&l, sq));
            self.painter.ring(c, q * 0.46, 3.0 * k, 1.2 * k, [1.0, 0.97, 0.88, 0.95]);
        }
        if let (Some(at), Turn::Yours) = (self.hint_at, self.turn) {
            let from = drill.plies[self.ply].squares().0;
            let c = px(self.sq_pos(&l, from));
            let breathe = 0.65 + 0.35 * ((t - at) * 4.0).cos() as f32;
            self.painter.glow(c, c, q * 0.30, q * 0.25, [1.0, 0.80, 0.30, 0.55 * breathe]);
            self.painter.glow_ring(c, q * 0.44, 2.5 * k, 2.0 * k, [1.0, 0.85, 0.40, 0.9]);
        }
        self.painter.render(gpu, &mut enc, target, size, None);

        // the pieces
        let mut inst: Vec<PieceInst> = Vec::new();
        let dragging = self.drag.filter(|d| d.2).and(self.selected);
        let mut top: Vec<PieceInst> = Vec::new();
        for s in &self.shown {
            let age = t - s.born;
            if age < 0.0 {
                continue;
            }
            let mut scale = if self.reduce_motion { 1.0 } else { pop(age / 0.45) };
            let mut alpha = 1.0;
            let mut lift = 0.0;
            let mut pos = s.pos;
            if let Some(g) = s.gone {
                let u = ((t - g) / 0.35).clamp(0.0, 1.0);
                alpha = 1.0 - u;
                scale *= 1.0 + 0.25 * u;
            }
            let target_pos = self.sq_pos(&l, s.sq);
            let travel = dist(pos, target_pos) / (l.s / 8.0);
            if travel > 0.05 {
                lift = travel.min(1.0);
                scale *= 1.0 + 0.12 * lift;
            }
            if Some(s.sq) == self.selected && s.gone.is_none() {
                lift = lift.max(0.35);
                scale *= 1.06;
            }
            let bump = t - s.bump;
            if bump < 0.4 && !self.reduce_motion {
                pos[0] += (bump * 45.0).sin() * (1.0 - bump / 0.4) * l.s / 8.0 * 0.12;
            }
            let is_drag = dragging == Some(s.sq) && s.gone.is_none();
            if is_drag {
                pos = [self.finger[0], self.finger[1] - l.s / 8.0 * 0.45];
                lift = 1.0;
                scale *= 1.2;
            }
            let p = px(pos);
            let one = PieceInst { a: [p[0], p[1], q * 0.6 * scale as f32, chess::cell(s.piece) as f32], b: [if s.piece.is_ascii_uppercase() { 0.0 } else { 1.0 }, alpha as f32, lift as f32, k] };
            if is_drag || lift > 0.05 { top.push(one) } else { inst.push(one) }
        }
        inst.extend(top);
        self.pieces.render(gpu, &mut enc, target, size, &inst);

        let ku = k * l.ui as f32;
        // over the pieces: the beads, the buttons, the misses, ripples, sparks
        for i in 0..self.drills.len() {
            let c = px(self.bead_pos(&l, i));
            let r = 5.2 * ku;
            let col = match self.drills[i].group.as_str() {
                "black" => [0.17, 0.15, 0.16, 1.0],
                "white" => [0.99, 0.96, 0.90, 1.0],
                _ => [0.96, 0.71, 0.27, 1.0],
            };
            let st = self.stats[i];
            if i == self.current {
                self.painter.ring(c, r + 4.0 * ku, 1.6 * ku, 0.8 * ku, [0.99, 0.96, 0.90, 0.9]);
            }
            if st.done == 0 {
                self.painter.ring(c, r - 0.9 * ku, 1.8 * ku, 0.6 * ku, col);
                if self.drills[i].group == "black" {
                    self.painter.ring(c, r + 0.4 * ku, 0.8 * ku, 0.6 * ku, [0.99, 0.96, 0.90, 0.35]);
                }
            } else {
                if self.drills[i].group == "black" {
                    self.painter.disc(c, r + 0.9 * ku, 0.6 * ku, [0.99, 0.96, 0.90, 0.35]);
                }
                self.painter.disc(c, r, 0.7 * ku, col);
                if st.last > 0 {
                    let d = [c[0], c[1] + r + 5.0 * ku];
                    self.painter.disc(d, 1.9 * ku, 0.6 * ku, [0.93, 0.36, 0.30, 1.0]);
                }
            }
        }
        // the buttons: a bulb (the hint) and a play mark (show it)
        let live = self.turn == Turn::Yours;
        for (kind, x) in [(0u8, l.hint_x), (1u8, l.show_x)] {
            let ku = ku * 1.2;
            let c = px([x, l.buttons_y]);
            let press = match self.pressed {
                Some((p, at)) if p == kind && t - at < 0.25 => 1.0 - (t - at) / 0.25,
                _ => 0.0,
            } as f32;
            let r = (l.button_r as f32) * k * (1.0 - 0.08 * press);
            let a = if live { 1.0 } else { 0.4 };
            self.painter.disc([c[0], c[1] + 3.0 * ku], r, 5.0 * ku, [0.0, 0.0, 0.0, 0.25 * a]);
            self.painter.disc(c, r, 0.8 * ku, [0.99, 0.96, 0.90, 0.10 + 0.08 * a + 0.2 * press]);
            self.painter.ring(c, r, 1.5 * ku, 0.8 * ku, [0.99, 0.96, 0.90, 0.55 * a]);
            let ink = [0.99, 0.94, 0.80, a];
            if kind == 0 {
                // a bulb: a round glass, two lines of its screw
                let g = [c[0], c[1] - 3.5 * ku];
                self.painter.ring(g, 7.0 * ku, 2.2 * ku, 0.7 * ku, ink);
                self.painter.line([c[0] - 3.5 * ku, c[1] + 7.0 * ku], [c[0] + 3.5 * ku, c[1] + 7.0 * ku], 1.2 * ku, 0.6 * ku, ink);
                self.painter.line([c[0] - 2.5 * ku, c[1] + 10.5 * ku], [c[0] + 2.5 * ku, c[1] + 10.5 * ku], 1.2 * ku, 0.6 * ku, ink);
                if self.hint_at.is_some() && live {
                    self.painter.glow(g, g, 6.0 * ku, 6.0 * ku, [1.0, 0.8, 0.3, 0.8]);
                }
            } else {
                // a play mark: a triangle, filled by strokes
                let (x0, x1, h) = (c[0] - 5.0 * ku, c[0] + 8.0 * ku, 8.5 * ku);
                for j in 0..6 {
                    let u = j as f32 / 5.0;
                    let y = c[1] - h * (1.0 - u) + 0.0;
                    let y2 = c[1] + h * (1.0 - u);
                    let x = x0 + (x1 - x0) * u * 0.92;
                    self.painter.line([x, y], [x, y2], 1.6 * ku, 0.6 * ku, ink);
                }
                self.painter.line([x0, c[1] - h], [x1, c[1]], 1.4 * ku, 0.6 * ku, ink);
                self.painter.line([x0, c[1] + h], [x1, c[1]], 1.4 * ku, 0.6 * ku, ink);
            }
        }
        // the misses so far: red pips between the buttons
        let n = self.misses.min(6) as usize;
        for i in 0..n {
            let x = l.show_x.min(l.hint_x) + (l.show_x - l.hint_x).abs() / 2.0 + (i as f64 - (n as f64 - 1.0) / 2.0) * 8.0 * l.ui;
            let c = px([x, l.buttons_y]);
            self.painter.disc(c, 3.0 * ku, 0.7 * ku, [0.93, 0.36, 0.30, 1.0]);
        }
        for r in &self.ripples {
            let u = ((t - r.born) / 0.6) as f32;
            self.painter.ring(px(r.p), (6.0 + 26.0 * u) * k, 2.0 * k * (1.0 - u), 1.0 * k, [r.color[0], r.color[1], r.color[2], r.color[3] * (1.0 - u)]);
        }
        for s in &self.sparks {
            if t < s.born || !s.placed {
                continue;
            }
            let u = ((t - s.born) / 1.8) as f32;
            let c = px(s.p);
            self.painter.disc(c, s.r * k * (1.0 - 0.5 * u), 0.8 * k, [s.color[0], s.color[1], s.color[2], (1.0 - u * u)]);
        }
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.bells.synth())
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&(self.current, &self.stats)).ok()
    }

    fn restore(&mut self, ctx: &mut Ctx, saved: &str) {
        if let Ok((cur, stats)) = serde_json::from_str::<(usize, Vec<Stat>)>(saved) {
            if stats.len() == self.drills.len() {
                self.stats = stats;
                if cur < self.drills.len() {
                    self.start(cur, ctx);
                }
            }
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        // the screen's size isn't kept between frames; the last pointer's layout is close enough for
        // plans, which run at one size: take it from the shown pieces' frame
        let points = self.points_hint();
        let l = self.layout(points);
        let mut a = Vec::new();
        for sq in 0..64u8 {
            a.push((chess::name(sq), self.sq_pos(&l, sq)));
        }
        a.push(("hint".into(), [l.hint_x, l.buttons_y]));
        a.push(("show".into(), [l.show_x, l.buttons_y]));
        for i in 0..self.drills.len() {
            a.push((format!("bead{i}"), self.bead_pos(&l, i)));
        }
        if self.turn == Turn::Yours {
            let (f, t) = self.drills[self.current].plies[self.ply].squares();
            a.push(("from".into(), self.sq_pos(&l, f)));
            a.push(("to".into(), self.sq_pos(&l, t)));
        }
        a
    }

    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        let l = self.layout(ctx.points());
        let strokes = if self.turn == Turn::Yours {
            let (f, t) = self.drills[self.current].plies[self.ply].squares();
            vec![Stroke::tap(0.7, self.sq_pos(&l, f)), Stroke::tap(1.7, self.sq_pos(&l, t))]
        } else {
            vec![Stroke::tap(0.7, [l.x0 + l.s / 2.0, l.y0 + l.s / 2.0])]
        };
        Some(Demo::new("Tap a piece, then where it goes.", strokes))
    }
}

impl Carol {
    /// The screen's size in points as last laid out (anchors are asked without a context).
    fn points_hint(&self) -> [f64; 2] {
        POINTS.with(|p| p.get())
    }
}

thread_local! {
    static POINTS: std::cell::Cell<[f64; 2]> = const { std::cell::Cell::new([390.0, 844.0]) };
}

/// Keeps the screen's size for `anchors` (called from `update`).
fn keep_points(p: [f64; 2]) {
    POINTS.with(|c| c.set(p));
}

// ---------------------------------------------------------------------------------------------
// The board, drawn in one fragment: the table, the frame's shadow, the frame, the squares, the
// last move and a wrong square.

const BOARD_WGSL: &str = r#"
fn sd_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
  let q = abs(p) - b + vec2<f32>(r, r);
  return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}
fn over(dst: vec3<f32>, src: vec3<f32>, a: f32) -> vec3<f32> { return mix(dst, src, clamp(a, 0.0, 1.0)); }
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let k = u.scale;
  let o = u.p0.xy; let s = u.p0.z; let fw = u.p0.w;
  let c = o + vec2<f32>(s, s) * 0.5;
  // the table: a deep slate, lighter where the board sits
  let v = px / u.size;
  var col = mix(vec3<f32>(0.20, 0.27, 0.34), vec3<f32>(0.11, 0.15, 0.20), v.y);
  let lamp = exp(-dot((px - c) / (s * 0.9), (px - c) / (s * 0.9)));
  col = col + vec3<f32>(0.05, 0.05, 0.045) * lamp;
  col = col * (0.96 + 0.04 * noise2(px / (3.0 * k)));
  // the frame's shadow, then the frame (wood, lighter at its top)
  let half = vec2<f32>(s * 0.5 + fw, s * 0.5 + fw);
  let ds = sd_box(px - c - vec2<f32>(0.0, 7.0 * k), half, fw * 1.1);
  col = over(col, vec3<f32>(0.02, 0.03, 0.05), 0.55 * (1.0 - smoothstep(-6.0 * k, 22.0 * k, ds)));
  let df = sd_box(px - c, half, fw * 1.1);
  let fa = clamp(0.5 - df / 1.2, 0.0, 1.0);
  let gy = clamp((px.y - (c.y - half.y)) / (2.0 * half.y), 0.0, 1.0);
  var wood = mix(vec3<f32>(0.62, 0.40, 0.24), vec3<f32>(0.47, 0.29, 0.17), gy);
  wood = wood * (0.94 + 0.06 * noise2(vec2<f32>(px.x / (40.0 * k), px.y / (2.0 * k))));
  // the frame's rim catches the light
  wood = wood + vec3<f32>(0.10, 0.07, 0.04) * (1.0 - smoothstep(0.0, 2.5 * k, abs(df + 1.5 * k)));
  // a finished drill gilds it
  wood = mix(wood, vec3<f32>(0.98, 0.78, 0.36), u.center.x * 0.55);
  col = over(col, wood, fa);
  // the squares
  let q = (px - o) / (s / 8.0);
  if (q.x >= 0.0 && q.y >= 0.0 && q.x < 8.0 && q.y < 8.0) {
    let i = floor(q);
    let light = (i32(i.x) + i32(i.y)) % 2 == 0;
    var sq = select(vec3<f32>(0.71, 0.51, 0.31), vec3<f32>(0.95, 0.86, 0.69), light);
    sq = sq * (0.95 + 0.05 * noise2(vec2<f32>(px.x / (60.0 * k) + i.y * 7.0, px.y / (3.0 * k) + i.x * 3.0)));
    let id = i.y * 8.0 + i.x;
    if (id == u.p1.x || id == u.p1.y) {
      sq = mix(sq, vec3<f32>(0.98, 0.80, 0.33), select(0.42, 0.55, id == u.p1.y));
    }
    if (id == u.p1.z) {
      sq = mix(sq, vec3<f32>(0.91, 0.30, 0.25), 0.85 * u.p1.w);
    }
    // a soft inner edge round every square, as a toy board's grooves
    let e = min(min(fract(q.x), 1.0 - fract(q.x)), min(fract(q.y), 1.0 - fract(q.y))) * (s / 8.0);
    sq = sq * (0.96 + 0.04 * smoothstep(0.0, 2.0 * k, e));
    // inset: the board's edge in shadow under the frame
    let inner = sd_box(px - c, vec2<f32>(s * 0.5, s * 0.5), 0.0);
    sq = sq * (1.0 - 0.18 * (1.0 - smoothstep(-5.0 * k, 0.0, inner)));
    col = sq;
  }
  return vec4<f32>(col, 1.0);
}
"#;

// ---------------------------------------------------------------------------------------------
// The pieces: one instanced pass of quads over the atlas (tools/toys/carol/pieces.py: R the
// silhouette, G its rim, B its shadow), coloured for either side.

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PieceInst {
    /// Centre (px), half size (px), atlas cell.
    a: [f32; 4],
    /// Side (0 white, 1 black), alpha, lift (0..1), pixels a point.
    b: [f32; 4],
}

const PIECES_WGSL: &str = r#"
struct U { size: vec2<f32>, _p: vec2<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
struct I { @location(0) a: vec4<f32>, @location(1) b: vec4<f32> };
struct O { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) cell: f32, @location(2) b: vec4<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32, i: I) -> O {
  let corner = array<vec2<f32>, 6>(vec2(-1.0,-1.0), vec2(1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,-1.0), vec2(1.0,1.0), vec2(-1.0,1.0))[vi];
  // the quad reaches a little further than the cell, for the shadow
  let grow = 1.25;
  let p = i.a.xy + corner * i.a.z * grow;
  var o: O;
  o.pos = vec4<f32>(p.x / u.size.x * 2.0 - 1.0, 1.0 - p.y / u.size.y * 2.0, 0.0, 1.0);
  o.uv = corner * grow * 0.5 + vec2<f32>(0.5, 0.5);
  o.cell = i.a.w;
  o.b = i.b;
  return o;
}
fn tap(uv: vec2<f32>, cell: f32) -> vec4<f32> {
  if (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) { return vec4<f32>(0.0); }
  return textureSampleLevel(atlas, samp, vec2<f32>((cell + uv.x) / 6.0, uv.y), 0.0);
}
@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  let lift = i.b.z;
  let here = tap(i.uv, i.cell);
  // the shadow falls further as the piece is lifted
  let sh = tap(i.uv - vec2<f32>(0.0, 0.035 + 0.06 * lift), i.cell).z * (0.42 - 0.15 * lift);
  let black = i.b.x > 0.5;
  var fill = select(vec3<f32>(0.99, 0.95, 0.87), vec3<f32>(0.17, 0.15, 0.16), black);
  let rim = select(vec3<f32>(0.24, 0.17, 0.13), vec3<f32>(0.04, 0.03, 0.04), black);
  // lit from above: lighter toward the top
  fill = fill * (1.0 + select(0.10, 0.45, black) * (0.55 - i.uv.y));
  var col = vec4<f32>(0.0, 0.0, 0.0, sh * (1.0 - here.x));
  col = vec4<f32>(fill, 1.0) * here.x + col * (1.0 - here.x);
  col = vec4<f32>(mix(col.rgb, rim * here.x, here.y * select(0.9, 0.7, black)), col.a);
  return col * i.b.y;
}
"#;

pub struct PiecePass {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
}

impl PiecePass {
    fn new(gpu: &Gpu, atlas: &Texture) -> PiecePass {
        let dev = &gpu.device;
        let shader = gpu.shader("carol pieces", PIECES_WGSL);
        let uniform = gpu.buffer(&wgpu::BufferDescriptor { label: Some("pieces u"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("pieces"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pieces"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pieces"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&atlas.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let layout = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("pieces"), bind_group_layouts: &[&bgl], push_constant_ranges: &[] });
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pieces"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<PieceInst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        PiecePass { pipeline, bind, uniform }
    }

    fn render(&self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, size: [u32; 2], inst: &[PieceInst]) {
        use wgpu::util::DeviceExt;
        if inst.is_empty() {
            return;
        }
        gpu.queue.write_buffer(&self.uniform, 0, bytemuck::cast_slice(&[size[0] as f32, size[1] as f32, 0.0, 0.0]));
        let buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("pieces"), contents: bytemuck::cast_slice(inst), usage: wgpu::BufferUsages::VERTEX });
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pieces"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        pass.draw(0..6, 0..inst.len() as u32);
    }
}
