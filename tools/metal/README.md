# Metal off the Mac

Terra's shaders (`App/TerraIncognita/**/*.metal`) compiled as C++ by clang on Linux: checked in a
second, drawn on the CPU as the device would draw them, and set beside their desktop twins. Before it,
a shader was first compiled, and first seen, at the end of a macOS run.

    python3 tools/metal/metal.py check                 # every shader, as the Mac would compile it
    python3 tools/metal/metal.py shot globe+ocean+clouds out.png --device ipad --at descent
    python3 tools/metal/metal.py twin clouds --at earth   # the Metal beside its web twin
    python3 tools/metal/tests/test_metal.py            # the stand-in's own tests (5 s)

## How

- `include/metal_stdlib` stands in for Metal's standard library. Its vectors are clang's extended
  vectors, the machinery Metal's own compiler is built on, so swizzles, splats and arithmetic behave
  as on the GPU and a mistake Metal refuses (a `float4` into a `float3`, `vec3`, GLSL's `mod` or
  `texture()`, a missing argument) is refused here too: `tests/test_metal.py` holds it to fourteen such
  mistakes and to a page of forms Metal accepts.
- `metal.py` rewrites the source a little on the way, keeping its lines, so an error names the
  `.metal` file's own line: unsuffixed floating literals become floats (as Metal takes them), a vector
  or matrix constructor becomes a call that converts values (clang's own would reinterpret bits), and
  a comparison among a vector's elements gives Metal's 1 for true (clang's is -1). It also writes an
  entry for each fragment and kernel from its bindings, so a binding Metal would refuse fails `check`.
- `runtime.cpp` runs a fragment as a GPU does: in quads, 2x2 pixels together, each pixel a coroutine,
  so `dfdx`, `dfdy` and `fwidth` are differences between neighbours at the same point of the program
  (fine: each row and column its own pair, which matches the twins' SwiftShader best; METAL_DERIVATIVES=coarse
  takes the quad's first row and column for all four). Textures are held as the device holds them (8-bit
  unorm as bytes, mipmaps as a blit encoder makes them) and sampled as Metal samples: filters,
  address modes, levels from `gradient2d` or the quad's own derivatives, `gather`, `read`, `write`.
  A kernel runs over its grid (the clouds' noise tile is made by `cloudsNoiseTile` itself).
- `layers.py` knows the app's three Metal layers: which map goes to which binding, and how GlobeView,
  CloudsLayer and OceanLayer fill their uniforms (mirrored: change them together). `shot` stacks
  layers as the stage composites them, each at its own scale; no names (GlobeView draws those apart).
- `twin` draws a layer's twin (the clouds' and the ocean's WebGL in headless Chromium, `twin.cjs`,
  which reads back the uniforms the twin's program was given and the pixels it wrote; the globe's
  `tools/globe/preview.py`), draws the Metal from the same uniforms, and reports how far apart they
  are: per 32-pixel block, how far the Metal leans from the twin (a colour, an opacity, a weight
  changed in one of the two leans its blocks one way; rounding at a threshold leans both ways and
  cancels) and how far apart in all (a width, a contrast). It also names any uniform the app's fill
  and the twin's give differently. Pictures in `build/metal/twin/`: the twin, the Metal, their
  difference.

The same places have the same names in `metal.py shot --at <name>` and in CI's `[shots]`
(`ci/fast.py`: the month 10, day 0.4 of the clouds' drift, the sea at 12 s), so a simulator's picture
and the stand-in's can be set side by side. Set so on October 28, 2026, the simulator's Metal and the
stand-in drew the same clouds, sea and ground at every place; the stand-in's globe sat about 40
points lower on the iPad, as it takes its room from `tools/globe/preview.py`'s `layout` (an estimate)
where the app measures its buttons and caption.

## What it can't vouch for

- **Derivatives apart.** Where the four pixels of a quad have parted ways (one returned, or skipped
  the line the others are on with `continue`) a derivative is undefined on a GPU. One that runs the
  quad in step (SwiftShader does; Apple's are believed to) gives the neighbour's value as it stood,
  sensible when it was computed before the branch; the stand-in can't see that value and takes the
  neighbours on the same line, or none. Such
  pixels are counted and their lines named (`parted: line 296 ...` in `shot`), and `twin` sets them
  aside. Clouds.metal takes two (lines 278 and 296, in the layers' loop after a `continue`): there the
  stand-in draws the cumulus with crisp, shaded rims the device doesn't.
- **Precision.** The CPU computes `exp`, `pow` and the trigonometry to the last bit; the GPU, under
  Metal's default fast math, and SwiftShader less so. Thresholded noise magnifies it: a twentieth of a
  pixel moves an eighth of the clouds' edges. `half` is computed as `float`.
- **The twin's sight.** `tests/twin_drift.py` changes each layer's Metal as a session might (a colour,
  an opacity, a width, a wind) and checks `twin` notices, at the whole Earth and at the descent: it
  does, all but two, kept in its list: one cloud layer's light a hundredth redder (under the clouds'
  own noise), and the land's relief made stronger (it lightens and darkens alike). The globe's twin
  has no mipmaps, so it is compared with the Metal's first level (`MTL_BASE_LEVEL`). Run it (two
  minutes, headless Chromium) when a layer's tolerances in `layers.py` change.
- **Not Metal's compiler.** Address spaces are erased and Metal's C++ restrictions (no recursion, no
  function pointers) aren't held to; `threadgroup` memory, atomics, SIMD-group functions and vertex
  stages aren't run. The Mac's build is still the last word: `[build-only]` compiles the app in about
  three minutes on a free runner.

## Adding to it

A shader that uses a builtin the stand-in lacks fails `check` with its name: add it to `metal_stdlib`
beside its kind, as Metal declares it (an overload for each type), and a case to `tests/test_metal.py`
if its value can be known. A new Metal layer gets a class in `layers.py` (its bindings and its fill,
mirroring its Swift) and, if it has a twin, a few drifts in `tests/twin_drift.py`.
