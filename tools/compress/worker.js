// worker.js — runs the codec (WebAssembly) off the main thread, in steps, with progress.
// nczl.wasm is ncz plus the 3M language model as a side input. The model pack (3.3 MB) is fetched
// only when it is needed: compressing text, or unpacking a file that was made with it.
// Also gzips the same bytes with the browser's CompressionStream so the page can show an honest comparison.
let wasm = null, modelId = null;
const STEP_LM = 4096, STEP = 64 * 1024;
const bytes = () => new Uint8Array(wasm.memory.buffer);   // re-read after every call: memory can grow and detach

// nczl.wasm uses relaxed SIMD (one int8 dot instruction, 2x faster); nczl-base.wasm is plain SIMD for
// browsers without it. Both produce the same bytes.
async function compile(url) {
  const bytes = await (await fetch(url)).arrayBuffer();
  return (await WebAssembly.instantiate(bytes, {})).instance;
}
async function load() {
  if (wasm) return;
  let instance;
  try { instance = await compile('nczl.wasm'); } catch { instance = await compile('nczl-base.wasm'); }
  wasm = instance.exports;
}
async function loadModel(id) {
  if (modelId) return;
  postMessage({ id, type: 'model', done: 0, total: 0 });
  const res = await fetch('model.slmq');
  const total = Number(res.headers.get('content-length')) || 0;
  const chunks = []; let got = 0; const reader = res.body.getReader();
  for (;;) { const { value, done } = await reader.read(); if (done) break; chunks.push(value); got += value.byteLength; postMessage({ id, type: 'model', done: got, total }); }
  const pack = new Uint8Array(got); let off = 0; for (const c of chunks) { pack.set(c, off); off += c.byteLength; }
  const ptr = wasm.nczl_alloc(pack.length); bytes().set(pack, ptr);
  modelId = wasm.nczl_load(ptr, pack.length) >>> 0;
  wasm.nczl_free(ptr, pack.length);
  if (!modelId) throw new Error('the model pack did not load');
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

async function decompress(u8, id) {
  await load();
  if (u8.length > 10 && u8[0] === 0x4e && u8[1] === 0x43 && u8[2] === 0x5a && u8[3] === 0x57) return { out: await learnedDecode(u8, id), mode: 'tiny' };
  let inPtr = put(u8);
  if (wasm.nczl_needs_model(inPtr, u8.length)) { wasm.nczl_free(inPtr, u8.length); await loadModel(id); inPtr = put(u8); }
  const dec = wasm.nczl_dec_new(inPtr, u8.length);
  wasm.nczl_free(inPtr, u8.length);
  if (!dec) throw new Error('not one of mine, or made with a different model');
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
  if (!ptr) throw new Error('the file is damaged');
  return { out: takeOut(ptr), mode };
}

// ---------- learned mode ("tiny"): any photo -> a learned encoder's latents -> NCZW; back through our small decoder ----------
// The networks run in ONNX Runtime Web (WebGPU if the browser has it, else WebAssembly); the latents are coded by the
// codec's wasm, whose mean/scale predictor is bit-exact on every machine, so files always decode. Pictures are not
// exact: they look close to the original, at a fraction of the size.
const ORT = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.1/dist/';
const LQ = 4;                                                  // the one quality shipped so far
let ortReady = null; const sessions = {}; const predLoaded = {};
function ortLoad() {
  if (!ortReady) ortReady = (async () => { importScripts(ORT + 'ort.webgpu.min.js'); ort.env.wasm.wasmPaths = ORT;
    // the CPU fallback can only use threads when the page is cross-origin isolated
    ort.env.wasm.numThreads = self.crossOriginIsolated ? Math.min(8, navigator.hardwareConcurrency || 4) : 1; })();
  return ortReady;
}
async function fetchProgress(url, id, label, part) {
  const res = await fetch(url); if (!res.ok) throw new Error(`could not fetch ${url}`);
  const total = Number(res.headers.get('content-length')) || 0; const chunks = []; let got = 0; const reader = res.body.getReader();
  for (;;) { const { value, done } = await reader.read(); if (done) break; chunks.push(value); got += value.byteLength; postMessage({ id, type: 'model', done: got, total, label, part }); }
  const out = new Uint8Array(got); let off = 0; for (const c of chunks) { out.set(c, off); off += c.byteLength; } return out;
}
async function session(name, id) {
  if (sessions[name]) return sessions[name];
  await ortLoad();
  const bytes = await fetchProgress(`learned/q${LQ}.${name}.onnx`, id, name === 'enc' ? 'the encoder' : 'the decoder', name);
  let s;
  try { s = await ort.InferenceSession.create(bytes, { executionProviders: ['webgpu'] }); s.ep = 'webgpu'; }
  catch { s = await ort.InferenceSession.create(bytes, { executionProviders: ['wasm'] }); s.ep = 'wasm'; }
  return (sessions[name] = s);
}
async function predictor(q, id) {
  if (predLoaded[q]) return;
  const b = await fetchProgress(`learned/q${q}.pred`, id, 'the predictor', 'pred');
  const p = put(b); const ok = wasm.nczl_w_load(p, b.length, q); wasm.nczl_free(p, b.length);
  if (!ok) throw new Error('the predictor did not load'); predLoaded[q] = true;
}
// Any size: both networks run in overlapping tiles (core T px, margin M px each side; the margins are cut away), so
// memory stays flat. Cores and margins are multiples of 64 px, on the encoder's grid; a picture up to T px a side is
// one tile, exactly as a single pass.
const T = 1024, M = 128, TL = T / 16, ML = M / 16;
const yieldNow = () => new Promise((r) => setTimeout(r, 0));
async function learnedEncode(u8, id) {
  await load();
  const bmp = await createImageBitmap(new Blob([u8]));
  const pw = bmp.width, ph = bmp.height;
  if (pw > 65535 || ph > 65535) throw new Error('that picture is wider or taller than 65535 pixels');
  const W = Math.ceil(pw / 64) * 64, H = Math.ceil(ph / 64) * 64;
  const cv = new OffscreenCanvas(pw, ph); const cx = cv.getContext('2d'); cx.drawImage(bmp, 0, 0); bmp.close();
  const rgba = cx.getImageData(0, 0, pw, ph).data;
  await predictor(LQ, id);
  const enc = await session('enc', id);
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
    const [, cc, lh, lw] = y.dims; if (!lat) { c = cc; lat = new Int16Array(c * h * w); }
    const oy = (ty - y0) / 16, ox = (tx - x0) / 16, ch_ = Math.min(TL, h - ty / 16), cw_ = Math.min(TL, w - tx / 16);
    for (let q = 0; q < c; q++) for (let yy = 0; yy < ch_; yy++) for (let xx = 0; xx < cw_; xx++)
      lat[(q * h + ty / 16 + yy) * w + tx / 16 + xx] = Math.max(-32767, Math.min(32767, Math.round(y.data[(q * lh + oy + yy) * lw + ox + xx])));
    y.dispose && y.dispose();
  }
  postMessage({ id, type: 'progress', done: tiles.length, total: tiles.length + 1, phase: 'learn-pack' }); await yieldNow();
  const lp = put(new Uint8Array(lat.buffer)); const zp = wasm.nczl_w_enc(lp, c, h, w, pw, ph, LQ); wasm.nczl_free(lp, lat.length * 2);
  if (!zp) throw new Error('could not pack the latents');
  return { out: takeOut(zp), ep: enc.ep };
}
async function learnedDecode(u8, id) {
  const pp = put(u8.subarray(0, 16)); const q = wasm.nczl_is_nczw(pp, 16) & 255; wasm.nczl_free(pp, 16);
  if (q !== LQ) throw new Error('made with a quality this page does not have');
  await predictor(q, id);
  const dec = await session('dec', id);
  postMessage({ id, type: 'progress', done: 0, total: 2, phase: 'learn-unpack' }); await new Promise((r) => setTimeout(r, 0));
  const p = put(u8); const lp = wasm.nczl_w_dec(p, u8.length); wasm.nczl_free(p, u8.length);
  if (!lp) throw new Error('the file is damaged');
  const raw = takeOut(lp); const lat = new Int16Array(raw.buffer, raw.byteOffset, raw.length / 2);
  const [c, h, w, pw, ph] = [0, 1, 2, 3, 4].map((i) => wasm.nczl_w_dim(i));
  const img = new ImageData(pw, ph); const d = img.data;
  const tiles = []; for (let ty = 0; ty < h; ty += TL) for (let tx = 0; tx < w; tx += TL) tiles.push([tx, ty]);
  for (let k = 0; k < tiles.length; k++) {
    postMessage({ id, type: 'progress', done: k + 1, total: tiles.length + 1, phase: 'learn-dec' }); await yieldNow();
    const [tx, ty] = tiles[k];
    const x0 = Math.max(0, tx - ML), y0 = Math.max(0, ty - ML), x1 = Math.min(w, tx + TL + ML), y1 = Math.min(h, ty + TL + ML), tw = x1 - x0, th = y1 - y0;
    const yf = new Float32Array(c * tw * th);
    for (let q = 0; q < c; q++) for (let yy = 0; yy < th; yy++) for (let xx = 0; xx < tw; xx++) yf[(q * th + yy) * tw + xx] = lat[(q * h + y0 + yy) * w + x0 + xx];
    const xo = (await dec.run({ y: new ort.Tensor('float32', yf, [1, c, th, tw]) })).x; const PW = tw * 16, PH = th * 16, P = PW * PH;
    const px0 = tx * 16, py0 = ty * 16, px1 = Math.min(pw, (tx + TL) * 16), py1 = Math.min(ph, (ty + TL) * 16);
    for (let yy = py0; yy < py1; yy++) for (let xx = px0; xx < px1; xx++) {
      const s = (yy - y0 * 16) * PW + (xx - x0 * 16), o = (yy * pw + xx) * 4;
      d[o] = xo.data[s] * 255 + 0.5; d[o + 1] = xo.data[P + s] * 255 + 0.5; d[o + 2] = xo.data[2 * P + s] * 255 + 0.5; d[o + 3] = 255; }
    xo.dispose && xo.dispose();
  }
  const cv = new OffscreenCanvas(pw, ph); cv.getContext('2d').putImageData(img, 0, 0);
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
    } else if (type === 'gzip') {
      postMessage({ id, type: 'done', gz: await gzipSize(u8) });
    } else if (type === 'learned') {
      if (!isImage(u8)) throw new Error('tiny mode takes photos (PNG, JPEG, WebP)');
      const t = performance.now(); const { out, ep } = await learnedEncode(u8, id);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t, gz: null, pngMode: 'tiny', ep }, [out.buffer]);
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
      const { out, mode: pm } = await decompress(u8, id);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t, pngMode: pm }, [out.buffer]);
    }
  } catch (err) { postMessage({ id, type: 'error', message: String(err && err.message || err) }); }
};
