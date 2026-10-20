// blog.js — every post is its own page at /blog/<slug>/. Posts are Markdown files in
// /posts/<slug>.md, listed newest first in /posts.json. No build step: Cloudflare Pages
// rewrites /blog/* to blog/index.html (see _redirects) and this script fills the page.
(() => {
  const page = document.getElementById('page');
  const esc = (s) => s.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

  // ---------- tiny markdown ----------
  const inline = (s) => esc(s)
    .replace(/!\[([^\]]*)\]\(([^)]+)\)/g, '<img src="$2" alt="$1" loading="lazy">')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2">$1</a>')
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
    .replace(/(^|[^*])\*([^*]+)\*/g, '$1<i>$2</i>');
  const md = (src) => {
    const lines = src.replace(/\r/g, '').split('\n'), out = [];
    let i = 0, para = [], list = null;
    const flush = () => { if (para.length) { out.push(`<p>${inline(para.join(' '))}</p>`); para = []; } if (list) { out.push(`</${list}>`); list = null; } };
    while (i < lines.length) {
      const l = lines[i];
      if (/^```/.test(l)) { flush(); const buf = []; i++; while (i < lines.length && !/^```/.test(lines[i])) buf.push(lines[i++]); out.push(`<pre><code>${esc(buf.join('\n'))}</code></pre>`); i++; continue; }
      const h = l.match(/^(#{1,3})\s+(.*)/);
      if (h) { flush(); const lv = Math.min(3, Math.max(2, h[1].length)); out.push(`<h${lv}>${inline(h[2])}</h${lv}>`); i++; continue; }   // # and ## are sections, ### a sub
      if (/^---+\s*$/.test(l)) { flush(); out.push('<hr>'); i++; continue; }
      if (/^>/.test(l)) { flush(); const buf = []; while (i < lines.length && /^>/.test(lines[i])) buf.push(lines[i++].replace(/^>\s?/, '')); out.push(`<blockquote>${inline(buf.join(' '))}</blockquote>`); continue; }
      const li = l.match(/^(\s*)([-*]|\d+\.)\s+(.*)/);
      if (li) { const kind = /\d/.test(li[2]) ? 'ol' : 'ul'; if (list !== kind) { if (para.length) { out.push(`<p>${inline(para.join(' '))}</p>`); para = []; } if (list) out.push(`</${list}>`); out.push(`<${kind}>`); list = kind; } out.push(`<li>${inline(li[3])}</li>`); i++; continue; }
      if (!l.trim()) { flush(); i++; continue; }
      if (list) flush();
      para.push(l.trim()); i++;
    }
    flush();
    return out.join('\n');
  };
  const front = (src) => {
    const m = src.match(/^---\n([\s\S]*?)\n---\n?/);
    const meta = {};
    if (m) for (const line of m[1].split('\n')) { const k = line.match(/^(\w+):\s*(.*)$/); if (k) meta[k[1]] = k[2].trim(); }
    return { meta, body: m ? src.slice(m[0].length) : src };
  };
  const firstLine = (body) => { const p = body.split('\n').map((l) => l.trim()).find((l) => l && !/^[#>\-*`!\d]/.test(l)); return p ? p.replace(/!\[[^\]]*\]\([^)]*\)/g, '').replace(/\[([^\]]+)\]\([^)]*\)/g, '$1').replace(/[*_`]/g, '').slice(0, 160) : ''; };
  const nice = (d) => { const t = new Date(d + 'T12:00:00'); return isNaN(t) ? d : t.toLocaleDateString('en-GB', { day: 'numeric', month: 'long', year: 'numeric' }); };

  // ---------- him, small, in the corner ----------
  const canvas = document.getElementById('face');
  const bubble = document.getElementById('bubble'), bubbleTxt = bubble.querySelector('.txt');
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
    Voice.say(text, {
      onChar: (i) => { bubbleTxt.textContent = text.slice(0, i + 1); },
      onMouth: (o) => { face.mouthOpen = o; },
      onEnd: () => { bubble.classList.add('done'); hideTimer = setTimeout(() => { bubble.hidden = true; }, 2600); },
    });
  };
  addEventListener('pointerdown', () => Voice.unlock(), { once: true });

  // ---------- routing: /blog/<slug>/ (or ?p=slug as a fallback) ----------
  const m = location.pathname.match(/^\/blog\/([^/]+)\/?$/);
  const slug = (m && m[1] !== 'index.html') ? decodeURIComponent(m[1]) : new URLSearchParams(location.search).get('p');
  const load = async (s) => { const r = await fetch(`/posts/${s}.md`, { cache: 'no-cache' }); if (!r.ok) throw new Error(s); return front(await r.text()); };

  (async () => {
    let slugs = [];
    try { slugs = await (await fetch('/posts.json', { cache: 'no-cache' })).json(); } catch {}
    slugs = slugs.filter((s) => !s.startsWith('_'));

    if (slug) {
      try {
        const { meta, body } = await load(slug);
        document.title = `${meta.title || slug} — hashawn!`;
        page.innerHTML = `<article class="post"><header><h1>${esc(meta.title || slug)}</h1><p class="date">${nice(meta.date || '')}</p></header>${md(body)}</article>`;
        if (meta.say) setTimeout(() => say(meta.say), 700);
      } catch { page.innerHTML = `<article class="post"><header><h1>not here</h1></header><p>no post called <code>${esc(slug)}</code>. <a href="/blog/">all posts</a></p></article>`; }
      return;
    }

    if (!slugs.length) {
      page.innerHTML = `<section class="index"><h1>blog</h1><p class="muted">nothing written yet.</p></section>`;
      setTimeout(() => say("i haven't written anything yet. soon."), 900);
      return;
    }
    const items = await Promise.all(slugs.map(async (s) => { try { const { meta, body } = await load(s); return { s, meta, line: firstLine(body) }; } catch { return null; } }));
    page.innerHTML = `<section class="index"><h1>blog</h1>` + items.filter(Boolean).map(({ s, meta, line }) =>
      `<a class="entry" href="/blog/${encodeURIComponent(s)}/"><h2>${esc(meta.title || s)}</h2><p class="date">${nice(meta.date || '')}</p>${line ? `<p class="line">${esc(line)}</p>` : ''}</a>`).join('') + `</section>`;
  })();
})();
