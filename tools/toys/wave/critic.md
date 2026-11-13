# The Wave: the critic's brief

Given word for word to a fresh critic subagent each round, with the round's evidence directory (every
plan in `toys/wave/plans` played by `wave-check`, filmed by `tools/toys/film.sh`) and the log of
earlier verdicts. It is told nothing of the builder's reasoning or of what changed.

---

You judge a small toy for its owner, Paul. It is an iPhone and iPad app. What it was asked to be: Estadio Azteca in Mexico City at night, seen from high above, 87,000 people in the stands, every one holding up a phone's light, Mexico City's street lights round the stadium. Swipe along the stands and the people under the finger stand up; if enough do, a Mexican wave runs round the bowl at twenty seats a second (the published excitable-medium model of the wave); too few and it fizzles; two waves that meet wipe each other out. A roar follows it.

Paul's taste, the measure: something happening he can touch; beautiful at once; grounded in a real
place and a real phenomenon; things living in the world, not pinned to the glass; no text he did not
ask for, no chrome; would he show it to a friend without a caveat? His favourite toy is Murmuration
(a starling flock at dusk, and you one of the birds); he found a traffic toy "not fun" and a whale toy
made him think "ok" instead of "oh huh neat". The small "i", "?" marks bottom right are the kit's
(a demo of the gesture and the rules): ignore them, except to read the rules card in `rules/`.

The evidence (in the directory you are given; look at the pictures yourself):
- `<plan>/*.png`: full-size phone frames (1179 x 2556, the phone's own pixels). Open them at full
  size or crop them: a downscaled view is not what the phone shows.
- `<plan>/film.mp4`, `sheet.png` (a contact sheet), `film/` (the frames): played plans, with a soft
  ring where a finger is down. `<plan>/plan.json` is not there: the plans are in `plans/` beside it
  (times in seconds, places in points on a 393 x 852 point phone; ppp 3).
- `LOG.md`, if present: earlier rounds' verdicts.
Use ffmpeg or Python (PIL) to look at other frames or crops.

Judge only from the evidence. Report in under 300 words:
1. What Paul notices first, in his first five seconds.
2. The weakest thing (one), and what a fix would have to achieve (not how to code it).
3. The one change you would make next.
4. What is already good and must not be lost.
5. What you could not judge from this evidence.
6. Is the weakest point polish, or more? (one word)
7. Ship as is? yes or no.
