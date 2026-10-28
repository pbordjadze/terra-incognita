# QA explore: agent, release build, on iPhone_17_Pro 26.5 E839A11B-51B4-4AD3-ACF4-A53B95B7C957

- commit: 1d119b3be84b0603c2f122edd2f38281a906f7b1
- run: https://github.com/pbordjadze/terra-incognita/actions/runs/40631260056
- steps: build=success prepare=success probe=success relay=skipped explore=success
- Maestro 2.11.0, Claude Code 2.1.301, claude-opus-5-5, 15 min, 120 turns, $5
- the agent ran its full 15 minutes and was stopped by the clock

- attempt 1: server maestro 1.0.0, protocol 2025-06-18; tools: list_devices, take_screenshot, run, inspect_screen, cheat_sheet, open_maestro_viewer, list_cloud_devices, run_on_cloud, get_cloud_run_status, describe_cloud_run
- list_devices: ok in 28.8 s; no image; {"devices":[{"device_id":"E839A11B-51B4-4AD3-ACF4-A53B95B7C957","name":"iPhone 17 Pro - iOS 26.5 - E839A11B-51B4-4AD3-ACF4-A53B95B7C957","platform":"ios","type"
- take_screenshot: ok in 86.1 s; 1-2-take_screenshot-1.jpg 919x2000 248 KiB; 
- inspect_screen: ok in 7.5 s; no image; {"ui_schema":{"platform":"ios","abbreviations":{"b":"bounds","txt":"text","rid":"resource-id","a11y":"accessibilityText","hint":"hintText","val":"value","c":"ch
- PASS: the simulator is listed
- PASS: a screenshot came back as an image
- PASS: the screen's elements came back
- passed in 145 s

- end: error_during_execution, 95 turns, 14.8 min, estimated $1.8325716000000003
- tool calls: {"mcp__maestro__take_screenshot": 29, "Read": 10, "mcp__maestro__run": 35, "mcp__maestro__inspect_screen": 12, "Write": 2}
- images the agent received: 34

The agent's report: session/report.md
