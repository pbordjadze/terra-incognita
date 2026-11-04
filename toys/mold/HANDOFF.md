# Mold: handoff (Nov 3, 2026)

painting-company row 203. A glowing slime mold on the night Earth: Jones's (2010) Physarum agents on the
GPU, fed by oat flakes laid at the Black Marble's lights (as Tero et al. 2010 laid them at Tokyo's towns),
kept off the open sea. ROUNDS.md has a line a round; SOURCES.md the data and the model.

## Where it stands

- Nov 4 (row 219): three marks, bottom right: the info mark (unchanged), the "?" (the rules alone) and
  an asterisk (where they came from; `origins()` in lib.rs). The card hangs over the half of the
  screen with less lit land (`card_at_top`). `plans/rules.json` photographs both; the Simulator keeps
  `rules.png` and `origins.png`. Open: whether Paul reads the asterisk as "where it came from".

- Shipped: round 4, `mold-build-103` (13:38 ET), from main 69b8264. What Paul taps: the globe opens on Europe and Africa, a mist
  of spores on the lights condensing into a network over about 7 s; a pinch in shows translucent tubes
  between the cities down to 9x (a pinch past it eases back); a tap lays a flake (a ring; the mold
  reaches it) or, on the sea, rings only; left alone after a fling it settles north up over lit land.
- Checks: `bash tools/toys/launch_check.sh mold` passes (the iPhone's limits, every shader to MSL 2.4,
  peak RSS 645 MB of 1400); every plan plays under KIT_IPHONE_LIMITS=1 with no GPU error.
- Plans: start (phone, the core moment), close (pinched to ~6x on Italy), deep (to the 9x stop, three
  feeds), touch (sea and land taps, a fling into the Pacific), ipad.

## The launch crash: found (Nov 4, 2026)

The first run of the toys in the iOS Simulator with Metal's validation on (`tools/toys/sim_launch.sh`,
now in every IPA job) showed it: Apple's Metal compiler refuses the streaks pipeline, because naga
writes the WGSL varying `half` through to MSL as `float half [[user(loc2)]]` and `half` is a Metal
type. Builds before mold-build-71 let wgpu's default handler panic on that error inside `toy_new`
(panic = abort): closed at once. Builds 71–103 log it instead; the frame's one render pass then goes
black whenever the streaks draw (zoom past about 1.8x). The varying is `halfw` now, and
`tools/kit/launch_check.py` fails on any MSL declaration named after a Metal type. Sure of the
failing pipeline (the Simulator's own compiler error); fairly sure it was Paul's crash (it is the
only error on launch, and it fits "closed the moment it opened" on a build before 50); the phone's
`toy-log.txt` on the fixed build is the last word.

## (Before Nov 4) the open issue: the launch crash

Paul reported Mold closing the moment it opened (on a build before 50; which one is not known). Nothing
on Linux reproduces it: the launch check above passes, memory was cut 1049 -> 645 MB on the check, and
the kit now logs instead of panicking. **What to ask Paul for:** after opening the current build once,
Files > On My iPhone > Mold > `toy-log.txt` (it says the GPU's limits, the memory free before and after
the toy starts, and any wgpu error or panic), and, if that log has no "after" line, Settings > Privacy &
Security > Analytics & Improvements > Analytics Data, the newest `JetsamEvent` or `Mold-...ips` file.
No "after" line and a JetsamEvent: memory (then halve the field to 2048 x 1024 on a phone: about 96 MB
less). A logged wgpu error: fix that line. Neither: the launch time or the shell (toys/ios).

## What the last critic said (round 3, before round 4)

Not yet without a caveat: the deepest zoom (~18x) was one blob over pixelated lights (round 4 caps the
zoom at 9x, B-spline lights, no hairs); the resting view drifted over the Pacific (round 4 turns to lit
land); a third feed close to others did not read as its own (round 4 flares a new flake); the far view a
little dim (round 4 brighter). Keep: the opening's condensing web, Europe at 2-6x, a feed becoming a star
of tubes, the clean tap ring, no text.

## What I'd do next

1. The crash, from Paul's log (above), before anything else.
2. A critic on round 4. Likely next weak points: dense knots (Po, Benelux) still the brightest blobs; thin
   desert threads come and go close in; no sound (a low organic hum that swells where the mold feeds would
   suit it); the GPU's frame time on the phone is unmeasured (the shell logs frames every 5 s).
3. If the touch check's warnings matter: claim cells in a deterministic order (the atomics make runs
   differ by about 20% of pixels, so a touch's effect is judged against that noise).
