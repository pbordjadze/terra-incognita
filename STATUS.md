# QA explore: agent, debug build, on iPhone_17_Pro 26.5 77AA4492-C787-4EEE-A772-05D4B3E9ECCD

- commit: 1677e4bc3f89cd5ca2f2041d7b94ec6b2eef64b1
- launch arguments: -prototype vine
- run: https://github.com/pbordjadze/terra-incognita/actions/runs/40988990968
- steps: build=success prepare=success probe=success relay=skipped explore=success
- Maestro 2.11.0, Claude Code 2.1.301, claude-opus-5-5, 8 min, 120 turns, $5
- the agent ran its full 8 minutes and was stopped by the clock

- attempt 1: server maestro 1.0.0, protocol 2025-06-18; tools: list_devices, take_screenshot, run, inspect_screen, cheat_sheet, open_maestro_viewer, list_cloud_devices, run_on_cloud, get_cloud_run_status, describe_cloud_run
- list_devices: ok in 46.4 s; no image; {"devices":[{"device_id":"77AA4492-C787-4EEE-A772-05D4B3E9ECCD","name":"iPhone 17 Pro - iOS 26.5 - 77AA4492-C787-4EEE-A772-05D4B3E9ECCD","platform":"ios","type"
- take_screenshot: ok in 154.8 s; 1-2-take_screenshot-1.jpg 919x2000 270 KiB; 
- inspect_screen: ok in 4.6 s; no image; {"ui_schema":{"platform":"ios","abbreviations":{"b":"bounds","txt":"text","rid":"resource-id","a11y":"accessibilityText","hint":"hintText","val":"value","c":"ch
- PASS: the simulator is listed
- PASS: a screenshot came back as an image
- PASS: the screen's elements came back
- passed in 250 s

- end: error_during_execution, 20 turns, 7.4 min, estimated $0.3711436
- tool calls: {"mcp__maestro__take_screenshot": 4, "mcp__maestro__inspect_screen": 1, "Read": 5, "mcp__maestro__run": 5}
- images the agent received: 7

No report.md from an agent.
