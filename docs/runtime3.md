# The pack runtime (3 to 5): packs drawn by the globe, scripted in JavaScriptCore

A pack is data, pictures and a small script. The script runs in JavaScriptCore inside the app, in
the app's own process, with no web page and no web view: it decides, and the globe draws, in
Swift, in step with the camera. Everything a pack asks for belongs to its stage and goes when the
prototype ends. This page is the contract; `docs/playground.md` is the workflow.
Every tool, here and in `tools/`, with the packs that use it and its limits: [`docs/tools.md`](tools.md).

**Runtime 4** (October 28, 2026) is the same contract with the stage shared: several experiences
run on the one globe at once, each entered and left by touch, and every doorway has a life of its
own while its experience sleeps ([Many at once](#many-at-once-runtime-4)). What it adds is marked
*new in 4*; a pack that uses none of it keeps `"runtime": 3` and runs on either app. A pack that
declares `"runtime": 4` needs an app that hosts it (an older one lists it as needing a new build).

**Runtime 5** (October 29, 2026) lets the globe's date move things: a sprite on a **dated way** is where
the date puts it, so it walks as the date plays and stands where the reader drags it
([Sprites](#sprites-things-that-live-and-move-on-the-globe-new-in-3)); the date may pass at an uneven
**pace** (deep time fast, the present slowly), run at a **speed** the pack sets (slower as the reader
comes close), and reach back to the Earth's beginning, read as years ago
([The date](#the-date-and-the-words-at-the-foot-since-runtime-2)). Marked *new in 5*.

**Runtime 6** (October 29, 2026; painting-company row 183) adds **sprite sheets**: a look's frames may be
rectangles of one picture, so a pack's many frames don't each cost a file
([Sprites](#sprites-things-that-live-and-move-on-the-globe-new-in-3)). Marked *new in 6*.

## The pack

```
Prototypes/<id>/
  prototype.json   who it is, where it lives, its scripts
  main.js          its script (and any others it lists)
  data.js, *.png   its data and pictures
  SOURCES.md       where its facts and pictures come from
```

### `prototype.json`

```json
{
  "id": "tide-pool",
  "title": "Tide pool",
  "summary": "One line for the workbench.",
  "runtime": 3,
  "presentation": "globe",
  "scripts": ["data.js", "main.js"],
  "anchor": { "lon": -4.5, "lat": 48.4, "zoom": 3 },
  "doorway": { "lon": -4.5, "lat": 48.4, "label": "A tide pool",
               "look": { "image": "pool.png", "size": [28, 28], "minZoom": 1.5 } },
  "taps": false,
  "shelf": "current"
}
```

- `id`: lower-case letters, digits and hyphens, 2–40 characters, the folder's name; never reused.
- `runtime`: 3, or 4 if the pack uses anything marked *new in 4*, 5 for anything *new in 5*, 6 for
  anything *new in 6* (an app with an older runtime lists the pack as needing a new build).
- `presentation`: always `"globe"`. Every pack lives on the globe; there are no pack screens.
- `scripts`: run in order in one context, like script tags. The global object is also `window`
  and `globalThis`, so a data file may say `window.TIDE = {...}`.
- `anchor` (optional): where the globe flies when the pack starts (`zoom` 0.8–8; 1 is the whole
  globe).
- `doorway` (optional): how the pack shows on the globe while it is *not* running, so the atlas
  has its things on it: a place, a VoiceOver label and a `look` (as a sprite's look, below).
  Touching it starts the pack. Keep the whole-Earth view sparse: give it a `minZoom`, or make it a
  small trace (`every`/`for`). *New in 4:* a doorway may have a small life of its own, so it can be
  seen from afar and draws the reader closer: `looks` (a list, as a sprite's: the first whose zoom
  range holds is drawn, so it can be a glint now and then from far out and itself up close),
  `path` (as a sprite's: it stirs along a short way, `loop: 'back'` to and fro) and
  `breathes: true` (a slow breath). Reduce Motion holds it still at its first place and frame, and a
  trace shows steadily. Entered by its doorway, a pack starts where the reader is: no flight to its
  `anchor` (`terra.entry` says how it was entered).

  ```json
  "doorway": { "lon": 24.5, "lat": 39.55, "label": "The sea between: an engraved chart of the Aegean",
               "looks": [ { "image": "glint.png", "size": [11, 11], "maxZoom": 1.5, "every": 9, "for": 1.2 },
                          { "frames": ["skiff-1.png", "skiff-2.png"], "fps": 1.4, "size": [18, 21], "minZoom": 1.5 } ],
               "path": { "coordinates": [[24.4, 39.5], [24.7, 39.7]], "seconds": 120, "loop": "back" } }
  ```
- `taps`: true gives the pack every tap on the globe (its `touch` events with `target: null`)
  instead of choosing countries. Taps on its own touchable things reach it either way.
- `shelf`: `current` or `parked` (the workbench's Parked shelf; no doorway).

## The script's world

No DOM, no `fetch`, no network, no storage that lasts. What there is:

| | |
| --- | --- |
| `terra.runtime` | 6 (5 on an app before sprite sheets; 4 before October 29, 2026; 3 before October 28; `flow` and the `hold` scheme since October 29) |
| `terra.pack` | `{ id, revision, origin }` (origin `bundled`, `downloaded` or `preview`) |
| `terra.focus`, `terra.on('focus', fn)` | *new in 4:* true while this is the experience in front, the one the reader is with ([Many at once](#many-at-once-runtime-4)); the event is `{ front, by }` (by `reader` or `host`) when that changes |
| `terra.entry` | *new in 4:* how the reader came in: `{ by, lon, lat, zoom }`, by `doorway` (where the reader was: no flight to the anchor), `workbench`, `link` or `launch` |
| `terra.settings`, `terra.on('settings', fn)` | `{ reduceMotion, textScale, active, wikipedia }` (`wikipedia`, since October 29, 2026: the reader wants [Wikipedia's words](#wikipedias-words)) |
| `terra.camera` | `{ lon, lat, zoom, cx, cy, radius, width, height }` now (points) |
| `terra.on('camera', fn)` | the camera, at most 20 times a second while it moves and once when it stops |
| `terra.project(lon, lat)` | `{ x, y, facing, visible }` on screen (facing 1 at the centre, < 0 behind) |
| `terra.unproject(x, y)` | `{ lon, lat }`, or null off the globe |
| `terra.flyTo(lon, lat, zoom, seconds)` | move the globe |
| `terra.read(path)`, `terra.readJSON(path)` | a file of the pack, as text or parsed (null if missing) |
| `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval` | as in a browser |
| `performance.now()`, `requestAnimationFrame` | a clock, and frames on the host's timers: for a short eased change (a line let down), not for animating the globe |
| `console.log/warn/error`, `terra.log(...)` | the workbench's log and the Xcode console |
| `terra.on('open', fn)` | the chip's **Open**; registering it shows Open |
| `terra.on('close', fn)` | the prototype is ending |
| `terra.close()` | end the prototype (back to the atlas) |

Scripts react to events and timers. They never draw and never run every frame: the globe animates
what they set up. An error is logged and the pack keeps running. JavaScriptCore in an app runs
without a JIT, so do heavy preparation (parsing, indexing, simplifying) in the pack's build step,
not at start-up.

## Shared script

Small helpers every pack would otherwise write again live in one file, `Prototypes/_shared/shared.js`,
the global `Shared`. A pack that uses it carries a copy named `shared.js`, first in its `scripts` (a pack
reaches nothing outside its folder, so the copy ships inside it and needs no new app or runtime):
`python3 tools/playground/playground.py share` writes the copies, and `validate` fails a copy that differs
from the source, so it is changed in one place. It changes only compatibly (each pack that carries it
takes the change with its next revision); `node tools/playground/shared_check.cjs` holds it to the
helpers it replaced, bit for bit.

```js
Shared.along([35.2, 31.8], [31.2, 30.0], 0.25)   // the place a quarter of the way along the great circle
Shared.arc([35.2, 31.8], [31.2, 30.0])           // the angle between two places, radians
Shared.unit([35.2, 31.8])                        // a place's unit vector, the runtime's axes
Shared.frames('dhow', 4)                         // ['dhow-1.png', …, 'dhow-4.png'], the art drawer's names
```

## Geography drawn by the globe: `terra.geo` (since runtime 2)

`terra.geo.set(id, features, options)`, `terra.geo.remove(id)`, `terra.geo.clear()`. Features are
GeoJSON (a FeatureCollection, an array of Features, or bare geometries; positions `[lon, lat]`).

`options.style`: `border`, `area`, `ghost` (dashed outline), `track` (fine dashes), `dot`, `ring`,
`label` (`properties.label`, on a halo; earlier ones win room), `symbol` (`options.image`, a picture
of the pack), `sounding` (a line hanging `properties.length` points, with `properties.label`).
Also `color`, `minZoom`/`maxZoom`, `font` (`regular`, `italic`, `caps`), `size`,
`width`/`height` (a symbol's picture), `offset: [dx, dy]`, and, new in 3, `lineWidth` (reading points,
for `border`, `area`, `ghost` and `track`: a river's order, a traced way) and **`touchable: true`**: its features take touches
(points and labels within 22 points, lines within 14, areas inside). Setting an id again replaces
it; the first set of an id fixes its place in the drawing order.

*New in 4 (October 29):* **`flow`**, light running along a `border`, `ghost` or `track` layer's
lines the way their coordinates go (water to the sea, a ship's way): beads `dash` reading points
long every `dash + gap`, moving `speed` reading points a second, in `color` (white by default) and
`width` (the line's by default). `flow: true` takes the defaults (`{ speed: 24, dash: 2, gap: 22 }`).
The globe moves the beads itself (Core Animation), so the script sets the layer once and spends
nothing a frame; Reduce Motion shows the line without them. The light is drawn again every frame, so
keep a flowing layer to a few thousand vertices. An app before October 29 draws the line alone.

```js
terra.geo.set('way-down', [{ type: 'LineString', coordinates: [[-62, -3.2], [-60, -3.1], [-58.5, -3.3]] }],
              { style: 'border', color: '#0B3D66', lineWidth: 2.6,
                flow: { speed: 30, dash: 2, gap: 20, color: '#E8F6FFE6' } })
``` Budgets: 100,000 vertices, 64
layers, 400 named or pictured places a layer. A touchable place with a name (`properties.label`)
is a VoiceOver button that touches it, so it can be reached without seeing or aiming at it.

## Pictures lying on the globe: `terra.images` (new in 3)

```js
terra.images.set('leaf', { image: 'engraving.png', bounds: [15, 31, 35, 45], opacity: 1,
                           minZoom: 2, maxZoom: 8, fade: 0.6 })
terra.images.remove('leaf'); terra.images.clear()
```

A picture in longitude and latitude (`bounds`: west, south, east, north; each pixel column the same
span of longitude and each row the same span of latitude) drawn on the globe, curving with it, in
step with the camera, under the geography and sprites. `fade` is the zoom range over which it fades
in above `minZoom`. At most 8 pictures, each at most 4096 pixels a side.

## Sprites: things that live and move on the globe (new in 3)

```js
terra.sprites.set('whale', {
  path: { coordinates: [[21.0, 35.6], [22.4, 35.2], [23.8, 35.4]], seconds: 60, loop: 'back' },
  looks: [
    { image: 'splash.png', size: [16, 16], maxZoom: 3, every: 11, for: 1.4 },
    { frames: ['whale-1.png', 'whale-2.png', 'whale-3.png', 'whale-2.png'], fps: 5,
      size: [46, 20], minZoom: 3, face: true }
  ],
  label: 'A sperm whale, drawn: not a sighting',
  touchable: true
})
terra.sprites.remove('whale'); terra.sprites.clear()
```

- Where: `at: [lon, lat]`, or `path: { coordinates, seconds, loop }`. Along a path the sprite moves
  on great circles between the points, the whole path in `seconds`; `loop` is `true` (starts again),
  `'back'` (to and fro) or `false` (stops at the end). Setting it again with the same path keeps its
  place along it.
- *New in 5:* a **dated way**, `path: { coordinates, dates }`, one date for each place, rising (in the
  unit of the pack's date, `terra.time`; two places may share a date, a jump). The sprite is where the
  stage's date puts it, on the great circle between the two places whose dates the date lies between,
  so it moves as the date plays and stands where the reader drags it; `seconds` and `loop` don't
  apply. Outside its first and last dates it isn't drawn (a life begun or ended), fading over half a
  second as the date crosses them, and with no date shown it isn't drawn at all. A `face: true` look
  turns the way it moved. Under Reduce Motion nothing plays by itself, but it still follows the date
  the reader sets. The globe reads the date every frame, so a sprite moves smoothly between
  playback's steps; a script never moves it frame by frame. Setting it again with the same way (a
  change of looks: walking while the date plays, grazing while it rests) keeps its place.

  ```js
  terra.sprites.set('camel-1', {
    path: { coordinates: [[-152, 65], [-169, 65.9], [-176, 66]], dates: [-7.4e6, -7.0e6, -6.88e6] },
    looks: [{ frames: ['walk-1.png', 'walk-2.png', 'walk-3.png', 'walk-4.png'], fps: 5, size: [22, 15], face: true }],
    label: 'Paracamelus, crossing Beringia', touchable: true
  })
  ```
- `looks`: the first look whose `minZoom`–`maxZoom` holds the zoom is drawn, so one thing can be a
  trace from afar and itself up close. A look is `image` (one picture) or `frames` (pictures shown in
  turn at `fps`), `size: [w, h]` in points at the reading scale (at most 400 a side; 200 before runtime 5), and optionally `every` and `for`
  (seconds: it shows for `for` seconds every `every`, a splash now and then; *new in 4:* a showing
  fades up and down over its first and last 0.3 s, so a glint is a glint and not a blink) and
  `face: true` (the picture, drawn facing east, is mirrored when it moves west).
- `label`: its VoiceOver name (needed when `touchable`). It is hidden behind the globe, off screen,
  and where the app keeps clear (the chip, the foot). *New in 5:* a sprite drawn larger than 60 points
  (an animal seen close) shows while any of it is on screen and its middle is clear of the chip and
  the foot, so it doesn't vanish as it walks to the edge.
- `touchable: true`: a tap on it reaches `terra.on('touch')` with `target: { sprite: id }`. A trace
  resting between its showings can't be touched. Sprites don't make room for names: place them.
- *New in 6:* a **sprite sheet**: a frame (or a look's `image`, or a doorway's) may be a rectangle of a
  bigger picture, named `'<sheet>.png#x,y,width,height'` in the sheet's pixels, so a pack with many
  frames (the herds' fifteen animals, each in four looks and two poses) spends one file a sheet rather
  than one a frame. The app decodes a sheet once and cuts its frames without copying them; the preview
  and `playground.py validate` read them too (the rectangle must lie inside the sheet). Sheets are
  written by the one art drawer, never by hand: a picture of an SVG sheet given `sheet: 'art/x.png'`
  goes into that sheet, pixel for pixel, and the sheet's `done(names)` hears each frame's name to write
  into the pack's index (`tools/packart/render.cjs`; `tools/herds/draw.cjs` is the example).

  ```js
  // art.js, written by the drawer: the frames of one animal's walk, in its kind's sheet
  looks: [{ frames: ['art/early.png#0,0,376,335', 'art/early.png#378,0,376,335'], fps: 6, size: [125.2, 111.4], face: true }]
  ```
- Reduce Motion: sprites hold still at their first place and first frame; traces show steadily.
- Budget: 64 sprites, each look at most 24 frames; a sheet at most 4096 pixels a side (the drawer's limit).

## Touch in the world (new in 3)

```js
terra.on('touch', function (t) {
  // t.phase: 'tap', or 'began', 'moved', 'ended' for a drag
  // t.target: { layer: id, index: n } (a touchable geography feature), { sprite: id }, or null
  // t.x, t.y (points), t.lon, t.lat (null off the globe)
})
terra.on('tap', fn)                // the same, taps alone
terra.touch.scheme('touchables')   // the default
terra.touch.scheme('one-finger')
terra.touch.scheme('hold')         // new in 4 (October 29)
```

- `touchables` (default): a one-finger drag that *begins on* a touchable feature is the pack's
  (`began`, `moved`…, `ended`); any other drag turns the globe. Taps on touchable things are the
  pack's; other taps choose countries (or are the pack's with `"taps": true`).
- `one-finger`: every one-finger drag on the globe is the pack's, and two fingers turn and zoom the
  globe. For close-up worlds you reach into. The scheme goes back to `touchables` when the pack ends.
- `hold` (*new in 4, October 29*): every one-finger drag turns the globe, wherever it begins, and a
  finger that rests a moment first (0.3 s, straying less than 8 points) is the pack's: `began` when
  the rest is felt (a light tap of the phone; the finger has not moved yet), then `moved`…, `ended`
  when it lifts, with `target: null` (find what is under the finger yourself). Taps are as in
  `touchables`. For a pack whose things lie everywhere (a river network): a pan is never taken for
  the pack's drag, nor a drag for a pan. The preview holds a pointer the same 0.3 s. An app before
  October 29 treats it as `touchables`.
- A drag's `moved` and `ended` carry the `target` it began on; find what is under the finger now
  yourself if it matters. No long press yet.

## The date and the words at the foot (since runtime 2)

| | |
| --- | --- |
| `terra.time.show({ unit, min, max, value, ticks, playable, rate, playing, carry, pace })` | the globe's date at the foot: `unit` `year` or `day` (days since 1970-01-01 UTC); `ticks` where the evidence is (a touch on the strip within a fingertip of one lands on it); `carry` starts from the date the last time-aware prototype left. A `day` date is also the globe's season: the ground's month and the clouds' follow it, and go back when it hides. *New in 5:* years reach back five billion either way, and before 10,000 BCE read as years ago, to two figures ("45 million years ago", "14,000 years ago"); `pace: [[date, seconds], …]` (dates and seconds both rising) makes the date pass unevenly: playback reaches each date that many seconds after the first, and the strip is laid out in those seconds, so an era given more of them has more of the strip; its first and last dates are the range (`min`, `max` and `rate` don't apply) |
| `terra.on('time', fn)` | `{ value, playing, by }`, by `reader`, `play` or `host` |
| `terra.time.set(v)`, `.play(bool)`, `.hide()` | move it (not echoed back), play or pause, hide |
| `terra.time.speed(f)` | *new in 5:* how fast playback runs, 1 as written (0.001–100): slower as the reader comes close, say, so what moves with the date walks rather than races. Not echoed; the strip and its date are unchanged |
| `terra.caption.set({ title, text, wikipedia })` | the words at the foot, in the globe's caption type: what the reader is looking at first, then why it matters; `wikipedia` (optional) names the article on it ([Wikipedia's words](#wikipedias-words)) |
| `terra.globe.set({ historical, still, maxZoom })` | hide today's borders and names (coasts stay); stop the idle turn; let the reader zoom deeper than 8 (up to 32) for the pack's close views, put back when it ends |

## The panel: longer words and sources (new in 3)

```js
terra.panel.show({
  title: 'Santorini',
  text: 'What you are looking at, first. Then why it is interesting.',
  sections: [{ title: 'The caldera', text: '…' }],
  sources: [{ title: 'Smithsonian Global Volcanism Program, volcano 212040',
              url: 'https://volcano.si.edu/volcano.cfm?vn=212040', note: 'description and captions' }]
})
terra.panel.hide()
terra.on('panel', function (p) { /* { open: false } when the reader closes it */ })
```

A native sheet over the globe (half height, can be pulled up), in the app's type, with the globe
still visible and live above it. Plain text only (paragraphs split on blank lines). Source links
open in Safari only when the reader taps them. The panel, and each section, may name the Wikipedia
article on its subject (`wikipedia: 'Santorini caldera'`): see below.

## Wikipedia's words

The app has a switch beside the clouds' (a W): on, every paragraph the atlas wrote itself gives way to
Wikipedia's own words on the same subject, the article's first sentences as they stand with a link to
it, so what is left to read is Wikipedia's. A pack takes part by naming its subjects: a `wikipedia`
key on `terra.caption.set`, on `terra.panel.show` and on each of its `sections`, with the article's
title. With the switch on, the host shows the article's first sentence at the foot (and "Wikipedia ↗"
under it) and the article's card where the panel's paragraphs were; a text with no key stays only when
it is a tip (one sentence under 120 characters), and a section with neither goes, its heading with
it; the sources go too, so only Wikipedia's words are left to read.

The words are bundled, never fetched by the app: `python3 tools/wikipedia/build.py --pack <id>` reads
the keys a pack names (and any listed for it in `tools/wikipedia/articles.txt`, as "Toward the Amazon
system" lists the systems' rivers) and writes the pack's `wikipedia.json`, the articles' first
paragraphs with their URLs and revisions, under Wikipedia's licence (CC BY-SA 4.0, shown on every
card). A pack's own articles are laid over the app's index (every country, the native experiences'
subjects), so a downloaded revision can name what the app never heard of; a key with no article is a
build failure (`ci/check_wikipedia.py`), not a quiet gap.

`terra.settings.wikipedia` says the switch is on, and `terra.on('settings')` hears it change: a line
that is data (a depth sounded, a station's reading) should then be sent without the atlas's
instruction after it, so it stays within a tip's length and is kept. The preview mirrors all of it
(the checkbox under Reader settings, `?wikipedia=1`, `check.cjs --wikipedia`).

## Many at once (runtime 4)

The globe is one world. Every current experience's doorway is on it at once, each alive in its own
small way, and several experiences run on it together: the reader touches a doorway and that
experience starts beside the ones already running, with nothing reset; touches a sprite or a
touchable feature of another and that one comes to the **front**; touches ✕ on the chip and the
front one ends, leaving the others as they were. Each running experience has a stage of its own,
and ending one takes away exactly what it made (the workbench's Live counts say so). Every pack and
tool assumes others share the stage; the host keeps these rules so none can take it from the reader:

- **The front experience** is the one the reader is with: the last started, or the last touched (a
  tap or drag on its things brings it to the front; so does its row in the workbench). The chip
  names it (and says how many others run), its date and caption are the foot, its gesture scheme
  is the globe's, its `historical` hides today's borders, and its `flyTo` moves the globe. The
  others run on in the **background**: their sprites, geography and pictures stay on the globe,
  their timers tick, they hear the camera, and they keep their date and caption for when they are
  in front again. A script hears the change as `terra.on('focus')` and can read `terra.focus`.
- **What a background experience asks for:** `flyTo` is dropped (logged once), `panel.show` is
  dropped unless it answers a touch of its own, `touch.scheme` and `historical` wait until it is in
  front, and its date is paused (it may `time.play` again when it hears it is in front).
  `still` holds while any experience asks for it; `maxZoom` is the deepest any asks for.
- **The reader's hands win.** A flight asked for while the reader's fingers are on the globe, or
  within a second of their last turn or pinch, is dropped. Two fingers always turn and zoom the
  globe, in any scheme. A touch on the globe ends any flight. ✕ is always there.
- **A lens** (a native sequence that takes the screen: the camel's, the whale's) is an excursion:
  while it is open, every other experience's things are hidden with the names, and closing it puts
  them back.
- **Budget:** at most 6 experiences run at once; starting a seventh ends the one the reader left
  longest ago, never the front one. Each stage keeps its own budgets (sprites, layers, pictures).

A pack needs none of this to run beside others; what it may use:

```js
if (terra.entry.by === 'doorway') { /* the reader is here already: no flight to the anchor */ }
terra.on('focus', function (f) {
  if (f.front) terra.time.play(true)       // back in front: the clock may run again
  else terra.caption.set({ title: 'The voyage', text: 'Paused while you are elsewhere.' })
})
```
- **The preview** (`serve.cjs`) mirrors all of it: `/atlas` is every current pack's doorway on the
  stand-in globe, each alive, and `?with=<id>,<id>` runs several together; `check.cjs --living`
  checks that two packs run side by side, by touch, and that ending one leaves the other whole.

## The desktop preview

`node tools/playground/serve.cjs` runs any pack in a browser over a stand-in globe with the same
`terra.js`, drawing geography, pictures, sprites, the foot and the panel as the app does (close, not
pixel-identical), and turning mouse drags and clicks into touch. `node tools/playground/check.cjs
<id>` runs it headless, fails on script errors, and saves screenshots: an agent can see its work
without a Mac.
