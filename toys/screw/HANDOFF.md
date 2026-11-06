# Screw: handoff

painting-company row 250. Paul's first try at a musical craft: chop and screw, Houston's. A record
turning on a dark page, his own song on it. EXPERIENCE.md is what it should be (written before the
tools); SOURCES.md what is real; ROUNDS.md a line a round.

## How it works

- `src/deck.rs`: the turntable, on the audio thread (`DeckSynth` locks the `Arc<Mutex<Deck>>` the toy
  keeps). The record is read at `vel` (Catmull-Rom; a one-pole low-pass above 1.05×): pitch and speed
  together. `chop` goes back exactly one beat with a 10 ms eased equal-gain crossfade between the two
  places; `hold` repeats the last beat each time the needle comes back round; `rewind` spins two bars
  back with an eased hand curve and drops; `stop`/`start` are the brake (1.5/s) and the motor's pull
  (4/s). A soft ceiling above 0.9. Tests measure the sound (pitch at 0.7, clicks a beat apart across
  cuts, no corner at a cut, the hold, the rewind's landing, the brake).
- `src/tempo.rs`: the beat from the song. Onsets in three bands; the period that folds them onto one
  phase (16 s windows); the level nearest 82 BPM; refined over the whole song. The first version
  (autocorrelation with a prior) took a beat and a half for the beat on the practice record and on
  Paul's song; the fold doesn't (a dotted period spreads the onsets over three phases). Checked on
  Paul's song against librosa: 0.923 s, the same to the millisecond at half librosa's tempo.
- `src/lib.rs`: the hand and the picture. A finger that moves 10 pt is a drag: mostly vertical rides
  the fader (400 pt for 1.0; ticks at 1.0 and 0.75), mostly sideways is a flick (left, fast or 70 pt:
  the rewind). Not moved: up is a tap (a chop; on the label, the brake; stopped, start; no song, the
  Files picker), resting 0.32 s on a turning record is the hold. The record is a backdrop shader
  (grooves, the lamp's two wedges, the label turning with `turns`); the ring (the song's loudness,
  clockwise from the top), the needle, its way back (a fading streak), the sleeve and the finger's
  light are the painter's. The page goes purple as the record slows.
- `src/practice.rs`: the checks' record (76 BPM, synthesized). `src/bin/check.rs` writes it to
  `build/screw/practice.wav`; the plans `open` it.
- The kit (this row): `Toy::wants_file`/`opened`/`title`/`plays_music`, the shell's Files picker
  (audio and movie types: AVAudioFile reads an .mp4's sound), the kept file in Application
  Support/opened/, the title label, the playback category; `check`'s `open` event and
  `AudioFile::wav`; `shell_text.py` draws the title.

## Checking it

    cd toys && cargo test --release -p screw
    for p in start empty rules info; do cargo run -q --release -p screw --bin screw-check -- play screw/plans/$p.json build/screw/out/$p; done
    bash ../tools/toys/film.sh build/screw/out/start 30 15
    bash ../tools/toys/screw/evidence.sh build/screw/out <dir>      # the critic's evidence
    python3 ../tools/toys/screw/listen.py screw/plans/start.json build/screw/out/start   # the sound as a picture

The critic's brief is `tools/toys/screw/CRITIC.md`. A plan may `open` any WAV: for a real song, convert
it with ffmpeg into a scratch directory, never the repository.

## The song

Never in the repository, a commit, CI or the app. Paul's first, "Houston" (Paul Wall feat. Slim Thug &
Z-Ro, the closing track of No Sleep Til Houston, 2012), reached him as a private claude.ai artifact.
