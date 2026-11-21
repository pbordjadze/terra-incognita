//! Where Nazaré's camera stands: on the fort, or down in the channel beside the peak, riding the swell;
//! and the tow surfer it shows there.

use crate::max::{Cam, Look};
use crate::{
    sea, Nazare, DROP_LEAD, DROP_S, FORT_AIM_M, FORT_BACK_M, GROUP, LIGHT_TALL_M, RATE_FORT,
    RATE_WALL, STAY_S, WALL_AHEAD_M, WALL_SIDE_M,
};

impl Nazare {
    /// The two places the camera goes: on the cliff by the lighthouse, looking over Praia do Norte at the
    /// peak; and low in the channel in front of the peak, looking up at its face.
    pub(crate) fn looks(&self) -> (Look, Look) {
        let l = self.light_m();
        let peak = self.peak.map_or_else(
            || {
                let c = self.bounds.places.praia_do_norte.cell;
                let m = self.cell_m() as f32;
                [c[0] * m - 500.0, c[1] * m]
            },
            |i| self.at(i),
        );
        // the fort at the headland's tip, by the lighthouse, a little back from the edge
        let to = [l[0] - peak[0], l[1] - peak[1]];
        let tl = to[0].hypot(to[1]).max(1.0);
        let dir = [to[0] / tl, to[1] / tl];
        let cliff = [l[0] + FORT_BACK_M[0], l[1] + FORT_BACK_M[1]];
        let ground = self.ground_m(cliff).max(40.0);
        // aimed over the shelf where the peaks stand, between the headland and Praia do Norte, the same
        // whatever the swell, so the fort's view stays put
        let aim = [l[0] + FORT_AIM_M[0], l[1] + FORT_AIM_M[1], 0.0];
        let fort = Look::at([cliff[0], cliff[1], ground + 20.0], aim, 60.0, 0.1);
        // in the channel beside the peak, a little ahead of it, where the tow teams wait: the wave seen
        // from its shoulder, its face and lip in profile
        let run = self.running(peak);
        let across = [-run[1], run[0]];
        let side = if across[0] * dir[0] + across[1] * dir[1] > 0.0 {
            1.0
        } else {
            -1.0
        };
        let ahead = WALL_AHEAD_M + self.ahead_m();
        let eye = [
            peak[0] + run[0] * ahead + across[0] * side * WALL_SIDE_M,
            peak[1] + run[1] * ahead + across[1] * side * WALL_SIDE_M,
            3.0,
        ];
        let wall = Look::at(
            eye,
            [peak[0] + run[0] * 10.0, peak[1] + run[1] * 10.0, 9.0],
            46.0,
            -0.1,
        );
        (fort, wall)
    }

    /// The ground's height at a point (m), bilinear over the floor.
    pub(crate) fn ground_m(&self, p: [f32; 2]) -> f32 {
        let c = self.cell_m() as f32;
        let (w, h) = (self.floor.w, self.floor.h);
        let x = (p[0] / c - 0.5).clamp(0.0, w as f32 - 1.001);
        let y = (p[1] / c - 0.5).clamp(0.0, h as f32 - 1.001);
        let (x0, y0) = (x as usize, y as usize);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let g = |xx: usize, yy: usize| self.floor.ground[yy * w + xx];
        (g(x0, y0) * (1.0 - fx) + g(x0 + 1, y0) * fx) * (1.0 - fy)
            + (g(x0, y0 + 1) * (1.0 - fx) + g(x0 + 1, y0 + 1) * fx) * fy
    }

    pub(crate) fn look(&self) -> Look {
        let (fort, wall) = self.looks();
        let k = self.down as f32;
        // ease in and out
        let mut look = fort.toward(wall, k * k * (3.0 - 2.0 * k));
        // low in the channel the camera rides the swell, as a ski waiting there does
        let ride = self.surface_m([look.eye[0], look.eye[1]]) + 3.0;
        look.eye[2] = look.eye[2].max(ride);
        look
    }

    /// How far the big wave has run past the peak (m), 0 before it gets there.
    pub(crate) fn ahead_m(&self) -> f32 {
        let Some(i) = self.peak else {
            return 0.0;
        };
        let past = (-self.to_big_wave(i)).clamp(0.0, 0.6 * self.field.period);
        let (c, _) = sea::speeds(self.field.period, self.floor.depth(i).max(1.0));
        (past * c) as f32
    }

    /// The way the arriving waves run at a point (a unit vector, x east, y south): up the crests' times.
    pub(crate) fn running(&self, p: [f32; 2]) -> [f32; 2] {
        let c = self.cell_m() as f32;
        let (w, h) = (self.floor.w, self.floor.h);
        let (x, y) = ((p[0] / c) as usize, (p[1] / c) as usize);
        let tau = |xx: usize, yy: usize| self.field.tau[yy.min(h - 1) * w + xx.min(w - 1)];
        let (x0, y0) = (x.saturating_sub(2), y.saturating_sub(2));
        let gx = tau(x + 2, y) - tau(x0, y);
        let gy = tau(x, y + 2) - tau(x, y0);
        let m = gx.hypot(gy);
        if !m.is_finite() || m < 1e-9 {
            let a = (self.field.from_deg + 180.0).to_radians();
            return [a.sin() as f32, -a.cos() as f32];
        }
        [(gx / m) as f32, (gy / m) as f32]
    }

    /// The sea's surface at a point now (m above the still sea), roughly as the trace draws it: both trains,
    /// in this set, their crests a little sharper than a cosine.
    pub(crate) fn surface_m(&self, p: [f32; 2]) -> f32 {
        let c = self.cell_m() as f32;
        let (x, y) = ((p[0] / c) as usize, (p[1] / c) as usize);
        if x >= self.floor.w || y >= self.floor.h {
            return 0.0;
        }
        let i = y * self.floor.w + x;
        let f = &self.field;
        if !self.floor.sea(i) {
            return 0.0;
        }
        let tg = f.period * GROUP;
        let g = std::f64::consts::TAU * (f.tau_group[i] / tg - self.t / tg - self.sets_off);
        let sets = 0.3 + 0.7 * (0.5 + 0.5 * g.cos()).powi(6);
        let train = |tau: f64, hh: f32| {
            if !tau.is_finite() {
                return 0.0;
            }
            let e = (std::f64::consts::TAU * (self.t - tau) / f.period).cos();
            0.5 * hh as f64 * sets * (e + 0.3 * (2.0 * e * e - 1.0))
        };
        (train(f.tau[i], f.height[i]) + train(f.tau_back[i], f.height_back[i])) as f32
    }

    /// The camera: down for a set's big wave if the swell stacks, up again once it has passed.
    pub(crate) fn steer(&mut self, dt: f64) {
        let rate = |down: f64| RATE_FORT + (RATE_WALL - RATE_FORT) * down;
        let mut want = 0.0;
        if let Some(i) = self.peak {
            let to = self.to_big_wave(i) / rate(self.down);
            if to < DROP_LEAD && to > -0.5 {
                self.stay_until = self.clock + to.max(0.0) + STAY_S;
            }
            if self.clock < self.stay_until {
                want = 1.0;
            }
        }
        let step = dt / DROP_S;
        self.down = if want > self.down {
            (self.down + step).min(want)
        } else {
            (self.down - step).max(want)
        };
    }

    /// The tow surfer on the big wave's face, for its scale: a dark figure on a pale board, 1.8 m tall.
    pub(crate) fn draw_surfer(&mut self, cam: &Cam, ppp: f32) {
        let Some(i) = self.peak else {
            return;
        };
        if self.down < 0.6 {
            return;
        }
        let f = &self.field;
        // on the face only while the big wave stands: from half a period before its crest to just after
        let to = self.to_big_wave(i);
        if to > 0.45 * f.period || to < -0.15 * f.period {
            return;
        }
        let tall = (f.crossed(i) as f32).min(28.0);
        let p = self.at(i);
        // sliding along the crest (the turned-back train's way, toward the beach)
        let run = self.running(p);
        let along = [-run[1], run[0]];
        let s = (0.45 * f.period - to) as f32 * 5.0 - 30.0;
        // on the face with the crest as it comes (behind the peak before it gets there), a third of the way
        // down from the lip
        let (c, _) = sea::speeds(f.period, self.floor.depth(i).max(1.0));
        let ahead = (-to * c) as f32 + 9.0;
        let feet = [
            p[0] + along[0] * s + run[0] * ahead,
            p[1] + along[1] * s + run[1] * ahead,
            tall * 0.12,
        ];
        let Some((at, dist)) = cam.project(feet) else {
            return;
        };
        let k = cam.px_per_m(dist);
        let ink = [0.012, 0.012, 0.03, self.down as f32];
        let board = [0.85, 0.82, 0.78, self.down as f32];
        let soft = 0.6 * ppp;
        self.painter.line(
            [at[0] - 1.0 * k, at[1] + 0.05 * k],
            [at[0] + 1.0 * k, at[1] - 0.12 * k],
            (0.12 * k).max(0.6 * ppp),
            soft,
            board,
        );
        self.painter.line(
            [at[0], at[1] - 0.2 * k],
            [at[0] + 0.08 * k, at[1] - 1.3 * k],
            (0.24 * k).max(0.8 * ppp),
            soft,
            ink,
        );
        self.painter.disc(
            [at[0] + 0.1 * k, at[1] - 1.6 * k],
            (0.2 * k).max(0.8 * ppp),
            soft,
            ink,
        );
    }

    /// The lighthouse on the fort at the headland's tip: a red tower, its lantern lit, so the fort's view
    /// says where it is.
    pub(crate) fn draw_lighthouse(&mut self, cam: &Cam, ppp: f32) {
        if self.down > 0.3 {
            return;
        }
        // just back from the tip, where the 50 m floor still holds the headland
        let l = self.light_m();
        let l = [l[0] + 60.0, l[1]];
        let base = self.ground_m(l).max(0.0);
        let (Some((foot, dist)), Some((top, _))) = (
            cam.project([l[0], l[1], base]),
            cam.project([l[0], l[1], base + LIGHT_TALL_M]),
        ) else {
            return;
        };
        let k = cam.px_per_m(dist);
        let red = [0.3, 0.04, 0.03, 1.0];
        // the tower (the fort under it is the headland's own tip)
        self.painter
            .line(foot, top, (1.6 * k).max(0.8 * ppp), 0.5 * ppp, red);
        // its lantern a lit point, not a halo
        let lamp = [1.0, 0.9, 0.7, 1.0];
        self.painter
            .disc(top, (0.6 * k).max(0.7 * ppp), 0.3 * ppp, lamp);
    }
}
