#!/usr/bin/env bash
# Plays one of Battle Ink's plans cut short, for looking while tuning: only the named moments, no film.
#
#     bash tools/toys/battle/quick.sh <plan name> <out dir> <seconds> <t1,t2,...> [--size WxH --ppp n]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
NAME="$1"; OUT="$2"; SECS="$3"; SHOTS="$4"; shift 4
mkdir -p "$OUT"
python3 - "$ROOT/toys/battle/plans/$NAME.json" "$OUT/plan.json" "$SECS" "$SHOTS" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
p['seconds'] = float(sys.argv[3])
p.pop('film', None)
p['shots'] = [{'t': float(t), 'name': f'{float(t):05.1f}'} for t in sys.argv[4].split(',')]
json.dump(p, open(sys.argv[2], 'w'))
PY
python3 "$ROOT/tools/toys/assets.py" battle --link > /dev/null
cd "$ROOT/toys"
cargo run -q --release -p battle --bin battle-check -- play "$OUT/plan.json" "$OUT" "$@"
