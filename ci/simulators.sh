#!/usr/bin/env bash
# Prints one "<kind> <udid> <model> <runtime>" line per simulator wanted.
#   ci/simulators.sh [iphone|ipad ...]   (default: iphone)
# Picks the newest iOS runtime that has an iPhone Pro and an iPad Pro, the newest iPhone Pro device type (and the largest iPad Pro,
# should one be asked for), and uses the runner image's device of that type if it has one, else
# creates a TerraIncognita-<kind>.
set -euo pipefail
python3 - "$@" <<'PY'
import json, subprocess, re, sys
wanted = sys.argv[1:] or ["iphone"]
def sim(*a): return subprocess.run(["xcrun", "simctl", *a], check=True, capture_output=True, text=True).stdout
runtimes = [r for r in json.loads(sim("list", "runtimes", "-j"))["runtimes"] if r["platform"] == "iOS" and r["isAvailable"]]
PHONE = [r"iPhone 17 Pro", r"iPhone 1\d Pro", r"iPhone .*Pro"]
PAD = [r"iPad Pro 13-inch \(M5\)", r"iPad Pro 13-inch \(M\d\)", r"iPad Pro 13-inch.*", r"iPad Pro.*"]
def pick(types, patterns):
    for p in patterns:
        c = [t for t in types if re.fullmatch(p, t["name"])]
        if c: return c[-1]
# the newest runtime that has both devices (an image may carry a newer runtime for one new device
# alone: Xcode 27.2's iOS 27.1 knew only "iPhone Duo")
seen = []
for runtime in sorted(runtimes, key=lambda r: [int(x) for x in r["version"].split(".")], reverse=True):
    types = runtime.get("supportedDeviceTypes") or json.loads(sim("list", "devicetypes", "-j"))["devicetypes"]
    phone, pad = pick(types, PHONE), pick(types, PAD)
    if phone and pad: break
    seen.append("%s: %s" % (runtime["version"], [t["name"] for t in types]))
else:
    raise SystemExit("no runtime with an iPhone Pro and an iPad Pro: %s" % seen)
devices = json.loads(sim("list", "devices", "available", "-j"))["devices"].get(runtime["identifier"], [])
for kind, t in (("ipad", pad), ("iphone", phone)):          # (the iPad first: the primary device)
    if kind not in wanted: continue
    # Reuse a device the runner image already created (its data container exists, so the
    # first boot is much faster than for a brand-new device).
    existing = [d for d in devices if d["deviceTypeIdentifier"] == t["identifier"]]
    if existing:
        print(kind, existing[0]["udid"], t["name"].replace(" ", "_"), runtime["version"])
        continue
    name = "TerraIncognita-" + kind
    udid = sim("create", name, t["identifier"], runtime["identifier"]).strip()
    print(kind, udid, t["name"].replace(" ", "_"), runtime["version"])
PY
