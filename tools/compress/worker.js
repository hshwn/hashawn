// worker.js — runs the codec (WebAssembly) off the main thread, in steps, with progress.
// nczl.wasm is ncz plus the 3M language model as a side input. The model pack (3.3 MB) is fetched
// only when it is needed: compressing text, or unpacking a file that was made with it.
// Also gzips the same bytes with the browser's CompressionStream so the page can show an honest comparison.
let wasm = null, modelId = null;
const STEP_LM = 4096, STEP = 64 * 1024;
const bytes = () => new Uint8Array(wasm.memory.buffer);   // re-read after every call: memory can grow and detach
// errors carry a code the page turns into what to say: unknown (not a file of ours), damaged, network, too-big,
// version (made with a model or version this page does not have)
const fail = (code, message) => Object.assign(new Error(message), { code });
async function get(url) {
  let res; try { res = await fetch(url); } catch { throw fail('network', `could not fetch ${url}: no connection?`); }
  if (!res.ok) throw fail('network', `could not fetch ${url} (${res.status})`);
  return res;
}
const readAll = async (res, onChunk) => {
  const chunks = []; let got = 0; const reader = res.body.getReader();
  try { for (;;) { const { value, done } = await reader.read(); if (done) break; chunks.push(value); got += value.byteLength; onChunk(got); } }
  catch { throw fail('network', 'the download was cut off'); }
  const out = new Uint8Array(got); let off = 0; for (const c of chunks) { out.set(c, off); off += c.byteLength; } return out;
};

// nczl.wasm uses relaxed SIMD (one int8 dot instruction, 2x faster); nczl-base.wasm is plain SIMD for
// browsers without it. Both produce the same bytes.
async function compile(url) {
  const bytes = await (await get(url)).arrayBuffer();
  return (await WebAssembly.instantiate(bytes, {})).instance;
}
async function load() {
  if (wasm) return;
  let instance;
  // ?v: the build with NCZW v6 (CRC) and nczl_w_check / nczl_w_fits, so a cached older build is not paired with this file
  try { instance = await compile('nczl.wasm?v=6'); } catch (e) { if (e.code) throw e; instance = await compile('nczl-base.wasm?v=6'); }
  wasm = instance.exports;
}
async function loadModel(id) {
  if (modelId) return;
  postMessage({ id, type: 'model', done: 0, total: 0 });
  const res = await get('model.slmq');
  const total = Number(res.headers.get('content-length')) || 0;
  const pack = await readAll(res, (got) => postMessage({ id, type: 'model', done: got, total }));
  const ptr = wasm.nczl_alloc(pack.length); bytes().set(pack, ptr);
  modelId = wasm.nczl_load(ptr, pack.length) >>> 0;
  wasm.nczl_free(ptr, pack.length);
  if (!modelId) throw fail('network', 'the language model came down damaged; try again');
}
const put = (u8) => { const p = wasm.nczl_alloc(u8.length); bytes().set(u8, p); return p; };
const takeOut = (ptr) => { const n = wasm.nczl_out_len(); const out = bytes().slice(ptr, ptr + n); wasm.nczl_free(ptr, n); return out; };

async function gzipSize(u8) {
  try {
    const cs = new CompressionStream('gzip');
    const w = cs.writable.getWriter(); w.write(u8); w.close();
    let n = 0; const r = cs.readable.getReader();
    for (;;) { const { value, done } = await r.read(); if (done) break; n += value.byteLength; }
    return n;
  } catch { return null; }
}

// mode (PNGs only): 0 every pixel exact, 1 every sample within +-1, 2 grain mode (looks the same)
async function compress(u8, id, mode) {
  await load();
  let inPtr = put(u8);
  const png = !!wasm.nczl_is_png(inPtr, u8.length);
  const text = !png && wasm.nczl_is_text(inPtr, u8.length);
  if (text) { wasm.nczl_free(inPtr, u8.length); await loadModel(id); inPtr = put(u8); }
  if (png && mode === 2) { postMessage({ id, type: 'progress', done: 0, total: u8.length, png, phase: 'grain' }); await new Promise((r) => setTimeout(r, 0)); }
  const enc = png ? wasm.nczl_enc_new_mode(inPtr, u8.length, mode | 0) : wasm.nczl_enc_new(inPtr, u8.length);   // grain mode: the split happens here
  wasm.nczl_free(inPtr, u8.length);
  const step = png ? 8 : text && modelId ? STEP_LM : STEP;      // PNG: steps are rows of pixels
  let t0 = performance.now(), done = 0;
  for (;;) {
    const n = wasm.nczl_enc_step(enc, step);
    done = wasm.nczl_enc_done(enc);
    const now = performance.now();
    if (now - t0 > 80 || n === 0) { t0 = now; postMessage({ id, type: 'progress', done, total: u8.length, brain: !!(text && modelId), png }); await new Promise((r) => setTimeout(r, 0)); }
    if (n === 0) break;
  }
  return takeOut(wasm.nczl_enc_finish(enc));
}
// what a .ncz gives back: 0 exact (or not a PNG stream), k = within +-k, 255 = grain mode
const pngMode = (u8) => { const p = put(u8); const m = wasm.nczl_png_mode(p, u8.length); wasm.nczl_free(p, u8.length); return m; };

async function decompress(u8, id, wantTex) {
  await load();
  if (u8.length >= 5 && u8[0] === 0x4e && u8[1] === 0x43 && u8[2] === 0x5a && u8[3] === 0x57) { const r = await learnedDecode(u8, id, wantTex); return { out: r.png, mode: r.tex ? 'tiny-tex' : 'tiny' }; }
  let inPtr = put(u8);
  const lm = wasm.nczl_needs_model(inPtr, u8.length);
  if (lm) { wasm.nczl_free(inPtr, u8.length); await loadModel(id); inPtr = put(u8); }
  const dec = wasm.nczl_dec_new(inPtr, u8.length);
  wasm.nczl_free(inPtr, u8.length);
  if (!dec) {                // an NCZL header that is whole but not for this page: another stream version, or another model
    const nczl = u8.length >= 18 && u8[0] === 0x4e && u8[1] === 0x43 && u8[2] === 0x5a && u8[3] === 0x4c;
    if (nczl && u8[4] !== wasm.nczl_version()) throw fail('version', 'made by a different version of this page');
    if (nczl && lm && new DataView(u8.buffer, u8.byteOffset).getUint32(6, true) !== modelId) throw fail('version', 'made with a different language model than this page has');
    throw fail('damaged', 'the file is damaged: its header does not add up');
  }
  const mode = pngMode(u8);
  const total = wasm.nczl_dec_total(dec);           // bytes, or rows for a PNG (only the fraction is shown)
  let t0 = performance.now();
  for (;;) {
    const n = wasm.nczl_dec_step(dec, STEP_LM);
    const done = wasm.nczl_dec_done(dec);
    const now = performance.now();
    if (now - t0 > 80 || n === 0) { t0 = now; postMessage({ id, type: 'progress', done, total }); await new Promise((r) => setTimeout(r, 0)); }
    if (n === 0) break;
  }
  if (mode === 255) { postMessage({ id, type: 'progress', done: total, total, phase: 'grain' }); await new Promise((r) => setTimeout(r, 0)); }
  const ptr = wasm.nczl_dec_finish(dec);         // grain mode: the grain is regrown here
  if (!ptr) throw fail('damaged', 'the file is damaged');
  return { out: takeOut(ptr), mode };
}

// ---------- learned mode ("tiny"): any photo -> a learned encoder's latents -> NCZW; back through our small decoder ----------
// The networks run in ONNX Runtime Web (WebGPU if the browser has it, else WebAssembly); the latents are coded by the
// codec's wasm, whose mean/scale predictor is bit-exact on every machine, so files always decode. Pictures are not
// exact: they look close to the original, at a fraction of the size.
const ORT = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.1/dist/';
const QUALITIES = [1, 2, 4];                                   // the teacher's qualities shipped: 1 smallest, 4 sharpest
const TEX = new Set([1]);                                      // sizes with a texture decoder (files made before swatches)
const SWATCH = new Set([1, 2, 4]);
// sizes coded with the teacher's own entropy model (ncz tm.rs: hyperprior + causal context, mean-centred latents,
// ~17% smaller than the group predictor): their files need the hyperprior encoder (ha), the model (tm) and a decoder
// trained on mean-centred latents (dmc); the predictor (pred) and plain decoder (dec) are only for older files
const TMQ = new Set([1]);                                 // sizes whose files carry swatches (real texture samples)
let ortReady = null; const sessions = {}; const predLoaded = {};
function ortLoad() {
  if (!ortReady) ortReady = (async () => { try { importScripts(ORT + 'ort.webgpu.min.js'); } catch { throw fail('network', 'could not fetch the network runtime: no connection?'); }
    ort.env.wasm.wasmPaths = ORT;
    // the CPU fallback can only use threads when the page is cross-origin isolated
    ort.env.wasm.numThreads = self.crossOriginIsolated ? Math.min(8, navigator.hardwareConcurrency || 4) : 1; })()
    .catch((e) => { ortReady = null; throw e; });         // offline: let the next try fetch it again
  return ortReady;
}
async function fetchProgress(url, id, label, part) {
  const res = await get(url);
  const total = Number(res.headers.get('content-length')) || 0;
  return readAll(res, (got) => postMessage({ id, type: 'model', done: got, total, label, part }));
}
// the network files; '.ncz' ones are packed with our own codec (8-bit weights pack to ~6 bits, 4-bit to ~3) and unpacked here
const file = (name, q) => `learned/q${q}.${name}${name === 'pred' || name === 'tm' ? '' : '.onnx'}.ncz${name === 'tex' ? '?v=7' : ''}`;   // ?v: the texture decoder was replaced (v7)
const LABEL = { enc: 'the encoder', dec: 'the decoder', pred: 'the predictor', tex: 'the texture decoder', ha: 'the hyperprior', tm: 'the entropy model', dmc: 'the decoder' };
// The texture decoder's side input: the original's fine-detail energy per 32 x 32 block (brightness minus its Gaussian
// blur, sigma 1.5, rms; log2 in half steps, -4..15), measured on the picture padded to W x H by repeating its edges.
// Must match bench/wonder/texture2.py's detail_map. Stored packed (ncz) in the file, ~150 B a photo.
function detailMap(rgba, pw, ph, W, H) {
  const L = new Float32Array(W * H);
  for (let y = 0; y < H; y++) { const sy = Math.min(y, ph - 1); for (let x = 0; x < W; x++) { const s = (sy * pw + Math.min(x, pw - 1)) * 4; L[y * W + x] = rgba[s] * 0.299 + rgba[s + 1] * 0.587 + rgba[s + 2] * 0.114; } }
  const g = []; let gs = 0; for (let i = -4; i <= 4; i++) { const v = Math.exp(-i * i / (2 * 1.5 * 1.5)); g.push(v); gs += v; } for (let i = 0; i < 9; i++) g[i] /= gs;
  const refl = (i, n) => (i < 0 ? -i : i >= n ? 2 * n - 2 - i : i);
  const A = new Float32Array(W * H);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { let a = 0; for (let k = 0; k < 9; k++) a += g[k] * L[y * W + refl(x + k - 4, W)]; A[y * W + x] = a; }
  const bh = H / 32, bw = W / 32, out = new Uint8Array(bh * bw), e = new Float64Array(bh * bw);
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) { let b = 0; for (let k = 0; k < 9; k++) b += g[k] * A[refl(y + k - 4, H) * W + x]; const h = L[y * W + x] - b; e[(y >> 5) * bw + (x >> 5)] += h * h; }
  for (let i = 0; i < out.length; i++) { const r = Math.max(Math.sqrt(Math.max(e[i] / 1024, 1e-6)), 0.25); out[i] = Math.max(-4, Math.min(15, Math.round(2 * Math.log2(r)))) + 4; }
  return out;
}
// small byte strings packed / unpacked with the codec's own byte model
function packBytes(u8) { const p = put(u8); const e = wasm.nczl_enc_new(p, u8.length); wasm.nczl_free(p, u8.length); while (wasm.nczl_enc_step(e, 1 << 20) > 0); return takeOut(wasm.nczl_enc_finish(e)); }
function unpackBytes(u8) { const p = put(u8); const d = wasm.nczl_dec_new(p, u8.length); wasm.nczl_free(p, u8.length); if (!d) return null; while (wasm.nczl_dec_step(d, 1 << 20) > 0); const o = wasm.nczl_dec_finish(d); return o ? takeOut(o) : null; }
async function fetchModel(name, q, id) {
  let bytes = await fetchProgress(file(name, q), id, LABEL[name], name);
  {
    await load(); const p = put(bytes); const d = wasm.nczl_dec_new(p, bytes.length); wasm.nczl_free(p, bytes.length);
    if (!d) throw fail('network', `${LABEL[name]} came down damaged; try again`);
    while (wasm.nczl_dec_step(d, 1 << 20) > 0); const o = wasm.nczl_dec_finish(d); if (!o) throw fail('network', `${LABEL[name]} came down damaged; try again`); bytes = takeOut(o);
  }
  return bytes;
}
async function session(name, q, id) {
  const key = `${name}${q}`; if (sessions[key]) return sessions[key];
  await ortLoad();
  const bytes = await fetchModel(name, q, id);
  let s;
  try { s = await ort.InferenceSession.create(bytes, { executionProviders: ['webgpu'] }); s.ep = 'webgpu'; }
  catch { s = await ort.InferenceSession.create(bytes, { executionProviders: ['wasm'] }); s.ep = 'wasm'; }
  return (sessions[key] = s);
}
async function predictor(q, id) {
  if (predLoaded[q]) return;
  const b = await fetchModel('pred', q, id);
  const p = put(b); const ok = wasm.nczl_w_load(p, b.length, q); wasm.nczl_free(p, b.length);
  if (!ok) throw fail('network', 'the predictor came down damaged; try again'); predLoaded[q] = true;
}
const tmLoaded = {};
async function tmLoad(q, id) {
  if (tmLoaded[q]) return;
  const b = await fetchModel('tm', q, id);
  const p = put(b); const ok = wasm.nczl_tm_load(p, b.length, q); wasm.nczl_free(p, b.length);
  if (!ok) throw fail('network', 'the entropy model came down damaged; try again'); tmLoaded[q] = true;
}
// Any size: both networks run in overlapping tiles (core T px, margin M px each side; the margins are cut away), so
// memory stays flat. Cores and margins are multiples of 64 px, on the encoder's grid; a picture up to T px a side is
// one tile, exactly as a single pass.
const T = 1024, M = 128, TL = T / 16, ML = M / 16;
const yieldNow = () => new Promise((r) => setTimeout(r, 0));
// Also gives back the picture as unpacking the file will show it (`wantTex`: textured), when the swatch step painted it
// anyway: the same latents, decoder, grain and swatches as learnedDecode, composed the same way (toPng).
async function learnedEncode(u8, id, q, wantTex) {
  if (!QUALITIES.includes(q)) throw new Error(`no tiny quality ${q}`);
  await load();
  let bmp; try { bmp = await createImageBitmap(new Blob([u8])); } catch { throw fail('damaged', 'could not read that picture: it may be damaged'); }
  const pw = bmp.width, ph = bmp.height;
  // the decoder takes so many latents and no more: refuse here rather than write a file nobody can open
  const tmq = TMQ.has(q); if (tmq) await tmLoad(q, id); else await predictor(q, id);
  if (!wasm.nczl_w_fits(q, pw, ph)) { bmp.close(); throw fail('too-big', `${pw} x ${ph} px is more than a tiny file can hold; try a smaller copy`); }
  const W = Math.ceil(pw / 64) * 64, H = Math.ceil(ph / 64) * 64;
  const cv = new OffscreenCanvas(pw, ph); const cx = cv.getContext('2d'); cx.drawImage(bmp, 0, 0); bmp.close();
  const rgba = cx.getImageData(0, 0, pw, ph).data;
  // tiny has no alpha (every pixel comes back opaque): tell the page when there was transparency to lose
  let alpha = false; for (let i = 3; i < rgba.length; i += 4) if (rgba[i] < 255) { alpha = true; break; }
  // the photo's grain fingerprint (the decoder smooths grain away; it is regrown from this on unpack), on a centre crop
  postMessage({ id, type: 'progress', done: 0, total: 1, phase: 'learn-grain' }); await yieldNow();
  let grain = new Uint8Array(0);
  { const rgb = new Uint8Array(pw * ph * 3); for (let i = 0, j = 0; i < rgb.length; i += 3, j += 4) { rgb[i] = rgba[j]; rgb[i + 1] = rgba[j + 1]; rgb[i + 2] = rgba[j + 2]; }
    const rp = put(rgb); const gp = wasm.nczl_grain_measure(rp, pw, ph, 1024); wasm.nczl_free(rp, rgb.length); if (gp) grain = takeOut(gp); }
  const enc = await session('enc', q, id);
  const tiles = []; for (let ty = 0; ty < H; ty += T) for (let tx = 0; tx < W; tx += T) tiles.push([tx, ty]);
  let lat = null, c = 0; const h = H / 16, w = W / 16;
  for (let k = 0; k < tiles.length; k++) {
    postMessage({ id, type: 'progress', done: k, total: tiles.length + 1, phase: 'learn-enc' }); await yieldNow();
    const [tx, ty] = tiles[k];
    const x0 = Math.max(0, tx - M), y0 = Math.max(0, ty - M), x1 = Math.min(W, tx + T + M), y1 = Math.min(H, ty + T + M), tw = x1 - x0, th = y1 - y0;
    const x = new Float32Array(3 * tw * th);                 // planes; edge pixels repeated into the padding
    for (let yy = 0; yy < th; yy++) { const sy = Math.min(y0 + yy, ph - 1);
      for (let xx = 0; xx < tw; xx++) { const s = (sy * pw + Math.min(x0 + xx, pw - 1)) * 4, d = yy * tw + xx;
        x[d] = rgba[s] / 255; x[tw * th + d] = rgba[s + 1] / 255; x[2 * tw * th + d] = rgba[s + 2] / 255; } }
    const y = (await enc.run({ x: new ort.Tensor('float32', x, [1, 3, th, tw]) })).y;
    const [, cc, lh, lw] = y.dims; if (!lat) { c = cc; lat = tmq ? new Float32Array(c * h * w) : new Int16Array(c * h * w); }
    const oy = (ty - y0) / 16, ox = (tx - x0) / 16, ch_ = Math.min(TL, h - ty / 16), cw_ = Math.min(TL, w - tx / 16);
    for (let q = 0; q < c; q++) for (let yy = 0; yy < ch_; yy++) for (let xx = 0; xx < cw_; xx++) {
      const v = y.data[(q * lh + oy + yy) * lw + ox + xx];
      lat[(q * h + ty / 16 + yy) * w + tx / 16 + xx] = tmq ? v : Math.max(-32767, Math.min(32767, Math.round(v))); }
    y.dispose && y.dispose();
  }
  // the teacher's entropy model: hyperprior z from the float latents, then the latents coded (mean-centred); what the
  // decoder will rebuild, y^, is what the swatches are chosen against
  let tmOut = null, yh = null;
  if (tmq) {
    postMessage({ id, type: 'progress', done: tiles.length, total: tiles.length + 1, phase: 'learn-pack' }); await yieldNow();
    const z = (await (await session('ha', q, id)).run({ y: new ort.Tensor('float32', lat, [1, c, h, w]) })).z;
    const lp = put(new Uint8Array(lat.buffer)), zp = put(new Uint8Array(z.data.buffer, z.data.byteOffset, z.data.byteLength)); z.dispose && z.dispose();
    const gp = grain.length ? put(grain) : 0;
    const tp = wasm.nczl_w_enc_tm(lp, c, h, w, zp, pw, ph, q, gp, grain.length, 0, 0, 0, 0);
    wasm.nczl_free(lp, lat.length * 4); wasm.nczl_free(zp, c * h * w / 4); if (gp) wasm.nczl_free(gp, grain.length);
    if (!tp) throw new Error('could not pack the latents');
    tmOut = takeOut(tp); const yb = takeOut(wasm.nczl_w_yhat()); yh = new Float32Array(yb.buffer, yb.byteOffset, yb.length / 4);
  }
  // swatches: the picture as unpack will show it (decoded, grain regrown), then which real texture goes where
  let sw = new Uint8Array(0), d = null;
  if (SWATCH.has(q)) {
    d = tmq ? await paint(yh, c, h, w, pw, ph, await session('dmc', q, id), id, null) : await paint(lat, c, h, w, pw, ph, await session('dec', q, id), id, null);
    regrow(d, pw, ph, grain.length ? grain : null, id);
    postMessage({ id, type: 'progress', done: 0, total: 1, phase: 'learn-swatch' }); await yieldNow();
    const o = rgbOf(rgba), r = rgbOf(d); const op = put(o), rp = put(r);
    const sp = wasm.nczl_sw_enc(op, rp, pw, ph); wasm.nczl_free(op, o.length); wasm.nczl_free(rp, r.length); if (sp) sw = takeOut(sp);
  }
  let out;
  if (tmq) {                  // the swatch section goes into the finished stream
    if (sw.length) { const p = put(tmOut), sp = put(sw); const r = wasm.nczl_w_set_swatch(p, tmOut.length, sp, sw.length); wasm.nczl_free(p, tmOut.length); wasm.nczl_free(sp, sw.length);
      if (!r) throw new Error('could not pack the latents'); out = takeOut(r); } else out = tmOut;
  } else {
    postMessage({ id, type: 'progress', done: tiles.length, total: tiles.length + 1, phase: 'learn-pack' }); await yieldNow();
    const lp = put(new Uint8Array(lat.buffer)); const gp = grain.length ? put(grain) : 0; const sp = sw.length ? put(sw) : 0;
    const zp = wasm.nczl_w_enc_sw(lp, c, h, w, pw, ph, q, gp, grain.length, 0, 0, sp, sw.length); wasm.nczl_free(lp, lat.length * 2);
    if (gp) wasm.nczl_free(gp, grain.length); if (sp) wasm.nczl_free(sp, sw.length);
    if (!zp) throw new Error('could not pack the latents');
    out = takeOut(zp);
  }
  const pic = d ? await toPng(d, pw, ph, wantTex && sw.length ? sw : null, id) : null;
  return { out, ep: enc.ep, alpha, pic, picTex: !!(pic && wantTex && sw.length) };
}
// The texture decoder draws its texture from noise (input z: 9 channels at full resolution). Standard-normal-ish
// values from a hash of the picture position (sum of 4 hash bytes, mean 0, sd 1), so tiles agree where they overlap
// and the same file always unpacks to the same picture. Matches randn in PSNR and grid measures (bench texparity).
function texNoise(px0, py0, PW, PH) {
  const z = new Float32Array(9 * PW * PH); let i = 0;
  for (let c = 0; c < 9; c++) { const hc = Math.imul(c, 0xC2B2AE3D);
    for (let y = 0; y < PH; y++) { const hy = Math.imul(py0 + y, 0x85EBCA77) ^ hc;
      for (let x = 0; x < PW; x++) {
        let h = Math.imul(px0 + x, 0x9E3779B1) ^ hy;
        h ^= h >>> 16; h = Math.imul(h, 0x7feb352d); h ^= h >>> 15; h = Math.imul(h, 0x846ca68b); h ^= h >>> 16;
        z[i++] = ((h & 255) + ((h >>> 8) & 255) + ((h >>> 16) & 255) + (h >>> 24) - 510) / 147.8; } } }
  return z;
}
const rgbOf = (rgba) => { const rgb = new Uint8Array(rgba.length / 4 * 3); for (let i = 0, j = 0; i < rgb.length; i += 3, j += 4) { rgb[i] = rgba[j]; rgb[i + 1] = rgba[j + 1]; rgb[i + 2] = rgba[j + 2]; } return rgb; };
const intoRgba = (d, rgb) => { for (let i = 0, j = 0; i < rgb.length; i += 3, j += 4) { d[j] = rgb[i]; d[j + 1] = rgb[i + 1]; d[j + 2] = rgb[i + 2]; } };
// latents -> RGBA picture, through decoder `dec` in tiles; `mraw`: the detail map, for the texture decoder
async function paint(lat, c, h, w, pw, ph, dec, id, mraw) {
  const d = new Uint8ClampedArray(pw * ph * 4);
  const tiles = []; for (let ty = 0; ty < h; ty += TL) for (let tx = 0; tx < w; tx += TL) tiles.push([tx, ty]);
  for (let k = 0; k < tiles.length; k++) {
    postMessage({ id, type: 'progress', done: k + 1, total: tiles.length + 1, phase: 'learn-dec' }); await yieldNow();
    const [tx, ty] = tiles[k];
    const x0 = Math.max(0, tx - ML), y0 = Math.max(0, ty - ML), x1 = Math.min(w, tx + TL + ML), y1 = Math.min(h, ty + TL + ML), tw = x1 - x0, th = y1 - y0;
    const yf = new Float32Array(c * tw * th);
    for (let q = 0; q < c; q++) for (let yy = 0; yy < th; yy++) for (let xx = 0; xx < tw; xx++) yf[(q * th + yy) * tw + xx] = lat[(q * h + y0 + yy) * w + x0 + xx];
    const feeds = { y: new ort.Tensor('float32', yf, [1, c, th, tw]) };
    if (mraw) {                // the detail map under this tile (2 latents a block)
      const mw = w / 2, mf = new Float32Array((th / 2) * (tw / 2));
      for (let yy = 0; yy < th / 2; yy++) for (let xx = 0; xx < tw / 2; xx++) mf[yy * (tw / 2) + xx] = mraw[(y0 / 2 + yy) * mw + x0 / 2 + xx] - 4;
      feeds.m = new ort.Tensor('float32', mf, [1, 1, th / 2, tw / 2]);
      if (dec.inputNames.includes('z')) feeds.z = new ort.Tensor('float32', texNoise(x0 * 16, y0 * 16, tw * 16, th * 16), [1, 9, th * 16, tw * 16]);
    }
    const xo = (await dec.run(feeds)).x; const PW = tw * 16, PH = th * 16, P = PW * PH;
    const px0 = tx * 16, py0 = ty * 16, px1 = Math.min(pw, (tx + TL) * 16), py1 = Math.min(ph, (ty + TL) * 16);
    for (let yy = py0; yy < py1; yy++) for (let xx = px0; xx < px1; xx++) {
      const s = (yy - y0 * 16) * PW + (xx - x0 * 16), o = (yy * pw + xx) * 4;
      d[o] = xo.data[s] * 255 + 0.5; d[o + 1] = xo.data[P + s] * 255 + 0.5; d[o + 2] = xo.data[2 * P + s] * 255 + 0.5; d[o + 3] = 255; }
    xo.dispose && xo.dispose();
  }
  return d;
}
// regrow the photo's grain on a decoded RGBA picture, in place
function regrow(d, pw, ph, grain, id) {
  if (!grain) return;
  postMessage({ id, type: 'progress', done: 1, total: 1, phase: 'learn-grain-back' });
  const rgb = rgbOf(d); const rp = put(rgb), gp = put(grain);
  if (wasm.nczl_grain_apply(rp, pw, ph, gp, grain.length)) intoRgba(d, bytes().subarray(rp, rp + rgb.length));
  wasm.nczl_free(rp, rgb.length); wasm.nczl_free(gp, grain.length);
}
async function learnedDecode(u8, id, wantTex) {
  const cp = put(u8); const ck = wasm.nczl_w_check(cp, u8.length); wasm.nczl_free(cp, u8.length);
  if (ck === 1) throw fail('unknown', 'not a file this page makes');
  if (ck === 2) throw fail('version', 'made by a newer version of this page: reload it');
  if (ck === 3) throw fail('damaged', 'the file is damaged or cut short');
  const hn = Math.min(u8.length, 64); const pp = put(u8.subarray(0, hn)); const q = wasm.nczl_is_nczw(pp, hn) & 255; wasm.nczl_free(pp, hn);   // the header check wants >= 18 bytes from v6 on
  if (!QUALITIES.includes(q)) throw fail('version', 'made with a tiny size this page does not have');
  const tp0 = put(u8); const tmf = !!wasm.nczl_w_uses_tm(tp0, u8.length); wasm.nczl_free(tp0, u8.length);
  if (tmf) await tmLoad(q, id); else await predictor(q, id);
  // textured, when asked for: the file's swatches (real texture samples) laid on the decoded picture; older files
  // carry a detail map instead, for the texture decoder
  const mq0 = put(u8); const sq = wasm.nczl_w_swatch(mq0, u8.length); const sw = sq ? takeOut(sq) : null;
  const mq = sw ? 0 : wasm.nczl_w_dmap(mq0, u8.length); wasm.nczl_free(mq0, u8.length);
  const mraw = mq ? unpackBytes(takeOut(mq)) : null; const texDec = !!(wantTex && TEX.has(q) && mraw); const swt = !!(wantTex && sw);
  const dec = await session(tmf ? 'dmc' : texDec ? 'tex' : 'dec', q, id);
  postMessage({ id, type: 'progress', done: 0, total: 2, phase: 'learn-unpack' }); await new Promise((r) => setTimeout(r, 0));
  const p = put(u8);
  if (!(tmf ? wasm.nczl_w_tm_match(p, u8.length) : wasm.nczl_w_match(p, u8.length))) { wasm.nczl_free(p, u8.length); throw fail('version', `made with a different version of tiny mode (another ${tmf ? 'entropy model' : 'predictor'})`); }
  const gq = wasm.nczl_w_grain(p, u8.length); const grain = gq ? takeOut(gq) : null;
  const lp = tmf ? wasm.nczl_w_dec_tm(p, u8.length) : wasm.nczl_w_dec(p, u8.length); wasm.nczl_free(p, u8.length);
  if (!lp) throw fail('damaged', 'the file is damaged: its latents do not decode');
  const raw = takeOut(lp); const lat = tmf ? new Float32Array(raw.buffer, raw.byteOffset, raw.length / 4) : new Int16Array(raw.buffer, raw.byteOffset, raw.length / 2);
  const [c, h, w, pw, ph] = [0, 1, 2, 3, 4].map((i) => wasm.nczl_w_dim(i));
  const d = await paint(lat, c, h, w, pw, ph, dec, id, texDec ? mraw : null);
  regrow(d, pw, ph, grain, id);
  return { png: await toPng(d, pw, ph, swt ? sw : null, id), tex: texDec || swt };
}
// the last steps of unpacking (the encoder's preview takes them too): the real texture laid when `sw`, then a PNG
async function toPng(d, pw, ph, sw, id) {
  if (sw) {
    postMessage({ id, type: 'progress', done: 1, total: 1, phase: 'learn-swatch-back' }); await yieldNow();
    const rgb = rgbOf(d); const rp = put(rgb), sp = put(sw);
    if (wasm.nczl_sw_apply(rp, pw, ph, sp, sw.length)) intoRgba(d, bytes().subarray(rp, rp + rgb.length));
    wasm.nczl_free(rp, rgb.length); wasm.nczl_free(sp, sw.length);
  }
  const cv = new OffscreenCanvas(pw, ph); cv.getContext('2d').putImageData(new ImageData(d, pw, ph), 0, 0);
  return new Uint8Array(await (await cv.convertToBlob({ type: 'image/png' })).arrayBuffer());
}
const isImage = (u8) => (u8[0] === 0x89 && u8[1] === 0x50 && u8[2] === 0x4e && u8[3] === 0x47) || (u8[0] === 0xff && u8[1] === 0xd8 && u8[2] === 0xff)
  || (u8[0] === 0x52 && u8[1] === 0x49 && u8[2] === 0x46 && u8[3] === 0x46 && u8[8] === 0x57 && u8[9] === 0x45 && u8[10] === 0x42 && u8[11] === 0x50);

// segment plan for big inputs: the split comes from the same Rust function the native build uses
async function plan(u8) {
  await load();
  const max = Math.ceil(u8.length / 1024) + 2;
  const p = put(u8), out = wasm.nczl_alloc(4 * max);
  const n = wasm.nczl_split(p, u8.length, out, max);
  const ends = Array.from(new Uint32Array(wasm.memory.buffer, out, n));
  wasm.nczl_free(p, u8.length); wasm.nczl_free(out, 4 * max);
  return { ends, seg: wasm.nczl_seg_size(), version: wasm.nczl_version(), modeSeg: wasm.nczl_mode_seg() };
}

onmessage = async (e) => {
  const { id, type, buf, gz: wantGz = true, mode = 0 } = e.data;
  const u8 = new Uint8Array(buf);
  try {
    if (type === 'plan') {
      postMessage({ id, type: 'done', ...(await plan(u8)) });
    } else if (type === 'mem') {     // diagnostics: how big the codec's wasm memory has grown
      postMessage({ id, type: 'done', wasmBytes: wasm ? wasm.memory.buffer.byteLength : 0 });
    } else if (type === 'gzip') {
      postMessage({ id, type: 'done', gz: await gzipSize(u8) });
    } else if (type === 'learned') {
      if (!isImage(u8)) throw new Error('tiny mode takes photos (PNG, JPEG, WebP)');
      const t = performance.now(); const { out, ep, alpha, pic, picTex } = await learnedEncode(u8, id, e.data.q || 4, !!e.data.tex);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t, gz: null, pngMode: 'tiny', ep, alpha, pic: pic && pic.buffer, picTex },
        pic ? [out.buffer, pic.buffer] : [out.buffer]);
    } else if (type === 'compress') {
      const t = performance.now();
      const out = await compress(u8, id, mode);
      const ms = performance.now() - t;
      const pm = pngMode(out);
      const isPng = u8.length > 8 && u8[0] === 0x89 && u8[1] === 0x50 && u8[2] === 0x4e && u8[3] === 0x47;
      const gz = wantGz && !isPng ? await gzipSize(u8) : null;          // gzip of a PNG says nothing: it is already deflated
      postMessage({ id, type: 'done', buf: out.buffer, ms, gz, pngMode: pm }, [out.buffer]);
    } else {
      const t = performance.now();
      const { out, mode: pm } = await decompress(u8, id, !!e.data.tex);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t, pngMode: pm }, [out.buffer]);
    }
  } catch (err) { postMessage({ id, type: 'error', code: err && err.code, message: String(err && err.message || err) }); }
};
