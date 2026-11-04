//! The GPU (copied from adrift-core/src/gpu, Nov 2, 2026; the toys' own from here): the device, the `View` uniform every drawing shader takes first, textures from
//! the data, and an offscreen target for the checks. (Engineering.)

pub mod textures;
pub mod view;

use std::sync::Arc;

pub use textures::Texture;
#[cfg(feature = "globe")]
pub use textures::Textures;
pub use view::{View, ViewUniform, VIEW_WGSL};

/// The device and its queue, shared by every pass.
pub struct Gpu {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    /// The colour format drawn to (the surface's, or the offscreen target's).
    pub format: wgpu::TextureFormat,
    pub adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    /// A device for a surface (or none: offscreen). Prefers the platform's own backend.
    pub async fn new(instance: &wgpu::Instance, surface: Option<&wgpu::Surface<'_>>) -> anyhow::Result<Gpu> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!("no adapter: {e}"))?;
        let adapter_info = adapter.get_info();
        log::info!("adapter: {adapter_info:?}");
        // What the kit asks of the device: WebGPU's default limits, the adapter's texture sizes (at
        // least 4096), no features. Under KIT_IPHONE_LIMITS=1 (the launch check) the same request is
        // made with an iPhone's limits in place of this adapter's, and it must fit what an iPhone
        // grants, so a buffer, binding or pipeline the phone's device would refuse fails here too.
        let request = |granted: &wgpu::Limits| {
            let mut l = wgpu::Limits::default().using_resolution(granted.clone());
            l.max_texture_dimension_2d = l.max_texture_dimension_2d.max(4096);
            l
        };
        let required_features = wgpu::Features::empty();
        let required_limits = if crate::budget::iphone() {
            let phone = crate::budget::iphone_limits();
            let l = request(&phone);
            let mut over = Vec::new();
            l.check_limits_with_fail_fn(&phone, false, |name, want, have| over.push(format!("{name} {want} > {have}")));
            anyhow::ensure!(over.is_empty(), "an iPhone would refuse the device: {}", over.join(", "));
            let lacking = required_features - crate::budget::iphone_features();
            anyhow::ensure!(lacking.is_empty(), "an iPhone lacks the features {lacking:?}");
            log::info!("the iPhone's limits (KIT_IPHONE_LIMITS)");
            l
        } else {
            request(&adapter.limits())
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("toy"),
                required_features,
                required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await?;
        if surface.is_some() {
            // the device: wgpu's default answer to an error it cannot return is a panic, which on the
            // phone is a crash; a toy is better drawn wrong than gone, so the error is logged instead
            // (the log says what to fix: toy-kit `log`), after what the device offers
            let l = adapter.limits();
            crate::log(&format!(
                "gpu: {} ({:?}); max buffer {} MB, storage binding {} MB, texture {}",
                adapter_info.name,
                adapter_info.backend,
                l.max_buffer_size >> 20,
                l.max_storage_buffer_binding_size >> 20,
                l.max_texture_dimension_2d
            ));
            device.on_uncaptured_error(Box::new(|e: wgpu::Error| crate::log(&format!("gpu: {e}"))));
            device.set_device_lost_callback(|reason, msg| crate::log(&format!("gpu: the device was lost ({reason:?}): {msg}")));
        } else {
            // the headless check: an error the device would only log is fatal here, so the check fails
            device.on_uncaptured_error(Box::new(|e: wgpu::Error| {
                crate::log(&format!("gpu: {e}"));
                panic!("a GPU validation error (logged, not fatal, on the device; fatal in the check)")
            }));
        }
        let device = Arc::new(device);
        crate::budget::start_report(device.clone());
        let format = match surface {
            Some(s) => {
                let caps = s.get_capabilities(&adapter);
                caps.formats
                    .iter()
                    .copied()
                    .find(|f| matches!(f, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm))
                    .unwrap_or(caps.formats[0])
            }
            None => wgpu::TextureFormat::Rgba8Unorm,
        };
        Ok(Gpu { device, queue: Arc::new(queue), format, adapter_info })
    }

    /// A texture, its bytes counted toward the toy's budget (`budget`).
    pub fn texture(&self, desc: &wgpu::TextureDescriptor) -> wgpu::Texture {
        crate::budget::count(crate::budget::texture_bytes(desc));
        self.device.create_texture(desc)
    }

    /// A buffer, its bytes counted toward the toy's budget (`budget`).
    pub fn buffer(&self, desc: &wgpu::BufferDescriptor) -> wgpu::Buffer {
        crate::budget::count(desc.size);
        self.device.create_buffer(desc)
    }

    pub fn shader(&self, label: &str, wgsl: &str) -> wgpu::ShaderModule {
        // TOY_DUMP_SHADERS=<dir>: every shader as the device will get it, for tools/toys/launch_check.sh
        // to translate to Metal (MSL) as the iPhone does
        if let Ok(dir) = std::env::var("TOY_DUMP_SHADERS") {
            let _ = std::fs::create_dir_all(&dir);
            let name: String = label.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();
            let _ = std::fs::write(std::path::Path::new(&dir).join(format!("{name}.wgsl")), wgsl);
        }
        self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        })
    }

    /// The usual blend: the pass's colour over what is there (premultiplied).
    pub fn over() -> wgpu::BlendState {
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                operation: wgpu::BlendOperation::Add,
            },
        }
    }

    /// A full-screen pipeline: three vertices, the fragment does everything (the shader's `vs_main`
    /// and `fs_main`).
    pub fn fullscreen_pipeline(
        &self,
        label: &str,
        shader: &wgpu::ShaderModule,
        layouts: &[&wgpu::BindGroupLayout],
        blend: Option<wgpu::BlendState>,
    ) -> wgpu::RenderPipeline {
        let layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: layouts,
            push_constant_ranges: &[],
        });
        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }
}

/// An offscreen colour target the checks draw to and read back.
pub struct Offscreen {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl Offscreen {
    pub fn new(gpu: &Gpu, width: u32, height: u32) -> Offscreen {
        let texture = gpu.texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Offscreen { texture, view, width, height }
    }

    /// The pixels, RGBA8, rows top down.
    pub fn read(&self, gpu: &Gpu) -> Vec<u8> {
        let bpr = (self.width * 4).div_ceil(256) * 256;
        // (the check's alone, and let go at once: not counted)
        let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (bpr * self.height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(bpr), rows_per_image: None },
            },
            wgpu::Extent3d { width: self.width, height: self.height, depth_or_array_layers: 1 },
        );
        gpu.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::PollType::wait()).expect("poll");
        let data = slice.get_mapped_range();
        let mut out = Vec::with_capacity((self.width * self.height * 4) as usize);
        for y in 0..self.height {
            let row = &data[(y * bpr) as usize..(y * bpr + self.width * 4) as usize];
            out.extend_from_slice(row);
        }
        if gpu.format == wgpu::TextureFormat::Bgra8Unorm {
            for px in out.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
        }
        out
    }
}
