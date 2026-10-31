# Adrift's data: the builders

Every sheet of Adrift's own in `adrift/data/` is built here, never hand-edited; Terra's sheets
(the currents and stream function, the coast's distance field, the ice by month, the land) are
read as they are, where `adrift/data/manifest.json` says they are. `adrift/SOURCES.md` says what
each measures and what is drawn from it. Needs Python 3 with numpy, scipy, Pillow, pandas and
netCDF4, and about 1.5 GB in a cache directory (the drifters' tracks, a year a file).

    C=/some/cache
    python3 tools/adrift/data/wind.py $C         # wind.png
    python3 tools/adrift/data/eddy.py $C         # eddy.png, eddy-time.png, eddy-time.json (fetches 8 years of tracks)
    python3 tools/adrift/data/drifters.py $C     # drifters.bin, drifters.json
    python3 tools/adrift/data/look.py out/ 7     # draws every sheet the rules read, for July, to look at

Each builder's docstring is its sheet's layout and source; `sheets.py` is what they share (the
tile grid, the manifest's entries), `tracks.py` reads the trajectories. `manifest.json` is read
by the app and the shells: `files` maps every file the code loads to its place in the repository,
`sheets` says how each is decoded and the credit it is owed, so a rebuilt sheet carries its own.
The rules that read the sheets are `adrift/crates/adrift-core/src/sea/`, held to the record by
`cargo test -p adrift-core sea::` (after `python3 tools/adrift/assets.py adrift/build/assets`).
`critics/r<n>/` holds the PNGs the critics judged each round.
