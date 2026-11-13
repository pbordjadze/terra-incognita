//! Builds Barchans' opening field, `data/opening.bin`: the dune rule run from scattered heaps until
//! crescents have formed and walked (too long to run at launch), and a hill-shaded preview beside it.
//!
//!     cd toys && cargo run --release -p barchans --example opening -- barchans/data/opening.bin [sweeps, 600]
use barchans::dunes::Dunes;
use barchans::{FIELD, SEED, TRADE};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = a.get(1).cloned().unwrap_or("barchans/data/opening.bin".into());
    let sweeps: usize = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(600);
    let (w, h) = (FIELD[0], FIELD[1]);
    let mut d = Dunes::new(w, h, SEED, TRADE);
    let t0 = std::time::Instant::now();
    for _ in 0..sweeps {
        d.run(w * h);
    }
    let el = t0.elapsed().as_secs_f64();
    println!("{sweeps} sweeps in {el:.1} s ({:.1} ms a sweep), tallest {}", el * 1000.0 / sweeps as f64, d.tallest);
    std::fs::write(&out, d.to_bytes()).expect("write the field");
    let mut img = image::RgbImage::new(w as u32, h as u32);
    for y in 0..h {
        for x in 0..w {
            let s = d.slabs[y * w + x];
            let e = d.slabs[y * w + (x + 1) % w] - s;
            let v = if s <= 1 { 90.0 } else { (160.0 + (s as f32) * 1.5 - e as f32 * 12.0).clamp(0.0, 255.0) };
            img.put_pixel(x as u32, y as u32, image::Rgb([v as u8, (v * 0.8) as u8, (v * 0.6) as u8]));
        }
    }
    let png = std::path::Path::new(&out).with_extension("png");
    img.save(&png).expect("write the preview");
    println!("wrote {out} and {}", png.display());
}
