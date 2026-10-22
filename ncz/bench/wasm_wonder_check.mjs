// Native vs wasm, learned mode: same NCZW bytes from the same latents, and the native stream decodes to the same latents.
//   node wasm_wonder_check.mjs <nczl.wasm> <q.pred> <q> <native_dir> <a.lat> ...
import { readFileSync } from 'node:fs';
const [wasmPath, predPath, q, natDir, ...lats] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {}); const w = instance.exports;
const mem = () => new Uint8Array(w.memory.buffer);
const put = (u8) => { const p = w.nczl_alloc(u8.length); mem().set(u8, p); return p; };
const take = (p) => { const n = w.nczl_out_len(); const o = mem().slice(p, p + n); w.nczl_free(p, n); return o; };
const pred = new Uint8Array(readFileSync(predPath)); const pp = put(pred); if (!w.nczl_w_load(pp, pred.length, +q)) throw new Error('predictor did not load'); w.nczl_free(pp, pred.length);
let bad = 0;
for (const f of lats) {
  const b = new Uint8Array(readFileSync(f)); const dv = new DataView(b.buffer, b.byteOffset); const [c, h, wd] = [dv.getUint16(0, true), dv.getUint16(2, true), dv.getUint16(4, true)];
  const lat = b.slice(6); let t = performance.now();
  const lp = put(lat); const zp = w.nczl_w_enc(lp, c, h, wd, wd * 16, h * 16, +q); w.nczl_free(lp, lat.length); const z = take(zp); const te = performance.now() - t;
  const name = f.split('/').pop().replace(/\.lat$/, ''); const nat = new Uint8Array(readFileSync(`${natDir}/${name}.nczw`));
  const sameStream = z.length === nat.length && z.every((v, i) => v === nat[i]);
  t = performance.now(); const np = put(nat); const dp = w.nczl_w_dec(np, nat.length); w.nczl_free(np, nat.length); const back = dp ? take(dp) : new Uint8Array(0); const td = performance.now() - t;
  const sameLat = back.length === lat.length && back.every((v, i) => v === lat[i]);
  if (!sameStream || !sameLat) bad++;
  console.log(`${name} ${z.length} B  stream ${sameStream ? '= native' : 'DIFFERENT'}  decode ${sameLat ? 'exact' : 'WRONG'}  enc ${(te / 1000).toFixed(2)}s dec ${(td / 1000).toFixed(2)}s`);
}
console.log(bad ? `${bad} MISMATCHES` : 'all identical'); process.exit(bad ? 1 : 0);
