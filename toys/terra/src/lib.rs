//! Terra Incognita: the toy that holds the others (painting-company row 217). The whole Earth, turning
//! slowly, and on it a plain light of each toy's colour at the place where it is real (`doors`). Touch a
//! light: it stirs and rings, and its postcard rises with the toy already going in it, its name, its place
//! and one line (`card`). Touch the card: it grows until the toy is the whole screen (`rooms`). The Earth
//! mark, bottom left, shrinks the toy back into its card over the Earth where the reader was; a touch on
//! the Earth elsewhere, or a swipe down on the card, puts the card away. (Row 351, Nov 15 2026: the dive
//! through the Earth into each toy, its likenesses and windows, gave way to this, which a new toy needs
//! nothing for but its place, its colour and its words.)
//!
//! Only one toy is open at a time; putting its card away frees it, and a toy opened afresh starts as new.

mod atlas;
pub mod card;
mod clock;
pub mod doors;
mod mixer;
pub mod rooms;
mod veil;
mod words;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use toy_kit::info::{Demo, Rules};
use toy_kit::paint::Painter;
use toy_kit::sound::{pentatonic, Bells, Note};
use toy_kit::{Assets, Camera, Ctx, Globe, Label, MemoryAssets, Phase, PointerEvent, Synth, Toy};

use doors::Life;
use mixer::Mixer;
use rooms::{Room, Shared};
use veil::Veil;

/// The Earth mark: bottom left, as the toy's own mark is bottom right (toy-kit info.rs).
fn earth_spot(points: [f64; 2]) -> [f64; 2] {
    let m = toy_kit::info::Info::mark_scale(points);
    [26.0 * m, points[1] - 34.0 * m]
}
const EARTH_REACH: f64 = 24.0;
/// The opening's zoom: on a tall phone the Earth a little wider than the screen, so its lights are large
/// (1.4, its height well inside); on a squarer screen (the iPad) the Earth just the screen's width, so no
/// light opens cut by the frame or hidden past it (painting-company row 246: at 1.4 the 13-inch iPad's
/// Earth ran off all four sides). The kit's disc radius is (short side / 2 - 14) x zoom.
pub fn opening_zoom(points: [f64; 2]) -> f64 {
    let (w, h) = (points[0].min(points[1]), points[0].max(points[1]));
    if points[0] > points[1] && w > 600.0 {
        // on its side (a laptop's browser, however wide; a landscape iPad): the whole Earth inside the
        // height, lights and all (a phone on its side keeps the phone's)
        return (w * 0.45) / (w / 2.0 - 14.0);
    }
    if h / w > 1.8 {
        return 1.4;
    }
    ((w * 0.53) / (w / 2.0 - 14.0).max(40.0)).min(1.4)
}

enum State {
    /// The Earth and its lights, and a toy's card if one is up.
    Atlas,
    /// The card growing to the glass.
    Opening { t0: f64 },
    /// In the toy.
    Inside,
    /// The toy shrinking back into its card.
    Closing { t0: f64 },
}

/// A toy's card over the Earth: when it rose, and since when it has been going away.
#[derive(Clone, Copy)]
struct Up {
    t0: f64,
    gone: Option<f64>,
}

/// The camera on its way somewhere (a light brought above its card, a gathering come down toward).
#[derive(Clone, Copy)]
struct Flight {
    from: Camera,
    to: Camera,
    t0: f64,
    dur: f64,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    camera: Camera,
    known: Option<Vec<String>>, // the toys no longer new (`doors/fresh.rs`); None: a save from before
}

pub struct Terra {
    globe: Globe,
    painter: Painter,
    assets: Arc<dyn Assets>,
    life: Life,
    bells: Bells,
    room_sound: Arc<Mutex<Option<Box<dyn Synth>>>>,
    room_level: Arc<AtomicU32>,
    room: Option<Box<dyn Room>>,
    /// The held toy's readiness (`rooms::Warming`): when it came ready, if after its card rose, and a touch
    /// that asked to go in before.
    warming: rooms::Warming,
    /// The light whose toy is held (its card up, or open).
    door: usize,
    seen: std::collections::HashSet<usize>, // toys opened since launch: a first opening shows its demo
    state: State,
    card: Option<Up>,
    flight: Option<Flight>,
    veil: Veil,
    t: f64,
    last_touch: f64,
    /// Fingers in a globe toy, and a two-finger pinch's start (their spread, the toy's zoom): pinched out
    /// past the whole Earth, the toy goes back into its card.
    inside_fingers: std::collections::HashMap<u64, [f64; 2]>,
    pinch: Option<(f64, f64)>,
    /// Fingers down on the Earth: where and when, and whether they moved (a tap is still and short).
    downs: std::collections::HashMap<u64, ([f64; 2], f64, bool)>,
    /// A finger on the card: where it went down and when.
    card_finger: Option<(u64, [f64; 2], f64)>,
    /// A finger on the Earth mark.
    earth_finger: Option<u64>,
    reduce_motion: bool,
    /// A passage played by itself (`TERRA_PASSAGE=<toy id>`, set by the Simulator's check: sim.env):
    /// that toy's card a few seconds after launch, in, and out again; with the first frame's time.
    auto: Option<(usize, f64, u8)>,
    /// The idle turn's pace now, degrees a second (eased toward what the lights in view ask).
    drift: f64,
}

fn ease(u: f64) -> f64 {
    card::ease(u)
}

/// A camera on the way from `a` to `b` (`u` 0..1, eased): the centre along the great circle, the zoom
/// in its logarithm.
fn fly(a: Camera, b: Camera, u: f64) -> Camera {
    let e = ease(u);
    let (pa, pb) = (
        toy_kit::LonLat::new(a.lon, a.lat).to_xyz(),
        toy_kit::LonLat::new(b.lon, b.lat).to_xyz(),
    );
    let w = (pa[0] * pb[0] + pa[1] * pb[1] + pa[2] * pb[2])
        .clamp(-1.0, 1.0)
        .acos();
    let ll = if w < 1e-6 {
        toy_kit::LonLat::new(b.lon, b.lat)
    } else {
        let (sa, sb) = (((1.0 - e) * w).sin() / w.sin(), (e * w).sin() / w.sin());
        toy_kit::LonLat::from_xyz([
            pa[0] * sa + pb[0] * sb,
            pa[1] * sa + pb[1] * sb,
            pa[2] * sa + pb[2] * sb,
        ])
    };
    Camera {
        lon: ll.lon,
        lat: ll.lat,
        zoom: (a.zoom.ln() * (1.0 - e) + b.zoom.ln() * e).exp(),
        roll: a.roll * (1.0 - e) + b.roll * e,
    }
}

impl Terra {
    fn strike(&self, degree: i32, amp: f32, pan: f32) {
        self.bells.strike(Note {
            freq: pentatonic(392.0, degree),
            amp,
            decay: 2.2,
            pan,
            bright: 0.9,
        });
    }

    /// How far the card is grown (0 a card .. 1 the glass), and how far risen (0 below the screen .. 1 at rest).
    fn growth(&self) -> (f64, f64) {
        let g = match self.state {
            State::Atlas => 0.0,
            State::Opening { t0 } => ((self.t - t0) / self.grow_time()).clamp(0.0, 1.0),
            State::Inside => 1.0,
            State::Closing { t0 } => 1.0 - ((self.t - t0) / self.grow_time()).clamp(0.0, 1.0),
        };
        let rise = match self.card {
            Some(Up { gone: Some(t1), .. }) => 1.0 - (self.t - t1) / card::RISE,
            Some(Up { t0, .. }) if !self.reduce_motion => (self.t - t0) / card::RISE,
            Some(_) => 1.0,
            None => 0.0,
        };
        (g, rise.clamp(0.0, 1.0))
    }

    fn grow_time(&self) -> f64 {
        if self.reduce_motion {
            0.2
        } else {
            card::GROW
        }
    }

    /// The Earth with its lights (dimmed by `fade`).
    fn draw_atlas(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView, fade: f64) {
        self.globe.upload(&ctx.gpu);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.globe.draw(&mut rp);
        }
        let chosen = self.card.map(|_| self.door);
        self.life.paint(
            &mut self.painter,
            &self.globe,
            self.t,
            ctx.ppp,
            fade,
            self.reduce_motion,
            chosen,
        );
        self.painter
            .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    /// The held toy in its card, grown `g` of the way to the glass.
    fn draw_card(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let (g, rise) = self.growth();
        let Some(r) = self.room.as_mut() else { return };
        let ready = r.ready();
        let view = self.veil.view(ctx);
        r.draw(view);
        // (a toy not yet ready shows its card's picture dark, of its colour; then fades in)
        let dark = self.warming.dark(ready, self.t);
        if dark > 0.0 {
            let tint = self.life.doors[self.door].tint;
            let (w, h) = (ctx.width as f32, ctx.height as f32);
            let ink = [tint[0] * 0.12, tint[1] * 0.12, tint[2] * 0.12, dark];
            self.painter.disc([w / 2.0, h / 2.0], w.hypot(h), 0.0, ink);
            let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
            self.painter
                .render(&ctx.gpu, &mut enc, view, [ctx.width, ctx.height], None);
            ctx.gpu.queue.submit([enc.finish()]);
        }
        let l = card::lay(
            ctx.points(),
            ctx.ppp,
            card::focus(self.life.doors[self.door].id),
            rise,
            g,
            1.0,
        );
        self.veil.lay(ctx, target, &l);
    }

    /// The Earth mark (a small globe: its rim, a meridian, the equator), on the marks' dark ground.
    fn draw_earth_mark(&mut self, ctx: &Ctx, target: &wgpu::TextureView, alpha: f32) {
        let k = ctx.ppp as f32;
        let s = earth_spot(ctx.points());
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        let k = k * toy_kit::info::Info::mark_scale(ctx.points()) as f32;
        let lit = 0.62 * alpha;
        let p = &mut self.painter;
        p.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42 * alpha]);
        p.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        p.line(
            [c[0] - 6.9 * k, c[1]],
            [c[0] + 6.9 * k, c[1]],
            0.5 * k,
            0.7,
            [1.0, 1.0, 1.0, lit],
        );
        // the meridian: an ellipse's two halves, drawn as short strokes
        let n = 14;
        let mut prev: Option<[f32; 2]> = None;
        for i in 0..=n {
            let a = std::f32::consts::PI * (i as f32 / n as f32 - 0.5);
            let q = [c[0] + a.cos() * 3.4 * k, c[1] + a.sin() * 7.2 * k];
            if let Some(pp) = prev {
                p.line(pp, q, 0.5 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            }
            prev = Some(q);
        }
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.painter
            .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    /// Inside: the Earth mark's finger (true if it was the mark's); lifted on it, the toy goes back into its card.
    fn earth_mark_pointer(&mut self, ctx: &mut Ctx, e: &PointerEvent) -> bool {
        let s = earth_spot(ctx.points());
        let m = toy_kit::info::Info::mark_scale(ctx.points());
        let near = |k: f64| (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH * m * k;
        if e.phase == Phase::Down && self.earth_finger.is_none() && near(1.0) {
            self.earth_finger = Some(e.id);
            return true;
        }
        if self.earth_finger != Some(e.id) {
            return false;
        }
        if matches!(e.phase, Phase::Up | Phase::Cancel) {
            self.earth_finger = None;
            if e.phase == Phase::Up && near(1.5) && matches!(self.state, State::Inside) {
                self.leave(ctx, e.t);
            }
        }
        true
    }

    /// Whether this finger ends a pinch that took a globe toy out past its whole Earth (as the kit's
    /// hand's own over-pinch would fly it Home): pinched to below the whole Earth (zoom 0.8), it goes back
    /// into its card.
    fn pinched_out(&mut self, e: &PointerEvent) -> bool {
        let spread = |f: &std::collections::HashMap<u64, [f64; 2]>| -> Option<f64> {
            let v: Vec<&[f64; 2]> = f.values().collect();
            (v.len() == 2).then(|| (v[0][0] - v[1][0]).hypot(v[0][1] - v[1][1]).max(1.0))
        };
        match e.phase {
            Phase::Down => {
                self.inside_fingers.insert(e.id, [e.x, e.y]);
                self.pinch = match (
                    spread(&self.inside_fingers),
                    self.room.as_ref().and_then(|r| r.camera()),
                ) {
                    (Some(d), Some(cam)) => Some((d, cam.zoom)),
                    _ => None,
                };
                false
            }
            Phase::Move => {
                if let Some(f) = self.inside_fingers.get_mut(&e.id) {
                    *f = [e.x, e.y];
                }
                false
            }
            Phase::Up | Phase::Cancel => {
                let out = match (self.pinch, spread(&self.inside_fingers)) {
                    // pinched in, ending at (or toward) the toy's whole Earth: its own hand stops a pinch at its
                    // least zoom, so the toy's camera is asked too, not only the fingers' spread
                    (Some((d0, z0)), Some(d)) => {
                        let now = self
                            .room
                            .as_ref()
                            .and_then(|r| r.camera())
                            .map_or(z0, |c| c.zoom);
                        e.phase == Phase::Up
                            && d / d0 < 0.85
                            && (z0 * d / d0 < 0.8
                                || now < 1.15
                                || (d / d0 < 0.65 && now > z0 * 0.85))
                    }
                    _ => false,
                };
                self.inside_fingers.remove(&e.id);
                self.pinch = None;
                out
            }
        }
    }

    /// The lights facing the reader now, each toy's alone, with where they are (points).
    fn visible_doors(&self) -> Vec<(usize, [f64; 2])> {
        self.life
            .shown(&self.globe)
            .into_iter()
            .filter(|s| s.facing > 0.35 && s.doors.len() == 1)
            .map(|s| (s.doors[0], s.at))
            .collect()
    }

    /// Terra's buttons for VoiceOver now: the card (to open it), then the lights shown alone.
    fn button_list(&self, ctx: &Ctx) -> Vec<(String, [f64; 2], Option<usize>)> {
        let mut out = Vec::new();
        if self.card.is_some_and(|c| c.gone.is_none()) {
            let s = card::shape(ctx.points()).pic;
            let mid = [(s[0] + s[2]) / 2.0, (s[1] + s[3]) / 2.0];
            let d = &self.life.doors[self.door];
            out.push((format!("Open {}", d.name), mid, None));
        }
        for (i, p) in self.visible_doors() {
            out.push((self.life.doors[i].name.to_string(), p, Some(i)));
        }
        out
    }
}

impl Toy for Terra {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        // the data shared with every toy Terra opens
        let assets: Arc<dyn Assets> = Arc::from(std::mem::replace(
            &mut ctx.assets,
            Box::new(MemoryAssets(Default::default())),
        ));
        ctx.assets = Box::new(Shared(assets.clone()));
        let month = clock::this_month();
        let mut globe = Globe::new(
            &ctx.gpu,
            assets.as_ref(),
            ctx.width,
            ctx.height,
            ctx.ppp,
            month,
        )?;
        // (no rivers: their ink over the whole Earth read as a little creepy, Nov 13; the kit's
        // `with_rivers` brings them back)
        // over the Atlantic's west: the Americas' lights and Europe's in view, the Pacific's coming round
        globe.camera = Camera::new(-66.0, 20.0, opening_zoom(ctx.points()));
        // (an opening under the whole-screen Earth stays there: the hand's coast would ease it up to 1)
        globe.hand.rest_zoom = globe.camera.zoom.min(1.0);
        globe.set_home();
        let mut life = Life::new();
        let [w, h] = ctx.points();
        life.beckon = life.nearest(&globe, [w / 2.0, h / 2.0]);
        Ok(Terra {
            globe,
            painter: Painter::new(&ctx.gpu),
            assets,
            life,
            bells: Bells::new(),
            room_sound: Arc::new(Mutex::new(None)),
            room_level: Arc::new(AtomicU32::new(0f32.to_bits())),
            room: None,
            warming: Default::default(),
            door: 0,
            seen: Default::default(),
            state: State::Atlas,
            card: None,
            flight: None,
            veil: Veil::new(ctx),
            t: 0.0,
            last_touch: -100.0,
            downs: Default::default(),
            card_finger: None,
            earth_finger: None,
            inside_fingers: Default::default(),
            pinch: None,
            reduce_motion: ctx.reduce_motion,
            auto: std::env::var("TERRA_PASSAGE")
                .ok()
                .and_then(|id| rooms::life_index(&id))
                .map(|i| (i, f64::NAN, 0)),
            drift: 1.6,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
        if let Some(r) = self.room.as_mut() {
            r.resize(ctx.width, ctx.height, ctx.ppp);
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match self.state {
            State::Atlas => self.atlas_pointer(ctx, e),
            State::Inside => {
                // the Earth mark is Terra's; everything else is the toy's
                if self.earth_mark_pointer(ctx, &e) {
                    return;
                }
                let out = self.room.as_ref().is_some_and(|r| r.camera().is_some())
                    && self.pinched_out(&e);
                if let Some(r) = self.room.as_mut() {
                    r.pointer(e);
                }
                if out {
                    self.leave(ctx, e.t);
                }
            }
            State::Opening { .. } | State::Closing { .. } => self.last_touch = e.t,
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.auto_step(ctx, t);
        let held = self.globe.camera;
        self.globe.update(&ctx.gpu, self.assets.as_ref(), t, dt);
        if !matches!(self.state, State::Atlas) {
            self.globe.camera = held;
        }
        self.life.update(t, dt);
        self.warm_room(t, false);
        if let Some(r) = self.room.as_mut() {
            r.sense(ctx.sensors);
            r.update(t);
        }
        match self.state {
            State::Atlas => self.atlas_step(ctx, t, dt),
            State::Opening { t0 } => {
                if t - t0 >= self.grow_time() {
                    self.settle_inside(ctx);
                }
            }
            State::Inside => {}
            State::Closing { t0 } => {
                if t - t0 >= self.grow_time() {
                    self.state = State::Atlas;
                    self.last_touch = t;
                }
            }
        }
        let (g, rise) = self.growth();
        let level = if self.room.is_some() {
            0.35 * rise + 0.65 * g
        } else {
            0.0
        };
        self.room_level
            .store((level as f32).to_bits(), Ordering::Relaxed);
        if let Some(r) = self.room.as_mut() {
            if r.take_haptic() {
                ctx.haptic();
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let (g, _) = self.growth();
        match self.state {
            State::Inside => {
                if let Some(r) = self.room.as_mut() {
                    r.draw(target);
                }
                self.draw_earth_mark(ctx, target, 1.0);
            }
            _ => {
                self.draw_atlas(ctx, target, 1.0 - 0.5 * g);
                if self.card.is_some() {
                    self.draw_card(ctx, target);
                }
                if g > 0.0 {
                    self.draw_earth_mark(ctx, target, ease((g - 0.6) / 0.4) as f32);
                }
            }
        }
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        // a shake is the open toy's (Frost sweeps its window clear); on the Earth, nothing
        if matches!(self.state, State::Inside) {
            if let Some(r) = self.room.as_mut() {
                r.shaken();
            }
        }
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(Box::new(Mixer {
            bells: self.bells.synth(),
            room: self.room_sound.clone(),
            want: self.room_level.clone(),
            level: 0.0,
            buf: Vec::new(),
        }))
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
        self.globe.hand.reduce_motion = ctx.reduce_motion;
        if let Some(r) = self.room.as_mut() {
            r.settings(ctx.night, ctx.reduce_motion);
        }
    }

    fn save(&self) -> Option<String> {
        // the Earth as it was left (a toy open is put away on the way out: Terra reopens on the Earth)
        let camera = self.globe.camera;
        serde_json::to_string(&Saved {
            camera: Camera {
                zoom: camera.zoom.min(2.5),
                ..camera
            },
            known: Some(self.life.known()),
        })
        .ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            let new = self.life.know(s.known.as_deref());
            if matches!(self.state, State::Atlas) {
                // turned toward a toy an update brought, so its mark is in view
                self.globe.camera =
                    new.map_or(s.camera, |at| Camera::new(at[0], at[1], s.camera.zoom));
            }
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        if self.inside() {
            let mut a = self.room.as_ref().map(|r| r.anchors()).unwrap_or_default();
            a.push(("earth".into(), earth_spot(self.globe_points())));
            return a;
        }
        // each toy's light by its id (a gathered one's at its gathering; one under the card is out of
        // reach), and the card
        let pts = self.globe_points();
        let up = self.card.is_some_and(|c| c.gone.is_none());
        let mut a: Vec<(String, [f64; 2])> = self
            .life
            .shown(&self.globe)
            .into_iter()
            .filter(|s| s.facing > 0.35 && !(up && card::on_card(pts, s.at)))
            .flat_map(|s| {
                s.doors
                    .iter()
                    .map(|&i| (self.life.doors[i].id.to_string(), s.at))
                    .collect::<Vec<_>>()
            })
            .collect();
        if up {
            let p = card::shape(pts).pic;
            a.push(("card".into(), [(p[0] + p[2]) / 2.0, (p[1] + p[3]) / 2.0]));
        }
        a
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.project(lonlat));
        }
        self.globe.point(lonlat)
    }

    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.demo());
        }
        // a fingertip touches the light nearest the middle
        let [w, h] = ctx.points();
        let at = self
            .life
            .nearest(&self.globe, [w / 2.0, h / 2.0])
            .map(|i| self.life.spot(&self.globe, i).0)
            .unwrap_or([w / 2.0, h / 2.0]);
        Some(words::earth_demo(at))
    }

    fn label(&self, ctx: &Ctx) -> Option<Label> {
        if matches!(self.state, State::Inside) || self.room.is_none() {
            return None;
        }
        self.card?;
        let (g, rise) = self.growth();
        let s = card::shape(ctx.points());
        let dy = (1.0 - ease(rise)) * (ctx.points()[1] - s.card[1] + 24.0);
        let d = &self.life.doors[self.door];
        Some(Label {
            at: [
                s.words[0],
                s.words[1] + dy,
                s.words[2] - s.words[0],
                s.words[3] - s.words[1],
            ],
            alpha: (1.0 - ease(g / 0.25)) * ease((rise - 0.5) / 0.5),
            dark: true,
            lines: vec![d.name.into(), d.place.into(), d.line.into()],
        })
    }

    fn buttons(&self, ctx: &Ctx) -> Vec<(String, [f64; 2])> {
        if self.inside() {
            return vec![("Back to the Earth".into(), earth_spot(ctx.points()))];
        }
        self.button_list(ctx)
            .into_iter()
            .map(|(l, p, _)| (l, p))
            .collect()
    }

    fn press(&mut self, ctx: &mut Ctx, i: usize, t: f64) {
        if self.inside() {
            if matches!(self.state, State::Inside) {
                self.leave(ctx, t);
            }
            return;
        }
        if let Some((_, _, what)) = self.button_list(ctx).get(i).cloned() {
            match what {
                None => self.open_card(t),
                Some(d) => self.raise_card(ctx, d, t),
            }
        }
    }

    fn origins(&self) -> Option<Rules> {
        self.held_origins()
    }

    fn style(&self) -> Option<toy_kit::info::Style> {
        self.inside()
            .then(|| self.room.as_ref().and_then(|r| r.style()))
            .flatten()
    }

    fn switches(&self) -> Vec<toy_kit::switches::Switch> {
        self.held_switches()
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        self.set_held_switch(i, on);
    }

    fn info_at_start(&self) -> bool {
        false
    }

    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        if !self.inside() {
            // a toy's card up over the Earth: the sheet hangs from the top, clear of it, not over it
            return self.card.is_some_and(|c| c.gone.is_none());
        }
        self.room.as_ref().is_some_and(|r| r.card_at_top())
    }

    fn rules(&self) -> Option<Rules> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.rules());
        }
        Some(words::earth_rules())
    }
}

impl Terra {
    fn globe_points(&self) -> [f64; 2] {
        let d = self.globe.view();
        [d.width as f64 / d.ppp, d.height as f64 / d.ppp]
    }

    /// The Simulator's passage (`auto`): the toy's card four seconds in, in two seconds later, out at 45.
    fn auto_step(&mut self, ctx: &mut Ctx, t: f64) {
        let Some((i, t0, step)) = self.auto else {
            return;
        };
        let t0 = if t0.is_nan() { t } else { t0 };
        self.auto = Some((i, t0, step));
        let atlas = matches!(self.state, State::Atlas);
        // (`TERRA_PASSAGE`: steps 0 to 2, the card at 4 s, in at 6 s, out at 45 s; a web link's: 10 and 11,
        // the card at 1 s and in at 1.6 s, staying; a reader who went elsewhere first is not taken anywhere)
        let next = match step {
            0 if t - t0 > 4.0 && atlas => {
                self.raise_card(ctx, i, t);
                1
            }
            1 if t - t0 > 6.0 && atlas => {
                self.open_card(t);
                2
            }
            2 if t - t0 > 45.0 && matches!(self.state, State::Inside) => {
                self.leave(ctx, t);
                3
            }
            10 if t - t0 > 1.0 => {
                if atlas && self.card.is_none() {
                    self.raise_card(ctx, i, t);
                    11
                } else {
                    3
                }
            }
            11 if t - t0 > 1.6 => {
                if atlas && self.door == i {
                    self.open_card(t);
                }
                3
            }
            _ => step,
        };
        self.auto = Some((i, t0, next));
    }
}

toy_kit::ios_shell!(host Terra);
