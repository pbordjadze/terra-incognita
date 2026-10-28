# Terra Incognita – exploratory test: "Sounding the deep" and "Borders on old maps"

iPhone 17 Pro simulator (402×874 pt), release build, 2026-10-28. Screenshots are in `shots/`.

## Summary
Both prototypes work end to end. The date strip responds, playback and pause work, stations, depth line, gap message, country choice, sea-clears and out-of-coverage messages all behave as described, and ✕ returns the plain globe with the Live line back to baseline. Reproduced problems:
- Sounding the deep ignores the other prototype's date when that date is after its record. Borders at 1997/1999 opens Sounding at the start, Feb 1873, instead of near May 1876.
- Borders on old maps always opens with Poland chosen. On the inherited 1870s dates that gives a first sentence that says Poland isn't there.
- Major country names (Germany, Poland) go missing depending on which country is chosen.
- A strip of the globe shows below the foot.

There are also several legibility and clarity issues.

## Coverage
- **Sounding the deep**: done. Start, autoplay, pause, double-tap zoom, stations (filled/hollow/dashed), touching a station, touching the date strip, gap in the record, Open (About + sources), Back, ✕, Live line. I did not reach "touch the tiny ship from afar" (needs zooming out, which needs a pinch).
- **Borders on old maps**: done. Start, eight ticks, nearest-map captions, choosing Poland and Germany, dashed earlier outline, sea clears, out-of-coverage land (Africa), first tick 1530, Open (About), Back.
- **One after the other**: done in both directions, three switches each way (see bug 1).
- **Quit/relaunch**: see the end of this report if reached.
- **VoiceOver**: read labels in `inspect_screen` only. I did not use VoiceOver's adjust action.

## App bugs (reproduced)

### 1. Sounding the deep starts at the beginning of its record when the previous prototype's date is later than the record
- Steps: start Borders on old maps. Touch the right end of the date strip (about 373–377 pt, y 804) so the date is 1993–1999. Open the workbench and choose Sounding the deep.
- Expected: Sounding opens near the first one's date. 1997 is past the record (2 Feb 1873 – May 1876), so the nearest date is the end, May 1876.
- Observed: it opens on **22 February 1873**, the start of the record. Within range the handoff works: Borders 1874 → Sounding 17 June 1874; Sounding 19 Sep 1874 → Borders 1875; Sounding Feb 1873 → Borders 1873.
- Reproduced: 2 of 2 (1997 → 22 Feb 1873, 1999 → 22 Feb 1873).
- Evidence: `shots/switch-to-sounding.png`, `shots/borders-late-2.png` → `shots/switch-to-sounding-late-2.png`. In-range counter-example: `shots/borders-1875ish.png` → `shots/switch-to-sounding-1874.png`.

### 2. Borders on old maps opens on a sentence that says its chosen country doesn't exist
- Steps: start Borders after Sounding, so the date is 1873–1875.
- Observed: the foot reads "POLAND / 1875 / This year shows the 1878 map, the nearest to 1875, which has no Poland. Drag the date to another map." The chosen country is invisible, so a newcomer's first words are about something that isn't on the globe. Nothing is filled, and there is no hint what the prototype is about. The intended opening (1945, Poland filled, 1938 dashed) is never seen when coming from Sounding.
- Reproduced: 2 of 2 (1875 and 1873). Evidence: `shots/borders-open.png`, `shots/switch-to-borders.png`.
- Suggestion: when the inherited date has no Poland, start with nothing chosen and use the general caption.

### 3. Major country names are missing on the 1994 map, depending on the choice
- With **Poland chosen** (1993/1999, 1994 map), GERMANY has no label, although BELGIUM, LITHUANIA, ESTONIA and MALTA do. Reproduced 2 of 2 (`shots/borders-last-tick.png`, a second screenshot a moment later, `shots/borders-late-2.png`).
- With **nothing chosen** (sea tapped), POLAND has no label although there is clear room between GERMANY and BYELARUS. LITHUANIA, ESTONIA and BELGIUM disappear too, and LATVIA appears. Reproduced 2 of 2 (`shots/borders-sea.png`, `shots/borders-last-tick-2.png`).
- With **Germany chosen**, GERMANY appears and POLAND disappears (`shots/borders-germany.png`).
- Expected: a large, important country (Germany, Poland) keeps its name before smaller ones (Belgium, Malta). The set of names should not depend on which country is chosen, apart from the chosen one.
- Russia is never labelled on any map I saw, perhaps because it is outside coverage.

### 4. The globe shows through below the foot in both prototypes
- While either prototype runs, the foot's paper background stops about 35 pt above the bottom edge. Below the date strip a band of the globe (sea, coast or land) is visible. It is not there on the plain globe.
- Reproduced in every screenshot of both prototypes (e.g. `shots/borders-open.png`, `shots/borders-late-2.png`, `shots/sounding-after-wait.png`).

### 5. Touching the last tick of the Borders date strip gives 1993, not 1994
- Steps: touch the eighth tick (373 pt, y 804).
- Observed: 1993, "There is no map for 1993 here: this is the 1994 map". Touching the first tick gives exactly 1530. The strip moves about 1.3 years per point and does not snap to ticks, so touching a map's own tick often gives a neighbouring year. The strip also runs past the last map (to at least 1999).
- Reproduced 2 of 2. Evidence: `shots/borders-last-tick.png`, `shots/borders-last-tick-2.png`, `shots/borders-first-tick.png`.

## Suspicions (seen once, or unsure)
1. **Blank About page on the first Open.** In Sounding the deep, the first tap on Open showed a page with only the Back button, still blank after more than 10 s and after a scroll. The log shows `04:49:20.223 sounding-the-deep r0: opening index.html`. Back and Open again showed the page normally. 1 of 2. Evidence: `shots/sounding-about.png`, `shots/sounding-about-wait.png`, `shots/sounding-about-scrolled.png`; second try `shots/sounding-about-2.png`.
2. **Slow first start.** The first start of Sounding showed its chip and turned to the Atlantic for several seconds while today's borders, names, "North Atlantic Ocean" and the plain foot ("THE GLOBE / The whole world") were still drawn. Then the foot, date and ship appeared. Later starts were quick. 1 of 1 first start. Evidence: `shots/sounding-open.png`, then `shots/sounding-after-wait.png`. Log `04:47:21.535 opening overlay.html`.
3. **Depth label hard to read and drawn over land.** The hanging line goes straight down the screen (26 pt per km, so 4,926 m is about 128 pt) from a station off the Caribbean, across Venezuela and Brazil. "4,926 m to the floor" is pale blue-grey on beige land. `shots/sounding-station1.png`.
4. **The ship's label covers a station.** "HMS Challenger" is drawn over a hollow station and the dashed passage next to the ship. `shots/sounding-station1.png`.
5. **Stations show through the foot.** When zoomed in, station dots and rings are visible behind "HMS CHALLENGER" and the date line. The foot fades over the globe rather than covering it. `shots/sounding-paused.png`, `shots/sounding-date-mid.png`.
6. **Invisible dashed outline.** With Poland chosen on the 1994 map, the caption says "The dashed line is Poland on the 1945 map". The two outlines nearly coincide, so no dashed line can be seen, and the caption points at something the reader can't find. `shots/borders-last-tick.png`.
7. **Playback at Earth scale is fast.** 2 May → 17 Aug 1873 in about 15 s, so the ship jumps across the ocean while you try to double tap it. The opening invites "Zoom in on the little ship", which is hard to do while it plays.
8. **About page, Borders: contradictory wording.** "a year with no map shows the nearest one … Nothing is drawn for the years in between." `shots/borders-about.png`.
9. **About page, Sounding: Back covers text.** When scrolled, the floating Back button covers the start of "What this record is" (the number of stations and the end date). `shots/sounding-about-sources.png`.
10. The first fresh start of Sounding opened on **2 May 1873**, not the record's start (2 Feb 1873) and not tied to anything I set. The plain globe has no date, so I can't tell where it came from.
11. "BYELARUS" on the 1994 map. This may be the source's spelling, but it looks like a typo to a reader.

## Works as described
- Sounding: autoplay; Pause/Play (`globeDatePlay`, label switches Pause/Play); stations filled when passed and hollow ahead, with dashed passages; touching a station sets the date (station 19, 11 March 1873, 4,926 m) and the foot explains it; the gap message ("This record has no stations between 11 September 1874 and 3 October 1874, so the ship is not shown"); today's borders and names are hidden.
- Borders: eight ticks; 1530 ("The maps this early show only the Ottoman Empire"); choosing Germany fills it with the 1938 outline dashed; the sea clears the choice; Africa says "These maps cover Europe; nothing is drawn there on the 1994 map."
- Open/Back on both. In Sounding, Back returns with date and zoom kept (the globe sat about 25 pt higher than before Open).
- ✕ on Sounding returned the plain globe with borders, country and sea names. Live afterwards read `stages 0 · markers 0 · globes 1 · films 0 · web views 0`, the same as at the start (`shots/workbench-live-initial.png`, `shots/live-after-sounding.png`). The globe stayed zoomed in after ✕.
- VoiceOver labels: `globeFootCaption` reads the whole foot; `globeDate` is "Date" with the value (e.g. "1874", "17 June 1874"); the chip reads "Active prototype: …", "Open", "Back to the plain globe"; `packBack` is "Back to the globe".

## Automation problems
- My first `tapOn id: prototype_sounding-the-deep` came after a scroll that left the row under the workbench's header. It closed the workbench and started nothing. Centring the row first (`centerElement: true`) fixed it.
- I can't pinch, so I couldn't zoom out after double-tap zooming. The zoom also carried over to the plain globe and into Borders.

## Not tested
- Touching the tiny ship from afar (flying to it), because I couldn't zoom back out to Earth scale.
- Dragging the date continuously, and how smooth playback is frame to frame. Screenshots only show the end of a gesture; I used touches on the strip.
- The VoiceOver adjust (increment/decrement) action on `globeDate`.
- iPad layout.
