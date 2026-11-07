//! The device's C ABI, the same for every toy (`toys/ios/Sources/toy.h`; the Swift shell is
//! `toys/ios/Sources/main.swift`). A toy's crate says `toy_kit::ios_shell!(MyToy);` once and its
//! static library exports these `toy_*` functions.

use crate::{block_on, Ctx, DirAssets, Gpu, Host, Phase, PointerEvent, SharedSynth, Toy};

extern "C" {
    /// How many bytes the app may still take before the system kills it for memory (iOS 13+,
    /// <os/proc.h>).
    fn os_proc_available_memory() -> usize;
}

/// MB the app may still take before it is killed for memory (jetsam).
pub(crate) fn available_mb() -> usize {
    unsafe { os_proc_available_memory() >> 20 }
}

/// A toy on a CAMetalLayer.
pub struct Shell<T: Toy> {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub host: Host<T>,
    store: Option<std::path::PathBuf>,
    stats_at: f64,
    frames: u32,
    cost: f64,
    /// Frames drawn since launch (the first few, and a few seconds on, go in the log).
    drawn: u64,
    started: Option<f64>,
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
        let gpu = match block_on(Gpu::new(&instance, Some(&surface))) {
            Ok(g) => g,
            Err(e) => {
                crate::log(&format!("no device: {e:#}"));
                return None;
            }
        };
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: gpu.format,
            color_space: wgpu::SurfaceColorSpace::Srgb,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
        };
        surface.configure(&gpu.device, &config);
        crate::step("the device and the surface");
        let ctx = Ctx::new(
            gpu,
            Box::new(DirAssets(assets.into())),
            width.max(1),
            height.max(1),
            scale,
        );
        match Host::<T>::new(ctx) {
            Ok(host) => {
                crate::log(&format!(
                    "launch: the toy started, {} MB still available ({} MB made through the kit)",
                    available_mb(),
                    crate::budget::kit_bytes() >> 20
                ));
                Some(Shell {
                    _instance: instance,
                    surface,
                    config,
                    host,
                    store: None,
                    stats_at: 0.0,
                    frames: 0,
                    cost: 0.0,
                    drawn: 0,
                    started: None,
                })
            }
            Err(e) => {
                crate::log(&format!("the toy did not start: {e:#}"));
                None
            }
        }
    }

    pub fn resize(&mut self, width: u32, height: u32, scale: f64) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface
            .configure(&self.host.ctx.gpu.device, &self.config);
        self.host.resize(width, height, scale);
    }

    pub fn frame(&mut self, t: f64) {
        let start = std::time::Instant::now();
        self.host.update(t);
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f)
            | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface
                    .configure(&self.host.ctx.gpu.device, &self.config);
                return;
            }
            other => {
                eprintln!("toy: frame: {other:?}");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        self.host.draw(&view);
        self.host.ctx.gpu.queue.present(frame);
        self.drawn += 1;
        let since = t - *self.started.get_or_insert(t);
        // the first frames and the first seconds, in the log: a toy that dies after it started says
        // whether it ever drew, and how its memory went
        if self.drawn <= 3 || (self.drawn.is_multiple_of(60) && since < 20.0) {
            crate::step(&format!("frame {} drawn at {since:.1} s", self.drawn));
        }
        self.frames += 1;
        self.cost += start.elapsed().as_secs_f64();
        // the frame's CPU cost every five seconds, in the device's log
        if t - self.stats_at >= 5.0 {
            if self.frames > 0 {
                eprintln!(
                    "toy: {} frames, {:.2} ms a frame on the CPU",
                    self.frames,
                    self.cost * 1000.0 / self.frames as f64
                );
            }
            (self.stats_at, self.frames, self.cost) = (t, 0, 0.0);
        }
    }

    pub fn pointer(&mut self, id: u64, phase: u32, x: f64, y: f64, t: f64) {
        let phase = match phase {
            0 => Phase::Down,
            1 => Phase::Move,
            2 => Phase::Up,
            _ => Phase::Cancel,
        };
        self.host.pointer(PointerEvent { id, phase, x, y, t });
    }

    /// Where to keep the toy's state; what is there is taken back now.
    pub fn store(&mut self, path: &str) {
        let p = std::path::PathBuf::from(path);
        if let Ok(s) = std::fs::read_to_string(&p) {
            self.host.toy.restore(&mut self.host.ctx, &s);
        }
        self.store = Some(p);
    }

    pub fn pause(&mut self) {
        self.host.pause();
        if let (Some(p), Some(s)) = (&self.store, self.host.toy.save()) {
            let _ = std::fs::write(p, s);
        }
    }

    pub fn synth(&self) -> Option<SharedSynth> {
        self.host.synth.clone()
    }
}
