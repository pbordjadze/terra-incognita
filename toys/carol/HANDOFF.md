# Carol: handoff (Nov 4, 2026)

painting-company row 215. A one-off toy chess trainer as its own app on the toy kit (no globe):
a wooden board, tap a piece, then where it goes. ROUNDS.md has a line a round; SOURCES.md the data.

## How it works

- `src/chess.rs`: FEN to a board, `drills.json` to drills, and `Ply::accepts` (the line's squares,
  or for castling the king onto its rook). No move generation: every position after every ply is
  given (`after`). Its tests walk every drill.
- `src/lib.rs`: the toy. The board is one fragment (`BOARD_WGSL`, a `Backdrop`); the pieces one
  instanced pass over `data/pieces.png` (`PIECES_WGSL`: R silhouette, G rim, B shadow), each piece
  easing to its square, so any change of position animates (castling, en passant, a capture fading);
  everything else is `paint`. Sound is `sound::Bells`.
- The turn: the reader's side is `orient`; the other side answers after 0.65 s; `auto` plies are
  played at once. A wrong square counts a miss and bumps the piece back; the bulb lights the
  from-square (a miss, once a ply); the play mark makes the move (a miss). A finished drill gilds the
  frame, throws confetti (more when clean) and after 2 s the mix picks the next: never done first,
  then last missed (most first), then least done.
- Beads: 24, grouped black / white / puzzle; hollow = new, filled = done, a red pip = missed last
  time, ringed = now. Tapping one starts it. Saved: the current drill and each drill's (done, last).
- Plans name places: squares (`"e2"`), `from` and `to` (the reader's move now), `hint`, `show`,
  `bead<i>`, `info`.

## Privacy (a hard rule)

Only the moves and positions go public. No titles or notes refer to any person, game, record or
preparation; anything added must be about the chess itself.

## Next

- The "?" rules mark (`Toy::rules`, toy-kit on branch `toys-rules`, row 214): once it is on main, add
  Carol's `fn rules` beside `demo` (two or three lines: tap a piece then its square; the other side
  answers; the bulb and the play mark each count a miss; then "Real: … Drawn: …") and a
  `plans/rules.json` like the other toys'.
