// The C ABI every toy's static library exports (toy-kit's `ios_shell!`, toys/crates/toy-kit/src/lib.rs).
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
void toy_shaken(ToyShell *shell);
bool toy_take_haptic(ToyShell *shell);
void toy_set_night(ToyShell *shell, bool on);
void toy_set_reduce_motion(ToyShell *shell, bool on);
void toy_store(ToyShell *shell, const char *path);
void toy_pause(ToyShell *shell);
ToySynth *toy_synth(ToyShell *shell);
void toy_audio(ToySynth *synth, float *out, uint32_t frames, uint32_t channels, double rate);
void toy_synth_free(ToySynth *synth);
void toy_free(ToyShell *shell);
// The info mark (toy-kit's info.rs): its place in points (false: the toy has none), its line while its
// demo shows (NULL otherwise; valid until the next frame), and opening it (VoiceOver's activation).
bool toy_info_spot(ToyShell *shell, double *x, double *y);
const char *toy_info_line(ToyShell *shell);
void toy_info_open(ToyShell *shell, double t);
// The rules mark beside it (info.rs's Rules): its place in points (false: none), the rules while they
// show (lines, then a blank line and the note on what is measured and what is drawn; NULL otherwise;
// valid until the next frame), and showing or hiding them (VoiceOver's activation, the panel's tap).
bool toy_rules_spot(ToyShell *shell, double *x, double *y);
const char *toy_rules_text(ToyShell *shell);
void toy_rules_open(ToyShell *shell);
// The origins mark beside that (info.rs; Toy::origins): where the rules came from, in the same card.
bool toy_origins_spot(ToyShell *shell, double *x, double *y);
const char *toy_origins_text(ToyShell *shell);
void toy_origins_open(ToyShell *shell);
// Whether the card (the rules or the origins) hangs from the top, asked as it opens (Toy::card_at_top).
bool toy_card_at_top(ToyShell *shell);
// The toy's buttons for VoiceOver (toy-kit's Toy::buttons): a line each, its label, a tab, x, a tab, y in
// points (empty: none; valid until the next call), and pressing one by its line's index.
const char *toy_buttons(ToyShell *shell);
void toy_button_press(ToyShell *shell, uint32_t index, double t);
// A sound file of the reader's own (Toy::wants_file, Toy::opened: Screw's song): whether the toy asks for
// one now (the shell shows the Files picker), the file decoded and handed over (left/right `frames`
// floats at `rate` Hz; right NULL for mono; copied), and the toy's quiet title line (NULL: none; valid
// until the next call).
bool toy_wants_file(ToyShell *shell);
void toy_opened_audio(ToyShell *shell, const char *name, const float *left, const float *right, uint64_t frames, double rate);
const char *toy_title(ToyShell *shell);
// Whether the toy's sound is music the reader asked for (Toy::plays_music): the playback category.
bool toy_plays_music(ToyShell *shell);
