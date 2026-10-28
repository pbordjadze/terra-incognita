# Terra Incognita – exploratory test: "Sounding the deep" and "Borders on old maps"

## Summary
(in progress) Both prototypes start from the workbench, hide today's names/borders, and respond to the date strip. Sounding the deep worked as described (play/pause, zoom, stations, depth line, gap message, About page, Back, ✕). In Borders on old maps a choice survives a tap outside the maps' coverage while the foot stops naming it, and several labels collide or crowd out neighbours.

## Coverage
- Sounding the deep: started from the workbench, paused, zoomed in with a double tap, touched 2 stations, tapped the date strip into a gap, Open → About/sources → Back, ended with ✕, checked the Live line. Not done: touching the tiny ship from far away (see Not tested).
- Borders on old maps: started straight after Sounding the deep (opened on 1874, near Sounding's last date, 29 April 1874), tapped 5 of the 8 ticks (1874→1878 map, 1920, 1938, 1945/1946, 1994), chose France and Italy, tapped the sea, tapped land outside coverage.
- Further items (Borders' Open/Back, ✕, relaunch) are pending.

## App bugs (reproduced)

### 1. Borders: tapping land outside coverage drops the chosen country's name from the foot, but the country stays filled
- Steps: Borders on old maps on the 1945 map (date 1946). Tap France (or Italy): it fills, and the foot reads "FRANCE / 1946 / As drawn on the 1945 map…". Tap North Africa (outside the maps).
- Expected: either the choice is cleared (fill gone), or the foot keeps naming the chosen country and adds that the touched land isn't covered.
- Observed: the foot changes to "BORDERS ON OLD MAPS / 1946 / These maps cover Europe; nothing is drawn there on the 1945 map." while France (or Italy) stays filled. If you then move the date, the foot names the country again ("ITALY / 1994"), so the choice was never cleared. Only the foot dropped it.
- Reproduced: 2 of 2 (France, Italy).
- Evidence: shots/borders-france.png → shots/borders-africa.png; shots/borders-italy2.png → shots/borders-africa2.png → shots/borders-africa-then-1994.png.

### 2. Borders: the long "UNITED KINGDOM OF GREAT BRITAIN AND IRELAND" label runs over the North Sea onto Jutland, and Denmark goes unnamed
- Steps: Borders on old maps, date 1874 (1878 map) or 1920.
- Observed: the one-line label runs from Ireland, across Great Britain and the North Sea, and ends on Jutland. On those maps Denmark has no name. On 1938, 1945 and 1994 the label reads "UNITED KINGDOM" and Denmark is named.
- Expected: wrap or shorten the label (or keep it clear of other land) so that it doesn't crowd out Denmark.
- Reproduced: 2 of 2 maps (1878, 1920).
- Evidence: shots/borders-open.png (1874), shots/borders-tick718.png (1920); compare shots/borders-tick748.png (1938).

### 3. Borders: the foot promises a dashed line that often can't be seen
- Steps: choose a country whose outline didn't change between the two maps: Poland on 1994 (vs 1945), France on 1946/1945 map (vs 1938), Italy on 1946/1945 map (vs 1938).
- Observed: the foot says "The dashed line is <country> on the <year> map.", but no dashed line can be seen, because it lies under the solid outline. A first-time user is told to look for something that isn't there. (When the outlines differ, the dashed line does show: Poland 1938 vs 1920, Poland 1945 vs 1938, Italy 1994 near Trieste.)
- Expected: when the outlines match, say "unchanged since the <year> map" (or similar) instead.
- Reproduced: 3 of 3 cases.
- Evidence: shots/borders-lasttick.png, shots/borders-france.png, shots/borders-italy2.png.

## Suspicions
- **Borders: the 1938 and 1945 ticks sit about 8 pt apart.** Tapping the 1945 dot gave "1946" ("the 1945 map, the nearest to 1946"), not 1945. Hitting either map exactly by touch is fiddly. Seen once (shots/borders-tick760.png).
- **Borders: labels lie on the chosen country's fill.** "SWITZERLAND" and "ANDORRA" lie over France's filled area (shots/borders-france.png). "SWITZERLAND" lies over Italy's fill on 1994 (shots/borders-africa-then-1994.png).
- **Borders: no name in the large Russian/Soviet area.** It is unnamed on every map I saw, and the Netherlands is unnamed on the 1945 map. This may be the label placement putting Russia's name off-screen. Not investigated.
- **Borders 1920: the "GREECE" label sits in western Anatolia (the Smyrna zone).** Mainland Greece has no name (shots/borders-tick718.png). This is probably faithful to the data, but it is confusing.
- **Sounding: the depth line is drawn on top of the dashed passage.** At station 339 (23 March 1876) the passage runs straight south, so the blue hanging line and the grey dashed passage coincide and are hard to tell apart (shots/sounding-station2.png). At station 95 the label "3,777 m to the floor" crosses the dashed passage to the next station (shots/sounding-station1.png).
- **Sounding: start-up delay.** For a few seconds after choosing Sounding the deep, the chip already reads "Sounding the deep", while the globe still shows today's names and borders and the foot says "THE GLOBE / The whole world" (shots/sounding-open.png). Seen once.
- **Sounding: the globe shifts after Back.** After Open → Back, the zoomed globe was about 25 pt higher than before Open (compare shots/sounding-gap.png with shots/sounding-after-back.png). The chip also turned dark, with white text. Seen once.

## Automation problems
- None so far. Maestro's `scrollUntilVisible` overshot the workbench list once, and a manual swipe brought the row back.

## Not tested
- Touching the tiny ship from Earth scale: after the double tap I couldn't zoom back out, because Maestro has no pinch.
- Dragging the date strip continuously: only taps were used so far, and anything that shows only while a finger is down can't be seen.
- Playback smoothness: screenshots only show the result.
