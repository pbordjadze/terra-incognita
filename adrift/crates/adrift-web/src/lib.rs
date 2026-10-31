//! Adrift's web shell: one canvas, WebGPU through wgpu, pointer events in, frames out. The data is
//! fetched once from `assets/` beside the page (the library itself fetches nothing).

use adrift_core::{App, Gpu, MemoryAssets, Phase, PointerEvent};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

fn now(perf: &web_sys::Performance) -> f64 {
    perf.now() / 1000.0
}

async fn fetch(window: &web_sys::Window, url: &str) -> Result<Vec<u8>, JsValue> {
    let resp: web_sys::Response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url)).await?.dyn_into()?;
    if !resp.ok() {
        return Err(JsValue::from_str(&format!("{url}: {}", resp.status())));
    }
    let buf = wasm_bindgen_futures::JsFuture::from(resp.array_buffer()?).await?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

fn status(document: &web_sys::Document, text: &str) {
    if let Some(el) = document.get_element_by_id("status") {
        el.set_text_content(Some(text));
    }
}

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let perf = window.performance().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document.get_element_by_id("adrift").unwrap().dyn_into()?;
    if js_sys::Reflect::get(&window.navigator(), &"gpu".into())?.is_undefined() {
        status(&document, "This browser has no WebGPU. Safari 26, Chrome or Edge can open Adrift.");
        return Ok(());
    }
    let dpr = window.device_pixel_ratio();
    let rect = canvas.get_bounding_client_rect();
    let (w, h) = ((rect.width() * dpr) as u32, (rect.height() * dpr) as u32);
    canvas.set_width(w.max(1));
    canvas.set_height(h.max(1));

    status(&document, "the sea is loading…");
    let mut files = HashMap::new();
    for name in adrift_core::assets::FILES {
        files.insert(name.to_string(), fetch(&window, &format!("assets/{name}")).await?);
    }

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::BROWSER_WEBGPU, ..Default::default() });
    let surface = instance.create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone())).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let gpu = Gpu::new(&instance, Some(&surface)).await.map_err(|e| JsValue::from_str(&e.to_string()))?;
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: gpu.format,
        width: w.max(1),
        height: h.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&gpu.device, &config);
    let app = App::new(gpu, Box::new(MemoryAssets(files)), w, h, dpr).map_err(|e| JsValue::from_str(&e.to_string()))?;
    status(&document, "");
    let app = Rc::new(RefCell::new(app));

    // pointer events (points: CSS pixels)
    {
        let make = |phase: Phase, app: Rc<RefCell<App>>, perf: web_sys::Performance, canvas: web_sys::HtmlCanvasElement| {
            Closure::<dyn FnMut(web_sys::PointerEvent)>::new(move |e: web_sys::PointerEvent| {
                let r = canvas.get_bounding_client_rect();
                let (x, y) = (e.client_x() as f64 - r.left(), e.client_y() as f64 - r.top());
                if phase == Phase::Down {
                    let _ = canvas.set_pointer_capture(e.pointer_id());
                }
                e.prevent_default();
                app.borrow_mut().pointer(PointerEvent { id: e.pointer_id() as u64, phase, x, y, t: now(&perf) });
            })
        };
        for (name, phase) in [("pointerdown", Phase::Down), ("pointermove", Phase::Move), ("pointerup", Phase::Up), ("pointercancel", Phase::Cancel)] {
            let c = make(phase, app.clone(), perf.clone(), canvas.clone());
            canvas.add_event_listener_with_callback(name, c.as_ref().unchecked_ref())?;
            c.forget();
        }
        // the wheel zooms (a desktop's pinch); keys: z undo, space holds the date
        let a = app.clone();
        let wheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new(move |e: web_sys::WheelEvent| {
            e.prevent_default();
            a.borrow_mut().camera.zoom_by((-e.delta_y() / 400.0).exp());
        });
        canvas.add_event_listener_with_callback("wheel", wheel.as_ref().unchecked_ref())?;
        wheel.forget();
        let a = app.clone();
        let key = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let mut app = a.borrow_mut();
            match e.key().as_str() {
                "z" | "Z" => app.intend(adrift_core::Intent::Undo),
                " " => {
                    let running = !app.clock.running;
                    app.intend(adrift_core::Intent::SetRunning { running })
                }
                "ArrowRight" => app.intend(adrift_core::Intent::Scrub { days: 7.0 }),
                "ArrowLeft" => app.intend(adrift_core::Intent::Scrub { days: -7.0 }),
                _ => {}
            }
        });
        window.add_event_listener_with_callback("keydown", key.as_ref().unchecked_ref())?;
        key.forget();
    }

    // the frame loop
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let surface = Rc::new(surface);
    let gpu_device = app.borrow().gpu.device.clone();
    let canvas2 = canvas.clone();
    let window2 = window.clone();
    let app2 = app.clone();
    let perf2 = perf.clone();
    *g.borrow_mut() = Some(Closure::new(move || {
        let dpr = window2.device_pixel_ratio();
        let r = canvas2.get_bounding_client_rect();
        let (w, h) = (((r.width() * dpr) as u32).max(1), ((r.height() * dpr) as u32).max(1));
        if w != config.width || h != config.height {
            canvas2.set_width(w);
            canvas2.set_height(h);
            config.width = w;
            config.height = h;
            surface.configure(&gpu_device, &config);
            app2.borrow_mut().resize(w, h, dpr);
        }
        if let Ok(frame) = surface.get_current_texture() {
            let view = frame.texture.create_view(&Default::default());
            app2.borrow_mut().frame(&view, now(&perf2));
            frame.present();
        }
        window2.request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref()).unwrap();
    }));
    window.request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())?;
    Ok(())
}
