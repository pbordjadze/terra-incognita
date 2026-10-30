# Terra Incognita — exploratory test, The night crossing (Hydaspes)

## Summary
Interim report. The Hydaspes sequence answers touches (camp tap starts the march, column drag, date strip drag moves the camera to the landing). Found so far: the active-prototype chip truncates/wraps its text ("The ni…", "Op/en"), and a double tap at the river bend zoomed the camera *out* to the whole Earth instead of in (once so far).

## Coverage
- Session task: touch camp — done; drag column — done; drag strip — done; zoom at bend — partly (double tap; no pinch); play to end, Open, ✕ — pending.
- Goals 1–6: pending.

## App bugs (reproduced)
### Active-prototype chip text cut off and broken mid-word
- Steps: launch with `-prototype hydaspes`; look at the chip top left.
- Expected: title "The night crossing" readable (or cleanly truncated) and "Open" on one line.
- Observed: title shows "The / ni…" over two lines; "Open" is split "Op / en" over two lines. Seen in every screenshot of the session (2 of 2+, steady state).
- Evidence: shots/01-start-chip.png, 02-tap-camp-a.png, 04-strip-middle.png, 05-doubletap-bend.png.

## Suspicions
### Double tap at the river bend zooms out to the whole Earth
- Steps: hydaspes running; tap Alexander's camp; drag column; drag date strip to the middle (camera moves in close on the landing); double tap at the river (200,400 pt).
- Expected: zoom in at the bend (the sequence may pause, by design).
- Observed: camera jumped out to the whole-Earth view (Eurasia/Africa); composition hidden; strip stays at the middle. 1 of 1 so far.
- Evidence: shots/04-strip-middle.png (before), shots/05-doubletap-bend.png (after).

### Date strip always reads "326 BCE"
- The strip (`globeDate`, label "Date") keeps value "326 BCE" at start, ~10% and ~50%; VoiceOver users get no sense of progress through the night.

## Automation problems
- No pinch in Maestro: zoom at the bend tried with double tap only.

## Not tested
- (pending)
