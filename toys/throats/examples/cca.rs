//! Tuning the cyclic automaton: `cargo run --release -p throats --example cca -- <r> <t> <q> <out.png> [w h steps]`
//! A rock's lizard is displaced by the colour that beats it when at least t of its neighbours within r
//! (a box) wear that colour, with probability q a step. Writes a strip of the field at four times.
use throats::sim::{beater, Rng};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let r: i32 = a[1].parse().unwrap();
    let t: u32 = a[2].parse().unwrap();
    let q: f32 = a[3].parse().unwrap();
    let out = &a[4];
    let w: usize = a.get(5).map(|s| s.parse().unwrap()).unwrap_or(200);
    let h: usize = a.get(6).map(|s| s.parse().unwrap()).unwrap_or(200);
    let steps: usize = a.get(7).map(|s| s.parse().unwrap()).unwrap_or(400);
    let mut rng = Rng(12345);
    let mut k: Vec<u8> = (0..w * h).map(|_| 1 + rng.below(3) as u8).collect();
    let mut img = image::RgbImage::new((w * 4 + 12) as u32, h as u32);
    let shots = [steps / 8, steps / 4, steps / 2, steps];
    let mut col = 0;
    for s in 1..=steps {
        let mut next = k.clone();
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let i = y as usize * w + x as usize;
                let p = beater(k[i]);
                let mut c = 0;
                for dy in -r..=r {
                    for dx in -r..=r {
                        let j = ((y + dy).rem_euclid(h as i32) as usize) * w
                            + (x + dx).rem_euclid(w as i32) as usize;
                        if k[j] == p {
                            c += 1;
                        }
                    }
                }
                if c >= t && rng.unit() < q {
                    next[i] = p;
                }
            }
        }
        k = next;
        if shots.contains(&s) {
            for (i, &v) in k.iter().enumerate() {
                let c = match v {
                    1 => [230, 120, 30],
                    2 => [40, 90, 220],
                    3 => [235, 210, 40],
                    _ => [30, 25, 20],
                };
                img.put_pixel(
                    (col * (w + 4) + i % w) as u32,
                    (i / w) as u32,
                    image::Rgb(c),
                );
            }
            col += 1;
        }
    }
    img.save(out).unwrap();
}
