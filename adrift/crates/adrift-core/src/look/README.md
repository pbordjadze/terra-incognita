# The look

Visual's directory (DECISIONS.md §2): every drawing shader and the Rust that owns its pipeline.
What is here before the skeleton, and how each pass binds, so it wires in a morning when the
skeleton lands.

- `WORKS.md`: the works the look is measured against. `CRITICS.md`: each round's hater and Holt,
  on the pictures in `studies/`. `PALETTE.md` and `palette.json`: every colour and size, the
  contrast measured (`tools/adrift/palette.py` fails below the rule). The shaders carry the
  palette's numbers as constants (0..1 sRGB); `palette.json` is the spec they are checked against.
- `marks.wgsl`: what every mark's shader shares after the `View` struct (`gpu/view.rs`) and
  `common.wgsl`: the view's binding `U`, `place` (a unit vector to pixels and its facing), `clip`,
  `corner` (a quad from six vertices), `rp` (pixels a reading point), `cover`, `by_zoom`.
  `look::marks_wgsl` prepends them to each mark's source; `bash tools/adrift/wgsl_check.sh`
  validates every mark that way with naga.
- `mod.rs`: `Look`, the marks behind one `prepare` and one `draw` in their order, and `LookState`,
  everything they draw in a frame, which the app fills from its state (the look reads nothing
  else); the shared `Instances`, `Uniform`, `mark_pipeline`, `view_layout`. Engineering's
  `ground` and `sea` (R0's ports of Terra's) and `sprites` (R0's plain forms, parked once `Look`
  is wired: DEMANDS.md D1) live beside them.
- Draw order (DECISIONS.md §3): ground, sea, then the marks, instanced quads, premultiplied-alpha
  blending (`Gpu::over`), no depth, six vertices an instance:

| pass | instance (vertex buffer, step mode instance) | group 1 |
| --- | --- | --- |
| `wake.wgsl` | `g: vec3<f32>`, `age: f32` (0 newest .. 1 oldest), `night: f32` | `Wake {layer}`; drawn twice, edges then beads |
| `maybes.wgsl` | `p: vec4<f32>` lon, lat, held, k (`gpu::Maybes`, the buffer `rules.wgsl` fills) | `Maybes {bloom, night}` uniform |
| `silver.wgsl` | `a: vec3<f32>`, `b: vec3<f32>`, `t: f32` (segments of the track) | `Silver {rise, night, layer}`; drawn twice, dark lines then wires |
| `floats.wgsl` | `g: vec3<f32>`, `beat` (0..1 the resting finger; 1 a float), `lit`, `held`, `night` | none |
| `words.wgsl` | `rect: vec4` (points, y down), `uv: vec4` (atlas 0..1), `ink: vec4`, `halo: vec4` | `Words {spread, halo_pt, layer}`, the atlas `texture_2d`, a linear sampler with mipmaps; drawn twice, halos (`layer` 0) then inks (1) |

- `type/libertinus.png` and `.json`: the signed-distance atlas of Libertinus Serif Regular and Italic
  (OFL) for the date, the note and the credits, built by `tools/adrift/glyphs.py` (its docstring is
  the layout); the Rust lays a line out from the JSON's advances and bearings in points: the date
  under the disc (over the globe when it fills the screen), the note in italic under it, the
  credits along the foot, each on a halo of what is under it.
- `Look::date_bounds()` is where the date was drawn (points), the one drawn thing that is also a
  control: Motion's scrub and pause probe it through the app. `LookState.resting` takes Motion's
  `Press { at, since }` as `(at, thicken(now - since))`.
- A float past the limb (facing under 0.3) is clipped away in the vertex shader; the Rust owns the
  slack (shown above 0.4, kept to 0.3) so nothing flickers at the rim.
- `tools/adrift/lookbench` (a crate of its own on `adrift-core`) draws the marks with wgpu, headless,
  over a stand-in sea from a scene `tools/adrift/study.py --scene` writes, with the maybes and the
  drifter in place before Truth's data exists (its `main.rs` says how). The whole app is
  `adrift-check`.

## Wiring (DEMANDS.md D1)

In `app.rs`: `use crate::look::{sea::Ripple, GroundPass, Look, LookFloat, LookState, SeaPass};`,
a field `look: Look` made with `Look::new(&gpu, &view_layout)` in place of `sprites` and
`maybes_pass`, and in `render`, where the sprites were prepared:

    let state = self.look_state();
    let d = view.disc;
    self.look.prepare(&self.gpu, &state, (self.width as f64 / self.ppp) as f32,
                      (self.height as f64 / self.ppp) as f32, self.reading as f32,
                      [d.cx as f32, d.cy as f32, d.r as f32]);
    ...
    self.blit.draw(&mut rp, self.stage.as_ref().unwrap());
    self.look.draw(&mut rp, &self.view_bind, &self.maybes);   // in place of maybes_pass, sprites and text_pass

and `look_state()`:

    fn look_state(&self) -> LookState {
        let (month, day) = self.clock.calendar();
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June",
            "July", "August", "September", "October", "November", "December"];
        LookState {
            floats: self.floats.iter().map(|f| LookFloat {
                at: f.at,
                lit: (1.0 - (f.lit_for / 3.0)).clamp(0.0, 1.0) as f32,
                held: f.hand == Hand::Held,
                wake: f.wake.iter().map(|b| b.at).collect(),
            }).collect(),
            resting: None,                     // D2: the gestures' resting finger and its beat
            bloom: 1.0,                        // (the maybes' bloom and fold, when Motion times them)
            drifter: Vec::new(), rise: 0.0,    // Truth's tracks, when they come
            night: false,
            date: format!("{} {}", day, MONTHS[(month as usize - 1).min(11)]),
            note: "a typical year, by the drifters\u{2019} record".into(),
            credits: "Currents: NOAA Global Drifter Program (Laurindo, Mariano and Lumpkin 2017), CC BY 4.0".into(),
        }
    }
