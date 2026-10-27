# One ship, two rhythms

Runtime-1 offline screen pack. Start **One ship, two rhythms** in the workbench.
The host injects terra; this pack loads only its own assets and does not record reader state.

## Reading the map

HMS Challenger's 1872–1876 expedition is represented by 23 selected sampling stations dated
18 February 1873 to 9 April 1876. This is not a complete ship log. The opening is the dense
February–March 1873 Atlantic interval.

Dots identify included observations. On an observed date, the ship symbol is held at the
reported station location as a display convention, not an assertion it stayed there all day. No exact time of sampling is asserted. Between short station pairs,
movement is an **illustrated great-circle transit**, not a logged course or measured speed.
The status line and dashed connections distinguish it from recorded positions.

Only gaps of at most 14 days can receive this illustrative connection, and a connection is
suppressed if sampled intermediate points cross the modern physical land reference. This
coarse test does not establish navigability. Longer gaps have no ship position. No connector
is drawn across missing months or years. Reported depth is displayed only at a recorded station;
it is not a depth profile along a hypothetical track.

Selecting the ship frames thirty days; selecting the British Empire frames a century. Both
use the **same numeric UTC day**. Selection preserves the date and geographic camera. A
time span plays across the screen in 24 seconds; wider spans therefore move faster. Manual
pinching overrides either proposed span. Return to the voyage changes the date explicitly.
Follow ship is an explicit, reversible camera choice.

At broad scales, brief rings mark included station dates crossed by this same clock. They
are visual traces of observations, not extra ships, repeated voyages or live vessel locations.
Automatic playback and transient traces are disabled by Reduce Motion.

The ship symbol is original schematic artwork, not a reconstruction of Challenger's rigging.
Modern Natural Earth physical land is an orientation reference. Political/imperial borders
are not reconstructed: Empire is a time-scale frame for the British expedition.

## Observation sources and licenses

- Murray, John; Renard, A F (1891). Annotated record of the detailed examination of Mn deposits
  from the HMS Challenger stations. PANGAEA, https://doi.pangaea.de/10.1594/PANGAEA.849072.
  CC BY 3.0: https://creativecommons.org/licenses/by/3.0/. Stations CHA-3, CHA-7, CHA-8.
- Murray, John; Renard, A F (1891). Chemical composition of manganese nodules and other manganese
  rich deposits discovered during the HMS Challenger expedition (table, page 370). PANGAEA,
  https://doi.pangaea.de/10.1594/PANGAEA.849018. CC BY 3.0. Stations CHA-16 and CHA-160.
- Zarkogiannis SD, Wood TJ, Miller CG, Stukins S and Clark B (2025). Reassessing the HMS Challenger
  collection as a late 19th century surface ocean indicator using X-ray micro-computed tomography,
  Table 1. https://doi.org/10.3389/fmars.2025.1454260.
  CC BY 4.0: https://creativecommons.org/licenses/by/4.0/. Eighteen distinct station/date records.

Modification: selected station labels, source dates, reported decimal coordinates and sampling
depths were transcribed; duplicate sample rows for one station/date were collapsed. PANGAEA
T00:00:00 timestamps are reduced to dates. Historical measurements are not modern GPS precision.
Source links and attribution are also bundled in data.js and available in the Sources panel.

Physical land: Natural Earth ne_110m_land.geojson, public domain; pinned original:
https://github.com/nvkelso/natural-earth-vector/blob/ca96624a56bd078437bca8184e78163e5039ad19/geojson/ne_110m_land.geojson

## Authoring and verification

Preview with node tools/playground/serve.cjs, then /preview/one-ship-two-rhythms?device=ipad.
The existing time-atlas prototype and shared native code remain unchanged.

- node --test tools/playground/ship/core.test.cjs
- node tools/playground/ship/check.cjs
- node tools/playground/ship/fetch-stations.cjs --self-test

The optional fetch-stations tool downloads licensed PANGAEA event metadata at authoring time
to a build artifact; it never silently changes the curated pack. A fuller export needs
source/precision review before inclusion. The runtime never fetches it.

Browser checks cover real painted ship motion, same-date scale switches, reversibility,
evidence gaps, map/time camera independence, touch/keyboard alternatives, host settings,
maximum text size and disposal. Browser WebKit and synthetic pointer checks do not replace
native custom-scheme or physical-device validation.
