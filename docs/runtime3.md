# Runtime 3: packs drawn by the globe, scripted in JavaScriptCore

A pack is data, pictures and a small script. The script runs in JavaScriptCore inside the app, in
the app's own process, with no web page and no web view: it decides, and the globe draws, in
Swift, in step with the camera. Everything a pack asks for belongs to its stage and goes when the
prototype ends. This page is the contract; `docs/playground.md` is the workflow.

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
- `runtime`: 3. An app with an older runtime lists the pack as needing a new build.
- `presentation`: always `"globe"`. Every pack lives on the globe; there are no pack screens.
- `scripts`: run in order in one context, like script tags. The global object is also `window`
  and `globalThis`, so a data file may say `window.TIDE = {...}`.
- `anchor` (optional): where the globe flies when the pack starts (`zoom` 0.8–8; 1 is the whole
  globe).
- `doorway` (optional): how the pack shows on the globe while it is *not* running, so the atlas
  has its things on it: a place, a VoiceOver label and a `look` (as a sprite's look, below).
  Touching it starts the pack. Keep the whole-Earth view sparse: give it a `minZoom`, or make it a
  small trace (`every`/`for`).
- `taps`: true gives the pack every tap on the globe (its `touch` events with `target: null`)
  instead of choosing countries. Taps on its own touchable things reach it either way.
- `shelf`: `current` or `parked` (the workbench's Parked shelf; no doorway).

## The script's world

No DOM, no `fetch`, no network, no storage that lasts. What there is:

| | |
| --- | --- |
| `terra.runtime` | 3 |
| `terra.pack` | `{ id, revision, origin }` (origin `bundled`, `downloaded` or `preview`) |
| `terra.settings`, `terra.on('settings', fn)` | `{ reduceMotion, textScale, active }` |
| `terra.camera` | `{ lon, lat, zoom, cx, cy, radius, width, height }` now (points) |
| `terra.on('camera', fn)` | the camera, at most 20 times a second while it moves and once when it stops |
| `terra.project(lon, lat)` | `{ x, y, facing, visible }` on screen (facing 1 at the centre, < 0 behind) |
| `terra.unproject(x, y)` | `{ lon, lat }`, or null off the globe |
| `terra.flyTo(lon, lat, zoom, seconds)` | move the globe |
| `terra.read(path)`, `terra.readJSON(path)` | a file of the pack, as text or parsed (null if missing) |
| `setTimeout`, `setInterval`, `clearTimeout`, `clearInterval` | as in a browser |
| `console.log/warn/error`, `terra.log(...)` | the workbench's log and the Xcode console |
| `terra.on('open', fn)` | the chip's **Open**; registering it shows Open |
| `terra.on('close', fn)` | the prototype is ending |
| `terra.close()` | end the prototype (back to the atlas) |

Scripts react to events and timers. They never draw and never run every frame: the globe animates
what they set up. An error is logged and the pack keeps running.

## Geography drawn by the globe: `terra.geo` (since runtime 2)

`terra.geo.set(id, features, options)`, `terra.geo.remove(id)`, `terra.geo.clear()`. Features are
GeoJSON (a FeatureCollection, an array of Features, or bare geometries; positions `[lon, lat]`).

`options.style`: `border`, `area`, `ghost` (dashed outline), `track` (fine dashes), `dot`, `ring`,
`label` (`properties.label`, on a halo; earlier ones win room), `symbol` (`options.image`, a picture
of the pack), `sounding` (a line hanging `properties.length` points, with `properties.label`).
Also `color`, `minZoom`/`maxZoom`, `font` (`regular`, `italic`, `caps`), `size`,
`width`/`height`, `offset: [dx, dy]`, and, new in 3, **`touchable: true`**: its features take touches
(points and labels within 22 points, lines within 14, areas inside). Setting an id again replaces
it; the first set of an id fixes its place in the drawing order. Budgets: 100,000 vertices, 64
layers, 400 named or pictured places a layer.

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
- `looks`: the first look whose `minZoom`–`maxZoom` holds the zoom is drawn, so one thing can be a
  trace from afar and itself up close. A look is `image` (one picture) or `frames` (pictures shown in
  turn at `fps`), `size: [w, h]` in points at the reading scale, and optionally `every` and `for`
  (seconds: it shows for `for` seconds every `every`, a splash now and then) and `face: true` (the
  picture, drawn facing east, turns to face the way it moves).
- `label`: its VoiceOver name (needed when `touchable`). It is hidden behind the globe, off screen,
  and where the app keeps clear (the chip, the foot).
- `touchable: true`: a tap on it reaches `terra.on('touch')` with `target: { sprite: id }`.
- Reduce Motion: sprites hold still at their first place and first frame; traces show steadily.
- Budget: 64 sprites, each look at most 24 frames.

## Touch in the world (new in 3)

```js
terra.on('touch', function (t) {
  // t.phase: 'tap', or 'began', 'moved', 'ended' for a drag
  // t.target: { layer: id, index: n } (a touchable geography feature), { sprite: id }, or null
  // t.x, t.y (points), t.lon, t.lat (null off the globe)
})
terra.touch.scheme('touchables')   // the default
terra.touch.scheme('one-finger')
```

- `touchables` (default): a one-finger drag that *begins on* a touchable feature is the pack's
  (`began`, `moved`…, `ended`); any other drag turns the globe. Taps on touchable things are the
  pack's; other taps choose countries (or are the pack's with `"taps": true`).
- `one-finger`: every one-finger drag on the globe is the pack's, and two fingers turn and zoom the
  globe. For close-up worlds you reach into. The scheme goes back to `touchables` when the pack ends.

## The date and the words at the foot (since runtime 2)

| | |
| --- | --- |
| `terra.time.show({ unit, min, max, value, ticks, playable, rate, playing, carry })` | the globe's date at the foot: `unit` `year` or `day` (days since 1970-01-01 UTC); `ticks` where the evidence is; `carry` starts from the date the last time-aware prototype left |
| `terra.on('time', fn)` | `{ value, playing, by }`, by `reader`, `play` or `host` |
| `terra.time.set(v)`, `.play(bool)`, `.hide()` | move it (not echoed back), play or pause, hide |
| `terra.caption.set({ title, text })` | the words at the foot, in the globe's caption type: what the reader is looking at first, then why it matters |
| `terra.globe.set({ historical, still })` | hide today's borders and names (coasts stay); stop the idle turn |

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

## Many at once

Every pack and tool assumes others may share the stage. Today the workbench runs one prototype at a
time, and the atlas shows every pack's doorway on the globe at once; a pack must still never assume
it owns what it didn't ask for.

## The desktop preview

`node tools/playground/serve.cjs` runs any pack in a browser over a stand-in globe with the same
`terra.js`, drawing geography, pictures, sprites, the foot and the panel as the app does (close, not
pixel-identical), and turning mouse drags and clicks into touch. `node tools/playground/check.cjs
<id>` runs it headless, fails on script errors, and saves screenshots: an agent can see its work
without a Mac.
