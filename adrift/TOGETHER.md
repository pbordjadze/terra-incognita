# Adrift: Together

An angle on Adrift (painting-company row 199), on the branch `adrift-together`: once a day you get
one lamp. Rest a finger on the sea and the ring gathers into a lit lamp (about two and a half
seconds); lift it and the lamp is on the sea. After that it can't be held, carried, undone or taken
out. It drifts in real time, a few km a day on the month's measured current, among everyone else's
lamps. When two lamps come within 22 km of each other, both glow toward each other for a few
seconds. The land's lights are turned down so the lamps are the night's lights.

- `crates/adrift-core/src/together.rs`: the rules (the wall clock, the day's lamp, the simulated
  others, meetings, letting time pass); `App::set_together` and the hooks at the end of `app.rs`;
  `look/lamps.rs` and `lamps.wgsl` draw the others.
- **Real:** the sea (the same rules, sheets and steps as Adrift), the clock (the wall's, local
  midnight turns the day), your lamps (kept in the browser's storage on this device).
- **Simulated:** everyone else's lamps. About 34 to 50 a day for the last 100 days, seeded by the
  day (so every visit and every device sees the same sea), set off coastal cities and some on the
  open sea, each stepped by the same rules in its own eddy world. A lamp that comes ashore goes out
  in a day and a half. Nothing is sent or fetched.
- **Letting time pass** (for trying it in one sitting): two fingers resting still anywhere, or one
  on the dark off the globe, for a moment; the days run at about one every three seconds until they
  lift. On a keyboard, hold `.`. The days passed are kept.

Check it headless (the release, a refused second lamp, days passing, a meeting, the next visit):

    cargo run --release -p adrift-check -- together build/out/together

The web build: `ADRIFT_MODE=together bash tools/adrift/web-build.sh`.
`tools/adrift/web-look.cjs` runs a web build in Chromium, but on this Linux box it can't screenshot
a WebGPU canvas (blank shots), so the look is judged on `adrift-check` renders.

A real shared sea needs a server: one small store of lamps (place, put day; no names, no accounts),
read on open and written once a day per device, with rate limits against floods. The sea itself
stays on the device: every lamp's path is a function of where and when it was set.
