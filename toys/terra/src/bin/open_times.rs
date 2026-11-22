//! How long each toy Terra holds takes to open: the work a touch on its light does at once (`rooms::open`),
//! then its warming (`Toy::warm`), which Terra spreads over frames while the card rises: its slices, the
//! longest of them, and all of it. A slice past a few milliseconds is a stall the reader feels; on the web,
//! whose WebAssembly is slower and has one thread, a frozen page (a friend's trace, November 22).
//!
//!     python3 ../tools/toys/assets.py terra
//!     cargo run -p terra --bin terra-open-times --release [-- <toy id>…]
//!
//! Native and headless (Mesa's lavapipe on Linux, whose pipelines cost more than a phone's GPU); the web's
//! WebAssembly runs the same work about two to three times slower.
use std::sync::Arc;
use std::time::Instant;
use toy_kit::{Assets, Ctx, DirAssets};

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn main() -> anyhow::Result<()> {
    let want: Vec<String> = std::env::args().skip(1).collect();
    let gpu = toy_kit::check::gpu()?;
    let dir = toy_kit::check::assets_dir("terra");
    let assets: Arc<dyn Assets> = Arc::new(DirAssets(dir.clone()));
    let ctx = Ctx::new(gpu, Box::new(DirAssets(dir)), 390 * 3, 844 * 3, 3.0);
    println!(
        "{:<12} {:>8} {:>7} {:>9} {:>9}  ms",
        "toy", "open", "slices", "longest", "warming"
    );
    for door in terra::doors::doors() {
        if !want.is_empty() && !want.iter().any(|w| w == door.id) {
            continue;
        }
        let t0 = Instant::now();
        let mut room = (door.open)(&ctx, &assets)?;
        let opened = ms(t0);
        let (mut slices, mut longest) = (0, 0.0f64);
        let t1 = Instant::now();
        loop {
            let t = Instant::now();
            let done = room.warm();
            slices += 1;
            longest = longest.max(ms(t));
            if done {
                break;
            }
        }
        let warming = ms(t1);
        room.update(0.0);
        println!(
            "{:<12} {opened:8.1} {slices:7} {longest:9.1} {warming:9.1}",
            door.id
        );
    }
    Ok(())
}
