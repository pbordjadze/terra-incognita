#!/usr/bin/env bash
# A film of a toy (or Terra) in the iOS Simulator with its own sound: one MP4 to watch and listen to,
# a contact sheet of its frames and pictures of its sound (a spectrogram with seconds and hertz, and a
# waveform), for a critic who can't hold the phone or hear it, and film.json: what played when.
#
#     bash tools/toys/sim_film.sh <toy> <script> <seconds> [out-dir]     # macOS with Xcode and rustup's aarch64-apple-ios-sim
#
# <script> is `demo` (the toy's own "i" demo, opened 3 s in, which plays real touches through the
# kit) or a file, `toys/<toy>/films/<script>.json` or a path: {"touches": [...], "open": "info",
# "open_after": 3}. Its touches are the shell's TOY_TOUCHES (toys/ios/Sources/main.swift): fingers
# played through toy_pointer, the call every UITouch makes, x and y as fractions of the screen, shown
# by a ring so the film shows the hand; their times are counted from the toy's start (a toy can take
# seconds, or a minute on a busy Mac, to compile its shaders), and film.json gives each touch's second
# in the film.
#
# How it hears: the Simulator's sound goes to the Mac's own output, which nothing on a Mac without a
# loopback device can record (ffmpeg's avfoundation offers only the microphone). So the shell records
# itself: TOY_RECORD_AUDIO taps the toy's AVAudioEngine mixer into a WAV in the app's Documents, the
# exact samples the toy made, not the room. How it lines up: the shell logs, once a frame, the samples
# rendered against the wall clock, and the samples' place in the file; the screen recording's clock is
# its stop less its length, checked against a mark the shell shows (a magenta square at the left edge, a
# second into the toy). The iOS 27 Simulator tells the engine 44.1 kHz and plays 48 kHz, so on the Mac's
# speakers a toy is 9% fast and sharp; the film keeps the toy's own pitch and puts its time back on the
# wall's (atempo by the rendered rate). On Bell's strikes, sound and touch agree within 50 ms.
#
# It builds the toy for the Simulator (SIM=1 toys/ios/build.sh; FILM_APP=<.app> reuses a build), makes
# its own simulator FILM-<toy> (FILM_DEVICE names another; an iPhone 17 Pro on the newest iOS), installs,
# records `simctl io recordVideo --codec h264` while the toy plays the script, then saves the simulator's
# log beside the film and deletes the simulator, even if a step fails (FILM_KEEP=1 keeps it; FILM_REMUX=1
# makes the film again from <out-dir>'s recordings, without a simulator). <out-dir> (default
# toys/build/<toy>/film-<script>) gets film.mp4 (720 px wide, AAC), sheet.png, sound.png, wave.png,
# film.json, and the raw: audio.wav, screen.mp4, audio-clock.txt, screen-clock.txt, mark.txt, sim-log.txt.
set -euo pipefail
TOY="${1:?usage: sim_film.sh <toy> <script> <seconds> [out-dir]}"
SCRIPT="${2:?usage: sim_film.sh <toy> <script> <seconds> [out-dir]}"
SECS="${3:?usage: sim_film.sh <toy> <script> <seconds> [out-dir]}"
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NAME="$(basename "$SCRIPT" .json)"
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${4:-$ROOT/toys/build/$TOY/film-$NAME}")"
mkdir -p "$OUT"
BUNDLE="$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['bundleIdentifier'])" "$ROOT/toys/$TOY/toy.json")"

# the script
OPEN=""; OPEN_AFTER=3; TOUCHES="[]"
if [ "$SCRIPT" = demo ]; then
  OPEN=info
else
  FILE="$SCRIPT"; [ -f "$FILE" ] || FILE="$ROOT/toys/$TOY/films/$SCRIPT.json"
  [ -f "$FILE" ] || { echo "no script $SCRIPT (nor $FILE)"; exit 1; }
  eval "$(python3 - "$FILE" <<'EOF'
import json, shlex, sys
s = json.load(open(sys.argv[1]))
print("TOUCHES=" + shlex.quote(json.dumps(s.get("touches", []), separators=(",", ":"))))
print("OPEN=" + shlex.quote(s.get("open", "")))
print("OPEN_AFTER=" + shlex.quote(str(s.get("open_after", 3))))
EOF
)"
fi

if [ "${FILM_REMUX:-0}" = 1 ]; then     # (again from <out-dir>'s screen.mp4, audio.wav and clocks, no simulator)
  read -r VIDEO_START VIDEO_END < "$OUT/screen-clock.txt"
  AUDIO_START="$VIDEO_START"
else
# the build
if [ -n "${FILM_APP:-}" ]; then
  APP="$FILM_APP"
else
  SIM=1 bash "$ROOT/toys/ios/build.sh" "$TOY" "$OUT/build" | tail -3
  APP="$(ls -d "$OUT"/build/Payload/*.app)"
fi

# its own simulator, its log kept and itself deleted when the film is made or the script fails
UDID=""
done_with_simulator() {
  [ -n "$UDID" ] || return 0
  xcrun simctl spawn "$UDID" log show --style compact --last 2h > "$OUT/sim-log.txt" 2> /dev/null || true
  if [ "${FILM_KEEP:-0}" != 1 ]; then
    xcrun simctl shutdown "$UDID" 2> /dev/null || true
    xcrun simctl delete "$UDID" || true
  fi
  UDID=""
}
trap done_with_simulator EXIT
DEV="${FILM_DEVICE:-FILM-$TOY}"
UDID="$(xcrun simctl list devices -j | DEV="$DEV" python3 -c '
import json, os, sys
for r, ds in json.load(sys.stdin)["devices"].items():
    for d in ds:
        if d["name"] == os.environ["DEV"]: print(d["udid"]); sys.exit()')"
if [ -z "$UDID" ]; then
  RUNTIME="$(xcrun simctl list runtimes -j | python3 -c '
import json, sys
rs = [r for r in json.load(sys.stdin)["runtimes"] if r["isAvailable"] and r["platform"] == "iOS" and r["identifier"].count("-") <= 4]
print(max(rs, key=lambda r: [int(x) for x in r["version"].split(".")])["identifier"])')"
  UDID="$(xcrun simctl create "$DEV" com.apple.CoreSimulator.SimDeviceType.iPhone-17-Pro "$RUNTIME")"
fi
echo "simulator: $DEV $UDID"
xcrun simctl boot "$UDID" 2> /dev/null || true
xcrun simctl bootstatus "$UDID" -b > /dev/null
xcrun simctl uninstall "$UDID" "$BUNDLE" 2> /dev/null || true
xcrun simctl install "$UDID" "$APP"
xcrun simctl io "$UDID" screenshot /dev/null > /dev/null 2>&1 || true     # (the warm-up: a first capture is slow)

# (the file's seconds may run faster than the wall's: below; so a tenth more, and a second)
RECORD_SECONDS="$(python3 -c "print(round($SECS * 1.1 + 1))")"
export SIMCTL_CHILD_TOY_RECORD_AUDIO=film-audio.wav SIMCTL_CHILD_TOY_RECORD_SECONDS="$RECORD_SECONDS"
export SIMCTL_CHILD_TOY_TOUCHES="$TOUCHES"
[ -n "$OPEN" ] && export SIMCTL_CHILD_TOY_OPEN="$OPEN" SIMCTL_CHILD_TOY_OPEN_AFTER="$OPEN_AFTER"

# the recording, then the toy
rm -f "$OUT/screen.mp4"
xcrun simctl io "$UDID" recordVideo --codec h264 --force "$OUT/screen.mp4" 2> "$OUT/record.txt" &
REC=$!
for _ in $(seq 1 300); do grep -q "Recording started" "$OUT/record.txt" 2> /dev/null && break; sleep 0.1; done
VIDEO_START="$(python3 -c 'import time; print(f"{time.time():.4f}")')"
xcrun simctl launch --terminate-running-process "$UDID" "$BUNDLE" > /dev/null
# <seconds> of the toy itself, from its first sound (a toy can take seconds to start: its shaders compile
# at launch), or from the launch if no sound comes in 30 s; the script's times count from the toy's start too
DATA="$(xcrun simctl get_app_container "$UDID" "$BUNDLE" data)"
for _ in $(seq 1 300); do [ -f "$DATA/Documents/film-audio.wav.start" ] && break; sleep 0.1; done
python3 -c "import time; time.sleep($SECS + 1)"
# then until the recorder has closed its file (it writes the clock as it does), at most <seconds> more
for _ in $(seq 1 $((SECS * 10))); do [ -f "$DATA/Documents/film-audio.wav.clock" ] && break; sleep 0.1; done
VIDEO_END="$(python3 -c 'import time; print(f"{time.time():.4f}")')"
kill -INT "$REC"; wait "$REC" || true
echo "$VIDEO_START $VIDEO_END" > "$OUT/screen-clock.txt"
xcrun simctl terminate "$UDID" "$BUNDLE" 2> /dev/null || true
cp "$DATA/Documents/film-audio.wav" "$OUT/audio.wav" 2> /dev/null || echo "no sound recorded (the toy has none, or its engine did not start)"
cp "$DATA/Documents/film-audio.wav.clock" "$OUT/audio-clock.txt" 2> /dev/null || true
cp "$DATA/Documents/film-audio.wav.mark" "$OUT/mark.txt" 2> /dev/null || true
AUDIO_START="$(cat "$DATA/Documents/film-audio.wav.start" 2> /dev/null || echo "$VIDEO_START")"
fi

# the sound's own clock, fitted from every buffer's time and the sound before it: where it starts on
# the wall clock and how many samples a wall-clock second really took. (The iOS 27 Simulator on this
# Mac reports 44.1 kHz to the engine and plays 48 kHz, so a toy renders for 44.1 and is heard 9% fast
# and sharp on the Mac's speakers; the file has the toy's own pitch, so the film keeps it and stretches
# its time onto the wall's: TEMPO, the true rate over the file's.)
TEMPO=1
if [ -s "$OUT/audio-clock.txt" ]; then
  read -r AUDIO_START TEMPO < <(python3 - "$OUT/audio-clock.txt" <<'EOF2'
import statistics, sys
rows = [line.split() for line in open(sys.argv[1]) if line.strip()]
rate = float(next(r[3] for r in rows if r[0] == "T"))
ren = [(float(r[2]), float(r[1])) for r in rows if r[0] == "R"]     # (samples rendered, wall)
ren = [p for p in ren if p[0] > 0]
s = [x for x, _ in ren]; w = [y for _, y in ren]
ms, mw = statistics.fmean(s), statistics.fmean(w)        # least squares: wall seconds a sample rendered
slope = sum((a - ms) * (b - mw) for a, b in zip(s, w)) / sum((a - ms) ** 2 for a in s)
# the file's first sample among those rendered: at each buffer at least the file and that buffer were
# rendered, so the least margin over the buffers is where the file began
first = max(min(int(r[4]) - int(r[1]) - int(r[5]) for r in rows if r[0] == "T"), 0)
start = mw + slope * (first - ms)                        # the wall at the file's first sample
spread = max(abs(y - (mw + slope * (x - ms))) for x, y in zip(s, w))
print(f"{start:.4f} {1 / slope / rate:.5f}")
print(f"sound clock: {len(ren)} frames, {1 / slope:.0f} samples a second rendered (the file says {rate:.0f}), "
      f"within {spread:.3f}s of the line; the file began {first / rate:.3f}s into them", file=sys.stderr)
EOF2
)
fi

# the simulator's log, then the simulator (also if anything above failed)
[ "${FILM_REMUX:-0}" = 1 ] || done_with_simulator

# one MP4: the sound moved to the screen's clock, both cut to <seconds>
# (the screen's clock is read from the end, the second it was stopped less its length: "Recording
# started" comes seconds after its first frame)
LENGTH="$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$OUT/screen.mp4")"
echo "screen: began $(python3 -c "print(f'{$VIDEO_END - $LENGTH - $VIDEO_START:.2f}')")s from its message"
VIDEO_START="$(python3 -c "print(f'{$VIDEO_END - $LENGTH:.4f}')")"
# better: the shell's mark (main.swift, showMark), a magenta square at the left edge a second into the
# toy, found in the recording; its wall-clock second less its place in the recording is the start
if [ -s "$OUT/mark.txt" ]; then
  FROM="$(python3 -c "print(f'{max($(cat "$OUT/mark.txt") - $VIDEO_START - 4, 0):.2f}')")"
  MARK_AT="$(ffmpeg -v error -i "$OUT/screen.mp4" -an \
      -vf "trim=start=$FROM:duration=10,crop=48:48:12:222,signalstats,metadata=print:file=-" -f null - 2> /dev/null | python3 -c "
import sys
t = None
for line in sys.stdin:
    if line.startswith('frame:'): t = float(line.split('pts_time:')[1])
    elif 'UAVG=' in line: u = float(line.split('=')[1])
    elif 'VAVG=' in line and u > 170 and float(line.split('=')[1]) > 180: print(f'{t:.4f}'); break
")"
  if [ -n "$MARK_AT" ]; then
    echo "screen: the mark at ${MARK_AT}s; from it the screen began $(python3 -c "print(f'{$(cat "$OUT/mark.txt") - $MARK_AT - $VIDEO_START:+.2f}')")s from its end's estimate"
    VIDEO_START="$(python3 -c "print(f'{$(cat "$OUT/mark.txt") - $MARK_AT:.4f}')")"
  else
    echo "screen: no mark found; the clock from its end"
  fi
fi
LAG="$(python3 -c "print(f'{$AUDIO_START - $VIDEO_START:.3f}')")"
echo "sound starts ${LAG}s after the screen"
# (the film begins half a second before the toy's first sound: the launch screen is left out. The
# recording's frames come only when the screen changes, so it is made steady first, then cut: seeking
# into it shifted the picture by half a second)
CUT="$(python3 -c "print(f'{max($LAG - 0.5, 0):.3f}')")"
# (the sound's lead-in is real silence put before it: adelay's was lost on the way into AAC)
GRAPH="[0:v]fps=30,trim=start=$CUT,setpts=PTS-STARTPTS,scale=720:-2[v]"
INPUTS=(-i "$OUT/screen.mp4"); MAP=(-map "[v]")
if [ -f "$OUT/audio.wav" ]; then
  RATE="$(ffprobe -v error -show_entries stream=sample_rate -of csv=p=0 "$OUT/audio.wav")"
  GRAPH="$GRAPH;$(python3 -c "
l = $LAG - $CUT
if l > 0:
    print(f'anullsrc=r=$RATE:cl=stereo:d={l:.3f}[s];[1:a]atempo=${TEMPO}[t];[s][t]concat=n=2:v=0:a=1[a]')
else:
    print(f'[1:a]atempo=${TEMPO},atrim=start={-l:.3f},asetpts=PTS-STARTPTS[a]')")"
  INPUTS+=(-i "$OUT/audio.wav"); MAP+=(-map "[a]" -c:a aac -b:a 160k)
fi
ffmpeg -loglevel error -y "${INPUTS[@]}" -filter_complex "$GRAPH" "${MAP[@]}" -t "$SECS" \
  -c:v libx264 -crf 26 -preset slow -pix_fmt yuv420p -movflags +faststart "$OUT/film.mp4"

# the contact sheet: 12 frames, left to right and down, at the seconds film.json lists (this ffmpeg
# may lack drawtext, so they aren't written on the frames)
ffmpeg -loglevel error -y -i "$OUT/film.mp4" -vf "fps=12/$SECS:round=down,scale=240:-2,tile=6x2:padding=4" \
  -frames:v 1 "$OUT/sheet.png"
# the sound as pictures: its spectrogram, seconds along the foot and hertz up the side (sound.png), and
# its waveform across the same seconds (wave.png)
if [ -f "$OUT/audio.wav" ]; then
  ffmpeg -loglevel error -y -i "$OUT/film.mp4" -lavfi "showspectrumpic=s=1400x500:legend=1:fscale=log:stop=8000" "$OUT/sound.png"
  ffmpeg -loglevel error -y -i "$OUT/film.mp4" -lavfi "showwavespic=s=1400x240:split_channels=0:colors=white" -frames:v 1 "$OUT/wave.png"
fi
FILM_ZERO="$(python3 -c "print(f'{$VIDEO_START + $CUT:.4f}')")"     # (the wall-clock second of the film's first frame)
python3 - "$OUT/film.json" "$TOY" "$NAME" "$SECS" "$LAG" "$TOUCHES" "$OPEN" "$OPEN_AFTER" "$TEMPO" "$FILM_ZERO" "$OUT/audio-clock.txt" <<'EOF'
import json, sys
import os
out, toy, script, secs, lag, touches, opn, after = sys.argv[1:9]
json.dump({"toy": toy, "film_starts_s_before_sound": 0.5, "sound_tempo": float(sys.argv[9]), "script": script, "seconds": float(secs), "sound_lag_s": float(lag),
           "touches": json.loads(touches), "open": opn or None, "open_after": float(after) if opn else None,
           "sheet_seconds": [round(k * float(secs) / 12, 2) for k in range(12)],
           "touched_at_film_seconds": [
               f"{'down' if r[0] == 'D' else 'up'} {r[1]} {float(r[2]) - float(sys.argv[10]):.2f}"
               for r in (line.split() for line in (open(sys.argv[11]) if os.path.exists(sys.argv[11]) else []))
               if r and r[0] in "DU"]},
          open(out, "w"), indent=1)
EOF
rm -rf "$OUT/build"
ls -l "$OUT"
