# Huddle: sources

What is real, what is drawn, and where each comes from. No data file is bundled: the ice and the
penguins are worked out from the constants and rules in `src/`.

## The place and the birds (real)

- **Atka Bay**, on the sea ice near Germany's Neumayer Station (70°39′ S, 8°15′ W), where a colony of
  about 2,000 emperor penguins breeds through the winter. On 3 August 2008 its huddles were filmed
  from 12 m up and 115 m away for four hours, at −33 to −43 °C with the wind at most 8.3 m/s:
  D. P. Zitterbart, B. Wienecke, J. P. Butler & B. Fabry, "Coordinated Movements Prevent Jamming in
  an Emperor Penguin Huddle", *PLoS ONE* 6 (2011) e20260, https://pmc.ncbi.nlm.nih.gov/articles/PMC3106014.
  From it: the density, about 21 penguins a square metre; the jammed huddle broken "every 30–60 s by
  small 5–10 cm steps" that "travel as a directed wave with a speed of ∼12 cm/s through the entire
  huddle"; the steps moving the whole huddle forward; and penguins that "tend to join a huddle at its
  rear (trailing) end and leave it at the front (leading) end". The paper leaves open who starts a
  wave.

## The rules (drawn, after the real behaviour)

- A penguin is in the wind if no one stands upwind of it (the huddle cut in strips a penguin wide
  across the wind; the first met from upwind takes all of it, the next two a little), or partly if
  it stands on the edge with few neighbours. In the wind it gets cold; sheltered it warms. A cold
  penguin on the windward edge walks round the outside (the nearer way) and presses in again on the
  lee side; this turnover is the pattern of the huddling models (e.g. R. Gerum et al., "The origin of
  traveling waves in an emperor penguin huddle", *New Journal of Physics* 15 (2013) 125022, and
  A. Waters, F. Blanchette & A. D. Kim, "Modeling Huddling Penguins", *PLoS ONE* 7 (2012) e50277,
  where the huddle drifts downwind; not fetched for this toy). The rates of cooling and warming, the
  walk's speed and the lean toward the middle are drawn.
- A wave: a penguin takes a 7.5 cm step forward; each neighbour steps after the time a 12 cm/s wave
  takes to reach it, unless it stepped in the last nine seconds. One starts on its own every 30–60 s
  at the huddle's upwind back, and a touch starts one. They face downwind (their backs to it), which
  the paper says is not the rule in light winds; drawn.
- Everything runs four times the real pace.

## The look and the sound (drawn)

- The ice (wind-carved sastrugi, drift streaming with the wind, the trodden snow under the huddle),
  the low sun's rose light and blue shade with a little of the aurora's green, the long shadows, the
  frost on the backs of those in the wind, the snow blowing: drawn in `ice.wgsl` and `lib.rs`.
- The wind and the calls are synthesised in `sound.rs`: an emperor's call as runs of buzzing
  syllables in two voices a little apart (emperors call with both sides of the syrinx at once, the two
  voices beating); no recordings.
