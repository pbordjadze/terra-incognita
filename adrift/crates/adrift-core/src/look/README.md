# The look

Visual's directory (DECISIONS.md §2): every drawing shader and the Rust that owns its pipeline.
What is here before the skeleton, and how each pass binds, so it wires in a morning when the
skeleton lands.

- `WORKS.md`: the works the look is measured against. `CRITICS.md`: each round's hater and Holt,
  on the pictures in `studies/`. `PALETTE.md` and `palette.json`: every colour and size, the
  contrast measured (`tools/adrift/palette.py` fails below the rule). The shaders carry the
  palette's numbers as constants (0..1 sRGB); `palette.json` is the spec they are checked against.
- `view.wgsl`: the `View` uniform (DECISIONS.md §3, Terra's `OceanUniforms` layout) at
  `@group(0) @binding(0)`, and what every pass shares: `place` (a unit vector to pixels and its
  facing), `clip`, `corner` (a quad from six vertices), `rp` (pixels a reading point), `cover`,
  `over` (premultiplied), `by_zoom`. The Rust prepends it to each pass's source;
  `bash tools/adrift/wgsl_check.sh` validates every pass that way with naga.
- Draw order (DECISIONS.md §3): ground, sea (Engineering's port of Terra's, which Visual takes
  from R1), then these, each a `Pass`, instanced quads, premultiplied-alpha blending
  (`One, OneMinusSrcAlpha`), no depth, six vertices an instance:

| pass | instance (vertex buffer, step mode instance) | group 1 |
| --- | --- | --- |
| `wake.wgsl` | `g: vec3<f32>`, `age: f32` (0 newest .. 1 oldest), `night: f32` | none |
| `maybes.wgsl` | `g: vec3<f32>` (Truth's buffer, written by `rules.wgsl`) | `Maybes {bloom, night}` uniform |
| `silver.wgsl` | `a: vec3<f32>`, `b: vec3<f32>`, `t: f32` (segments of the track) | `Silver {rise, night}` uniform |
| `floats.wgsl` | `g: vec3<f32>`, `beat` (0..1 the resting finger; 1 a float), `lit`, `held`, `night` | none |
| `words.wgsl` | `rect: vec4` (points, y down), `uv: vec4` (atlas 0..1), `ink: vec4` | `Words {halo, spread, halo_pt, layer}`, the atlas `texture_2d`, a linear sampler with mipmaps; drawn twice, halos (`layer` 0) then inks (1) |

- `type/libertinus.png` and `.json`: the signed-distance atlas of Libertinus Serif Regular (OFL)
  for the date and the credits, built by `tools/adrift/glyphs.py` (its docstring is the layout);
  the Rust lays a line out from the JSON's advances and bearings in points, centred at the foot.
- A float past the limb (facing under 0.3) is clipped away in the vertex shader; the Rust owns the
  slack (shown above 0.4, kept to 0.3) so nothing flickers at the rim.
- `tools/adrift/study.py` draws all of this over Terra's stand-in sea without the build; it is the
  numpy twin of these shaders until `adrift-check` renders them, and stays as the place to try a
  colour or a size in seconds.
