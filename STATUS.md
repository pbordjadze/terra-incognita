# QA explore: agent, release build, on iPhone_17_Pro 26.5 77AA4492-C787-4EEE-A772-05D4B3E9ECCD

- commit: 8635b95701f6447e5de0118659f88f0ef7c9ca40
- run: https://github.com/pbordjadze/terra-incognita/actions/runs/40665453332
- steps: build=success prepare=success probe=success relay=skipped explore=success
- Maestro 2.11.0, Claude Code 2.1.301, claude-opus-5-5, 15 min, 120 turns, $5
- the agent ran its full 15 minutes and was stopped by the clock

- attempt 1: server maestro 1.0.0, protocol 2025-06-18; tools: list_devices, take_screenshot, run, inspect_screen, cheat_sheet, open_maestro_viewer, list_cloud_devices, run_on_cloud, get_cloud_run_status, describe_cloud_run
- list_devices: ok in 40.4 s; no image; {"devices":[{"device_id":"77AA4492-C787-4EEE-A772-05D4B3E9ECCD","name":"iPhone 17 Pro - iOS 26.5 - 77AA4492-C787-4EEE-A772-05D4B3E9ECCD","platform":"ios","type"
- take_screenshot: ok in 145.1 s; 1-2-take_screenshot-1.jpg 919x2000 172 KiB; 
- inspect_screen: ok in 11.7 s; no image; {"ui_schema":{"platform":"ios","abbreviations":{"b":"bounds","txt":"text","rid":"resource-id","a11y":"accessibilityText","hint":"hintText","val":"value","c":"ch
- PASS: the simulator is listed
- PASS: a screenshot came back as an image
- PASS: the screen's elements came back
- passed in 235 s

- end: error_during_execution, 76 turns, 14.6 min, estimated $1.3015072
- tool calls: {"mcp__maestro__take_screenshot": 14, "mcp__maestro__inspect_screen": 22, "mcp__maestro__run": 25, "Read": 7, "Write": 1}
- images the agent received: 19

The agent's report: session/report.md
