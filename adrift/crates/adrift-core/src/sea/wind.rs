//! The second medium: things on the wind (the wind preset's truth, round 10). A light thing (a
//! pollen grain, a down seed) lifted into the air near the ground goes with the month's mean 10 m
//! wind plus a gust: the weather about that mean, a velocity that decays over the gusts' measured
//! time scale and is kicked so its variance stays the measured variance there (an
//! Ornstein-Uhlenbeck velocity, the sea's `flight` with the air's numbers). It does not stop at
//! the coast. It comes down by itself (settling out of the air at a steady rate) or is washed out
//! by rain (at the month's mean rain there), on land or on the sea.
//!
//! The weather is a world's, as the sea's eddies are (R6): world k is one random gust field on a
//! lattice a weather system wide, a fresh draw each quarter day, shared by every thing's k-th path,
//! so two seeds lifted near each other in one world fly together through the same weather until
//! a front parts them; rain in world k falls on every thing there alike. Only the dry fall is the
//! thing's own dice (its `seed`).
//!
//! Deterministic: the same (air, day, dt, seed, k) always gives the same answer, here and in
//! `wind.wgsl` (to the rounding of a multiply). The numbers' sources: `adrift/SOURCES.md` › The
//! wind; what differs from the sea, for the studio's dials: `sea/MEDIA.md`.

use super::rules::{bytes_gaussian, hash_seed, mix, uniform, wrap, EARTH_RADIUS, SECONDS_A_DAY};
use super::sheets::{months, Coast, Sheet, Tiled, Wind};
use crate::assets::Assets;
use crate::model::LonLat;

/// The gusts sheet (`gusts.png`, tools/adrift/data/weather.py): R, G the wind's standard deviation
/// about its monthly mean (255 sqrt(sigma / 12 m/s)), B the gusts' time scale (255 T / 5 days),
/// A the month's mean rain (255 sqrt(P / 25 mm/day)).
pub struct Gusts(pub Tiled);

/// What the gusts sheet says at a place on a day.
#[derive(Clone, Copy, Debug)]
pub struct Weather {
    /// The standard deviation of the eastward and northward wind about the month's mean, m/s.
    pub sigma: (f32, f32),
    /// The gusts' integral time scale, days.
    pub time: f32,
    /// The month's mean rain, mm/day.
    pub rain: f32,
}

impl Gusts {
    pub const SMAX: f32 = 12.0;
    pub const TMAX: f32 = 5.0;
    pub const PMAX: f32 = 25.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<Gusts> {
        Ok(Gusts(Tiled::new(Sheet::load(assets, "gusts.png", false)?, (360, 180), (4, 3))?))
    }

    /// The day's mix of months, each decoded then mixed.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> Weather {
        let (a, b, t) = months(day);
        let pa = self.0.sample_all(a, lon, lat);
        let pb = self.0.sample_all(b, lon, lat);
        let sq = |x: f32, mx: f32| {
            let s = x / 255.0;
            s * s * mx
        };
        let m = |c: usize, f: &dyn Fn(f32) -> f32| f(pa[c]) * (1.0 - t) + f(pb[c]) * t;
        Weather {
            sigma: (m(0, &|x| sq(x, Self::SMAX)), m(1, &|x| sq(x, Self::SMAX))),
            time: m(2, &|x| x / 255.0 * Self::TMAX),
            rain: m(3, &|x| sq(x, Self::PMAX)),
        }
    }
}

/// Where a thing on the wind is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aloft {
    /// In the air.
    Flying,
    /// Come down on land (fallen out of the air, or rained out).
    Land,
    /// Come down on the sea (where the sea's rules could take it as a float).
    Sea,
}

impl Aloft {
    /// The GPU's code (`Path.held` in wind.wgsl).
    pub fn code(self) -> u32 {
        match self {
            Aloft::Flying => 0,
            Aloft::Land => 1,
            Aloft::Sea => 2,
        }
    }

    pub fn from_code(c: u32) -> Aloft {
        match c {
            1 => Aloft::Land,
            2 => Aloft::Sea,
            _ => Aloft::Flying,
        }
    }
}

/// A thing on the wind with the memory of its gust: where it is, the gust it carries (m/s east,
/// north), and whether it is still up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Air {
    pub at: LonLat,
    pub gust: (f32, f32),
    pub aloft: Aloft,
}

/// The weather's lattice (degrees east, north): a world's gust field has a node every
/// `AIR_CELL` and is mixed bilinearly between, so its correlation falls to 1/e at roughly 0.6 of
/// a cell: about 1000 km east and 530 km north at the equator, 700 and 530 at 45 degrees, as the
/// reanalysis measures (600-1100 km east, 500-800 north: `adrift/data/weather.json` › scale).
pub const AIR_CELL: (f32, f32) = (15.0, 8.0);
/// The rain's lattice (degrees): a shower, or a front's rain band, about 250 km across (drawn:
/// the reanalysis gives the month's mean rain, not its showers' size).
pub const RAIN_CELL: f32 = 2.5;
/// The quarter days a step may span (a longer one reads only the first eight).
pub const AIR_SPAN: u32 = 8;
/// The streams of the air's worlds, apart from the sea's and each other's.
pub const GUST_STREAM: u32 = 0x41_1700;
pub const RISE_STREAM: u32 = 0x41_2900;
pub const RAIN_STREAM: u32 = 0x41_3b00;

pub struct AirRules {
    pub wind: Wind,
    pub gusts: Gusts,
    pub coast: Coast,
    /// The fraction of the 10 m wind a thing rides: 1 (a thing lifted near the ground; one carried
    /// a kilometre up would ride a faster wind than the sheet's).
    pub carry: f32,
    /// The gusts' scale: 1 is the measured weather; 0 switches it off (the mean wind alone).
    pub gust: f32,
    /// The chance a day of coming down by itself, dry (per day): a birch pollen grain settles
    /// at about 2 cm/s through a mixed layer a kilometre deep, by day; at night the air above the
    /// shallow night layer is cut off from the ground: 0.02 / 1000 * 86400 / 2, about 0.86.
    pub fall: f32,
    /// How fast rain washes a thing out, per day per mm/day of the month's mean rain: a
    /// below-cloud scavenging coefficient of 1e-4 /s in rain of 1 mm/h (coarse particles), linear in
    /// the rate, averaged over the month's wet and dry hours: 1e-4 / 24 * 86400, 0.36.
    pub washout: f32,
}

impl AirRules {
    pub const FALL: f32 = 0.86;
    pub const WASHOUT: f32 = 0.36;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<AirRules> {
        Ok(AirRules {
            wind: Wind::load(assets)?,
            gusts: Gusts::load(assets)?,
            coast: Coast::load(assets)?,
            carry: 1.0,
            gust: 1.0,
            fall: Self::FALL,
            washout: Self::WASHOUT,
        })
    }

    /// A thing as it is lifted at `at` on `day`: with the gust world k's air has there at that
    /// moment (a Gaussian of the measured variance), so the cloud is right from the first hour.
    pub fn rise(&self, at: LonLat, day: f64, k: u32) -> Air {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let w = self.gusts.at(day as f32, lon, lat);
        let (gu, gv) = air_field(k ^ RISE_STREAM, quarter(day as f32), lon, lat);
        Air { at, gust: (gu * w.sigma.0 * self.gust, gv * w.sigma.1 * self.gust), aloft: Aloft::Flying }
    }

    /// One step of `dt_days`: the gust decays over the time scale and is kicked by world k's
    /// weather; the thing moves with the carried mean wind and its gust; then it may come down
    /// (rain first, world k's; then its own dry fall, by `seed`). A thing that is down stays.
    pub fn step(&self, a: Air, day: f64, dt_days: f64, seed: u64, k: u32) -> Air {
        if a.aloft != Aloft::Flying {
            return a;
        }
        let (lon, lat) = (a.at.lon as f32, a.at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let (wu, wv) = self.wind.at(day, lon, lat);
        let w = self.gusts.at(day, lon, lat);
        let secs = dt * SECONDS_A_DAY;
        let t = w.time.max(0.25) * SECONDS_A_DAY;
        let al = (-secs / t).exp();
        let kick = (1.0 - al * al).sqrt() * self.gust;
        let (gx, gy) = air_kick(k ^ GUST_STREAM, day, dt, lon, lat);
        let gu = a.gust.0 * al + gx * w.sigma.0 * kick;
        let gv = a.gust.1 * al + gy * w.sigma.1 * kick;
        let dx = (self.carry * wu + gu) * secs;
        let dy = (self.carry * wv + gv) * secs;
        let coslat = (lat.to_radians()).cos().max(0.01);
        let nlon = wrap(lon + (dx / (EARTH_RADIUS * coslat)).to_degrees());
        let nlat = (lat + (dy / EARTH_RADIUS).to_degrees()).clamp(-89.5, 89.5);
        let at = LonLat { lon: nlon as f64, lat: nlat as f64 };
        let down = air_rain(k, day, dt, lon, lat, self.washout * w.rain)
            || uniform(hash_seed(seed, k, day.to_bits()), 0) < 1.0 - (-self.fall * dt).exp();
        let aloft = if !down {
            Aloft::Flying
        } else if self.coast.distance(nlon, nlat) > 0.0 {
            Aloft::Land
        } else {
            Aloft::Sea
        };
        Air { at, gust: if down { (0.0, 0.0) } else { (gu, gv) }, aloft }
    }

    /// The velocity a thing has between steps (m/s east, north), for the look's glide.
    pub fn velocity(&self, a: Air, day: f64) -> (f32, f32) {
        if a.aloft != Aloft::Flying {
            return (0.0, 0.0);
        }
        let (wu, wv) = self.wind.at(day as f32, a.at.lon as f32, a.at.lat as f32);
        (self.carry * wu + a.gust.0, self.carry * wv + a.gust.1)
    }

    /// A thing's maybes (k = 1..=n), each lifted afresh at `from` on `day0` and stepped `dt_days`
    /// at a time until `until`: where each is (in the air or come down).
    pub fn maybes(&self, from: LonLat, day0: f64, until: f64, dt_days: f64, seed: u64, n: u32) -> Vec<Air> {
        (1..=n).map(|k| self.path(from, day0, until, dt_days, seed, k).pop().unwrap()).collect()
    }

    /// Path k from `from`: every step's place, the lift first.
    pub fn path(&self, from: LonLat, day0: f64, until: f64, dt_days: f64, seed: u64, k: u32) -> Vec<Air> {
        let mut a = self.rise(from, day0, k);
        let mut out = vec![a];
        let mut day = day0;
        while day < until - 1e-9 {
            let dt = dt_days.min(until - day);
            a = self.step(a, day, dt, seed, k);
            out.push(a);
            day += dt;
        }
        out
    }
}

/// The quarter day `day` falls in.
#[inline]
pub fn quarter(day: f32) -> i32 {
    (day * 4.0).floor() as i32
}

/// World k's two unit draws at (lon, lat) in quarter day `cell`, on the weather's lattice: the
/// sea's `world_field` with the air's cell.
#[inline]
pub fn air_field(k: u32, cell: i32, lon: f32, lat: f32) -> (f32, f32) {
    let n = (360.0 / AIR_CELL.0) as i32;
    let fx = (lon + 180.0) / AIR_CELL.0;
    let fy = (lat + 90.0) / AIR_CELL.1;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let base = mix(mix(k ^ 0xa1e5_0b0e) ^ cell as u32);
    let (x0, y0) = (x0 as i32, y0 as i32);
    let (mut gu, mut gv) = (0.0f32, 0.0f32);
    for (dx, dy, w) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
        let ix = (x0 + dx).rem_euclid(n) as u32;
        let h = mix(mix(base ^ ix) ^ (y0 + dy) as u32);
        gu += w * bytes_gaussian(h);
        gv += w * bytes_gaussian(mix(h ^ 0x68e3_1da4));
    }
    let a = (1.0 - tx) * (1.0 - tx) + tx * tx;
    let b = (1.0 - ty) * (1.0 - ty) + ty * ty;
    let norm = 1.0 / (a * b).sqrt();
    (gu * norm, gv * norm)
}

/// The quarter days whose middles a step spans (the one it starts in, if it spans none).
#[inline]
pub fn air_span(day: f32, dt: f32) -> (i32, u32) {
    let first = ((day - 0.125) * 4.0).ceil() as i32;
    let end = ((day + dt - 0.125) * 4.0).ceil() as i32;
    if end > first {
        (first, ((end - first) as u32).min(AIR_SPAN))
    } else {
        (quarter(day), 1)
    }
}

/// World k's gust kick over a step: the spanned quarter days' draws, summed and renormalised.
#[inline]
pub fn air_kick(k: u32, day: f32, dt: f32, lon: f32, lat: f32) -> (f32, f32) {
    let (first, count) = air_span(day, dt);
    let (mut gu, mut gv) = (0.0f32, 0.0f32);
    for i in 0..count {
        let (u, v) = air_field(k, first + i as i32, lon, lat);
        gu += u;
        gv += v;
    }
    let norm = 1.0 / (count as f32).sqrt();
    (gu * norm, gv * norm)
}

/// Whether world k's rain washes a thing out at (lon, lat) over a step, at `rate` per day: each
/// spanned quarter day tests the uniform of the nearest node of a lattice `RAIN_CELL` wide (one
/// shower over every thing in a rain band's width) against its share of the chance.
#[inline]
pub fn air_rain(k: u32, day: f32, dt: f32, lon: f32, lat: f32, rate: f32) -> bool {
    let (first, count) = air_span(day, dt);
    let p = 1.0 - (-rate * dt / count as f32).exp();
    let n = (360.0 / RAIN_CELL) as i32;
    let ix = (((lon + 180.0) / RAIN_CELL + 0.5).floor() as i32).rem_euclid(n) as u32;
    let iy = ((lat + 90.0) / RAIN_CELL + 0.5).floor() as i32 as u32;
    for i in 0..count {
        let h = mix(mix(mix(mix((k ^ RAIN_STREAM) ^ 0xa1e5_0b0e) ^ (first + i as i32) as u32) ^ ix) ^ iy);
        if ((h >> 8) as f32 / 16_777_216.0) < p {
            return true;
        }
    }
    false
}

/// The step's parameters on the GPU (`Params` in wind.wgsl): 48 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct AirParams {
    pub day: f32,
    pub dt: f32,
    pub carry: f32,
    pub gust: f32,
    pub fall: f32,
    pub washout: f32,
    pub seed_lo: u32,
    pub seed_hi: u32,
    pub count: u32,
    pub base: u32,
    pub _pad: [u32; 2],
}

impl AirParams {
    pub fn new(rules: &AirRules, day: f64, dt_days: f64, seed: u64, count: u32) -> AirParams {
        AirParams {
            day: day as f32,
            dt: dt_days as f32,
            carry: rules.carry,
            gust: rules.gust,
            fall: rules.fall,
            washout: rules.washout,
            seed_lo: seed as u32,
            seed_hi: (seed >> 32) as u32,
            count,
            base: 0,
            _pad: [0; 2],
        }
    }
}

/// A thing as the GPU holds it: the sea's `GpuPath` layout (24 bytes), `held` the `Aloft` code,
/// `measured` the month's mean rain where it last was, `eddy_u`, `eddy_v` its gust.
pub fn gpu_path(a: &Air) -> super::GpuPath {
    super::GpuPath { lon: a.at.lon as f32, lat: a.at.lat as f32, held: a.aloft.code(), measured: 0.0, eddy_u: a.gust.0, eddy_v: a.gust.1 }
}

/// The wind's maybes on the GPU: `wind.wgsl` as a compute pass over a buffer of paths, one step a
/// dispatch (the sea's `Maybes`, plainer: the preset seam can grow it to `step_many` and keeps).
pub struct AirMaybes {
    pipeline: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    params: wgpu::Buffer,
    paths: wgpu::Buffer,
    readback: wgpu::Buffer,
    pub capacity: u32,
    _wind: crate::gpu::textures::Texture,
    _gusts: crate::gpu::textures::Texture,
}

impl AirMaybes {
    pub fn new(gpu: &crate::gpu::Gpu, assets: &dyn Assets, textures: &crate::gpu::Textures, capacity: u32) -> anyhow::Result<AirMaybes> {
        use crate::gpu::textures::{decode, Texture};
        let wind = Texture::flat(gpu, "wind", &decode(&assets.read("wind.png")?, 4)?);
        let gusts = Texture::flat(gpu, "gusts", &decode(&assets.read("gusts.png")?, 4)?);
        let tex = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("air maybes"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                tex(1),
                tex(2),
                tex(3),
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let params = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("air params"),
            size: std::mem::size_of::<AirParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let size = capacity as u64 * std::mem::size_of::<super::GpuPath>() as u64;
        let paths = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("air paths"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("air readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("air maybes"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&wind.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&gusts.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&textures.geo.view) },
                wgpu::BindGroupEntry { binding: 4, resource: paths.as_entire_binding() },
            ],
        });
        let shader = gpu.shader("wind", super::WIND_WGSL);
        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("air maybes"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let pipeline = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("air maybes"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("maybes"),
            compilation_options: Default::default(),
            cache: None,
        });
        Ok(AirMaybes { pipeline, bind, params, paths, readback, capacity, _wind: wind, _gusts: gusts })
    }

    pub fn seed(&self, gpu: &crate::gpu::Gpu, paths: &[super::GpuPath]) {
        assert!(paths.len() as u32 <= self.capacity);
        gpu.queue.write_buffer(&self.paths, 0, bytemuck::cast_slice(paths));
    }

    /// One step of every path (k = its index + 1).
    pub fn step(&self, gpu: &crate::gpu::Gpu, params: AirParams) {
        gpu.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("air maybes") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("air maybes"), timestamp_writes: None });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.dispatch_workgroups((params.count + 63) / 64, 1, 1);
        }
        gpu.queue.submit([enc.finish()]);
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.paths
    }

    pub fn read(&self, gpu: &crate::gpu::Gpu, count: u32) -> Vec<super::GpuPath> {
        let size = count as u64 * std::mem::size_of::<super::GpuPath>() as u64;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        enc.copy_buffer_to_buffer(&self.paths, 0, &self.readback, 0, size);
        gpu.queue.submit([enc.finish()]);
        let slice = self.readback.slice(..size);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::PollType::wait()).expect("poll");
        let out = bytemuck::cast_slice::<u8, super::GpuPath>(&slice.get_mapped_range()).to_vec();
        self.readback.unmap();
        out
    }
}
