// Known answers for the stand-in (test_metal.py): derivatives, sampling, gather, a comparison made a
// vector, and a kernel's writes.
#include <metal_stdlib>
using namespace metal;

struct Probe { float4 position [[position]]; };
struct Knobs { float4 which; };

kernel void probeFill(texture2d<float, access::write> out [[texture(0)]], uint2 gid [[thread_position_in_grid]]) {
    out.write(float4(float(gid.x) / 3.0, float(gid.y) / 3.0, 0.5, 1.0), gid);
}

fragment float4 probeFragment(Probe in [[stage_in]], constant Knobs &K [[buffer(0)]],
                              texture2d<float> tex [[texture(0)]]) {
    constexpr sampler linear(filter::linear, address::clamp_to_edge);
    float2 p = in.position.xy;
    int which = int(K.which.x);
    if (which == 0) return float4(dfdx(p.x), dfdy(p.y), fwidth(p.x * 2.0 + p.y), dfdy(p.y * p.y));
    if (which == 1) return tex.sample(linear, float2(0.5, 0.5));
    if (which == 2) return tex.gather(linear, float2(0.5, 0.5));
    if (which == 3) return float4(float2(0.25, 0.75) < 0.5, 0.0, 1.0);
    float3x3 m = float3x3(float3(1, 0, 0), float3(0, 2, 0), float3(0, 0, 3));
    return float4(transpose(m) * float3(1.0), fract(-0.25));
}
