# Terra Incognita – exploratory test report (in progress)

## Summary
Plain globe works: taps choose the right country (Kiribati, United States, Palau) and the caption matches. Starting Camel and Llama keeps the previous country's caption ("Palau") while the globe shows Arabia, and its chip offers "Open" although it is listed as "on the globe".

## Coverage
1. Plain globe: done (auto-turn, double-tap zoom, 3 countries incl. small ones).
2. Workbench + on-globe prototype: in progress.
3–6: pending.

## App bugs (reproduced)
(none yet confirmed twice)

## Suspicions
- **Caption keeps stale country when a prototype starts.** Steps: choose Palau, open Workbench, choose Camel and Llama. Expected: caption describes the prototype or the plain globe. Observed: caption "MICRONESIA / Palau / Capital Melekeok · 18 thousand people" while the globe flies to Arabia; Palau is off-screen and no longer highlighted. 1 of 1. Evidence: shots/palau.png, shots/camel1.png.
- **"Open" shown on the chip of an on-the-globe prototype.** Camel and Llama is listed "Native · on the globe", but the chip has `openActivePrototype` ("Open"). 1 of 1. Evidence: shots/camel1.png, inspect_screen.
- **Double space in the caption:** "Capital Washington,  D.C. · 328 million people" (two spaces after the comma, in the accessibility text too). Seen on 2 reads of the same state. Evidence: shots/us-tap.png.
- **Russia unlabelled when zoomed** over East Asia (one double tap): Mongolia, China, North Korea and Japan are labelled, but Russia, which fills the top half of the screen, is not. 1 of 1. Evidence: shots/zoom1.png.
- **A faded, half-tone name** overlapping SPAIN/MALTA near the globe's west Mediterranean on the very first screen (a ghosted "...Sea" label), against "names appear fully or not at all". Seen once, first screenshot only (not saved to shots/).
- The auto-turn was fast: about 70° of longitude in the ~15 s between my first screenshot and the start of the next flow (shots/start.png vs. the first screenshot), so a tap aimed at Brazil landed in the Pacific and chose Kiribati (correct for where it landed).

## Automation problems
- The globe moves between a screenshot and the tap; take the screenshot in the same flow as the tap.
- The workbench is a popover: the list needs swipes to scroll.

## Not tested
Pinch, two-finger pan (no Maestro support).
