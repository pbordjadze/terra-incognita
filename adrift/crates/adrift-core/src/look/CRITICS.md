# Visual: the critics

Each round: a fresh *hater* (wants Adrift to fail; judges only the running build's pictures) and
the *Holt* critic for the whole (can you wander, stop, undo and leave without penalty; does
something unscripted happen). Pictures judged are in `studies/` (device pixels, 1:1 crops; made by
`python3 tools/adrift/study.py everything --put=-74.5,35.3`), drawn over Terra's Metal stand-in
of the real sea with numpy, since the wgpu build does not exist yet. The next round's critics judge
`adrift-check` renders, nothing else.

## R0 (October 31, 2026): studies over Terra's stand-in sea

Judged: `studies/float-by-zoom.png` (phone zoom 1, 2.6, 6; iPad 2.6), `studies/ring-beat-phone.png`
(five moments of the resting finger), `studies/held-phone-day-night.png`, `studies/held-phone-z1-z6.png`,
`studies/wake-phone-and-ipad.png`, `studies/foot-phone.png` (the date and credits).

**The hater, a cartographer who has seen every "beautiful globe" demo die on a phone.**

- *The float.* "Fine: at the whole Earth it is nine points of dark ring and amber and I can find
  it; at zoom 6 it is fourteen and it is a glass marble with its sun upper left. I will grant it
  reads at a glance in day and at night. But it is a marker: it never grows with the water, so at
  zoom 6 the Gulf Stream's own lines run past it like it was pinned on. Decide if that is the
  design (I think it is) and never let it grow." Verdict: passes, on the condition it stays a
  marker.
- *The ring that thickens.* "The five frames read: a thin white ring drawing in, going glassy,
  then the lamp lighting at the end of the beat. The third and fourth frames are mud: a grey ring
  with nothing in it. In motion that is 150 ms of nothing. Put the amber in sooner or make the
  glass bluer so the middle of the beat is a thing." Verdict: fails the middle of the beat; fix in
  the shader where the ring is a continuous function of the beat, not five frames.
- *The wake.* "Beads bunch where the walk slowed and string out in the jet, and the tail fades.
  That is honest. But at the whole Earth (held-phone-z1-z6, left) forty-five beads at a fixed
  point size pile into an orange slug. The beads must shrink with zoom-out or thin to every third
  day, or the whole Earth is a smear of warm pins." Verdict: fails at zoom 1.
- *The maybes.* "Two thousand pale specks: by day a sand spray, which is what a cloud of maybes
  should be, and the grain keeps it from being a tint. By night, even at half strength, it is a
  galaxy that out-shines the city lights beside it (held-phone-day-night, right). And the cloud
  here is forty-five days wide; at zoom 1 it is a thumbprint. Fine. But if the maybes are drawn
  each as a disc of 0.75 pt they will cost two thousand discs a frame; draw them as points with a
  soft edge or the phone will tell you." Verdict: day passes; night too loud; the cost unproven
  until it runs.
- *The silver.* "A white wire on a grey line, smoothed to look like a real track. Readable, cool,
  does not compete with the amber. At zoom 1 it is a worm; it must thin with the beads." Verdict:
  passes at 2.6 and 6, fails at 1.
- *The words.* "The date in Libertinus on a halo of the sea reads; the credits at ten points
  read at arm's length on the phone. But the halo is the sea's tone and here the words lie over
  Brazil, so it is a sticker. Either the globe leaves the foot clear (as Terra's caption does) or
  the halo takes the ground under it. A sticker over the land fails your own rule." Verdict:
  fails until the layout leaves the foot clear (Engineering's View) or the halo samples the ground.
- *The sea at rest.* "Not shown. You have a palette and no picture of the streamlines resting
  with the floats. Nothing to judge."

**Holt** (on the whole, from the studies): "I can see what I would have done: put a float, pick it
up and see its maybes, and the words at the foot ask nothing of me. Nothing in these pictures
points or waits, and the one warm thing is mine. But a study cannot show me wandering, stopping or
undoing; the ring's beat is five stills. The unscripted thing is there in the wake, bunched and
strung by a real current, which no one drew. Judge again when it runs."

**Later in R0, the passes running on wgpu.** The same scenes drawn by the look's own shaders and
Rust through `tools/adrift/lookbench` on Mesa's lavapipe (`studies/wgpu-held-day-phone.png`,
`studies/wgpu-held-night-phone.png`, `studies/wgpu-foot-phone.png`), over the stand-in's sea. The
hater, again: "It is the numpy picture to the pixel, which is the point. Two things only the running
build showed: the silver was a row of dashes until its dark lines and its wires were drawn as two
layers (every line, then every wire), and the credits at ten points come out a hair heavier than
the numpy twin's, the mipmapped distance field's doing; acceptable, watch it on the device. The
foot still lies over Brazil." Verdict unchanged otherwise; the layout of the foot stands as the one
thing not in Visual's hands.

**What Visual takes from R0 into R1** (in the shaders, with `adrift-check` renders to show it):
the beads and the silver scale with zoom (a floor of 0.8 pt, full size from zoom 2.5); the night
maybes at a third; the ring's beat as one continuous function with the amber from 0.5 of the beat;
the sea at rest drawn and photographed; the maybes as point sprites, costed on the device; the
foot's layout raised with Engineering (a demand if the View's disc does not leave it clear).

## R1, first look (October 31, 2026): the marks in the app, through `adrift-check`

Judged: `studies/check-gulf-stream-close-phone.png` (two floats put twenty days before, one held,
at zoom 5 off Hatteras: `adrift-check render scenes/gulf-stream-close.json`, the wiring of
DEMANDS.md D1 applied), `studies/check-atlantic-phone.png` (the whole Atlantic at zoom 1, three
floats), `studies/check-foot-phone.png` (the date and credits under the disc after
`plans/put-and-drift.json`). All on Engineering's ground and sea, on lavapipe.

**The hater, a reviewer who gives apps one star for a single pixel out of place.**

- "The floats are what the studies promised, on the real sea, and the words sit under the disc on
  the paper, so the sticker is gone. One star back."
- *The wake at zoom 5.* "Twenty beads in slow water pile into a caterpillar behind the float: each
  bead's dark edge over the last bead's amber makes a brown worm. In a jet they string out and are
  beautiful; in an eddy they are a stain. Draw a bead's edge only where it isn't over another
  bead, or draw all edges then all beads, as you did the silver." Verdict: fails in slow water.
- *The whole Earth.* "At zoom 1 the floats are nine-point rings with a dot of amber: findable,
  and the wakes thin as asked. Fine."
- *The ring's beat.* "Still not in the running build: the float appears from nothing at the beat,
  because the gestures keep the resting finger to themselves (D2). Not judged until it is."
- *The maybes and the silver.* "Not in the app; Truth's. The bench shows what they will be."
- *The sea at rest.* "Still unphotographed."

**Holt**: "In `put-and-drift` I put a float, it drifted forty days, I stopped the date and took it
back, and the pictures show each state with nothing pointing at me. The ring's thickening is the
one promise the build doesn't yet keep. Something unscripted: the float in the Sargasso barely
moved in forty days, which no one chose."

**Done the same day**: the beads are drawn edges then beads (`wake.wgsl`, two layers), so a bunch
stays amber; the bench shows it (`tools/adrift/lookbench`).

**The sea at rest, photographed** (`look/plans/sea-at-rest.json` through `adrift-check play`;
`studies/check-sea-running-then-at-rest-phone.png`, the last running frame beside the first at
rest): running, three frames 0.3 s apart differ by 0.16–0.21 grey levels a pixel on average (the
light runs, the beads advance); stopped, three frames differ by exactly 0 (the light holds, the
glitter is gone), and the held light sits at half its running strength (`sea.wgsl`). The hater:
"It rests. But it drops to rest in one frame, a cut, not a settling; an eased rest wants a value
the app owns, which is next round's ask, not a shader's." Verdict: the rule holds; the ease is owed.

**Into the rest of R1**: the sea at rest's ease (a demand on Engineering's View in R2); photographed in a play (the
light held, the glitter still); the ring's beat the moment D2 lands; the night palette waiting on
a night.

**Later in R1, on the moved skeleton** (maybes on the GPU, the stage to a texture, a text tool of
Engineering's): the maybes pass draws Engineering's buffer with the designed specks (the bench
shows it, two thousand from a buffer), the words gained an italic note and the credits moved to
the foot, each glyph on a halo of its own ground (`studies/check-foot-phone.png`,
`check-foot-over-globe-phone.png`: both the look's words and R0's un-haloed chrome are in these,
since the wiring isn't merged; the hater: "two of everything is worse than either; pick"). The
maybes in the app all sit under the float: Truth's R0 rules have no wander yet, so every self takes
the same path; nothing for the look to fix, and the first thing the cloud will show when the wander
lands.

**What stays lit, photographed** (`look/plans/stays-lit.json`; `studies/check-stays-lit-phone.png`:
a float just put, the same four seconds on, then a second put beside it): the wide amber glow after
the touch, gone when it settles, and only the newest touched one lit. The hater: "It stays lit, and
only it. The glow is a haze two and a half radii wide; on the pale sea it reads as a smudge for the
first second. Try it a radius tighter on the device." Verdict: the rule holds; the reach is a knob.
On the iPad (`studies/check-gulf-stream-close-ipad.png`, 2 px a point, reading 1.2) the floats and
the beads read at the same apparent size as on the phone, as the reading scale means them to.

## R2 (October 31, 2026): the merged build, and the night

Judged: `studies/r2-merged-gulf-phone.png` and `r2-merged-gulf-ipad.png` (the merged app at
5c21f63: Truth's maybes with their wander, the wake with the drifters' memory, Motion's touch),
`r2-merged-foot-phone.png` (the words over the globe), `r2-night-gulf-half-bloom-phone.png` (the
night palette in the ground and the sea, the maybes half-way out of the float: the night and the
bloom forced in a local build, since D3 and D4 are open), `r2-night-atlantic-phone.png`.

**The hater, a Blue Marble cartographer who thinks every night map since 2012 is a cliché.**

- *The merged day.* "Now the maybes are a cloud a current made: dense along the stream, thin on
  the shelf, grey where a self went aground. The wake loops where the drifters' memory took it. I
  can't call that scripted. The specks are a little many on the phone: two thousand at 0.75 points
  is a sandstorm when the cloud is tight; thin them with density, not count." Verdict: passes; the
  sandstorm is a knob.
- *The iPad.* "Same marks, same apparent size, more sea round them. Fine."
- *The bloom.* "Half-way out, the specks sit in a ring round the float as if thrown. That is the
  right picture at 0.15 s; the ease is the app's (D3). Not judged in motion."
- *The night.* "The N_ palette on the ported ground: slate land, a dark sea by depth, the ice grey,
  the limb's glow. The amber lamps are the warmest thing on the screen, as the brief wants. But the
  surf was a white glare along every coast until it was cut to a third, and the ground has no
  cities: without Terra's night lights it is a planet nobody lives on. Ask for the sheet." Verdict:
  the palette holds; the lights are owed (D4's answer should bundle `night.jpg`).
- *The words.* "On a halo, under the disc or over the globe, one set now. 'held' as the note when
  the date is stopped is the app's wording and it is right: one word, no icon."

**Holt**: "I held a float and its other lives were the current's, not a designer's; I stopped the
date and the sea stood still with the words saying so. Nothing asked me to do anything."

**Into the rest of R2**: the maybes thinned by density where the cloud is tight; the night lights
when the sheet arrives; the eases (D3) photographed in a play; the silver when Truth hands the
track; the lookbench retired by Engineering.

**The silver with a real track** (`studies/r2-silver-drifter-phone.png`, the merged build at
d2bd376 with Truth's nearest run and its label): a buoy's whole run across the frame at zoom 5,
rounded through its daily points so its eddies read as loops, and "Drifter 132676, 1 Jan 2016…"
in italic on a halo at the point nearest the held float. The hater: "A real buoy with a number
and a date is the most honest thing on the screen, and the loops are the Gulf Stream's own. The run
is too long for a hold: it leaves the frame both ways and reads as a border. A window of days
round the nearest point (Engineering is passing one) will make it a visit, not a fence. The label
sits on the track; move it off." Verdict: passes once windowed; the label's offset is a knob.

**The bloom and the fold, photographed** (`look/plans/bloom.json`: a float twenty days adrift, the
date stopped, held by intent and let go where it is; `studies/check-bloom-and-fold-phone.png`,
frames at 0.02, 0.12, 0.22, 0.32 and 0.62 s of the hold, then 0.08, 0.16 and 0.36 s after the
lift): the specks travel out of the float to where the current would have taken each self, the
silver and its label rise with them, and on the lift they fold back into the float and the silver
sinks, the label staying at its point. The hater: "Out means out now, and back means back; the
fold is quick enough that nothing lingers to be mistaken for a mark. The first held frame is
already a third of the way out: the ease is front-loaded, which is right for a hand that wants an
answer. The silver's whole run is still a fence until it is windowed." Verdict: passes; the
window is Engineering's.

**The night, in the app** (`adrift-check render scenes/gulf-stream-night.json`, Engineering's D4
answer: the night from the shell, `night.jpg` bound; `studies/check-night-gulf-phone.png` at device
pixels, `check-night-atlantic-half-phone.png` at half): the N_ ground with the Black Marble's
lights in sodium gold whitening at the cities' cores, the sea by depth, the lines pale, the amber
lamps the warmest thing on the screen, the words pale on halos of the night's sea. The hater:
"Now it is lived on. The eastern seaboard reads as itself at zoom 5 and the lamps still win. The
maybes by night are a dust at a third, right. The one thing: the Gulf Stream's light runs white
over a black sea and the silver is white too; at the whole Earth they are the same thread. Tint
the silver a hair warmer by night, or the light a hair cooler." Verdict: passes; one hue apart.

**Holt**, on the day's merged build and the night: "I held, I let go, I stopped the date, the
room went dark and the floats stayed mine. Nothing marked, nothing pointed, nothing waited."

**The press, with the lamp on Put** (`look/plans/press.json`; `studies/check-press-phone.png`:
0.10, 0.25 and 0.37 s into a rest, the put at 0.43, held at 0.73): a tap's ring, the ring still,
glass formed without a lamp, then the float with its lamp lit and its maybes and silver out.
Motion's hater had seen a lit float vanish on a lift just short of the beat; now a lift before the
beat only un-forms glass, and the beat is the moment the light comes on. Verdict: passes.

## R3 (October 31, 2026): the whole Earth, and the knobs

Judged on the merged build at 37e8ee1 (`look/plans/atlantic-held.json`, a float held at zoom 1, phone
and iPad; `scenes/gulf-stream-close.json`; `look/plans/stays-lit.json`): `studies/r3-before-whole-earth-held-phone.png`
(as merged), `r3-whole-earth-held-phone.png` (after), `r3-gulf-stream-close-phone.png`,
`r3-stays-lit-phone.png`, `r3-foot-ipad-half.png`.

**The hater, a phone reviewer who holds the thing at arm's length and squints.**

- *The maybes at zoom 1.* "As merged: two thousand specks over a thumb of ocean, a smudge under
  the halo, Motion's hater was right. After: a quarter of them, fainter, in the cloud's own shape
  (a hash of each speck's number picks it, so coming and going the dusting keeps its form). It is a
  cloud now, not a stain. At zoom 5 nothing changed." Verdict: passes.
- *The label at zoom 1.* "A buoy's number and two dates are a banner across the Atlantic at the
  whole Earth. Gone below zoom 1.5; the silver stays, which is enough to say a real one passed."
  Verdict: passes.
- *What stays lit.* "A radius tighter. Still a glow, no longer a smudge; the sea's own ring from
  the put spreads round it, which says 'just now' better than the glow does." Verdict: passes.
- *The wake's oldest beads.* "They end at a tenth instead of nothing; the tail has an end you can
  see. Fine." Verdict: passes.
- *The lamp by day at zoom 1.* "Nine points, dark rim, amber heart: I find it across the globe at
  arm's length. Leave it." Verdict: passes.
- *The iPad's foot.* "Date and note under the disc, the credits a colophon at the very bottom, a
  hand's width of paper between. It is a page, not a dashboard; I expected to mind and don't."
  Verdict: passes.
- *A credits panel.* "Nobody asked for it and I won't. One line names the sources; the device's
  settings can hold the five sentences if the owner wants them." Verdict: not built.

**Holt**: "At the whole Earth I can still see what I did and what the sea did with it, and when I
hold a float the cloud is a hint, not a wall. Nothing marks, nothing points, nothing waits."

**Left for the device's captures**: the night on an actual OLED (the N_ page may read as grey),
the speck's three pixels on the real panel, and the words' halos at the device's gamma.
