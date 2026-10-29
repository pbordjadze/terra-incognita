# Prototypes

The playground's packs: one folder each, runtime 3 or 4. A pack is data, pictures and a small script that runs
in JavaScriptCore inside the app; the globe draws what the script asks for (geography, pictures, sprites, the
date, the words at the foot, a panel). Several packs run on the one globe at once, each entered by touching its
doorway, which has a life of its own meanwhile (runtime 4: a glint from afar, `doorway-glint.png`, drawn by
`tools/playground/glint_png.py`). There are no pack pages and no web views. Packs are bundled in the app and
published as remote revisions. `_runtime/` is the host's side (`terra.js`, the bundled sources), not a pack.
The contract is [docs/runtime3.md](../docs/runtime3.md); the workflow is [docs/playground.md](../docs/playground.md).

```
<id>/
  prototype.json   who it is ("runtime": 3, "presentation": "globe"), where it lives, its "scripts"
  main.js          its script (and any others "scripts" lists, run in order)
  data.js, *.png   its data and pictures
  SOURCES.md       where its facts and pictures come from, and what it does not claim
```

```sh
python3 tools/playground/playground.py new <id>              # scaffold Prototypes/<id>/ (prototype.json, main.js, SOURCES.md)
node tools/playground/serve.cjs                              # preview over a stand-in globe, http://127.0.0.1:8790/
node tools/playground/check.cjs <id> [--device ipad]         # headless Chromium: errors fail, screenshots in build/playground/
node tools/playground/check.cjs --living                     # two packs side by side on the preview's /atlas, by touch
node tools/packart/render.cjs tools/<pack>/art.js Prototypes/<id>   # a pack's art, drawn from a sheet of canvas (or .cjs SVG) code
node tools/packart/moments.cjs <id> <out dir> '<plan>'       # screenshots of a pack over time, with touches and the date
python3 tools/playground/playground.py validate              # the contract, sizes and the offline boundary
```

A pack reaches nothing outside its folder: `validate` refuses `fetch`, `XMLHttpRequest`, `WebSocket` and `import`
in its scripts, and warns about a link anywhere but SOURCES.md or a `terra.panel` source's `url`. An id is never
reused.

## Publishing: the `packs` branch

On a push to `main` that changes a pack, `packs.yml` validates every pack and publishes to this repository's
`packs` branch: only a pack whose contents changed gets the next revision (`packs/<id>/r<rev>/`), the files of
a published revision never change, and `catalog.json` lists them all. The app reads it, as a bundled source, at
`https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog.json` (raw.githubusercontent.com
caches for about five minutes). `[no-publish]` in the commit message holds a publish back; other branches only
validate and keep the packed output as an artifact. CI owns the branch (an orphan, with no workflows) and needs
no secret. A link `terraincognita://open/<id>` opens a pack on a phone with the development build.

To withdraw a published pack (the owner's call), retire it on the branch; its files stay:

```sh
git fetch origin packs && git worktree add ../packs-branch origin/packs
python3 tools/playground/playground.py retire <id> --into ../packs-branch     # --undo brings it back
git -C ../packs-branch commit -am "Retire <id>" && git -C ../packs-branch push origin HEAD:packs
```

`playground.py publish --into DIR` is what CI runs, into any checkout of the branch (or an empty folder); it
leaves committing to the caller. `playground.py publish` without `--into` still publishes to an R2 bucket, for
later. `hello-pack` is the delivery check: it shows which revision of itself arrived, and from where.
