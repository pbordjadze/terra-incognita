#!/usr/bin/env bash
# Captures actual Debug-app simulator output: the sequence on the playground's one globe, its chrome
# over it. No device-performance inference is made.
# Usage: ci/camelid_capture.sh <TerraIncognita.app> <outdir> <udid>
# Set CAMELID_VIDEO=1 for a full, uninterrupted native playback recording.
set -euo pipefail
python3 - "$1" "$2" "$3" <<'PY'
import json
import os
from pathlib import Path
import plistlib
import signal
import subprocess
import sys
import time
import traceback
import uuid
from datetime import datetime, timezone

app, out, udid = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(), sys.argv[3]
out.mkdir(parents=True, exist_ok=True)
with (app / 'Info.plist').open('rb') as f:
    bundle = plistlib.load(f)['CFBundleIdentifier']

def run(*args, timeout=60, check=True):
    stamp = datetime.now(timezone.utc).isoformat()
    message = stamp + ' ' + ' '.join(str(a) for a in args)
    print(message, flush=True)
    with (out / 'capture-commands.log').open('a') as log:
        log.write(message + '\n')
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    if result.returncode and check:
        raise RuntimeError(f'{args[0:3]} failed: {result.stderr.strip()}')
    return result

def terminate():
    run('xcrun', 'simctl', 'terminate', udid, bundle, check=False)

def launch_ready(*arguments):
    token = str(uuid.uuid4())
    ready_marker.unlink(missing_ok=True)
    run('xcrun', 'simctl', 'launch', udid, bundle, *arguments,
        '-camelid-capture-token', token, timeout=120)
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        try:
            if ready_marker.read_text() == token:
                message = datetime.now(timezone.utc).isoformat() + ' camelid first frame submitted: ' + token
                print(message, flush=True)
                with (out / 'capture-commands.log').open('a') as log:
                    log.write(message + '\n')
                # Allow the compositor one brief settling interval after native submission.
                time.sleep(1)
                return
        except FileNotFoundError:
            pass
        time.sleep(0.25)
    raise RuntimeError('Camelid did not submit its first frame on the globe within 90 seconds; readiness token ' + token)

recording = None
try:
    run('xcrun', 'simctl', 'boot', udid, check=False)
    run('xcrun', 'simctl', 'bootstatus', udid, '-b', timeout=300)
    run('xcrun', 'simctl', 'install', udid, str(app), timeout=120)
    data_container = Path(run('xcrun', 'simctl', 'get_app_container', udid, bundle, 'data').stdout.strip())
    if not data_container.is_dir():
        raise RuntimeError('Simulator did not return an accessible app data container')
    ready_marker = data_container / 'tmp' / 'camelid-capture-ready.txt'
    # Match the established sim_shots.sh budgets while a cold runtime is still settling.
    # (a plain launch: the sequence lies on the playground's one globe, in its own style)
    run('xcrun', 'simctl', 'launch', udid, bundle, timeout=180)
    time.sleep(12)
    frames = []
    for label, elapsed in [('rewind', 4), ('origin', 17), ('branches', 32),
                           ('beringia', 47), ('americas', 62), ('cousins', 82)]:
        terminate()
        launch_ready('-camelid', str(elapsed))
        name = f'{label}.png'
        run('xcrun', 'simctl', 'io', udid, 'screenshot', '--type=png', str(out / name), timeout=90)
        frames.append({'file': name, 'guided_time_seconds': elapsed})

    if os.environ.get('CAMELID_VIDEO') == '1':
        terminate()
        recording = subprocess.Popen(['xcrun', 'simctl', 'io', udid, 'recordVideo',
                                      '--codec=h264', '--force', str(out / 'native-playback.mp4')],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        time.sleep(2)
        launch_ready('-camelid-play')
        # Include initial rendering and the final freely explorable resting composition.
        for _ in range(19):
            time.sleep(5)
            if recording.poll() is not None:
                raise RuntimeError('Simulator video recorder ended before playback finished')
        recording.send_signal(signal.SIGINT)
        _, errors = recording.communicate(timeout=30)
        if recording.returncode:
            raise RuntimeError(errors.decode('utf-8', 'replace'))
        recording = None

    (out / 'capture.json').write_text(json.dumps({
        'kind': 'native iOS Simulator output',
        'real_device_test': False,
        'measured_frame_rate': None,
        'frames': frames,
        'capture_readiness': 'Unique app acknowledgement after the playground globe submits its first frame with the sequence composed on it, then 1 second compositor settling; not a GPU completion or performance measurement.',
        'video': 'native-playback.mp4' if os.environ.get('CAMELID_VIDEO') == '1' else None,
        'note': 'Screenshots use held story times. Video runs the actual native guided clock.'
    }, indent=2) + '\n')
except Exception:
    (out / 'capture-error.txt').write_text(traceback.format_exc())
    raise
finally:
    if recording is not None and recording.poll() is None:
        recording.send_signal(signal.SIGINT)
        try:
            recording.communicate(timeout=30)
        except subprocess.TimeoutExpired:
            recording.kill()
            recording.wait(timeout=10)
    terminate()
    run('xcrun', 'simctl', 'shutdown', udid, check=False)
PY
