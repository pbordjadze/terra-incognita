//! Builds the toy's library of tides (`data/tides.bin`): for each pair of switches and each of fifteen
//! moons, the tide two tides after rest with its front 1.6 km short of Yanguan (or, with no front, the
//! flood's first rise there), too long to run at launch. Run from toys/:
//!
//!     cargo run --release -p bore --example library
use bore::tide::{age_moon, slot, write_library, Tide, AGES};

fn main() -> anyhow::Result<()> {
    let mut snaps: Vec<Option<bore::tide::Snap>> = (0..4 * AGES).map(|_| None).collect();
    let jobs: Vec<([bool; 2], usize)> =
        [[false, false], [false, true], [true, false], [true, true]]
            .iter()
            .flat_map(|&on| (0..AGES).map(move |a| (on, a)))
            .collect();
    let done: Vec<(usize, bore::tide::Snap)> = std::thread::scope(|sc| {
        let hs: Vec<_> = jobs
            .chunks(jobs.len().div_ceil(8))
            .map(|chunk| {
                sc.spawn(move || {
                    chunk
                        .iter()
                        .map(|&(on, a)| {
                            (
                                slot(on, a),
                                Tide::for_library(age_moon(a), on, 1600.0).snap(),
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    for (k, s) in done {
        snaps[k] = Some(s);
    }
    let snaps: Vec<_> = snaps.into_iter().map(|s| s.unwrap()).collect();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/tides.bin");
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, write_library(&snaps))?;
    for (k, s) in snaps.iter().enumerate() {
        let t = Tide::from_snap(s);
        println!(
            "{k:2} moon {:.2} on {:?}: front {:?}",
            s.moon,
            s.on,
            t.front().map(|f| (
                f.x as i64,
                (f.height() * 100.0).round() / 100.0,
                (f.froude * 100.0).round() / 100.0
            ))
        );
    }
    println!("wrote {}", path.display());
    Ok(())
}
