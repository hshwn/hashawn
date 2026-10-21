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

  const run = async (file) => {
    if (busy) return; busy = true; drop.classList.add('busy');
    const id = ++jobId;
    const isNcz = /\.ncz$/i.test(file.name);
    const buf = await file.arrayBuffer();
    const total = buf.byteLength;
    progress(isNcz ? 'unpacking' : 'compressing', 0, total);
    const done = await new Promise((resolve) => {
      const onMsg = (e) => {
        const m = e.data; if (m.id !== id) return;
        if (m.type === 'progress') progress(isNcz ? 'unpacking' : (m.brain ? 'compressing, with the brain' : 'compressing'), m.done, m.total);
        else if (m.type === 'model') render(`<div class="job"><p class="label">fetching the brain</p><div class="bar"><i style="width:${m.total ? (m.done / m.total * 100).toFixed(1) : 0}%"></i></div><p class="sub">3 MB, once</p></div>`);
        else { worker.removeEventListener('message', onMsg); resolve(m); }
      };
      worker.addEventListener('message', onMsg);
      worker.postMessage({ id, type: isNcz ? 'decompress' : 'compress', buf }, [buf]);
    });
    busy = false; drop.classList.remove('busy');
    if (done.type === 'error') { render(`<div class="job"><p class="label">couldn't do that</p><p class="sub">${esc(done.message)}</p></div>`); say("that's not one of mine."); return; }
    const out = new Uint8Array(done.buf);
    if (isNcz) {
      const name = file.name.replace(/\.ncz$/i, '');
      render(`<div class="job"><p class="label">unpacked</p><p class="big">${fmt(out.length)}</p><p class="sub">${esc(name)} · ${(done.ms / 1000).toFixed(1)}s</p><button class="btn" id="dl">save it</button></div>`);
      document.getElementById('dl').onclick = () => download(out, name);
      say("there it is. every byte.");
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
      <p class="sub">${(done.ms / 1000).toFixed(1)}s · nothing left your browser</p>
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
  window.__compress = run; window.__face = face;   // for testing
})();
