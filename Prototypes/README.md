# Prototypes

Web prototypes for the playground: one folder each, plain HTML, CSS, JavaScript and media, bundled in the app
and published as remote revisions. `_runtime/` is the host's side (`terra.js`, the bundled sources), not a
prototype. The contract and the workflow are in [docs/playground.md](../docs/playground.md).

```sh
python3 tools/playground/playground.py new <id> --globe      # or --screen: scaffold Prototypes/<id>/
node tools/playground/serve.cjs                              # preview over a stand-in globe, http://127.0.0.1:8790/
node tools/playground/check.cjs <id> [--device ipad]         # headless Chromium: errors fail, screenshots in build/playground/
python3 tools/playground/playground.py validate              # the contract, sizes and the offline boundary
```

Publishing is `playground.py publish` (or `packs.yml` on push). A pack reaches nothing outside its folder, and an id
is never reused. `hello-pack` is the delivery check: it shows which revision of itself arrived, and from where.
