//! What froze, read back from the GPU each frame: the stats buffer's few words (each slot's reach and
//! count, then the probe's). On the device and in the check at once, the same on every run; in a browser,
//! where WebGPU maps a buffer only once the page's event loop runs, a frame or two late (the last words
//! read stand in meanwhile), and the probe's answer is fresh only once a copy made after it has come back.

use toy_kit::Gpu;

pub struct Readback {
    buffer: wgpu::Buffer,
    size: u64,
    #[cfg(target_arch = "wasm32")]
    late: Late,
}

/// The browser's readback: 0 idle, 1 mapping, 2 mapped; the copies made, the one the probe needs.
#[cfg(target_arch = "wasm32")]
struct Late {
    state: std::sync::Arc<std::sync::atomic::AtomicU8>,
    last: Vec<u32>,
    copies: u64,
    copy_in_flight: u64,
    probe_from: u64,
    have: u64,
}

impl Readback {
    pub fn new(gpu: &Gpu, size: u64) -> Readback {
        let buffer = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("frost readback"),
            size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Readback {
            buffer,
            size,
            #[cfg(target_arch = "wasm32")]
            late: Late {
                state: Default::default(),
                last: vec![0; size as usize / 4],
                copies: 0,
                copy_in_flight: 0,
                probe_from: 0,
                have: 0,
            },
        }
    }

    /// The probe ran this frame: its answer is fresh once this frame's copy (or a later one) is read.
    pub fn probed(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.late.probe_from = self.late.copies + 1;
        }
    }

    /// Copies `stats` back, submits `enc`, and gives the words and whether the probe's are fresh.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn read(
        &mut self,
        gpu: &Gpu,
        mut enc: wgpu::CommandEncoder,
        stats: &wgpu::Buffer,
    ) -> (Vec<u32>, bool) {
        enc.copy_buffer_to_buffer(stats, 0, &self.buffer, 0, self.size);
        gpu.queue.submit([enc.finish()]);
        let slice = self.buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let counts = bytemuck::cast_slice(&slice.get_mapped_range().expect("mapped")).to_vec();
        self.buffer.unmap();
        (counts, true)
    }

    /// Copies `stats` back, submits `enc`, and gives the words and whether the probe's are fresh.
    #[cfg(target_arch = "wasm32")]
    pub fn read(
        &mut self,
        gpu: &Gpu,
        mut enc: wgpu::CommandEncoder,
        stats: &wgpu::Buffer,
    ) -> (Vec<u32>, bool) {
        use std::sync::atomic::Ordering;
        let l = &mut self.late;
        if l.state.load(Ordering::Acquire) == 2 {
            if let Ok(view) = self.buffer.slice(..).get_mapped_range() {
                l.last = bytemuck::cast_slice(&view).to_vec();
                l.have = l.copy_in_flight;
            }
            self.buffer.unmap();
            l.state.store(0, Ordering::Release);
        }
        let copying = l.state.load(Ordering::Acquire) == 0;
        if copying {
            l.copies += 1;
            l.copy_in_flight = l.copies;
            enc.copy_buffer_to_buffer(stats, 0, &self.buffer, 0, self.size);
        }
        gpu.queue.submit([enc.finish()]);
        if copying {
            l.state.store(1, Ordering::Release);
            let state = l.state.clone();
            self.buffer
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    state.store(if r.is_ok() { 2 } else { 0 }, Ordering::Release);
                });
        }
        (l.last.clone(), l.probe_from <= l.have)
    }
}
