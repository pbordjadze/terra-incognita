# Throats: sources

What is real and what is drawn. Every source here was fetched (Nov 7, 2026); nothing is quoted from
memory. Throats loads no data: the hill, its lizards and the rule are drawn and run from code.

## Real: the lizards' game

- **Sinervo, B. & Lively, C. M. (1996).** The rock–paper–scissors game and the evolution of alternative
  male strategies. *Nature* 380(6571), 240–243. doi:10.1038/380240a0. The citation as Crossref gives it
  (https://api.crossref.org/works/10.1038/380240a0); the paper's own abstract could not be fetched (the
  publisher's page asks for a sign-in).
- **UC Santa Cruz news (Feb 2010),** "Evolutionary game of rock-paper-scissors may lead to new species"
  (https://news.ucsc.edu/2010/02/evolutionary-game-of-rock-paper-scissors-may-lead-to-new-species/), on
  Sinervo's side-blotched lizards (*Uta stansburiana*): "Males with orange throats can take territory
  from blue-throated males"; "Blue-throated males cooperate with each other to defend territories and
  closely guard females"; "Yellow-throated males are not territorial, but mimic female behavior and
  coloration". Orange beats blue, blue beats yellow, yellow beats orange. The study population is
  "near Los Banos, Calif."; there the most common colour changes every four to five years.

What the toy takes from it: the three throats and who takes whose territory. What it does not: in
the field the game is played in frequencies over years, not in travelling waves over seconds; the
toy's spirals are what the same game does in space, which the next sources show.

## Real: spirals from rock–paper–scissors in space

- **Reichenbach, T., Mobilia, M. & Frey, E. (2007).** Mobility promotes and jeopardizes biodiversity in
  rock–paper–scissors games. *Nature* 448(7157), 1046–1049. doi:10.1038/nature06095. PubMed's abstract
  (PMID 17728757): below a threshold of mobility "all subpopulations coexist and an entanglement of
  travelling spiral waves forms"; above it, "biodiversity is jeopardized and lost".
- **Kerr, B., Riley, M. A., Feldman, M. W. & Bohannan, B. J. M. (2002).** Local dispersal promotes
  biodiversity in a real-life game of rock–paper–scissors. *Nature* 418(6894), 171–174.
  doi:10.1038/nature00823. PubMed's abstract (PMID 12110887): three populations of *Escherichia coli*;
  diversity is "rapidly lost" when dispersal and interaction are over large scales, and all coexist
  "when ecological processes are localized".

## The rule as the toy runs it

- **Fisch, R., Gravner, J. & Griffeath, D. (1991).** Threshold-range scaling of excitable cellular
  automata. *Statistics and Computing* 1(1), 23–39. doi:10.1007/BF01890834 (arXiv patt-sol/9304001; the
  full text at https://www.math.ucdavis.edu/~gravner/papers/fgg.pdf). A site advances "if at least θ
  neighboring sites within range ρ" hold the next colour; cyclic cellular automata from random starts
  show "nucleation of locally periodic 'ram's horns' or spirals".

`src/sim.rs` is that rule with three colours, made stochastic: a lizard loses its rock to the colour
that beats its own when at least 5 of the rocks within 2 (a 5 x 5 box) wear it, with probability one
half a step, 2.2 steps a second. The Reichenbach–Mobilia–Frey model itself (selection, reproduction
into empty rocks, swapping) was tried first (`examples/` history, ROUNDS.md): at sizes that fit a phone
it gave muddy patches, not arms.

## Drawn

The speed of the game and the size of the hill (512 x 512 territories, a torus). Outcrops and shrubs hold
no territory and are left black.

## The look

Drawn in the style of John Whitney's *Permutations* (UCLA, Los Angeles, 1968), made with Jack Citron on
an IBM System/360 and its 2250 display.

- "Architectures of color dots that develop pattern while displaying a kinetic rhythm", "constructed
  entirely off the black-and-white monitor of a large computer system (IBM 360, IBM 2250 Display, written
  in GRAF and FORTRAN)"; "color was added by editing with an optical printer" (Visual Music Archive,
  *Permutations*). The 2250 could "control an external camera to film image sequences programmed by
  punched cards" (Animation Studies blog, *Experiments in motion graphics, or when John Whitney met Jack
  Citron and the IBM 2250*). "Recorded in black and white from the monitor of an IBM 360 mainframe…
  Color was added afterwards" (Motionographer, *John Whitney: Permutations*).
- So each territory is a dot of light on black (`hill.wgsl`), and each throat's colour is its own pass:
  orange, blue and yellow dots are printed separately, each pass a little off the others, differently
  every frame; 24 frames a second.

**Drawn, not sourced:** the dots on a lattice that sways on two slow waves and swells a little as a slow
pulse runs out across the field; a newcomer's dot swelling in and the loser's opening into a fading ring;
close in, each dot a rosette of six turned to its lizard's heading; the dots' bloom and the print's grain;
the three colours (the toy's own throats).
