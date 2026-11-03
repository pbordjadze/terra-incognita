// The sand table drawn: one torch, low and moving, over a height field of sand. Bicubic heights for
// smooth normals, a grain at every pixel or two (facets that catch the raking light, a few quartz
// glints, seeds and grains of the armies as little domes), shadows marched toward the flame,
// occlusion from the blurred height, the river wet and dark, dust drifting at the edges.
(function () {
  const S = window.SIM, cv = document.getElementById('c');
  const gl = cv.getContext('webgl2', { antialias: false, preserveDrawingBuffer: true });
  const F32 = !!gl.getExtension('OES_texture_float_linear');   // half floats band the heights
  const VS = `#version 300 es
  in vec2 p; void main(){ gl_Position = vec4(p,0.,1.); }`;
  const FS = `#version 300 es
  precision highp float;
  uniform sampler2D T0, T1; uniform vec2 grid, res; uniform vec3 L; uniform float Li, time, hmax;
  out vec4 o;
  float hash(vec2 p){ uvec2 q = uvec2(ivec2(floor(p)) + 32768) * uvec2(1597334673u, 3812015801u);
    uint n = (q.x ^ q.y) * 1597334673u; n ^= n >> 16; n *= 2246822519u; n ^= n >> 13; return float(n) * (1.0/4294967296.0); }
  vec2 hash2(vec2 p){ return vec2(hash(p), hash(p+vec2(37.,91.))); }
  float vnoise(vec2 p){ vec2 i=floor(p), f=fract(p); f=f*f*(3.-2.*f);
    return mix(mix(hash(i),hash(i+vec2(1,0)),f.x), mix(hash(i+vec2(0,1)),hash(i+vec2(1,1)),f.x), f.y); }
  float fbm(vec2 p){ float s=0., a=.5; for(int i=0;i<5;i++){ s+=a*vnoise(p); p=p*2.03+vec2(1.7,9.2); a*=.5; } return s; }
  vec4 bicubic(sampler2D t, vec2 c){   // B-spline, 4 bilinear taps: C2-smooth heights
    c -= .5; vec2 i = floor(c), f = c - i, f2 = f*f, f3 = f2*f;
    vec2 w0 = (1.-3.*f+3.*f2-f3)/6., w1 = (4.-6.*f2+3.*f3)/6., w2 = (1.+3.*f+3.*f2-3.*f3)/6., w3 = f3/6.;
    vec2 g0 = w0+w1, g1 = w2+w3, h0 = (w1/g0) - .5 + i, h1 = (w3/g1) + 1.5 + i;
    vec4 a = texture(t, vec2(h0.x,h0.y)/grid), b = texture(t, vec2(h1.x,h0.y)/grid);
    vec4 cc = texture(t, vec2(h0.x,h1.y)/grid), d = texture(t, vec2(h1.x,h1.y)/grid);
    return g0.y*(g0.x*a + g1.x*b) + g1.y*(g0.x*cc + g1.x*d);
  }
  vec3 aces(vec3 x){ return clamp((x*(2.51*x+.03))/(x*(2.43*x+.59)+.14),0.,1.); }
  void main(){
    vec2 uv = gl_FragCoord.xy/res;
    vec2 c = vec2(uv.x, 1.-uv.y)*grid;
    float px = grid.x/res.x;                       // cells per pixel
    vec4 s = bicubic(T0, c);
    float e = .5;
    float h = s.r, hxp = bicubic(T0, c+vec2(e,0)).r, hyp = bicubic(T0, c+vec2(0,e)).r;
    vec4 s1 = texture(T1, c/grid);
    float wet = clamp(s1.r,0.,1.), heap = s1.b, churn = s1.a;
    vec3 n = normalize(vec3(-(hxp-h)/e, -(hyp-h)/e, 1.));

    // --- grains ---
    vec3 P = vec3(c, h);
    vec3 Lv = L - P; float dist = length(Lv); vec3 ld = Lv/dist;
    float gsz = .2;                                 // sand grain, in cells
    vec2 gi = floor(c/gsz); float r0 = hash(gi), r1 = hash(gi+vec2(3,1)), r2 = hash(gi+vec2(7,77));
    vec3 alb = vec3(.56,.47,.36) * (.78 + .44*r0);
    alb = mix(alb, vec3(.24,.19,.15), step(.95, r1));          // dark mineral grains
    alb = mix(alb, vec3(.80,.76,.68), step(.975, r2));          // white
    vec2 mg = (hash2(gi+vec2(5,0))-.5);
    vec3 ng = normalize(n + vec3(mg*.9, 0.));
    float glint = step(.985, hash(gi+vec2(11,3)));
    // army grain: larger domes over the heaps
    float fa = s.g, fb = s.b, fc = s.a, ft = fa+fb+fc;
    float asz = .55;
    vec2 ac = c/asz, ai0 = floor(ac), af0 = fract(ac), ai = ai0, dv = vec2(9.); float dd = 9., rad = .5;
    for (int j=-1;j<=1;j++) for (int i=-1;i<=1;i++) {      // nearest jittered grain: no grid
      vec2 o = vec2(i,j), q = o + hash2(ai0+o+vec2(2,300)) - af0;
      float rr = .42 + .22*hash(ai0+o+vec2(61,7)), d = length(q)/rr;
      if (d < dd) { dd = d; dv = -q; ai = ai0+o; rad = rr; }
    }
    float pick = hash(ai+vec2(5,900)), take = hash(ai+vec2(800,1));
    float rough = 1.;
    if (take < ft && dd < 1.) {
      float u = pick*ft;
      vec3 ca;
      if (u < fa) { ca = mix(vec3(.36,.06,.025), vec3(.52,.13,.04), hash(ai+vec2(1,0))); }        // red ochre
      else if (u < fa+fb) { ca = mix(vec3(.38,.52,.10), vec3(.55,.66,.22), hash(ai+vec2(2,0))); } // pale lime grain
      else { ca = mix(vec3(.045,.03,.04), vec3(.11,.07,.07), hash(ai+vec2(3,0))); rough = .25; } // black beans
      alb = ca * (.85 + .3*hash(gi));
      vec2 dn = dv/rad;
      ng = normalize(n*1.2 + vec3(dn*.9, sqrt(max(0.,1.-dd*dd))*.6));
      glint = 0.;
    } else if (take < ft) {                         // in the gaps between grains: shadowed
      alb *= .45;
    }
    // churned ground: darker, rougher
    alb *= 1. - .25*churn;
    // wet: the river and its banks
    alb *= mix(1., .22, smoothstep(.15,.9,wet));
    float water = smoothstep(.75,.98,wet);

    // --- shadow toward the flame ---
    float sh = 1., t = .8;
    for (int i=0;i<30;i++){
      vec3 q = P + ld*t;
      if (q.z > hmax + .5) break;
      float hh = texture(T0, q.xy/grid).r;
      sh = min(sh, clamp(7.*(q.z-hh)/t + .5, 0., 1.));
      t *= 1.18; t += .25;
    }
    sh = smoothstep(0.,1.,sh);
    // --- occlusion ---
    float ao = clamp(1. - (s1.g - h)*.22, .35, 1.1);
    float microAO = mix(1., .7, smoothstep(.6,1.,dd)*step(take,ft));

    float fall = Li / (.45 + pow(dist/230., 2.4));
    vec3 torch = vec3(1., .52, .21);
    float dif = max(dot(ng, ld), 0.);
    dif = mix(dif, max(dot(n, ld),0.), .35);
    vec3 col = alb * torch * fall * dif * sh * microAO * 1.9;
    // the flame's glints on quartz, seeds and water
    vec3 hv = normalize(ld + vec3(0,0,1));
    float sp = pow(max(dot(ng,hv),0.), rough < .5 ? 60. : 400.) * (rough < .5 ? 1.6 : glint*3.);
    vec2 wc = c*.35 + vec2(time*.4, time*.13);
    vec3 nw = normalize(vec3((vnoise(wc)-.5)*.5 + (vnoise(wc*3.1)-.5)*.25, (vnoise(wc+9.)-.5)*.5 + (vnoise(wc*2.7+3.)-.5)*.25, 1.));
    sp += water * pow(max(dot(nw,hv),0.), 90.) * 4.;
    sp += smoothstep(.2,.7,wet)*(1.-water) * pow(max(dot(ng,hv),0.), 30.) * .25;
    col += torch * fall * sp * sh;
    // moonless sky: a cold whisper
    col += alb * vec3(.018,.024,.040) * ao * (.5+.5*n.z);
    col *= ao;
    // --- dust drifting through the light, thicker at the table's edges and over the low ground ---
    vec2 q = c*.012 + vec2(time*.015, -time*.008);
    float fog = fbm(q + fbm(q*1.7 + time*.02));
    float edge = smoothstep(.32, .78, length((uv-.5)*vec2(1.,.8))*1.3);
    float low = smoothstep(8., 3., h)*.25;
    float fd = clamp((edge*.9 + low) * smoothstep(.3,.8,fog), 0., 1.);
    float fl = Li / (.35 + pow(length(L.xy - c)/320., 2.));
    col = mix(col, torch*.22*fl + vec3(.006,.008,.012), fd*.75);
    col *= 1. - .55*smoothstep(.45,.95,length((uv-.5)*vec2(1.,.85))*1.25);
    col = aces(col*1.0);
    col = pow(col, vec3(1./2.2));
    col += (hash(gl_FragCoord.xy + vec2(floor(time*30.)*7., 0.)) - .5) * .012;
    o = vec4(col, 1.);
  }`;
  function sh(t, src) { const s = gl.createShader(t); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; }
  const pr = gl.createProgram();
  gl.attachShader(pr, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(pr, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(pr);
  if (!gl.getProgramParameter(pr, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(pr));
  gl.useProgram(pr);
  const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  const lp = gl.getAttribLocation(pr, 'p'); gl.enableVertexAttribArray(lp); gl.vertexAttribPointer(lp, 2, gl.FLOAT, false, 0, 0);
  function tex(unit) {
    const t = gl.createTexture(); gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, t);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    return t;
  }
  const t0 = tex(0), t1 = tex(1);
  const U = n => gl.getUniformLocation(pr, n);
  gl.uniform1i(U('T0'), 0); gl.uniform1i(U('T1'), 1); gl.uniform2f(U('grid'), S.W, S.H);

  function resize() {
    const d = window.devicePixelRatio || 1, w = Math.round(innerWidth * d), h = Math.round(innerHeight * d);
    if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h; }
  }
  // the torch, carried slowly round the table's south-west, its flame never still
  function torch(t) {
    const a = 2.5 - 0.04 * t + 0.2 * Math.sin(t * 0.07), cx = S.C[0] - 20, cy = S.C[1] + 10;
    const fl = 1 + 0.07 * Math.sin(t * 13.1) + 0.05 * Math.sin(t * 23.7 + 1) + 0.04 * Math.sin(t * 5.3 + 2) + 0.03 * Math.sin(t * 41 + 0.3);
    return { x: cx + Math.cos(a) * 200 + Math.sin(t * 9.1) * 1.2, y: cy + Math.sin(a) * -160 + Math.sin(t * 7.3) * 1.2, z: 62 + Math.sin(t * 3.1) * 0.8, li: 1.0 * fl };
  }
  function draw() {
    resize();
    const p = S.pack(), t = S.time, L = torch(t);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, t0);
    gl.texImage2D(gl.TEXTURE_2D, 0, F32 ? gl.RGBA32F : gl.RGBA16F, S.W, S.H, 0, gl.RGBA, gl.FLOAT, p.out0);
    gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, t1);
    gl.texImage2D(gl.TEXTURE_2D, 0, F32 ? gl.RGBA32F : gl.RGBA16F, S.W, S.H, 0, gl.RGBA, gl.FLOAT, p.out1);
    gl.viewport(0, 0, cv.width, cv.height);
    gl.uniform2f(U('res'), cv.width, cv.height); gl.uniform3f(U('L'), L.x, L.y, L.z);
    gl.uniform1f(U('Li'), L.li); gl.uniform1f(U('time'), t); gl.uniform1f(U('hmax'), p.hmax);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
  const cell = e => [e.clientX / innerWidth * S.W, e.clientY / innerHeight * S.H];
  cv.addEventListener('pointerdown', e => { cv.setPointerCapture && cv.setPointerCapture(e.pointerId); S.pointer('down', ...cell(e)); });
  cv.addEventListener('pointermove', e => S.pointer('move', ...cell(e)));
  cv.addEventListener('pointerup', e => S.pointer('up', ...cell(e)));
  window.DRAW = draw;
  if (!window.__manual) {
    let last = performance.now();
    const loop = now => { const dt = Math.min(0.05, (now - last) / 1000); last = now; S.step(dt); draw(); requestAnimationFrame(loop); };
    requestAnimationFrame(loop);
  }
})();
