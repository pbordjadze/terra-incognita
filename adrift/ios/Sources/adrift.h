// The C ABI of adrift-core (crates/adrift-core/src/ffi.rs).
#include <stdint.h>
#include <stdbool.h>
typedef struct AdriftShell AdriftShell;
AdriftShell *adrift_new(void *layer, uint32_t width, uint32_t height, double scale, const char *assets);
void adrift_resize(AdriftShell *shell, uint32_t width, uint32_t height, double scale);
void adrift_frame(AdriftShell *shell, double t);
void adrift_pointer(AdriftShell *shell, uint64_t id, uint32_t phase, double x, double y, double t);
void adrift_set_reduce_motion(AdriftShell *shell, bool on);
void adrift_free(AdriftShell *shell);
