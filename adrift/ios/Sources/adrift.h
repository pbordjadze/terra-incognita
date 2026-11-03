// The C ABI of adrift-core (crates/adrift-core/src/ffi.rs).
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
typedef struct AdriftShell AdriftShell;
AdriftShell *adrift_new(void *layer, uint32_t width, uint32_t height, double scale, const char *assets);
void adrift_resize(AdriftShell *shell, uint32_t width, uint32_t height, double scale);
void adrift_frame(AdriftShell *shell, double t);
void adrift_pointer(AdriftShell *shell, uint64_t id, uint32_t phase, double x, double y, double t);
void adrift_shaken(AdriftShell *shell);
bool adrift_take_haptic(AdriftShell *shell);
void adrift_store(AdriftShell *shell, const char *path);
void adrift_pause(AdriftShell *shell);
void adrift_set_night(AdriftShell *shell, bool on);
void adrift_set_reduce_motion(AdriftShell *shell, bool on);
bool adrift_take_export(AdriftShell *shell);
uint32_t adrift_export_begin(AdriftShell *shell, double fps, uint32_t *width, uint32_t *height);
bool adrift_export_frame(AdriftShell *shell, uint8_t *out, size_t stride);
void adrift_export_end(AdriftShell *shell);
uint32_t adrift_panel_count(AdriftShell *shell);
bool adrift_panel_item(AdriftShell *shell, uint32_t i, float *rect, char *words, size_t len);
void adrift_set_song(AdriftShell *shell, const char *name);
bool adrift_recording(AdriftShell *shell);
bool adrift_export_song(AdriftShell *shell, double *start, double *length);
bool adrift_menu(AdriftShell *shell, uint32_t *picked, uint32_t *takes, uint32_t *chosen, uint32_t *kept);
uint32_t adrift_keep(AdriftShell *shell, bool on);
uint32_t adrift_medium(AdriftShell *shell);
void adrift_set_medium(AdriftShell *shell, uint32_t medium);
uint32_t adrift_journey_begin(AdriftShell *shell, double seconds, double fps, uint32_t *width, uint32_t *height);
bool adrift_journey_bounds(AdriftShell *shell, double seconds, double *born, double *now, double *from, double *to);
bool adrift_preview_begin(AdriftShell *shell);
void adrift_preview_at(AdriftShell *shell, double day);
void adrift_preview_play(AdriftShell *shell, double from, double to, double pace);
void adrift_preview_end(AdriftShell *shell);
double adrift_day(AdriftShell *shell);
uint32_t adrift_journey_begin_edit(AdriftShell *shell, double seconds, double from, double to, double pace, double fps, uint32_t *width, uint32_t *height);
size_t adrift_lamps_text(AdriftShell *shell, uint8_t *out, size_t len);
uint32_t adrift_lamps_paste(AdriftShell *shell, const char *text);
void adrift_free(AdriftShell *shell);
