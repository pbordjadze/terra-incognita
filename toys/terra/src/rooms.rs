//! The toys Terra holds, each behind one face (`Room`): a toy and its own `Ctx` on Terra's device and
//! data, driven by Terra as a shell drives a toy. Its info and rules become Terra's while it is open;
//! its haptic taps pass through; its sound goes to Terra's mixer.

use std::sync::Arc;
use toy_kit::info::{Demo, Rules};
use toy_kit::{Assets, Camera, Ctx, Gpu, PointerEvent, Synth, Toy};

/// Terra's data, shared by every toy it opens (the toys' manifests merged: `manifest.json`).
pub struct Shared(pub Arc<dyn Assets>);

impl Assets for Shared {
    fn read(&self, name: &str) -> anyhow::Result<Vec<u8>> {
        self.0.read(name)
    }
}

/// A toy as Terra drives it.
pub trait Room {
    fn pointer(&mut self, e: PointerEvent);
    fn update(&mut self, t: f64);
    fn draw(&mut self, target: &wgpu::TextureView);
    fn resize(&mut self, width: u32, height: u32, ppp: f64);
    fn settings(&mut self, night: bool, reduce_motion: bool);
    fn demo(&self) -> Option<Demo>;
    fn rules(&self) -> Option<Rules>;
    fn origins(&self) -> Option<Rules>;
    fn card_at_top(&self) -> bool;
    fn synth(&mut self) -> Option<Box<dyn Synth>>;
    fn take_haptic(&mut self) -> bool;
    /// The device was shaken (Frost sweeps its window clear).
    fn shaken(&mut self);
    /// The toy's globe camera now, if it has a globe (read from what it saves: every globe toy keeps
    /// its camera, as `camera` or last in a list).
    fn camera(&self) -> Option<Camera>;
    /// Puts the toy's globe camera at `cam` (a globe toy takes back what it saved with the camera in it
    /// changed: every one keeps its camera, as `camera` or last in a list). False: no globe camera.
    fn set_camera(&mut self, cam: Camera) -> bool;
    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]>;
    fn anchors(&self) -> Vec<(String, [f64; 2])>;
    /// Where the toy lies, when its picture is the ground seen from above (`Toy::seen_from_above`).
    fn seen_from_above(&self) -> Option<toy_kit::Above>;
}

pub struct Inside<T: Toy> {
    ctx: Ctx,
    toy: T,
    last_t: Option<f64>,
}

/// A new context on the same device and data, the screen's size and settings.
pub fn context(from: &Ctx, assets: &Arc<dyn Assets>) -> Ctx {
    let g = &from.gpu;
    let gpu = Gpu {
        device: g.device.clone(),
        queue: g.queue.clone(),
        format: g.format,
        adapter_info: g.adapter_info.clone(),
    };
    let mut ctx = Ctx::new(
        gpu,
        Box::new(Shared(assets.clone())),
        from.width,
        from.height,
        from.ppp,
    );
    ctx.night = from.night;
    ctx.reduce_motion = from.reduce_motion;
    ctx
}

pub fn open<T: Toy + 'static>(
    from: &Ctx,
    assets: &Arc<dyn Assets>,
) -> anyhow::Result<Box<dyn Room>> {
    let mut ctx = context(from, assets);
    let toy = T::new(&mut ctx)?;
    Ok(Box::new(Inside {
        ctx,
        toy,
        last_t: None,
    }))
}

/// What a toy saves with its camera changed to `cam` (None: no camera in it).
pub fn with_camera(saved: &str, cam: Camera) -> Option<String> {
    let mut v: serde_json::Value = serde_json::from_str(saved).ok()?;
    let c = serde_json::to_value(cam).ok()?;
    match &mut v {
        serde_json::Value::Object(m) if m.contains_key("camera") => {
            m.insert("camera".into(), c);
        }
        serde_json::Value::Array(a)
            if a.last()
                .is_some_and(|l| serde_json::from_value::<Camera>(l.clone()).is_ok()) =>
        {
            *a.last_mut()? = c;
        }
        _ => return None,
    }
    serde_json::to_string(&v).ok()
}

/// A camera from what a toy saves: `{"camera": ..}`, or a list whose last item is one.
pub fn saved_camera(saved: &str) -> Option<Camera> {
    let v: serde_json::Value = serde_json::from_str(saved).ok()?;
    let c = match &v {
        serde_json::Value::Object(m) => m.get("camera")?.clone(),
        serde_json::Value::Array(a) => a.last()?.clone(),
        _ => return None,
    };
    serde_json::from_value(c).ok()
}

impl<T: Toy> Room for Inside<T> {
    fn seen_from_above(&self) -> Option<toy_kit::Above> {
        self.toy.seen_from_above(&self.ctx)
    }
    fn pointer(&mut self, e: PointerEvent) {
        self.toy.pointer(&mut self.ctx, e);
    }
    fn update(&mut self, t: f64) {
        let dt = match self.last_t {
            Some(l) => (t - l).clamp(0.0, 0.1),
            None => 0.0,
        };
        self.last_t = Some(t);
        self.toy.update(&mut self.ctx, t, dt);
    }
    fn draw(&mut self, target: &wgpu::TextureView) {
        self.toy.draw(&mut self.ctx, target);
    }
    fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.ctx.width, self.ctx.height, self.ctx.ppp) = (width.max(1), height.max(1), ppp);
        self.toy.resize(&mut self.ctx);
    }
    fn settings(&mut self, night: bool, reduce_motion: bool) {
        self.ctx.night = night;
        self.ctx.reduce_motion = reduce_motion;
        self.toy.settings(&mut self.ctx);
    }
    fn demo(&self) -> Option<Demo> {
        self.toy.demo(&self.ctx)
    }
    fn rules(&self) -> Option<Rules> {
        self.toy.rules()
    }
    fn origins(&self) -> Option<Rules> {
        self.toy.origins()
    }
    fn card_at_top(&self) -> bool {
        self.toy.card_at_top(&self.ctx)
    }
    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        self.toy.synth()
    }
    fn take_haptic(&mut self) -> bool {
        self.ctx.take_haptic()
    }
    fn shaken(&mut self) {
        self.toy.shaken(&mut self.ctx);
    }
    fn camera(&self) -> Option<Camera> {
        self.toy.save().as_deref().and_then(saved_camera)
    }
    fn set_camera(&mut self, cam: Camera) -> bool {
        let Some(s) = self.toy.save().and_then(|s| with_camera(&s, cam)) else {
            return false;
        };
        self.toy.restore(&mut self.ctx, &s);
        true
    }
    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        self.toy.project(lonlat)
    }
    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        self.toy.anchors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_camera_is_found_in_what_the_toys_save() {
        let c = Camera::new(12.0, 42.0, 1.55);
        let obj = serde_json::to_string(&serde_json::json!({"day": 3.0, "camera": c})).unwrap();
        assert_eq!(saved_camera(&obj), Some(c));
        // mold's (feeds, camera)
        let pair = serde_json::to_string(&(Vec::<u8>::new(), c)).unwrap();
        assert_eq!(saved_camera(&pair), Some(c));
        assert_eq!(saved_camera("{\"seed\": 4}"), None);
        assert_eq!(saved_camera("nonsense"), None);
        let d = Camera::new(-83.0, 35.6, 2.4);
        assert_eq!(saved_camera(&with_camera(&obj, d).unwrap()), Some(d));
        assert_eq!(saved_camera(&with_camera(&pair, d).unwrap()), Some(d));
        assert_eq!(with_camera("{\"seed\": 4}", d), None);
    }
}
