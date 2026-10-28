# QA explore: agent, release build, on iPhone_17_Pro 26.5 E839A11B-51B4-4AD3-ACF4-A53B95B7C957

- commit: e497e35cefaa4f1c7a8cb2999ad8c2f8a1382476
- run: https://github.com/pbordjadze/terra-incognita/actions/runs/40649281864
- steps: build=success prepare=success probe=success relay=skipped explore=success
- Maestro 2.11.0, Claude Code 2.1.301, claude-opus-5-5, 15 min, 120 turns, $5
- the agent ran its full 15 minutes and was stopped by the clock

- attempt 1: server maestro 1.0.0, protocol 2025-06-18; tools: list_devices, take_screenshot, run, inspect_screen, cheat_sheet, open_maestro_viewer, list_cloud_devices, run_on_cloud, get_cloud_run_status, describe_cloud_run
- list_devices: ok in 69.1 s; no image; {"devices":[{"device_id":"E839A11B-51B4-4AD3-ACF4-A53B95B7C957","name":"iPhone 17 Pro - iOS 26.5 - E839A11B-51B4-4AD3-ACF4-A53B95B7C957","platform":"ios","type"
- take_screenshot: ok in 152.9 s; 1-2-take_screenshot-1.jpg 919x2000 186 KiB; 
- inspect_screen: ok in 39.7 s; no image; {"ui_schema":{"platform":"ios","abbreviations":{"b":"bounds","txt":"text","rid":"resource-id","a11y":"accessibilityText","hint":"hintText","val":"value","c":"ch
- PASS: the simulator is listed
- PASS: a screenshot came back as an image
- PASS: the screen's elements came back
- passed in 302 s

- end: error_during_execution, 59 turns, 14.7 min, estimated $1.1020513999999997
- tool calls: {"mcp__maestro__take_screenshot": 10, "mcp__maestro__inspect_screen": 13, "Read": 9, "mcp__maestro__run": 16, "Write": 2}
- images the agent received: 17

The agent's report: session/report.md
