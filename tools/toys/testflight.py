#!/usr/bin/env python3
"""Terra Incognita's side of App Store Connect, through its API (painting-company row 312).

    ASC_KEY_ID=… ASC_ISSUER_ID=… ASC_KEY_P8=… python3 tools/toys/testflight.py register
    ASC_KEY_ID=… ASC_ISSUER_ID=… ASC_KEY_P8=… [BUILD=7.1] [WAIT_MINUTES=50] python3 tools/toys/testflight.py testers

`register` makes the App ID com.pbordjadze.terraincognita in the team's identifiers if it isn't there
(so App Store Connect's New App offers it), and says whether the app record exists yet.
`testers` puts a build in front of the internal testers, no Beta App Review: it finds the app by its
bundle id and the build (BUILD, the CFBundleVersion, or the newest processed; with WAIT_MINUTES it waits
that long for App Store Connect to process it, and stops on a build it rejected), makes the internal
group "Team" the first time (every build reaches it by itself), puts every App Store Connect user of the
team in it who isn't (internal testers are the team's own users; their addresses are never printed, as
the repository's logs are public), and sets the build's What to Test. External testing (a Beta App
Review) is a separate step, later. Every step is idempotent. Run by `.github/workflows/testflight.yml`.

The client (`Client`, `APIError`, `private_key`, `token`, `find_app`, `find_build`, `summarize`) is Paint by
Moonlight's (pbordjadze/paint-by-number, ci/testflight_testers.py), copied: keep the two alike.
Needs `cryptography` (ES256 tokens).
"""

import base64
import json
import os
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import decode_dss_signature

API = "https://api.appstoreconnect.apple.com/v1"
BUNDLE_ID = "com.pbordjadze.terraincognita"
GROUP = "Team"
WHAT_TO_TEST = (
    "Terra Incognita: the whole Earth, with a living light for each toy where it is real. Touch a light to "
    "go into its toy; the Earth mark or a pinch out brings you back. Tell us what felt slow, confusing or "
    "broken, and what you loved."
)


class APIError(Exception):
    pass


def private_key():
    """The .p8 key, rebuilt from its base64 alone (a key pasted on a phone loses line breaks)."""
    body = re.sub(r"-----[A-Z ]+-----|\s", "", os.environ["ASC_KEY_P8"])
    pem = "-----BEGIN PRIVATE KEY-----\n" + "\n".join(body[i:i + 64] for i in range(0, len(body), 64))
    return serialization.load_pem_private_key((pem + "\n-----END PRIVATE KEY-----\n").encode(), None)


def b64(data):
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def token(key):
    """An ES256 JWT for the App Store Connect API, good for ten minutes."""
    now = int(time.time())
    header = {"alg": "ES256", "kid": os.environ["ASC_KEY_ID"], "typ": "JWT"}
    payload = {"iss": os.environ["ASC_ISSUER_ID"], "iat": now, "exp": now + 600, "aud": "appstoreconnect-v1"}
    signing = b64(json.dumps(header).encode()) + "." + b64(json.dumps(payload).encode())
    r, s = decode_dss_signature(key.sign(signing.encode(), ec.ECDSA(hashes.SHA256())))
    return signing + "." + b64(r.to_bytes(32, "big") + s.to_bytes(32, "big"))


class Client:
    def __init__(self, key):
        self.key = key

    def call(self, method, path, body=None, query=None):
        # A path naming its version (/v2/…) is used as it is; any other is a /v1 one.
        base = API.removesuffix("/v1") if path.startswith("/v2/") else API
        url = base + path + ("?" + urllib.parse.urlencode(query) if query else "")
        data = json.dumps(body).encode() if body is not None else None
        request = urllib.request.Request(url, data=data, method=method)
        request.add_header("Authorization", "Bearer " + token(self.key))
        if data is not None:
            request.add_header("Content-Type", "application/json")
        try:
            with urllib.request.urlopen(request, timeout=60) as response:
                raw = response.read()
                return json.loads(raw) if raw else {}
        except urllib.error.HTTPError as error:
            # Apple's errors say what is wrong and carry no secrets.
            try:
                errors = json.loads(error.read()).get("errors", [])
                detail = "; ".join(f"{e.get('title')}: {e.get('detail')}" for e in errors)
            except ValueError:
                detail = error.reason
            raise APIError(f"{method} {path}: {error.code} {detail}") from None


def find_app(client):
    """The app whose bundle id is BUNDLE_ID itself: the filter also matches longer ones."""
    for app in client.call("GET", "/apps", query={"filter[bundleId]": BUNDLE_ID})["data"]:
        if app["attributes"].get("bundleId") == BUNDLE_ID:
            return app
    raise APIError(f"no App Store Connect app has the bundle id {BUNDLE_ID}: make it (App Store Connect › Apps › + › New App, that bundle id)")


def find_build(client, app_id, wanted, wait_minutes=0):
    query = {"filter[app]": app_id, "sort": "-uploadedDate", "limit": "20",
             "fields[builds]": "version,processingState,uploadedDate,expired"}
    if wanted:
        query["filter[version]"] = wanted
    deadline = time.time() + wait_minutes * 60
    while True:
        builds = client.call("GET", "/builds", query=query)["data"]
        usable = [b for b in builds if b["attributes"]["processingState"] == "VALID" and not b["attributes"]["expired"]]
        if usable:
            return usable[0]
        states = ", ".join(f"{b['attributes']['version']}: {b['attributes']['processingState']}" for b in builds)
        # A freshly uploaded build shows up after a few minutes, PROCESSING, then VALID or rejected.
        rejected = any(b["attributes"]["processingState"] in ("FAILED", "INVALID") for b in builds)
        if rejected or time.time() >= deadline:
            raise APIError(f"no processed build{' ' + wanted if wanted else ''} ({states or 'none uploaded'})")
        print(f"Waiting for App Store Connect to process build {wanted or '(newest)'} ({states or 'not listed yet'})…", flush=True)
        time.sleep(60)


def summarize(lines):
    """Adds `lines` to the Actions run's summary page, where Paul reads the outcome."""
    path = os.environ.get("GITHUB_STEP_SUMMARY")
    if path:
        with open(path, "a") as file:
            file.write("\n".join(lines) + "\n")



def register(client):
    """The App ID, made if missing (App Store Connect's New App lists only registered ones)."""
    found = client.call("GET", "/bundleIds", query={"filter[identifier]": BUNDLE_ID, "limit": "200"})["data"]
    if any(b["attributes"].get("identifier") == BUNDLE_ID for b in found):
        print(f"The App ID {BUNDLE_ID} is registered.")
    else:
        client.call("POST", "/bundleIds", {"data": {"type": "bundleIds", "attributes": {
            "identifier": BUNDLE_ID, "name": "Terra Incognita", "platform": "IOS"}}})
        print(f"Registered the App ID {BUNDLE_ID}.")
    try:
        find_app(client)
        print("Its App Store Connect app record exists.")
        summarize([f"App ID {BUNDLE_ID} registered; the app record exists."])
        return True
    except APIError as error:
        print(f"::warning::{error}")
        summarize([f"App ID {BUNDLE_ID} registered; **no app record yet**: App Store Connect › Apps › + › New App."])
        return False


def ensure_internal_group(client, app_id):
    groups = client.call("GET", f"/apps/{app_id}/betaGroups", query={"limit": "200"})["data"]
    group = next((g for g in groups if g["attributes"].get("isInternalGroup") and g["attributes"]["name"] == GROUP), None)
    if group is None:
        body = {"data": {"type": "betaGroups", "attributes": {
            "name": GROUP, "isInternalGroup": True, "hasAccessToAllBuilds": True, "feedbackEnabled": True},
            "relationships": {"app": {"data": {"type": "apps", "id": app_id}}}}}
        group = client.call("POST", "/betaGroups", body)["data"]
        print(f"Made the internal group {GROUP!r}: every build reaches it once processed.")
    return group


def ensure_team_in_group(client, group):
    """Every App Store Connect user of the team, as a tester in the internal group. Returns how many are in."""
    users = client.call("GET", "/users", query={"limit": "200", "fields[users]": "username,firstName,lastName"})["data"]
    present = client.call("GET", f"/betaGroups/{group['id']}/betaTesters", query={"limit": "200", "fields[betaTesters]": "email"})["data"]
    emails = {(t["attributes"].get("email") or "").lower() for t in present}
    added = failed = 0
    for user in users:
        a = user["attributes"]
        email = (a.get("username") or "").lower()
        if not email or email in emails:
            continue
        try:
            client.call("POST", "/betaTesters", {"data": {"type": "betaTesters", "attributes": {
                "email": a["username"], "firstName": a.get("firstName") or "", "lastName": a.get("lastName") or ""},
                "relationships": {"betaGroups": {"data": [{"type": "betaGroups", "id": group["id"]}]}}}})
            added += 1
        except APIError as error:
            # the address is not printed; Apple's message names no one
            failed += 1
            print(f"::warning::A team member couldn't be added to {GROUP!r}: {re.sub(r'[^ ]+@[^ ]+', '<email>', str(error))}")
    print(f"{GROUP!r}: {len(emails) + added} tester(s), {added} added now" + (f", {failed} not added" if failed else "") + ".")
    return len(emails) + added, failed


def ensure_what_to_test(client, build_id, text):
    existing = client.call("GET", f"/builds/{build_id}/betaBuildLocalizations")["data"]
    english = next((l for l in existing if l["attributes"]["locale"] == "en-US"), None)
    if english is None:
        client.call("POST", "/betaBuildLocalizations", {"data": {
            "type": "betaBuildLocalizations", "attributes": {"locale": "en-US", "whatsNew": text},
            "relationships": {"build": {"data": {"type": "builds", "id": build_id}}}}})
    elif english["attributes"].get("whatsNew") != text:
        client.call("PATCH", f"/betaBuildLocalizations/{english['id']}", {"data": {
            "type": "betaBuildLocalizations", "id": english["id"], "attributes": {"whatsNew": text}}})


def testers(client):
    app_id = find_app(client)["id"]
    wait = int(os.environ.get("WAIT_MINUTES", "0") or 0)
    build = find_build(client, app_id, os.environ.get("BUILD", "").strip(), wait)
    version = build["attributes"]["version"]
    print(f"Build {version} ({build['attributes']['processingState']}).")
    group = ensure_internal_group(client, app_id)
    # an internal group made by hand may not take every build by itself: add this one (a no-op if it has it)
    if not group["attributes"].get("hasAccessToAllBuilds"):
        client.call("POST", f"/betaGroups/{group['id']}/relationships/builds", {"data": [{"type": "builds", "id": build["id"]}]})
    count, failed = ensure_team_in_group(client, group)
    ensure_what_to_test(client, build["id"], os.environ.get("WHAT_TO_TEST", "").strip() or WHAT_TO_TEST)
    print("What to Test is set.")
    summarize([f"### Build {version} is in TestFlight's internal group {GROUP}",
               f"{count} tester(s): open TestFlight on a device signed in with your Apple ID to install it."
               + (f" {failed} team member(s) couldn't be added: App Store Connect › TestFlight › {GROUP} › +." if failed else "")])


def main():
    for name in ("ASC_KEY_ID", "ASC_ISSUER_ID", "ASC_KEY_P8"):
        if not os.environ.get(name):
            sys.exit(f"{name} is not set")
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    client = Client(private_key())
    if command == "register":
        register(client)
    elif command == "testers":
        testers(client)
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    try:
        main()
    except APIError as error:
        sys.exit(f"App Store Connect refused: {error}")
