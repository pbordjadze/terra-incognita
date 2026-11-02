# Adrift: the rounds and their clocks

One line a round: what changed, and the wall clock of each loop step (the Interviewer's measure of
the developer experience against the Swift Terra loop: a look check there is 21–74 minutes on macOS
CI; Georgia ran 9 rounds in about 6 h, the night crossing 15 in about 5¾ h). Times UTC, Oct 31 2026.

| round | change | → Linux render seen | → web build seen | → device build (CI) | minutes waiting on CI |
| --- | --- | --- | --- | --- | --- |
| R0 | the skeleton: ground and sea ported, a float on the current, checks, shells, CI (started 06:50) | 07:14 (first `adrift-check` render, ~2 s a render on lavapipe) | 07:26 (local wasm build; published as a claude.ai artifact 07:27) | IPA built 07:50 on run 3 (27 MB); the job's own `ls` failed on a relative path, fixed; run 4 pending | ~35 (three runs: a `set -u` slip, a relative output path, the IPA's path) |
| R1 | the three crafts' heads merged (Truth's rules and sampler, Visual's Look, Motion's gestures), the app seam written once against them | 08:20 (render + played plan after the rebuilt merge) | — (not rebuilt this round) | run 5 (08:34): the IPA built in 1 min 45 s on the Mac but the job's own `ls` looked in the wrong directory (a fix lost in the branch rebuild) | ~25 |
| R2 (Truth, Visual's D3/D4, Motion's plans) | the put velocity, the drifter's track in silver with the bloom's clock, the credit line; the sea's eased rest and the night told to the look; 23 of Motion's 24 plans pass | 08:41 (held scene), 08:58 (night scene) | 09:02 (artifact v2, the web shell takes a data index for hosts that serve no binaries) | run 12 green on 3fa614f: pushed 08:53, Linux job done 08:56, the IPA artifact at 08:58:33 (5½ minutes push to device build); runs 6–11 were each superseded by the next push | ~40 across runs 4–12, mostly the pushes cancelling each other |

What the loop costs on Linux: a change to the core is a render in 10–20 s (incremental build 5–8 s,
load 1.8 s, draw 1–3 s on the software driver); a played plan of 25 s of touches is about 25 s;
Motion's 24 plans about 10 min. The web build is 45 s. The device build is CI only (no Mac here):
about 2 min of Rust plus Swift on a macOS runner once the runner is free, after the Linux job's 9 min.

Merged into `main` at 09:01 (R2 complete for all three crafts): the first Adrift build to the
SideStore source is `main`'s adrift.yml run. The device loop so far: a change pushed to `adrift` is
an unsigned IPA in about 5½ minutes when nothing else holds the Mac; Terra's own look check is
21–74 minutes.

| R3 | Truth's maybes budget (512 at a half day) and the measured sea on the float; Visual's dusting at the whole Earth, the label from zoom 1.5; Motion's ride aiming ahead, 27 plans | 09:08 (atlantic, whole Earth), 09:11 (27 plans in 2 min) | — | main run 14 built the IPA in 66 s and made release adrift-build-14 at 09:10, but its source publish failed on the merge's wrong copy of ci/sidestore_source.py; fixed and merged again at 09:12 (main run 15) | ~10 queued behind Terra's full run on the shared Macs |

**The first SideStore build:** main's run 20 (c94b4eb, pushed 09:13) published release adrift-build-20
and the two-app source at 09:17:44, four and a half minutes after the push; the Interviewer was
told at 09:20. From the first line of Rust (06:55) to an app in the SideStore source beside Terra:
2 h 23 m, three rounds of three crafts.

**The shipped round:** main 5823c0d (R3 closed) pushed 09:25; adrift.yml run 23 published release
adrift-build-23 and the two-app source at 09:29:10, three and a half minutes after the push (the
Linux job and the Mac's IPA in parallel on main). The source lists Terra 1.0.196 and Adrift 1.0.23.
From the first line of Rust (06:55) to the shipped third round in the source: 2 h 34 m, four rounds
(R0 the skeleton, then three of three crafts), each round's build kept as a release or a branch head.

**The loop against Terra's, in one line:** a look is seen in 10–20 s on Linux here against 21–74 min
on macOS CI there; a device build is about 5½ min after a push here against an hour and more for a
full run there; a round carried three crafts' work in about 40 min here against one change in
25–40 min there (Georgia, the night crossing). What this loop cannot do: show the thing moving on a
device. Every motion verdict so far is a still or a played plan on a software driver, and the beat
under a real finger and a fling toward the pole wait for the owner's capture (both shells log their
frame stats every five seconds for it).

**The shared source from Terra's side (11:35 UTC):** Terra's own full CI run on main (run 245,
2a19c67) finished green at 10:19 and published build-245 through the same `ci/sidestore_publish.sh`;
the source then listed Terra 1.0.245 and Adrift 1.0.23 together. Either workflow's publish keeps the
other app's entries, which is what the two-app script was for. The craft branches carry nothing
beyond `adrift`; nothing is left to merge.

| R4 (the first device verdict; DECISIONS § 11) | Engineering's seams first (clear-all and home, a ridden or pressed float showing its maybes, the check relinking assets, a scene's credits flag): opened 14:35, pushed 14:38–14:47; the crafts' heads 14:45–14:56 (Motion: home by over-pinch or three fingers up, clear by three fingers down or a second shake, a flick on the date, the credits by a rest on the date, 31 plans; Visual: the glitter quieted, the land's lights alive by night, the date's rule and lamp answering the finger, the foot cleared to the date; Truth: a float put anywhere the mask shows water, a lake's float on leeway and wander, the run and the loiter judged; nine demands filed, every one answered, countered or withdrawn the same hour) | 14:41 (clear-and-home played), 15:05 (the merged head: 27 tests, 36 plans, the credits page and the night's lights rendered) | 15:07 (web build 30 s; artifact version 5) | main 095620e pushed 15:01:06; adrift.yml run 30 published adrift-build-30 and the source at 15:04:42 (3 m 36 s push to device build) | ~4 |

The round's loop, from the owner's words (14:31) to the merged head with every craft's work (15:05): 34 minutes; the seams written before the crafts began cost 12 minutes and no branch was rebuilt.

| R5 (the second device verdict; DECISIONS § 12) | Engineering's seams first (the choice; then time made reversible: a float's place read off its leg's path on the quarter-day grid, the wake its day marks, a carry a new leg; the maybes stepped on from where they are, 24 half-day steps a frame in one submit, a keep every ten days for the rewind; the scrub measured in the check): opened 16:58, the choice pushed 17:05, reversible time 17:25; the crafts' heads 17:09–17:13 (Motion: the scrub leaves the clock as it found it, the choice through pinch and drag, 33 plans; Visual: a hue per lamp worn by its wake, cloud and silver, the date's breath, the owner's white bands and grey rings found and quieted; Truth: the flight's purity stated and tested, the sampler a third cheaper, keep/restore/step_many); merged 17:22 | 17:12 (rewind-returns played: a float back to the same place to a thousandth of a degree), 17:33 (the merged head: 30 tests, 47 plans) | 17:25 (artifact version 6; version 8 at 17:33 with the glance pass) | main ea76aa9 pushed 17:25 (run 36, superseded), the glance pass 0453e1c at 17:31 (run 38, superseded), the field's alpha dad5bf3 at 17:32: run 40 published adrift-build-40 and the source at 17:38:01 (5 m 27 s push to device build) | ~13 across the three |

The scrub, measured (`adrift-check/plans/scrub-measured.json`, thirty floats, one chosen, −100 and +100 days in two seconds each at 60 fps): the app's state 0.16–0.27 ms a frame mean, 0.7 worst, before and after; the frame on the software driver 490 → 356 → 296 ms mean as the maybes went from a re-run every half day (up to 600 dispatches a frame) to 24 steps a frame to one submit. What the device will feel is the stage's redraw at the budget's scale and the maybes' 24 steps; the state is nothing.

| R6 (the third device verdict; DECISIONS § 13–14; the first round on Opus) | Engineering's seams first (several clouds out at once in one buffer, the chosen set, the panel's bounds; then the camera's roll for the poles, the check's `panel` and `frames`): opened 19:15, pushed 19:28 and 19:35; three crafts started 19:31; heads 19:51–20:45 (Visual: the night's cities by brightness and the maybes cool glass, the lamp one speck of its cloud, the panel, pairs drawn as one speck until the currents part them, gentler rings; Motion: the panel's touch, a scrub limit of 600 days a second braked by the clouds, the rim, the orb over the poles; Truth: the worlds, a shared eddy field per maybe index, the lamp shown to be one of its cloud, at the same cost); merged 20:55 | 19:27 (two clouds at once rendered), 20:58 (the merged head: 38 tests, 51 plans) | 21:12 (artifact version 9) | main 75400f4 pushed 21:07:24; adrift.yml run 41 published adrift-build-41 and the source at 21:16:57 (9½ minutes push to device build) | ~10 |

The scrub with eight clouds out (`adrift-check/plans/scrub-eight-clouds.json`, thirty floats, −100 and +100 days in two seconds each): the app's state 0.16–0.18 ms a frame mean, as with one; the frame on the software driver 230 → 300 ms mean (one cloud → eight), the pair drawing within noise of that. From the owner's words (19:12) to the build in the source (21:17): 2 h 5 m.

| R7 (the pinch's second ring; DECISIONS § 15) | Engineering alone: the second finger that starts a pinch rings the sea where it lands; one test | 22:15 (39 tests, 51 plans) | 22:24 (artifact version 10) | main 8d44792 pushed 22:21:27; adrift.yml run published adrift-build-42 and the source at 22:31:09 (9¾ minutes) | ~10 |

| R8 (each lamp its own way; DECISIONS § 15) | Engineering's seams first (a lamp chosen as itself or as its cloud, the places that frame everything chosen): opened 22:20, pushed 22:25; Visual and Motion (round 6's sessions, briefed again) 22:25–00:02 (Visual: a followed lamp ringed with its whole journey, the panel's three marks; Motion: one tap three ways, the framing, the two-finger twist); a ride-keeps-the-cloud bug found by the crafts and fixed at merge | 00:30 (the merged head: 40 tests, 61 plans) | 00:35 (artifact version 11) | main 782e7ec pushed 00:32:49; adrift.yml run published adrift-build-43 and the source at 00:42:49 (10 minutes) | ~10 |

| R9 (the studio, step 1: a take recorded as data and exported as a 9:16 clip; DECISIONS § 16) | Engineering's seams first (the take, its replay and export, the device's AVAssetWriter and share sheet, the check's record/export; then 9:16): opened 07:20, pushed 07:31 and 07:55; Motion and Visual (reused) 07:33–08:12 (one control records, stops and exports; the clean clip and its caption); the GPU runner script for the owner's homelab | 08:25 (the merged head: 40 tests, 63 plans; a take's replay ends 0.000 km from the live take, its last frame identical) | 08:34 (artifact version 12) | main 4d26493 pushed 08:31:28; adrift.yml published adrift-build-46 and the source at 08:42:22 (11 minutes) | ~11 |

| R10 (the clip made, not captured; DECISIONS § 17; the first round under a new conductor) | Engineering's seams first (the clip drawn at its own 9:16 size through a camera of its own, eased beats at both ends, a speed; `render --clip`; plan expects for takes): opened 08:47, pushed 08:55; three fresh crafts briefed 08:58 with every peer's id; heads 09:11–09:22 (Visual: journeys one tapering stroke through quarter-day steps, the panel a row of journey marks without words, the clip's touch feedback out, the record ring an arc, a tap's ring over a lamp's cloud; Motion: `ClipCamera` (a damped follow, an opening push-in, an end beat that settles), a fling the same at any frame rate, the marks' touch; Truth: the wind's rules with gusts measured from reanalysis, `sea/MEDIA.md`'s dials); Engineering between (a float's trail, `panel_press`, VoiceOver for the marks, the export's camera trace) | 09:44 (the merged head: 54 tests, 72 plans; two panel plans updated for the mark's ring; replay 0.000 km) | 09:48 (artifact version 13, with `gusts.png`) | main e510c11 pushed 09:48:53; adrift.yml published adrift-build-47 and the source at 09:59:15 (10½ minutes) | ~18.6 (Visual 4.11, Motion 3.44, Truth 4.54, conductor 6.51 so far) |

| R10's fix (the web's night lights) | `night.jpg` added to the files the web loads (the web build could not start without it) | 14:42 | 14:45 (artifact version 14) | main 23cdfdd; adrift-build-48 | ~1 |

| R11 (conducting, the studio's step 2; DECISIONS § 18) | Engineering's seams first (a leg's world and carried eddy, the take's cut, each lamp's worlds on the CPU, `Intent::Conduct`, the look's cues; the spike's chooser ported): opened 14:45, pushed 14:51; the round-10 crafts briefed again 14:52; heads 15:04–15:16 (Visual: the cue flashed and folded, the clip's date on the paper, two clouds' hues; Truth: cues and cuts exact on the grid, 1024 worlds a share a frame, the tempo measured; Motion: the conductor's hand, the sweep at its peak, the clip's lean); Engineering's tempo (3 days a second while a take records); two older take plans moved their puts before the take | 15:44 (the merged head: 67 tests, 69 plans; replays 0.000 km) | 15:48 (artifact version 15) | main e9ffe44 pushed 15:48:21; adrift.yml published adrift-build-49 at 15:59 (11 minutes) | ~10 (crafts, estimated) |

| R12 + R13 (conducting seen and a song; two lamps kept and made into a clip; DECISIONS § 19–20) | Engineering's seams first (the song: shell playback during a take, the export's mix, the check's ffmpeg mix); the round-11 crafts briefed again 16:08; heads 16:15–16:33 (Visual: heavier clip lines, the cue as a fan, marks away while recording, the fermata; Truth: spinning eddies, a tap that looks ahead, the fan's runners-up; Motion: the clip and the live view frame the lamps, misreads tested). Held from main when the owner's request arrived (16:40): his two lamps would have changed paths under the spin; R13: Truth's per-leg law (16:48), Engineering's studio menu, journey clip, kept lamps, lamps as text, the check's `journey`; Visual's kept pin (17:11) | 17:28 (the merged head: 75 tests, 70 plans; replays 0.000 km; the Swift built in CI runs 50–51) | 17:31 (artifact version 16) | main 6db234c pushed 17:30:39; adrift.yml published adrift-build-52 at 17:39 (9 minutes; the build number counts the branch IPA runs) | ~30 (estimated) |

| R14 (the journey clip shows the dance; DECISIONS § 21) | Engineering's seam first (a varying pace carried in the take): 17:50; the crafts again 17:52; heads 17:55–18:11 (Visual: whole lines held, the marks quieter; Truth: the pace, the dancing pair; Motion: the clip framed tight and centred, breathing); Engineering's pair framing (60 km, zoom 40) after the first dance render lost the lines | 18:45 (79 tests, 70 plans) | 18:47 (artifact version 17) | main 5346442 pushed 18:47:30; adrift.yml published adrift-build-53 | ~12 (estimated) |

| R15 (the whole clip a dance; DECISIONS § 22) | the crafts again 19:00; heads 19:09–19:14 (Truth: the coast's slide as law 2, a bolder pace, an early-dancing pair; Visual: lines bend, a breathing first beat, no marks at rest; Motion: the pair and their lines framed, pull back not pan, a wide opening, a pulled-out end) | 19:40 (81 tests, 70 plans) | 19:45 (artifact version 18) | main 66862b7 pushed 19:45:01; adrift.yml published adrift-build-54 | ~10 (estimated) |

| R16 (the edges taken off; DECISIONS § 23) | the crafts again 20:10; heads 20:13–20:37 (Truth: pace by visible change; Visual: round joins and turns, magnified land eased; Motion: the whole braid held in the last third, the zoom capped near land) | 21:05 (82 tests, 70 plans) | 21:10 (artifact version 19) | main 5a82154 pushed 21:09:54; adrift.yml published adrift-build-55 | ~8 (estimated) |

| R17–R20 (the journey clip polished; DECISIONS § 24–27) | Motion and Visual each round (Truth to R16): the pair centred and only pulling out, splined lines; an establishing opening, smooth streamlines; a first-order then second-order camera limiter, open sea framed closer, a held ending, the whole trail kept, a glass head with a glint | each round 70 plans, 82–83 tests | artifact versions 20–23 | main 3d0fd2c (build 56), 40fedee (57), 5d9d94c (58), f6a2ded (59) | ~25 (estimated) |

Handover (Nov 2, 02:25 UTC): conductor session_01BAtskAkFkoMvNuQwcLwqT1 → session_01BAtsk6w32VucGg7U1d2NqA (from round 21), at context 0.73M. Spend since round 9 measured ≈ $185.
