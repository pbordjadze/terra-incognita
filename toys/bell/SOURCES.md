# Bell: sources

What is measured, what is drawn, and from where.

## Measured

- **The Earth's free oscillations**: the fundamental spheroidal modes 0S0 and 0S2–0S12 of PREM, the
  Preliminary Reference Earth Model (A. M. Dziewonski and D. L. Anderson, "Preliminary reference Earth
  model", Phys. Earth Planet. Inter. 25, 1981), their frequencies (0S2 0.3093 mHz, a period of 54
  minutes, up to 0S12 at 1.99 mHz; 0S0, the "breathing" mode, 0.8147 mHz, 20.5 minutes) and quality
  factors (0S0 about 5300, the rest 300–510), as tabulated in the normal-mode literature (e.g. the
  observed 0S2 singlet at 0.30926 mHz, S. Rosat et al. 2008). `src/modes.rs` holds the table.
- **Their splitting by the Earth's turning**: 0S2's five singlets, computed for PREM, 0.3000–0.3185
  mHz, about 4.6 µHz apart; the Earth's ringing after the 1960 Chile earthquake was the first in which
  that splitting was seen.
- **How a point strike excites them**: each degree l as the Legendre polynomial P_l about the strike
  (the addition theorem over its 2l+1 singlets), heard at a place through the Schmidt-normalised
  associated Legendre functions, singlet by singlet; so the chord at a place depends on how far it is
  from the strike, and each degree is silent on its own nodal circles (`modes::tests` checks the
  theorem).
- **The surface wave**: Rayleigh waves at about 3.9 km/s, out round the world to the antipode and back
  (R1, R2, …), focusing at the strike's antipode.
- **The great earthquakes** the idle Earth rings from: Valdivia 1960 (M9.5), Alaska 1964, Sumatra–Andaman
  2004, Tōhoku 2011, Kamchatka 1952 and Maule 2010, at their USGS epicentres.
- **The ground**: the kit's globe, Terra's data (NASA Blue Marble, Natural Earth, GEBCO), credited in
  Terra's sources.

## Drawn (not measured)

- **Time and pitch**: one factor (about 475,000) takes every mode into hearing (0S2 to D3, 146.8 Hz), so
  the chord's ratios are the Earth's; every ring lasts 5 times what the quality factors give at that
  pitch (their ratios kept: the breathing mode outlasts the rest, the high degrees fade first). The
  picture runs 300 times slower than the sound, so the swells can be watched.
- **Splitting of degrees 3–6**: 0S2's spacing scaled as 1/(l(l+1)), the Coriolis form; degrees above 6
  are not split (they fade before a beat could be heard).
- **The strike's spectrum** (how strongly each degree rings: falling with l like a soft mallet's), the
  knock, the motion's height (exaggerated about a million times: a great quake moves the ground by
  millimetres), its light and colour.
