// The C ABI the one frame core exports (toy-kit's frame.rs, through `ios_shell!`: toys/crates/toy-kit/src/ios.rs).
// The pixels, the fingers, the senses and the sound go straight through; everything around the toy is data
// (toy-kit's chrome.rs, as JSON: toy_chrome, toy_events out, toy_send in).
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
typedef struct ToyShell ToyShell;
typedef struct ToySynth ToySynth;
void toy_set_log(const char *path);
ToyShell *toy_new(void *layer, uint32_t width, uint32_t height, double scale, const char *assets);
void toy_resize(ToyShell *shell, uint32_t width, uint32_t height, double scale);
void toy_frame(ToyShell *shell, double t);
void toy_pointer(ToyShell *shell, uint64_t id, uint32_t phase, double x, double y, double t);
void toy_shaken(ToyShell *shell, double t);
void toy_motion(ToyShell *shell, double x, double y, double z, double t);
void toy_sense(ToyShell *shell, double gx, double gy, double gz, double bx, double by, double bz, double heading);
void toy_set_night(ToyShell *shell, bool on);
void toy_set_reduce_motion(ToyShell *shell, bool on);
void toy_store(ToyShell *shell, const char *path);
void toy_pause(ToyShell *shell);
ToySynth *toy_synth(ToyShell *shell);
void toy_audio(ToySynth *synth, float *out, uint32_t frames, uint32_t channels, double rate);
void toy_synth_free(ToySynth *synth);
void toy_free(ToyShell *shell);
// The chrome (chrome.rs's Chrome) as JSON when it changed since the last call, NULL while the same; what the toy
// asks of the frame (a JSON list of chrome.rs's Out: {"event":"haptic","haptic":"tick"}, {"event":"file"}),
// NULL when nothing; each valid until the next call of the same kind. And what the reader did in the frame's
// controls (JSON: chrome.rs's In, e.g. "rules", {"switch":{"i":0,"on":false}}, {"back":{"t":12.5}}).
const char *toy_chrome(ToyShell *shell);
const char *toy_events(ToyShell *shell);
void toy_send(ToyShell *shell, const char *json);
// A sound file of the reader's own (Toy::wants_file, Toy::opened: Screw's song), decoded and handed over
// (left/right `frames` floats at `rate` Hz; right NULL for mono; copied).
void toy_opened_audio(ToyShell *shell, const char *name, const float *left, const float *right, uint64_t frames, double rate);
