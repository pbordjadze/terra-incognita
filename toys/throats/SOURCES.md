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

The hillside (soil, gravel, granite outcrops, sage, summer grass meadows, the afternoon sun from the
upper left), the lizards' bodies, their running in and off, push-ups, the speed of the game, and the
size of the hill (512 x 512 territories, a torus). A side-blotched lizard's throat is not seen from
above: the toy shows each lizard's colour on its chin, shoulders and flanks.
