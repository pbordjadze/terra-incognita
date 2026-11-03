#!/usr/bin/env python3
"""Mold's peak memory at launch, as the headless check sees it (a rough stand-in for the device: on
Linux, Mesa's software driver keeps the GPU's buffers in the process, so they count as they do on an
iPhone, where the GPU's memory counts toward the app's limit).

    python3 tools/toys/mold/memory.py            # builds nothing; run cargo build --release first

Plays a two-second plan and prints the check's peak resident set (resource.getrusage of the child).
"""
import json, resource, subprocess, sys, tempfile
from pathlib import Path

TOYS = Path(__file__).resolve().parents[3] / 'toys'
with tempfile.TemporaryDirectory() as d:
    plan = Path(d) / 'mem.json'
    plan.write_text(json.dumps({'seconds': 2, 'night': True, 'shots': [2.0]}))
    subprocess.run([str(TOYS / 'target/release/mold-check'), 'play', str(plan), str(Path(d) / 'out')], check=True, stdout=subprocess.DEVNULL)
peak = resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss / 1024
print(f'mold-check peak RSS: {peak:.0f} MB')
