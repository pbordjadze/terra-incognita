# The pack runtime (3 to 5): packs drawn by the globe, scripted in JavaScriptCore

A pack is data, pictures and a small script. The script runs in JavaScriptCore inside the app, in
the app's own process, with no web page and no web view: it decides, and the globe draws, in
Swift, in step with the camera. Everything a pack asks for belongs to its stage and goes when the
prototype ends. This page is the contract; `docs/playground.md` is the workflow.

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
- `runtime`: 3, or 4 if the pack uses anything marked *new in 4*, or 5 for anything *new in 5* (an
  app with an older runtime lists the pack as needing a new build).
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
| `terra.runtime` | 5 (4 on an app before October 29, 2026; 3 before October 28) |
| `terra.pack` | `{ id, revision, origin }` (origin `bundled`, `downloaded` or `preview`) |
| `terra.focus`, `terra.on('focus', fn)` | *new in 4:* true while this is the experience in front, the one the reader is with ([Many at once](#many-at-once-runtime-4)); the event is `{ front, by }` (by `reader` or `host`) when that changes |
| `terra.entry` | *new in 4:* how the reader came in: `{ by, lon, lat, zoom }`, by `doorway` (where the reader was: no flight to the anchor), `workbench`, `link` or `launch` |
| `terra.settings`, `terra.on('settings', fn)` | `{ reduceMotion, textScale, active }` |
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
it; the first set of an id fixes its place in the drawing order. Budgets: 100,000 vertices, 64
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
  and where the app keeps clear (the chip, the foot).
- `touchable: true`: a tap on it reaches `terra.on('touch')` with `target: { sprite: id }`. A trace
  resting between its showings can't be touched. Sprites don't make room for names: place them.
- Reduce Motion: sprites hold still at their first place and first frame; traces show steadily.
- Budget: 64 sprites, each look at most 24 frames.

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
```

- `touchables` (default): a one-finger drag that *begins on* a touchable feature is the pack's
  (`began`, `moved`…, `ended`); any other drag turns the globe. Taps on touchable things are the
  pack's; other taps choose countries (or are the pack's with `"taps": true`).
- `one-finger`: every one-finger drag on the globe is the pack's, and two fingers turn and zoom the
  globe. For close-up worlds you reach into. The scheme goes back to `touchables` when the pack ends.
- A drag's `moved` and `ended` carry the `target` it began on; find what is under the finger now
  yourself if it matters. No long press yet.

## The date and the words at the foot (since runtime 2)

| | |
| --- | --- |
| `terra.time.show({ unit, min, max, value, ticks, playable, rate, playing, carry, pace })` | the globe's date at the foot: `unit` `year` or `day` (days since 1970-01-01 UTC); `ticks` where the evidence is (a touch on the strip within a fingertip of one lands on it); `carry` starts from the date the last time-aware prototype left. A `day` date is also the globe's season: the ground's month and the clouds' follow it, and go back when it hides. *New in 5:* years reach back five billion either way, and before 10,000 BCE read as years ago, to two figures ("45 million years ago", "14,000 years ago"); `pace: [[date, seconds], …]` (dates and seconds both rising) makes the date pass unevenly: playback reaches each date that many seconds after the first, and the strip is laid out in those seconds, so an era given more of them has more of the strip; its first and last dates are the range (`min`, `max` and `rate` don't apply) |
| `terra.on('time', fn)` | `{ value, playing, by }`, by `reader`, `play` or `host` |
| `terra.time.set(v)`, `.play(bool)`, `.hide()` | move it (not echoed back), play or pause, hide |
| `terra.time.speed(f)` | *new in 5:* how fast playback runs, 1 as written (0.001–100): slower as the reader comes close, say, so what moves with the date walks rather than races. Not echoed; the strip and its date are unchanged |
| `terra.caption.set({ title, text })` | the words at the foot, in the globe's caption type: what the reader is looking at first, then why it matters |
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
open in Safari only when the reader taps them.

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
