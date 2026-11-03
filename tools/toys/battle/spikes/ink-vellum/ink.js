// Living ink on vellum: Cannae drawn with a finger. WebGL2, no libraries.
// The page is calfskin bearing Cannae's ground (ground.js); two inks lie on it as the armies, and a
// stroke is a wet channel the inks seep along. ?manual=1 stops the clock: window.ink.step(n)/render().
(() => {
'use strict';
const canvas = document.getElementById('c');
const gl = canvas.getContext('webgl2', { antialias: false, preserveDrawingBuffer: true, premultipliedAlpha: false });
if (!gl) throw new Error('webgl2');
gl.getExtension('EXT_color_buffer_float');
gl.getExtension('OES_texture_float_linear');

const SW = 448, SH = 336;                    // the simulation's grid (the full-size skin carries the detail)
const manual = /manual=1/.test(location.search);

// ---------------------------------------------------------------- GL helpers
const VS = `#version 300 es
in vec2 p; out vec2 uv; void main(){ uv = p*0.5+0.5; gl_Position = vec4(p,0.,1.); }`;
const NOISE = `
float h21(vec2 p){ p = fract(p*vec2(123.34, 456.21)); p += dot(p, p+45.32); return fract(p.x*p.y); }
vec2 h22(vec2 p){ float n = h21(p); return vec2(n, h21(p+n+17.1)); }
float vnoise(vec2 p){ vec2 i = floor(p), f = fract(p); vec2 u = f*f*(3.-2.*f);
  return mix(mix(h21(i), h21(i+vec2(1,0)), u.x), mix(h21(i+vec2(0,1)), h21(i+vec2(1,1)), u.x), u.y); }
float fbm(vec2 p){ float s = 0., a = .5; mat2 m = mat2(1.6,1.2,-1.2,1.6); for(int i=0;i<5;i++){ s += a*vnoise(p); p = m*p; a *= .5; } return s; }
float fbm3(vec2 p){ float s = 0., a = .5; mat2 m = mat2(1.6,1.2,-1.2,1.6); for(int i=0;i<3;i++){ s += a*vnoise(p); p = m*p; a *= .5; } return s; }
`;
// The skin itself, shared by the grid-sized paper (for the flow) and the full-size paper (for the eye).
const PAPER = NOISE + `
uniform sampler2D G;                 // ground: r elevation, g flow accumulation
const float ASP = 1.3333;
float fiberAngle(vec2 uv){ return 0.55 + 1.1*(fbm3(uv*vec2(1.3,1.0)+3.1)-0.5); }   // the grain follows the beast's back
vec2 fiberDir(vec2 uv){ float a = fiberAngle(uv); return vec2(cos(a), sin(a)); }
float river(vec2 uv){
  return smoothstep(0.05, 0.9, texture(G, uv).b);   // the Ofanto, softened (build_ground.py)
}
float gully(vec2 uv){ return smoothstep(0.55, 0.75, texture(G, uv).g); }
float elev(vec2 uv){ return texture(G, uv).r; }
float cockle(vec2 uv){
  vec2 d = fiberDir(uv); vec2 q = vec2(dot(uv*vec2(ASP,1.), d), dot(uv*vec2(ASP,1.), vec2(-d.y,d.x)));
  return fbm3(q*vec2(2.2, 5.5)+7.) ;          // ripples run along the grain
}
float streak(vec2 uv, float s){            // fibres: noise stretched along the grain
  vec2 d = fiberDir(uv); vec2 q = uv*vec2(ASP,1.)*s;
  q = vec2(dot(q,d), dot(q,vec2(-d.y,d.x)));
  return vnoise(q*vec2(0.08, 1.0)) * 0.6 + vnoise(q*vec2(0.05,2.3)+9.)*0.4;
}
float height(vec2 uv){
  float e = elev(uv);
  return 0.55*cockle(uv) + 0.35*e - 0.25*river(uv) - 0.05*gully(uv);
}
float absorb(vec2 uv){
  float a = 0.15 + 0.85*smoothstep(0.30, 0.78, streak(uv, 130.)*0.7 + streak(uv, 300.)*0.3);
  a = mix(a, 1.0, 0.6*river(uv));                     // the river line is thinner, wetter skin
  a *= 0.8 + 0.4*fbm3(uv*9.+2.);
  return clamp(a, 0.05, 1.);
}
`;
function compile(fs, outs) {
  const p = gl.createProgram();
  for (const [t, s] of [[gl.VERTEX_SHADER, VS], [gl.FRAGMENT_SHADER, fs]]) {
    const sh = gl.createShader(t); gl.shaderSource(sh, s); gl.compileShader(sh);
    if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) { console.error(s.split('\n').map((l, i) => (i + 1) + ' ' + l).join('\n')); throw new Error(gl.getShaderInfoLog(sh)); }
    gl.attachShader(p, sh);
  }
  gl.bindAttribLocation(p, 0, 'p'); gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {}; const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) { const a = gl.getActiveUniform(p, i); u[a.name.replace('[0]', '')] = gl.getUniformLocation(p, a.name); }
  return { p, u };
}
const quad = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, quad);
gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

function tex(w, h, fmt, data, filter) {
  const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
  const [ifmt, f, ty] = fmt === 'f' ? [gl.RGBA16F, gl.RGBA, gl.HALF_FLOAT] : [gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE];
  gl.texImage2D(gl.TEXTURE_2D, 0, ifmt, w, h, 0, f, ty, data || null);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter || gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter || gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return t;
}
function fbo(...ts) {
  const f = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, f);
  ts.forEach((t, i) => gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0 + i, gl.TEXTURE_2D, t, 0));
  gl.drawBuffers(ts.map((_, i) => gl.COLOR_ATTACHMENT0 + i));
  return f;
}
function pingpong(n) { // n textures per side, rendered together (MRT)
  const a = [], b = [];
  for (let i = 0; i < n; i++) { a.push(tex(SW, SH, 'f')); b.push(tex(SW, SH, 'f')); }
  return { r: a, w: b, fr: fbo(...a), fw: fbo(...b), swap() { [this.r, this.w] = [this.w, this.r]; [this.fr, this.fw] = [this.fw, this.fr]; } };
}
function run(prog, target, w, h, binds, set) {
  gl.useProgram(prog.p); gl.bindFramebuffer(gl.FRAMEBUFFER, target); gl.viewport(0, 0, w, h);
  let unit = 0;
  for (const [name, t] of Object.entries(binds)) { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t); gl.uniform1i(prog.u[name], unit++); }
  if (prog.u.px) gl.uniform2f(prog.u.px, 1 / SW, 1 / SH);
  if (set) set(prog.u);
  gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
}

// ---------------------------------------------------------------- programs
// S: r water, g iron-gall suspended, b vermilion suspended, a wet time
// D: r iron-gall deposit, g vermilion deposit, b dried-ness (iron gall browns as it dries), a bloom/tide
// F: rg the strokes' direction x strength, b strength
// P (grid paper): r height, g absorbency, ba grain direction
const pPaperGrid = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; ` + PAPER + `
void main(){ vec2 u = vec2(uv.x, 1.-uv.y); o = vec4(height(u), absorb(u), fiberDir(u)); }`);

// The full-size skin as the eye sees it: colour, and in alpha the pores the pigment settles into.
const pPaperFull = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform vec2 res; ` + PAPER + `
float follicles(vec2 u){
  vec2 d = fiberDir(u); vec2 q = u*vec2(ASP,1.)*150.; q = vec2(dot(q,d), dot(q,vec2(-d.y,d.x)));
  vec2 i = floor(q), f = fract(q); float m = 0.;
  for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++){
    vec2 g = vec2(x,y); vec2 c = h22(i+g);
    if (h21(i+g+3.3) < 0.45) continue;
    // groups of three, as calf follicles grow
    for(int k=0;k<3;k++){
      vec2 pt = g + c*0.6 + 0.2 + 0.09*vec2(cos(float(k)*2.1+c.x*6.), sin(float(k)*2.1+c.x*6.));
      vec2 dd = (f-pt)*vec2(1.0, 1.9);
      float r = 0.035 + 0.03*h21(i+g+float(k));
      m = max(m, smoothstep(r, r*0.35, length(dd)) * (0.5+0.5*h21(i+g+9.+float(k))));
    }
  }
  return m;
}
void main(){
  vec2 u = vec2(uv.x, 1.-uv.y); vec2 e = vec2(1.5/res.x, 1.5/res.y);
  float hc = height(u);
  vec2 n2 = vec2(height(u+vec2(e.x,0.))-height(u-vec2(e.x,0.)), height(u+vec2(0.,e.y))-height(u-vec2(0.,e.y)));
  vec3 n = normalize(vec3(-n2*vec2(res.x,res.y)*0.0009, 1.));
  float lit = dot(n, normalize(vec3(-0.5,-0.7,0.9)));
  // ground relief, embossed faintly into the skin
  float el = elev(u);
  vec2 ge = vec2(elev(u+vec2(e.x*3.,0.))-elev(u-vec2(e.x*3.,0.)), elev(u+vec2(0.,e.y*3.))-elev(u-vec2(0.,e.y*3.)));
  float hill = clamp(-(ge.x*0.8+ge.y*1.0)*55., -1., 1.);
  // colour of calfskin: ivory, warmer in patches, a pinkish flush, thin grey where translucent
  float big = fbm(u*vec2(2.,1.6)+11.);
  float mid = fbm(u*14.+4.);
  vec3 ivory = vec3(0.955, 0.905, 0.800);
  vec3 warm  = vec3(0.880, 0.790, 0.640);
  vec3 flush = vec3(0.930, 0.840, 0.770);
  vec3 c = mix(ivory, warm, smoothstep(0.3, 0.85, big)*0.55);
  c = mix(c, flush, smoothstep(0.5, 0.8, fbm(u*3.+20.))*0.30);
  c *= 0.97 + 0.06*mid;
  float fib = streak(u, 900.); c *= 0.965 + 0.06*fib;
  // veins: faint ridged lines in the skin
  float v = 1.-abs(fbm(u*vec2(7.,5.5)+31.)*2.-1.); v = pow(v, 40.) * smoothstep(0.45, 0.7, fbm(u*2.+5.));
  c = mix(c, c*vec3(0.84,0.76,0.70), v*0.30);
  // the ground: higher land a little paler, slopes shaded; the river a darker, wetter line in the skin
  c *= 1.0 + 0.05*(el-0.5) + 0.06*hill;
  float rv = river(u), gu = gully(u);
  c = mix(c, c*vec3(0.80,0.78,0.72), rv*0.45);   // thin wet skin: darker, translucent
  c = mix(c, c*vec3(0.92,0.89,0.84), gu*0.25);
  // cockling light
  c *= 0.95 + 0.11*lit;
  // follicles
  float fo = follicles(u); c = mix(c, c*vec3(0.62,0.52,0.42), fo*0.55);
  // pores for granulation
  float pore = clamp(0.45*fo + 0.22*(1.-fib) + 0.45*vnoise(u*vec2(ASP,1.)*1600.) + 0.25*vnoise(u*vec2(ASP,1.)*450.) - 0.2, 0., 1.);
  // the skin's edge darkens a little (handled, aged)
  vec2 q = abs(uv-0.5)*2.; float edge = pow(max(q.x,q.y), 6.);
  c *= 1. - 0.12*edge - 0.05*dot(q,q)*0.25;
  o = vec4(c, pore);
}`);

// Splat capsules of water and pigment (the armies' pools, a stroke's ink) and the stroke's direction.
const pSplat = compile(`#version 300 es
precision highp float; in vec2 uv; layout(location=0) out vec4 oS; layout(location=1) out vec4 oF;
uniform sampler2D S, F, P; uniform vec4 seg[32]; uniform int n; uniform float rad, water, pigA, pigB, flowR, flowK, ragged;
` + NOISE + `
void main(){
  vec4 s = texture(S, uv), f = texture(F, uv);
  vec2 su = vec2(uv.x, 1.-uv.y); vec2 q = su*vec2(1.3333,1.);
  float best = 1e9; vec2 dir = vec2(0.);
  for(int i=0;i<32;i++){ if(i>=n) break;
    vec2 a = seg[i].xy*vec2(1.3333,1.), b = seg[i].zw*vec2(1.3333,1.);
    vec2 ab = b-a; float L = dot(ab,ab); float t = L>0. ? clamp(dot(q-a,ab)/L,0.,1.) : 0.;
    float d = length(q-(a+t*ab)); if(d<best){ best = d; dir = L>0. ? ab/sqrt(L) : vec2(0.); }
  }
  float r = rad*(1. + ragged*(vnoise(q*60.)-0.5) + ragged*0.5*(vnoise(q*190.)-0.5));
  float m = smoothstep(r, r*0.55, best);
  float wcur = s.r;
  s.r += water*m; s.g += pigA*m; s.b += pigB*m;
  float fm = smoothstep(flowR, flowR*0.3, best)*flowK;
  if (fm > 0.) { f.xy = mix(f.xy, dir*vec2(1.,-1.), fm); f.z = max(f.z, fm); }
  oS = s; oF = f;
}`);

// Advection: along the strokes, down the cockles' slopes, and the coffee ring (pigment to the drying edge).
const pAdvect = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform sampler2D S, F, P; uniform vec2 px; uniform float speed;
void main(){
  vec4 s = texture(S, uv); vec4 f = texture(F, uv); vec4 p = texture(P, uv);
  float hL = texture(P, uv-vec2(px.x,0.)).r, hR = texture(P, uv+vec2(px.x,0.)).r, hD = texture(P, uv-vec2(0.,px.y)).r, hU = texture(P, uv+vec2(0.,px.y)).r;
  float wL = texture(S, uv-vec2(px.x,0.)).r, wR = texture(S, uv+vec2(px.x,0.)).r, wD = texture(S, uv-vec2(0.,px.y)).r, wU = texture(S, uv+vec2(0.,px.y)).r;
  vec2 gh = vec2(hR-hL, hU-hD), gw = vec2(wR-wL, wU-wD);
  float wet = smoothstep(0.0, 0.25, s.r);
  vec2 v = f.xy*speed*wet - gh*1.6*wet;
  // capillary outflow to the edge: the pigment rides out where the film thins
  vec2 vc = -gw/(length(gw)+0.02) * 0.22 * smoothstep(0.02, 0.15, s.r) * (1.-smoothstep(0.3, 1.2, s.r));
  vec4 bw = texture(S, uv - v*px);
  vec4 bp = texture(S, uv - (v+vc)*px);
  o = vec4(bw.r, bp.g, bp.b, s.a);
}`);

// Anisotropic capillary spreading through the skin, with a wet front that only advances where the
// skin drinks (a threshold), so edges feather along the grain.
const pDiffuse = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform sampler2D S, P; uniform vec2 px; uniform float K, tau;
void main(){
  vec4 c = texture(S, uv); vec4 pc = texture(P, uv);
  vec2 fd = pc.ba;
  vec3 acc = vec3(0.);
  for(int y=-1;y<=1;y++) for(int x=-1;x<=1;x++){
    if(x==0 && y==0) continue;
    vec2 o2 = vec2(float(x), float(y)); vec2 nuv = uv + o2*px;
    vec4 n = texture(S, nuv);
    vec2 dir = normalize(o2*vec2(1.,-1.));
    float al = abs(dot(dir, fd));
    float k = K * mix(0.025, 1.0, al*al*al) * (c.a+n.a)*0.5 * ((x!=0 && y!=0) ? 0.6 : 1.0);
    float dw = n.r - c.r;
    // the receiver's threshold: a dry, tight patch resists the front
    float rw = dw > 0. ? c.r : n.r; float ra = dw > 0. ? c.a : n.a;
    float t = tau * (1.2 - ra) * (1. - smoothstep(0.02, 0.12, rw));
    float fl = sign(dw) * max(abs(dw) - t, 0.) * k;
    vec2 conc = dw > 0. ? n.gb/max(n.r, 1e-3) : c.gb/max(c.r, 1e-3);
    acc += vec3(fl, fl*conc);
  }
  vec4 r = c; r.rgb += acc; r.rgb = max(r.rgb, 0.); r.a = c.a;
  o = r;
}`);

// Drying, settling, the clash's bloom.
const pDry = compile(`#version 300 es
precision highp float; in vec2 uv; layout(location=0) out vec4 oS; layout(location=1) out vec4 oD;
uniform sampler2D S, D, P; uniform vec2 px; uniform float evap, dep, bloom;
void main(){
  vec4 s = texture(S, uv), d = texture(D, uv), p = texture(P, uv);
  float w = s.r;
  // evaporates faster at a thin edge
  float wn = 0.25*(texture(S, uv+vec2(px.x*2.,0.)).r + texture(S, uv-vec2(px.x*2.,0.)).r + texture(S, uv+vec2(0.,px.y*2.)).r + texture(S, uv-vec2(0.,px.y*2.)).r);
  float edge = clamp((wn - w)*4. , 0., 1.) + (1.-smoothstep(0.0, 0.2, w));
  float e = evap*(0.6 + 1.6*edge);
  float w2 = max(w - e, 0.);
  // pigment settles: slowly while wet (faster in thirsty skin), all of it as the water goes
  float r = dep*(0.4 + p.g) + (1.-smoothstep(0.0, 0.06, w2))*0.6;
  r = clamp(r, 0., 1.);
  vec2 settle = s.gb*r;
  d.rg += settle; s.gb -= settle;
  // fresh water lifts what has dried (vermilion readily; oxidised iron gall barely): blooms, backruns
  vec2 lift = d.rg * 0.025 * smoothstep(0.25, 0.9, w2) * vec2(1.-0.85*d.b, 1.);
  d.rg -= lift; s.gb += lift;
  // two inks meeting: the clash blooms (water pushed out from the meeting) and flocculates darker
  float m = min(s.g, s.b);
  float cl = smoothstep(0.03, 0.25, m) * smoothstep(0.05, 0.3, w2);
  w2 += bloom*cl;
  d.a = min(d.a + cl*0.02, 1.);
  d.rg += vec2(s.g, s.b)*cl*0.015; s.gb *= 1. - cl*0.015;
  // iron gall browns where it has dried
  d.b = w2 < 0.02 ? min(d.b + 0.004, 1.) : d.b*0.995;
  s.r = w2;
  oS = s; oD = d;
}`);

const pInit = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform sampler2D P;
void main(){ o = vec4(0.,0.,0., texture(P, uv).g); }`);

const pDecay = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform sampler2D F; uniform float k;
void main(){ vec4 f = texture(F, uv); o = vec4(f.xy*k, f.z*k, 0.); }`);

// Once a frame at the grid's size: what lies around each cell (edge darkening, halo) and the slopes (sheen).
const pPost = compile(`#version 300 es
precision highp float; in vec2 uv; layout(location=0) out vec4 oE; layout(location=1) out vec4 oN;
uniform sampler2D S, D, P; uniform vec2 px;
void main(){
  vec2 b1 = vec2(0.), b2 = vec2(0.);
  for(int i=0;i<8;i++){ float a = float(i)*0.785+0.3; vec2 o2 = vec2(cos(a), sin(a))*px;
    vec4 d1 = texture(D, uv+o2*1.6); vec4 s1 = texture(S, uv+o2*1.6); b1 += d1.rg + s1.gb*0.5;
    b2 += texture(D, uv+o2*4.5).rg; }
  oE = vec4(b1/8., b2/8.);
  vec2 g = vec2(texture(S, uv+vec2(px.x,0.)).r - texture(S, uv-vec2(px.x,0.)).r, texture(S, uv+vec2(0.,px.y)).r - texture(S, uv-vec2(0.,px.y)).r);
  vec2 gp = vec2(texture(P, uv+vec2(px.x,0.)).r - texture(P, uv-vec2(px.x,0.)).r, texture(P, uv+vec2(0.,px.y)).r - texture(P, uv-vec2(0.,px.y)).r);
  oN = vec4(g, gp);
}`);

// The eye: the skin, the inks by absorption (iron gall) and by cover (vermilion), wet sheen.
const pShow = compile(`#version 300 es
precision highp float; in vec2 uv; out vec4 o; uniform sampler2D S, D, E, N, PC;
float lum(vec3 c){ return dot(c, vec3(.3,.59,.11)); }
void main(){
  vec4 pc = texture(PC, uv); vec4 s = texture(S, uv); vec4 d = texture(D, uv); vec4 e = texture(E, uv); vec4 nn = texture(N, uv);
  float pore = pc.a;
  vec2 den = d.rg + s.gb*0.5;
  // the skin's fibres break the grid's soft edge into a fibrous one
  float fibre = pore - 0.5;
  den = max(den * (1. + 0.35*fibre) - 0.015*fibre, 0.);
  float gran = mix(1.0, 0.3 + 1.5*pore, 0.6);
  vec2 edge = max(den - e.xy, 0.);
  vec2 dEff = den*gran + edge*(0.8 + 1.4*pore);
  // iron gall: blue-black wet, brown-black dry, a warm halo where it bit the skin
  float ox = d.b;
  vec3 Kw = vec3(2.2, 2.05, 1.8), Kd = vec3(1.45, 1.9, 2.7);
  vec3 K = mix(Kw, Kd, ox);
  vec3 col = pc.rgb * exp(-K*dEff.r*1.7);
  float halo = clamp((e.z - den.r)*3., 0., 1.) * ox * (0.35 + 0.9*pore) * 0.6;
  col *= mix(vec3(1.), vec3(0.90, 0.80, 0.64), halo);
  // vermilion: an opaque earth that covers, deep in its pools, granulating
  float cov = 1. - exp(-2.8*dEff.g);
  vec3 vl = vec3(0.90, 0.33, 0.18), vd = vec3(0.58, 0.10, 0.07);
  vec3 vc = mix(vl, vd, smoothstep(0.2, 1.6, dEff.g)) * (0.72 + 0.32*lum(pc.rgb)/0.85);
  vc *= exp(-K*dEff.r*0.4);                        // iron gall over vermilion: a maroon black
  col = mix(col, vc, cov);
  float halo2 = clamp((e.w - den.g)*2.5, 0., 1.);
  col *= mix(vec3(1.), vec3(0.99, 0.90, 0.84), halo2*0.45*(0.4+pore));
  // the clash's tide-lines: blooms leave a darker feathered rim
  col *= 1. - clamp(d.a*(edge.r+edge.g)*6., 0., 0.45);
  // wet: the skin darkens and the surface glistens
  float wet = smoothstep(0.0, 0.35, s.r);
  col *= 1. - 0.10*wet;
  vec3 n = normalize(vec3(-(nn.xy*1.3 + nn.zw*0.6), 0.22));
  vec3 L = normalize(vec3(-0.45, 0.6, 0.65));
  float spec = pow(max(reflect(-L, n).z, 0.), 60.) * smoothstep(0.2, 0.9, s.r);
  col += vec3(1.0, 0.98, 0.94) * spec*0.28;
  o = vec4(clamp(col, 0., 1.), 1.);
}`);

// ---------------------------------------------------------------- state
const S = pingpong(1), D = pingpong(1), F = pingpong(1);
const P = tex(SW, SH, 'f');
const E = tex(SW, SH, 'f'), N = tex(SW, SH, 'f'); const fEN = fbo(E, N);
const PCs = {};
const fP = fbo(P);
let groundTex = null;
const pending = []; // splats queued for the next step: {segs, params}

function splat(segs, params) { pending.push({ segs, params }); }
function flushSplats() {
  for (const { segs, params } of pending) {
    for (let i = 0; i < segs.length; i += 32) {
      const chunk = segs.slice(i, i + 32); const flat = new Float32Array(32 * 4);
      chunk.forEach((s, j) => flat.set(s, j * 4));
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      // S and F written together: render into S.w (0) and F.w (1)
      const f = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, f);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, S.w[0], 0);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT1, gl.TEXTURE_2D, F.w[0], 0);
      gl.drawBuffers([gl.COLOR_ATTACHMENT0, gl.COLOR_ATTACHMENT1]);
      run(pSplat, f, SW, SH, { S: S.r[0], F: F.r[0], P }, u => {
        gl.uniform4fv(u.seg, flat); gl.uniform1i(u.n, chunk.length);
        gl.uniform1f(u.rad, params.rad); gl.uniform1f(u.water, params.water);
        gl.uniform1f(u.pigA, params.pigA || 0); gl.uniform1f(u.pigB, params.pigB || 0);
        gl.uniform1f(u.flowR, params.flowR || 0.0001); gl.uniform1f(u.flowK, params.flowK || 0);
        gl.uniform1f(u.ragged, params.ragged || 0);
      });
      gl.deleteFramebuffer(f);
      S.swap(); F.swap();
    }
  }
  pending.length = 0;
}

let mrtSD = null;
function step(n = 1) {
  for (let i = 0; i < n; i++) {
    flushSplats();
    run(pAdvect, S.fw, SW, SH, { S: S.r[0], F: F.r[0], P }, u => gl.uniform1f(u.speed, 2.4)); S.swap();
    for (let k = 0; k < 1; k++) {
      run(pDiffuse, S.fw, SW, SH, { S: S.r[0], P }, u => { gl.uniform1f(u.K, 0.11); gl.uniform1f(u.tau, 0.05); }); S.swap();
    }
    const f = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, f);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, S.w[0], 0);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT1, gl.TEXTURE_2D, D.w[0], 0);
    gl.drawBuffers([gl.COLOR_ATTACHMENT0, gl.COLOR_ATTACHMENT1]);
    run(pDry, f, SW, SH, { S: S.r[0], D: D.r[0], P }, u => { gl.uniform1f(u.evap, ink.evap); gl.uniform1f(u.dep, 0.004); gl.uniform1f(u.bloom, 0.02); });
    gl.deleteFramebuffer(f);
    S.swap(); D.swap();
    run(pDecay, F.fw, SW, SH, { F: F.r[0] }, u => gl.uniform1f(u.k, 0.9996)); F.swap();
  }
}
function render() {
  run(pPost, fEN, SW, SH, { S: S.r[0], D: D.r[0], P }, null);
  run(pShow, null, canvas.width, canvas.height, { S: S.r[0], D: D.r[0], E, N, PC: PCs[scale] }, null);
}
// the canvas's pixels per CSS pixel: the device's for the eye, 1 for a quick film frame
let scale = 0;
function setScale(k) {
  scale = k;
  canvas.width = Math.round(canvas.clientWidth * k); canvas.height = Math.round(canvas.clientHeight * k);
  if (!PCs[k]) {
    PCs[k] = tex(canvas.width, canvas.height, 'b');
    run(pPaperFull, fbo(PCs[k]), canvas.width, canvas.height, { G: groundTex }, u => gl.uniform2f(u.res, canvas.width, canvas.height));
  }
}

// ---------------------------------------------------------------- the armies (the ground truth underneath)
// uv: x right, y down. The Ofanto runs up the page at x~0.34; the Romans face down, the river on their right.
const IRON = { pigA: 1, pigB: 0 }, VERM = { pigA: 0, pigB: 1 };
const pools = [];
let seed = 216;
function rnd() { seed = (seed * 1664525 + 1013904223) >>> 0; return seed / 4294967296; }
// the battle laid a little larger than life on the page, about its centre
const place = (x, y) => [0.55 + (x - 0.55) * 1.15, 0.47 + (y - 0.45) * 1.15];
function army(ink, segs, rad, depth) {
  segs = segs.map(s => [...place(s[0], s[1]), ...place(s[2], s[3])]); rad *= 1.15;
  segs.forEach(s => pools.push({ ink, x: (s[0] + s[2]) / 2, y: (s[1] + s[3]) / 2 }));
  splat(segs, { rad, water: 1.6 * depth, pigA: ink.pigA * 1.1 * depth, pigB: ink.pigB * 1.1 * depth, ragged: 0.5 });
}
// a body of men as many small drops laid close: units (maniples, bands) jittered, so the pool is lumpy
function body(ink, cx, cy, w, h, cols, rows, rad, depth, bow) {
  const segs = [];
  for (let r = 0; r < rows; r++) for (let c = 0; c < cols; c++) {
    const fx = cols > 1 ? c / (cols - 1) - 0.5 : 0, fy = rows > 1 ? r / (rows - 1) - 0.5 : 0;
    const x = cx + fx * w + (rnd() - 0.5) * w / cols * 0.6;
    const y = cy + fy * h + (rnd() - 0.5) * h / Math.max(rows, 1) * 0.5 + (bow || 0) * (fx * fx * 4 - 1);
    const a = (rnd() - 0.5) * 0.6, l = 0.006 + rnd() * 0.008;
    segs.push([x - Math.cos(a) * l, y - Math.sin(a) * l, x + Math.cos(a) * l, y + Math.sin(a) * l]);
  }
  army(ink, segs, rad, depth);
}
function arc(cx, cy, rx, ry, a0, a1, k) {
  const out = [];
  for (let i = 0; i < k; i++) {
    const t0 = a0 + (a1 - a0) * i / k, t1 = a0 + (a1 - a0) * (i + 1) / k;
    out.push([cx + rx * Math.cos(t0), cy + ry * Math.sin(t0), cx + rx * Math.cos(t1), cy + ry * Math.sin(t1)]);
  }
  return out;
}
function setupArmies() {
  // Rome: the deep centre, its horse by the river, the allied horse on its left
  body(IRON, 0.553, 0.30, 0.22, 0.075, 11, 4, 0.0145, 1.0);
  body(IRON, 0.40, 0.295, 0.035, 0.02, 3, 1, 0.012, 0.8);
  body(IRON, 0.725, 0.295, 0.045, 0.02, 3, 1, 0.012, 0.8);
  // Carthage: Gauls and Spaniards bowed toward Rome, Africans on the wings, Hasdrubal by the river, Numidians right
  const cres = arc(0.553, 0.60, 0.125, 0.10, Math.PI * 1.2, Math.PI * 1.8, 10).map(s => s.map((v, k) => v + (rnd() - 0.5) * 0.006));
  army(VERM, cres, 0.0135, 1.0);
  body(VERM, 0.41, 0.585, 0.02, 0.05, 1, 3, 0.012, 0.9);
  body(VERM, 0.695, 0.585, 0.02, 0.05, 1, 3, 0.012, 0.9);
  body(VERM, 0.375, 0.625, 0.03, 0.03, 2, 2, 0.012, 0.9);
  body(VERM, 0.74, 0.615, 0.04, 0.02, 3, 1, 0.011, 0.8);
}

// ---------------------------------------------------------------- the finger
let stroke = null;
function toUV(e) { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height]; }
function inkAt(x, y) {
  let best = null, bd = 1e9;
  for (const p of pools) { const d = Math.hypot((p.x - x) * 1.333, p.y - y); if (d < bd) { bd = d; best = p; } }
  return bd < 0.07 ? best.ink : null;
}
canvas.addEventListener('pointerdown', e => {
  const [x, y] = toUV(e); canvas.setPointerCapture(e.pointerId);
  stroke = { x, y, x0: x, y0: y, ink: inkAt(x, y), len: 0 };
});
canvas.addEventListener('pointermove', e => {
  if (!stroke) return;
  const [x, y] = toUV(e); const d = Math.hypot((x - stroke.x) * 1.333, y - stroke.y);
  if (d < 0.004) return;
  const ink = stroke.ink || { pigA: 0.25, pigB: 0 };
  // the stroke: a wet line of the ink it was dipped in, and a channel the pools will seep along
  splat([[stroke.x, stroke.y, x, y]], { rad: 0.0042, water: 1.0, pigA: ink.pigA * 0.22, pigB: ink.pigB * 0.22, flowR: 0.026, flowK: 1.0, ragged: 0.4 });
  stroke.len += d; stroke.x = x; stroke.y = y;
});
function up(e) {
  if (!stroke) return;
  if (stroke.len < 0.01) { // a tap: a drop of clear water, which blooms whatever it falls in
    splat([[stroke.x0, stroke.y0, stroke.x0, stroke.y0]], { rad: 0.03, water: 2.2, ragged: 0.7 });
  }
  stroke = null;
}
canvas.addEventListener('pointerup', up); canvas.addEventListener('pointercancel', up);

// ---------------------------------------------------------------- boot
const ink = window.ink = { ready: false, step, render, setScale, evap: 0.0011, splat, place };
const img = new Image();
img.onload = () => {
  groundTex = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, groundTex);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, img);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  run(pPaperGrid, fP, SW, SH, { G: groundTex });
  run(pInit, S.fr, SW, SH, { P });
  setScale(window.devicePixelRatio || 1);
  setupArmies();
  step(1); render();
  ink.ready = true;
  if (!manual) { const loop = () => { step(3); render(); requestAnimationFrame(loop); }; loop(); }
};
img.src = window.GROUND;
})();
