# What's new: the owner's two minutes with a build

The SideStore build plays what is new in it one piece at a time, each from where its makers meant it
to be seen, and asks one tap after each. The owner's reactions are the scarcest input the company
has; this is how they are gathered without homework (painting-company row 187). This page is the
contract: what the owner does, how a session adds its piece, where the answers go. The code is
`App/TerraIncognita/WhatsNew/` (the shared module) and `TerraWhatsNew.swift` (Terra's host); the
tool is `tools/whatsnew/whatsnew.py`.

- [What the owner does](#what-the-owner-does)
- [Add your piece](#add-your-piece) (every session, as part of shipping)
- [`start`: where a piece begins](#start-where-a-piece-begins)
- [Where the answers go](#where-the-answers-go)
- [Checking it without a Mac](#checking-it-without-a-mac)
- [The seam: another app](#the-seam-another-app)

## What the owner does

1. Open the build. If it has pieces not yet reacted to, a gold pill under the buttons says
   **12 new things · Show me**. Touch it. (The workbench's **What's new** row is the other way in,
   and the way back to any piece, new or seen.)
2. A round is five pieces, the newest first. Each starts where its makers meant: the camera flies,
   the experience starts, the date is set, a country is lit; the other doorways step off the globe
   and the clouds thin, so nothing else is in the way. The bar above says *2 of 5* and the piece's
   name, and under them, in gold, the one thing its makers ask to be judged. The globe is live:
   touch it, drag, zoom, as always.
3. One tap on the band below: **Wow**, **Nice** or **Meh**. The next piece comes. **Skip** (not
   now) and **Didn't work** are plain words under them. A voice note: tap **Say**, speak, tap again
   (or just tap a verdict: the note stops and goes with it). A picture of the screen: **Picture**.
   **‹** goes back a piece; **Panel** opens the piece's own words; **Later** leaves the round where
   it is (what was answered stays answered).
4. At the end, a card: what was said, **Done**, **Keep going** while pieces wait, **Share…** (the
   files as a zip, by AirDrop, Files, Mail), and one quiet line about sending to the company (one
   tap once a token is set up; see [Where the answers go](#where-the-answers-go)).

Two minutes. Nothing is scored but the owner's time: a skipped piece stops asking until its makers
change it, and the list keeps the way back.

## Add your piece

A piece is a few lines saying what it is, where to start and what you want to know. Add it in the
same commit as the work; a session that ships without one ships into silence.

**A pack:** a `whatsNew` entry in its `prototype.json` (the scaffold has one to fill in). The piece's
id is the pack's; its title the pack's unless given. Its `start` runs the pack unless given.

```json
"whatsNew": {
  "what": "Challenger sailing her whole way. Touch the sea anywhere and a lead goes down to the floor's true depth.",
  "ask": "Is touching the sea to sound it discoverable without being told?",
  "since": "2026-10-29",
  "version": "1",
  "row": 176
}
```

**Native work, or a piece that runs several things** (a globe style, a switch, a set of rules): an
entry in `App/TerraIncognita/WhatsNew/WhatsNew.json`, with an `id` of its own and a `start`.

```json
{ "id": "quiet-globe", "title": "The quiet globe",
  "what": "Three rules at once: almost no names, clouds that clear where you look, nothing at the foot until you choose.",
  "ask": "Quieter, or just emptier?", "since": "2026-10-29", "version": "1",
  "start": { "run": ["quiet-names", "quiet-clouds", "quiet-frame"], "camera": { "lon": 10, "lat": 45, "zoom": 3 } }, "row": 174 }
```

| Key | | |
| --- | --- | --- |
| `id` | central entries only | lower-case letters, digits, hyphens; never reused (a pack's piece is the pack's id) |
| `title` | 1–60 characters | on the bar; a pack's defaults to the pack's |
| `ask` | up to 160 | the bar's line, in gold: the one thing you want judged, as an imperative the taps can answer ("Come close to one animal: judge the date slowing with you"), not a yes/no question. One. |
| `what` | one line, up to 240 | what is on the screen, for the tool's `list` and for the bar when there is no `ask`; say what the owner sees, not what you built |
| `since` | `YYYY-MM-DD` | the day it landed; the queue plays the newest first |
| `version` | optional, `"1"` | bump it (`"2"`, a date) when the piece changed enough to ask again: the owner sees it as *changed since you saw it*. A fix that changes nothing they would notice leaves it alone |
| `start` | optional | where to begin ([below](#start-where-a-piece-begins)); a pack's defaults to running the pack |
| `row` | optional | the painting-company row, kept with the answers, never shown |

A pack's entry travels with the pack: a published revision brings its piece to every phone with no
IPA. A central entry needs the next build. A pack's entry wins over a central one of the same id.
`python3 tools/playground/playground.py validate` checks a pack's entry; `python3 tools/whatsnew/whatsnew.py validate`
checks them all (CI's `plates` job runs both); `whatsnew.py list` prints the queue as the app orders it.

What makes a good piece: the owner sees it cold, on a phone, for twenty seconds. Start it at its
moment, not at its beginning (`camera`, `date`: the vine with centuries grown, the animal large in
frame, the country centred and clear); a piece that arrives at zero is skipped. Ask the one thing
you would change the work for, as something to judge. The first critic's reading of the first
round, in `tools/whatsnew/ROUNDS.md`, is what a cold owner sees.

## `start`: where a piece begins

Every key is optional. With none, the pack's own opening (its anchor, its script).

| Key | | |
| --- | --- | --- |
| `run` | `["herds"]` | prototypes to start, in order, the last in front (packs or natives; up to 6). Everything else running ends first |
| `camera` | `{ "lon", "lat", "zoom" }` | where the globe flies, instead of the first one's anchor; zoom 0.8–32 (over 8 only if a pack asks the globe for it) |
| `date` | a number | the front pack's date in its unit (a year; days since 1970 for `day`), taken when the pack shows its date |
| `playing` | `true`/`false` | the date plays from there (or waits) |
| `wikipedia`, `clouds` | `true`/`false` | the switches, put back after the round |
| `select` | `"Georgia"` | a country lit by name (its short or long name) |
| `open` | `true` | the front piece's panel opens at once |
| `doorways` | `true` | the other experiences' doorways stay on the globe (hidden while a piece plays, unless the piece is about them) |

A piece runs what any reader can run: nothing starts that the build doesn't have (a piece naming a
prototype the build lacks waits for the build that has it).

## Where the answers go

Every answer is one JSON file, with the voice note (`.m4a`) and the picture (`.jpg`) beside it when
there are any, under `interviews/paul/terra/<round>/` in the private company repository, where
sessions read them. The round is its start time, Eastern (`2026-10-30T0912-33`); a file is
`<time>-<piece>.json`:

```json
{ "app": "terra-incognita", "build": "214", "appVersion": "1.0.214", "device": "iPad16,6", "os": "26.1",
  "piece": "herds", "pieceVersion": "1", "title": "The herds", "ask": "Does the date slowing…", "row": 167,
  "reaction": "wow", "at": "2026-10-30T09:13:10-04:00", "seconds": 37.2,
  "voice": "2026-10-30T0913-10-herds.m4a", "screenshot": null,
  "state": { "camera": { "lon": -104.2, "lat": 41.5, "zoom": 2.2 }, "running": ["herds"], "front": "herds",
             "date": { "unit": "year", "value": -6900000, "playing": true }, "wikipedia": false, "clouds": true, "month": 10 },
  "round": "2026-10-30T0912-33" }
```

`reaction` is `wow`, `nice`, `meh`, `broken` (Didn't work) or `skip`. `state` is what was on the
screen when the tap came, enough to put the globe back there (`-globe-at`, the preview's `at`).
`seconds` is how long the piece was looked at.

**The path.** The answers stay on the device until the owner taps **Send** (the end card, or the
workbench's What's new › Sending). Sending is the module's one use of the network: an HTTPS PUT a
file to the GitHub contents API (`api.github.com/repos/<repo>/contents/<path>`), one commit each,
with the owner's token. A file already there is left alone, so a retry after a half-sent round sends
the rest. **The token** is a fine-grained personal access token for the private repository only, with
*Contents: Read and write*, entered once under Sending and kept in the device's Keychain; it is never
in a repository, a log, a file or a screenshot. Without a token, **Share…** gives the same files as a
zip to AirDrop, Files or Mail, and someone files them by hand. No analytics, no accounts, nothing
sent that the owner didn't tap.

**What is private.** The voice notes and pictures are the owner's; so is everything in `state`. They
go only to the private repository. Nothing of them is in this public repository, its issues or its
CI (the UI test's picture is of the simulator, judged for darkness, not kept).

**The ledger.** What was seen and how it was answered is `ledger.json` in the app's Application
Support, by piece id and version; a build shows only what isn't there at its current version. The
workbench's What's new › *See everything again* forgets it. `-whatsnew-reset` (Debug) wipes it and
the device's answers for a check, and leaves the token unused.

## Checking it without a Mac

```sh
python3 tools/whatsnew/whatsnew.py list                 # the queue as the app orders it, with each piece's start
python3 tools/whatsnew/whatsnew.py validate             # every piece sound; what CI's plates job runs
python3 tools/whatsnew/whatsnew.py preview build/whatsnew   # each pack piece's start photographed in the desktop preview
```

On the simulator: `WhatsNewTests` (`[test WhatsNewTests]`, about a quarter of an hour on the iPad)
walks a round from `-whatsnew` (the round at launch; `-whatsnew-piece <id>` one piece;
`-whatsnew-reset` a clean ledger), reacts, takes a picture and checks it has the globe in it,
reaches the end card, finds the way back in the workbench and plays one piece alone, and leaves the
plain globe with nothing alive. Its screenshots are the report's (`ci/fetch.sh`).

## The seam: another app

The module is five files that know nothing of a globe (`WhatsNewPiece`, `WhatsNewStore`,
`WhatsNewRecorder`, `WhatsNewSession`, `WhatsNewView`) and one that does (`TerraWhatsNew.swift`).
Another app (Paint by Moonlight, Pigeon) copies the five, writes its own host against the protocol
`WhatsNewHost` (put the app where a piece begins, take it away, say what the reader sees, open its
thing, photograph the screen; six calls), decides what its `start` keys mean, lists its pieces
(from a bundled JSON and whatever its content units carry), and gives the store its app name and
destination folder (`interviews/paul/<app>`). The chrome, the taps, the ledger, the files, the
courier and the token are shared as they are. `UNRELEASED` in the build settings compiles it in;
a store build leaves it out.
