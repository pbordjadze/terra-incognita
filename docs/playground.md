# The playground

The app opens on the globe. Every prototype has a **doorway** on it while it isn't running, with a
small life of its own (a glint now and then from afar, itself as you come closer, stirring): touch
one and it starts, where you are, beside whatever else is running. The **workbench** (the small grid
button, top right) lists them all, parked ones too. Several prototypes run at once on the one globe
(runtime 4, [`docs/runtime3.md`](runtime3.md) › Many at once): the one in **front** is the one you are
with (the last started or touched); its name is on the chip, with how many others run; its date and
caption are at the foot; ✕ ends it and the next is in front; the workbench's **The plain globe** ends
them all. Each one's sprites, pictures, geography, script and screens are made when it starts and
released when it ends, and the Live counts come back exactly.

Every new experience or map element is a **pack on the runtime** (3, 4 for a doorway with a life, 5 for things the date moves): data, pictures and a small script
that runs in JavaScriptCore inside the app, with no web view; the globe draws what it asks for.
[`docs/runtime3.md`](runtime3.md) is the contract. Native code is for the stage itself and for new
runtime tools (CLAUDE.md › The runtime).

- [From "prototype this" to the phone](#from-prototype-this-to-the-phone) (the short version)
- [Kinds of prototype](#kinds-of-prototype)
- [Make a pack](#make-a-pack) (no app build: preview, check, publish, see it on the phone)
- [Add a runtime tool, or a native prototype](#add-a-runtime-tool-or-a-native-prototype) (Swift: an IPA)
- [Update, compare, park, remove](#update-compare-park-remove)
- [The catalog](#the-catalog), [Remote delivery](#remote-delivery)
- [What needs a new IPA](#what-needs-a-new-ipa)

## From "prototype this" to the phone

For Claude Code or Codex, the whole loop for a pack (most new ideas):

1. `python3 tools/playground/playground.py new <id>`, then write it against `docs/runtime3.md`.
2. `node tools/playground/check.cjs <id>`: look at the screenshots in `build/playground/<id>/`
   (`node tools/playground/serve.cjs` to try it in a browser).
3. `python3 tools/playground/playground.py validate`, then commit. A push that changes only packs
   uses no macOS runner.
4. On `main`, `packs.yml` publishes changed packs to the `packs` branch, which the app reads
   (`Prototypes/_runtime/sources.json`). Open the workbench on the phone: the new revision is listed,
   or the pack's doorway is on the globe once it has been downloaded. A link
   `terraincognita://open/<id>` starts it.

If a pack needs something the runtime can't do yet, add it to the runtime (Swift, the next IPA)
rather than working around it; there are no web views to fall back on.

In the app: a doorway or a workbench row starts a prototype beside the ones running (a row of one
running brings it to the front); the chip at the top right names the one in front, **Open** shows its
panel (or its screen) and ✕ ends it. The workbench's **Live** line counts stages, markers, globes,
films and scripts: with nothing running it reads one globe and nothing else, and a prototype that
leaves something behind shows up there (and fails `PlaygroundTests`).

## Kinds of prototype

| Kind | Lives in | Starts | Needs an IPA to change |
| --- | --- | --- | --- |
| Pack, bundled | `Prototypes/<id>/` (bundled in the app) | from its doorway or the workbench | no, once published as a remote revision |
| Pack, remote | the `packs` branch's `catalog.json` | the same, downloaded on demand | no |
| Native | `App/TerraIncognita/<Name>/` + an entry in `Playground/Catalog.swift` | from its doorway (it opens straight away) or the workbench | yes |
| Parked | the Persian globe style, the films, test packs | from the workbench's Parked shelf | per its kind |

## Make a pack

```sh
python3 tools/playground/playground.py new tide-pool          # scaffolds Prototypes/tide-pool/
node tools/playground/serve.cjs                               # http://127.0.0.1:8790/ : every pack, over a stand-in globe
node tools/playground/check.cjs tide-pool                     # headless Chromium: runs it, fails on errors, screenshots
node tools/playground/check.cjs tide-pool --wikipedia         # the same with Wikipedia's words on (the reader's switch)
python3 tools/wikipedia/build.py --pack tide-pool             # its wikipedia.json: the articles its words name (docs/runtime3.md)
node tools/playground/check.cjs --living                      # two packs side by side on /atlas, by touch, ended in turn
node tools/playground/explore.cjs tide-pool out/ '[{"wait":3000,"shot":"a"}]'   # its moments from a plan: fly, scrub, touch, screenshot
python3 tools/playground/playground.py validate               # the contract, sizes, the offline boundary (CI runs it too)
```

The preview's `/atlas` is the living globe: every current pack's doorway on the one stand-in globe,
each alive; `/preview/<id>?with=<id>,<id>` runs a pack beside others. A pack's doorway is its first
reader's first sight of it: give it a glint from afar (`python3 tools/playground/glint_png.py` draws
the mark every pack uses) and itself up close.

`Prototypes/hello-pack/` is the smallest complete example (words at the foot, a panel, rings where
the sea is tapped); `Prototypes/lead-line/` and `Prototypes/vine/` use the globe's date and geography,
`Prototypes/sea-of-whales/` pictures on the globe faded in and out, `Prototypes/monsoon-sails/` sprites
placed by the date; `Prototypes/herds/` (runtime 5) moves things with the date: sprites on dated ways, a
date with an uneven pace, slowed as the reader comes close. A pack's art is drawn from code: a sheet of canvas drawings in `tools/<pack>/art.js`,
rendered by `node tools/packart/render.cjs tools/<pack>/art.js Prototypes/<id>` (headless Chromium,
three pixels a point); and `node tools/packart/moments.cjs <id> <out dir> '<plan>'` photographs a pack
over time in the preview (camera, waits, touches, drags, the date's strip), for what `check.cjs`'s fixed
screenshots can't show (`check.cjs` taps a thing under way where it is at that moment). A pack is a script, its data and pictures, with everything inside its
folder; it cannot reach the network. Keep the first-reader test in mind: someone who touches this
first, knowing nothing, should be able to tell what they are looking at.

## Add a runtime tool, or a native prototype

A runtime tool (a new `terra.*` capability: something drawn, animated or touched every frame, or
something the system has):

1. The native side in Swift (`Playground/PackScript.swift` turns the script's message into a
   `PackScriptEvent`; `PrototypeStage` turns that into the globe's drawing), the script side in
   `Prototypes/_runtime/terra.js`, its mirror in the desktop preview (`tools/playground/harness/`),
   and its entry in `docs/runtime3.md` with an example. Raise the runtime version only for an
   incompatible change.
2. The packs that use it, and a UI check (`GlobePackTests` or `PlaygroundTests`).
3. `[build-only]` in the commit message for a quick compile, then `[playground-check]`; `main`
   publishes the IPA to SideStore.

A native prototype: its code in its own folder `App/TerraIncognita/<Name>/`, one entry in
`Playground/Catalog.swift` (an id, a title, a one-line summary, an anchor, a doorway, and what
`start` does with the stage), its timers stopped when its views go away. A prototype of the globe's own
look (the quiet globe's rules: no doorway, nothing placed, a rule asked of the stage) sets `veils: false`,
so the clouds and the sea stay full while it runs and the comparison with today's look is fair. An experience on the globe
opens as a lens (`stage.openLens`, as the camelid and whale sequences do): its canvas, a view the
stage lays over the one globe, sets its hooks in `attach` and drives the globe's authored camera;
its chrome lies over the playground and closes it through the `close` it is given; the stage puts
back everything it set on the globe. It never makes a globe of its own. Only an experience with a map
of its own (the ocean) presents a screen, closed with `@Environment(\.dismiss)`.

## Update, compare, park, remove

- **Update** a pack: change its files and push; the publisher gives it the next revision only
  if its contents changed. The workbench marks it *available*, downloads it on the next tap and
  keeps the one before: **Previous** (under Packs) switches back, **Newest** forward again (a running
  copy of the pack ends with the switch; the next start opens the chosen revision).
- **Compare**: start both and touch one, then the other: each comes to the front with its own date
  and caption; the chip's ✕ ends the one in front and the workbench's first row starts the last one
  again; two variants are two packs (`tide-pool`, `tide-pool-b`). A variant to try on a phone is a
  new pack, not a change to the old one on a branch: the app opens the newest downloaded revision
  before the bundled one, and downloads `main`'s published revision whenever it differs from the
  bundled copy, so a branch's IPA shows `main`'s version of a pack it changed in place.
- **CI and packs**: a push whose changes are all under `Prototypes/` (or `tools/playground/`) runs
  only `packs.yml`; the macOS workflow ignores it, tags and all. A `[playground-check]` or `[ipa]`
  meant for such a change rides a commit that also touches something the macOS workflow watches.
- **Park**: `"shelf": "parked"` in `prototype.json` (or `shelf: .parked` in Swift) moves it to the
  Parked shelf, still one tap away (and takes its doorway off the globe).
- **Remove**: delete the folder or entry. A published pack is withdrawn with
  `playground.py retire <id>` (`--undo` brings it back; `--out DIR` acts on a local packed folder);
  its files stay on the `packs` branch. Ids are never reused: nothing can check that across branches, so
  pick a new one rather than recycling an old one.

## The catalog

`catalog.json`, what a source publishes (written by `playground.py`):

```json
{
  "format": 1,
  "generated": "2026-10-28T05:03:00-04:00",
  "packs": [
    { "id": "tide-pool", "revision": 3, "runtime": 3, "title": "Tide pool", "summary": "…",
      "presentation": "globe", "shelf": "current", "hash": "<sha256 of the file list>",
      "path": "packs/tide-pool/r3/", "bytes": 20480,
      "files": [ { "path": "prototype.json", "bytes": 512, "sha256": "…" } ],
      "published": "2026-10-28T05:03:00-04:00", "retired": false }
  ]
}
```

`path` is relative to the catalog's URL. Revisions are whole numbers that only go up; a pack's
files never change under the same revision (they are cached forever).

## Remote delivery

- **The source** is this repository's `packs` branch, which `packs.yml` writes on pushes to `main`
  that change packs, read through `https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog.json`
  (`Prototypes/_runtime/sources.json`; raw.githubusercontent.com caches for about five minutes).
  Other sources can be added in the workbench or by the link `terraincognita://source?url=<catalog URL>`
  (it asks first). HTTPS only; `http://127.0.0.1` and `localhost` are allowed for the simulator.
- **Only from the workbench.** The globe and every bundled experience make no requests. Opening the
  workbench checks its sources (at most every 30 seconds); starting a remote pack downloads it. No
  identifiers, cookies or analytics: an ephemeral session, plain GETs.
- **Integrity.** Every file is checked against the catalog's SHA-256 and size; paths are confined to
  the pack; a pack is at most 50 MB and 500 files, a file at most 25 MB. A download is staged and
  moved into place only when complete, so a failed update leaves the previous revision. Two
  revisions are kept per pack; offline, the newest installed revision (or the bundled one) runs.
- **Containment.** A pack's scripts run in their own JavaScriptCore context with no network, no
  storage that lasts and no access to anything but their pack's files and the `terra` bridge
  (docs/runtime3.md). The catalog is not signed: it is trusted because only this repository's CI
  writes the `packs` branch. Signing is the next step before any wider distribution.
- **App Store.** This is a development path for the SideStore build. Apple's rules on downloaded
  code (App Review 2.5.2, and the developer agreement's terms for interpreted code) need checking
  before any store build; a store build would bundle its packs and compile the workbench out
  (`WORKBENCH` in the build settings).

## What needs a new IPA

Anything in Swift or Metal: a runtime tool, a native prototype, the globe, the host, the workbench,
`terra.js` (the runtime ships in the app), new entitlements or permissions. A pack's scripts, data,
pictures, doorway (its looks, path and breath included) and anchor do not.
