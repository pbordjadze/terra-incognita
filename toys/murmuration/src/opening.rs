//! The opening's warming (`Toy::warm`): the flock run ahead six seconds, so it opens already wheeling, and the
//! dusk sky's table worked out, a slice at a time (a browser's WebAssembly has one thread: done at once, they
//! froze the page for seconds). Both are the same every time the toy opens on the same dusk, so they are kept
//! for the next opening, which is ready at once.

use crate::flock::{norm, Flock};
use crate::{sky, Murmuration};
use toy_kit::Ctx;

/// Steps the flock runs ahead before it is shown (six seconds, a sixtieth each).
pub const RUN_AHEAD: usize = 360;
/// Rows of the sky's table a slice works out.
const SKY_ROWS: usize = 4;

/// The flock run ahead and the sky worked out at the last opening, and the sun's elevation it is for.
static OPENING: std::sync::Mutex<Option<(Flock, u64, sky::Sky)>> = std::sync::Mutex::new(None);

impl Murmuration {
    /// Takes up the last opening's flock (and its sky, on the same dusk), if the toy has opened before.
    pub(crate) fn as_last_opened(&mut self) {
        let Some((f, at, s)) = OPENING.lock().ok().and_then(|o| o.clone()) else {
            return;
        };
        self.flock = f;
        self.run_ahead = 0;
        self.flock_run_ahead();
        if at == self.sun_el.to_bits() {
            self.sky = s;
        }
    }

    /// One slice of the warming; true once the toy is ready to show.
    pub(crate) fn warm_part(&mut self, ctx: &mut Ctx) -> bool {
        if self.run_ahead > 0 {
            self.flock.step(1.0 / 60.0);
            self.run_ahead -= 1;
            if self.run_ahead == 0 {
                self.flock_run_ahead();
            }
            return false;
        }
        if self.sky.ready_for.is_none() {
            if !self
                .sky
                .compute_part(self.sun_el, &mut self.sky_work, SKY_ROWS)
            {
                return false;
            }
            if let Ok(mut o) = OPENING.lock() {
                *o = Some((self.flock.clone(), self.sun_el.to_bits(), self.sky.clone()));
            }
        }
        self.upload_sky(ctx);
        self.place_eye(ctx, 1.0);
        true
    }

    /// The flock has run ahead: the eye starts on it (heading with your bird, looking at the flock).
    fn flock_run_ahead(&mut self) {
        let v = self.flock.birds[self.flock.you].vel;
        self.heading = norm([v[0], 0.0, v[2]]);
        self.dir3 = norm(v);
        self.look = self.flock.centre().0;
        self.flock_look = self.look;
    }
}
