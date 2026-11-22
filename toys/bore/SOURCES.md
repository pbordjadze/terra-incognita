# The Silver Dragon: sources

What is real and what is drawn. The toy loads one file of its own, `data/tides.bin`: for each of the four
settings of the switches and fifteen ages of the moon, the river a little before the tide reaches Yanguan,
built by `cargo run --release -p bore --example library` (about a minute). `--example tune` prints the
bore's height and Froude number for every moon and setting; `--example profile -- 1 1 0.57` prints the
level along the estuary through one tide.

## Real

- **The place and the wave.** The Qiantang River's tidal bore, watched each autumn from the seawall at
  Yanguan, Haining, Zhejiang, where the tide from Hangzhou Bay comes up the river as one breaking front
  and crowds stand behind the rail. Bores there are commonly 1–3 m high and run at several metres a
  second; the largest come just after the full and new moons (springs), and the famous one about the
  18th of the eighth lunar month.
- **Why it forms.** Hangzhou Bay narrows from about 100 km wide at its mouth to about 20 km at Ganpu and 3 km at Yanguan, and
  the bed rises into broad sand bars, so the tide is piled up (a funnel) while, over the shallows, the
  deep part of the wave runs faster than the shallow part ahead of it and the front steepens into a
  shock. Pan, C., Lin, B. & Mao, X. (2007). Case study: numerical modeling of the tidal bore on the
  Qiantang River, China. *Journal of Hydraulic Engineering* 133(2), 130–138. Chanson, H. (2011).
  *Tidal Bores, Aegir, Eagre, Mascaret, Pororoca: Theory and Observations.* World Scientific: a bore
  breaks once its Froude number is above about 1.3–1.5 and is undular (a train of whelps) below.
- **The model.** `tide.rs` solves the one-dimensional shallow-water (Saint-Venant) equations for a river
  of varying breadth from Ganpu to past Yanguan (50 m cells), second order (MUSCL, Heun), with a
  hydrostatic reconstruction for the sloping bed (Audusse, E. et al. (2004), *SIAM J. Sci. Comput.*
  25(6), 2050–2065), Manning friction, and the mouth driven by a semi-diurnal tide whose range follows
  the moon's age. The bore is what the equations make; its height and Froude number are read from them.
- **The switches.** *The bay narrows*: off, the river is one breadth all the way and the bay's gain is
  gone (Green's law). *Crests outrun troughs*: off, the equations are linearised, every part of the wave
  runs at one speed and the tide only swells.

## Drawn

The funnel's gain is a factor on the mouth's range rather than the whole bay; the bed and breadth are
smooth curves, not surveyed. The pace (hours of tide in seconds, slowing as the bore comes near), the
camera's walk down to the steps, the crowd, the spray thrown up the wall, the far shore and the moon's
place in the sky are drawn. The sound is filtered noise: a roar that rises as the bore nears, the hiss of
the flood, the crowd.

## The look: ink-wash (after the real films; the details drawn)

- **Ink-wash animation** (水墨动画): *Little Tadpoles Looking for Mama* (小蝌蚪找妈妈), Shanghai Animation
  Film Studio, 1960, under Te Wei, in a style after Qi Baishi
  (https://en.wikipedia.org/wiki/Tadpoles_Searching_for_Mother); later *The Cowboy's Flute* (1963) and
  *Feeling from Mountain and Water* (1988) (https://en.wikipedia.org/wiki/Ink_wash_animation).
- **Here** (`src/river.wgsl`): the river ray-marched as a heightfield and painted, not lit: the calm water
  as dry horizontal strokes on bare paper, the bore's face a dark wash under a white roller left as
  paper, churned flood behind it, the fish-scale stones of the revetment in a few faint arcs, a firm
  stroke at the wall's edge, warm mottled paper, a vermilion seal of cut strokes (no characters). The
  crowd and spray are brush marks over it. A painter's reading of the finished films, not their method.
