//! The runtime's GPU: the device, the `View` uniform every drawing shader takes first, textures from
//! the data, and an offscreen target for the checks. (Engineering.)

pub mod stage;
pub mod textures;
pub mod view;

use std::sync::Arc;

pub use stage::{Blit, Budget, Stage};
pub use textures::{Texture, Textures};
pub use view::{View, ViewUniform};

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
    pub async fn new(
        instance: &wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> anyhow::Result<Gpu> {
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|e| anyhow::anyhow!("no adapter: {e}"))?;
        let adapter_info = adapter.get_info();
        log::info!("adapter: {adapter_info:?}");
        let limits = wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        let mut required_limits = wgpu::Limits::default().using_resolution(adapter.limits());
        required_limits.max_texture_dimension_2d =
            required_limits.max_texture_dimension_2d.max(4096);
        let _ = limits;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("adrift"),
                required_features: wgpu::Features::empty(),
                required_limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            })
            .await?;
        let format = match surface {
            Some(s) => {
                let caps = s.get_capabilities(&adapter);
                caps.formats
                    .iter()
                    .copied()
                    .find(|f| {
                        matches!(
                            f,
                            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Rgba8Unorm
                        )
                    })
                    .unwrap_or(caps.formats[0])
            }
            None => wgpu::TextureFormat::Rgba8Unorm,
        };
        Ok(Gpu {
            device: Arc::new(device),
            queue: Arc::new(queue),
            format,
            adapter_info,
        })
    }

    pub fn shader(&self, label: &str, wgsl: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
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
        let layouts: Vec<Option<&wgpu::BindGroupLayout>> =
            layouts.iter().copied().map(Some).collect();
        let layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &layouts,
                immediate_size: 0,
            });
        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
                multiview_mask: None,
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
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        Offscreen {
            texture,
            view,
            width,
            height,
        }
    }

    /// The pixels, RGBA8, rows top down.
    pub fn read(&self, gpu: &Gpu) -> Vec<u8> {
        let bpr = (self.width * 4 + 255) / 256 * 256;
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
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bpr),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("mapped");
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

/// A ring of readback buffers for an export (R24): each frame's pixels are copied into the next
/// buffer and mapped while the GPU draws the frame after it, and taken from the oldest straight into
/// the caller's rows (the video encoder's buffer), swizzled only if the formats differ. Nothing is
/// allocated per frame and only that frame's copy is waited for.
pub struct Readback {
    bufs: Vec<wgpu::Buffer>,
    bpr: u32,
    width: u32,
    height: u32,
    /// Submitted copies, oldest first: (buffer, submission).
    pending: std::collections::VecDeque<(usize, wgpu::SubmissionIndex)>,
    next: usize,
}

impl Readback {
    pub fn new(gpu: &Gpu, target: &Offscreen, depth: usize) -> Readback {
        let bpr = (target.width * 4 + 255) / 256 * 256;
        let bufs = (0..depth.max(1))
            .map(|_| {
                gpu.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("export readback"),
                    size: (bpr * target.height) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Readback {
            bufs,
            bpr,
            width: target.width,
            height: target.height,
            pending: Default::default(),
            next: 0,
        }
    }

    /// Copies were submitted and not yet taken.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Room for another frame in flight.
    pub fn has_room(&self) -> bool {
        self.pending.len() < self.bufs.len()
    }

    /// Copies the target's current pixels into the next buffer and asks for it mapped.
    pub fn submit(&mut self, gpu: &Gpu, target: &Offscreen) {
        let i = self.next;
        self.next = (self.next + 1) % self.bufs.len();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &target.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.bufs[i],
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bpr),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        let idx = gpu.queue.submit([enc.finish()]);
        self.bufs[i]
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        self.pending.push_back((i, idx));
    }

    /// The oldest frame's pixels written to `out` (rows `stride` bytes apart), as BGRA when `bgra`
    /// else RGBA; false if none is pending.
    pub fn take(&mut self, gpu: &Gpu, out: &mut [u8], stride: usize, bgra: bool) -> bool {
        let Some((i, idx)) = self.pending.pop_front() else {
            return false;
        };
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(idx),
                timeout: None,
            })
            .expect("poll");
        let row = (self.width * 4) as usize;
        {
            let data = self.bufs[i].slice(..).get_mapped_range().expect("mapped");
            let swap = bgra
                != (gpu.format == wgpu::TextureFormat::Bgra8Unorm
                    || gpu.format == wgpu::TextureFormat::Bgra8UnormSrgb);
            for y in 0..self.height as usize {
                let src = &data[y * self.bpr as usize..y * self.bpr as usize + row];
                let dst = &mut out[y * stride..y * stride + row];
                dst.copy_from_slice(src);
                if swap {
                    for px in dst.chunks_exact_mut(4) {
                        px.swap(0, 2);
                    }
                }
            }
        }
        self.bufs[i].unmap();
        true
    }
}
