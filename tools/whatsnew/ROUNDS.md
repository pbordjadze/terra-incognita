# What's new: the rounds

The build-and-critic rounds that made the mode (painting-company row 187). Each: what was built or
changed, how it was looked at (CI's simulator shots, the preview), what a fresh hard-to-impress critic
given only the pictures said would make the owner skip it (ranked by how much he'd notice), and what
the next round fixed. Stop: the critic's worst complaint is a quibble, or is overruled here with a
reason.

## Round 1 (October 30, 2026, ET)

**Built.** The module (pieces, ledger, session, store and courier, recorder and snapshot, views), the
Terra host, the queue seeded with twelve pieces, `WhatsNewTests`, `tools/whatsnew/whatsnew.py`, the
docs. Compiled with `[build-only]`; the first round of pictures comes from `[test WhatsNewTests]` on
the iPad and a hand-started run on both devices.

**Looked at.** `[test WhatsNewTests]` on the 13-inch iPad (run 212/213): the pill, the first piece, the
second, a picture taken, the mic tapped. The round test stopped at piece 2: Skip did not move on.

**Found before any critic.** The queue showed *1 of 5*: the seven packs' pieces were missing, and
with them the packs. A pack's `whatsNew` has no `id` of its own, the manifest's decoder required one,
and `PackStore` drops a pack whose manifest won't decode: a half-written piece could take a whole
pack off the workbench. The piece now decodes with empty defaults and `isSound` decides. The bar
came out as a dark grey box over the clouds (glass takes the scheme and what is behind it), four
lines of text, the gold question unreadable on it; the pill and the taps came out light. Skip and
Didn't work were grey words on the sky, 16 points tall, at the screen's bottom edge. A picture of the
screen had the globe in it (the Metal layers made readable for the frame: the one thing that could
not be reasoned out off the Mac).

**Changed.** Everything of the mode sits on the caption's paper (a little translucent, light in any
scheme), with the atlas's gold as an ink dark enough to read; `what` is two lines at most; Skip and
Didn't work are paper buttons, 36 points tall; the test asserts the count after each Skip and
answers the microphone sheet from an inert touch (a touch on the globe had lit Turkey).
