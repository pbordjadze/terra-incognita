# The playground

The app opens on the Wikipedia globe and nothing else. Everything being tried lives in the
**workbench** (the small grid button, top right): pick one, it starts on the globe or on its own
screen, and ending it leaves the plain globe exactly as it was. One prototype is active at a time;
its markers, overlays, screens and web views are made when it starts and released when it ends.
That is a workbench rule for comparing things cleanly, not a decision about the finished atlas.

- [Kinds of prototype](#kinds-of-prototype)
- [Make a web prototype](#make-a-web-prototype) (no app build: preview, test, publish, see it on the phone)
- [Make a native prototype](#make-a-native-prototype) (Swift or Metal: needs an IPA)
- [Update, compare, park, remove](#update-compare-park-remove)
- [The contract](#the-contract): `prototype.json`, `terra.js` (runtime 1), the catalog
- [Remote delivery](#remote-delivery): sources, integrity, limits, privacy, R2 set-up
- [What needs a new IPA](#what-needs-a-new-ipa)

## Kinds of prototype

| Kind | Lives in | Starts | Needs an IPA to change |
| --- | --- | --- | --- |
| Native, on the globe | `App/TerraIncognita/<Name>/` + an entry in `Playground/Catalog.swift` | markers on the globe; a marker opens its screen | yes |
| Native, own screen | same | its screen over the paused globe | yes |
| Web pack, bundled | `Prototypes/<id>/` (bundled in the app) | per its `prototype.json` | no, once published as a remote revision |
| Web pack, remote | a source's `catalog.json` (Cloudflare R2) | the same, downloaded on demand | no |
| Parked | the Persian globe style, the films | from the workbench's Parked shelf | yes |

## Make a web prototype

```sh
python3 tools/playground/playground.py new tide-pool --globe     # or --screen; scaffolds Prototypes/tide-pool/
node tools/playground/serve.cjs                                   # http://127.0.0.1:8790/ : every prototype, over a stand-in globe
node tools/playground/check.cjs tide-pool                         # headless Chromium: loads it, fails on errors, screenshots
python3 tools/playground/playground.py validate                   # the contract, sizes, offline boundary (CI runs it too)
```

Then commit and push. `packs.yml` validates and, when R2 is configured, publishes changed packs
as new revisions (a Linux job; no macOS runner). On the phone: open the workbench, it checks its
sources, the new revision appears; tap it. A link `<PACKS_PUBLIC_URL>/open/<id>.html` opens it
straight from Safari or a message.

A pack is plain HTML, CSS, JavaScript and media, with everything it needs inside its folder. It
cannot reach the network (the host blocks every request outside the pack), so data is bundled in
the pack. Keep the first-reader test in mind: someone who taps this first, knowing nothing, should
be able to tell what they are looking at.

## Make a native prototype

1. Put the code in its own folder, `App/TerraIncognita/<Name>/` (the project picks up new files).
2. Add one entry in `App/TerraIncognita/Playground/Catalog.swift`: an id, a title, a one-line
   summary, an optional anchor, and what `start` does with the stage (place markers, present a
   screen). Existing entries are examples of each kind.
3. Close its screen with `@Environment(\.dismiss)`; stop timers and display links when its views
   go away. The workbench's live counts show anything left behind.
4. Push with `[build-only]` in the commit message for a quick compile; a full push runs the
   interaction tests; `main` publishes the IPA to SideStore.

## Update, compare, park, remove

- **Update** a web pack: change its files and push; the publisher gives it the next revision only
  if its contents changed. The workbench marks it *update available*, downloads it on the next tap
  and keeps the previous revision: **Use previous revision** in its details switches back.
- **Compare**: the active prototype's chip has an ✕ for the plain globe and the workbench's first
  row starts the last prototype again; two variants are two packs (`tide-pool`, `tide-pool-b`).
- **Park**: `"shelf": "parked"` in `prototype.json` (or `shelf: .parked` in Swift) moves it to the
  Parked shelf, still one tap away.
- **Remove**: delete the folder or entry. A published pack is withdrawn with
  `playground.py retire <id>`; its files stay in the bucket, so the retirement is reversible.

## The contract

### `Prototypes/<id>/prototype.json`

```json
{
  "id": "tide-pool",
  "title": "Tide pool",
  "summary": "One line for the workbench.",
  "runtime": 1,
  "presentation": "globe",
  "entry": "index.html",
  "anchor": { "lon": -4.5, "lat": 48.4, "zoom": 3 },
  "markers": [
    { "id": "pool", "lon": -4.5, "lat": 48.4, "image": "pool.png", "width": 44, "height": 44,
      "label": "A tide pool", "minZoom": 2, "opens": true }
  ],
  "overlay": "overlay.html",
  "taps": false,
  "shelf": "current",
  "background": "#F6F2E7"
}
```

- `id`: lower-case letters, digits and hyphens, 2–40 characters, the folder's name; never reused.
- `runtime`: the `terra.js` version it needs (1). An app with an older runtime lists it as
  needing a new build instead of opening it.
- `presentation`: `screen` (its own full-screen page over the paused globe; ends when it closes)
  or `globe` (lives on the globe while active: markers, an optional overlay; ends from the chip).
- `entry`: the screen's page (default `index.html`). A globe prototype opens it from a marker with
  `"opens": true`, or with `terra.open()`; it is optional for a globe prototype.
- `anchor`: where the globe flies when it starts (`zoom` 0.8–8, 1 = the whole globe).
- `markers` (globe): pictures from the pack drawn by the app at places on the globe, in points;
  hidden behind the limb and outside `minZoom`–`maxZoom`; accessible by `label`; names keep clear.
- `overlay` (globe): a transparent page laid over the globe, told the camera every frame it moves,
  that never takes touches. With `"taps": true` it receives the globe's taps instead of country
  selection.
- `shelf`: `current` or `parked`. `background`: the screen's colour while it loads.

### `terra.js`, runtime 1

The host injects `window.terra` before the page's scripts (`Prototypes/_runtime/terra.js`; the
same file in the app and the preview):

| | |
| --- | --- |
| `terra.pack`, `terra.mode`, `terra.params` | `{ id, revision, origin }`; `'screen'` or `'overlay'`; e.g. `{ marker: 'pool' }` |
| `terra.settings`, `terra.on('settings', fn)` | `{ reduceMotion, textScale, active }`; also CSS `var(--terra-text-scale)` and `[data-reduce-motion]` on `<html>` |
| `terra.close()` | back to the globe (a screen) or end the prototype (an overlay) |
| `terra.open(params)` | an overlay opens the pack's screen |
| `terra.on('camera', fn)`, `terra.camera` | overlay: `{ lon, lat, zoom, cx, cy, radius, width, height }` in CSS px |
| `terra.project(lon, lat)`, `terra.unproject(x, y)` | the globe's orthographic projection, both ways |
| `terra.on('tap', fn)` | overlay with `taps`: `{ x, y, lon, lat }` (`lon`/`lat` null off the globe) |
| `terra.flyTo(lon, lat, zoom, seconds)` | overlay: move the globe |
| `terra.log(...)` | the workbench's log and the Xcode console |

A screen keeps the top-left 56 pt (inside the safe area) free: the app's own Back button sits
there, so no page can trap the reader. Pages are loaded from `terra-pack://<id>/…`; relative URLs
and `fetch()` of the pack's own files work. Storage does not persist between launches.

### `catalog.json` (what a source publishes; written by `playground.py pack`)

```json
{
  "format": 1,
  "generated": "2026-10-27T14:03:00-04:00",
  "packs": [
    { "id": "tide-pool", "revision": 3, "runtime": 1, "title": "Tide pool", "summary": "…",
      "presentation": "globe", "shelf": "current", "hash": "<sha256 of the file list>",
      "path": "packs/tide-pool/r3/", "bytes": 20480,
      "files": [ { "path": "prototype.json", "bytes": 512, "sha256": "…" } ],
      "published": "2026-10-27T14:03:00-04:00", "retired": false }
  ]
}
```

`path` is relative to the catalog's URL. Revisions are whole numbers that only go up; a pack's
files never change under the same revision (they are cached forever).

## Remote delivery

- **Only from the workbench.** The globe and every bundled experience make no requests. Opening
  the workbench checks its sources (at most every 30 seconds); tapping a remote prototype
  downloads it. No identifiers, cookies or analytics: an ephemeral session, plain GETs.
- **Sources** are catalog URLs: `Prototypes/_runtime/sources.json` (bundled), plus any added in
  the workbench or by the link `terraincognita://source?url=<catalog URL>` (it asks first).
  HTTPS only; `http://127.0.0.1` and `localhost` are allowed for the simulator.
- **Integrity.** Every file is checked against the catalog's SHA-256 and size; paths are confined
  to the pack; a pack is at most 50 MB and 500 files. A download is staged and moved into place
  only when complete, so a failed update leaves the previous revision. Two revisions are kept per
  pack; offline, the newest installed revision (or the bundled one) opens.
- **Containment.** Pack pages run in a non-persistent `WKWebView` that can load only its own
  pack's files: other schemes and every `http(s)` request are blocked by a content rule and the
  navigation policy; the bridge exposes only the table above. The catalog is not signed: it is
  trusted because only the owner's CI can write the bucket. Signing is the next step before any
  wider distribution.
- **App Store.** This is a development path for the SideStore build. Apple's App Review
  Guidelines restrict downloading code that changes an app's features (2.5.2) and set conditions
  for HTML5 mini apps (4.7); do not assume remote packs would pass review. A store build would
  bundle its prototypes and compile the workbench out (`WORKBENCH` in the build settings).

### Setting up R2 (owner, once)

1. Cloudflare dashboard → R2 → create a bucket (e.g. `terra-packs`), and allow public access (an
   `r2.dev` URL for development, or a custom domain on a Cloudflare zone).
2. R2 → Manage API tokens → create a token with *Object Read & Write* on that bucket only.
3. In this repository's settings → Secrets and variables → Actions: secrets `R2_ACCOUNT_ID`,
   `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, and variables `R2_BUCKET`, `PACKS_PUBLIC_URL`.
4. Add `<PACKS_PUBLIC_URL>/catalog.json` to `Prototypes/_runtime/sources.json` (next IPA), or add
   it now in the workbench.

Locally, the same variables in the environment let `playground.py publish` upload directly;
`--dry-run` shows what would change. Without them, `packs.yml` validates and keeps the packed
output as an artifact, and publishes nothing.

## What needs a new IPA

Anything in Swift or Metal: a new native prototype, a change to the globe, the host, the
workbench, `terra.js` (the runtime ships in the app), new entitlements or permissions. A web
pack's HTML, CSS, JavaScript, images and data, its markers, overlay and anchor do not.
