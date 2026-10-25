#!/usr/bin/env python3
"""Writes the SideStore/AltStore source (JSON) listing the latest IPA builds.

    ci/sidestore_source.py <TerraIncognita.app> <icon-url>  > source.json

Versions come from this repository's `build-<n>` releases that carry an .ipa asset (newest
first). Needs `gh` with GH_TOKEN and GITHUB_REPOSITORY.
"""
import json
import os
import plistlib
import subprocess
import sys

# Builds listed; the ipa job in .github/workflows/ci.yml keeps this many releases (its `tail -n +6`).
KEEP = 5

app_path, icon_url = sys.argv[1], sys.argv[2]
repo = os.environ["GITHUB_REPOSITORY"]
info = plistlib.load(open(os.path.join(app_path, "Info.plist"), "rb"))

releases = json.loads(subprocess.run(
    ["gh", "api", f"repos/{repo}/releases?per_page=30"], check=True, capture_output=True, text=True).stdout)
versions = []
for r in releases:
    tag = r["tag_name"]
    ipa = next((a for a in r["assets"] if a["name"].endswith(".ipa")), None)
    if not tag.startswith("build-") or ipa is None:
        continue
    build = tag.removeprefix("build-")
    versions.append({
        "version": f"1.0.{build}",
        "buildVersion": build,
        "date": r["published_at"] or r["created_at"],
        "localizedDescription": (r["body"] or "").strip() or f"Build {build}",
        "downloadURL": ipa["browser_download_url"],
        "size": ipa["size"],
        "minOSVersion": info.get("MinimumOSVersion", "26.0"),
    })
versions.sort(key=lambda v: int(v["buildVersion"]), reverse=True)
versions = versions[:KEEP]
if not versions:
    sys.exit("no build-* release with an .ipa asset")
latest = versions[0]

description = (
    "An atlas drawn in letters: the world as a medieval map, every coast, wood, wave and sea "
    "monster made of characters. Open it and a six-minute tour plays; touch the map to wander.")
app = {
    "name": info.get("CFBundleDisplayName", "Terra Incognita"),
    "bundleIdentifier": info["CFBundleIdentifier"],
    "developerName": repo.split("/")[0],
    "subtitle": "An atlas drawn in letters",
    "localizedDescription": description,
    "iconURL": icon_url,
    "tintColor": "#C9A243",
    "category": "education",
    "versions": versions,
    # Legacy fields for clients that predate `versions`.
    "version": latest["version"],
    "versionDate": latest["date"],
    "versionDescription": latest["localizedDescription"],
    "downloadURL": latest["downloadURL"],
    "size": latest["size"],
    "appPermissions": {"entitlements": [], "privacy": {}},
}
source = {
    "name": "Terra Incognita (CI builds)",
    "identifier": f"{info['CFBundleIdentifier']}.source",
    "subtitle": "Automatic builds",
    "iconURL": icon_url,
    "website": f"https://github.com/{repo}",
    "tintColor": "#C9A243",
    "apps": [app],
    "news": [],
}
json.dump(source, sys.stdout, indent=2)
print()
