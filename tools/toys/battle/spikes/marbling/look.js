// The look: one WebGL2 pass that lays the pigment polygons (drawn by canvas 2D) on a tray of size over
// Cannae's ground, or, once printed, on paper. Rims, flocculated grain, bleed, size texture, fibre.
'use strict';

const VS = `#version 300 es
in vec2 a; out vec2 uv;
void main(){ uv = a*0.5+0.5; uv.y = 1.0-uv.y; gl_Position = vec4(a,0,1); }`;

const NOISE = `#version 300 es
precision highp float;
in vec2 uv; out vec4 o;
uniform vec2 css; uniform float seed;
float h(vec2 p){ p = fract(p*vec2(123.34,456.21)+seed); p += dot(p,p+45.32); return fract(p.x*p.y); }
float vn(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
  return mix(mix(h(i),h(i+vec2(1,0)),f.x), mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x), f.y); }
float fbm(vec2 p){ float s=0.0,a=0.5; for(int i=0;i<5;i++){ s+=a*vn(p); p=p*2.03+17.1; a*=0.5; } return s; }
void main(){
  vec2 p = uv*css;
  float sz = fbm(p*0.035)*0.6 + fbm(p*0.21)*0.4;              // the size's texture
  float grain = fbm(p*0.11 + 3.0)*0.55 + fbm(p*0.6)*0.45;    // flocculated pigment
  float fib = fbm(vec2(p.x*0.05, p.y*0.9)+fbm(p*0.02)*4.0);  // paper fibre
  float soak = fbm(p*0.3+5.0)*0.6 + fbm(vec2(p.x*0.7, p.y*0.04)+9.0)*0.4;
  o = vec4(sz, grain, fib, soak);
}`;

const FS = `#version 300 es
precision highp float;
in vec2 uv; out vec4 o;
uniform sampler2D dye, ground, nz;
uniform vec2 px;      // dye texel size
uniform vec2 css;     // css size
uniform float t, printed, sheet, seed;
float h(vec2 p){ p = fract(p*vec2(123.34,456.21)+seed); p += dot(p,p+45.32); return fract(p.x*p.y); }
float vn(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
  return mix(mix(h(i),h(i+vec2(1,0)),f.x), mix(h(i+vec2(0,1)),h(i+vec2(1,1)),f.x), f.y); }
vec4 D(vec2 q){ return texture(dye, q); }
void main(){
  vec2 p = uv*css;
  vec3 g = texture(ground, uv).rgb;
  vec4 N = texture(nz, uv);           // hillshade, river, elevation
  // ---- the tray of size: thickened water, cream-grey, the ground faint beneath
  float sz = N.x;
  vec3 size = vec3(0.835,0.815,0.745);
  size *= 0.93 + 0.13*(g.r-0.55) + 0.05*(g.b-0.4);
  size = mix(size, vec3(0.56,0.62,0.64), g.g*0.42);
  size *= 0.975 + 0.05*sz;
  float speck = step(0.9965, h(floor(p*0.9)));          // dust and bubbles on the size
  size = mix(size, size*1.06, speck*0.6);
  // ---- the pigment: bled a little, rimmed where colours meet, grainy where the film is thin
  vec4 c0 = D(uv);
  vec2 k1 = vec2(1.6)/css, k2 = vec2(4.0)/css;
  vec4 bl = (D(uv+vec2(k1.x,0.0))+D(uv-vec2(k1.x,0.0))+D(uv+vec2(0.0,k1.y))+D(uv-vec2(0.0,k1.y)))*0.25;
  vec4 b2 = (D(uv+k2)+D(uv-k2)+D(uv+vec2(k2.x,-k2.y))+D(uv+vec2(-k2.x,k2.y)))*0.25;
  vec4 c = mix(c0, bl, 0.35);
  float edge = clamp(length(c0.rgb - b2.rgb)*1.6 + abs(c0.a-b2.a)*0.7, 0.0, 1.0);
  float grain = N.y + (h(floor(p*2.0))-0.5)*0.12;
  float floc = 0.70 + 0.42*grain;
  vec3 pig = (c.rgb/max(c.a,1e-3)) * (1.0 - 0.16*edge);                      // pigment gathers at its boundary
  pig = mix(pig, pig*pig*1.15, 0.25*edge);
  float dens = clamp(c.a * floc * (1.0 - 0.10*smoothstep(0.55,0.9,grain)), 0.0, 1.0);
  // translucent film: the size, and the ground, show a little through it
  vec3 tray = mix(size, pig, dens*0.93);
  // the size's surface: a soft, broad reflection of a window, and a darker wall at the tray's rim
  float sheen = smoothstep(0.0,1.0, 1.0-length((uv-vec2(0.28,0.18))*vec2(1.0,1.6)));
  tray += vec3(0.035,0.035,0.03)*sheen*(0.6+0.4*sz);
  vec2 e = min(uv,1.0-uv)*css; float wall = smoothstep(0.0,26.0,min(e.x,e.y));
  tray *= 0.82 + 0.18*wall;
  // ---- the print: paper with fibre, the pigment soaked in unevenly
  float fib = N.z, fib2 = N.w;
  vec3 paper = vec3(0.955,0.93,0.865) * (0.965 + 0.05*fib + 0.025*fib2) * (0.985 + 0.03*N.x);
  float soak = clamp(dens*(0.82 + 0.25*N.w) * (0.92+0.1*fib), 0.0, 1.0);
  vec3 pr = mix(paper, pig*(0.96+0.06*fib), soak);
  pr = mix(pr, pr*vec3(0.9,0.88,0.82), (1.0-smoothstep(0.0,40.0,min(e.x,e.y)))*0.25); // deckle shadow
  vec3 col = mix(tray, pr, printed);
  // ---- the sheet laid down: paper settles from the left, wet where it touches
  float front = sheet*1.25 - 0.12 + 0.05*sin(uv.y*9.0+t);
  float cov = smoothstep(front, front-0.12, uv.x);
  vec3 wet = mix(paper*0.92, pig, dens*0.35);
  col = mix(col, wet, cov*step(printed,0.5));
  o = vec4(pow(col, vec3(1.0)), 1.0);
}`;

class Look {
  constructor(canvas, dyeCanvas, groundImg) {
    const gl = canvas.getContext('webgl2', { preserveDrawingBuffer: true, antialias: false });
    this.gl = gl; this.canvas = canvas; this.dye = dyeCanvas;
    const sh = (type, src) => { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; };
    const prog = (fs) => { const pr = gl.createProgram(); gl.attachShader(pr, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(pr, sh(gl.FRAGMENT_SHADER, fs)); gl.bindAttribLocation(pr, 0, 'a'); gl.linkProgram(pr);
      if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(pr)); return pr; };
    const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
    const tex = (unit) => { const t = gl.createTexture(); gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE); return t; };
    // the static textures (size, grain, fibre) once, at sketch resolution
    const np = prog(NOISE); gl.useProgram(np);
    this.tNoise = tex(2);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, window.MW, window.MH, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    const fb = gl.createFramebuffer(); gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, this.tNoise, 0);
    gl.viewport(0, 0, window.MW, window.MH);
    gl.uniform2f(gl.getUniformLocation(np, 'css'), window.MW, window.MH); gl.uniform1f(gl.getUniformLocation(np, 'seed'), 0.37);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    const pr = prog(FS); this.pr = pr; gl.useProgram(pr);
    this.tDye = tex(0); this.tGround = tex(1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, groundImg);
    const u = (n) => gl.getUniformLocation(pr, n);
    this.u = { px: u('px'), css: u('css'), t: u('t'), printed: u('printed'), sheet: u('sheet'), seed: u('seed') };
    gl.uniform1i(u('dye'), 0); gl.uniform1i(u('ground'), 1); gl.uniform1i(u('nz'), 2);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
  }
  render(t, printed, sheet) {
    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, this.tDye);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, this.dye);
    gl.uniform2f(this.u.px, 1 / this.dye.width, 1 / this.dye.height);
    gl.uniform2f(this.u.css, window.MW, window.MH);
    gl.uniform1f(this.u.t, t); gl.uniform1f(this.u.printed, printed); gl.uniform1f(this.u.sheet, sheet); gl.uniform1f(this.u.seed, 0.37);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }
}
window.Look = Look;
