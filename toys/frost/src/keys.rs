//! Frost drawn as Peter Foldès drew *Metadata* (1971) and *Hunger* (1974) on the National Research Council
//! of Canada's key-frame system (Nestor Burtnyk and Marceli Wein; SOURCES.md): the artist made the key
//! drawings and the computer made the drawings between them, "line interpolation", so the lines "dissolve
//! and reshape". Here the automaton is the artist: every `KEY_S` the ice it has grown is taken as a key
//! drawing, and the window shows the drawing between the last two keys (pane.wgsl draws it in line).

/// A key drawing this often, seconds.
pub const KEY_S: f64 = 0.2;

/// The last two key drawings (the field as it was), and when the newer was taken.
pub struct Keys {
    field: wgpu::Buffer,
    a: wgpu::Buffer,
    b: wgpu::Buffer,
    at: f64,
}

impl Keys {
    pub fn new(gpu: &toy_kit::Gpu, field: &wgpu::Buffer) -> Keys {
        let key = |label| {
            gpu.buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: field.size(),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        Keys {
            field: field.clone(),
            a: key("frost key a"),
            b: key("frost key b"),
            at: f64::NEG_INFINITY,
        }
    }

    /// The window's bind group: the look, the older key, the newer.
    pub fn bind(
        &self,
        dev: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        look: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frost draw"),
            layout,
            entries: &[entry(0, look), entry(1, &self.a), entry(2, &self.b)],
        })
    }

    /// Takes a new key drawing when one is due (the newer key becomes the older); how far the drawing at
    /// `t` is from the older key toward the newer, 0..1.
    pub fn turn(&mut self, enc: &mut wgpu::CommandEncoder, t: f64) -> f32 {
        if t - self.at >= KEY_S || t < self.at {
            let size = self.field.size();
            if self.at.is_finite() {
                enc.copy_buffer_to_buffer(&self.b, 0, &self.a, 0, size);
            } else {
                enc.copy_buffer_to_buffer(&self.field, 0, &self.a, 0, size);
            }
            enc.copy_buffer_to_buffer(&self.field, 0, &self.b, 0, size);
            // a key is held to the beat if a frame came late; a clock set back starts the beat again
            self.at = if (0.0..2.0 * KEY_S).contains(&(t - self.at)) {
                self.at + KEY_S
            } else {
                t
            };
        }
        ((t - self.at) / KEY_S).clamp(0.0, 1.0) as f32
    }
}

fn entry(binding: u32, b: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: b.as_entire_binding(),
    }
}
