# Adrift: where it stands (handoff, Nov 3 2026)

The conductor's handoff at a stable stopping point. The rulings are in `DECISIONS.md` (§ 28–32 for
rounds 21–25), the clocks in `ROUNDS.md`. painting-company row 199.

## The build

- **adrift-build-92** (main 7eaac99) is the latest in the SideStore source; the web link is the
  claude.ai artifact (version 30; `?wind` opens with seeds). The main merge after this file adds
  only tools (`adrift-check play-all`, the kit's web tooling), not the app's look or behaviour.
- **What the owner taps:** the blue lamp's mark top left, then purple's; rest on the record control
  (top right), "A clip of these 2 lamps' journey"; in the panel drag the two handles to the days,
  pick a speed, ▶︎ to preview, Export (60 fps HEVC master; a long export shows the time left and a
  cancel). "Paste lamps" in the same menu puts back copied lamps. "Seeds on the wind" switches what a
  put makes.

## What landed in rounds 21–25 (Nov 2–3)

- R21: seeds on the wind (a second medium behind one trait; conducting, clouds and journeys for seeds);
  a journey at the app's own pace past today; clips HEVC.
- R22: a 60 fps master for TikTok and an export look that survives its re-encode (`tools/adrift/tiktok.sh`);
  the journey camera keeps the dance (the pair's median share of the width 2% → 30%).
- R23: the edit view (`EditPanel` in `ios/Sources/main.swift`; core `App::preview_*`,
  `journey_take_edit`); one ring style for chosen lamps.
- R24: export pipelined (`gpu::Readback`), the GPU's work per export frame −40%; **the teleported
  lamp fixed** (a snapshot written mid-export kept the clip's world; `App::snapshot` now always writes
  the live state; test `touches_during_an_export_change_no_lamp`).
- R25: lamps with character: four glasses by id (roped float, tall diamond, lantern, onion dome), a
  living flame, light on the water, growing as the camera closes in.

## What the critics last said

- Lamps (on build 80, before the follow-up in build 92): by night "glass lanterns … a little
  festival", show without a caveat; by day and in the clip still read as dots, with a ring like chrome.
  Build 92 answered that (no badge or ring in a clip, 1.75x close in, silhouettes apart). Not yet
  judged by a critic.
- The journey clip (R22): crisp after a TikTok-like re-encode; the camera's last third zooms deep
  over empty sea and centres off the pair.

## Open known issues

- **The edit view has never been seen:** UIKit can't render off a Mac. It compiles (CI); the owner's
  phone is its first look.
- **Export speed on a real GPU is unmeasured:** compare `adrift-check journey crates/adrift-check/lamps/dance.txt 60 out.mp4`
  on main ca76b77 (before R24) and today's main on the homelab GPU (`ADRIFT_PROFILE=1` prints the
  passes' shares); the phone logs `adrift: export of N frames took …`.
- By day the blue diamond's edge is a dark teal line; the lamps' light on the water is faint by day.
- The journey camera: one controller instead of a spring stacked on the R19/R20 limiter, centring on
  the pair's predicted midpoint, and a lower zoom cap at sea (Motion's R22 finding).
- Chosen states: the kept star still sits beside its lamp; the panel's soft ground at the top left.
- Seeds read poorly at a whole-globe zoom; the wind itself isn't drawn.
- Other conductors' merges into main restart Adrift's CI (adrift.yml fires on `ci/sidestore_apps.json`):
  a long Mac queue delayed build 92 by ~1.5 h.
- The owner's lamps lost to the teleport (before build 73) can only come back by "Paste lamps" from a
  copy.

## What I'd do next

1. A critic on build 92's lamps (phone size and the clip after tiktok.sh), and the owner's look at the
   edit view.
2. The journey camera as one controller (the critic's last-third note).
3. The ground and sea merged into one pass (the export's next ~30%).
4. Then the plan's later items: the dials that differ between sea and wind; a flat plane; the fate as a
   timing tool.

## Spend since round 9

About $435 of the $600 cap (estimated: the sessions report no cost): rounds 9–20 ≈ $185 before this
conductor; this conductor's rounds 21–25 ≈ $250 with their crafts and critics. Performance used about
$18 of its $100, lamp design about $30 of its $100.
