# Terra Incognita – exploratory QA report (2026-10-30)

## Summary
The new thing in this build is a **"What's new" guided tour** (pill "12 new things · Show me" at the top of the plain globe), which zooms the globe in, shows a bar "1 of 12 …" with a question, and a reaction tray (Say / Wow / Nice / Meh / Picture / Skip / Didn't work). Opening a doorway from the tour (the camel) puts the camel screen's own header and bottom bar on top of the tour's bar and reaction tray, so both are partly hidden.

## Coverage
- Goal 1 (plain globe): not yet.
- Goal 2–5: not yet.
- New feature (What's new): in progress.

## App bugs (reproduced)
(none yet)

## Suspicions
### First tap on the "Show me" pill did nothing
- Steps: fresh launch, plain globe. `tapOn id: whatsNewPill`.
- Observed: no change (hierarchy and screenshot identical). A second tap, `tapOn text: "Show me"`, opened the tour. Seen 1 of 1. Evidence: shots/whatsnew1.png (after 1st tap), shots/whatsnew2.png (after 2nd).

### Camel screen overlaps the What's new bar and reaction tray
- Steps: open What's new (piece 1 of 12 "The living globe"), tap the camel doorway (`doorway.camelid`).
- Observed: camel screen's header (✕, "Today", "Arabia", "Dromedary camel", ⓘ) is drawn over the tour bar, which shows ghosted beneath it (its new "Open" and ✕ still in the hierarchy at the same place); the "Follow lineage" bar covers the bottom half of Wow/Nice/Meh labels and all of Skip / Didn't work. Evidence: shots/wn1_camel.png. 1 of 1 so far.

## Automation problems
(none yet)

## Not tested
Voice note (mic) and Picture reactions – avoided (microphone / no recording wanted).
