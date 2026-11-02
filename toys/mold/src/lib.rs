//! Mold (a working title): a glowing slime mold seeking the night's cities and finding their routes
//!
//! The starter the seam left: the real globe by night, turned by a finger. The toy's session replaces
//! all of it (toys/README.md says how a toy is built, checked and shipped).

use toy_kit::{Ctx, Globe, PointerEvent, Toy};

pub struct Mold {
    globe: Globe,
}

impl Toy for Mold {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(&ctx.gpu, ctx.assets.as_ref(), ctx.width, ctx.height, ctx.ppp, 10)?;
        globe.night = 1.0;
        Ok(Mold { globe })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        self.globe.pointer(e);
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mold"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.globe.draw(&mut rp);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

toy_kit::ios_shell!(Mold);
