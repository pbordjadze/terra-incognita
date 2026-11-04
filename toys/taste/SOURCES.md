# Taste: sources

Nothing in Taste is measured from the world except the reader's finger. Every form is drawn from code
(`src/creature.rs`); no image, recording or dataset is used.

## What is real

- **The walk** (`src/walk.rs`) is a Metropolis walk (Metropolis et al., 1953) over four numbers (colour,
  spikes, arms, glow; the creature's other four traits, curl, markings, second colour and temper, follow
  from them smoothly: `creature.rs`' `body`). From the form it is, the creature proposes a form a random
  step away (a gaussian in each of the four numbers, mostly small,
  about one step in seven a leap; colour wrapped round, the rest reflected at their ends, so the proposal is
  symmetric), shows it, and keeps it with probability min(1, (L(new) / L(old))^3), else goes back.
  Every step lasts as long (about 1.5 s), kept or not, so the walk's time at each kept form is shared as the rule shares it. What the screen shows is not only the kept form: about half of each step shows the try, and a try turned down is on screen too before it goes back. With a
  fixed L the walk's time at each form is proportional to L^3 (`walk.rs`' test
  `a_fixed_liking_is_sampled_in_proportion`).
- **The target** L is the reader's: every moment the creature is shown while the reader is with it (a
  finger on it now or within the last 6 s; before the first touch, watching counts too) is remembered
  (the form they saw 0.3 s earlier, their eye's lag) with whether a finger rested on it, and L(form) is
  the held time near that form over the shown time near it (a gaussian of width 0.17 in the four
  numbers), with 0.8 s of an even prior (held 15% of the time) so a form never seen is neither loved
  nor shunned. Moments while the reader is away (the finger lifted more than 6 s ago) are not
  remembered: watching without touching neither teaches it nor unteaches it, and the walk goes on
  sampling the taste as it was left. The power 3 makes the walk keep to what was held more firmly than
  the bare ratio (a cooler walk on the same liking); it is still the reader's attention, and nothing
  else, that it samples.
- **Begun again.** On the wheel a touch on one of the small kept creatures begins the walk again from
  that form (the path marks it as a fresh start, not a step): any kept form is a fair place to walk on
  from, and what it remembers is untouched.
- **A steady hold.** A finger held through everything credits every form shown, the tries too, so a
  reader who never lifts makes the liking even and the walk wanders evenly: it learns from the
  difference between what was held and what wasn't.
- **What changes as you watch.** L is learned while the walk runs, so each step uses L as it stands
  then: the walk samples the reader's attention as far as it is known, not a fixed density given in
  advance. Nothing in the rule points the creature at a favourite: no gradient, no memory of the best
  form, no steering. Ignored, it seeks forms it has not shown (they keep the prior while shown ones
  fall); held everywhere, it wanders evenly.
- **The light on the wheel** is the target itself, L^3, at every place on the wheel: for each colour and
  spikiness, the best over a few arm counts and glows (the wheel shows two of the four numbers), scaled
  to its brightest. The path crowds where it is bright because the walk's time follows it. The small
  kept creatures are the forms kept longest (by time alone), none crowding another.
- **Chosen again.** After a touch on the wheel brings back a kept form, its first rest is 2 s longer
  (the same rule, only slower to take its first step).
- **The wheel** (the pulled-back view) is a picture of two of the four numbers: colour round the
  wheel, spikes outward. The path joins the forms kept, in order; the short twigs point toward tries it
  turned down; the light under it is the target described above.

## Tuning

`src/bin/taster.rs` plays pretend readers with a taste of their own (a favourite in two or three of the
numbers; they hold while what they saw is close enough) and prints how close the creature's kept form
is to the taste, minute by minute, against the same creature with a reader who never touches. Its
numbers set the step, the leap, the width of "near", the power, the step's pace and how long a reader
counts as with it (Nov 4, 2026, four tastes by six seeds, readers who now and then rest a while: in the
third and fourth minutes the kept form's mean closeness to the taste is mostly 0.3–0.6 against
0.02–0.25 for the same creature untouched; two of the 24 (a reader of blue jellies, one of many-armed matte forms) never found theirs in four minutes; most readers who stop touching at two minutes stay about as close for the next two, though some drift off, by up to half).
About seven tries in ten are kept, so it still wanders. The walk is deterministic: `TASTE_TRACE=1` prints
each decision, and the check's walk matches the pretend reader's frame for frame. The start plan's holds are one such reader's.

## Drawn

The creature (shape, colour, markings, glow, motion from its four numbers), the ground, the wheel's
tints, and the sounds (a struck note for each form kept, its pitch from the colour; a low soft note for
each one turned down; a hum while held).
