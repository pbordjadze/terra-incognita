# Carol: sources

- **The drills** (`data/drills.json`): 24 chess positions and lines (openings as Black, as White, and
  single-move puzzles), each ply with its move and the position after it as FEN. Chess moves and
  positions only; no games, players or notes. Given with the ask (painting-company row 215).
- **The pieces** (`data/pieces.png`): drawn by `tools/toys/carol/pieces.py` from DejaVu Sans's chess
  glyphs (Bitstream Vera licence, free to embed and modify): each glyph's outline filled to a
  silhouette, with a rim and a shadow. The icon (`icon.png`) is drawn from them by `tools/toys/icon.py`.
- **Everything else** (the board, the wood, the beads, the sounds) is drawn or synthesised in code.
