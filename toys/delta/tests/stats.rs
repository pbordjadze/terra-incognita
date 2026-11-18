use delta::land::*;
#[test]
fn stats() {
    let mut l = Land::new(72, 156, 0x0CA7_A460);
    for s in 0..40 {
        l.run(1.0, 1440);
        let tot: f32 = l.water.iter().sum();
        let wet = l.water.iter().filter(|w| **w > WET).count();
        let any = l.water.iter().filter(|w| **w > 0.0).count();
        let mx = l.water.iter().cloned().fold(0.0, f32::max);
        let maxy = (0..l.water.len())
            .filter(|i| l.water[*i] > WET)
            .map(|i| i / 72)
            .max();
        if s % 5 == 0 {
            eprintln!("t={s} tot={tot:.1} wet={wet} any={any} max={mx:.2} maxy={maxy:?}");
        }
    }
}
