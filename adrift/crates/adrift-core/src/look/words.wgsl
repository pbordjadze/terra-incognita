// The date and the credits as drawn things: glyphs from a signed-distance atlas of Libertinus Serif
// (type/libertinus.png, built by tools/adrift/glyphs.py), each on a halo 1.5 reading points each side
// in the open sea's tone (the night's sea by night), as Terra's names are. One instanced quad a glyph,
// placed by the Rust in points; nothing here is a UI kit. Drawn twice: every glyph's halo, then every
// glyph's ink (W.layer), or a letter's halo would cover the letter before it, as the cells overlap.

struct GlyphIn {
    @location(0) rect: vec4<f32>,   // x, y, w, h on the screen, points (y down), the halo's room included
    @location(1) uv: vec4<f32>,     // the glyph's atlas rectangle, 0..1
    @location(2) ink: vec4<f32>,    // the ink, and its alpha (fading only in time)
};

struct GlyphOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ink: vec4<f32>,
    @location(2) px_per_texel: f32,
};

struct Words {
    halo: vec4<f32>,    // the halo's colour and alpha
    spread: f32,        // the atlas's distance spread in texels (glyphs.py writes it)
    halo_pt: f32,       // the halo's reach in reading points (1.5)
    layer: f32,         // 0 this draw lays the halos, 1 the inks
    _pad: f32,
};
@group(1) @binding(0) var<uniform> W: Words;
@group(1) @binding(1) var atlas: texture_2d<f32>;
@group(1) @binding(2) var atlas_s: sampler;

@vertex
fn vs(@builtin(vertex_index) vi: u32, g: GlyphIn) -> GlyphOut {
    let c = corner(vi % 6u) * 0.5 + 0.5;
    let ppp = V.frame.z;
    var o: GlyphOut;
    o.pos = clip((g.rect.xy + c * g.rect.zw) * ppp);
    o.uv = g.uv.xy + c * g.uv.zw;
    o.ink = g.ink;
    let dims = vec2<f32>(textureDimensions(atlas));
    o.px_per_texel = g.rect.z * ppp / (g.uv.z * dims.x);
    return o;
}

@fragment
fn fs(i: GlyphOut) -> @location(0) vec4<f32> {
    // the atlas holds (distance / spread) * 0.5 + 0.5, inside positive
    let s = textureSample(atlas, atlas_s, i.uv).r;
    let d_tex = (s - 0.5) * 2.0 * W.spread;                 // texels inside the outline
    let d = d_tex * i.px_per_texel;                         // pixels inside
    let halo_px = W.halo_pt * rp();
    var acc = vec4<f32>(0.0);
    if (W.layer < 0.5) {
        acc = over(acc, W.halo.rgb, W.halo.a * cover(-(d + halo_px)));
    } else {
        acc = over(acc, i.ink.rgb, i.ink.a * cover(-d));
    }
    return acc;
}
