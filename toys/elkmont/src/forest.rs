//! The woods at Elkmont by night, seen from the trail at eye height: the canopy overhead with the sky
//! in its gaps, the trunks of tulip poplars and hemlocks going back into the dark, rhododendron low
//! among them, the leaf litter underfoot. The far woods fade into the night's haze. Drawn, not
//! measured: the place is real, its trees are made up after it.

use crate::swarm::rand;

/// The watcher's eye above the ground (m).
pub const EYE: f32 = 1.6;

/// One trunk: where it stands (x, z), its radius (m) and its lean (m of x per m of height).
#[derive(Clone, Copy)]
pub struct Trunk {
    pub x: f32,
    pub z: f32,
    pub r: f32,
    pub lean: f32,
}

/// One clump of rhododendron or fern: its centre (x, y, z), its size (m), its shade.
#[derive(Clone, Copy)]
pub struct Leaf {
    pub at: [f32; 3],
    pub size: f32,
    pub shade: f32,
}

/// One fern by the trail: where it grows (x, z), its fronds' length (m), its turn.
#[derive(Clone, Copy)]
pub struct Fern {
    pub x: f32,
    pub z: f32,
    pub size: f32,
    pub turn: f32,
}

pub struct Forest {
    pub trunks: Vec<Trunk>,
    pub leaves: Vec<Leaf>,
    pub ferns: Vec<Fern>,
}

impl Forest {
    pub fn new(seed: u64) -> Forest {
        let mut s = seed | 1;
        let mut trunks = Vec::new();
        // the trail runs straight away from the watcher: no trunk stands on it
        let place = |s: &mut u64, z0: f32, z1: f32, r0: f32, r1: f32| -> Option<Trunk> {
            let z = z0 + (z1 - z0) * rand(s) as f32;
            let half = z * 0.75 + 3.0;
            let x = (rand(s) as f32 * 2.0 - 1.0) * half;
            if x.abs() < 1.1 + z * 0.02 {
                return None;
            }
            Some(Trunk {
                x,
                z,
                r: r0 + (r1 - r0) * (rand(s) as f32).powi(2),
                lean: (rand(s) as f32 - 0.5) * 0.04,
            })
        };
        // two near trunks framing the view
        trunks.push(Trunk {
            x: -1.9,
            z: 2.6,
            r: 0.38,
            lean: 0.01,
        });
        trunks.push(Trunk {
            x: 2.6,
            z: 4.2,
            r: 0.3,
            lean: -0.012,
        });
        while trunks.len() < 56 {
            if let Some(t) = place(&mut s, 4.0, 80.0, 0.12, 0.55) {
                trunks.push(t);
            }
        }
        let mut leaves = Vec::new();
        while leaves.len() < 260 {
            let z = 4.0 + 55.0 * (rand(&mut s) as f32).powf(1.3);
            let half = z * 0.75 + 3.0;
            let x = (rand(&mut s) as f32 * 2.0 - 1.0) * half;
            if x.abs() < 0.9 + z * 0.01 {
                continue;
            }
            let size = 0.3 + 0.6 * rand(&mut s) as f32;
            leaves.push(Leaf {
                at: [x, size * 0.5 * rand(&mut s) as f32 + 0.1, z],
                size,
                shade: rand(&mut s) as f32,
            });
        }
        // ferns and Christmas ferns along the trail's edges, near the watcher's feet
        let mut ferns = Vec::new();
        while ferns.len() < 70 {
            let z = 1.2 + 9.0 * (rand(&mut s) as f32).powf(1.4);
            let side = if rand(&mut s) < 0.5 { -1.0 } else { 1.0 };
            let x = side * (0.55 + z * 0.02 + 2.5 * rand(&mut s) as f32);
            ferns.push(Fern {
                x,
                z,
                size: 0.35 + 0.35 * rand(&mut s) as f32,
                turn: rand(&mut s) as f32 * std::f32::consts::TAU,
            });
        }
        Forest {
            trunks,
            leaves,
            ferns,
        }
    }
}

/// The backdrop (toy-kit backdrop.rs): `u.p0`, `u.p1` = [the horizon's y (px), the focal length (px), dusk
/// (1 at the light's end, 0 by night), the swarm's light, the view's sway (px), the screen's pixels a
/// point, 0, 0].
pub const BACKDROP_WGSL: &str = r#"
fn hash3(p: vec2<f32>) -> f32 {
  return fract(sin(dot(p, vec2(41.3, 289.1))) * 43758.5453);
}
fn fbm(p: vec2<f32>) -> f32 {
  var a = 0.0; var w = 0.5; var q = p;
  for (var i = 0; i < 4; i++) { a += w * noise2(q); q = q * 2.03 + vec2(1.7, 9.2); w *= 0.5; }
  return a;
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let horizon = u.p0.x;
  let f = u.p0.y;
  let dusk = u.p0.z;
  let glow = u.p0.w;
  let ppp = u.p1.y;
  let d = vec2((px.x - u.size.x * 0.5 - u.p1.x) / f, (horizon - px.y) / f);
  // the night: a deep blue sky, lighter toward the west's last light at dusk
  let sky_night = vec3(0.034, 0.050, 0.105);
  let sky_dusk = vec3(0.20, 0.29, 0.50);
  let sky = mix(sky_night, sky_dusk, dusk);
  // the haze the far woods fade into
  let haze = mix(vec3(0.034, 0.044, 0.068), vec3(0.13, 0.17, 0.26), dusk);
  // the fireflies' light, caught in the haze and on the leaves
  let green = vec3(0.30, 0.55, 0.10) * glow;
  var col: vec3<f32>;
  if (d.y > 0.0) {
    // up: the far woods' wall, then the canopy with sky in its gaps
    let t = (24.0 - 1.6) / d.y; // where the ray meets the canopy (m)
    let c = vec2(d.x * t, t) * 0.9;
    let leaf = 0.55 * fbm(c) + 0.45 * fbm(c * 5.3);
    let gap = smoothstep(0.63, 0.67, leaf);
    let far_c = smoothstep(0.1, 1.2, d.y);
    let twinkle = hash3(floor(px / (2.0 * ppp)));
    var star = 0.0;
    if (twinkle > 0.9965) { star = 0.55 * (1.0 - dusk) * (0.5 + 0.5 * sin(u.time * 2.0 + twinkle * 300.0)); }
    let open = mix(sky + vec3(star), vec3(0.012, 0.016, 0.020) + green * 0.22 * (1.0 - 0.6 * far_c) * (0.6 + 0.4 * leaf), 1.0 - gap);
    // toward the horizon the canopy is the far woods: haze
    let far = smoothstep(0.02, 0.32, d.y);
    col = mix(haze + green * 0.3, open, far);
  } else {
    // down: the leaf litter, darker near; the trail a little paler
    let t = 1.6 / max(-d.y, 1e-4);
    let g = vec2(d.x * t, t);
    // the litter: last year's leaves, in flecks the light catches
    let litter = 0.5 + 0.3 * fbm(g * 1.3) + 0.35 * smoothstep(0.5, 0.85, noise2(g * 26.0 + vec2(fbm(g * 6.0) * 3.0, 0.0)));
    let trail = 1.0 - smoothstep(0.7, 1.3, abs(g.x));
    var ground = vec3(0.030, 0.034, 0.030) * litter + vec3(0.030, 0.030, 0.027) * trail;
    ground += green * 0.38 * litter * (0.2 + 0.8 * exp(-t / 5.0));
    let mist = 1.0 - exp(-t / 22.0);
    col = mix(ground, haze + green * 0.5, mist);
  }
  return vec4(col, 1.0);
}
"#;
