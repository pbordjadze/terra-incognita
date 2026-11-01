#!/usr/bin/env python3
"""Readies a web build (tools/adrift/web-build.sh) for a host that serves no binaries, such as a
claude.ai artifact: drifters.bin is written beside itself as drifters.bin.json ({"base64": ...}) and
assets/index.json points the web shell at it (adrift-web reads that index first). The .bin stays for
any static server.

    python3 tools/adrift/web-artifact.py adrift/build/web
"""
import base64, json, pathlib, sys

web = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "adrift/build/web")
a = web / "assets"
(a / "drifters.bin.json").write_text(json.dumps({"base64": base64.b64encode((a / "drifters.bin").read_bytes()).decode()}))
(a / "index.json").write_text(json.dumps({"drifters.bin": "assets/drifters.bin.json"}))
print("ready:", web, sum(1 for p in web.rglob("*") if p.is_file()), "files")
