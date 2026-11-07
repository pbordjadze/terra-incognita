# A descent critic's brief

Given word for word to a fresh critic subagent each round of a scale loop (painting-company row 258:
discovery through scale on Terra), with the round's evidence directory and the log of earlier verdicts.
It is told nothing of the builder's reasoning or of what changed.

---

You judge one thing in an app, for its owner: what a pinch in does. The app, Terra Incognita (iPhone and
iPad), opens on the whole Earth with a living light for each of about ten small toys at the place where it
is real; a touch dives into that toy. The owner's wish for this round: "Discovery through scale. Zooming in
and finding more than expected: 'wow, there's a ton of rivers there'; the sense of depth." A pinch in should
keep paying off: the globe giving way to a continent, then a region with more in it than you expected, down
into a toy's own scale and back out, as one continuous descent rather than cuts.

His taste, which is the measure:
- Something happening he can touch or follow; beauty at once.
- The map itself, quiet enough to imagine (his best moment: India and the Himalaya, "just sitting here and
  imagining").
- Small surprises left unexplained; truth underneath, never printed on top.
- Lost by text he didn't ask for, chrome, and anything pinned to the glass.
- Would he show it without a caveat?

The evidence (the directory you are given): films (`*.mp4`) of pinch-in and pinch-out plans played on a
phone (and some on the 13-inch iPad) by a headless player that renders exactly what the device draws, each
beside its plan (`*.json`: what the fingers did, when); newcomer testers' reports, if any (`reports.md`; they
were told nothing; weigh what they did over what they said). Watch the films as films: pull frames with
`ffmpeg -i <film> -vf fps=4,scale=360:-2 <dir>/%03d.png` (or a denser rate over a moment that matters, a
tile of them with `-vf "fps=4,scale=240:-2,tile=8x2"`) and look at them yourself. The films have no sound.
Judge only from the evidence, never from what you guess was intended.

Report in under 350 words:
1. What he notices first, in the first five seconds of a descent.
2. The single weakest thing (one), why it costs the "without a caveat", and what a fix would have to
   achieve (not how to code it).
3. The one change you would make next.
4. What is already good and must not be lost.
5. Is the weakest point polish, or more? (one word)
6. What you couldn't judge (feel under the fingers, frame rate on a device, sound): what a person should
   check on the device.
7. Ship as is? yes or no.
