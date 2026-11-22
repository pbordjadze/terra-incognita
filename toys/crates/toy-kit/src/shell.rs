//! The device's C ABI, the same for every toy (`toys/ios/Sources/toy.h`; the Swift shell is
//! `toys/ios/Sources/main.swift`). A toy's crate says `toy_kit::ios_shell!(MyToy);` once and its
//! static library exports these `toy_*` functions.

use crate::{block_on, DirAssets, Frame, Toy};

extern "C" {
    /// How many bytes the app may still take before the system kills it for memory (iOS 13+,
    /// <os/proc.h>).
    fn os_proc_available_memory() -> usize;
}

/// MB the app may still take before it is killed for memory (jetsam).
pub(crate) fn available_mb() -> usize {
    unsafe { os_proc_available_memory() >> 20 }
}

/// A toy on a CAMetalLayer: the one frame core (`frame.rs`) on iOS's surface, and where its state is kept.
pub struct Shell<T: Toy> {
    pub frame: Frame<T>,
    store: Option<std::path::PathBuf>,
}

impl<T: Toy> Shell<T> {
    /// # Safety
    /// `layer` is a live CAMetalLayer.
    pub unsafe fn new(
        layer: *mut std::ffi::c_void,
        width: u32,
        height: u32,
        scale: f64,
        assets: &str,
    ) -> Option<Shell<T>> {
        // the launch's memory in the log: if the system kills the toy for memory while it starts, the
        // second line is missing and the first says how much there was (tools/kit/launch_check.py's
        // budget is checked against these)
        crate::log(&format!(
            "launch: {} MB available to the app",
            available_mb()
        ));
        crate::log(&format!(
            "launch: {} {}x{} px at {scale}x",
            env!("CARGO_PKG_VERSION"),
            width,
            height
        ));
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::METAL,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = match instance
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer))
        {
            Ok(s) => s,
            Err(e) => {
                crate::log(&format!("no surface: {e}"));
                return None;
            }
        };
        let assets = Box::new(DirAssets(assets.into()));
        match block_on(Frame::<T>::start(
            instance, surface, width, height, scale, assets,
        )) {
            Ok(frame) => {
                crate::log(&format!(
                    "launch: the toy started, {} MB still available ({} MB made through the kit)",
                    available_mb(),
                    crate::budget::kit_bytes() >> 20
                ));
                Some(Shell { frame, store: None })
            }
            Err(e) => {
                crate::log(&format!("the toy did not start: {e:#}"));
                None
            }
        }
    }

    /// Where to keep the toy's state; what is there is taken back now.
    pub fn store(&mut self, path: &str) {
        let p = std::path::PathBuf::from(path);
        if let Ok(s) = std::fs::read_to_string(&p) {
            self.frame.restore(&s);
        }
        self.store = Some(p);
    }

    /// The app went away: the clock forgotten, the state kept.
    pub fn pause(&mut self) {
        self.frame.pause();
        if let (Some(p), Some(s)) = (&self.store, self.frame.save()) {
            let _ = std::fs::write(p, s);
        }
    }
}
