//! The one frame core every platform stands on (Nov 22 2026: Terra native on iPhone, Pixel, Samsung and the
//! web from one library). A platform's frame makes a `wgpu::Surface` from its own window (a CAMetalLayer, an
//! Android `Surface`, a canvas: `shell.rs`, `android.rs`, `toys/terra-web`) and hands it here; from then on
//! every platform drives the toy through the same calls: a frame at the display's pace, fingers in points,
//! what the phone feels, the chrome as data (`chrome.rs`) out and the reader's answers in. What differs
//! between platforms is only the surface, the sound's plumbing and the frame's own controls.

use crate::chrome::{Chrome, In};
use crate::{Assets, Ctx, Gpu, Host, Phase, PointerEvent, Sensors, SharedSynth, Toy};

/// A toy on a platform's surface.
pub struct Frame<T: Toy> {
    /// (kept: some platforms lose the device once its instance is gone)
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pub host: Host<T>,
    /// The chrome as the frame last read it, and as JSON.
    chrome: Option<Chrome>,
    chrome_json: String,
    stats_at: f64,
    frames: u32,
    cost: f64,
    /// Frames drawn since launch (the first few, and a few seconds on, go in the log).
    drawn: u64,
    started: Option<f64>,
}

impl<T: Toy> Frame<T> {
    /// Starts the toy on `surface` (`width` x `height` pixels at `scale` pixels a point), its data in
    /// `assets`. Native platforms wait on it with `block_on`; the web awaits it.
    pub async fn start(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
        scale: f64,
        assets: Box<dyn Assets>,
    ) -> anyhow::Result<Frame<T>> {
        let gpu = Gpu::new(&instance, Some(&surface)).await?;
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
        let ctx = Ctx::new(gpu, assets, width.max(1), height.max(1), scale);
        let host = Host::<T>::new(ctx)?;
        Ok(Frame {
            _instance: instance,
            surface,
            config,
            host,
            chrome: None,
            chrome_json: String::new(),
            stats_at: 0.0,
            frames: 0,
            cost: 0.0,
            drawn: 0,
            started: None,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, scale: f64) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface
            .configure(&self.host.ctx.gpu.device, &self.config);
        self.host.resize(width, height, scale);
    }

    /// A frame at `t` (the frame's seconds).
    pub fn frame(&mut self, t: f64) {
        let start = crate::wall_clock();
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
                crate::log(&format!("frame: {other:?}"));
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
        self.cost += crate::wall_clock() - start;
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

    /// A finger: `phase` 0 down, 1 moved, 2 up, 3 cancelled; points from the top left.
    pub fn pointer(&mut self, id: u64, phase: u32, x: f64, y: f64, t: f64) {
        let phase = match phase {
            0 => Phase::Down,
            1 => Phase::Move,
            2 => Phase::Up,
            _ => Phase::Cancel,
        };
        self.host.pointer(PointerEvent { id, phase, x, y, t });
    }

    /// The device's own acceleration (g, gravity out) at `t`: a few hard jolts are a shake.
    pub fn motion(&mut self, a: [f64; 3], t: f64) {
        self.host.motion(a, t);
    }

    /// What the phone feels, in the device's frame held upright (`Sensors`); a NaN in a vector, or a NaN
    /// heading, is "no such sense".
    pub fn sense(&mut self, g: [f64; 3], b: [f64; 3], heading: f64) {
        let v = |a: [f64; 3]| a.iter().all(|x| x.is_finite()).then_some(a);
        self.host.sense(Sensors {
            gravity: v(g),
            field: v(b),
            heading: heading.is_finite().then_some(heading),
        });
    }

    /// The chrome as JSON (`chrome::Chrome`), when it changed since the last call; None while it is the same.
    pub fn chrome_json(&mut self) -> Option<&str> {
        let c = self.host.chrome();
        if self.chrome.as_ref() == Some(&c) {
            return None;
        }
        self.chrome_json = serde_json::to_string(&c).unwrap_or_default();
        self.chrome = Some(c);
        Some(&self.chrome_json)
    }

    /// What the toy asks of the frame now (`chrome::Out`), as a JSON list; None when nothing.
    pub fn events_json(&mut self) -> Option<String> {
        let e = self.host.take_events();
        (!e.is_empty()).then(|| serde_json::to_string(&e).unwrap_or_default())
    }

    /// What the reader did in the frame's controls (`chrome::In`, as JSON). A message this kit can't read
    /// is logged and dropped: a newer frame's on an older library.
    pub fn send_json(&mut self, json: &str) {
        match serde_json::from_str::<In>(json) {
            Ok(e) => self.host.send(e),
            Err(e) => crate::log(&format!("frame: {json}: {e}")),
        }
    }

    /// What the toy keeps when the app goes away, and taking it back (`Toy::save`, `restore`).
    pub fn save(&self) -> Option<String> {
        self.host.toy.save()
    }

    pub fn restore(&mut self, saved: &str) {
        self.host.toy.restore(&mut self.host.ctx, saved);
    }

    /// The app went away: the next frame's dt is 0, not the time away.
    pub fn pause(&mut self) {
        self.host.pause();
    }

    pub fn synth(&self) -> Option<SharedSynth> {
        self.host.synth.clone()
    }
}
