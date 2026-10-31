// The View every look shader takes first (DECISIONS.md §3: Terra's OceanUniforms layout), and the
// projection they share. Prepended to each pass's shader by the Rust that owns it (WGSL has no
// include); tools/adrift/wgsl_check.sh does the same to validate them with naga.

struct View {
    frame: vec4<f32>,   // the frame's width and height (pixels), pixels a point, the reading scale
    disc: vec4<f32>,    // the globe's centre x, y and radius (points, y down), the zoom
    rot0: vec4<f32>,    // world from view, by columns
    rot1: vec4<f32>,
    rot2: vec4<f32>,
    time: vec4<f32>,    // seconds; this month (0 = January), the next, how far between
    tone: vec4<f32>,    // x how much of the sea shows; y 1 for motion, 0 under Reduce Motion;
                        // z the drawn motion's seconds (held with the date); w 1 while the date is stopped
};

@group(0) @binding(0) var<uniform> V: View;

// A position's place on the screen: xy in pixels (y down), z its facing (the view's z; hidden
// below 0.3, as the names are), w the globe's radius in pixels.
fn place(g: vec3<f32>) -> vec4<f32> {
    let v = vec3<f32>(dot(V.rot0.xyz, g), dot(V.rot1.xyz, g), dot(V.rot2.xyz, g));
    let ppp = V.frame.z;
    return vec4<f32>((V.disc.x + v.x * V.disc.z) * ppp, (V.disc.y - v.y * V.disc.z) * ppp, v.z, V.disc.z * ppp);
}

// Clip space from a pixel.
fn clip(px: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(px.x / V.frame.x * 2.0 - 1.0, 1.0 - px.y / V.frame.y * 2.0, 0.0, 1.0);
}

// A quad's corner for vertex i of six, in [-1, 1].
fn corner(i: u32) -> vec2<f32> {
    let x = select(-1.0, 1.0, i == 1u || i == 2u || i == 4u);
    let y = select(-1.0, 1.0, i == 2u || i == 4u || i == 5u);
    return vec2<f32>(x, y);
}

// Pixels a reading point: every size below is in reading points (CLAUDE.md › Design rules).
fn rp() -> f32 { return V.frame.z * V.frame.w; }

// A coverage from a signed distance in pixels (half a pixel of edge).
fn cover(d: f32) -> f32 { return clamp(0.5 - d, 0.0, 1.0); }

// Lays a colour over premultiplied acc.
fn over(acc: vec4<f32>, col: vec3<f32>, a: f32) -> vec4<f32> {
    return vec4<f32>(col * a, a) + acc * (1.0 - a);
}

// How large a mark is by zoom: full size from zoom 'full' down to 'floor' of it at the whole Earth,
// so the wake and the silver don't pile into a slug when the camera is far (CRITICS.md, R0).
fn by_zoom(floor_k: f32, full: f32) -> f32 {
    return mix(floor_k, 1.0, smoothstep(1.0, full, V.disc.w));
}
