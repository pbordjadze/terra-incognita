//! Textures from the data: decoded as they are (no colour management, which would bend the
//! distances), with mips made here, and the noise tile the sea's shader reads.

use super::Gpu;
use crate::assets::Assets;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

/// An image's pixels as RGBA8 (grey and RGB widened; a one-channel image stays one channel when
/// asked).
pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub data: Vec<u8>,
}

pub fn decode(bytes: &[u8], channels: u32) -> anyhow::Result<Pixels> {
    let img = image::load_from_memory(bytes)?;
    let (width, height) = (img.width(), img.height());
    let data = match channels {
        1 => img.to_luma8().into_raw(),
        _ => img.to_rgba8().into_raw(),
    };
    Ok(Pixels { width, height, channels: if channels == 1 { 1 } else { 4 }, data })
}

/// The next mip of an image: each texel the mean of four (the box filter the map's distances take
/// best; wrapping east-west is immaterial at a texel's scale).
fn half(p: &Pixels) -> Pixels {
    let (w, h, c) = ((p.width / 2).max(1), (p.height / 2).max(1), p.channels as usize);
    let mut data = vec![0u8; (w * h) as usize * c];
    for y in 0..h as usize {
        for x in 0..w as usize {
            for k in 0..c {
                let mut s = 0u32;
                for (dy, dx) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                    let sx = (2 * x + dx).min(p.width as usize - 1);
                    let sy = (2 * y + dy).min(p.height as usize - 1);
                    s += p.data[(sy * p.width as usize + sx) * c + k] as u32;
                }
                data[(y * w as usize + x) * c + k] = ((s + 2) / 4) as u8;
            }
        }
    }
    Pixels { width: w, height: h, channels: c as u32, data }
}

impl Texture {
    /// A 2D texture with a full mip chain.
    pub fn from_pixels(gpu: &Gpu, label: &str, p: &Pixels, srgb: bool) -> Texture {
        let format = match (p.channels, srgb) {
            (1, _) => wgpu::TextureFormat::R8Unorm,
            (_, true) => wgpu::TextureFormat::Rgba8UnormSrgb,
            _ => wgpu::TextureFormat::Rgba8Unorm,
        };
        let mips = 32 - p.width.max(p.height).leading_zeros();
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: p.width, height: p.height, depth_or_array_layers: 1 },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut level = Pixels { width: p.width, height: p.height, channels: p.channels, data: p.data.clone() };
        for m in 0..mips {
            if m > 0 {
                level = half(&level);
            }
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: m, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
                &level.data,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(level.width * level.channels), rows_per_image: None },
                wgpu::Extent3d { width: level.width, height: level.height, depth_or_array_layers: 1 },
            );
        }
        let view = texture.create_view(&Default::default());
        Texture { texture, view, width: p.width, height: p.height }
    }

    /// A texture without mips (a data sheet read texel by texel).
    pub fn flat(gpu: &Gpu, label: &str, p: &Pixels) -> Texture {
        let format = if p.channels == 1 { wgpu::TextureFormat::R8Unorm } else { wgpu::TextureFormat::Rgba8Unorm };
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: p.width, height: p.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &p.data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(p.width * p.channels), rows_per_image: None },
            wgpu::Extent3d { width: p.width, height: p.height, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&Default::default());
        Texture { texture, view, width: p.width, height: p.height }
    }

    /// Twelve months of one-channel sheets as the layers of one texture (sampled between months).
    pub fn months(gpu: &Gpu, label: &str, sheets: &[Pixels]) -> Texture {
        let (w, h) = (sheets[0].width, sheets[0].height);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: sheets.len() as u32 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        for (i, s) in sheets.iter().enumerate() {
            gpu.queue.write_texture(
                wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d { x: 0, y: 0, z: i as u32 }, aspect: wgpu::TextureAspect::All },
                &s.data,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(w), rows_per_image: Some(h) },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
        }
        let view = texture.create_view(&wgpu::TextureViewDescriptor { dimension: Some(wgpu::TextureViewDimension::D2Array), ..Default::default() });
        Texture { texture, view, width: w, height: h }
    }

    /// Terra's noise tile: gradient noise, 16 cells a side wrapping round, six samples a cell, in a
    /// 3D texture the shaders sample trilinearly (`gnoise`: a lookup an octave).
    pub fn noise_tile(gpu: &Gpu) -> Texture {
        const N: usize = 96;
        fn pcg3d(mut v: [u32; 3]) -> [u32; 3] {
            for x in v.iter_mut() {
                *x = x.wrapping_mul(1664525).wrapping_add(1013904223);
            }
            v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
            v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
            v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
            for x in v.iter_mut() {
                *x ^= *x >> 16;
            }
            v[0] = v[0].wrapping_add(v[1].wrapping_mul(v[2]));
            v[1] = v[1].wrapping_add(v[2].wrapping_mul(v[0]));
            v[2] = v[2].wrapping_add(v[0].wrapping_mul(v[1]));
            v
        }
        fn hash3(c: [i32; 3]) -> [f32; 3] {
            let h = pcg3d([c[0] as u32, c[1] as u32, c[2] as u32]);
            [h[0] as f32 / 4294967295.0, h[1] as f32 / 4294967295.0, h[2] as f32 / 4294967295.0]
        }
        let mut data = vec![0u8; N * N * N];
        for z in 0..N {
            for y in 0..N {
                for x in 0..N {
                    let p = [(x as f32 + 0.5) / 6.0, (y as f32 + 0.5) / 6.0, (z as f32 + 0.5) / 6.0];
                    let i = [p[0].floor(), p[1].floor(), p[2].floor()];
                    let f = [p[0] - i[0], p[1] - i[1], p[2] - i[2]];
                    let u: Vec<f32> = f.iter().map(|&t| t * t * t * (t * (t * 6.0 - 15.0) + 10.0)).collect();
                    let mut v = 0.0f32;
                    for c in 0..8 {
                        let d = [c & 1, (c >> 1) & 1, c >> 2];
                        let cell = [
                            ((i[0] as i32 + d[0] as i32) % 16 + 16) % 16,
                            ((i[1] as i32 + d[1] as i32) % 16 + 16) % 16,
                            ((i[2] as i32 + d[2] as i32) % 16 + 16) % 16,
                        ];
                        let g = hash3(cell).map(|h| h * 2.0 - 1.0);
                        let w = (if d[0] == 1 { u[0] } else { 1.0 - u[0] })
                            * (if d[1] == 1 { u[1] } else { 1.0 - u[1] })
                            * (if d[2] == 1 { u[2] } else { 1.0 - u[2] });
                        v += w * (g[0] * (f[0] - d[0] as f32) + g[1] * (f[1] - d[1] as f32) + g[2] * (f[2] - d[2] as f32));
                    }
                    data[(z * N + y) * N + x] = ((v * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                }
            }
        }
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("noise tile"),
            size: wgpu::Extent3d { width: N as u32, height: N as u32, depth_or_array_layers: N as u32 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(N as u32), rows_per_image: Some(N as u32) },
            wgpu::Extent3d { width: N as u32, height: N as u32, depth_or_array_layers: N as u32 },
        );
        let view = texture.create_view(&Default::default());
        Texture { texture, view, width: N as u32, height: N as u32 }
    }
}

/// Every texture the passes read, and the samplers.
pub struct Textures {
    pub geo: Texture,
    pub terrain: Texture,
    pub floor: Texture,
    pub currents: Texture,
    pub stream: Texture,
    /// Twelve layers: each month's sea ice.
    pub ice: Texture,
    /// The month's land (and the next's), swapped as the date moves.
    pub land_a: Texture,
    pub land_b: Texture,
    pub land_months: (u32, u32),
    pub noise: Texture,
    pub linear_wrap_x: wgpu::Sampler,
    pub linear_clamp: wgpu::Sampler,
    pub nearest_clamp: wgpu::Sampler,
    pub linear_repeat: wgpu::Sampler,
}

impl Textures {
    pub fn load(gpu: &Gpu, assets: &dyn Assets, month: u32) -> anyhow::Result<Textures> {
        let geo = Texture::from_pixels(gpu, "geo", &decode(&assets.read("geo.png")?, 4)?, false);
        let terrain = Texture::from_pixels(gpu, "terrain", &decode(&assets.read("terrain.png")?, 4)?, false);
        let floor = Texture::from_pixels(gpu, "floor", &decode(&assets.read("floor.png")?, 1)?, false);
        let currents = Texture::flat(gpu, "currents", &decode(&assets.read("currents.png")?, 4)?);
        let stream = Texture::flat(gpu, "stream", &decode(&assets.read("stream.png")?, 4)?);
        let mut ice_sheets = Vec::with_capacity(12);
        for m in 1..=12 {
            ice_sheets.push(decode(&assets.read(&format!("ice-{m:02}.png"))?, 1)?);
        }
        let ice = Texture::months(gpu, "ice", &ice_sheets);
        let (ma, mb) = (month % 12, (month + 1) % 12);
        let land_a = Self::land(gpu, assets, ma)?;
        let land_b = Self::land(gpu, assets, mb)?;
        let noise = Texture::noise_tile(gpu);
        let sampler = |label, u, v, filter| {
            gpu.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: u,
                address_mode_v: v,
                address_mode_w: u,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            })
        };
        use wgpu::AddressMode::{ClampToEdge, Repeat};
        use wgpu::FilterMode::{Linear, Nearest};
        Ok(Textures {
            geo,
            terrain,
            floor,
            currents,
            stream,
            ice,
            land_a,
            land_b,
            land_months: (ma, mb),
            noise,
            linear_wrap_x: sampler("linear wrap x", Repeat, ClampToEdge, Linear),
            linear_clamp: sampler("linear clamp", ClampToEdge, ClampToEdge, Linear),
            nearest_clamp: sampler("nearest clamp", ClampToEdge, ClampToEdge, Nearest),
            linear_repeat: sampler("linear repeat", Repeat, Repeat, Linear),
        })
    }

    fn land(gpu: &Gpu, assets: &dyn Assets, m: u32) -> anyhow::Result<Texture> {
        Ok(Texture::from_pixels(gpu, &format!("land {}", m + 1), &decode(&assets.read(&format!("land-{:02}.jpg", m + 1))?, 4)?, false))
    }

    /// Keeps the two land textures on the date's months; says whether they changed (the bind groups
    /// must be remade then).
    pub fn set_months(&mut self, gpu: &Gpu, assets: &dyn Assets, ma: u32, mb: u32) -> anyhow::Result<bool> {
        if self.land_months == (ma, mb) {
            return Ok(false);
        }
        if self.land_months.1 == ma {
            std::mem::swap(&mut self.land_a, &mut self.land_b);
            self.land_b = Self::land(gpu, assets, mb)?;
        } else {
            self.land_a = Self::land(gpu, assets, ma)?;
            self.land_b = Self::land(gpu, assets, mb)?;
        }
        self.land_months = (ma, mb);
        Ok(true)
    }
}
