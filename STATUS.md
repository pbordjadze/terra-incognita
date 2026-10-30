# QA explore: agent, debug build, on iPhone_17_Pro 26.5 F0C70C77-E423-4478-B648-3D25045B311B

- commit: adec54af7f29dadbb7add402c1e133e6f913322d
- launch arguments: -prototype hydaspes
- run: https://github.com/pbordjadze/terra-incognita/actions/runs/40990401010
- steps: build=success prepare=success probe=success relay=skipped explore=success
- Maestro 2.11.0, Claude Code 2.1.301, claude-opus-5-5, 8 min, 130 turns, $6
- the agent ran its full 8 minutes and was stopped by the clock

- attempt 1: server maestro 1.0.0, protocol 2025-06-18; tools: list_devices, take_screenshot, run, inspect_screen, cheat_sheet, open_maestro_viewer, list_cloud_devices, run_on_cloud, get_cloud_run_status, describe_cloud_run
- list_devices: ok in 29.1 s; no image; {"devices":[{"device_id":"F0C70C77-E423-4478-B648-3D25045B311B","name":"iPhone 17 Pro - iOS 26.5 - F0C70C77-E423-4478-B648-3D25045B311B","platform":"ios","type"
- take_screenshot: ok in 112.6 s; 1-2-take_screenshot-1.jpg 919x2000 112 KiB; 
- inspect_screen: ok in 32.4 s; no image; {"ui_schema":{"platform":"ios","abbreviations":{"b":"bounds","txt":"text","rid":"resource-id","a11y":"accessibilityText","hint":"hintText","val":"value","c":"ch
- PASS: the simulator is listed
- PASS: a screenshot came back as an image
- PASS: the screen's elements came back
- passed in 224 s

- end: error_during_execution, 44 turns, 7.8 min, estimated $0.7149072
- tool calls: {"mcp__maestro__take_screenshot": 15, "mcp__maestro__inspect_screen": 10, "mcp__maestro__run": 14, "Read": 2, "Write": 1}
- images the agent received: 14

The agent's report: session/report.md
