# Battle Ink: rounds

One line a round (painting-company row 206); times ET.

- R1 (Nov 3, 2026): six spikes and a blind judge chose the fusion: the "currents" spike's living copperplate (thousands of tapered burin cuts combed along each army's current, iron-gall Rome, Tyrian Carthage, on Cannae's real relief and the traced Aufidus), the "ink-orders" spike's simulation (a stroke is a channel Carthaginian men ride, about 3,050 units on local rules, the double envelopment emergent), and marbling's folded chevrons where the inks meet; the pen combs the cuts as it passes, the fallen are laid into hatching so the page ends as a print. Linux build and tuning 01:20–02:00 ET, device proof (toys-ipa, IPA green) 02:00–02:21, ship (one IPA, the merge) 02:31–.
- R2 (Nov 3, 2026): the critic's weakest point, the ending: the fallen take room and the ringed-in stand, flight is a short trail that fades, isolated fallen fade, no wash; when Rome is beaten time slows to a still page as the pocket presses into a dense mass and Hannibal's men close round it in a ring combed tangent to it (any other ending freezes as it lies); night on a warm dark paper, the pen rakes the men it passes. Verdict 03:35, Linux build and tuning 03:40–04:30 ET, ship (one IPA, the merge) 04:30–.
- R3 (Nov 3, 2026): the critic's weakest point, the composed ring: dropped; the page now freezes where the day was decided (encirclement detected, an army broken, or the day run long), each cut as it last lay, the strokes drying to channels, the real day's ghost lifting beside them; a camera eases in on the fight so it fills the frame; deployment ranks given give, each cut a little off true; night Rome a cold blue; tap drops bloom and dry. Verdict 04:30, Linux build and tuning 04:32–04:50 ET, ship (one IPA, the merge) 04:50–.
- R4 (Nov 3, 2026): the critic's weakest point, the keepsake framing: once the day is decided the camera draws back over 3.5 s to the whole field, river to ridge, the frozen ink between them; the ghost rises, then its dashes close into a fine engraved line; the strokes dry thin; the opening frames the river too; a tap is a small blot that wicks and bristles the cuts round it. Verdict 05:55, Linux build and tuning 05:56–06:05 ET, ship (one IPA, the merge) 06:05–.
- R5 (Nov 3, 2026): the critic's weakest point, the print unreadable: the plan dries into Carthage's purple (tapering the way it was drawn), the real day is a quieter sanguine stipple with arrowheads, only Hasdrubal's ride, the wings' turn and the bent-back bow (the stray straight line, the Roman press, removed), visible at night; the print framing settles between field and close, the frozen fight about half the width with the river beside it (and now in landscape too). Verdict 06:10, build and tuning 06:12–06:27 ET, ship 06:28–.

## Where it stands (Nov 3, 2026, 13:00 ET; the loop stopped short of its cost cap)

- Shipped: R5 is `battle-build-99` (main a06bf4f), in the SideStore source; every toy's Linux check and the
  launch check green (iPhone GPU limits; peak RSS about 176 MB of 1400, GPU about 63 MB).
- The last critic (R5, fresh, full-size renders of every plan): not yet showable without a caveat. Novelty 8,
  beauty at full size 7, in motion 7, legibility 6, touch 6 (R1: 7/6/5/4/5). Best: the close clash (the two inks
  combing through each other, never grey), a drawn stroke visibly becoming a channel the cavalry streams along,
  the quiet opening page on the real ground. Weakest: the decided page is not yet a keepsake: the real day's
  sanguine line is too faint beside the ink and reads as an overlay, and on the phone the frozen fight is
  clipped at the right edge.
- Next, in this order: (1) the print: the real day as a bolder engraved line, the frozen fight wholly in frame
  on the phone (print_frame in lib.rs); (2) night's inks pastel (Carthage stops reading purple); (3) touch: a
  scribble or hold should answer distinctly, a shake shouldn't silently throw the battle away; fleeing
  fragments left as debris at the edges; the river's stair-stepped edge at full size.
- Known: the simulation is tuned in Hannibal's favour so a good plan ends near the recorded losses; the decision
  rules, the camera and the ghost are drawn choices (SOURCES.md). The spikes are on `spike-battle-*` branches.
- Checking: `bash tools/toys/battle/films.sh` plays every plan into toys/build/battle/ (minutes on software
  Vulkan); `quick.sh` for a few shots; `BATTLE_CENSUS=1` prints why a battle was decided. Compare the final
  frames across plans and check the last seconds still differ: both caught real bugs late.
