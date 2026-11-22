//! The kit's clocks: the wall's, and one for how long a frame's work has taken.

/// Seconds since 1970 by the wall clock. A browser's WebAssembly has no clock of its own (std's
/// `SystemTime::now` panics there), so on wasm32 the web shell sets it each frame (`set_wall_clock`).
pub fn wall_clock() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        f64::from_bits(WALL.load(std::sync::atomic::Ordering::Relaxed))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

/// A clock for how long work in a frame has taken (seconds, from any start), to stop it in time
/// (`Toy::warm`). None in the headless check, whose frames must come out the same on every run however
/// fast the machine (it does all such work at once), and on wasm32 outside the web shell.
pub fn work_clock() -> Option<f64> {
    if HEADLESS.load(std::sync::atomic::Ordering::Relaxed) {
        return None;
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now() / 1000.0)
    }
    #[cfg(all(not(feature = "web"), target_arch = "wasm32"))]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        Some(
            START
                .get_or_init(std::time::Instant::now)
                .elapsed()
                .as_secs_f64(),
        )
    }
}

#[cfg(target_arch = "wasm32")]
static WALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The page's clock (JavaScript's `Date.now() / 1000`), for `wall_clock` on wasm32.
#[cfg(target_arch = "wasm32")]
pub fn set_wall_clock(secs: f64) {
    WALL.store(secs.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

/// From here on the process is the headless check (`check::gpu`): `work_clock` is None.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn headless() {
    HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
}

static HEADLESS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
