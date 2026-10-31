// The maybes: a held float's other possible selves, as pale specks where each would be now. The
// positions come from the GPU (gpu/maybes.rs: lon, lat, held, k); the vertex shader projects them on
// the globe and the fragment draws each (Visual's to design; R0 is a soft dot).

@group(0) @binding(0) var<uniform> U: View;

struct Maybe { @location(0) p: vec4f };   // lon, lat (degrees), held, k
struct MOut { @builtin(position) position: vec4f, @location(0) local: vec2f, @location(1) fade: f32, @location(2) held: f32 };

@vertex fn vs_maybe(@builtin(vertex_index) vid: u32, m: Maybe) -> MOut {
    let la = radians(m.p.y); let lo = radians(m.p.x);
    let w = vec3f(cos(la) * sin(lo), sin(la), cos(la) * cos(lo));
    let rot = mat3x3f(U.rot0.xyz, U.rot1.xyz, U.rot2.xyz);
    let v = transpose(rot) * w;
    let size = 1.4 * U.frame.w;
    let corner = vec2f(f32(vid & 1u) * 2.0 - 1.0, f32((vid >> 1u) & 1u) * 2.0 - 1.0);
    let pt = vec2f(U.disc.x + v.x * U.disc.z, U.disc.y - v.y * U.disc.z) + corner * (size + 1.0);
    var out: MOut;
    let behind = v.z < 0.0;
    out.position = select(vec4f(pt.x / (U.frame.x / U.frame.z) * 2.0 - 1.0, 1.0 - pt.y / (U.frame.y / U.frame.z) * 2.0, 0.0, 1.0), vec4f(0.0, 0.0, 2.0, 1.0), behind);
    out.local = corner * (size + 1.0);
    out.fade = smoothstep(0.0, 0.15, v.z);
    out.held = m.p.z;
    return out;
}

@fragment fn fs_maybe(in: MOut) -> @location(0) vec4f {
    let d = length(in.local);
    let size = 1.4 * U.frame.w;
    let a = (1.0 - smoothstep(size - 0.5, size + 0.5, d)) * 0.55 * in.fade;
    let col = select(vec3f(1.0, 0.96, 0.88), vec3f(0.85, 0.82, 0.78), in.held > 0.5);
    return vec4f(col * a, a);
}
