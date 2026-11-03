# toy-kit changes

One line a change that a toy or another repository may notice; a minor version's line says how to move.

- Unreleased: launch safety. `KIT_IPHONE_LIMITS=1` requests the device as an Apple7 iPhone would get
  it (limits from wgpu-hal 26's Metal backend, no feature a phone lacks); a GPU validation error is fatal
  in the headless check (still logged, not fatal, on the device); `KIT_MEMORY_REPORT=<file>` writes the
  GPU's peak bytes; `Gpu::texture` and `Gpu::buffer` count what they make (`budget::kit_bytes`); the
  device's log gains the GPU's limits and the memory available before and after the toy starts. A toy's
  own `device.create_*` still works and is counted only in the GPU's total. `tools/kit/launch_check.py`
  uses it all. No public signature changed.
