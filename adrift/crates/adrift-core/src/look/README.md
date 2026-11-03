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
| `veil.wgsl` | none: a full-screen quad | `Veil {centre, radius, soft, strength, night}`: the night deepening round the chosen lamp, drawn before the marks |
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
- `ground.wgsl` and `sea.wgsl` are Visual's from R2: Terra's ports, with the night palette gated by
  `View.pad.x` (DEMANDS.md D4: the `N_` ground with the Earth's lights from `lightsTex` at `@group(1) @binding(7)`, the sea's lines pale, the surf a line, no sheen) and
  the sea's rest eased by `View.tone.w` (D3: the running light settles to half, the glitter goes).
- (R24, cost) The ground and the sea read only what can show (the land's picture on land and the
  coast's line, the lights by night, the sea's water after its land/ice/off-globe early-outs); the
  stream sheet is paired by month (`sea.rs` `pair_months`: 16 loads a pixel for both surfaces, from
  32); the light along the lines is worked out at half resolution (`fs_glow`, `SeaPass::glow_pass`,
  which the stage encodes before its pass) and read between texels. `ADRIFT_PROFILE=1` times each
  pass (`profile.rs`; `adrift-check journey` prints the table). Before/after: `studies/r24-export-cuts.png`.
  The maybes bloom from the held float's place to their own as `LookState.bloom` runs 0 → 1.
- (R25, lamps with character) A lamp is a light in glass of its own (`floats.wgsl` `lamp_kind`, by
  its id, which `FloatInstance.id` carries): a round float with a rope at its waist and a knot at its
  crown, a cut (octagonal) float, a lantern with its cap, bail and frame, or an onion-domed lamp with a
  finial (neighbours by id and ids four apart never share a glass); inside, a flame of its own height
  and width that flickers and sways at its own rate, the glass lit by it and tinted by its hue (more so
  when small, so the hue still tells lamps apart at the whole Earth), its light breathing on its own
  period, a faint shadow by day, and below it the light's broken path on the swell. Studies:
  `studies/r25-*` (plans `seven-lamps.json`, `-night`; sheets by `tools/adrift/lamp_study.py`).
  R25b, a clip's head: 1.5x (from 1.3x) so its glass reads full-frame, its glass nearer its hue, its
  flame a quarter larger, its glint smaller; the light's path on the water a little stronger by day
  (`studies/r25b-*`: after tiktok.sh at 4 s, day then night, each R24 | R25 | R25b).
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

## R6: lights and maybes apart, the lamp one maybe among them, the panel, several clouds

- **The night's lights** (`ground.wgsl`): each light coloured by its brightness as photographs from
  orbit show it (a village an ember, a town gold, a city's heart whitening), and its glow read off the
  same sheet a few mips coarser than the pixel's own, a near pool and a wide warm air, spilling a little
  over the coast. Measured lights only; the glow is how they are drawn.
- **The maybes by night** (`maybes.wgsl`) are of the sea: the lamp's hue cooled toward the glass's
  pale, and close in each a tiny ring of glass with its lamp's spark at the heart. Towns are warm points
  in pools of glow; nothing on the water is drawn that way, so a maybe is never read as a town.
- **The lamp while its cloud is out** is drawn as one maybe of it (`floats.wgsl › as_maybe`, the
  speck's own sizes and inks, crossfading with the cloud's bloom), with no wake; a lamp whose cloud is
  not out keeps its glass float and wake. The panel says which lamp is which.
- **The panel** (`words.rs › Words::panel`): a very small column at the top left (from 60 reading
  points down), one row a lamp, oldest first, the latest ten: its mark, a filled lamp in its own hue
  (large with a soft glow of its colour while its cloud is out, small while not; the front the largest) and where and when it was put in 10 pt
  italic ("9 Jun · 35°N 74°W"). Rows are 36 reading points tall (a thumb's reach, Motion's ask); `panel_bounds` holds each row's whole
  rect (from 6 points left of the mark to the end of the widest line) for Motion's probe.
- **Several clouds** (`maybes.rs`): laid in turns, an eighth of each in turn, so where they overlap
  their hues mix like two sands and where the currents part them each is its own colour; the ones
  behind the front a shade quieter. The hues are reordered so each next lamp is far round the wheel
  from the last (amber, sky, rose, mint, coral, lilac, lime, gold).
- **The rim**: maybes and floats fade by facing from 0.32 to 0.02 instead of clipping at 0.3; the
  field's kernels shrink and thin with the foreshortening and nothing is drawn past the globe's edge.
- The atlas's neighbour bleed at coarse mips (thin lines beside small glyphs) is masked at each cell's
  edge in `words.wgsl`.
- Studies: `studies/r6-*.png`, from `scenes/r6-*.json` by `bash scenes/render.sh <out>`.
- **The tap's rings** (`sea.wgsl`) gentler: two rings, not three, their light at 0.40 (0.65 before)
  and less by night, the shade under them lighter, gone sooner (`plans/ripple.json`,
  `studies/r6-ripple-before-after-phone.png`, before above, after below).
- **Pairs** (R6, after Truth's worlds: the i-th self of every lamp met the same sea): each cloud's
  partner is the one in front (and the front's, the one out before it); the maybes pass reads the
  partner's i-th self from a second instance slot over the same buffer (`maybes.rs › PARTNER`), so
  no storage binding. Close in, a pair still within ~25 km is one speck, the front lamp's hue ringed
  in its partner's, and the other cloud's self steps aside; by 160 km apart the ring is gone and each
  is its own lamp's speck. From afar the pair's hues mix in the field. So where two clouds diverge is
  exactly where the rings go (`studies/r6-pairs-gyre-before-after-phone.png`: quiet water, most
  still ringed; `r6-pairs-jet-…`: the Gulf Stream, few). The quad grows for the ring only close in,
  where it is drawn: the eight-cloud scrub's frame is unchanged within the software GPU's noise.

## R8: each lamp its own way

- **A lamp chosen as itself** (`LookFloat.followed`): its glass float stays lit and is ringed in its
  own colour (`floats.wgsl`, the held ring tinted by the hue), and its whole kept journey is drawn back
  to where it was put (`wake.wgsl › follow`: the beads a little larger, never fading below about a
  third), so two lamps followed together show their dance whole. A lamp chosen as its cloud is drawn
  as R6 has it (one speck among its maybes); a lamp not chosen keeps its float and the comet's tail of
  its last 45 days.
- **The panel** says which way, as quietly: as its cloud a larger lamp in a soft glow of its colour;
  as itself a small lamp in a ring of its colour; not chosen a small lamp alone. Over the globe the
  column lies on one quiet card of the page's tone (a halo per glyph lay as boxes at this size).
- Studies: `studies/r8-cloud-lamp-neither{,-night}-phone-half.png` (one of each, two followed),
  `studies/r8-two-lamps-followed-before-after-phone.png`; scenes `scenes/r8-*.json`.

## R9: the studio's first tool, a take and its clip

- **The clip** (`LookState.exporting`): the sea and its lamps, nothing of the screen's chrome (no
  panel, no date band, no credits page, no record control); the date alone, 15 points at the foot
  where it stands live, since time passing is a clip's story. Over the globe it is pale on a soft dark
  halo, as a film's subtitle (a halo of the sea's tone lay on the land as a sticker); on the page, the
  page's own ink (`Words::prepare_caption`).
- **The record control** (`Words::record_control`, Motion's gestures): top right, opposite the panel,
  its rect 44 reading points square at y 52 in `record_bounds`. At rest a small soft ring round a
  muted vermilion lamp; recording, the lamp lit and breathing and a ring of forty beads filling
  clockwise toward the take's longest (`studio::TAKE_MAX`), the seconds beside it in small italic.
  Never in a clip.
- Studies: `studies/r9-live-recording-vs-clip-phone-half.png` (the live screen at the take's end, and
  the clip's last frame, 9:16, its caption inside the window), `studies/r9-record-control-idle-recording-phone.png`; the plan is
  `adrift-check/plans/take-and-export.json`.
