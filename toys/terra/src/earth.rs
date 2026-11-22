//! The Earth mark: Terra's way back from a toy, bottom left as the toy's own mark is bottom right (toy-kit
//! info.rs). Where the frame draws its own back button (`Ctx::native_chrome`), it is the frame's and this is
//! neither drawn nor touched.

use toy_kit::{Ctx, Phase, PointerEvent};

use crate::{earth_spot, State, Terra, EARTH_REACH};

impl Terra {
    /// The Earth mark (a small globe: its rim, a meridian, the equator), on the marks' dark ground.
    pub(crate) fn draw_earth_mark(&mut self, ctx: &Ctx, target: &wgpu::TextureView, alpha: f32) {
        let k = ctx.ppp as f32;
        let s = earth_spot(ctx.points());
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        let k = k * toy_kit::info::Info::mark_scale(ctx.points()) as f32;
        let lit = 0.62 * alpha;
        let p = &mut self.painter;
        p.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42 * alpha]);
        p.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        p.line(
            [c[0] - 6.9 * k, c[1]],
            [c[0] + 6.9 * k, c[1]],
            0.5 * k,
            0.7,
            [1.0, 1.0, 1.0, lit],
        );
        // the meridian: an ellipse's two halves, drawn as short strokes
        let n = 14;
        let mut prev: Option<[f32; 2]> = None;
        for i in 0..=n {
            let a = std::f32::consts::PI * (i as f32 / n as f32 - 0.5);
            let q = [c[0] + a.cos() * 3.4 * k, c[1] + a.sin() * 7.2 * k];
            if let Some(pp) = prev {
                p.line(pp, q, 0.5 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            }
            prev = Some(q);
        }
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.painter
            .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    /// Inside: the Earth mark's finger (true if it was the mark's); lifted on it, the toy goes back into its card.
    pub(crate) fn earth_mark_pointer(&mut self, ctx: &mut Ctx, e: &PointerEvent) -> bool {
        let s = earth_spot(ctx.points());
        let m = toy_kit::info::Info::mark_scale(ctx.points());
        let near = |k: f64| (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH * m * k;
        if e.phase == Phase::Down && self.earth_finger.is_none() && near(1.0) {
            self.earth_finger = Some(e.id);
            return true;
        }
        if self.earth_finger != Some(e.id) {
            return false;
        }
        if matches!(e.phase, Phase::Up | Phase::Cancel) {
            self.earth_finger = None;
            if e.phase == Phase::Up && near(1.5) && matches!(self.state, State::Inside) {
                self.leave(ctx, e.t);
            }
        }
        true
    }
}
