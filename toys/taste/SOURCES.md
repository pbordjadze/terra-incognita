# Taste: sources

Nothing in Taste is measured from the world except the reader's finger. Every form is drawn from code
(`src/creature.rs`); no image, recording or dataset is used.

## What is real

- **The walk** (`src/walk.rs`) is a Metropolis walk (Metropolis et al., 1953): from the form it is, the
  creature proposes a form a random step away (a gaussian in each of its eight numbers, mostly small,
  one step in five a leap; colour wrapped round, the rest reflected at their ends, so the proposal is
  symmetric), shows it, and keeps it with probability min(1, (L(new) / L(old))^2.5), else goes back.
  Every step lasts as long, kept or not, so the time on screen is shared as the rule shares it. With a
  fixed L the walk's time at each form is proportional to L^2.5 (`walk.rs`' test
  `a_fixed_liking_is_sampled_in_proportion`).
- **The target** L is the reader's: every moment the creature is shown is remembered (the form they saw
  0.3 s earlier, their eye's lag) with whether a finger rested on the screen, and L(form) is the held
  time near that form over the shown time near it (a gaussian of width 0.2 in the eight numbers), with
  0.8 s of an even prior (held 30% of the time) so a form never seen is neither loved nor shunned. The
  power 2.5 makes the walk keep to what was held more firmly than the bare ratio (a cooler walk on the
  same liking); it is still the reader's attention, and nothing else, that it samples.
- **What changes as you watch.** L is learned while the walk runs, so each step uses L as it stands
  then: the walk samples the reader's attention as far as it is known, not a fixed density given in
  advance. Nothing in the rule points the creature at a favourite: no gradient, no memory of the best
  form, no steering. Ignored, it seeks forms it has not shown (they keep the prior while shown ones
  fall); held everywhere, it wanders evenly.
- **The wheel** (the pulled-back view) is a picture of two of the eight numbers: colour round the
  wheel, spikes outward. The path joins the forms kept, in order; the short twigs point toward tries it
  turned down; the light under the path is L at each kept form; the small creatures are the forms kept
  longest, none crowding another.

## Tuning

`src/bin/taster.rs` plays pretend readers with a taste of their own (a favourite in two or three of the
numbers; they hold while what they saw is close enough) and prints how near the creature stays to the
taste, minute by minute, against a reader who never touches. Its numbers set the step, the leap, the
width of "near" and the power (Nov 4, 2026: by the third and fourth minutes the creature spends about a
third more of its time near a pretend reader's taste than near nothing). The start plan's holds are
one such reader's.

## Drawn

The creature (shape, colour, markings, glow, motion from its eight numbers), the ground, the wheel's
tints, and the sounds (a struck note for each form kept, its pitch from the colour; a low soft note for
each one turned down; a hum while held).
