"""Static bundle and privacy checks for the offline atlas leaf (no browser dependency)."""
import json
import re
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
leaf=ROOT/'Leaf'
expected={'index.html','leaf.js','leaf.css','notes.js','SOURCES.md','data/chart.js','data/day.png','data/night.png','fonts/LibertinusSerif-Regular.otf','fonts/LibertinusSerif-Italic.otf','fonts/LibertinusSerif-Semibold.otf','fonts/OFL-Libertinus.txt'}
actual={str(p.relative_to(leaf)) for p in leaf.rglob('*') if p.is_file()}
assert actual==expected, f'Unexpected/missing Leaf resources: {actual^expected}'
html=(leaf/'index.html').read_text()
for path in re.findall(r'(?:src|href)="([^"]+)"',html):
    assert not path.startswith(('http:','https:','//')) and (leaf/path).is_file(),path
for path in re.findall(r'url\(([^)]+)\)',(leaf/'leaf.css').read_text()):
    assert (leaf/path).is_file(),path
assert "connect-src 'none'" in html
js=(leaf/'leaf.js').read_text()
for forbidden in ['fetch(', 'XMLHttpRequest', 'sendBeacon','localStorage','sessionStorage','document.cookie','indexedDB','WebSocket']:
    assert forbidden not in js,forbidden
raw=(leaf/'data/chart.js').read_text()
data=json.loads(raw.removeprefix('window.LeafChart=').strip().removesuffix(';'))
assert data['projection']['lon0']==24.6
assert len(data['polygons'])>100 and len(data['soundings'])>50
import base64
for k in ['land','depth']:
    assert len(base64.b64decode(data['raster'][k]))==data['raster']['width']*data['raster']['height']
project=(ROOT/'App/TerraIncognita.xcodeproj/project.pbxproj').read_text()
assert 'Leaf in Resources' in project and 'path = ../Leaf;' in project
assert '.nonPersistent()' in (ROOT/'App/TerraIncognita/Leaf/LeafScreen.swift').read_text()
print(f"Leaf: {len(actual)} bundled files, {sum(p.stat().st_size for p in leaf.rglob('*') if p.is_file())/1e6:.1f} MB, no runtime data fetching or persistence")
