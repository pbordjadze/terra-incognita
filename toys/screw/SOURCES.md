# Screw: what is real and what is drawn

**The song is the reader's own.** It is opened from Files through the sleeve (the kit's
`Toy::wants_file` / `Toy::opened`; the shell decodes it with AVAudioFile), kept in the app's own
storage (Application Support, not Documents) and handed back at each launch. It is never bundled,
committed, uploaded by CI or fetched; this repository holds no song. The checks play a practice
record synthesized in `src/practice.rs` (76 BPM; nobody's music).

**The moves are a Houston DJ's** (DJ Screw, Houston, the early 1990s onward; the "screwed and chopped"
tapes):

- *The screw*: a turntable's pitch ridden down. A record read slower is lower in pitch by the same
  ratio; nothing here is time-stretched (`deck.rs`: the record read at `vel`, by Catmull-Rom between
  samples, a low-pass when it runs faster than recorded). Screw tapes usually sit around two thirds
  to three quarters of the record's speed; the fader runs from 0.5 to 1.08, with a haptic tick at
  as-recorded and at three quarters.
- *The chop*: two copies of the same record a beat apart and the crossfader cut between them, so a
  phrase repeats and the song keeps its time. Here the needle goes back exactly one beat of the song,
  with a 10 ms eased crossfade between the two places (the tests show no click at the cut and the
  beats staying evenly spaced across it, at 1.0 and at 0.7).
- *A hand on the platter*: a palm resting on a turning record drags it slower and lower; lifted, the
  motor pulls it back. Here a finger resting on the record drags it to 0.7×.
- *Bring it back* (a rewind: the platter spun back by hand, here two bars) and *the brake* (the motor
  off: the platter slowing to a stop in about two thirds of a second, as a direct-drive deck's does).
  The platter turns at 33⅓ rpm times its speed.

**The beat** is found from the song (`tempo.rs`): onsets in three bands, the period that folds them
onto one phase, its level chosen near 82 BPM, refined over the whole song. Checked on the owner's
first song (a 2012 mixtape track, kept off this repository) against librosa's beat tracker: the same
beat to the millisecond, at half its tempo.

**Drawn**: the record, its label and grooves, the lamp's light on them, the purple round a slowed
record (the colour Houston's slowed music is known by), the song's shape round the rim (its
loudness, start at the top, clockwise) and the needle's place on it.
