// What every mark's shader shares (floats, wake, maybes, silver, words), after the View struct
// (gpu/view.rs) and common.wgsl: placing a unit vector on the screen, a quad's corner, reading points,
// coverage, the zoom's scaling. The Rust prepends it (look::marks_wgsl); tools/adrift/wgsl_check.sh
// does the same to validate the marks with naga.

@group(0) @binding(0) var<uniform> U: View;

// A position's place on the screen: xy in pixels (y down), z its facing (the view's z; hidden
// below 0.3, as the names are), w the globe's radius in pixels.
fn place(g: vec3<f32>) -> vec4<f32> {
    let v = vec3<f32>(dot(U.rot0.xyz, g), dot(U.rot1.xyz, g), dot(U.rot2.xyz, g));
    let ppp = U.frame.z;
    return vec4<f32>((U.disc.x + v.x * U.disc.z) * ppp, (U.disc.y - v.y * U.disc.z) * ppp, v.z, U.disc.z * ppp);
}

// Clip space from a pixel.
fn clip(px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(px.x / U.frame.x * 2.0 - 1.0, 1.0 - px.y / U.frame.y * 2.0, 0.0, 1.0);
}

// A quad's corner for vertex i of six, in [-1, 1].
fn corner(i: u32) -> vec2<f32> {
    let x = select(-1.0, 1.0, i == 1u || i == 2u || i == 4u);
    let y = select(-1.0, 1.0, i == 2u || i == 4u || i == 5u);
    return vec2<f32>(x, y);
}

// Pixels a reading point: every size below is in reading points (CLAUDE.md › Design rules).
fn rp() -> f32 { return U.frame.z * U.frame.w; }

// A coverage from a signed distance in pixels (half a pixel of edge).
fn cover(d: f32) -> f32 { return clamp(0.5 - d, 0.0, 1.0); }

// How large a mark is by zoom: full size from zoom 'full' down to 'floor' of it at the whole Earth,
// so the wake and the silver don't pile into a slug when the camera is far (CRITICS.md, R0).
fn by_zoom(floor_k: f32, full: f32) -> f32 {
    return mix(floor_k, 1.0, smoothstep(1.0, full, U.disc.w));
}
