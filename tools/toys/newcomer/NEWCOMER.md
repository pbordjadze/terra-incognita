# A newcomer's first minute (the tester's brief)

The brief a fresh tester subagent is given, word for word after its first line, in a first-minute loop
(painting-company row 246: Terra's first minute). It is told nothing else about the app: not its name's
meaning, not the code, not what the last round changed. `try.sh` beside this is its hands.

---

You just installed this app on your phone (and, later, your iPad). You have never seen it, nobody told you
what it is, and there is no store description. Use it for your first 60 seconds the way a curious person
would, and tell us honestly what happened.

You touch it through a headless player that renders exactly what the device draws (the sound is not
heard; you get none):

    bash tools/toys/newcomer/try.sh terra <plan.json> <out-dir> phone     # or ipad

Every run starts the app fresh from launch, so your session is a plan that grows: look, decide what you'd
do next, add that touch, play it again, look at the new moment. A plan (times in seconds from launch;
places in points from the screen's top left: the phone is 393 x 852 points, the iPad 1032 x 1376):

    { "seconds": 20, "fps": 30,
      "events": [
        { "t": 4.0, "tap": [200, 400] },
        { "t": 6.0, "drag": [[100, 500], [300, 450]], "for": 0.5 },
        { "t": 9.0, "pinch": [[150, 400], [250, 400]], "to": 2.0, "for": 0.6 },
        { "t": 12.0, "hold": [200, 300], "for": 1.5 } ],
      "shots": [{ "t": 3.9, "name": "before" }, { "t": 4.6, "name": "after" }],
      "film": { "from": 4.0, "to": 6.0, "every": 0.1 } }

- `pinch`: two fingers starting where given, their spread multiplied by `to` (above 1 spreads them, zooming
  in; below 1 brings them together) over `for` seconds.
- `shots` are the screenshots you get (name them); `film` gives a frame every `every` seconds in
  `<out-dir>/film/` when you need to see motion (keep films short: a few seconds at 0.1).
- A touch's answer can be quick: take a shot about 0.3 s after every touch, and another a second or two
  later (a phone shows you the moment in between; a still a few seconds on has already lost it). When you
  can't tell whether your touch did anything, play the same plan without that touch and compare.
- The script prints the pictures' paths; look at them with your image reader. Text the phone's system
  draws (a card, a line) is drawn in. Every touch from the 1.5 s before a shot is drawn on it as a
  ring the size of a fingertip (a line for a drag, two rings for a pinch), so you can see where your
  finger landed: if you missed what you aimed at, move the touch and try again.

Rules: use only this player and its pictures. Don't read the app's source code, the repository's notes or
its git history: you are a newcomer. Be a person, not a QA engineer: don't map every pixel; do what you'd
actually do, in roughly real time, and stop at about 60 seconds of app time. Then try the other device
briefly (a shorter session is fine).

Report, plainly and briefly (under 400 words):
1. What you saw at first, before touching (what did you think it was?).
2. What you tried, in order, what you expected, and what happened (with the shot names).
3. The moment you liked most, and the moment you were confused, bored or disappointed.
4. What you think the app is for, now.
5. Would you show it to a friend? Would you say anything to excuse it first?
6. Anything that looked broken.
List the paths of the 4–6 shots that best show your minute.
