//! One leaf, filling the picture: the backdrop draws its blade (scene.wgsl › leaf) from the twist and
//! swing kept here; its stalk is painted over it, a ribbon flattened side to side that turns from
//! face-on at the twig to edge-on and over as the blade twists. Its motion is the same spring as every
//! leaf on the stem (stem.rs › Leaf), only near.

use crate::stem::Leaf;
use crate::Cam;
use std::f32::consts::PI;
use toy_kit::paint::Painter;

pub struct OneLeaf {
    pub leaf: Leaf,
    /// The finger's breath on it: strength (m/s) and age (s).
    breath: f32,
}

impl OneLeaf {
    pub fn new(seed: u32, freq: f32) -> OneLeaf {
        OneLeaf {
            leaf: Leaf {
                anchor: [0.46, 0.10],
                stalk: 0.52,
                blade: 0.95,
                rest: PI / 2.0,
                twist: 0.3,
                twist_v: 0.0,
                swing: 0.0,
                swing_v: 0.0,
                freq,
                seed,
            },
            breath: 0.0,
        }
    }

    /// A finger's drag across it: its speed (screen widths a second).
    pub fn breathe(&mut self, speed: f32) {
        self.breath = self.breath.max(6.0 * (1.0 - (-speed / 1.2).exp()));
    }

    pub fn step(&mut self, w: f32, t: f32, dt: f32) {
        self.breath *= (-dt / 0.9).exp();
        let n = (dt / (1.0 / 240.0)).ceil().max(1.0) as usize;
        let h = dt / n as f32;
        // the blade is large and the stalk long here, but the leaf is the same leaf: only its swing is
        // held down a little so it stays in the picture
        // near, a leaf in the breeze mostly shows its face, flicking over in the gusts: the wind it
        // feels here is the stem's, eased (it hangs low and in the lee of the crown)
        let felt = 0.7 * w + self.breath;
        for s in 0..n {
            self.leaf.step(felt, t + s as f32 * h, h);
        }
        self.leaf.swing = self.leaf.swing.clamp(-0.18, 0.18);
    }

    /// The blade's base (screen widths) and the angle the stalk points.
    pub fn base(&self) -> ([f32; 2], f32) {
        let l = &self.leaf;
        let ang = l.angle();
        (
            [
                l.anchor[0] + ang.cos() * l.stalk,
                l.anchor[1] + ang.sin() * l.stalk,
            ],
            ang,
        )
    }

    /// How squarely the leaf's face is turned to the sun (0 edge-on or its back, 1 face on).
    pub fn facing(&self) -> f32 {
        self.leaf.twist.cos().max(0.0)
    }

    pub fn draw(&self, p: &mut Painter, cam: &Cam, w: f32, fade: f32) {
        let px = |q: [f32; 2]| cam.pt([q[0] * w, q[1] * w]);
        let len = |r: f32| cam.len(r * w);
        let l = &self.leaf;
        let (base, _) = self.base();
        // the twig it hangs from
        p.line(
            px([-0.1, l.anchor[1] + 0.06]),
            px([1.1, l.anchor[1] - 0.05]),
            len(0.022),
            len(0.004),
            [0.34, 0.30, 0.25, fade],
        );
        // the stalk, flattened: wide where it faces us, a thread where it turns edge-on
        let n = 28;
        for i in 0..n {
            let u0 = i as f32 / n as f32;
            let u1 = (i + 1) as f32 / n as f32;
            let at = |u: f32| {
                // a slight S, as a stalk hangs
                let bend = 0.025 * (u * PI).sin() * (1.0 + l.swing * 2.0);
                [
                    l.anchor[0] + (base[0] - l.anchor[0]) * u + bend,
                    l.anchor[1] + (base[1] - l.anchor[1]) * u,
                ]
            };
            let tw = (l.twist * (u0 + u1) * 0.5).cos();
            let wid = 0.0035 + 0.0075 * tw.abs();
            let shade = 0.65 + 0.35 * tw.abs();
            p.line(
                px(at(u0)),
                px(at(u1)),
                len(wid),
                len(0.0015),
                [0.86 * shade, 0.64 * shade, 0.22 * shade, fade],
            );
        }
    }
}
