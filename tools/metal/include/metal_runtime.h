// What a shader compiled by tools/metal/metal.py shares with the runtime that runs it
// (tools/metal/runtime.cpp): the bindings, the quad's exchange for derivatives, and the entry points
// metal.py generates for the one function a build runs.
#pragma once

struct __mtl_texture {
    int width, height, depth, levels;
    int format;                 // 0 RGBA floats; 1 RGBA bytes, an 8-bit unorm (r8Unorm, rgba8Unorm) as the GPU holds it
    void *level[16];            // x fastest, then y, then z; level i is (width >> i) x ... at least 1
};

struct __mtl_bindings {
    const void *buffer[31];
    __mtl_texture *texture[31];
};

// dfdx, dfdy and fwidth are differences across the 2x2 pixels run together; `line` says where, for
// the runtime's report of derivatives taken where the four had parted ways
extern "C" void __mtl_quad(const float *v, int n, int line, float *dx, float *dy);

extern "C" void __mtl_fragment(const __mtl_bindings *b, float x, float y, float *out);
extern "C" void __mtl_kernel(const __mtl_bindings *b, unsigned x, unsigned y, unsigned z);
