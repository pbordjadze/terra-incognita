//! Where the data comes from: a directory (Linux, the device's bundle) or memory (the web, which
//! fetched it first). Nothing is fetched by the library itself.

use std::collections::HashMap;
use std::path::PathBuf;

pub trait Assets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>>;
}

pub struct DirAssets(pub PathBuf);

impl Assets for DirAssets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        let p = self.0.join(name);
        std::fs::read(&p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))
    }
}

pub struct MemoryAssets(pub HashMap<String, Vec<u8>>);

impl Assets for MemoryAssets {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        self.0.get(name).cloned().ok_or_else(|| anyhow::anyhow!("no asset {name}"))
    }
}

/// The files the app loads, by the names the code uses (`adrift/data/manifest.json` maps them to
/// their places in the repository; `tools/adrift/assets.py` assembles a directory of them).
pub const FILES: &[&str] = &[
    "geo.png",
    "terrain.png",
    "floor.png",
    "currents.png",
    "stream.png",
    "ice-01.png", "ice-02.png", "ice-03.png", "ice-04.png", "ice-05.png", "ice-06.png",
    "ice-07.png", "ice-08.png", "ice-09.png", "ice-10.png", "ice-11.png", "ice-12.png",
    "land-01.jpg", "land-02.jpg", "land-03.jpg", "land-04.jpg", "land-05.jpg", "land-06.jpg",
    "land-07.jpg", "land-08.jpg", "land-09.jpg", "land-10.jpg", "land-11.jpg", "land-12.jpg",
    // Adrift's own sheets (Truth; adrift/SOURCES.md): the wind, the eddy wander and its time
    // scale, the real drifters' runs
    "wind.png",
    "eddy.png",
    "eddy-time.png",
    "drifters.bin",
];
