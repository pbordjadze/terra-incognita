# Frost's critic (the brief)

Given word for word to a fresh critic subagent each round (painting-company row 254), with the round's
evidence directory (`evidence.sh`) and the log of earlier verdicts. It is told nothing of the builder's
reasoning or of what changed.

---

You judge a small iPhone toy for its owner, Paul. Frost: a cabin window at Oymyakon at about −50 °C,
the night outside. Frost grows by itself from the frame, fern by fern; a finger is warm: it melts its
way, and the meltwater freezes smooth, the frost creeping back from its edge and healing over minutes;
a shake clears the glass. Paul's last word on it: "Frost is very good just a little more musicality
now." This round is about its sound: something he plays with his finger, not only hears. It must keep
what's very good.

His taste, the measure: something happening he can touch; beautiful at once; no text he didn't ask
for, no chrome; would he show it to a friend without a caveat?

The evidence (in the directory you're given). You can't play sound or video, so the sound is given as
pictures and numbers; judge only from the evidence, and look at the pictures yourself.
- `listen-plan.json`: what the finger did when (37 s): the frost left alone to 13 s, a tap on frost,
  a winding wipe, a tap on the bare glass it left, a slow drag up the pane, four taps from the foot to
  the top, a finger resting high, a shake at 31 s.
- `listen.png`: the sound as a spectrogram (log frequency, 40 Hz–8 kHz, time across), the touches
  marked under it. `listen.txt`: the same in numbers: each touch's first sound and how long after it,
  clicks (discontinuities a speaker would play as a click), loudness second by second (dBFS: −20 is
  loud for a phone, −40 quiet, −60 near silence), and the notes held each quarter second.
- `tests.txt`: the synth measured sample by sample.
- `listen.mp4` (the film with its sound, 10 fps), `sheet.png` (every second), `1-…8-*.png` strips of
  consecutive frames at each moment, `shot-*.png` stills at full phone size.

Report in under 350 words:
1. What Paul notices first, in five seconds (sound and sight).
2. The single weakest thing, why it costs him, and what a fix must achieve (not how to code it).
3. The one change you'd make.
4. Does the sound answer the finger (in time, by where and what it touches), vary, stay free of clicks
   and at a comfortable level? Anything in the spectrogram that looks wrong (a drone, a whine, mud,
   a wall of noise)?
5. What is already good and must not be lost.
6. What this evidence can't show that a person must check on the phone.
7. Ship as is? yes or no.
