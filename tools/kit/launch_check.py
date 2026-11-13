#!/usr/bin/env python3
"""One launch check for any engine app: what made apps close at launch on the phone, caught on Linux.

    python3 tools/kit/launch_check.py --check '<command> {plan} {out} --size {size} --ppp {ppp}' \\
        --manifest <manifest.json> [--budget-mb N | --app-json <toy.json>] [--plan <plan.json>] \\
        [--size 1170x2532] [--ppp 3] [--ipa <app.ipa> [--ipa-data data]]

    python3 tools/kit/launch_check.py --check 'toys/target/release/whales-check play {plan} {out} --size {size} --ppp {ppp}' \\
        --manifest toys/whales/manifest.json --app-json toys/whales/toy.json

The check command is run once with the app's short launch plan (default: two seconds, a frame at the
end), under the environment toy-kit reads (toys/crates/toy-kit/src/budget.rs):

  (a) shaders: TOY_DUMP_SHADERS=<dir> makes the kit write every shader it builds; each is translated to
      Metal Shading Language 2.4 by naga, as wgpu does on iOS (Linux runs them through SPIR-V instead).
      Needs naga-cli 30, the naga of the toys' wgpu (cargo install naga-cli --version 30.0.1 --locked).
  (b) the iPhone's GPU: KIT_IPHONE_LIMITS=1, so the device is requested with the limits an Apple7 iPhone
      gives wgpu 26 and no feature it lacks, at the phone's pixel size (390x844 points at 3x: 1170x2532);
      a GPU validation error is fatal in the check (on the device it is logged).
  (c) memory: the process's peak resident set (the child's ru_maxrss, i.e. VmHWM) against the budget,
      with the GPU's own peak bytes (KIT_MEMORY_REPORT; wgpu's counters) said beside it. On Linux the
      GPU is Mesa's lavapipe, whose buffers and textures live in the process; on an iPhone memory is
      unified and the GPU's allocations count toward the footprint the system kills on (jetsam); so
      peak RSS is a fair proxy, not a measurement of the phone.
  (d) with --ipa, every file of the manifest is in the IPA's Payload/*.app/<ipa-data>/.

The budget is --budget-mb, or the app JSON's "memory_mb", or DEFAULT_BUDGET_MB. Exit status 1 if any
part fails; the last lines say which. It cannot run Apple's Metal compiler or the phone's memory
pressure; the device's own log (toy-log.txt) is the last word.
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

# The oldest iPhones iOS 17 runs on (toys/ios/Info.plist: MinimumOSVersion 17.0) are the iPhone XS and
# XR (A12, 3-4 GB). Apple publishes no per-app memory limit; the community's measured table (Stack
# Overflow question 5887248, "iOS app max memory budget": a test app allocating until killed) gives
# about 1.8 GB for the XR and 2.0 GB for the XS. The budget leaves room for what the Linux check cannot
# see (the phone's drawables, UIKit and the audio engine, Metal's compiled pipelines) and is an
# estimate, not Apple's figure: 1400 MB. (Nov 3, 2026, the four toys peaked on this check at 206-650 MB;
# Mold before its memory cut, 6aeb591's parent, at 797 MB with today's kit, 1058 MB with the kit of then:
# under the budget, so that crash, if it was memory, is not one this budget can claim to catch.)
DEFAULT_BUDGET_MB = 1400
LAUNCH_PLAN = {"seconds": 2, "fps": 10, "shots": [2.0]}


# A declaration (a member, a variable, an argument) whose name is one of Metal's type names.
MSL_TYPE_NAME = re.compile(r"^\s*(?:const\s+)?(?:metal::)?[A-Za-z_][\w:<>]*\s+(half[234]?|bfloat[234]?|uchar[234]?|ushort[234]?|char[234]?|short[234]?|float[234]|int[234]|uint[234]|bool[234]|vec|texture2d|sampler|kernel|vertex|fragment|device|constant|thread|threadgroup)\s*(?:\[\[|;|=|,|\))")


def say(ok, line):
    print(("ok    " if ok else "FAIL  ") + line, flush=True)
    return ok


def shaders(dump: Path):
    naga = shutil.which("naga")
    if not naga:
        return say(False, "shaders: naga not found (cargo install naga-cli --version 30.0.1 --locked)")
    files = sorted(dump.glob("*.wgsl"))
    if not files:
        return say(False, f"shaders: none dumped in {dump} (does the app build them through toy-kit's Gpu::shader?)")
    bad = []
    for f in files:
        r = subprocess.run([naga, "--metal-version", "2.4", str(f), str(f.with_suffix(".metal"))], capture_output=True, text=True, check=False)
        if r.returncode != 0:
            bad.append(f"{f.name}: {(r.stderr or r.stdout).strip()}")
            continue
        # naga passes some names through that Apple's Metal compiler refuses: a declaration named
        # after a Metal type (Mold's varying `half`, Nov 4, 2026: `float half [[user(loc2)]]`, its
        # pipeline refused on the phone while every check on Linux passed). Rename it in the WGSL.
        for n, line in enumerate(f.with_suffix(".metal").read_text().splitlines(), 1):
            m = MSL_TYPE_NAME.search(line)
            if m:
                bad.append(f"{f.name}: MSL line {n} declares `{m.group(1)}`, a Metal type name Apple's compiler refuses: {line.strip()}")
    for b in bad:
        print("      MSL FAILS: " + b)
    return say(not bad, f"shaders: {len(files) - len(bad)} of {len(files)} translate to MSL 2.4")


def bundle(ipa: Path, manifest: Path, data: str):
    names = list(json.loads(Path(manifest).read_text())["files"])
    with zipfile.ZipFile(ipa) as z:
        have = {m.split("/", 3)[3] for m in z.namelist() if re.match(rf"Payload/[^/]+\.app/{re.escape(data)}/[^/]+$", m)}
    missing = [n for n in names if n not in have]
    if missing:
        print(f"      missing from the IPA: {missing}")
    return say(not missing, f"bundle: {len(names) - len(missing)} of {len(names)} manifest files in the IPA")


def launch(args, work: Path):
    plan = Path(args.plan) if args.plan else work / "plan.json"
    if not args.plan:
        plan.write_text(json.dumps(LAUNCH_PLAN))
    dump, report, out = work / "shaders", work / "memory.json", work / "out"
    cmd = args.check.format(plan=plan, out=out, size=args.size, ppp=args.ppp)
    env = dict(os.environ, KIT_IPHONE_LIMITS="1", TOY_DUMP_SHADERS=str(dump), KIT_MEMORY_REPORT=str(report))
    p = subprocess.Popen(cmd, shell=True, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    output, _ = p.communicate()
    rss = __import__("resource").getrusage(__import__("resource").RUSAGE_CHILDREN).ru_maxrss / 1024
    ok = say(p.returncode == 0, f"launch under the iPhone's limits at {args.size} @{args.ppp}x: exit {p.returncode}")
    if p.returncode != 0:
        print("      " + "\n      ".join(output.strip().splitlines()[-15:]))
    mem = json.loads(report.read_text()) if report.exists() else {}
    return ok, dump, rss, mem


def main():
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--check", help="the app's check command; {plan} {out} {size} {ppp} are filled in")
    a.add_argument("--manifest", required=True)
    a.add_argument("--app-json", help="a JSON file whose memory_mb is the budget (a toy's toy.json)")
    a.add_argument("--budget-mb", type=float)
    a.add_argument("--plan", help="the launch plan (default: two seconds, a frame at the end)")
    a.add_argument("--size", default="1170x2532", help="pixels (default the iPhone's 390x844 points at 3x)")
    a.add_argument("--ppp", default="3")
    a.add_argument("--ipa")
    a.add_argument("--ipa-data", default="data", help="the data directory inside the .app")
    a.add_argument("--json", help="write the numbers here")
    args = a.parse_args()
    results = []
    if args.ipa:
        results.append(bundle(Path(args.ipa), Path(args.manifest), args.ipa_data))
    else:
        if not args.check:
            a.error("--check is needed without --ipa")
        budget = args.budget_mb
        if budget is None and args.app_json:
            budget = json.loads(Path(args.app_json).read_text()).get("memory_mb")
        budget = budget or DEFAULT_BUDGET_MB
        with tempfile.TemporaryDirectory() as w:
            ok, dump, rss, mem = launch(args, Path(w))
            results.append(ok)
            results.append(shaders(dump))
            gpu = mem.get("gpu_peak_bytes", 0) / 2**20
            kit = mem.get("kit_bytes", 0) / 2**20
            results.append(say(rss <= budget, f"memory: peak RSS {rss:.0f} MB of a {budget:.0f} MB budget (GPU peak {gpu:.0f} MB, of it through the kit's helpers {kit:.0f} MB)"))
            if args.json:
                Path(args.json).write_text(json.dumps({"peak_rss_mb": round(rss), "gpu_peak_mb": round(gpu), "kit_mb": round(kit), "budget_mb": budget, "ok": all(results)}) + "\n")
    print("launch check: " + ("passes" if all(results) else "FAILS"))
    sys.exit(0 if all(results) else 1)


if __name__ == "__main__":
    main()
