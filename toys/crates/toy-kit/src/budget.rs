//! Launch safety: what an iPhone's GPU allows, and how much memory a toy takes, measured on Linux so a
//! toy the phone would refuse or kill fails the check instead (`tools/kit/launch_check.py`).
//!
//! - `KIT_IPHONE_LIMITS=1`: [`crate::Gpu::new`] requests exactly what it would request on an iPhone
//!   (the limits worked out from [`iphone_limits`], not the Linux adapter's), and refuses features the
//!   phone lacks, so a buffer, texture, binding or pipeline the phone's device would refuse fails here.
//! - `KIT_MEMORY_REPORT=<file>`: a thread samples the GPU's live bytes (wgpu's own counters: every
//!   buffer and texture, the kit's, the toy's own and wgpu's staging) and the bytes made through the
//!   kit's helpers ([`count`]), and keeps the peaks in `<file>` as JSON. The process's peak resident
//!   set is read by the caller (the child's `ru_maxrss`, the same as `VmHWM`).
//!
//! Why peak RSS is a fair proxy: on Linux the check runs on Mesa's lavapipe, whose "GPU" memory is
//! the process's own heap, so every buffer and texture is in the resident set; on an iPhone memory is
//! unified and the GPU's allocations count toward the app's footprint, which is what the system's
//! memory limit (jetsam) kills on. Neither is exact: lavapipe adds LLVM's shader compiler (tens of MB),
//! the phone adds its drawables and system frameworks.

use std::sync::atomic::{AtomicU64, Ordering};

/// Bytes allocated through the kit's helpers (textures, the offscreen target), cumulative.
static KIT_BYTES: AtomicU64 = AtomicU64::new(0);

/// Counts `bytes` as allocated through the kit.
pub fn count(bytes: u64) {
    KIT_BYTES.fetch_add(bytes, Ordering::Relaxed);
}

/// Bytes allocated through the kit's helpers so far.
pub fn kit_bytes() -> u64 {
    KIT_BYTES.load(Ordering::Relaxed)
}

/// A texture's bytes (every mip level and layer; block formats by their block size).
pub fn texture_bytes(desc: &wgpu::TextureDescriptor) -> u64 {
    let f = desc.format;
    let (bw, bh) = f.block_dimensions();
    let block = f.block_copy_size(None).unwrap_or(4) as u64;
    let layers = if desc.dimension == wgpu::TextureDimension::D3 { 1 } else { desc.size.depth_or_array_layers as u64 };
    let mut total = 0;
    for m in 0..desc.mip_level_count {
        let e = desc.size.mip_level_size(m, desc.dimension);
        let d = if desc.dimension == wgpu::TextureDimension::D3 { e.depth_or_array_layers as u64 } else { 1 };
        total += (e.width.div_ceil(bw) as u64) * (e.height.div_ceil(bh) as u64) * d * block;
    }
    total * layers * desc.sample_count as u64
}

/// Whether the check runs under the iPhone's limits (`KIT_IPHONE_LIMITS=1`).
pub fn iphone() -> bool {
    std::env::var("KIT_IPHONE_LIMITS").is_ok_and(|v| v == "1")
}

/// The limits wgpu 30 gives an iPhone's GPU on Metal, for the Apple7 family (A14, A15: iPhone 12 and
/// 13; the A12 of the oldest iOS 17 phones differs only in 72 sampled textures a stage and 128-entry
/// sampler arrays, both far above what the kit requests). From wgpu-hal 30.0.1, src/metal/adapter.rs:
/// `PrivateCapabilities::new` (lines 847–1022: per-stage counts, buffer size, texture sizes, varyings,
/// threads and threadgroup memory) and `capabilities()` (lines 1370–1490: how they become
/// `wgt::Limits`, Metal's 31 buffer slots less the 2 wgpu keeps), then src/auxil/mod.rs:128–141
/// (`adjust_raw_limits`: 8 bind groups, 16 vertex buffers, 24 of both). Where the kit's own figure is
/// stricter than the phone's (offset alignments, bindings a group) it is kept: nothing the kit makes
/// needs more.
pub fn iphone_limits() -> wgpu::Limits {
    // maxBufferLength is queried from the device (adapter.rs:897–903); 256 MB is the hal's floor for
    // iOS (line 902) and the least any iPhone reports, so it is what is assumed here (an estimate: an
    // A14 reports more, but nothing above 256 MB can be counted on)
    let max_buffer: u64 = 1 << 28;
    // Metal's 31 buffer slots a stage, less wgpu's two (immediates, buffer sizes): adapter.rs:1390
    let buffers: u32 = 31 - 2;
    let base = wgpu::Limits::default();
    wgpu::Limits {
        max_texture_dimension_1d: 16384, // :907–920 (iOS_GPUFamily3_v1 and later, below Apple10)
        max_texture_dimension_2d: 16384,
        max_texture_dimension_3d: 2048, // :922
        max_texture_array_layers: 2048, // :924
        max_bind_groups: 8,             // auxil :130
        max_bind_groups_plus_vertex_buffers: 24, // auxil :138
        max_bindings_per_bind_group: 65535,
        max_dynamic_uniform_buffers_per_pipeline_layout: base.max_dynamic_uniform_buffers_per_pipeline_layout,
        max_dynamic_storage_buffers_per_pipeline_layout: base.max_dynamic_storage_buffers_per_pipeline_layout,
        max_sampled_textures_per_shader_stage: 96, // :847–855 (Apple6 and later: 96 sampled, 32 storage)
        max_samplers_per_shader_stage: 16,         // :1418
        max_storage_buffers_per_shader_stage: buffers,
        max_storage_textures_per_shader_stage: 32,
        max_uniform_buffers_per_shader_stage: buffers,
        max_binding_array_elements_per_shader_stage: 1_000_000, // :856–862 (argument buffers tier 2)
        max_binding_array_acceleration_structure_elements_per_shader_stage: 0,
        max_binding_array_sampler_elements_per_shader_stage: 1024, // :864–877 (Apple7)
        max_uniform_buffer_binding_size: max_buffer, // :1426
        max_storage_buffer_binding_size: max_buffer, // :1428
        max_vertex_buffers: 16,                      // auxil :131–133, MAX_VERTEX_BUFFERS
        max_buffer_size: max_buffer,
        max_vertex_attributes: 31,
        max_vertex_buffer_array_stride: base.max_vertex_buffer_array_stride,
        min_uniform_buffer_offset_alignment: 64, // the phone's is 4 (:887–895)
        min_storage_buffer_offset_alignment: 64, // and 32 (:1431)
        max_inter_stage_shader_variables: 31, // :981–988 (Apple4 and later; not the Mac's)
        max_color_attachments: 8,             // :934–945
        max_color_attachment_bytes_per_sample: 64, // :948–958 (Apple4 and later)
        max_compute_workgroup_storage_size: 32 << 10, // :1012–1022
        max_compute_invocations_per_workgroup: 1024, // :997–1007
        max_compute_workgroup_size_x: 1024,
        max_compute_workgroup_size_y: 1024,
        max_compute_workgroup_size_z: 1024,
        max_compute_workgroups_per_dimension: 0xFFFF,
        max_immediate_size: 0x1000, // :1447
        max_non_sampler_bindings: u32::MAX,
        // mesh shaders, multiview and ray tracing: none (the kit uses none, and an A14 lacks them)
        max_task_workgroup_total_count: 0,
        max_task_workgroups_per_dimension: 0,
        max_mesh_workgroup_total_count: 0,
        max_mesh_workgroups_per_dimension: 0,
        max_task_invocations_per_workgroup: 0,
        max_task_invocations_per_dimension: 0,
        max_mesh_invocations_per_workgroup: 0,
        max_mesh_invocations_per_dimension: 0,
        max_task_payload_size: 0,
        max_mesh_output_vertices: 0,
        max_mesh_output_primitives: 0,
        max_mesh_output_layers: 0,
        max_mesh_multiview_view_count: 0,
        max_blas_primitive_count: 0,
        max_blas_geometry_count: 0,
        max_tlas_instance_count: 0,
        max_acceleration_structures_per_shader_stage: 0,
        max_buffers_and_acceleration_structures_per_shader_stage: buffers,
        max_multiview_view_count: 0,
        max_ray_dispatch_count: 0,
        max_ray_recursion_depth: 0,
    }
}

/// The features an Apple7 iPhone on iOS 17 has under wgpu 30 that the kit could ever want: those
/// `features()` sets unconditionally (wgpu-hal 30.0.1, src/metal/adapter.rs:1185–1197, less the MSL
/// passthrough, external textures and adapter-specific format features, which a toy must not lean
/// on), and those that hold for every Apple7 phone (:1202 indirect, :1223 and :1228 ASTC and ETC2,
/// :1230 depth clip, :1263–1266 int64). BC compression, float32 filtering (:1200) and clamp-to-border
/// are left out: an iPhone lacks them or may. (Multi-draw indirect, a feature under wgpu 26, is no
/// longer one in wgpu 30.)
pub fn iphone_features() -> wgpu::Features {
    use wgpu::Features as F;
    F::MAPPABLE_PRIMARY_BUFFERS
        | F::VERTEX_WRITABLE_STORAGE
        | F::IMMEDIATES
        | F::POLYGON_MODE_LINE
        | F::CLEAR_TEXTURE
        | F::TEXTURE_FORMAT_16BIT_NORM
        | F::SHADER_F16
        | F::SHADER_I16
        | F::DEPTH32FLOAT_STENCIL8
        | F::BGRA8UNORM_STORAGE
        | F::INDIRECT_FIRST_INSTANCE
        | F::TEXTURE_COMPRESSION_ASTC
        | F::TEXTURE_COMPRESSION_ETC2
        | F::DEPTH_CLIP_CONTROL
        | F::SHADER_INT64
}

/// `KIT_MEMORY_REPORT=<file>`: samples the device's live GPU bytes every 5 ms and keeps the peaks in
/// the file (written whole, then renamed, so the file is always complete when the process ends).
/// (Native only: the web has no threads or environment, and wgpu's device is not `Send` there.)
#[cfg(target_arch = "wasm32")]
pub fn start_report(_device: std::sync::Arc<wgpu::Device>) {}

/// `KIT_MEMORY_REPORT=<file>`, natively.
#[cfg(not(target_arch = "wasm32"))]
pub fn start_report(device: std::sync::Arc<wgpu::Device>) {
    let Ok(path) = std::env::var("KIT_MEMORY_REPORT") else { return };
    let path = std::path::PathBuf::from(path);
    std::thread::spawn(move || {
        let (mut peak, mut kit_peak, mut written) = (0i64, 0u64, (-1i64, 0u64));
        loop {
            let c = device.get_internal_counters();
            let live = (c.hal.buffer_memory.read() + c.hal.texture_memory.read()) as i64;
            peak = peak.max(live);
            kit_peak = kit_peak.max(kit_bytes());
            if (peak, kit_peak) != written {
                let tmp = path.with_extension("tmp");
                let json = format!("{{\"gpu_peak_bytes\": {peak}, \"gpu_live_bytes\": {live}, \"kit_bytes\": {kit_peak}}}\n");
                if std::fs::write(&tmp, json).is_ok() {
                    let _ = std::fs::rename(&tmp, &path);
                }
                written = (peak, kit_peak);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_bytes_count_mips_and_layers() {
        let d = |w, h, layers, mips, format| wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: layers },
            mip_level_count: mips,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        assert_eq!(texture_bytes(&d(4, 4, 1, 1, wgpu::TextureFormat::Rgba8Unorm)), 64);
        assert_eq!(texture_bytes(&d(4, 4, 1, 3, wgpu::TextureFormat::R8Unorm)), 16 + 4 + 1);
        assert_eq!(texture_bytes(&d(8, 2, 12, 1, wgpu::TextureFormat::R8Unorm)), 16 * 12);
    }

    #[test]
    fn the_request_fits_an_iphone() {
        // what the kit requests of the device must be within what an Apple7 iPhone grants
        let mut l = wgpu::Limits::default().using_resolution(iphone_limits());
        l.max_texture_dimension_2d = l.max_texture_dimension_2d.max(4096);
        assert!(l.check_limits(&iphone_limits()));
        assert!(!wgpu::Features::TEXTURE_COMPRESSION_BC.intersects(iphone_features()));
    }

    /// Under the iPhone's limits a buffer bigger than an iPhone grants is a validation error, and in
    /// the check a validation error is fatal. Needs a GPU (Mesa's lavapipe will do).
    #[test]
    #[should_panic(expected = "GPU validation error")]
    fn a_buffer_the_phone_refuses_fails_the_check() {
        std::env::set_var("KIT_IPHONE_LIMITS", "1");
        let gpu = crate::check::gpu().expect("a GPU");
        let _ = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("too big for a phone"),
            size: (256 << 20) + 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
    }
}
