// app.js — drop a file: it gets compressed with ncz in your browser and handed back as .ncz (tiny photos: .nczw).
// Drop one of those (known by its first bytes, whatever its name): you get the original back. Nothing leaves the machine.
(() => {
  const drop = document.getElementById('drop'), input = document.getElementById('file');
  const result = document.getElementById('result');
  const worker = new Worker('worker.js');
  const fmt = (n) => n < 1024 ? `${n} B` : n < 1048576 ? `${(n / 1024).toFixed(1)} KB` : `${(n / 1048576).toFixed(2)} MB`;
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));
  let jobId = 0, busy = false;

  // ---------- PNG mode: 0 every pixel exact, 1 every sample within +-1, 2 looks the same (grain regrown) ----------
  const modes = document.getElementById('modes');
  // lossy modes are not remembered: every visit starts exact, so nothing is lost without picking it that visit
  let mode = 0; try { localStorage.removeItem('pngMode'); } catch {}
  // tiny mode's size: the teacher quality (1 smallest .. 4 sharpest); its networks are fetched when first used
  const tinyq = document.getElementById('tinyq');
  let tq = 4; try { tq = [1, 2, 4].includes(+localStorage.getItem('tinyQ')) ? +localStorage.getItem('tinyQ') : 4; } catch {}
  const showQ = () => tinyq.querySelectorAll('button').forEach((b) => { const on = +b.dataset.q === tq; b.classList.toggle('on', on); b.setAttribute('aria-checked', on); });
  tinyq.addEventListener('click', (e) => { const b = e.target.closest('button'); if (!b || busy) return; tq = +b.dataset.q; showQ(); try { localStorage.setItem('tinyQ', tq); } catch {} });
  showQ();
  // how tiny files unpack (sizes that have a texture decoder): smooth, or with texture drawn in
  const tinytex = document.getElementById('tinytex');
  let texPref = true; try { texPref = localStorage.getItem('tinyTex') !== '0'; } catch {}
  const showTex = () => tinytex.querySelectorAll('button').forEach((b) => { const on = (b.dataset.t === '1') === texPref; b.classList.toggle('on', on); b.setAttribute('aria-checked', on); });
  tinytex.addEventListener('click', (e) => { const b = e.target.closest('button'); if (!b || busy) return; texPref = b.dataset.t === '1'; showTex(); try { localStorage.setItem('tinyTex', texPref ? '1' : '0'); } catch {} });
  showTex();
  const showMode = () => { modes.querySelectorAll('button').forEach((b) => { const on = +b.dataset.m === mode; b.classList.toggle('on', on); b.setAttribute('aria-checked', on); }); tinyq.hidden = mode !== 3; tinytex.hidden = mode !== 3; };
  modes.addEventListener('click', (e) => { const b = e.target.closest('button'); if (!b || busy) return; mode = +b.dataset.m; showMode(); });
  showMode();
  // what came back: 0 exact, k within +-k, 255 grain mode
  const isTiny = (m) => m === 'tiny' || m === 'tiny-tex';
  const pngNote = (m) => m === 'tiny-tex' ? 'tiny, textured: small samples of the photo\'s own texture, carried in the file, laid back where it went soft · '
    : m === 'tiny' ? 'tiny: a learned codec, it looks close but it is not the original · ' : m === 255 ? 'looks the same: the grain is regrown, the pixels are not the originals · '
    : m > 0 ? `every pixel within ±${m} of the original, metadata kept · ` : 'every pixel comes back, metadata too · ';

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
  // ---------- seeing pictures: a .ncz is not something any viewer opens, so the page shows what comes back ----------
  const isPng = (u) => u.length > 8 && u[0] === 0x89 && u[1] === 0x50 && u[2] === 0x4e && u[3] === 0x47;
  let urls = []; const blobUrl = (data, type) => { const u = URL.createObjectURL(new Blob([data], { type })); urls.push(u); return u; };
  const dropUrls = () => { urls.forEach((u) => URL.revokeObjectURL(u)); urls = []; };
  // one picture, or the original and what comes back with a slider between them
  const viewer = (outUrl, origUrl, ext) => origUrl
    ? `<div class="view cmp" id="cmp"><img src="${origUrl}" alt="original"><img class="top" src="${outUrl}" alt="what comes back"><i class="seam"></i>
         <span class="tag l">original</span><span class="tag r">from the ${ext}</span></div>
       <input class="slide" id="slide" type="range" min="0" max="100" value="50" aria-label="compare">`
    : `<div class="view"><img src="${outUrl}" alt="the picture"></div>`;
  const wireSlider = () => {
    const c = document.getElementById('cmp'), s = document.getElementById('slide'); if (!c || !s) return;
    const set = () => { c.style.setProperty('--x', `${s.value}%`); }; s.addEventListener('input', set); set();
    const drag = (e) => { const r = c.getBoundingClientRect(); s.value = Math.max(0, Math.min(100, ((e.touches ? e.touches[0].clientX : e.clientX) - r.left) / r.width * 100)); set(); };
    c.addEventListener('pointerdown', (e) => { drag(e); c.setPointerCapture(e.pointerId); c.onpointermove = drag; });
    c.addEventListener('pointerup', () => { c.onpointermove = null; });
  };
  const download = (u8, name, type = 'application/octet-stream') => { const url = URL.createObjectURL(new Blob([u8], { type })); const a = document.createElement('a'); a.href = url; a.download = name; a.click(); setTimeout(() => URL.revokeObjectURL(url), 5000); };

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
      for (;;) { const i = next++; if (i >= n) return; results[i] = await job(w, i); if (results[i].type === 'error') throw Object.assign(new Error(results[i].message), { code: results[i].code }); }
    }));
    return results;
  };
  const u32le = (v, n) => { const b = new Uint8Array(n); let x = BigInt(v); for (let i = 0; i < n; i++) { b[i] = Number(x & 255n); x >>= 8n; } return b; };

  const isImage = (u8) => (u8[0] === 0x89 && u8[1] === 0x50 && u8[2] === 0x4e && u8[3] === 0x47) || (u8[0] === 0xff && u8[1] === 0xd8 && u8[2] === 0xff)
    || (u8[0] === 0x52 && u8[1] === 0x49 && u8[2] === 0x46 && u8[3] === 0x46 && u8[8] === 0x57 && u8[9] === 0x45 && u8[10] === 0x42 && u8[11] === 0x50);
  const compressAll = async (u8, onProgress) => {
    const t0 = performance.now();
    if (mode === 3 && isImage(u8)) {      // tiny: the learned codec, one worker (the networks are big; one copy of them)
      const buf = u8.slice().buffer;
      return call(pool[0], { type: 'learned', buf, q: tq, tex: texPref }, [buf], (p) => onProgress(p.type === 'model' ? { model: p } : { done: p.done / p.total * u8.length, phase: p.phase, k: 1 }));
    }
    const pl = await call(pool[0], { type: 'plan', buf: u8.slice().buffer });
    if (pl.ends.length <= 1) {
      const buf = u8.slice().buffer;
      const m = await call(pool[0], { type: 'compress', buf, mode }, [buf], (p) => onProgress(p.type === 'model' ? { model: p } : { done: p.done, brain: p.brain, png: p.png, phase: p.phase, k: 1 }));
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

  // files of ours, known by their first bytes: NCZL (ncz, maybe with the language model), NCZP (PNGs), NCZW (tiny),
  // NCZ3 / NCZ2 (the site's earlier files)
  const MAGICS = ['NCZL', 'NCZP', 'NCZW', 'NCZ3', 'NCZ2'];
  const isMine = (u8) => u8.length >= 5 && MAGICS.includes(String.fromCharCode(u8[0], u8[1], u8[2], u8[3]));
  // what went wrong, by the worker's error code
  const FAIL = {
    unknown: ['not a file i know', "that's not one of mine."], damaged: ['that file is damaged', "that one's broken. i can't open it."],
    network: ['could not fetch what it needs', "couldn't fetch what i need. check the connection."],
    'too-big': ['that picture is too big', 'too big for me. try a smaller copy.'], version: ['made with a model this page does not have', "that's from a different version of me."],
  };
  const fail = (e) => { const [label, line] = FAIL[e.code] || ["couldn't do that", "that didn't work."]; render(`<div class="job"><p class="label">${label}</p><p class="sub">${esc(e.message || '')}</p></div>`); say(line); };

  const decompressAll = async (u8, onProgress) => {
    const t0 = performance.now();
    const seg = u8.length >= 22 && u8[0] === 78 && u8[1] === 67 && u8[2] === 90 && u8[3] === 76 && u8[5] === 3;
    if (!seg) { const buf = u8.slice().buffer; return call(pool[0], { type: 'decompress', buf, tex: texPref }, [buf], (p) => onProgress(p.type === 'model' ? { model: p } : { done: p.total ? p.done / p.total * u8.length : p.done, phase: p.phase, k: 1 })); }
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
    if (o !== total) return { type: 'error', code: 'damaged', message: 'the file is damaged' };
    return { type: 'done', buf: out.buffer, ms: performance.now() - t0, k };
  };

  const run = async (file) => {
    if (busy) return; busy = true; drop.classList.add('busy');
    const u8 = new Uint8Array(await file.arrayBuffer());
    const isNcz = isMine(u8), total = u8.length;
    if (!isNcz && /\.nczw?$/i.test(file.name)) { busy = false; drop.classList.remove('busy'); fail({ code: 'unknown', message: 'it is named like one of mine, but it does not start like one' }); return; }
    progress(isNcz ? 'unpacking' : 'compressing', 0, total);
    const onProgress = (p) => {
      if (p.model) {
        const tiny = !!p.model.label;
        render(`<div class="job"><p class="label">${tiny ? `fetching ${p.model.label}` : 'fetching the brain'}</p><div class="bar"><i style="width:${p.model.total ? (p.model.done / p.model.total * 100).toFixed(1) : 0}%"></i></div><p class="sub">${tiny ? `${fmt(p.model.done)} of ${fmt(p.model.total)}, once` : '3 MB, once'}</p></div>`); return; }
      const lph = { 'learn-grain': 'measuring the grain', 'learn-enc': 'looking at the picture', 'learn-pack': 'packing it', 'learn-unpack': 'unpacking', 'learn-dec': 'painting it back', 'learn-grain-back': 'growing the grain back', 'learn-swatch': 'choosing real texture', 'learn-swatch-back': 'laying the texture' }[p.phase];
      if (lph) { progress(lph, Math.min(p.done, total), total); return; }
      const label = p.phase === 'grain' ? (isNcz ? 'growing the grain back' : 'finding the grain') : isNcz ? 'unpacking' : p.png ? 'compressing the pixels' : (p.brain ? 'compressing, with the brain' : 'compressing');
      progress(p.k > 1 ? `${label} · ${p.k} at once` : label, Math.min(p.done, total), total);
    };
    let done;
    try { done = await (isNcz ? decompressAll(u8, onProgress) : compressAll(u8, onProgress)); }
    catch (err) { done = { type: 'error', code: err && err.code, message: String(err && err.message || err) }; }
    busy = false; drop.classList.remove('busy');
    if (done.type === 'error') { fail(done); return; }
    const out = new Uint8Array(done.buf); dropUrls();
    if (isNcz) {
      let name = file.name.replace(/\.nczw?$/i, '');
      if (isTiny(done.pngMode)) name = name.replace(/\.(jpe?g|webp|png)$/i, '') + '.png';
      const pic = isPng(out) ? viewer(blobUrl(out, 'image/png')) : '';
      render(`<div class="job"><p class="label">unpacked</p><p class="big">${fmt(out.length)}</p>${pic}<p class="sub">${esc(name)} · ${(done.ms / 1000).toFixed(1)}s${done.pngMode ? ' · ' + pngNote(isTiny(done.pngMode) ? done.pngMode : done.pngMode | 0).replace(/ · $/, '') : ''}</p><button class="btn" id="dl">save it</button></div>`);
      document.getElementById('dl').onclick = () => download(out, name, isPng(out) ? 'image/png' : undefined);
      const png = out.length > 8 && out[0] === 0x89 && out[1] === 0x50 && out[2] === 0x4e && out[3] === 0x47;
      const m = done.pngMode | 0;
      say(isTiny(done.pngMode) ? "there it is. close enough to fool you." : !png ? "there it is. every byte." : m === 255 ? "there it is. same look, fresh grain." : m > 0 ? "there it is. every pixel, give or take one." : "there it is. every pixel.");
      return;
    }
    const ext = isTiny(done.pngMode) ? '.nczw' : '.ncz';
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
      <p class="sub">${(done.ms / 1000).toFixed(1)}s · ${done.pngMode || /\.png$/i.test(file.name) ? pngNote(isTiny(done.pngMode) ? done.pngMode : done.pngMode | 0) : ''}nothing left your browser</p>
      ${done.alpha ? '<p class="warn">this picture has see-through parts. tiny has none: they come back solid. pick exact to keep them.</p>' : ''}
      <div id="peek"></div>
      <button class="btn" id="dl">save ${ext}</button><button class="btn ghost" id="dlpic" hidden>save the picture</button>
    </div>`);
    document.getElementById('dl').onclick = () => download(out, file.name + ext);
    say(done.alpha ? "careful: tiny fills in the see-through parts." : lineFor(ratio, gzRatio));
    // a lossy picture mode: show what comes back next to the original. Tiny hands back the picture its encoder already
    // painted (composed exactly as unpacking does, see worker.js learnedEncode); the others are unpacked straight away
    const lossy = isTiny(done.pngMode) || (done.pngMode | 0) > 0;
    if (lossy) {
      const peek = document.getElementById('peek'); peek.innerHTML = '<p class="sub">unpacking it so you can see it…</p>';
      const back = done.pic ? { type: 'done', buf: done.pic } : await decompressAll(out.slice(), () => {});
      if (back.type === 'done' && document.getElementById('peek') === peek) {
        const pic = new Uint8Array(back.buf);
        peek.innerHTML = viewer(blobUrl(pic, 'image/png'), blobUrl(u8, file.type || 'image/png'), ext); wireSlider();
        const b = document.getElementById('dlpic'); b.hidden = false; b.onclick = () => download(pic, file.name.replace(/\.(jpe?g|webp|png)$/i, '') + ext + '.png', 'image/png');
      } else if (back.type !== 'done') peek.innerHTML = `<p class="sub">couldn't unpack it to show: ${esc(back.message || '')}</p>`;
    }
  };

  // ---------- drop zone ----------
  ['dragenter', 'dragover'].forEach((t) => drop.addEventListener(t, (e) => { e.preventDefault(); drop.classList.add('over'); }));
  ['dragleave', 'drop'].forEach((t) => drop.addEventListener(t, (e) => { e.preventDefault(); drop.classList.remove('over'); }));
  drop.addEventListener('drop', (e) => { const f = e.dataTransfer.files[0]; if (f) run(f); });
  drop.addEventListener('click', () => input.click());
  input.addEventListener('change', () => { if (input.files[0]) run(input.files[0]); input.value = ''; });
  window.__compress = run; window.__face = face; window.__codec = { compressAll, decompressAll, POOL };   // for testing
})();
