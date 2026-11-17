//! A shake, told from the device's motion (painting-company row: shaking never reached a toy). The shells
//! feed the phone's own acceleration, gravity taken out, in g (iOS: Core Motion's `userAcceleration`; the
//! web: `devicemotion`'s `acceleration`); a shake is a few hard jolts close together. The system's own
//! shake (iOS's `motionEnded`, which only comes while Shake to Undo is on and the view holds the first
//! responder, and the Simulator's Device › Shake) still counts; [`Shake::cooled`] keeps the two from
//! landing twice for one shake.

/// A jolt starts above this (g): a deliberate shake peaks at 2–3 g; walking, a bus's bump, a phone set
/// down stay under it.
const HIGH: f64 = 1.4;
/// ...and must fall under this before the next counts, so one long push is one jolt.
const LOW: f64 = 0.6;
/// This many jolts...
const JOLTS: usize = 3;
/// ...within this long (seconds) are a shake: a back and forth and a back.
const WITHIN: f64 = 0.9;
/// Jolts closer than this (seconds) are one: a phone set down hard rings for a tenth of a second, while a
/// hand's strokes are 80–250 ms apart.
const APART: f64 = 0.08;
/// After a shake, nothing for this long (seconds), from either path.
const REST: f64 = 1.2;

#[derive(Debug, Default)]
pub struct Shake {
    /// When recent jolts started.
    jolts: Vec<f64>,
    /// Above `HIGH` and not yet back under `LOW`.
    high: bool,
    /// When the last shake landed.
    last: Option<f64>,
}

impl Shake {
    /// One sample of the device's own acceleration `a` (g, gravity out) at `t` seconds; true when it
    /// completes a shake.
    pub fn sample(&mut self, a: [f64; 3], t: f64) -> bool {
        let m = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
        if !m.is_finite() {
            return false;
        }
        if self.high {
            self.high = m > LOW;
            return false;
        }
        if m < HIGH {
            return false;
        }
        self.high = true;
        if self.jolts.last().is_some_and(|&j| (t - j).abs() < APART) {
            return false;
        }
        self.jolts.retain(|&j| t - j <= WITHIN && t >= j);
        self.jolts.push(t);
        if self.jolts.len() < JOLTS {
            return false;
        }
        self.jolts.clear();
        self.cooled(t)
    }

    /// Whether a shake at `t` (seconds) lands: not within `REST` of the last one.
    pub fn cooled(&mut self, t: f64) -> bool {
        if self.last.is_some_and(|l| (t - l).abs() < REST) {
            return false;
        }
        self.last = Some(t);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A shake's samples at 60 Hz: `hz` strokes a second of `peak` g along x, for `secs`.
    fn shake(peak: f64, hz: f64, secs: f64, from: f64) -> Vec<(f64, [f64; 3])> {
        (0..(secs * 60.0) as usize)
            .map(|i| {
                let t = from + i as f64 / 60.0;
                let x = peak * (std::f64::consts::TAU * hz * t).sin();
                (t, [x, 0.1, -0.05])
            })
            .collect()
    }

    fn count(s: &mut Shake, samples: &[(f64, [f64; 3])]) -> usize {
        samples.iter().filter(|(t, a)| s.sample(*a, *t)).count()
    }

    #[test]
    fn a_shake_is_one() {
        let mut s = Shake::default();
        assert_eq!(count(&mut s, &shake(2.5, 3.0, 0.8, 0.0)), 1);
    }

    #[test]
    fn a_long_shake_lands_once_a_rest() {
        let mut s = Shake::default();
        let n = count(&mut s, &shake(2.5, 3.0, 3.0, 0.0));
        assert!((2..=3).contains(&n), "{n}");
    }

    #[test]
    fn walking_and_a_bump_are_not() {
        let mut s = Shake::default();
        assert_eq!(count(&mut s, &shake(0.6, 2.0, 10.0, 0.0)), 0);
        // one hard knock (set down on a table), then still
        let knock: Vec<_> = (0..60)
            .map(|i| {
                (
                    20.0 + i as f64 / 60.0,
                    [if i == 3 { 3.0 } else { 0.0 }, 0.0, 0.0],
                )
            })
            .collect();
        assert_eq!(count(&mut s, &knock), 0);
        // a hard set-down that rings: big, but within a tenth of a second
        let ring = [5.8, 0.0, 3.0, 0.0, 1.55, 0.0, 0.0];
        let rung: Vec<_> = ring
            .iter()
            .enumerate()
            .map(|(i, &x)| (40.0 + i as f64 / 60.0, [x, 0.0, 0.0]))
            .collect();
        assert_eq!(count(&mut s, &rung), 0);
    }

    #[test]
    fn the_systems_shake_and_ours_land_once() {
        let mut s = Shake::default();
        assert_eq!(count(&mut s, &shake(2.5, 3.0, 0.8, 0.0)), 1);
        // iOS's motionEnded comes as the shake ends
        assert!(!s.cooled(0.9));
        assert!(s.cooled(5.0));
    }
}
