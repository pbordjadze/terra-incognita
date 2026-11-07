# Throats: the critic's brief

Given word for word to a fresh critic subagent each round (painting-company row 257), with the round's
evidence directory and the log of earlier verdicts (`toys/throats/ROUNDS.md`). It is told nothing of
the builder's reasoning or of what changed. `evidence.sh <round>` makes the directory.

---

You judge a small toy for its owner, Paul, who wants to hand it to a friend without a caveat. It is an
iPhone and iPad app. What it was asked to be: a sunlit rocky California hillside seen from above, full
of tiny side-blotched lizards with orange, blue and yellow throats. The colours chase each other round
in slowly turning spirals. Orange takes blue's rock, blue takes yellow's, yellow takes orange's; that's
the whole rule. A finger drawn across it pours in a stroke of one colour, which breaks up within seconds
into new spiral arms. Pinch out to see the whole hillside, in to see the lizards.

Paul's taste, the measure: something happening he can touch; beautiful at once; discovery through scale
(the closer you go, the more there is); no text he didn't ask for, no chrome; things living in the
world, not pinned to the glass; would he show it without a caveat? The small "i" and "?" marks bottom
right are the kit's (a demo of the gesture and the rules): ignore them.

The evidence (in the directory you are given; look at the pictures yourself):
- `*.png`: full-size phone frames (1179 x 2556, the phone's own pixels; a lizard is drawn at the size
  the phone shows it). `crops/` holds parts of them at 1:1. Open them at full size: a downscaled view
  is not what the phone shows.
- `films/<plan>/`: played plans. `film.mp4` (the frames at 10 fps), `sheet.png` (a contact sheet),
  `frames/` (frames 0.1 s apart, full size), `plan.json` (what the fingers did: times in seconds,
  places in points on a 393 x 852 point phone; `pinch` gives two fingers and the spread they end at).
  Use ffmpeg if you want other frames from the MP4.
- `ROUNDS.md`: earlier rounds' verdicts.

Judge only from the evidence. Report in under 300 words:
1. What Paul notices first, in his first five seconds.
2. The weakest thing (one), and what a fix would have to achieve (not how to code it).
3. The one change you would make next.
4. What is already good and must not be lost.
5. What you couldn't judge from this evidence.
6. Is the weakest point polish, or more? (one word)
7. Ship as is? yes or no.
