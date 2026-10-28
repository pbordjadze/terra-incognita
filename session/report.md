# Terra Incognita – exploratory test report (moving sea build)

## Summary
In progress. The moving sea is calm and readable. Thin blue-grey current lines with soft light running along them are clearest in the equatorial Pacific, the Caribbean, off Somalia and in the Agulhas. They stay off land and don't cover place names or prototype drawings. Two things look wrong: a pack source is already set and checked on a fresh install, and the chart in "The sea between" has stepped seams.

## Coverage
- Goal 1: partly done. Turned the clouds off, tapped a country (Bosnia and Herzegovina: caption and label agree), zoomed with a double tap, and tapped the open sea (caption went back to "The whole world").
- Goal 2: done. Started "A gathering of water", tapped the Amazon (caption "TOWARD THE AMAZON SYSTEM", basin lit), and ended it with ✕. Afterwards the Live line read "stages 0 · markers 0 · globes 1 · films 0 · scripts 0". I didn't read the line before starting, so I have no earlier value to compare with.
- Goal 3: partly done. "The sea between" (sounding works); Open shows a note panel.
- Goal 4: "The still sea" followed "A gathering of water"? No: I ended the first with ✕ before starting the second. See later updates.

## App bugs (reproduced)
### A pack source is set and checked over the network on a fresh install
- Steps: fresh install → open the Workbench → scroll to Packs.
- Expected (per the brief): no pack source set on a fresh install, so no network check.
- Observed: the source `https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog.json` is listed, and the status reads "Checked at 9:08:23 AM ET". The log shows `checked 1 source` at 13:06:18.603 and again at 13:08:23.212, once for each opening of the workbench.
- Reproduced: 2 of 2 openings.
- Evidence: shots/11-packs-source.png, shots/12-parked.png; app.log lines 117 and 131.

## Suspicions
### "The sea between": the chart's right edge and meridians step at three heights
- Steps: Workbench → "The sea between".
- Observed: the chart frame's right edge jumps outward in steps at about y = 580, 880 and 1180 (screenshot px of 2000). The vertical meridian lines inside the chart break and shift sideways at the same heights. It looks like strips drawn slightly out of line with one another. The meridians also show as light-blue bars in the sea that run past the chart's bottom frame.
- Seen in 3 screenshots of one session, while the globe stood still (shots/14-sea-between-start, 15-sea-between-again, 16-sea-between-sound). Not yet seen after a restart.
### Open in "The sea between" shows the "summer wind" note, and the foot caption doesn't match
- After sounding the water, Open brought up the "The summer wind" panel while the foot caption still read "THE SOUNDING GLASS". Seen once (shots/17-sea-between-open).

## Notes on the moving sea (first-time viewer)
- The lines are fine, quiet and readable. Light runs along them in the equatorial Pacific (shots/09-ended). The Somali Current and the Agulhas show clearly (shots/04-zoom1, 03-tap-land).
- In what I saw, no line crossed land or covered a name. "Gulf of Aden" and "Mozambique Channel" sit among lines but stay legible.
- The pale white patches on the sea are in "The still sea" too (shots/13-still-sea), so they belong to the base imagery, not the new effect.
- The cloud switch works: Clouds On/Off. With clouds on, the clouds sit over the sea and its lines (shots/01-start). After switching them off, the log's `clouds:` lines stop.
- The plain globe has no country names or borders at the whole-world view. The workbench says "The Wikipedia globe" is parked ("as the globe was until the ground"), so this looks intended.

## Automation problems
- None so far.

## Not tested
- The rings from tapping the sea: the screenshot comes about 5 s after the tap, and nothing was left by then (shots/05-sea-tap).
- Pinch and two-finger pan (Maestro has neither); smoothness while turning (only still frames).
