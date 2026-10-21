// app.js — drop a file: it gets compressed with ncz in your browser and handed back as .ncz.
// Drop a .ncz: you get the original back. Nothing leaves the machine.
(() => {
  const drop = document.getElementById('drop'), input = document.getElementById('file');
  const result = document.getElementById('result');
  const worker = new Worker('worker.js');
  const fmt = (n) => n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(2)} MB`;
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  let jobId = 0, busy = false;

  // ---------- him ----------
  const canvas = document.getElementById('face'), bubble = document.getElementById('bubble'), bubbleTxt = bubble.querySelector('.txt');
  const phone = () => matchMedia('(max-width: 720px)').matches;
  const face = Face.create(canvas, {
    open_closed: '/assets/face/open_closed.webp', open_open: '/assets/face/open_open.webp',
    shut_closed: '/assets/face/shut_closed.webp', shut_open: '/assets/face/shut_open.webp',
    normal: '/assets/face/normal.webp',
  });
  const corner = () => { const r = phone() ? 34 : 48; return { x: innerWidth - r - (phone() ? 14 : 28), y: innerHeight - r - (phone() ? 14 : 28), r }; };
  if (face) {
    const c = corner(); face.snap(c.x, c.y, c.r);
    addEventListener('resize', () => { const c = corner(); face.target(c.x, c.y, c.r); });
    const tick = () => { face.setFace(-0.45, 0.35, 1); if (!bubble.hidden) { const half = bubble.offsetWidth / 2, x = Math.max(half + 8, Math.min(innerWidth - half - 8, face.x)); bubble.style.left = `${x}px`; bubble.style.top = `${face.y - face.r - 14}px`; bubble.style.setProperty('--tail', `${face.x - x}px`); } requestAnimationFrame(tick); };
    tick();
  }
  try { if (localStorage.getItem('mute') === '1') Voice.setMuted(true); } catch {}
  let hideTimer = null;
  const say = (text) => {
    if (!face || !text) return;
    clearTimeout(hideTimer); bubble.hidden = false; bubble.classList.remove('done'); bubbleTxt.textContent = '';
    Voice.say(text, { onChar: (i) => { bubbleTxt.textContent = text.slice(0, i + 1); }, onMouth: (o) => { face.mouthOpen = o; },
      onEnd: () => { bubble.classList.add('done'); hideTimer = setTimeout(() => { bubble.hidden = true; }, 2600); } });
  };
  addEventListener('pointerdown', () => Voice.unlock(), { once: true });
  const lineFor = (ratio, gzRatio) => {
    if (ratio < 1.02) return "that's already squeezed. or it's noise. can't tell.";
    if (gzRatio && ratio > gzRatio * 1.25) return `${ratio.toFixed(1)} times smaller. gzip got ${gzRatio.toFixed(1)}. i did that.`;
    if (ratio > 3) return `${ratio.toFixed(1)} times smaller. not bad.`;
    return `${ratio.toFixed(1)} times smaller.`;
  };

  // ---------- jobs ----------
  const render = (html) => { result.innerHTML = html; result.hidden = false; };
  const progress = (label, done, total) => render(`<div class="job"><p class="label">${label}</p><div class="bar"><i style="width:${total ? (done / total * 100).toFixed(1) : 0}%"></i></div><p class="sub">${fmt(done)} of ${fmt(total)}</p></div>`);
  const download = (u8, name) => { const url = URL.createObjectURL(new Blob([u8])); const a = document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 5000); };

  // ---------- the codec, across workers ----------
  // Big files are split into independent segments (where: decided by the codec itself, so the bytes match
  // the native build) and compressed by several workers at once. Small files go through one worker.
  const POOL = Math.max(1, Math.min(8, (navigator.hardwareConcurrency || 4) - 1));
  const pool = [worker];
  const getWorker = (i) => pool[i] || (pool[i] = new Worker('worker.js'));
  const call = (w, msg, transfer, onProgress) => new Promise((resolve) => {
    const id = ++jobId;
    const on = (e) => { const m = e.data; if (m.id !== id) return; if (m.type === 'progress' || m.type === 'model') { if (onProgress) onProgress(m); return; } w.removeEventListener('message', on); resolve(m); };
    w.addEventListener('message', on); w.postMessage({ ...msg, id }, transfer || []);
  });
  // run jobs[i] on up to `k` workers; each job gets (worker, report(done)) and returns a result
  const fanOut = async (n, k, job) => {
    const results = new Array(n); let next = 0;
    await Promise.all(Array.from({ length: Math.min(k, n) }, async (_, wi) => {
      const w = getWorker(wi);
      for (;;) { const i = next++; if (i >= n) return; results[i] = await job(w, i); if (results[i].type === 'error') throw new Error(results[i].message); }
    }));
    return results;
  };
  const u32le = (v, n) => { const b = new Uint8Array(n); let x = BigInt(v); for (let i = 0; i < n; i++) { b[i] = Number(x & 255n); x >>= 8n; } return b; };

  const compressAll = async (u8, onProgress) => {
    const t0 = performance.now();
    const pl = await call(pool[0], { type: 'plan', buf: u8.slice().buffer });
    if (pl.ends.length <= 1) {
      const buf = u8.slice().buffer;
      const m = await call(pool[0], { type: 'compress', buf }, [buf], (p) => onProgress(p.type === 'model' ? { model: p } : { done: p.done, brain: p.brain, png: p.png, k: 1 }));
      return m;
    }
    const starts = [0, ...pl.ends.slice(0, -1)];
    const done = new Array(pl.ends.length).fill(0); let brain = false;
    const k = Math.min(POOL, pl.ends.length);
    const gzP = call(pool[0], { type: 'gzip', buf: u8.slice().buffer });
    const res = await fanOut(pl.ends.length, k, (w, i) => {
      const part = u8.slice(starts[i], pl.ends[i]).buffer;
      return call(w, { type: 'compress', buf: part, gz: false }, [part], (p) => {
        if (p.type === 'model') { if (i === 0) onProgress({ model: p }); return; }
        done[i] = p.done; brain = brain || p.brain; onProgress({ done: done.reduce((a, b) => a + b, 0), brain, k });
      });
    });
    const streams = res.map((m) => new Uint8Array(m.buf));
    const head = 18 + 4 + 4 * streams.length, size = head + streams.reduce((a, s) => a + s.length, 0);
    const out = new Uint8Array(size);
    out.set([78, 67, 90, 76, pl.version, pl.modeSeg], 0);        // "NCZL" | version | segmented; model id 0 (each segment has its own)
    out.set(u32le(0, 4), 6); out.set(u32le(u8.length, 8), 10); out.set(u32le(streams.length, 4), 18);
    let off = head; streams.forEach((s, i) => { out.set(u32le(s.length, 4), 22 + 4 * i); out.set(s, off); off += s.length; });
    return { type: 'done', buf: out.buffer, ms: performance.now() - t0, gz: (await gzP).gz, k };
  };

  const decompressAll = async (u8, onProgress) => {
    const t0 = performance.now();
    const seg = u8.length >= 22 && u8[0] === 78 && u8[1] === 67 && u8[2] === 90 && u8[3] === 76 && u8[5] === 3;
    if (!seg) { const buf = u8.slice().buffer; return call(pool[0], { type: 'decompress', buf }, [buf], (p) => onProgress(p.type === 'model' ? { model: p } : { done: p.total ? p.done / p.total * u8.length : p.done, k: 1 })); }
    const dv = new DataView(u8.buffer, u8.byteOffset, u8.byteLength);
    const total = Number(dv.getBigUint64(10, true)), n = dv.getUint32(18, true);
    const lens = Array.from({ length: n }, (_, i) => dv.getUint32(22 + 4 * i, true));
    const offs = []; let off = 22 + 4 * n; for (const l of lens) { offs.push(off); off += l; }
    const done = new Array(n).fill(0); const k = Math.min(POOL, n);
    const res = await fanOut(n, k, (w, i) => {
      const part = u8.slice(offs[i], offs[i] + lens[i]).buffer;
      return call(w, { type: 'decompress', buf: part }, [part], (p) => { if (p.type === 'model') { if (i === 0) onProgress({ model: p }); return; } done[i] = p.done; onProgress({ done: done.reduce((a, b) => a + b, 0), k }); });
    });
    const out = new Uint8Array(total); let o = 0;
    for (const m of res) { const b = new Uint8Array(m.buf); out.set(b, o); o += b.length; }
    if (o !== total) return { type: 'error', message: 'the file is damaged' };
    return { type: 'done', buf: out.buffer, ms: performance.now() - t0, k };
  };

  const run = async (file) => {
    if (busy) return; busy = true; drop.classList.add('busy');
    const isNcz = /\.ncz$/i.test(file.name);
    const u8 = new Uint8Array(await file.arrayBuffer());
    const total = u8.length;
    progress(isNcz ? 'unpacking' : 'compressing', 0, total);
    const onProgress = (p) => {
      if (p.model) { render(`<div class="job"><p class="label">fetching the brain</p><div class="bar"><i style="width:${p.model.total ? (p.model.done / p.model.total * 100).toFixed(1) : 0}%"></i></div><p class="sub">3 MB, once</p></div>`); return; }
      const label = isNcz ? 'unpacking' : p.png ? 'compressing the pixels' : (p.brain ? 'compressing, with the brain' : 'compressing');
      progress(p.k > 1 ? `${label} · ${p.k} at once` : label, Math.min(p.done, total), total);
    };
    let done;
    try { done = await (isNcz ? decompressAll(u8, onProgress) : compressAll(u8, onProgress)); }
    catch (err) { done = { type: 'error', message: String(err && err.message || err) }; }
    busy = false; drop.classList.remove('busy');
    if (done.type === 'error') { render(`<div class="job"><p class="label">couldn't do that</p><p class="sub">${esc(done.message)}</p></div>`); say("that's not one of mine."); return; }
    const out = new Uint8Array(done.buf);
    if (isNcz) {
      const name = file.name.replace(/\.ncz$/i, '');
      render(`<div class="job"><p class="label">unpacked</p><p class="big">${fmt(out.length)}</p><p class="sub">${esc(name)} · ${(done.ms / 1000).toFixed(1)}s</p><button class="btn" id="dl">save it</button></div>`);
      document.getElementById('dl').onclick = () => download(out, name);
      const png = out.length > 8 && out[0] === 0x89 && out[1] === 0x50 && out[2] === 0x4e && out[3] === 0x47;
      say(png ? "there it is. every pixel." : "there it is. every byte.");
      return;
    }
    const ratio = total / out.length, gzRatio = done.gz ? total / done.gz : null;
    const pct = (n) => `${Math.min(100, n / total * 100).toFixed(1)}%`;
    render(`<div class="job">
      <p class="label">${esc(file.name)}</p>
      <p class="big">${ratio.toFixed(2)}×</p>
      <div class="bars">
        <div class="row"><span>original</span><div class="bar"><i style="width:100%;background:#141311"></i></div><b>${fmt(total)}</b></div>
        ${done.gz ? `<div class="row"><span>gzip</span><div class="bar"><i style="width:${pct(done.gz)};background:#7a766f"></i></div><b>${fmt(done.gz)}</b></div>` : ''}
        <div class="row"><span>ncz</span><div class="bar"><i style="width:${pct(out.length)};background:#009f4d"></i></div><b>${fmt(out.length)}</b></div>
      </div>
      <p class="sub">${(done.ms / 1000).toFixed(1)}s · ${/\.png$/i.test(file.name) ? 'every pixel comes back, metadata too · ' : ''}nothing left your browser</p>
      <button class="btn" id="dl">save .ncz</button>
    </div>`);
    document.getElementById('dl').onclick = () => download(out, file.name + '.ncz');
    say(lineFor(ratio, gzRatio));
  };

  // ---------- drop zone ----------
  ['dragenter', 'dragover'].forEach((t) => drop.addEventListener(t, (e) => { e.preventDefault(); drop.classList.add('over'); }));
  ['dragleave', 'drop'].forEach((t) => drop.addEventListener(t, (e) => { e.preventDefault(); drop.classList.remove('over'); }));
  drop.addEventListener('drop', (e) => { const f = e.dataTransfer.files[0]; if (f) run(f); });
  drop.addEventListener('click', () => input.click());
  input.addEventListener('change', () => { if (input.files[0]) run(input.files[0]); input.value = ''; });
  window.__compress = run; window.__face = face; window.__codec = { compressAll, decompressAll, POOL };   // for testing
})();
