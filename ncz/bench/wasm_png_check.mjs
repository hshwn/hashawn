// Native vs wasm, PNG modes: the browser build must write the same .ncz bytes and rebuild the same PNG bytes.
//   node wasm_png_check.mjs <nczl.wasm> <dir> <name> ...
// <dir>/<name>.png is the input; <dir>/<name>.m<mode>.ncz and .m<mode>.png are what `nczcli png-mode` wrote.
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const [wasmPath, dir, ...names] = process.argv.slice(2);
const { instance } = await WebAssembly.instantiate(readFileSync(wasmPath), {});
const w = instance.exports;
const mem = () => new Uint8Array(w.memory.buffer);
const put = (u8) => { const p = w.nczl_alloc(u8.length); mem().set(u8, p); return p; };
const take = (p) => { const n = w.nczl_out_len(); const o = mem().slice(p, p + n); w.nczl_free(p, n); return o; };
const sha = (u8) => createHash('sha256').update(u8).digest('hex').slice(0, 16);

let bad = 0;
for (const name of names) {
  const png = new Uint8Array(readFileSync(`${dir}/${name}.png`));
  for (const mode of [0, 1, 2]) {
    let t = performance.now();
    let p = put(png); const e = w.nczl_enc_new_mode(p, png.length, mode); w.nczl_free(p, png.length);
    while (w.nczl_enc_step(e, 64) > 0);
    const z = take(w.nczl_enc_finish(e)); const te = performance.now() - t;
    t = performance.now();
    p = put(z); const d = w.nczl_dec_new(p, z.length); w.nczl_free(p, z.length);
    while (w.nczl_dec_step(d, 64) > 0);
    const back = take(w.nczl_dec_finish(d)); const td = performance.now() - t;
    p = put(z); const got = w.nczl_png_mode(p, z.length); w.nczl_free(p, z.length);
    const nz = new Uint8Array(readFileSync(`${dir}/${name}.m${mode}.ncz`)), nb = new Uint8Array(readFileSync(`${dir}/${name}.m${mode}.png`));
    const ok = sha(z) === sha(nz) && sha(back) === sha(nb);
    if (!ok) bad++;
    console.log(`${name.padEnd(14)} mode ${mode} -> ${String(got).padStart(3)}  ncz ${sha(z)} ${sha(z) === sha(nz) ? '=' : '!='} native  png ${sha(back)} ${sha(back) === sha(nb) ? '=' : '!='} native  enc ${(te / 1000).toFixed(1)}s dec ${(td / 1000).toFixed(1)}s  ${ok ? 'SAME' : 'DIFFERENT'}`);
  }
}
console.log(bad ? `${bad} MISMATCHES` : 'all identical');
process.exit(bad ? 1 : 0);
