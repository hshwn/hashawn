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

async function compress(u8, id) {
  await load();
  let inPtr = put(u8);
  const text = wasm.nczl_is_text(inPtr, u8.length);
  if (text) { wasm.nczl_free(inPtr, u8.length); await loadModel(id); inPtr = put(u8); }
  const enc = wasm.nczl_enc_new(inPtr, u8.length);
  wasm.nczl_free(inPtr, u8.length);
  const step = text && modelId ? STEP_LM : STEP;
  let t0 = performance.now(), done = 0;
  for (;;) {
    const n = wasm.nczl_enc_step(enc, step);
    done = wasm.nczl_enc_done(enc);
    const now = performance.now();
    if (now - t0 > 80 || n === 0) { t0 = now; postMessage({ id, type: 'progress', done, total: u8.length, brain: !!(text && modelId) }); await new Promise((r) => setTimeout(r, 0)); }
    if (n === 0) break;
  }
  return takeOut(wasm.nczl_enc_finish(enc));
}

async function decompress(u8, id) {
  await load();
  let inPtr = put(u8);
  if (wasm.nczl_needs_model(inPtr, u8.length)) { wasm.nczl_free(inPtr, u8.length); await loadModel(id); inPtr = put(u8); }
  const dec = wasm.nczl_dec_new(inPtr, u8.length);
  wasm.nczl_free(inPtr, u8.length);
  if (!dec) throw new Error('not one of mine, or made with a different model');
  const total = wasm.nczl_dec_total(dec);
  let t0 = performance.now();
  for (;;) {
    const n = wasm.nczl_dec_step(dec, STEP_LM);
    const done = wasm.nczl_dec_done(dec);
    const now = performance.now();
    if (now - t0 > 80 || n === 0) { t0 = now; postMessage({ id, type: 'progress', done, total }); await new Promise((r) => setTimeout(r, 0)); }
    if (n === 0) break;
  }
  const ptr = wasm.nczl_dec_finish(dec);
  if (!ptr) throw new Error('the file is damaged');
  return takeOut(ptr);
}

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
  const { id, type, buf, gz: wantGz = true } = e.data;
  const u8 = new Uint8Array(buf);
  try {
    if (type === 'plan') {
      postMessage({ id, type: 'done', ...(await plan(u8)) });
    } else if (type === 'gzip') {
      postMessage({ id, type: 'done', gz: await gzipSize(u8) });
    } else if (type === 'compress') {
      const t = performance.now();
      const out = await compress(u8, id);
      const ms = performance.now() - t;
      const gz = wantGz ? await gzipSize(u8) : null;
      postMessage({ id, type: 'done', buf: out.buffer, ms, gz }, [out.buffer]);
    } else {
      const t = performance.now();
      const out = await decompress(u8, id);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t }, [out.buffer]);
    }
  } catch (err) { postMessage({ id, type: 'error', message: String(err && err.message || err) }); }
};
