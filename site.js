// site.js — choreography: the ball on the hero, rolling to the corner on scroll,
// the logo shrinking into the top, posters you pull out of the grid, and him
// rolling over to talk about them.
(() => {
  const canvas = document.getElementById('face');
  const hero = document.getElementById('hero');
  const logo = document.getElementById('lockup');
  const bubble = document.getElementById('bubble');
  const bubbleTxt = bubble.querySelector('.txt');
  const shade = document.getElementById('shade');
  const muteBtn = document.getElementById('mute');
  const pills = document.querySelector('.pills');
  const phone = () => matchMedia('(max-width: 720px)').matches;
  const pick = (a) => a[Math.floor(Math.random() * a.length)];

  // ---------- lines (drafts, rewrite in your voice) ----------
  const LINES = {
    hello: ["hi! i'm hassan. well, my face.", "hello. yes, it's a ball. keep scrolling.", "oh, hi. don't mind me."],
    poke: ["ow.", "stop that.", "hey!", "my face.", "rude.", "boop yourself."],
    dizzy: ["ok that's enough spinning", "i'm gonna be sick", "where am i", "the room is spinning. or i am."],
    corner: ["i'm down here.", "those are mine, by the way.", "go on, pull one out."],
    idle: ["psst.", "spin me.", "still here.", "you can drag the posters out, you know.", "i'm a ball. deal with it."],
    unmute: ["oh hi.", "can you hear me now?"],
    bye: ["ok bye.", "that one's good right?", "next."],
  };

  const face = Face.create(canvas, {
    open_closed: 'assets/face/open_closed.webp', open_open: 'assets/face/open_open.webp',
    shut_closed: 'assets/face/shut_closed.webp', shut_open: 'assets/face/shut_open.webp',
    normal: 'assets/face/normal.webp',
  }, () => { setTimeout(() => say(pick(LINES.hello)), 600); });
  if (!face) { canvas.remove(); return; }
  window.__face = face;

  // ---------- geometry ----------
  // the logo lives on top of him in the hero, then shrinks into the top bar on scroll
  const logoHero = () => { const w = innerWidth, h = hero.getBoundingClientRect().height; const lw = phone() ? w * 0.9 : Math.min(620, w * 0.74); return { x: w / 2, y: h * (phone() ? 0.16 : 0.22), w: lw }; };
  const logoTop = () => ({ x: innerWidth / 2, y: phone() ? 30 : 36, w: phone() ? 118 : 150 });
  const LOGO_RATIO = 205 / 906;
  const placeLogo = (e) => {
    const a = logoHero(), b = logoTop();
    const w = a.w + (b.w - a.w) * e, x = a.x + (b.x - a.x) * e, y = (a.y - scrollY) + (b.y - (a.y - scrollY)) * e;
    logo.style.width = `${w}px`;
    logo.style.transform = `translate(${x - w / 2}px, ${y - w * LOGO_RATIO / 2}px)`;
    logo.classList.toggle('small', e > 0.9);
    // don't sit on the pills while they scroll past
    const lh = w * LOGO_RATIO, pr = pills.getBoundingClientRect();
    logo.style.opacity = (e > 0.2 && pr.bottom > y - lh / 2 - 6 && pr.top < y + lh / 2 + 6) ? '0' : '1';
  };
  const heroPose = () => {
    const w = innerWidth, h = hero.getBoundingClientRect().height, lg = logoHero();
    const r = phone() ? Math.min(w * 0.40, h * 0.26) : Math.min(w * 0.26, h * 0.31);
    const logoBottom = lg.y + lg.w * LOGO_RATIO / 2;
    return { x: w * 0.5, y: logoBottom - r * 0.14 + r, r };          // his crown tucks 14% of his radius under the logo
  };
  const cornerPose = () => { const r = phone() ? 44 : 58; return { x: innerWidth - r - (phone() ? 18 : 36), y: innerHeight - r - (phone() ? 18 : 36), r }; };
  // beside the open poster (its box includes the caption and strip): to its left when there's
  // room, else below it, else small in the bottom-right corner
  const talkPose = (rect) => {
    let r = phone() ? 42 : 74;
    const clamp = (p) => ({ x: Math.max(r + 8, Math.min(innerWidth - r - 8, p.x)), y: Math.max(r + 8, Math.min(innerHeight - r - 8, p.y)), r });
    if (!phone() && rect.left > r * 2.6) return clamp({ x: rect.left - r * 1.4, y: rect.bottom - r * 0.9, r });
    if (innerHeight - rect.bottom > r * 2 + 16) return clamp({ x: r + 18, y: rect.bottom + r + 8, r });
    r = 40; return clamp({ x: innerWidth - r - 12, y: innerHeight - r - 12, r });
  };
  const progress = () => {
    const p = Math.max(0, Math.min(1, scrollY / (hero.getBoundingClientRect().height * 0.55)));
    return p < 0.5 ? 2 * p * p : 1 - Math.pow(-2 * p + 2, 2) / 2;
  };

  // ---------- where he stands, and what he should face ----------
  let mode = 'hero';                 // hero | corner | talking
  let openPoster = null;
  let trip = null;                   // { A:{x,y}, B:{x,y} } — one planned revolution across it
  const setTrip = (A, B) => { trip = { A, B }; };
  const roll = () => {
    if (!trip) { face.setRoll(0, 1, 0); face.moving = false; return; }
    const dx = trip.B.x - trip.A.x, dy = trip.B.y - trip.A.y, d = Math.hypot(dx, dy) || 1;
    const p = Math.max(0, Math.min(1, ((face.x - trip.A.x) * dx + (face.y - trip.A.y) * dy) / (d * d)));
    face.setRoll(dy / d, dx / d, Math.PI * 2 * (d < 80 ? 0 : 1) * p);
    face.moving = p > 0.03 && p < 0.97;
  };
  // face a screen point: direction from him to it, with some depth so it's a lean, not a stare
  const faceToward = (px, py) => {
    const lx = px - face.x, ly = -(py - face.y);
    const dz = Math.max(Math.hypot(lx, ly) * 1.1, 200);
    face.setFace(lx, ly, dz);
  };
  const facing = () => {
    if (mode === 'talking' && openPoster) { const r = openPoster.getBoundingClientRect(); faceToward(r.left + r.width / 2, r.top + r.height / 2); }
    else if (mode === 'corner') faceToward(innerWidth * 0.42, innerHeight * 0.4);
    else face.setFace(0, 0, 1);
  };
  let said = { corner: false };
  const place = () => {
    const e = progress();
    placeLogo(e);
    if (mode === 'talking' && openPoster) { const t = talkPose(openPoster.getBoundingClientRect()); face.target(t.x, t.y, t.r); return; }
    const hp = heroPose(), cp = cornerPose(), hy = hp.y - scrollY;
    face.target(hp.x + (cp.x - hp.x) * e, hy + (cp.y - hy) * e, hp.r + (cp.r - hp.r) * e);
    setTrip({ x: hp.x, y: hy }, { x: cp.x, y: cp.y });
    const was = mode;
    mode = e > 0.98 ? 'corner' : 'hero';
    if (mode === 'corner' && was !== 'corner' && !said.corner) { said.corner = true; setTimeout(() => say(pick(LINES.corner)), 700); }
  };
  { const hp = heroPose(); face.snap(hp.x, hp.y, hp.r); }
  addEventListener('scroll', place, { passive: true });
  addEventListener('resize', place);
  place();
  logo.addEventListener('click', () => scrollTo({ top: 0, behavior: 'smooth' }));

  // ---------- one voice ----------
  const placeBubble = () => {
    const half = bubble.offsetWidth / 2, x = Math.max(half + 8, Math.min(innerWidth - half - 8, face.x));
    bubble.style.left = `${x}px`; bubble.style.top = `${face.y - face.r - 14}px`;
    bubble.style.setProperty('--tail', `${face.x - x}px`);
  };
  let speaking = false, hideTimer = null;
  let openedAt = 0, closedAt = 0, closeTimer = null, openTimer = null;
  const mouth = (o) => {
    if (o) {
      clearTimeout(closeTimer); closeTimer = null;
      if (face.mouthOpen || openTimer) return;
      const wait = Math.max(0, 70 - (performance.now() - closedAt));
      openTimer = setTimeout(() => { openTimer = null; face.mouthOpen = true; openedAt = performance.now(); }, wait);
      return;
    }
    if (closeTimer || !face.mouthOpen && !openTimer) return;
    const wait = Math.max(110, 120 - (performance.now() - openedAt));
    closeTimer = setTimeout(() => { closeTimer = null; clearTimeout(openTimer); openTimer = null; face.mouthOpen = false; closedAt = performance.now(); }, wait);
  };
  // say(): a new line always replaces the old one cleanly. quip(): only when he's quiet.
  const say = (text) => {
    hush();
    bubble.hidden = false; bubble.classList.remove('done'); bubbleTxt.textContent = '';
    speaking = true;
    Voice.say(text, {
      onChar: (i) => { bubbleTxt.textContent = text.slice(0, i + 1); },
      onMouth: mouth,
      onEnd: () => { speaking = false; bubble.classList.add('done'); hideTimer = setTimeout(() => { if (!speaking) bubble.hidden = true; }, 2200); },
    });
  };
  const quip = (text) => { if (!speaking) say(text); };
  const hush = () => {
    Voice.stop(); clearTimeout(hideTimer); clearTimeout(closeTimer); clearTimeout(openTimer); closeTimer = openTimer = null;
    face.mouthOpen = false; speaking = false; bubble.hidden = true;
  };
  // idle quips, rarely, only when nothing is open
  let lastTouch = performance.now();
  setInterval(() => { if (mode !== 'talking' && performance.now() - lastTouch > 30000 && Math.random() < 0.5) { quip(pick(LINES.idle)); lastTouch = performance.now(); } }, 12000);

  // ---------- pointer on the ball ----------
  const onBall = (px, py) => Math.hypot(px - face.x, py - face.y) <= face.r;
  let drag = null;
  addEventListener('pointermove', (e) => {
    if (drag) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      drag.dist += Math.hypot(dx, dy);
      face.drag(dx, dy); drag.x = e.clientX; drag.y = e.clientY;
      return;
    }
    canvas.classList.toggle('hot', onBall(e.clientX, e.clientY));
  }, { passive: true });
  canvas.addEventListener('pointerdown', (e) => {
    if (!onBall(e.clientX, e.clientY)) return;
    Voice.unlock(); lastTouch = performance.now();
    drag = { x: e.clientX, y: e.clientY, dist: 0 };
    face.grab();
    canvas.classList.add('grabbing'); try { canvas.setPointerCapture(e.pointerId); } catch {}
  });
  const endDrag = () => {
    if (!drag) return;
    canvas.classList.remove('grabbing');
    face.release();
    if (drag.dist < 6) { face.poke(); quip(pick(LINES.poke)); }
    else if (face.dizzy) setTimeout(() => quip(pick(LINES.dizzy)), 900);
    drag = null;
  };
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);

  // ---------- posters: pull one out ----------
  const posters = [...document.querySelectorAll('.poster')];
  let sayTimer = null;
  const openIt = (el) => {
    if (openPoster) return;
    openPoster = el; lastTouch = performance.now();
    const r0 = el.getBoundingClientRect();
    const slot = document.createElement('div'); slot.className = 'slot'; slot.style.height = `${r0.height}px`;
    el._slot = slot; el.after(slot);
    el.style.transform = ''; el.classList.remove('lifted'); el.classList.add('open');
    const big = el.querySelector(':scope > img'); big.dataset.thumb = big.dataset.thumb || big.src; big.dataset.cur = el.getAttribute('href'); big.src = el.getAttribute('href');
    const r1 = el.getBoundingClientRect();
    el.animate([
      { transform: `translate(-50%, -50%) translate(${r0.left - r1.left}px, ${r0.top - r1.top}px) scale(${r0.width / r1.width})` },
      { transform: 'translate(-50%, -50%)' },
    ], { duration: 420, easing: 'cubic-bezier(.2,.9,.25,1.1)' });
    shade.hidden = false; document.body.classList.add('locked');
    hush();
    mode = 'talking'; place();
    setTrip({ x: face.x, y: face.y }, { x: face.tx, y: face.ty });
    el._justOpened = true; setTimeout(() => { el._justOpened = false; }, 400);
    sayTimer = setTimeout(() => say(el.dataset.line), 350);
  };
  const closeIt = () => {
    const el = openPoster; if (!el) return;
    clearTimeout(sayTimer); hush();
    const r1 = el.getBoundingClientRect();
    const slot = el._slot, r0 = slot.getBoundingClientRect();
    const a = el.animate([
      { transform: 'translate(-50%, -50%)' },
      { transform: `translate(-50%, -50%) translate(${r0.left - r1.left}px, ${r0.top - r1.top}px) scale(${r0.width / r1.width})` },
    ], { duration: 340, easing: 'cubic-bezier(.3,.8,.3,1)' });
    shade.hidden = true; document.body.classList.remove('locked');
    a.onfinish = () => {
      el.classList.remove('open'); slot.remove(); el._slot = null;
      const big = el.querySelector(':scope > img'); if (big.dataset.thumb) { big.src = big.dataset.thumb; big.dataset.cur = ''; }
      el.querySelectorAll('.strip button').forEach((o) => o.classList.toggle('cur', o.dataset.src === el.getAttribute('href')));
    };
    openPoster = null; const from = { x: face.x, y: face.y }; mode = 'hero'; place();
    setTrip(from, { x: face.tx, y: face.ty });
    if (Math.random() < 0.4) setTimeout(() => quip(pick(LINES.bye)), 500);
  };
  shade.addEventListener('click', closeIt);
  addEventListener('keydown', (e) => { if (e.key === 'Escape') closeIt(); });

  posters.forEach((el) => {
    let p = null;
    el.addEventListener('pointerdown', (e) => {
      if (el.classList.contains('open') || e.button !== 0) return;
      Voice.unlock();
      p = { x: e.clientX, y: e.clientY, dx: 0, dy: 0, lifted: false, id: e.pointerId };
    });
    el.addEventListener('pointermove', (e) => {
      if (!p) return;
      p.dx = e.clientX - p.x; p.dy = e.clientY - p.y;
      const d = Math.hypot(p.dx, p.dy);
      if (!p.lifted && d > 8 && (!phone() || Math.abs(p.dx) > Math.abs(p.dy) * 1.4)) {
        p.lifted = true; el.classList.add('lifted'); try { el.setPointerCapture(p.id); } catch {}
      }
      if (p.lifted) el.style.transform = `translate(${p.dx}px, ${p.dy}px) scale(1.04) rotate(${p.dx * 0.01}deg)`;
    });
    const up = () => {
      if (!p) return;
      const d = Math.hypot(p.dx, p.dy);
      if (p.lifted && d > 36) openIt(el);
      else if (p.lifted) {
        el.animate([{ transform: el.style.transform }, { transform: 'none' }], { duration: 300, easing: 'cubic-bezier(.2,.9,.3,1.2)' });
        el.style.transform = ''; el.classList.remove('lifted');
      } else if (d < 6) openIt(el);
      p = null;
    };
    el.addEventListener('pointerup', up);
    el.addEventListener('pointercancel', up);
    el.addEventListener('click', (e) => {
      e.preventDefault();
      if (e.target.closest('.meta')) return;                 // caption and strip are not the close target
      if (el.classList.contains('open') && !el._justOpened) closeIt();
    });
    // process strip: tap a version to see it big; the current one is marked
    el.querySelectorAll('.strip button').forEach((b) => {
      b.addEventListener('pointerdown', (e) => e.stopPropagation());
      b.addEventListener('pointerup', (e) => e.stopPropagation());
      b.addEventListener('click', (e) => {
        e.preventDefault(); e.stopPropagation();
        const big = el.querySelector(':scope > img');
        if (b.dataset.src !== big.dataset.cur) { big.dataset.cur = b.dataset.src; big.src = b.dataset.src; }
        el.querySelectorAll('.strip button').forEach((o) => o.classList.toggle('cur', o === b));
        if (b.dataset.say) say(b.dataset.say);
      });
    });
    el.addEventListener('dragstart', (e) => e.preventDefault());
  });

  // ---------- mute ----------
  try { if (localStorage.getItem('mute') === '1') { Voice.setMuted(true); muteBtn.setAttribute('aria-pressed', 'true'); } } catch {}
  muteBtn.addEventListener('click', () => {
    const m = !Voice.muted; Voice.setMuted(m); muteBtn.setAttribute('aria-pressed', String(m));
    try { localStorage.setItem('mute', m ? '1' : '0'); } catch {}
    if (!m) quip(pick(LINES.unmute));
  });

  // ---------- every frame ----------
  const tick = () => {
    if (mode === 'talking' && openPoster) { place(); if (trip) trip.B = { x: face.tx, y: face.ty }; }
    roll(); facing();
    if (!bubble.hidden) placeBubble();
    requestAnimationFrame(tick);
  };
  tick();
})();
