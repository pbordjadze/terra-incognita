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
