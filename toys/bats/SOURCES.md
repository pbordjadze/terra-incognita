# Bats: sources

What is real, what is drawn, and where each comes from. No data file is bundled: the dusk, the land and
the bats are worked out from the constants and rules in `src/`.

## The place (real)

- **Bracken Cave**, on the northern edge of San Antonio, Texas, in the Hill Country: the summer home of
  the world's largest bat colony, "more than 15 million Mexican free-tailed bats" from about March to
  October (Bat Conservation International, cited by the Texas Parks and Wildlife Department's
  brochure, https://tpwd.texas.gov/publications/pwdpubs/media/pwd_bk_w7000_1411.pdf; other outlets
  give up to 20 million). They come out at dusk to feed, for hours; the emergence shows on weather
  radar.
- **Hawks hunt the emergence.** Swainson's hawks taking Brazilian (Mexican) free-tailed bats as they
  leave their roost: C. H. Brighton, L. Zusi, K. McGowan, M. Kinniry, L. N. Kloepper & G. K. Taylor,
  "Aerial attack strategies of hawks hunting bats, and the adaptive benefits of swarming",
  *Behavioral Ecology* 32 (2021) 464–476 (studied at a cave in New Mexico, not Bracken). They found no
  stereotyped escape by single bats, and that lone bats were caught far more often than their share.

## The rules (drawn, after the real behaviour)

- A bat leaving the cave circles as it climbs, pulled toward the column's radius; at the column's top
  each takes a lane in one of two streams and follows a point a little ahead on it. Speeds (about 10
  m/s climbing round, 16 m/s in the streams) are of the right order for free-tailed bats; the column's
  height, the streams' paths and the pulses of the emergence are drawn.
- The ripple is drawn: a bat within sight of the hawk jinks aside and down, and a bat within 3.6 m of
  one that jinked copies it 0.07–0.16 s later, a little weaker, unless it jinked in the last 1.8 s (an
  excitable medium, the same family of rule as a stadium's wave). Bats also keep clear of the hawk.
  Real bats' escapes are not known to pass on like this (Brighton et al., above); it is the toy's
  picture of a startle spreading through a crowd.
- The hawk follows the finger on a spring, so a flick is a stoop; it folds its wings when fast and is
  drawn larger than life (a 1.3 m wingspan, shown at least 30 points across) so it can be seen.

## The look and the sound (drawn)

- The sky (a dusk gradient toward the west, the Earth's shadow behind, thin clouds, the first stars),
  the hills, the scrub, the sinkhole and the near junipers: drawn in `land.wgsl`, no data.
- The clicks are what a heterodyne bat detector by the cave would make of the bats' calls, which are
  too high to hear: short tones of 1.7–4 kHz, more of them the more bats are low in the column, a
  flurry when a ripple runs. Under them, crickets; the air past the hawk when it dives. All synthesised
  in `sound.rs`; no recordings.
