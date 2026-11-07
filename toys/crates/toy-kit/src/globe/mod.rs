//! The globe a toy may stand on: the real ground of the month (or the night and its lights) drawn in
//! one pass, a camera a finger turns and pinches, and the `View` uniform every drawing shader takes as
//! its group 0. A toy that wants no globe never makes one.
//!
//! Drawing over it: write a pass whose WGSL starts with `toy_kit::globe::wgsl(own)` (the `View`
//! struct and the shared helpers of `common.wgsl`), bind `globe.view_bind()` at group 0, and use
//! `globe.view()` on the CPU to place things (`disc.project(&camera, lonlat)`).

pub mod ground;
pub mod rivers;
pub mod sheet;

use crate::assets::Assets;
use crate::gpu::{Gpu, Textures, View, ViewUniform, VIEW_WGSL};
use crate::model::{Camera, Clock, Disc, LonLat, Phase, PointerEvent};
use std::collections::HashMap;

pub const COMMON_WGSL: &str = include_str!("common.wgsl");

/// A pass's WGSL with the `View` struct and the shared helpers before it.
pub fn wgsl(own: &str) -> String {
    format!("{VIEW_WGSL}\n{COMMON_WGSL}\n{own}")
}

/// The view's bind group layout (group 0 of every pass): one uniform buffer, `ViewUniform`.
pub fn view_layout(gpu: &Gpu) -> wgpu::BindGroupLayout {
    gpu.device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        })
}

/// The ground, its textures, the camera and the hand that turns it.
pub struct Globe {
    pub textures: Textures,
    pub ground: ground::GroundPass,
    pub view_layout: wgpu::BindGroupLayout,
    view_buf: wgpu::Buffer,
    view_bind: wgpu::BindGroup,
    pub camera: Camera,
    pub clock: Clock,
    /// 1 by night (the lights), 0 by day; a toy may ease it.
    pub night: f32,
    /// The drawn motion's seconds (the shimmer of the lights by night).
    pub seconds: f64,
    /// The hand: on by default; a toy that reads fingers itself turns it off.
    pub hand: Hand,
    /// The world's rivers over the ground, shown by the scale ([`Globe::with_rivers`]; None: none).
    pub rivers: Option<rivers::Rivers>,
    /// Pictures of the ground laid at their true places over the ground and its rivers, the first lowest
    /// ([`Globe::set_sheets`]).
    pub sheets: Vec<sheet::SheetPass>,
    width: u32,
    height: u32,
    ppp: f64,
}

impl Globe {
    /// The globe over the month `month` (0 = January).
    pub fn new(
        gpu: &Gpu,
        assets: &dyn Assets,
        width: u32,
        height: u32,
        ppp: f64,
        month: u32,
    ) -> anyhow::Result<Globe> {
        crate::step("the globe's textures");
        let textures = Textures::load(gpu, assets, month)?;
        crate::step("the globe's ground pass");
        let view_layout = view_layout(gpu);
        let ground = ground::GroundPass::new(gpu, &view_layout, &textures);
        let view_buf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: std::mem::size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: view_buf.as_entire_binding(),
            }],
        });
        let day = month as f64 * 30.44 + 15.0;
        Ok(Globe {
            textures,
            ground,
            view_layout,
            view_buf,
            view_bind,
            camera: Camera::new(-30.0, 25.0, 1.0),
            clock: Clock {
                day,
                running: false,
                rate: 0.0,
            },
            night: 0.0,
            seconds: 0.0,
            hand: Hand::default(),
            rivers: None,
            sheets: Vec::new(),
            width,
            height,
            ppp,
        })
    }

    /// The world's rivers over the ground (`rivers.bin` in the toy's data): from afar the great rivers,
    /// closer more and more of their tributaries (`rivers`).
    pub fn with_rivers(&mut self, gpu: &Gpu, assets: &dyn Assets) -> anyhow::Result<()> {
        crate::step("the globe's rivers");
        self.rivers = Some(rivers::Rivers::new(
            gpu,
            &self.view_layout,
            assets,
            &self.textures,
        )?);
        Ok(())
    }

    /// Lays pictures of the ground at their places (`Toy::seen_from_above`'s sheet, a wider one under it), the
    /// first lowest, or takes them away (empty). A sheet already laid is kept as it is (its picture loaded
    /// once).
    pub fn set_sheets(
        &mut self,
        gpu: &Gpu,
        assets: &dyn Assets,
        sheets: &[crate::Sheet],
    ) -> anyhow::Result<()> {
        if self.sheets.len() == sheets.len()
            && self.sheets.iter().zip(sheets).all(|(p, s)| &p.sheet == s)
        {
            return Ok(());
        }
        let mut old = std::mem::take(&mut self.sheets);
        for s in sheets {
            match old.iter().position(|p| &p.sheet == s) {
                Some(i) => self.sheets.push(old.remove(i)),
                None => self.sheets.push(sheet::SheetPass::new(
                    gpu,
                    &self.view_layout,
                    assets,
                    &self.textures,
                    s.clone(),
                )?),
            }
        }
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.width, self.height, self.ppp) = (width, height, ppp);
    }

    /// The reading scale: 1.0 on a phone, 1.2 on the 13-inch iPad (Terra's `reading`).
    pub fn reading(&self) -> f64 {
        let short = (self.width.min(self.height) as f64 / self.ppp).max(1.0);
        (1.0 + (short - 390.0) / (1032.0 - 390.0) * 0.2).clamp(1.0, 1.2)
    }

    pub fn disc(&self) -> Disc {
        View::layout(self.width, self.height, self.ppp, self.camera)
    }

    pub fn view(&self) -> View {
        View {
            width: self.width,
            height: self.height,
            ppp: self.ppp,
            reading: self.reading(),
            disc: self.disc(),
            camera: self.camera,
            seconds: self.seconds,
            clock: self.clock,
            rest: 1.0,
            night: self.night,
            export: 0.0,
        }
    }

    /// Where a place is on screen (points) and how far it faces the reader (1 centre, 0 rim, <0 behind).
    pub fn project(&self, at: LonLat) -> ([f64; 2], f64) {
        self.disc().project(&self.camera, at)
    }

    /// Where `[lon, lat]` is on screen (points), if it faces the reader: what a toy's `Toy::project`
    /// returns, so a plan can touch a place by its longitude and latitude.
    pub fn point(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        let (p, facing) = self.project(LonLat::new(lonlat[0], lonlat[1]));
        (facing > 0.0).then_some(p)
    }

    /// The first of `places` ([lon, lat]) that lies well inside the screen now (its middle three
    /// fifths), in points; else the screen's centre. For a demo's ghost finger (`info`).
    pub fn spot_for(&self, places: &[[f64; 2]]) -> [f64; 2] {
        let (w, h) = (self.width as f64 / self.ppp, self.height as f64 / self.ppp);
        places
            .iter()
            .filter_map(|&ll| self.point(ll))
            .find(|p| (p[0] - w / 2.0).abs() < 0.3 * w && (p[1] - h / 2.0).abs() < 0.3 * h)
            .unwrap_or([w / 2.0, h / 2.0])
    }

    /// The place under a point of the screen, if it is on the globe.
    pub fn unproject(&self, x: f64, y: f64) -> Option<LonLat> {
        self.disc().unproject(&self.camera, x, y)
    }

    pub fn view_bind(&self) -> &wgpu::BindGroup {
        &self.view_bind
    }

    /// Advances the hand's fling and the date; keeps the land textures on the date's months.
    pub fn update(&mut self, gpu: &Gpu, assets: &dyn Assets, t: f64, dt: f64) {
        self.seconds = t;
        if self.clock.running {
            self.clock.day += self.clock.rate * dt;
        }
        let (ma, mb, _) = self.clock.months();
        if let Ok(true) = self.textures.set_months(gpu, assets, ma, mb) {
            self.ground.rebind(gpu, &self.textures);
        }
        let disc = self.disc();
        self.camera = self.hand.coast(self.camera, disc, dt);
        let view = self.view();
        if let Some(r) = self.rivers.as_mut() {
            r.update(gpu, &view, t);
        }
    }

    /// A finger for the hand (drag turns, pinch zooms, a flick carries on). Says whether it took it.
    pub fn pointer(&mut self, e: PointerEvent) -> bool {
        if !self.hand.on {
            return false;
        }
        let disc = self.disc();
        self.camera = self.hand.pointer(self.camera, disc, e);
        true
    }

    /// Home: the camera flies the short way to the hand's `home` and lands upright (what the hand's
    /// own Home gestures do; a toy may call it from a gesture of its own).
    pub fn home(&mut self) {
        self.hand.go_home();
    }

    /// The camera now, upright, is where Home goes (call it once the opening view is set).
    pub fn set_home(&mut self) {
        self.hand.home = Some(Camera {
            roll: 0.0,
            ..self.camera
        });
    }

    /// Writes the view uniform (call once a frame before drawing anything that reads it).
    pub fn upload(&self, gpu: &Gpu) {
        let view = self.view();
        gpu.queue
            .write_buffer(&self.view_buf, 0, bytemuck::bytes_of(&view.uniform()));
        if let Some(r) = &self.rivers {
            r.mask(gpu, &view, &self.view_bind);
        }
        for s in &self.sheets {
            s.upload(gpu, &view);
        }
    }

    /// The ground, filling the target (it clears nothing: it draws every pixel).
    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>) {
        self.ground.draw(rp, &self.view_bind);
        // (the sheets under the rivers: from afar a sheet is more of the same ground, its rivers the map's)
        for s in &self.sheets {
            s.draw(rp, &self.view_bind);
        }
        if let Some(r) = &self.rivers {
            r.lay(rp, &self.view_bind);
        }
    }
}

/// The globe under the fingers, Adrift's gestures exactly (`adrift-core/src/touch/gestures.rs` and
/// `camera.rs`, lifted Nov 4, 2026, so every globe app turns the same way):
///
/// - one finger drags the orb: the place it took stays under it (a trackball over the poles too, no
///   north to stop at), after a slop of `SLOP` points so a tap never turns it;
/// - let go moving, it flings on and slows (`FLING_DECAY`), at the same glide whatever the frame rate;
/// - two fingers pinch about what they hold (the place under their midpoint stays under it) and, turned
///   about each other, roll the orb about that place (the twist);
/// - Home: pinched out past the whole globe and let go, or three fingers swiped up, the camera flies the
///   short way back to `home` (or the toy calls [`Globe::home`]) and lands upright.
///
/// Zoom is kept within `min_zoom..max_zoom`; below `rest_zoom` it eases back once the fingers lift.
pub struct Hand {
    pub on: bool,
    pub min_zoom: f64,
    pub max_zoom: f64,
    /// Below this zoom the globe eases back up once the fingers lift (a pinch out springs back to
    /// fill its room); set it to `min_zoom` to keep wherever the pinch left it.
    pub rest_zoom: f64,
    /// Where Home flies to (the toy's opening view, upright: [`Globe::set_home`]); None: the place at
    /// the centre now, north up, at `rest_zoom`.
    pub home: Option<Camera>,
    /// Reduce Motion: no fling, and Home a cut.
    pub reduce_motion: bool,
    fingers: HashMap<u64, [f64; 2]>,
    state: HandState,
    /// The drag's last samples (t, x, y), for the fling.
    samples: std::collections::VecDeque<(f64, f64, f64)>,
    /// The fling, points a second.
    spin: (f64, f64),
    flight: Option<Flight>,
    /// The hand's clock (the fingers' and the frames' seconds).
    now: f64,
    /// Home asked for by a gesture, flown on the next frame (the hand has the disc then).
    home_asked: bool,
}

#[derive(Clone, Copy, Debug)]
enum HandState {
    Idle,
    /// A finger down, not yet past the slop.
    Down {
        id: u64,
        x0: f64,
        y0: f64,
    },
    /// One finger dragging: `grab` the place it took (None off the disc).
    Orbit {
        id: u64,
        x: f64,
        y: f64,
        grab: Option<LonLat>,
    },
    /// Two fingers: the zoom when they began, and the pinch's scale since.
    Pinch {
        a: u64,
        b: u64,
        zoom0: f64,
        scale: f64,
    },
    /// Three fingers: waiting for a swipe up (Home).
    Three {
        x0: f64,
        y0: f64,
        done: bool,
    },
}

#[derive(Clone, Copy, Debug)]
struct Flight {
    from: Camera,
    to: Camera,
    start: f64,
    duration: f64,
}

/// Adrift's numbers (`adrift-core/src/touch/motion.rs`), the same in every globe app.
pub mod feel {
    /// How far a fingertip may stray (points) before a touch is a drag.
    pub const SLOP: f64 = 8.0;
    /// A fling's velocity decays by `exp(-FLING_DECAY * dt)` and stops under `FLING_REST` points a second.
    pub const FLING_DECAY: f64 = 3.2;
    pub const FLING_REST: f64 = 2.0;
    /// The samples within this many seconds of the lift make the fling.
    pub const FLING_WINDOW: f64 = 0.08;
    /// A finger that stopped this long before lifting has no fling.
    pub const FLING_STOPPED: f64 = 0.15;
    /// Home's flight, eased in and out.
    pub const FLIGHT_S: f64 = 0.70;
    /// Pinched out so far below the least zoom (a fraction of it) and let go: Home.
    pub const OVERPINCH: f64 = 0.75;
    /// A three-finger swipe up this far (points): Home.
    pub const SWIPE: f64 = 40.0;
}
use feel::*;

impl Default for Hand {
    fn default() -> Hand {
        Hand {
            on: true,
            min_zoom: 0.6,
            max_zoom: 40.0,
            rest_zoom: 1.0,
            home: None,
            reduce_motion: false,
            fingers: HashMap::new(),
            state: HandState::Idle,
            samples: Default::default(),
            spin: (0.0, 0.0),
            flight: None,
            now: 0.0,
            home_asked: false,
        }
    }
}

impl Hand {
    /// How many fingers are down.
    pub fn fingers(&self) -> usize {
        self.fingers.len()
    }

    /// Whether the camera is moving of the hand's own accord (a fling, Home's flight), or a finger
    /// is down: a toy's own camera (an autopilot, an idle turn) waits while it is.
    pub fn busy(&self) -> bool {
        !self.fingers.is_empty()
            || self.flight.is_some()
            || self.home_asked
            || self.spin.0.abs() + self.spin.1.abs() > FLING_REST
    }

    /// Asks for Home (flown on the next frame).
    pub fn go_home(&mut self) {
        self.home_asked = true;
    }

    pub fn pointer(&mut self, cam: Camera, disc: Disc, e: PointerEvent) -> Camera {
        let mut cam = cam;
        self.now = self.now.max(e.t);
        match e.phase {
            Phase::Down => {
                // a finger landed: whatever the camera was doing of its own stops
                self.flight = None;
                self.spin = (0.0, 0.0);
                self.fingers.insert(e.id, [e.x, e.y]);
                let n = self.fingers.len();
                self.state = match (self.state, n) {
                    (_, 1) => HandState::Down {
                        id: e.id,
                        x0: e.x,
                        y0: e.y,
                    },
                    (HandState::Down { id, .. } | HandState::Orbit { id, .. }, 2) => {
                        HandState::Pinch {
                            a: id,
                            b: e.id,
                            zoom0: cam.zoom,
                            scale: 1.0,
                        }
                    }
                    (HandState::Pinch { .. }, 3) => {
                        let (x0, y0) = self.mean();
                        HandState::Three {
                            x0,
                            y0,
                            done: false,
                        }
                    }
                    (s, _) => s,
                };
            }
            Phase::Move => {
                let Some(prev) = self.fingers.get(&e.id).copied() else {
                    return cam;
                };
                self.fingers.insert(e.id, [e.x, e.y]);
                match self.state {
                    HandState::Down { id, x0, y0 } if id == e.id => {
                        if (e.x - x0).hypot(e.y - y0) > SLOP {
                            let grab = disc.unproject(&cam, x0, y0);
                            cam = drag(cam, &disc, grab, e.x, e.y, e.x - x0, e.y - y0);
                            self.samples.clear();
                            self.sample(&e);
                            self.state = HandState::Orbit {
                                id,
                                x: e.x,
                                y: e.y,
                                grab,
                            };
                        }
                    }
                    HandState::Orbit { id, x, y, grab } if id == e.id => {
                        cam = drag(cam, &disc, grab, e.x, e.y, e.x - x, e.y - y);
                        self.sample(&e);
                        self.state = HandState::Orbit {
                            id,
                            x: e.x,
                            y: e.y,
                            grab,
                        };
                    }
                    HandState::Pinch { a, b, zoom0, scale } if e.id == a || e.id == b => {
                        let Some(&other) = self.fingers.get(&if e.id == a { b } else { a }) else {
                            return cam;
                        };
                        let d0 = (prev[0] - other[0]).hypot(prev[1] - other[1]).max(1.0);
                        let d1 = (e.x - other[0]).hypot(e.y - other[1]).max(1.0);
                        let (ax, ay) = ((prev[0] + other[0]) / 2.0, (prev[1] + other[1]) / 2.0);
                        let (bx, by) = ((e.x + other[0]) / 2.0, (e.y + other[1]) / 2.0);
                        let grab = disc.unproject(&cam, ax, ay);
                        let (c, grown) = pinch(
                            cam,
                            &disc,
                            grab,
                            ax,
                            ay,
                            bx,
                            by,
                            d1 / d0,
                            self.min_zoom,
                            self.max_zoom,
                        );
                        cam = c;
                        // and turned about each other: the orb rolls with them, about their midpoint
                        let a0 = (prev[1] - other[1]).atan2(prev[0] - other[0]);
                        let a1 = (e.y - other[1]).atan2(e.x - other[0]);
                        let mut turn = a1 - a0;
                        if turn > std::f64::consts::PI {
                            turn -= std::f64::consts::TAU;
                        } else if turn < -std::f64::consts::PI {
                            turn += std::f64::consts::TAU;
                        }
                        if turn.abs() > 1e-9 {
                            cam = twist(cam, &grown, bx, by, turn);
                        }
                        self.state = HandState::Pinch {
                            a,
                            b,
                            zoom0,
                            scale: scale * d1 / d0,
                        };
                    }
                    HandState::Three {
                        x0,
                        y0,
                        done: false,
                    } => {
                        let (_, my) = self.mean();
                        let (mx, _) = self.mean();
                        let (dx, dy) = (mx - x0, my - y0);
                        if dy <= -SWIPE && dy.abs() >= dx.abs() {
                            self.home_asked = true;
                            self.state = HandState::Three { x0, y0, done: true };
                        }
                    }
                    _ => {}
                }
            }
            Phase::Up | Phase::Cancel => {
                if self.fingers.remove(&e.id).is_none() {
                    return cam;
                }
                match self.state {
                    HandState::Orbit { id, .. } if id == e.id => {
                        if e.phase == Phase::Up && !self.reduce_motion {
                            self.spin = self.velocity(e.t);
                        }
                        self.state = HandState::Idle;
                    }
                    HandState::Down { id, .. } if id == e.id => self.state = HandState::Idle,
                    HandState::Pinch { a, b, zoom0, scale } if e.id == a || e.id == b => {
                        // pinched out past the whole globe and let go: Home
                        if e.phase == Phase::Up && zoom0 * scale < self.min_zoom * OVERPINCH {
                            self.home_asked = true;
                            self.state = HandState::Idle;
                        } else {
                            // the drag goes on with the other finger, anchored where it is now
                            let rest = if e.id == a { b } else { a };
                            self.samples.clear();
                            self.state = match self.fingers.get(&rest) {
                                Some(&f) => HandState::Orbit {
                                    id: rest,
                                    x: f[0],
                                    y: f[1],
                                    grab: disc.unproject(&cam, f[0], f[1]),
                                },
                                None => HandState::Idle,
                            };
                        }
                    }
                    HandState::Three { .. } if self.fingers.is_empty() => {
                        self.state = HandState::Idle
                    }
                    _ => {}
                }
                if self.fingers.is_empty() && !matches!(self.state, HandState::Idle) {
                    self.state = HandState::Idle;
                }
            }
        }
        cam
    }

    fn mean(&self) -> (f64, f64) {
        let n = self.fingers.len().max(1) as f64;
        (
            self.fingers.values().map(|p| p[0]).sum::<f64>() / n,
            self.fingers.values().map(|p| p[1]).sum::<f64>() / n,
        )
    }

    fn sample(&mut self, e: &PointerEvent) {
        self.samples.push_back((e.t, e.x, e.y));
        while self.samples.len() > 8 {
            self.samples.pop_front();
        }
    }

    /// The fling's velocity, points a second, from the drag's last samples (Adrift's `velocity`).
    fn velocity(&self, now: f64) -> (f64, f64) {
        let recent: Vec<_> = self
            .samples
            .iter()
            .filter(|s| now - s.0 <= FLING_WINDOW)
            .collect();
        let (first, last) = if recent.len() >= 2 {
            (*recent[0], *recent[recent.len() - 1])
        } else {
            let n = self.samples.len();
            if n < 2 || now - self.samples[n - 1].0 > FLING_STOPPED {
                return (0.0, 0.0);
            }
            (self.samples[n - 2], self.samples[n - 1])
        };
        let dt = last.0 - first.0;
        if dt < 0.005 {
            return (0.0, 0.0);
        }
        ((last.1 - first.1) / dt, (last.2 - first.2) / dt)
    }

    /// The camera's own motion this frame: Home's flight, the fling, and a pinch out easing back.
    pub fn coast(&mut self, cam: Camera, disc: Disc, dt: f64) -> Camera {
        let dt = dt.clamp(0.0, 0.05);
        self.now += dt;
        let mut cam = cam;
        if std::mem::take(&mut self.home_asked) {
            let to = self
                .home
                .map(|h| Camera { roll: 0.0, ..h })
                .unwrap_or(Camera {
                    roll: 0.0,
                    zoom: self.rest_zoom,
                    ..Camera::new(cam.lon, cam.lat, self.rest_zoom)
                });
            self.spin = (0.0, 0.0);
            if self.reduce_motion {
                return to;
            }
            self.flight = Some(Flight {
                from: cam,
                to,
                start: self.now,
                duration: FLIGHT_S,
            });
        }
        if let Some(fl) = self.flight {
            let u = ((self.now - fl.start) / fl.duration).clamp(0.0, 1.0);
            let e = ease_in_out(u);
            // the orientation slerped (the shortest turn, a pole included, the roll with it, so Home
            // comes back upright), the zoom by its log
            let zoom = (fl.from.zoom.ln() + (fl.to.zoom.ln() - fl.from.zoom.ln()) * e).exp();
            if u >= 1.0 {
                self.flight = None;
                return Camera { zoom, ..fl.to };
            }
            return Camera::from_rotation(
                quat_mat(slerp(
                    mat_quat(fl.from.rotation()),
                    mat_quat(fl.to.rotation()),
                )(e)),
                zoom,
            );
        }
        if !self.fingers.is_empty() {
            return cam;
        }
        if cam.zoom < self.rest_zoom {
            // a pinch out springs back: halfway every 0.25 s
            let k = 0.5f64.powf(dt / 0.25);
            cam.zoom = self.rest_zoom - (self.rest_zoom - cam.zoom) * k;
            if self.rest_zoom - cam.zoom < 1e-3 {
                cam.zoom = self.rest_zoom;
            }
        }
        if self.spin.0.abs() + self.spin.1.abs() > FLING_REST {
            // the exact distance a decaying fling covers in dt (the same glide at 20 frames a second
            // as at 120)
            let along = (1.0 - (-FLING_DECAY * dt).exp()) / FLING_DECAY;
            cam = turn(cam, &disc, self.spin.0 * along, self.spin.1 * along);
            let k = (-FLING_DECAY * dt).exp();
            self.spin = (self.spin.0 * k, self.spin.1 * k);
        } else {
            self.spin = (0.0, 0.0);
        }
        cam
    }
}

// --- the sphere (Adrift's camera.rs) ---

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Terra's ease: cubic in, cubic out.
fn ease_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// The globe turned as a finger moving (dx, dy) points would drag the point at the screen's centre:
/// a trackball about the view's axis (dy, dx, 0) by hypot(dx, dy) / r.
fn turn(cam: Camera, disc: &Disc, dx: f64, dy: f64) -> Camera {
    if disc.r <= 0.0 || (dx == 0.0 && dy == 0.0) {
        return cam;
    }
    cam.turned([dy, dx, 0.0], dx.hypot(dy) / disc.r)
}

/// A drag: `grab`, the place the finger took, exactly under the finger at (x, y), by the one rotation
/// carrying its view direction to the finger's; off the disc, or the place gone round the back, the
/// finger's own move turns the globe by `turn`.
fn drag(
    cam: Camera,
    disc: &Disc,
    grab: Option<LonLat>,
    x: f64,
    y: f64,
    dx: f64,
    dy: f64,
) -> Camera {
    let Some(grab) = grab else {
        return turn(cam, disc, dx, dy);
    };
    let vx = (x - disc.cx) / disc.r;
    let vy = (disc.cy - y) / disc.r;
    let r2 = vx * vx + vy * vy;
    let a = cam.view(grab.to_xyz());
    if r2 >= 1.0 || a[2] <= 0.0 {
        return turn(cam, disc, dx, dy);
    }
    let b = [vx, vy, (1.0 - r2).sqrt()];
    let axis = cross(a, b);
    if dot(axis, axis) < 1e-24 {
        return cam;
    }
    cam.turned(axis, dot(a, b).clamp(-1.0, 1.0).acos())
}

/// Two fingers: zoom by `factor`, and the place under (ax, ay) moved to (bx, by). Returns the camera
/// and the disc as it is drawn now (grown with the zoom).
#[allow(clippy::too_many_arguments)]
fn pinch(
    cam: Camera,
    disc: &Disc,
    grab: Option<LonLat>,
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
    factor: f64,
    min: f64,
    max: f64,
) -> (Camera, Disc) {
    let mut cam = cam;
    let z0 = cam.zoom;
    cam.zoom = (z0 * factor).clamp(min, max);
    let grown = Disc {
        r: disc.r * cam.zoom / z0,
        ..*disc
    };
    (drag(cam, &grown, grab, bx, by, bx - ax, by - ay), grown)
}

/// The twist: the orb rolled about the place under (x, y) by the fingers' turn on the screen
/// (y down: positive is clockwise), so what they hold turns with them.
fn twist(cam: Camera, disc: &Disc, x: f64, y: f64, angle: f64) -> Camera {
    if angle == 0.0 || disc.r <= 0.0 {
        return cam;
    }
    let vx = (x - disc.cx) / disc.r;
    let vy = (disc.cy - y) / disc.r;
    let r2 = vx * vx + vy * vy;
    let axis = if r2 < 1.0 {
        [vx, vy, (1.0 - r2).sqrt()]
    } else {
        [0.0, 0.0, 1.0]
    };
    // view z toward the reader: a positive turn about it is counterclockwise on the screen
    cam.turned(axis, -angle)
}

/// A rotation (columns) as a unit quaternion (w, x, y, z).
fn mat_quat(c: [[f64; 3]; 3]) -> [f64; 4] {
    let m = |i: usize, j: usize| c[j][i];
    let tr = m(0, 0) + m(1, 1) + m(2, 2);
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [
            0.25 * s,
            (m(2, 1) - m(1, 2)) / s,
            (m(0, 2) - m(2, 0)) / s,
            (m(1, 0) - m(0, 1)) / s,
        ]
    } else if m(0, 0) > m(1, 1) && m(0, 0) > m(2, 2) {
        let s = (1.0 + m(0, 0) - m(1, 1) - m(2, 2)).sqrt() * 2.0;
        [
            (m(2, 1) - m(1, 2)) / s,
            0.25 * s,
            (m(0, 1) + m(1, 0)) / s,
            (m(0, 2) + m(2, 0)) / s,
        ]
    } else if m(1, 1) > m(2, 2) {
        let s = (1.0 + m(1, 1) - m(0, 0) - m(2, 2)).sqrt() * 2.0;
        [
            (m(0, 2) - m(2, 0)) / s,
            (m(0, 1) + m(1, 0)) / s,
            0.25 * s,
            (m(1, 2) + m(2, 1)) / s,
        ]
    } else {
        let s = (1.0 + m(2, 2) - m(0, 0) - m(1, 1)).sqrt() * 2.0;
        [
            (m(1, 0) - m(0, 1)) / s,
            (m(0, 2) + m(2, 0)) / s,
            (m(1, 2) + m(2, 1)) / s,
            0.25 * s,
        ]
    };
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// A unit quaternion as a rotation (columns).
fn quat_mat(q: [f64; 4]) -> [[f64; 3]; 3] {
    let [w, x, y, z] = q;
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - w * z),
            2.0 * (x * z + w * y),
        ],
        [
            2.0 * (x * y + w * z),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - w * x),
        ],
        [
            2.0 * (x * z - w * y),
            2.0 * (y * z + w * x),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    [
        [r[0][0], r[1][0], r[2][0]],
        [r[0][1], r[1][1], r[2][1]],
        [r[0][2], r[1][2], r[2][2]],
    ]
}

/// The shortest turn from `a` to `b`, a share `t` of the way.
fn slerp(a: [f64; 4], b: [f64; 4]) -> impl Fn(f64) -> [f64; 4] {
    let mut b = b;
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if d < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        d = -d;
    }
    move |t| {
        let (wa, wb) = if d > 0.9995 {
            (1.0 - t, t)
        } else {
            let th = d.clamp(-1.0, 1.0).acos();
            (((1.0 - t) * th).sin() / th.sin(), (t * th).sin() / th.sin())
        };
        let q = [
            wa * a[0] + wb * b[0],
            wa * a[1] + wb * b[1],
            wa * a[2] + wb * b[2],
            wa * a[3] + wb * b[3],
        ];
        let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_keeps_the_place_under_the_finger() {
        let disc = Disc {
            cx: 200.0,
            cy: 400.0,
            r: 180.0,
        };
        let mut hand = Hand::default();
        let mut cam = Camera::new(10.0, 20.0, 1.0);
        let (x0, y0) = (230.0, 380.0);
        let under = disc.unproject(&cam, x0, y0).unwrap();
        cam = hand.pointer(
            cam,
            disc,
            PointerEvent {
                id: 1,
                phase: Phase::Down,
                x: x0,
                y: y0,
                t: 0.0,
            },
        );
        let (mut x, mut y) = (x0, y0);
        for i in 1..=20 {
            x += 1.5;
            y += 1.0;
            cam = hand.pointer(
                cam,
                disc,
                PointerEvent {
                    id: 1,
                    phase: Phase::Move,
                    x,
                    y,
                    t: i as f64 / 60.0,
                },
            );
        }
        let ([px, py], _) = disc.project(&cam, under);
        assert!(
            (px - x).abs() < 1.5 && (py - y).abs() < 1.5,
            "the place is at {px},{py}, the finger at {x},{y}"
        );
    }

    fn ev(id: u64, phase: Phase, x: f64, y: f64, t: f64) -> PointerEvent {
        PointerEvent { id, phase, x, y, t }
    }

    #[test]
    fn a_drag_carries_the_orb_over_the_pole_and_home_lands_upright() {
        let disc = Disc {
            cx: 200.0,
            cy: 400.0,
            r: 180.0,
        };
        let mut hand = Hand::default();
        let mut cam = Camera::new(10.0, 60.0, 1.0);
        hand.home = Some(Camera::new(-30.0, 25.0, 1.0));
        let (x, mut y) = (200.0, 260.0);
        let grab = disc.unproject(&cam, x, y).unwrap();
        cam = hand.pointer(cam, disc, ev(1, Phase::Down, x, y, 0.0));
        let mut north = 0.0f64;
        for i in 1..=60 {
            y += 4.0;
            cam = hand.pointer(cam, disc, ev(1, Phase::Move, x, y, i as f64 / 60.0));
            north = north.max(cam.lat);
        }
        let ([px, py], _) = disc.project(&cam, grab);
        assert!(
            (px - x).abs() < 1e-6 && (py - y).abs() < 1e-6,
            "{px},{py} vs {x},{y}"
        );
        assert!(north > 89.0 && cam.roll.abs() > 90.0, "{north} {cam:?}");
        cam = hand.pointer(cam, disc, ev(1, Phase::Up, x, y, 1.5));
        // three fingers swiped up: Home, the short way, upright
        for (i, fx) in [150.0, 200.0, 250.0].into_iter().enumerate() {
            cam = hand.pointer(cam, disc, ev(10 + i as u64, Phase::Down, fx, 500.0, 2.0));
        }
        for (i, fx) in [150.0, 200.0, 250.0].into_iter().enumerate() {
            cam = hand.pointer(cam, disc, ev(10 + i as u64, Phase::Move, fx, 440.0, 2.1));
        }
        for i in 0..3 {
            cam = hand.pointer(cam, disc, ev(10 + i, Phase::Up, 0.0, 440.0, 2.2));
        }
        for _ in 0..60 {
            cam = hand.coast(cam, disc, 1.0 / 60.0);
        }
        assert_eq!((cam.lon, cam.lat, cam.roll), (-30.0, 25.0, 0.0));
    }

    #[test]
    fn a_twist_rolls_the_orb_and_a_pinch_holds_what_the_fingers_hold() {
        let disc = Disc {
            cx: 200.0,
            cy: 400.0,
            r: 180.0,
        };
        let mut hand = Hand::default();
        let mut cam = Camera::new(-30.0, 25.0, 1.0);
        let (mx, my) = (230.0, 380.0);
        let mid = disc.unproject(&cam, mx, my).unwrap();
        let right = disc.unproject(&cam, mx + 60.0, my).unwrap();
        cam = hand.pointer(cam, disc, ev(1, Phase::Down, mx - 60.0, my, 0.0));
        cam = hand.pointer(cam, disc, ev(2, Phase::Down, mx + 60.0, my, 0.0));
        // the right finger goes a quarter turn clockwise round the left... both turn about the midpoint
        let q = std::f64::consts::FRAC_PI_2;
        for i in 1..=30 {
            let a = q * i as f64 / 30.0;
            let (c, s) = (a.cos() * 60.0, a.sin() * 60.0);
            cam = hand.pointer(
                cam,
                disc,
                ev(1, Phase::Move, mx - c, my - s, i as f64 / 60.0),
            );
            cam = hand.pointer(
                cam,
                disc,
                ev(2, Phase::Move, mx + c, my + s, i as f64 / 60.0),
            );
        }
        let ([px, py], _) = disc.project(&cam, mid);
        assert!((px - mx).abs() < 1.0 && (py - my).abs() < 1.0, "{px},{py}");
        let ([px, py], _) = disc.project(&cam, right);
        assert!((px - mx).abs() < 8.0 && py > my + 50.0, "{px},{py}");
        assert!(cam.roll.abs() > 60.0, "{cam:?}");
        // and spread apart: zoomed, the midpoint's place still under it
        let z0 = cam.zoom;
        for i in 1..=20 {
            let d = 60.0 + 3.0 * i as f64;
            let t = 0.5 + i as f64 / 60.0;
            // (the globe hands the hand the disc as drawn now, grown with the zoom)
            cam = hand.pointer(
                cam,
                Disc {
                    r: disc.r * cam.zoom,
                    ..disc
                },
                ev(1, Phase::Move, mx, my - d, t),
            );
            cam = hand.pointer(
                cam,
                Disc {
                    r: disc.r * cam.zoom,
                    ..disc
                },
                ev(2, Phase::Move, mx, my + d, t),
            );
        }
        let grown = Disc {
            r: disc.r * cam.zoom,
            ..disc
        };
        let ([px, py], _) = grown.project(&cam, mid);
        assert!(
            cam.zoom > z0 * 1.9 && (px - mx).abs() < 1.0 && (py - my).abs() < 1.0,
            "{} {px},{py}",
            cam.zoom
        );
    }

    #[test]
    fn a_fling_glides_on_and_a_tap_never_turns_it() {
        let disc = Disc {
            cx: 200.0,
            cy: 400.0,
            r: 180.0,
        };
        let mut hand = Hand::default();
        let cam0 = Camera::new(0.0, 0.0, 1.0);
        let mut cam = hand.pointer(cam0, disc, ev(1, Phase::Down, 200.0, 400.0, 0.0));
        cam = hand.pointer(cam, disc, ev(1, Phase::Move, 203.0, 400.0, 0.05));
        cam = hand.pointer(cam, disc, ev(1, Phase::Up, 203.0, 400.0, 0.1));
        assert_eq!(cam, cam0, "a tap within the slop turns nothing");
        for i in 1..=10 {
            cam = hand.pointer(
                cam,
                disc,
                ev(
                    2,
                    if i == 1 { Phase::Down } else { Phase::Move },
                    100.0 + 12.0 * i as f64,
                    400.0,
                    1.0 + i as f64 / 60.0,
                ),
            );
        }
        cam = hand.pointer(cam, disc, ev(2, Phase::Up, 220.0, 400.0, 1.0 + 10.5 / 60.0));
        let lon = cam.lon;
        assert!(hand.busy());
        for _ in 0..240 {
            cam = hand.coast(cam, disc, 1.0 / 60.0);
        }
        assert!(cam.lon < lon - 20.0 && !hand.busy(), "{lon} -> {}", cam.lon);
    }

    #[test]
    fn a_pinch_out_springs_back() {
        let disc = Disc {
            cx: 200.0,
            cy: 400.0,
            r: 180.0,
        };
        let mut hand = Hand::default();
        let mut cam = Camera::new(0.0, 0.0, 0.6);
        for _ in 0..120 {
            cam = hand.coast(cam, disc, 1.0 / 60.0);
        }
        assert!((cam.zoom - 1.0).abs() < 0.01, "zoom {}", cam.zoom);
    }
}
