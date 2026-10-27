#!/usr/bin/env bash
# Captures actual Debug-app simulator output. No device-performance inference is made.
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

app, out, udid = Path(sys.argv[1]).resolve(), Path(sys.argv[2]).resolve(), sys.argv[3]
out.mkdir(parents=True, exist_ok=True)
with (app / 'Info.plist').open('rb') as f:
    bundle = plistlib.load(f)['CFBundleIdentifier']

def run(*args, timeout=60, check=True):
    result = subprocess.run(args, capture_output=True, text=True, timeout=timeout)
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    if result.returncode and check:
        raise RuntimeError(f'{args[0:3]} failed: {result.stderr.strip()}')
    return result

def terminate():
    run('xcrun', 'simctl', 'terminate', udid, bundle, check=False)

recording = None
try:
    run('xcrun', 'simctl', 'boot', udid, check=False)
    run('xcrun', 'simctl', 'bootstatus', udid, '-b', timeout=300)
    run('xcrun', 'simctl', 'install', udid, str(app), timeout=120)
    run('xcrun', 'simctl', 'launch', udid, bundle, '-camelid', '16')
    time.sleep(12)
    frames = []
    for label, elapsed in [('rewind', 4), ('origin', 17), ('branches', 32),
                           ('beringia', 47), ('americas', 62), ('cousins', 82)]:
        terminate()
        run('xcrun', 'simctl', 'launch', udid, bundle, '-camelid', str(elapsed))
        time.sleep(6)
        name = f'{label}.png'
        run('xcrun', 'simctl', 'io', udid, 'screenshot', '--type=png', str(out / name))
        frames.append({'file': name, 'guided_time_seconds': elapsed})

    if os.environ.get('CAMELID_VIDEO') == '1':
        terminate()
        recording = subprocess.Popen(['xcrun', 'simctl', 'io', udid, 'recordVideo',
                                      '--codec=h264', '--force', str(out / 'native-playback.mp4')],
                                     stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        time.sleep(2)
        run('xcrun', 'simctl', 'launch', udid, bundle, '-camelid-play')
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
        'video': 'native-playback.mp4' if os.environ.get('CAMELID_VIDEO') == '1' else None,
        'note': 'Screenshots use held story times. Video runs the actual native guided clock.'
    }, indent=2) + '\n')
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
