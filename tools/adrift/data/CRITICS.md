# Truth: the critics' verdicts

Each round a fresh hater (who wants Adrift to fail and judges only what runs: renders, never
descriptions) and a Holt critic (does it let you wander, stop, undo and leave without penalty; does
something unscripted happen). What they judged is in `critics/r<n>/`.

## R0 (October 31, 2026)

There is no running build yet; the skeleton is Engineering's R0. What ran: the builders, and
`look.py`, which draws every sheet exactly as the rules decode it (`critics/r0/`: `wind-07.png`,
`currents-07.png`, `eddy.png`, `ice-01.png`, `drifters.png`), and the rules' tests in
`sea/tests.rs` (all pass; run from the crate once the skeleton landed).

**The hater (a sea-going physical oceanographer who has watched too many "ocean simulations"
that were screensavers).**
- *The wind sheet* (`wind-07.png`): the July monsoon jet off Somalia, the trades, the roaring
  forties: it is NCEP's July, recognisably. But it is a monthly *mean*: the real wind veers and
  gusts, and a float's leeway on the mean is a smoothed fiction. Verdict: honest as labelled, and
  the label had better be on the screen.
- *The eddy sheet* (`eddy.png`): the Gulf Stream, Kuroshio, Agulhas rings, the Brazil-Malvinas
  confluence and the equatorial bands are where they should be, the gyre interiors quiet. Two
  objections. The equatorial band is bright because the tropical instability waves are in the
  variance, fair; but the jets are bright partly because a 0.25° cell straddling a 1 m/s jet holds
  the jet's own shear as "eddy" variance, and nobody corrected it (Zhurbas and Oh did). And a
  memoryless random walk with K = 30,000 m²/s throws a float 70 km sideways a day: of two
  thirty-day floats stepped off Cape Hatteras (one seed, two days of the year), one ran the stream to 55° W, the
  other (the same seed, a day later in the year) left it in two days and sat on the Delaware shore.
  Against the record (the test `the_flight_off_hatteras_keeps_to_the_record` prints it): of 500 maybes put off Hatteras in a typical July,
  thirty days on, 38% are east of 65° W and the median is at 68.2° W, with 3% on a coast; of the
  18 bundled GDP runs that passed there, half were east of 65° W (median 64.8° W) and none ashore.
  Real drifters stay in the stream because their velocity has memory (the time scale this very
  tool measured, 2–4 days) and the jet is coherent. **Verdict: the wander's size is measured; its
  shape in the jets is wrong: behind the record, and some of it on the beach.** The fix is a random flight (an Ornstein–Uhlenbeck velocity with the
  measured T), which needs the float to carry its eddy velocity: filed as D1.
- *The drifters* (`drifters.png`): 1,979 real runs, and the mean distance from an open-sea point to
  the nearest bundled point is 18 km, with 3 of 1,193 points beyond 300 km (the southern Pacific
  and the Arctic shelves). Fair. But a track drawn whole on a flat map is a lie of omission: these
  are eight different years laid on top of each other. Verdict: fine as data; the screen must show
  one run at a time, with its year.
- *The coast and the ice* (`ice-01.png`): the ice edge is January's mean of 2015–2024, the coast
  Natural Earth's 1:50m. An island under 10 km does not exist here; a float will sail through the
  Maldives. Verdict: say so (SOURCES.md does).
- *The leeway*: 3% is derived, not measured; no glass float was ever towed behind a current meter.
  Verdict: tolerable only because the derivation is written down with its one measured anchor
  (a sphere three quarters under at 2%).

**Holt.** Nothing to wander in yet. On the rules as tests: a float put down is left alone by them
(no script decides its path; two seeds part ways, the test says so), a coast holds without
trapping forever (the next step tries again), the ice lets go when the month turns. Something
unscripted does happen on paper: the harness's float ran up the shelf to Delaware and came back
off it, which no one wrote. Verdict withheld until there is a screen to leave.

**Later the same day, the flight.** `Rules::flight` (and `rules.wgsl`'s `maybes_flight`) carries
an eddy velocity with the measured time scale (`eddy-time.png`). The same 500 off Hatteras,
thirty days on: 46% east of 65° W (the record's half), median 65.9° W (the record's 64.8°),
6 on a coast (the record's none; real drifters do run aground, though none of these 18 did in
their thirty days). The hater: better; the beach count is the next thing to look at on a screen.
D1 now asks Engineering only to carry the two numbers.

**The running build** (the skeleton landed the same day; `critics/r0/atlantic.png` from
`adrift-check render scenes/atlantic.json`, `play-04-drifted-40-days.png` from `play
plans/put-and-drift.json`, both on Linux's software Vulkan, the rules now Truth's `Rules::step`).
The hater: three floats on the Atlantic, and their wakes run the right way: the Hatteras one east
along the stream, the one off West Africa down the Canary Current. But the put float's forty-day
wake (play-04) is a scribble: a daily bead of a memoryless walk at a quarter-day step, each bead
seventy kilometres from the last in no particular direction. That is the walk, drawn honestly; it
is not what a drifter's track looks like. The app steps with `Rules::step`; the flight waits on D1.
Nothing on the screen says the date is a typical year. Holt: a float was put, drifted, the date
was stopped and it rested, undo took it back and nothing else moved (`06-undone`): the first four
promises hold in the check. Something unscripted happened: the float went north-east where the
current and its draws took it, and no one wrote that.

**What Truth takes from R0:** D1 (velocity memory for the wander); the date and the years must be
said on the screen (a demand on Visual once the look exists); the jets' shear contamination is a
R1 improvement to `eddy.py` (Zhurbas and Oh's correction, or a coarser variance in the jets).

## R2 (October 31, 2026; the merged build at 5c21f63 with Truth's R2 commit on it)

What ran: `adrift-check render scenes/gulf-stream-close.json` (`critics/r2/gulf-stream-close.png`,
and `-hatteras.png`, the coast close up), `play plans/put-and-drift.json` (`play-04-drifted-40-days.png`);
23 sea tests, among them a new one: no maybe is on land after any step, on the CPU or the GPU.

**The hater (the same oceanographer, now looking at a cloud of 2,000 maybes).**
- *The wakes* now follow the flight: the Hatteras float's beads bunch where it dawdled on the
  shelf and stretch where the stream took it, the second float's run north-east along the jet.
  That is what a drifter's track looks like. Verdict: the walk's scribble is gone.
- *The maybes* bunch along the stream and spread north-east, thinner on the shelf and in the
  sounds: the right shape. Specks sit in Pamlico Sound and on the shoreline: the sounds are sea
  to Natural Earth and to the rules, and a held float sits five kilometres off a shore drawn ten
  kilometres a pixel, so at this zoom it lies on the line. A few specks look inland on the
  Virginia shore: river mouths the ground paints as land. Not a lie, but a reader will think a
  float went up a river. Verdict: honest to the data's resolution; say the resolution somewhere.
- *The maybes start from rest.* Every speck starts with zero eddy velocity (app.rs seeds the
  buffer that way); the measured sea at the put place is already moving. `Rules::put` draws the
  first velocity from the measured variance; until the app seeds with it, the cloud's first two
  or three days are too tight. Verdict: wire it (asked).
- *The words.* "19 July, a typical year, by the drifters' record" is the honest date, at last on
  the screen. But the foot credits only the currents; the wind, the ice, the coast and the real
  drifters are owed theirs. `sea::CREDIT_LINE` is one line that names all four. Verdict: a licence
  is not met by naming one of four.
- *Not yet there:* the silver drifter (`Drifters::track` exists; nothing draws it), and nothing
  says the coast is ten kilometres a pixel.

**Holt.** In the played session a float was put, drifted forty days where the rules and its
draws took it (north-east, not where anyone said), the date was stopped and it rested, undo took
it back and the globe was as before. Holding a float shows its other selves without moving it.
You can wander, stop, undo. Leaving is not yet in a plan; the silver drifter is not yet on the
screen, so the one thing that would make a stranger believe the cloud (a real buoy that went
there) is missing. Verdict: the rules keep their promises; the screen is a round behind them.

**Later, on d2bd376 (`critics/r2/gulf-stream-close-silver.png`).** Engineering wired the put's
draw, the silver drifter and the credit line. The hater: the silver is a real run (a tight loop
on the shelf, then off along the stream to the edge of the frame and beyond), and it is too much
of one: a whole year of a buoy across the frame says "a drifter went everywhere", not "a drifter
passed here". `Track::window(30)` is the month about the nearest point, with `label_window` for
its dates; draw that. The credit line is on the foot and runs off both edges of the phone: a
licence cut in half is not met; it is now under eighty characters. Holt: holding the float shows
a real buoy's way; nothing moved. Fine.

## R3 (October 31, 2026; the merged build at 37e8ee1, the first build on main)

What ran: `adrift-check render scenes/gulf-stream-close.json` and `gulf-stream-night.json`
(`critics/r3/`), `play plans/put-and-drift.json`, 23 tests, and the budget tool
`sea::tests::maybes_budget` (its table is in SOURCES.md › How many maybes).

**The hater.**
- *The silver* is now a month and a half of a real buoy about where it passed, with its name and
  its dates: "Drifter 132676, 7 Jan to 6 Apr 2016" beside a float held on 19 July. That is the
  truth and it is a little uncomfortable: the buoy's winter beside the float's summer. Good: it
  says "any year" without a word. Verdict: honest; the label is the thing that makes it so.
- *The credit line* fits the foot now and names the four. The licence's sentence is still not
  on the device anywhere a reader can reach: D2.
- *The guess.* Nothing on the screen tells a float on a measured current from one riding the
  sheet's smoothed-in guess (Hudson Bay, under the ice, some gulfs). The rules know (`measured`);
  the look doesn't show it: D3. Until then SOURCES.md says it and the screen doesn't.
- *The budget.* 2048 paths at a quarter day, re-run each half day, is more than the honesty
  needs: 512 at a half day tell the same shape (Kolmogorov under 0.05 on both axes at Hatteras
  and in the Sargasso). A whole day is not honest in a jet (the coast catches nearly twice the
  paths). Verdict: spend the frame elsewhere.
- *Still owed from R0:* the jets' variance holds their meandering as well as their eddies (no
  shear correction), and K is capped at 40,000 m²/s; both are written in SOURCES.md and neither
  is on the screen, which is right: a reader can't use them, a reviewer can.

**Holt.** Put, drift, stop the date, undo: the four hold in the played session as in R2. Hold a
float and a real buoy's winter rises beside its summer, which no one scripted and which is the
most honest thing on the screen. Leaving still isn't in a plan (nothing marks, nothing points,
nothing waits: that is for the device, where a reader can put the phone down). Verdict: the
rules keep their promises; two demands stand between the screen and the whole truth.
