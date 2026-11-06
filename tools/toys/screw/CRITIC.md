# Screw's critic (the brief)

Given word for word to a fresh critic subagent each round (painting-company row 250), with the round's
evidence directory (`evidence.sh`). It is told nothing of the builder's reasoning or of what changed.

---

You judge a small iPhone toy for its owner, Paul, who will open it on his phone tonight. It is his
first-ever try at a musical craft, and he says he is truly not musical. The craft is chop and screw,
Houston's, from DJ Screw: a record slowed on a turntable's pitch so voices drop thick and low (the
screw), and a phrase repeated by cutting between two copies of the record a beat apart (the chop).

The toy: a record turning on a dark page, his own song on it (opened from Files through a small
sleeve, bottom left). A tap plays it. A finger pushed down the record slows it, voice and all, and it
stays there. A tap says the last beat again; held, it keeps saying it. A flick to the left spins it back
two bars. A tap on the label is the brake. An "i" plays a ghost hand doing it; a "?" has the rules; an
asterisk, where they came from. In the evidence a synthesized practice record (76 BPM, drums, bass,
piano, a buzzy synthetic voice) stands in for his song.

His taste, the measure: "oh huh neat" within seconds, before anything is explained; something
happening that a finger can touch; nothing to learn first (no numbers, no BPM, no knobs, no grid); the
moves he learns are the real ones, so his skills carry over; minimal; nothing that feels
AI-generated or templated; lost by text he didn't ask for, chrome, friction, dead touches.

The evidence (in the directory you're given): `start.mp4` (the first 24 s at phone size, with sound;
you can't play it, but `1-…6-*.png` are strips of consecutive frames at each touch: the tap that starts
it, the push down, a chop, the hold, the flick back, the brake; `start-plan.json` says what the finger
did when), `shot-*.png` stills (the top line is the song's name, as the phone's system draws it),
`listen.png` and `listen.txt` (the sound as a spectrogram, log frequency, the plan's touches marked
under it: a slowed record's harmonics fall, a chop is a block said twice, a rewind a swoosh),
`tests.txt` (the sound measured sample by sample: pitch and speed together, chops a beat apart, no
click at the cut), `info-sheet.png` (the "i" demo), `rules-*.png` (the cards), `first-launch.png` and
`first-tap.png` (the very first launch, before any song of his own, and its first tap). Look at the pictures yourself.

Report in under 350 words:
1. The single weakest thing for Paul's first minute (one), why it costs the "oh huh neat", and what a
   fix must achieve (not how to code it).
2. The next two weakest, a line each.
3. What is already good and must not be lost.
4. Does the sound, as far as the spectrogram and the tests show, do what the toy says (slow and low
   together, chops in time, no clicks)? Anything in the picture that looks wrong?
5. Verdict, one word: ready / polish / more.
6. What this evidence can't show that a person must check on the phone.
