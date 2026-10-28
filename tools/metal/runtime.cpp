// Runs one fragment or kernel of a shader compiled by tools/metal/metal.py, on the CPU.
//
// A fragment runs as a GPU runs it: in quads, 2x2 pixels together, so dfdx, dfdy and fwidth are the
// differences between neighbours at the same moment of the program. Each of the four is a coroutine
// (its own stack; on x86-64 switched by six registers, elsewhere by ucontext, whose every switch is a
// system call); a derivative parks it until every live one has parked or finished, then the
// differences are shared out. Where the four have parted ways (one returned, or skipped the line the
// others are on), a derivative comes from the neighbours on the same line, else the other row's,
// else 0, and the line is reported: on the GPU that derivative is undefined.
//
// The job (metal.py writes it): `threads n`; `fragment w h` or `kernel x y z`; `buffer i path`;
// `texture i path w h d levels unorm8` (bytes if unorm8, else floats; every level in turn);
// `output path i`. A fragment writes w x h RGBA floats to the
// output's path (and beside it, `.parted`, a byte a pixel: 1 where its quad took a derivative apart);
// a kernel writes the texture bound at i.
#include "metal_runtime.h"

#include <atomic>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <map>
#include <mutex>
#include <sstream>
#include <string>
#include <thread>
#include <vector>
#include <ucontext.h>

extern "C" __attribute__((weak)) void __mtl_fragment(const __mtl_bindings *, float, float, float *);
extern "C" __attribute__((weak)) void __mtl_kernel(const __mtl_bindings *, unsigned, unsigned, unsigned);

#if defined(__x86_64__)
// switch(from, to): save the callee-saved registers on this stack, keep its top in *from, take up `to`
extern "C" void __mtl_switch(void **from, void *to);
asm(R"(
    .text
    .globl __mtl_switch
    .type __mtl_switch, @function
__mtl_switch:
    pushq %rbp
    pushq %rbx
    pushq %r12
    pushq %r13
    pushq %r14
    pushq %r15
    movq %rsp, (%rdi)
    movq %rsi, %rsp
    popq %r15
    popq %r14
    popq %r13
    popq %r12
    popq %rbx
    popq %rbp
    ret
    .size __mtl_switch, .-__mtl_switch
)");
#define MTL_FAST_SWITCH 1
#endif

namespace {

const size_t STACK = 512 * 1024;

enum State { RUNNING, PARKED, DONE };

struct Lane {
#if MTL_FAST_SWITCH
    void *sp;
#else
    ucontext_t context;
#endif
    std::vector<char> stack;
    State state;
    float x, y;
    float out[4];
    float value[4], dx[4], dy[4];
    int n, line;
};

struct Quad {
#if MTL_FAST_SWITCH
    void *scheduler;
#else
    ucontext_t scheduler;
#endif
    Lane lane[4];               // (0,0), (1,0), (0,1), (1,1): x + 2y
    int current;
    bool apart;                 // whether any derivative in this quad was taken with it apart
    const __mtl_bindings *bindings;
    std::map<int, long> parted; // line: how many times a derivative there was taken with the quad apart
};

thread_local Quad *active = nullptr;

void lane_main() {
    Quad *q = active;
    Lane &l = q->lane[q->current];
    __mtl_fragment(q->bindings, l.x, l.y, l.out);
    l.state = DONE;
#if MTL_FAST_SWITCH
    __mtl_switch(&l.sp, q->scheduler);                   // (never resumed: the next quad starts it afresh)
    __builtin_unreachable();
#endif
}

// to the lane from the scheduler, and back
void enter(Quad &q, Lane &l) {
#if MTL_FAST_SWITCH
    __mtl_switch(&q.scheduler, l.sp);
#else
    swapcontext(&q.scheduler, &l.context);
#endif
}

void leave(Quad &q, Lane &l) {
#if MTL_FAST_SWITCH
    __mtl_switch(&l.sp, q.scheduler);
#else
    swapcontext(&l.context, &q.scheduler);
#endif
}

void start(Quad &q, Lane &l) {
#if MTL_FAST_SWITCH
    // a stack whose first switch returns into lane_main as a call would enter it (16-byte aligned)
    char *top = l.stack.data() + l.stack.size();
    void **sp = (void **)((uintptr_t)top & ~uintptr_t(15));
    *--sp = nullptr;
    *--sp = (void *)lane_main;
    for (int i = 0; i < 6; i++) *--sp = nullptr;
    l.sp = sp;
#else
    getcontext(&l.context);
    l.context.uc_stack.ss_sp = l.stack.data();
    l.context.uc_stack.ss_size = l.stack.size();
    l.context.uc_link = &q.scheduler;
    makecontext(&l.context, lane_main, 0);
#endif
}

// fine derivatives (each row and column its own pair) unless METAL_DERIVATIVES=coarse (the quad's
// first row and column for all four)
bool coarse = false;

// The differences for every parked lane, by the line each is on.
void share(Quad &q) {
    bool apart = false;
    int lines[4];
    for (int i = 0; i < 4; i++) lines[i] = q.lane[i].state == PARKED ? q.lane[i].line : -1;
    for (int i = 0; i < 4; i++) if (lines[i] != lines[0]) apart = true;
    for (int i = 0; i < 4; i++) {
        Lane &l = q.lane[i];
        if (l.state != PARKED) continue;
        auto with = [&](int k) { return lines[k] == l.line; };
        int x = coarse ? 0 : i & 1, y = coarse ? 0 : i >> 1;
        for (int c = 0; c < l.n; c++) {
            auto v = [&](int k) { return q.lane[k].value[c]; };
            // fine: this row's pair, else the other row's; the same for columns
            if (with(2 * y) && with(2 * y + 1)) l.dx[c] = v(2 * y + 1) - v(2 * y);
            else if (with(2 * (1 - y)) && with(2 * (1 - y) + 1)) l.dx[c] = v(2 * (1 - y) + 1) - v(2 * (1 - y));
            else l.dx[c] = 0;
            if (with(x) && with(x + 2)) l.dy[c] = v(x + 2) - v(x);
            else if (with(1 - x) && with(3 - x)) l.dy[c] = v(3 - x) - v(1 - x);
            else l.dy[c] = 0;
        }
        if (apart) { q.parted[l.line]++; q.apart = true; }
    }
}

void run_quad(Quad &q, float x0, float y0) {
    active = &q;
    q.apart = false;
    for (int i = 0; i < 4; i++) {
        Lane &l = q.lane[i];
        l.x = x0 + (i & 1) + 0.5f;
        l.y = y0 + (i >> 1) + 0.5f;
        l.state = RUNNING;
        start(q, l);
    }
    for (;;) {
        for (int i = 0; i < 4; i++) {
            if (q.lane[i].state != RUNNING) continue;
            q.current = i;
            enter(q, q.lane[i]);
        }
        bool any = false;
        for (int i = 0; i < 4; i++) any |= q.lane[i].state == PARKED;
        if (!any) break;
        share(q);
        for (int i = 0; i < 4; i++) if (q.lane[i].state == PARKED) q.lane[i].state = RUNNING;
    }
}

[[noreturn]] void fail(const std::string &what) {
    std::fprintf(stderr, "runtime: %s\n", what.c_str());
    std::exit(2);
}

std::vector<char> slurp(const std::string &path) {
    std::ifstream f(path, std::ios::binary);
    if (!f) fail("can't read " + path);
    return std::vector<char>((std::istreambuf_iterator<char>(f)), std::istreambuf_iterator<char>());
}

}  // namespace

extern "C" void __mtl_quad(const float *v, int n, int line, float *dx, float *dy) {
    Quad *q = active;
    if (!q) {                                           // (a kernel has no quad: no neighbours)
        for (int i = 0; i < n; i++) dx[i] = dy[i] = 0;
        return;
    }
    Lane &l = q->lane[q->current];
    std::memcpy(l.value, v, sizeof(float) * n);
    l.n = n;
    l.line = line;
    l.state = PARKED;
    leave(*q, l);
    std::memcpy(dx, l.dx, sizeof(float) * n);
    std::memcpy(dy, l.dy, sizeof(float) * n);
}

int main(int argc, char **argv) {
    if (argc != 2) fail("usage: <binary> <job>");
    std::ifstream job(argv[1]);
    if (!job) fail("no job");
    int threads = 4, w = 0, h = 0, output = -1;
    unsigned grid[3] = {0, 0, 0};
    bool kernel = false;
    std::string out;
    __mtl_bindings bindings = {};
    std::vector<std::vector<char>> buffers;
    std::vector<__mtl_texture> textures(31);
    std::vector<std::vector<char>> storage(31);
    buffers.reserve(31);
    std::string line;
    while (std::getline(job, line)) {
        std::istringstream s(line);
        std::string word;
        s >> word;
        if (word == "threads") s >> threads;
        else if (word == "fragment") s >> w >> h;
        else if (word == "kernel") { kernel = true; s >> grid[0] >> grid[1] >> grid[2]; }
        else if (word == "buffer") {
            int i; std::string path;
            s >> i >> path;
            buffers.push_back(slurp(path));
            bindings.buffer[i] = buffers.back().data();
        } else if (word == "texture") {
            int i, levels, unorm8; std::string path;
            __mtl_texture t = {};
            s >> i >> path >> t.width >> t.height >> t.depth >> levels >> unorm8;
            t.levels = levels;
            t.format = unorm8 ? 1 : 0;
            storage[i] = slurp(path);
            size_t at = 0, texel = unorm8 ? 4 : 4 * sizeof(float);
            for (int k = 0; k < levels; k++) {
                t.level[k] = storage[i].data() + at;
                size_t lw = std::max(t.width >> k, 1), lh = std::max(t.height >> k, 1), ld = std::max(t.depth >> k, 1);
                at += lw * lh * ld * texel;
            }
            if (at > storage[i].size()) fail("texture " + std::to_string(i) + " is short of its levels");
            textures[i] = t;
            bindings.texture[i] = &textures[i];
        } else if (word == "output") s >> out >> output;
    }
    if (threads < 1) threads = 1;
    if (const char *d = std::getenv("METAL_DERIVATIVES")) coarse = std::string(d) == "coarse";

    if (kernel) {
        if (!__mtl_kernel) fail("this build has no kernel");
        if (output < 0 || !bindings.texture[output]) fail("a kernel's output names the texture it writes");
        std::atomic<unsigned> next{0};
        std::vector<std::thread> pool;
        for (int k = 0; k < threads; k++) pool.emplace_back([&] {
            for (unsigned z; (z = next++) < grid[2];)
                for (unsigned y = 0; y < grid[1]; y++)
                    for (unsigned x = 0; x < grid[0]; x++) __mtl_kernel(&bindings, x, y, z);
        });
        for (auto &t : pool) t.join();
        __mtl_texture &t = textures[output];
        std::ofstream f(out, std::ios::binary);
        f.write((const char *)t.level[0], (t.format == 1 ? 4 : 4 * sizeof(float)) * size_t(t.width) * t.height * t.depth);
        return 0;
    }

    if (!__mtl_fragment) fail("this build has no fragment");
    std::vector<float> image(size_t(w) * h * 4);
    std::vector<unsigned char> undefined(size_t(w) * h);
    std::atomic<int> next{0};
    std::mutex report;
    std::map<int, long> parted;
    std::vector<std::thread> pool;
    int qw = (w + 1) / 2, qh = (h + 1) / 2;
    for (int k = 0; k < threads; k++) pool.emplace_back([&] {
        Quad q;
        q.bindings = &bindings;
        for (auto &l : q.lane) l.stack.resize(STACK);
        for (int row; (row = next++) < qh;) {
            for (int col = 0; col < qw; col++) {
                run_quad(q, 2.0f * col, 2.0f * row);
                for (int i = 0; i < 4; i++) {
                    int x = 2 * col + (i & 1), y = 2 * row + (i >> 1);
                    if (x < w && y < h) {
                        std::memcpy(&image[(size_t(y) * w + x) * 4], q.lane[i].out, sizeof(float) * 4);
                        undefined[size_t(y) * w + x] = q.apart;
                    }
                }
            }
        }
        std::lock_guard<std::mutex> g(report);
        for (auto &p : q.parted) parted[p.first] += p.second;
    });
    for (auto &t : pool) t.join();
    std::ofstream f(out, std::ios::binary);
    f.write((const char *)image.data(), sizeof(float) * image.size());
    // which pixels took a derivative the GPU leaves undefined (their quad apart): one byte each
    std::ofstream(out + ".parted", std::ios::binary).write((const char *)undefined.data(), undefined.size());
    for (auto &p : parted)
        std::fprintf(stderr, "parted: line %d, %ld derivatives taken where the quad had parted ways\n", p.first, p.second);
    return 0;
}
