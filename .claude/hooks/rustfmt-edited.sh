#!/usr/bin/env bash
# Claude Code's PostToolUse hook (.claude/settings.json): formats a Rust file as soon as an agent writes it,
# so rustfmt's verdict never waits for the commit. Reads the hook's JSON on stdin; quiet, and never blocks.
path=$(python3 -c 'import json, sys; print(json.load(sys.stdin).get("tool_input", {}).get("file_path", ""))' 2> /dev/null)
case "$path" in
  *.rs) [ -f "$path" ] && command -v rustfmt > /dev/null && rustfmt --edition 2021 "$path" 2> /dev/null ;;
esac
exit 0
