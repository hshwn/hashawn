// worker.js — runs the ncz codec (WebAssembly) off the main thread, in chunks, with progress.
// Also gzips the same bytes with the browser's CompressionStream so the page can show an honest comparison.
let wasm = null, mem = null;
const CHUNK = 64 * 1024;
const bytes = () => new Uint8Array(wasm.memory.buffer);   // re-read after every call: memory can grow and detach

async function load() {
  if (wasm) return;
  let instance;
  try { ({ instance } = await WebAssembly.instantiateStreaming(fetch('ncz.wasm'), {})); }
  catch { ({ instance } = await WebAssembly.instantiate(await (await fetch('ncz.wasm')).arrayBuffer(), {})); }   // server without the wasm MIME type
  wasm = instance.exports;
}

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
  const enc = wasm.ncz_enc_new();
  const scratch = wasm.ncz_alloc(CHUNK);
  let t0 = performance.now();
  for (let off = 0; off < u8.length; off += CHUNK) {
    const part = u8.subarray(off, Math.min(u8.length, off + CHUNK));
    bytes().set(part, scratch);
    wasm.ncz_enc_push(enc, scratch, part.length);
    const now = performance.now();
    if (now - t0 > 80 || off + CHUNK >= u8.length) { t0 = now; postMessage({ id, type: 'progress', done: Math.min(u8.length, off + CHUNK), total: u8.length, out: wasm.ncz_enc_out_len(enc) }); }
    await new Promise((r) => setTimeout(r, 0));          // let progress messages flush
  }
  wasm.ncz_free(scratch, CHUNK);
  const lenPtr = wasm.ncz_alloc(8);
  const outPtr = wasm.ncz_enc_finish(enc, lenPtr);
  const outLen = Number(new DataView(wasm.memory.buffer).getUint32(lenPtr, true));
  const out = bytes().slice(outPtr, outPtr + outLen);
  wasm.ncz_free(outPtr, outLen); wasm.ncz_free(lenPtr, 8);
  return out;
}

async function decompress(u8, id) {
  await load();
  const inPtr = wasm.ncz_alloc(u8.length);
  bytes().set(u8, inPtr);
  const dec = wasm.ncz_dec_new(inPtr, u8.length);
  wasm.ncz_free(inPtr, u8.length);
  if (!dec) throw new Error('not an ncz file');
  const total = Number(wasm.ncz_dec_total(dec));
  const out = new Uint8Array(total);
  const scratch = wasm.ncz_alloc(CHUNK);
  let off = 0, t0 = performance.now();
  while (off < total) {
    const n = wasm.ncz_dec_next(dec, scratch, Math.min(CHUNK, total - off));
    out.set(bytes().subarray(scratch, scratch + n), off);
    off += n;
    const now = performance.now();
    if (now - t0 > 80 || off >= total) { t0 = now; postMessage({ id, type: 'progress', done: off, total }); }
    await new Promise((r) => setTimeout(r, 0));
    if (n === 0) break;
  }
  wasm.ncz_free(scratch, CHUNK); wasm.ncz_dec_free(dec);
  return out;
}

onmessage = async (e) => {
  const { id, type, buf } = e.data;
  const u8 = new Uint8Array(buf);
  try {
    if (type === 'compress') {
      const t = performance.now();
      const out = await compress(u8, id);
      const ms = performance.now() - t;
      const gz = await gzipSize(u8);
      postMessage({ id, type: 'done', buf: out.buffer, ms, gz }, [out.buffer]);
    } else {
      const t = performance.now();
      const out = await decompress(u8, id);
      postMessage({ id, type: 'done', buf: out.buffer, ms: performance.now() - t }, [out.buffer]);
    }
  } catch (err) { postMessage({ id, type: 'error', message: String(err && err.message || err) }); }
};
