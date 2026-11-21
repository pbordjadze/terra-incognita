//! For tools/toys/murmuration/holds.py: how many birds are on the glass, and how many of them
//! against the sky (MURMURATION_COUNT), and where your bird is (MURMURATION_YOU).

use crate::flock::{dot, length, sub};
use crate::{local, Murmuration};
use toy_kit::Ctx;

impl Murmuration {
    pub(crate) fn count(&self, ctx: &Ctx) {
        let e = self.eye;
        let [w, h] = [ctx.width as f32, ctx.height as f32];
        let (mut n, mut sky, mut big) = (0, 0, 0);
        for b in &self.flock.birds {
            let rel = sub(b.pos, e.pos);
            let z = dot(rel, e.fwd);
            let me = std::ptr::eq(b, &self.flock.birds[self.flock.you]);
            // As the shader culls: other birds nearer than 5 m, or wider than an eighth of
            // the glass, are not drawn, so not counted.
            let span_px = 0.38 * b.size * e.focal / z.max(0.05);
            if z < 0.1 || (!me && (z < 5.0 || span_px > 0.12 * w)) {
                continue;
            }
            let x = dot(rel, e.right) * e.focal / z;
            let y = dot(rel, e.up) * e.focal / z;
            if x.abs() < 0.5 * w && y.abs() < 0.5 * h {
                n += 1;
                // Big enough to read as a bird near you: a wingspan of 12 points or more.
                if 0.5 * e.focal / z >= 12.0 * ctx.ppp as f32 {
                    big += 1;
                }
                if b.pos[1] > e.pos[1] || z > 2000.0 {
                    sky += 1;
                }
            }
        }
        println!(
            "visible {:.2} {} {} near {:.2} big {}",
            self.t, n, sky, self.near, big
        );
        if std::env::var_os("MURMURATION_YOU").is_some() {
            let b = &self.flock.birds[self.flock.you];
            let rel = sub(b.pos, e.pos);
            let z = dot(rel, e.fwd);
            println!("you {} z {:.2} x {:.0} y {:.0} dist {:.2} alt {:.1} vel {:.1} {:.1} {:.1} eye {:.1}", self.flock.you, z, dot(rel, e.right) * e.focal / z, dot(rel, e.up) * e.focal / z, length(rel), b.pos[1], b.vel[0], b.vel[1], b.vel[2], e.pos[1]);
            let (_, _, n15) = local(&self.flock, 15.0);
            let c = self.flock.centre().0;
            let to_c = sub(c, e.pos);
            println!(
                "round {} centre_dist {:.1} centre_in_view_z {:.1} cx {:.0} cy {:.0}",
                n15,
                length(sub(c, b.pos)),
                dot(to_c, e.fwd),
                dot(to_c, e.right) * e.focal / dot(to_c, e.fwd),
                dot(to_c, e.up) * e.focal / dot(to_c, e.fwd)
            );
        }
    }
}
