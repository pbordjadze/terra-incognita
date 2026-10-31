//! The device's C ABI (adrift/ios/Sources/Adrift/*.swift calls it): a CAMetalLayer in, frames out.

use crate::app::App;
use crate::assets::DirAssets;
use crate::gpu::Gpu;
use crate::model::{Phase, PointerEvent};
use std::ffi::{c_char, c_void, CStr};

pub struct Shell {
    _instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    app: App,
    store: Option<std::path::PathBuf>,
}

/// Makes the app over a CAMetalLayer. `scale` is the screen's pixels a point; `assets` the bundle's
/// resource directory. Null on failure (the reason is logged).
#[no_mangle]
pub unsafe extern "C" fn adrift_new(layer: *mut c_void, width: u32, height: u32, scale: f64, assets: *const c_char) -> *mut Shell {
    let dir = CStr::from_ptr(assets).to_string_lossy().into_owned();
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::METAL, ..Default::default() });
    let surface = match instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(layer)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("adrift: no surface: {e}");
            return std::ptr::null_mut();
        }
    };
    let gpu = match pollster_lite(Gpu::new(&instance, Some(&surface))) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("adrift: no device: {e}");
            return std::ptr::null_mut();
        }
    };
    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gpu.format,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);
    let app = match App::new(gpu, Box::new(DirAssets(dir.into())), width, height, scale) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("adrift: {e}");
            return std::ptr::null_mut();
        }
    };
    Box::into_raw(Box::new(Shell { _instance: instance, surface, config, app, store: None }))
}

#[no_mangle]
pub unsafe extern "C" fn adrift_resize(shell: *mut Shell, width: u32, height: u32, scale: f64) {
    let Some(s) = shell.as_mut() else { return };
    s.config.width = width.max(1);
    s.config.height = height.max(1);
    s.surface.configure(&s.app.gpu.device, &s.config);
    s.app.resize(width, height, scale);
}

/// A frame at the display's time `t` (seconds).
#[no_mangle]
pub unsafe extern "C" fn adrift_frame(shell: *mut Shell, t: f64) {
    let Some(s) = shell.as_mut() else { return };
    let frame = match s.surface.get_current_texture() {
        Ok(f) => f,
        Err(wgpu::SurfaceError::Outdated) | Err(wgpu::SurfaceError::Lost) => {
            s.surface.configure(&s.app.gpu.device, &s.config);
            return;
        }
        Err(e) => {
            eprintln!("adrift: frame: {e}");
            return;
        }
    };
    let view = frame.texture.create_view(&Default::default());
    s.app.frame(&view, t);
    frame.present();
}

/// A touch: `phase` 0 down, 1 move, 2 up, 3 cancel; x, y in points; t seconds.
#[no_mangle]
pub unsafe extern "C" fn adrift_pointer(shell: *mut Shell, id: u64, phase: u32, x: f64, y: f64, t: f64) {
    let Some(s) = shell.as_mut() else { return };
    let phase = match phase {
        0 => Phase::Down,
        1 => Phase::Move,
        2 => Phase::Up,
        _ => Phase::Cancel,
    };
    s.app.pointer(PointerEvent { id, phase, x, y, t });
}

/// The device was shaken: undo (touch/VOCABULARY.md).
#[no_mangle]
pub unsafe extern "C" fn adrift_shaken(shell: *mut Shell) {
    if let Some(s) = shell.as_mut() {
        s.app.shaken();
    }
}

/// Whether a Put or a Hold happened since the shell last asked (it plays a light haptic: the beat is
/// felt as well as seen); cleared by the asking.
#[no_mangle]
pub unsafe extern "C" fn adrift_take_haptic(shell: *mut Shell) -> bool {
    match shell.as_mut() {
        Some(s) => std::mem::take(&mut s.app.haptic),
        None => false,
    }
}

/// Where the state is kept between launches (a file in the app's documents): read now, written by
/// `adrift_pause`.
#[no_mangle]
pub unsafe extern "C" fn adrift_store(shell: *mut Shell, path: *const c_char) {
    let Some(s) = shell.as_mut() else { return };
    let p = std::path::PathBuf::from(CStr::from_ptr(path).to_string_lossy().into_owned());
    if let Ok(json) = std::fs::read_to_string(&p) {
        s.app.restore(&json);
    }
    s.store = Some(p);
}

/// The app is going to the background: the state is written.
#[no_mangle]
pub unsafe extern "C" fn adrift_pause(shell: *mut Shell) {
    let Some(s) = shell.as_mut() else { return };
    if let Some(p) = &s.store {
        if let Err(e) = std::fs::write(p, s.app.snapshot()) {
            eprintln!("adrift: store: {e}");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn adrift_set_reduce_motion(shell: *mut Shell, on: bool) {
    if let Some(s) = shell.as_mut() {
        s.app.reduce_motion = on;
    }
}

#[no_mangle]
pub unsafe extern "C" fn adrift_free(shell: *mut Shell) {
    if !shell.is_null() {
        drop(Box::from_raw(shell));
    }
}

/// Runs a future that is ready at once or after the Metal backend's synchronous work (wgpu's Metal
/// adapter and device requests complete without a reactor).
fn pollster_lite<F: std::future::Future>(f: F) -> F::Output {
    use std::sync::Arc;
    use std::task::{Context, Poll, Wake, Waker};
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}
