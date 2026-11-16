//! The bind group layouts of the automaton's passes and of the window's drawing.

/// The automaton's: the globals, the two grids, the mask, the stats, the field, the op, the glaze.
pub fn sim(dev: &wgpu::Device) -> wgpu::BindGroupLayout {
    let cs = wgpu::ShaderStages::COMPUTE;
    dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("frost sim"),
        entries: &[
            uniform(0, cs),
            storage(1, true, cs),
            storage(2, false, cs),
            storage(3, false, cs),
            storage(4, false, cs),
            storage(5, false, cs),
            uniform(6, cs),
            storage(7, false, cs),
        ],
    })
}

/// The window's: the look and the field.
pub fn draw(dev: &wgpu::Device) -> wgpu::BindGroupLayout {
    let fs = wgpu::ShaderStages::FRAGMENT;
    dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("frost draw"),
        entries: &[uniform(0, fs), storage(1, true, fs)],
    })
}

fn storage(binding: u32, read_only: bool, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
