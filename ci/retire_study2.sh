#!/usr/bin/env bash
# One-time retirement after the integrated main app and its public source have been published.
# Authentication comes from the runner's normal GH_TOKEN; no credential is printed or embedded.
set -euo pipefail
python3 - "${BASH_SOURCE[0]}" <<'PY'
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time

root = Path(sys.argv[1]).resolve().parent.parent
out = root / 'out'
out.mkdir(parents=True, exist_ok=True)
evidence_path = out / 'study2-retirement.json'
evidence = {'status': 'not_started', 'source_replaced': False, 'source_replacement_attempts': 0,
            'release_metadata_updated': False}

def run(*arguments, timeout=60):
    result = subprocess.run(arguments, cwd=root, capture_output=True, timeout=timeout)
    if result.returncode:
        detail = result.stderr.decode('utf-8', 'replace').strip()[:2000]
        raise RuntimeError(f'{arguments[0]} failed: {detail}')
    return result.stdout

def api(path):
    return json.loads(run('gh', 'api', path))

def asset_named(release, name):
    matches = [asset for asset in release['assets'] if asset['name'] == name]
    if len(matches) != 1 or matches[0].get('state') != 'uploaded':
        raise RuntimeError('Expected one uploaded release asset: ' + name)
    return matches[0]

def download_source(repository, tag):
    return run('gh', 'release', 'download', tag, '--repo', repository,
               '--pattern', 'study2-source.json', '--output', '-')

try:
    repository = os.environ['GITHUB_REPOSITORY']
    build = os.environ['BUILD']
    if repository != 'pbordjadze/terra-incognita' or not re.fullmatch(r'[1-9][0-9]*', build):
        raise RuntimeError('Retirement requires the known repository and a numeric BUILD')
    if os.environ.get('GITHUB_REF_NAME') != 'main':
        raise RuntimeError('Retirement is permitted only in the main publication job')
    if '[retire-study2]' not in run('git', 'log', '-1', '--format=%B').decode('utf-8'):
        raise RuntimeError('The publication commit must explicitly request [retire-study2]')

    tag = 'study2-40440112973'
    old_identifier = 'com.pbordjadze.terraincognita.study2.source'
    old_bundle = 'com.pbordjadze.terraincognita.study2'
    old_source_url = f'https://github.com/{repository}/releases/download/{tag}/study2-source.json'
    main_source_url = f'https://raw.githubusercontent.com/{repository}/sidestore/source.json'
    main_tag = 'build-' + build
    version = '1.0.' + build
    expected_download = f'https://github.com/{repository}/releases/download/{main_tag}/TerraIncognita.ipa'
    draft_path = root / 'ci' / 'retired-study2-source.json'
    tombstone_bytes = draft_path.read_bytes()
    tombstone = json.loads(tombstone_bytes)
    if tombstone.get('identifier') != old_identifier or tombstone.get('apps') != []:
        raise RuntimeError('The retirement source must retain the original identifier and list no apps')
    if tombstone.get('sourceURL') != old_source_url or not tombstone.get('name'):
        raise RuntimeError('The retirement source must retain its known URL and a source name')
    news = tombstone.get('news')
    if not isinstance(news, list) or not news:
        raise RuntimeError('The retirement source must explain how to reach the main app')
    for item in news:
        if not all(isinstance(item.get(key), str) and item[key] for key in ['title', 'identifier', 'caption', 'date']):
            raise RuntimeError('The retirement news is incomplete')
        if item.get('notify') is not False or 'appID' in item:
            raise RuntimeError('Retirement news must not notify or reference a removed app listing')
    if not any(main_source_url in item['caption'] for item in news):
        raise RuntimeError('The retirement news must point to the live main source')

    main_release = api(f'repos/{repository}/releases/tags/{main_tag}')
    main_asset = asset_named(main_release, 'TerraIncognita.ipa')
    if main_asset['browser_download_url'] != expected_download or main_asset['size'] <= 0:
        raise RuntimeError('The current main device release is not published at its expected URL')
    if main_release.get('draft') or main_release.get('tag_name') != main_tag:
        raise RuntimeError('The current main release is not public')
    if os.environ.get('GITHUB_SHA') and main_release['target_commitish'] != os.environ['GITHUB_SHA']:
        raise RuntimeError('The current main release does not target this publication commit')

    # A bounded public read allows the raw source CDN to catch up with the just-pushed source.
    deadline = time.monotonic() + 180
    while True:
        source_bytes = run('curl', '--fail', '--silent', '--show-error', '--location',
                           '--max-time', '30', main_source_url + '?retirement-check=' + build)
        published = json.loads(source_bytes)
        apps = published.get('apps', [])
        app = apps[0] if len(apps) == 1 else {}
        versions = app.get('versions', [])
        latest = versions[0] if versions else {}
        if (published.get('identifier') == 'com.pbordjadze.terraincognita.source'
            and app.get('bundleIdentifier') == 'com.pbordjadze.terraincognita'
            and latest.get('version') == version and latest.get('buildVersion') == build
            and latest.get('downloadURL') == expected_download and latest.get('size') == main_asset['size']
            and app.get('version') == version and app.get('downloadURL') == expected_download
            and app.get('size') == main_asset['size']):
            break
        if time.monotonic() >= deadline:
            raise RuntimeError('The published main source did not confirm this exact device build within 180 seconds')
        print('Waiting for the published main source to confirm build ' + build, flush=True)
        time.sleep(5)

    retired_release = api(f'repos/{repository}/releases/tags/{tag}')
    if retired_release['id'] != 424998388:
        raise RuntimeError('The Study II release identity differs from the known historical release')
    preserved = {name: asset_named(retired_release, name) for name in ['TerraIncognita-StudyII.ipa', 'icon.png']}
    old_asset = asset_named(retired_release, 'study2-source.json')
    original_bytes = download_source(repository, tag)
    original = json.loads(original_bytes)
    if original.get('identifier') != old_identifier or not isinstance(original.get('apps'), list):
        raise RuntimeError('The existing Study II source has an unexpected identity or app list')
    if len(original['apps']) > 1 or any(app.get('bundleIdentifier') != old_bundle for app in original['apps']):
        raise RuntimeError('The existing source contains an unexpected app; refusing to replace it')
    if original_bytes != tombstone_bytes:
        if len(original['apps']) != 1:
            raise RuntimeError('An empty existing source must already match the exact intended tombstone')
        historical = original['apps'][0]
        historical_versions = historical.get('versions', [])
        historical_latest = historical_versions[0] if len(historical_versions) == 1 else {}
        historical_download = f'https://github.com/{repository}/releases/download/{tag}/TerraIncognita-StudyII.ipa'
        if (historical.get('version') != '1.0.68' or historical.get('downloadURL') != historical_download
            or historical.get('size') != preserved['TerraIncognita-StudyII.ipa']['size']
            or historical_latest.get('version') != '1.0.68' or historical_latest.get('buildVersion') != '68'
            or historical_latest.get('downloadURL') != historical_download
            or historical_latest.get('size') != preserved['TerraIncognita-StudyII.ipa']['size']):
            raise RuntimeError('The historical Study II source does not match its known build and preserved IPA')

    evidence.update({
        'status': 'main_publication_verified', 'repository': repository, 'build': build,
        'main_release': main_tag, 'main_source': main_source_url, 'main_download': expected_download,
        'main_ipa_size': main_asset['size'], 'main_source_sha256': hashlib.sha256(source_bytes).hexdigest(),
        'retired_release_id': retired_release['id'], 'retired_tag': tag,
        'original_title': retired_release['name'], 'original_body': retired_release['body'],
        'original_source_sha256': hashlib.sha256(original_bytes).hexdigest(),
        'source_asset_id_before': old_asset['id'],
        'preserved_asset_ids': {name: asset['id'] for name, asset in preserved.items()}
    })
    evidence_path.write_text(json.dumps(evidence, indent=2) + '\n')

    title = 'Retired — Terra Incognita II (Study II, build 68)'
    notes = (
        'This separate test app is retired. Its Water study is included in the main Terra Incognita app, '
        'alongside the Camel and the Llama, the Whale and the Desert, the Ocean encounter, the Aegean leaf, '
        'the globe and the illustrated films. Open **Demos** in the main app to choose a demo.\n\n'
        'Install or update **Terra Incognita** using its existing SideStore source:\n\n'
        + main_source_url + '\n\n'
        'The old IPA, icon and release are retained as a historical archive. The separate Study II source '
        'no longer lists an app. An already-installed Terra Incognita II is unchanged; it can be removed '
        'after switching to the main app.\n'
    )
    with tempfile.TemporaryDirectory(prefix='study2-retirement-') as temporary:
        temporary = Path(temporary)
        upload = temporary / 'study2-source.json'
        upload.write_bytes(tombstone_bytes)
        if original_bytes != tombstone_bytes:
            # --clobber deletes then uploads. Retry only this already-validated exact target;
            # a missing asset on entry still requires explicit operator review and recovery.
            for attempt in range(1, 4):
                evidence['source_replacement_attempts'] = attempt
                try:
                    run('gh', 'release', 'upload', tag, str(upload), '--repo', repository, '--clobber')
                except (RuntimeError, subprocess.TimeoutExpired):
                    if attempt == 3:
                        raise
                    print('Study II source upload failed; retrying the same verified asset.', flush=True)
                    time.sleep(2)
                else:
                    evidence['source_replaced'] = True
                    break
        notes_path = temporary / 'retired-release-notes.txt'
        notes_path.write_text(notes)
        if retired_release['name'] != title or retired_release['body'] != notes:
            run('gh', 'release', 'edit', tag, '--repo', repository, '--title', title, '--notes-file', str(notes_path))
            evidence['release_metadata_updated'] = True
        downloaded = temporary / 'downloaded-source.json'
        downloaded.write_bytes(download_source(repository, tag))
        run('cmp', str(draft_path), str(downloaded))

    verified = api(f'repos/{repository}/releases/tags/{tag}')
    if verified['id'] != retired_release['id'] or verified['target_commitish'] != retired_release['target_commitish']:
        raise RuntimeError('The historical release identity or target changed')
    if verified['name'] != title or verified['body'] != notes:
        raise RuntimeError('The retirement release title or notes did not match')
    for name, asset in preserved.items():
        current = asset_named(verified, name)
        if any(current[key] != asset[key] for key in ['id', 'size', 'browser_download_url']):
            raise RuntimeError('A historical asset was unexpectedly changed: ' + name)
    evidence.update({
        'status': 'retired', 'source_identifier': old_identifier, 'listed_apps': 0,
        'source_asset_id_after': asset_named(verified, 'study2-source.json')['id'],
        'retired_source_url': old_source_url, 'tombstone_sha256': hashlib.sha256(tombstone_bytes).hexdigest(),
        'download_cmp': 'passed', 'historical_ipa_and_icon': 'unchanged'
    })
    print('Study II source retired after verifying main build ' + build + '; historical IPA and icon preserved.', flush=True)
except Exception as error:
    evidence['status'] = 'failed'
    evidence['error'] = str(error)
    raise
finally:
    evidence['observed_at'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    evidence_path.write_text(json.dumps(evidence, indent=2) + '\n')
PY
