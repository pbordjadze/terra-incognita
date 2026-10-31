# Adrift: the rounds and their clocks

One line a round: what changed, and the wall clock of each loop step (the Interviewer's measure of
the developer experience against the Swift Terra loop: a look check there is 21–74 minutes on macOS
CI; Georgia ran 9 rounds in about 6 h, the night crossing 15 in about 5¾ h). Times UTC, Oct 31 2026.

| round | change | → Linux render seen | → web build seen | → device build (CI) | minutes waiting on CI |
| --- | --- | --- | --- | --- | --- |
| R0 | the skeleton: ground and sea ported, a float on the current, checks, shells, CI (started 06:50) | 07:14 (first `adrift-check` render, ~2 s a render on lavapipe) | 07:26 (local wasm build; published as a claude.ai artifact 07:27) | IPA built 07:50 on run 3 (27 MB); the job's own `ls` failed on a relative path, fixed; run 4 pending | ~35 (three runs: a `set -u` slip, a relative output path, the IPA's path) |
| R1 | the three crafts' heads merged (Truth's rules and sampler, Visual's Look, Motion's gestures), the app seam written once against them | 08:20 (render + played plan after the rebuilt merge) | — (not rebuilt this round) | run 4 (08:29) superseded by run 5 (08:34): the IPA built in 1 min 45 s on the Mac but the job's own `ls` looked in the wrong directory (a fix lost in the branch rebuild); run 6 pending | ~25 so far |
| R2 (Truth, Visual's D3/D4, Motion's plans) | the put velocity, the drifter's track in silver with the bloom's clock, the credit line; the sea's eased rest and the night told to the look; 23 of Motion's 24 plans pass | 08:41 (held scene), 08:58 (night scene) | 09:02 (artifact v2, the web shell takes a data index for hosts that serve no binaries) | runs 7 and 8 superseded by run 9 (08:56, Visual's silver merged): each push restarts the branch's one run | (filling) |

What the loop costs on Linux: a change to the core is a render in 10–20 s (incremental build 5–8 s,
load 1.8 s, draw 1–3 s on the software driver); a played plan of 25 s of touches is about 25 s;
Motion's 24 plans about 10 min. The web build is 45 s. The device build is CI only (no Mac here):
about 2 min of Rust plus Swift on a macOS runner once the runner is free, after the Linux job's 9 min.
