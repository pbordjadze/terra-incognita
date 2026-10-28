# Terra Incognita — exploratory test report (iPhone 17 Pro simulator, release build)

## Summary
The doorway → prototype → ✕ cycle worked for Sounding the deep and A gathering of water. Switching straight from one to the other was clean, and the Live line was back at plain-globe counts afterwards. The biggest finding: this freshly installed app already had a pack source set (a raw.githubusercontent.com catalog), checked it over the network when the workbench opened, and downloaded and installed a new revision of Sounding the deep (r2). Whale and Desert was hard to use: its foot never changed from "The whole world", and the zoomed-in globe kept turning by itself and carried the fossil off-screen.

## Coverage
1. Plain globe: partly. I turned it with swipes, and one tap chose France (caption, red highlight and label agreed: shots/02). I didn't choose a small country, and I didn't double-tap on the plain globe (prototypes zoomed it for me).
2. Workbench + on-globe prototype + ✕: done for Sounding the deep (from its ship doorway) and A gathering of water (from the workbench). After ✕ the plain globe, its caption and the clouds came back. With nothing running, Live read `stages 0 · markers 0 · globes 1 · films 0 · scripts 0`. I didn't capture Live while a prototype was running.
3. Own screen: partly. Sounding the deep's Open shows a notes sheet ("Sounding the deep", half screen, Close works). I didn't try a web pack's Back, because the bundled packs now run on the globe.
4. Two in a row: done. A gathering of water, then tapping the Challenger ship doorway, replaced it with Sounding the deep: the rivers, highlights and caption all went (shots/10, then the screenshot after it).
5. Quit/relaunch: not reached yet (see Not tested).
6. VoiceOver labels: looked at in `inspect_screen` (notes below).

## App bugs (reproduced)
None confirmed twice yet.

## Suspicions
- **A pack source is set on a fresh install, and the app downloaded a pack revision.** Steps: freshly installed app → open the workbench. Expected (per the brief): no pack source, no network use. Observed: Packs lists the source `https://raw.githubusercontent.com/pbordjadze/terra-incognita/packs/catalog.json`, with status "Checked at 3:45:04 AM ET" and `sounding-the-deep` "opens r2 · downloaded r2" (Previous / Remove downloads). The log has `07:41:55 checked 1 source`, then `07:43:30.809 sounding-the-deep r2 installed` and `07:43:31.472 sounding-the-deep r2: started (4 scripts)`; the first run at `07:37:58` was `r0`. The new revision was swapped in while I was switching prototypes. Evidence: shots/12-workbench-lower, shots/12b-workbench-packs, app.log lines 91 and 102–103. Seen once (one workbench opening).
- **Sounding the deep didn't start at its beginning the second time.** First start (07:37:58, r0): date 18 February 1873 (shots/03 and the screenshot after it). Second start (07:43:31, r2, from its doorway while A gathering of water ran): 20 June 1875 (`inspect_screen`), 24 June 1875 a moment later (shots/10). It may resume where it was left, keep running after ✕, or the r2 revision may start elsewhere. Seen once.
- **The HMS Challenger doorway stays on the globe while A gathering of water runs, though all the other doorways hide** (shots/08-water, shots/09-water-amazon: the ship at the top right over the Atlantic). Tapping it does switch cleanly. It is a 16×16 pt target (`doorway.sounding-the-deep` bounds), small for a finger.
- **Whale and Desert: the foot gives no guidance, and the zoomed globe drifts away from Egypt.** Started from the workbench (07:46): the globe zoomed onto the Mediterranean and North Africa, with the fossil, camel, sailboat and ghost-border doorways visible (screenshot after shots/13). The foot still read "THE GLOBE / The whole world / Spin it, pinch it, tap a country." The globe kept turning west at this zoom: within about 10 s Egypt was off-screen (West Africa), and over about 2 min the view passed North America, Hawaii, Japan and China (shots/15a, 15b, 16a, 16b). A `tapOn id: whaleFossil` after a swipe failed because the fossil was no longer on screen. The slow turn is by design, but at this zoom it carries the prototype's own subject away before it can be tapped. Seen once (one run).
- **Accessibility frames for doorways on the far side of the globe.** On the first screen (globe showing East Asia and the Pacific), `doorway.gathering-of-water` was listed with on-screen bounds [234,320][266,352] over the open Pacific, though South America was out of sight. Hidden doorways may be reachable by VoiceOver at places where nothing is drawn. Seen once; I didn't check it further.
- Minor: when a river is chosen, its highlight line and dot are drawn over the "Amazon" label (shots/09-water-amazon). The scroll-bar element is listed twice in the workbench and the notes sheet (duplicate "Vertical scroll bar" elements).

## Automation problems
- Screenshots taken in the same flow right after a `tapOn` sometimes show the screen from before the tap (shots/03-sounding still shows the plain globe with France, while `inspect_screen` already listed the running prototype). I trusted `inspect_screen` and a later screenshot.
- The globe's own turning makes element bounds go stale between `inspect_screen` and the next tap.

## Not tested
- Quit and relaunch (with and without a prototype running), and the clouds switch, if this report isn't updated later.
- The Whale sequence itself, Camel and Llama, A small thing, The sea between chart, Borders on old maps date strip, the parked directions.
- Pinch and two-finger pan (Maestro can't do them).
