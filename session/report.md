# Terra Incognita – exploratory test report (2026-10-28, iPhone 17 Pro simulator, release build)

## Summary
(In progress.) Camel and Llama works as a lens over the globe: you can start it from the workbench, touch the animal, follow, pause/resume, read the evidence, close it and end it. The playground's buttons come back and the Live line returns to the plain globe's counts. Two things stand out so far. On a fresh install a pack source is already set and gets checked over the network. The first Pause tap during a moving sequence didn't pause.

## Coverage
- Goal 1 (plain globe): not yet.
- Goal 2 (prototype on the globe): done for Camel and Llama.
- Goals 3–6: in progress.

## App bugs (reproduced)
### A pack source is preset on a fresh install and is checked over the network
- Steps: fresh install (as handed over) → Workbench → scroll to Packs.
- Expected (per the brief): no pack source set on a fresh install, so nothing goes online.
- Observed: the source `https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog…` is listed. "Checked at 5:52:41 AM ET" on the first opening and "Checked at 5:57:37 AM ET" on the second.
- Log: `09:52:41.093 … [playground] checked 1 source`.
- Reproduced: 2 of 2 workbench openings ran a check.
- Evidence: shots/workbench-bottom.png, shots/camel-ended-live.png.

## Suspicions
### First Pause tap in the camel sequence didn't pause
- Steps: Camel and Llama → tap the dromedary → Follow lineage (44 Ma) → tap Pause.
- Observed: the sequence kept playing (≈44 Ma → ≈7 Ma; counter 41 s → 48 s, still "Pause"). A second tap paused it (Resume, 65 s, held).
- Possible cause: the Pause/More row moves by 26 pt (y 714 → 740) when a scene adds the extra "Broad dispersal · many generations" line, so a tap during a scene change can miss. Seen once.
- Evidence: shots/camel-paused.png, shots/camel-paused2.png.

### Small hit targets (per hierarchy bounds)
- `camelidClose` "Return to atlas" is 13×13 pt and `camelidEvidence` is 17×17 pt. The doorway `doorway.sounding-the-deep` (HMS Challenger) is 16×16 pt and `doorway.sea-between` is 18×21 pt. All are below the 44 pt guideline, though the real hit area may be larger than these bounds.

## Automation problems
- None yet.

## Not tested
- (To be filled.)
