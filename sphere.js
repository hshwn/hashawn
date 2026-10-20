// sphere.js — his face on a ball. Raw WebGL2, one sphere, five 2k textures.
// Screen-space API: the ball lives at (x, y) px with radius r px on a fixed full-window canvas.
window.Face = (() => {
  const VS = `#version 300 es
  precision highp float;
  in vec3 aPos; in vec2 aUV;
  uniform mat3 uRot; uniform vec2 uViewport; uniform vec2 uCenter; uniform float uRadius; uniform float uSquash;
  out vec3 vN; out vec3 vT; out vec3 vB; out vec2 vUV;
  void main() {
    float lon = (aUV.x - 0.5) * 6.28318530718;
    float lat = (0.5 - aUV.y) * 3.14159265359;
    vec3 T = vec3(cos(lon), 0.0, -sin(lon));
    vec3 B = vec3(-sin(lon) * sin(lat), cos(lat), -cos(lon) * sin(lat));
    vec3 p = uRot * aPos;
    vN = uRot * aPos; vT = uRot * T; vB = uRot * B; vUV = aUV;
    p.x *= 1.0 + uSquash; p.y *= 1.0 - uSquash;
    vec2 px = uCenter + vec2(p.x, -p.y) * uRadius;
    gl_Position = vec4(px.x / uViewport.x * 2.0 - 1.0, 1.0 - px.y / uViewport.y * 2.0, -p.z * 0.5, 1.0);
  }`;
  const FS = `#version 300 es
  precision highp float;
  in vec3 vN; in vec3 vT; in vec3 vB; in vec2 vUV;
  uniform sampler2D uAlbedo; uniform sampler2D uNormal;
  out vec4 o;
  vec3 aces(vec3 x) { return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0); }
  void main() {
    vec3 alb = texture(uAlbedo, vUV).rgb;
    vec3 nm = texture(uNormal, vUV).rgb * 2.0 - 1.0; nm.xy *= 0.7;
    vec3 N = normalize(normalize(vT) * nm.x + normalize(vB) * nm.y + normalize(vN) * nm.z);
    vec3 V = vec3(0.0, 0.0, 1.0);
    vec3 L1 = normalize(vec3(-2.5, 3.0, 4.0)); vec3 C1 = vec3(1.0, 0.957, 0.918) * 0.78;
    vec3 L2 = normalize(vec3(3.0, 1.0, -3.0)); vec3 C2 = vec3(1.0, 0.706, 0.541) * 0.45;
    vec3 sky = vec3(1.0, 0.945, 0.902), ground = vec3(0.165, 0.094, 0.063);
    vec3 hemi = mix(ground, sky, N.y * 0.5 + 0.5) * 0.3;
    float d1 = max(dot(N, L1), 0.0), d2 = max(dot(N, L2), 0.0);
    vec3 H = normalize(L1 + V);
    float spec = pow(max(dot(N, H), 0.0), 28.0) * 0.14 * d1;
    float fres = pow(1.0 - max(dot(N, V), 0.0), 3.0);
    vec3 sheen = vec3(1.0, 0.76, 0.66) * fres * 0.22;
    vec3 col = alb * (hemi + C1 * d1 + C2 * d2) + spec + sheen * (hemi + C1 * d1);
    col = aces(col * 1.15);
    col = pow(col, vec3(1.0 / 2.2));
    o = vec4(col, 1.0);
  }`;

  const sphere = (su, sv) => {
    const pos = [], uv = [], idx = [];
    for (let j = 0; j <= sv; j++) for (let i = 0; i <= su; i++) {
      const u = i / su, v = j / sv, lon = (u - 0.5) * Math.PI * 2, lat = (0.5 - v) * Math.PI;
      pos.push(Math.sin(lon) * Math.cos(lat), Math.sin(lat), Math.cos(lon) * Math.cos(lat)); uv.push(u, v);
    }
    const at = (i, j) => j * (su + 1) + i;
    for (let j = 0; j < sv; j++) for (let i = 0; i < su; i++) {
      const a = at(i, j), b = at(i + 1, j), c = at(i + 1, j + 1), d = at(i, j + 1);
      if (j !== 0) idx.push(a, c, b);
      if (j !== sv - 1) idx.push(a, d, c);
    }
    return { pos: new Float32Array(pos), uv: new Float32Array(uv), idx: new Uint16Array(idx) };
  };

  const mul = (A, B) => { const M = new Array(9); for (let c = 0; c < 3; c++) for (let r = 0; r < 3; r++) M[c * 3 + r] = A[r] * B[c * 3] + A[3 + r] * B[c * 3 + 1] + A[6 + r] * B[c * 3 + 2]; return M; };
  // quaternions [w, x, y, z]: always a pure rotation after normalising
  const qI = () => [1, 0, 0, 0];
  const qAxis = (ax, ay, az, a) => { const s = Math.sin(a / 2); return [Math.cos(a / 2), ax * s, ay * s, az * s]; };
  const qMul = (a, b) => [
    a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
    a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
    a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
    a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0]];
  const qNorm = (q) => { const n = Math.hypot(q[0], q[1], q[2], q[3]) || 1; const o = q.map((v) => v / n); return o[0] < 0 ? o.map((v) => -v) : o; };
  const qToMat = ([w, x, y, z]) => [
    1 - 2 * (y * y + z * z), 2 * (x * y + w * z), 2 * (x * z - w * y),
    2 * (x * y - w * z), 1 - 2 * (x * x + z * z), 2 * (y * z + w * x),
    2 * (x * z + w * y), 2 * (y * z - w * x), 1 - 2 * (x * x + y * y)];

  const create = (canvas, urls, onReady) => {
    const gl = canvas.getContext('webgl2', { antialias: true, alpha: true, premultipliedAlpha: true });
    if (!gl) return null;
    const sh = (t, s) => { const o = gl.createShader(t); gl.shaderSource(o, s); gl.compileShader(o); if (!gl.getShaderParameter(o, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(o)); return o; };
    const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
    gl.useProgram(prog);
    const U = {}; for (const n of ['uRot', 'uViewport', 'uCenter', 'uRadius', 'uSquash', 'uAlbedo', 'uNormal']) U[n] = gl.getUniformLocation(prog, n);

    const g = sphere(144, 72);
    const vao = gl.createVertexArray(); gl.bindVertexArray(vao);
    const buf = (data, loc, size) => { const b = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, b); gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW); gl.enableVertexAttribArray(loc); gl.vertexAttribPointer(loc, size, gl.FLOAT, false, 0, 0); };
    buf(g.pos, gl.getAttribLocation(prog, 'aPos'), 3); buf(g.uv, gl.getAttribLocation(prog, 'aUV'), 2);
    const ib = gl.createBuffer(); gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, ib); gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, g.idx, gl.STATIC_DRAW);
    gl.enable(gl.DEPTH_TEST); gl.enable(gl.CULL_FACE); gl.cullFace(gl.BACK);

    const aniso = gl.getExtension('EXT_texture_filter_anisotropic');
    const tex = {};
    const load = (key, url, srgb) => new Promise((res) => {
      const im = new Image(); im.onload = () => {
        const t = gl.createTexture(); gl.bindTexture(gl.TEXTURE_2D, t);
        gl.pixelStorei(gl.UNPACK_COLORSPACE_CONVERSION_WEBGL, gl.NONE);
        gl.texImage2D(gl.TEXTURE_2D, 0, srgb ? gl.SRGB8_ALPHA8 : gl.RGB8, srgb ? gl.RGBA : gl.RGB, gl.UNSIGNED_BYTE, im);
        gl.generateMipmap(gl.TEXTURE_2D);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
        if (aniso) gl.texParameterf(gl.TEXTURE_2D, aniso.TEXTURE_MAX_ANISOTROPY_EXT, Math.min(8, gl.getParameter(aniso.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
        tex[key] = t; res();
      }; im.src = url;
    });

    const F = {
      // screen-space pose (springs toward targets)
      x: 0, y: 0, r: 100, tx: 0, ty: 0, tr: 100, vx: 0, vy: 0, vr: 0,
      q: qI(), roll: qI(), moving: false,              // q = how you spun him, roll = the trip's planned turn
      w: [0, 0, 0], dizzy: false, held: false, lastDrag: 0,      // w = angular velocity, rad/s
      setRoll(ax, ay, angle) { F.roll = angle ? qAxis(ax, ay, 0, angle) : qI(); },
      squash: 0, squashV: 0,
      eyesShut: false, mouthOpen: false, ready: false,
      snap(x, y, r) { F.x = F.tx = x; F.y = F.ty = y; F.r = F.tr = r; F.vx = F.vy = F.vr = 0; },
      target(x, y, r) { F.tx = x; F.ty = y; F.tr = r; },
      grab() { F.held = true; F.w = [0, 0, 0]; F.lastDrag = performance.now(); },
      drag(dx, dy) {                                   // trackball: the surface follows the finger 1:1
        const d = Math.hypot(dx, dy); if (d < 0.01) return;
        const now = performance.now(), dt = Math.max(0.004, (now - F.lastDrag) / 1000); F.lastDrag = now;
        const angle = d / F.r;
        F.q = qNorm(qMul(qAxis(dy / d, dx / d, 0, angle), F.q));
        const sp = angle / dt; F.w = [dy / d * sp, dx / d * sp, 0];
      },
      release() {
        F.held = false;
        if (performance.now() - F.lastDrag > 80) F.w = [0, 0, 0];   // stopped before letting go: no fling
        F.dizzy = Math.hypot(...F.w) > 9;
      },
      poke() { F.squashV = -1.6; },
    };

    let last = performance.now();
    const spring = (p, v, t, dt, k = 220, c = 26) => { const a = (t - p) * k - v * c; v += a * dt; p += v * dt; return [p, v]; };
    const step = (now) => {
      const dt = Math.min(0.05, (now - last) / 1000); last = now;
      [F.x, F.vx] = spring(F.x, F.vx, F.tx, dt, 160, 22);
      [F.y, F.vy] = spring(F.y, F.vy, F.ty, dt, 160, 22);
      [F.r, F.vr] = spring(F.r, F.vr, F.tr, dt, 160, 22);
      // the roll itself is planned by the page (setRoll): a whole turn across each trip
      if (!F.held) {
        // a ball in space: spins freely, slows with friction, and just happens to stop
        // with the face at least peeking out. no spring, no uprighting.
        const M = qToMat(F.q);
        const f = [M[6], M[7], M[8]];
        const th = Math.acos(Math.max(-1, Math.min(1, f[2])));          // 0 = face dead on, π = hidden
        const VIS = 1.15, CRAWL = 0.9;                                   // ~66° cone counts as visible
        // one smooth friction curve, never a brake
        const damp = Math.pow(0.962, dt * 60);
        F.w[0] *= damp; F.w[1] *= damp; F.w[2] *= damp;
        let sp = Math.hypot(F.w[0], F.w[1], F.w[2]);
        if (th > VIS && sp < CRAWL) {
          // hidden: don't let him stall, keep drifting round at a crawl. same axis if that brings
          // the face out; otherwise ease the axis toward the one that does
          const n = Math.hypot(f[1], f[0]) || 1, tw = [f[1] / n, -f[0] / n, 0];
          let ax = sp > 0.05 ? [F.w[0] / sp, F.w[1] / sp, F.w[2] / sp] : tw;
          const dot = ax[0] * tw[0] + ax[1] * tw[1];
          if (dot < 0.3) { ax = [ax[0] * 0.9 + tw[0] * 0.1, ax[1] * 0.9 + tw[1] * 0.1, ax[2] * 0.9]; const l = Math.hypot(...ax) || 1; ax = ax.map((v) => v / l); }
          sp = Math.max(sp, CRAWL); F.w = [ax[0] * sp, ax[1] * sp, ax[2] * sp];
        }
        if (sp > 1e-4) F.q = qNorm(qMul(qAxis(F.w[0] / sp, F.w[1] / sp, F.w[2] / sp, sp * dt), F.q));
        if (sp < 2) F.dizzy = false;
      }
      [F.squash, F.squashV] = spring(F.squash, F.squashV, 0, dt, 300, 16);

      const w = canvas.clientWidth, h = canvas.clientHeight, dpr = Math.min(devicePixelRatio || 1, 3);
      if (canvas.width !== Math.round(w * dpr) || canvas.height !== Math.round(h * dpr)) { canvas.width = Math.round(w * dpr); canvas.height = Math.round(h * dpr); }
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      if (F.ready) {
        const key = `${F.eyesShut || F.dizzy ? 'shut' : 'open'}_${F.mouthOpen ? 'open' : 'closed'}`;
        gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex[key]); gl.uniform1i(U.uAlbedo, 0);
        gl.activeTexture(gl.TEXTURE1); gl.bindTexture(gl.TEXTURE_2D, tex.normal); gl.uniform1i(U.uNormal, 1);
        gl.uniformMatrix3fv(U.uRot, false, new Float32Array(mul(qToMat(F.roll), qToMat(F.q))));
        gl.uniform2f(U.uViewport, w, h); gl.uniform2f(U.uCenter, F.x, F.y); gl.uniform1f(U.uRadius, F.r); gl.uniform1f(U.uSquash, F.squash);
        gl.drawElements(gl.TRIANGLES, g.idx.length, gl.UNSIGNED_SHORT, 0);
      }
      requestAnimationFrame(step);
    };
    requestAnimationFrame(step);

    // blink, with the odd double blink
    const blink = () => setTimeout(() => {
      F.eyesShut = true;
      setTimeout(() => {
        F.eyesShut = false;
        if (Math.random() < 0.18) setTimeout(() => { F.eyesShut = true; setTimeout(() => { F.eyesShut = false; }, 110); }, 160);
        blink();
      }, 120 + Math.random() * 60);
    }, 1800 + Math.random() * 4200);
    blink();

    Promise.all([
      load('open_closed', urls.open_closed, true), load('open_open', urls.open_open, true),
      load('shut_closed', urls.shut_closed, true), load('shut_open', urls.shut_open, true),
      load('normal', urls.normal, false),
    ]).then(() => { F.ready = true; onReady && onReady(); });
    return F;
  };
  return { create };
})();
