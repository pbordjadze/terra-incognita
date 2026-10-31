# Adrift's data: the builders

Every sheet in `adrift/data/` is built here, never hand-edited; `adrift/SOURCES.md` says what each
measures and what is drawn from it. Needs Python 3 with numpy, scipy, Pillow, pandas and netCDF4,
and about 1.5 GB in a cache directory (the drifters' tracks, a year a file).

    C=/some/cache
    python3 tools/adrift/data/currents.py $C     # currents.png, stream.png (Terra's builder, run here)
    python3 tools/adrift/data/wind.py $C         # wind.png
    python3 tools/adrift/data/ice.py             # ice.png, from Terra's Globe/ground/ice-MM.png
    python3 tools/adrift/data/coast.py           # coast.png, from Terra's Globe/geo.png
    python3 tools/adrift/data/eddy.py $C         # eddy.png, eddy-time.json (fetches 8 years of tracks)
    python3 tools/adrift/data/drifters.py $C     # drifters.bin, drifters.json
    python3 tools/adrift/data/look.py out/ 7     # draws every sheet for July, to look at

Each builder's docstring is its sheet's layout and source; `sheets.py` is what they share (the
tile grid, the manifest), `tracks.py` reads the trajectories. `manifest.json` is written by the
builders and read by the app: the decoding constants and the credit owed, so a rebuilt sheet
carries its own. `sea-harness/` compiles the sea module's Rust against a stub while the skeleton
is not there (`cargo test` in it); `critics/r<n>/` holds the PNGs the critics judged each round.
