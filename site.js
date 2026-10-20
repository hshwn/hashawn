// site.js — choreography: the ball on the hero, rolling to the corner on scroll,
// posters you pull out of the grid, and him rolling over to talk about them.
(() => {
  const canvas = document.getElementById('face');
  const hero = document.getElementById('hero');
  const bubble = document.getElementById('bubble');
  const bubbleTxt = bubble.querySelector('.txt');
  const shade = document.getElementById('shade');
  const muteBtn = document.getElementById('mute');
  const phone = () => matchMedia('(max-width: 720px)').matches;

  const LINES = {
    hello: "hi! i'm hassan. well, my face.",
    poke: ["ow.", "stop that.", "hey!", "my face."],
    dizzy: "ok that's enough spinning",
  };

  const face = Face.create(canvas, {
    open_closed: 'assets/face/open_closed.webp', open_open: 'assets/face/open_open.webp',
    shut_closed: 'assets/face/shut_closed.webp', shut_open: 'assets/face/shut_open.webp',
    normal: 'assets/face/normal.webp',
  }, () => { setTimeout(() => say(LINES.hello), 600); });
  if (!face) { canvas.remove(); return; }
  window.__face = face;

  // ---------- where he stands ----------
  let mode = 'hero';                 // hero | corner | talking
  let openPoster = null;
  const heroPose = () => {
    const w = innerWidth, h = hero.getBoundingClientRect().height;
    const r = phone() ? Math.min(w * 0.40, h * 0.26) : Math.min(w, h) * 0.29;
    const logoBottom = h * 0.16 + (w * 0.9) * (205 / 906) / 2;          // phone logo: top 16%, 90vw wide
    return { x: w * 0.5, y: phone() ? logoBottom - 14 + r : h * 0.51, r };
  };
  const cornerPose = () => { const r = phone() ? 44 : 56; return { x: innerWidth - r - (phone() ? 18 : 36), y: innerHeight - r - (phone() ? 18 : 36), r }; };
  const talkPose = (rect) => {
    const r = phone() ? 46 : 72;
    const clamp = (p) => ({ x: Math.max(r + 8, Math.min(innerWidth - r - 8, p.x)), y: Math.max(r + 8, Math.min(innerHeight - r - 8, p.y)), r });
    if (!phone() && rect.left > r * 2.6) return clamp({ x: rect.left - r * 1.4, y: rect.bottom - r * 0.9, r });
    return clamp({ x: r + 22, y: innerHeight - r - 22, r });
  };
  const place = () => {
    if (mode === 'talking' && openPoster) { const t = talkPose(openPoster.getBoundingClientRect()); face.target(t.x, t.y, t.r); return; }
    const hp = heroPose(), cp = cornerPose();
    const p = Math.max(0, Math.min(1, scrollY / (hero.getBoundingClientRect().height * 0.55)));
    const e = p < 0.5 ? 2 * p * p : 1 - Math.pow(-2 * p + 2, 2) / 2;
    face.target(hp.x + (cp.x - hp.x) * e, (hp.y - scrollY) + (cp.y - (hp.y - scrollY)) * e, hp.r + (cp.r - hp.r) * e);
    mode = e > 0.98 ? 'corner' : 'hero';
  };
  { const hp = heroPose(); face.snap(hp.x, hp.y, hp.r); }
  addEventListener('scroll', place, { passive: true });
  addEventListener('resize', place);
  place();

  // ---------- bubble ----------
  const placeBubble = () => {
    const half = bubble.offsetWidth / 2, x = Math.max(half + 8, Math.min(innerWidth - half - 8, face.x));
    bubble.style.left = `${x}px`; bubble.style.top = `${face.y - face.r - 14}px`;
    bubble.style.setProperty('--tail', `${face.x - x}px`);            // tail stays on him
  };
  let speaking = false;
  const say = (text) => {
    bubble.hidden = false; bubble.classList.remove('done'); bubbleTxt.textContent = '';
    speaking = true;
    Voice.say(text, {
      onChar: (i) => { bubbleTxt.textContent = text.slice(0, i + 1); },
      onMouth: (o) => { face.mouthOpen = o; },
      onEnd: () => { speaking = false; bubble.classList.add('done'); setTimeout(() => { if (!speaking) bubble.hidden = true; }, 2200); },
    });
  };
  const hush = () => { Voice.stop(); face.mouthOpen = false; speaking = false; bubble.hidden = true; };

  // ---------- pointer on the ball ----------
  const onBall = (px, py) => Math.hypot(px - face.x, py - face.y) <= face.r;
  let drag = null, lastMove = 0;
  addEventListener('pointermove', (e) => {
    lastMove = performance.now();
    if (drag) {
      const dx = e.clientX - drag.x, dy = e.clientY - drag.y;
      drag.dist += Math.hypot(dx, dy);
      face.drag(dx, dy); drag.vx = dx; drag.vy = dy; drag.x = e.clientX; drag.y = e.clientY;
      return;
    }
    canvas.classList.toggle('hot', onBall(e.clientX, e.clientY));
    face.look(e.clientX, e.clientY);
  }, { passive: true });
  canvas.addEventListener('pointerdown', (e) => {
    if (!onBall(e.clientX, e.clientY)) return;
    Voice.unlock();
    drag = { x: e.clientX, y: e.clientY, vx: 0, vy: 0, dist: 0 };
    canvas.classList.add('grabbing'); try { canvas.setPointerCapture(e.pointerId); } catch {}
  });
  const endDrag = () => {
    if (!drag) return;
    canvas.classList.remove('grabbing');
    if (drag.dist < 6) {                                   // a tap: poke
      face.poke(); face.free = false;
      if (!speaking) say(LINES.poke[Math.floor(Math.random() * LINES.poke.length)]);
    } else {
      face.release(drag.vx, drag.vy);
      if (face.dizzy && !speaking) setTimeout(() => { if (!speaking) say(LINES.dizzy); }, 900);
    }
    drag = null;
  };
  canvas.addEventListener('pointerup', endDrag);
  canvas.addEventListener('pointercancel', endDrag);
  // idle: after a while with no pointer, glance around
  setInterval(() => { if (!drag && !face.free && performance.now() - lastMove > 4000) face.look(face.x + (Math.random() - 0.5) * 600, face.y + (Math.random() - 0.5) * 300); }, 2600);

  // ---------- posters: pull one out ----------
  const posters = [...document.querySelectorAll('.poster')];
  const openIt = (el) => {
    if (openPoster) return;
    openPoster = el;
    const r0 = el.getBoundingClientRect();
    const slot = document.createElement('div'); slot.className = 'slot'; slot.style.height = `${r0.height}px`;
    el._slot = slot; el.after(slot);
    el.style.transform = ''; el.classList.remove('lifted'); el.classList.add('open');
    const r1 = el.getBoundingClientRect();
    el.animate([
      { transform: `translate(-50%, -50%) translate(${r0.left - r1.left}px, ${r0.top - r1.top}px) scale(${r0.width / r1.width})` },
      { transform: 'translate(-50%, -50%)' },
    ], { duration: 420, easing: 'cubic-bezier(.2,.9,.25,1.1)' });
    shade.hidden = false;
    mode = 'talking'; place();
    el._justOpened = true; setTimeout(() => { el._justOpened = false; }, 400);
    sayTimer = setTimeout(() => say(el.dataset.line), 350);
  };
  let sayTimer = null;
  const closeIt = () => {
    const el = openPoster; if (!el) return;
    clearTimeout(sayTimer); hush();
    const r1 = el.getBoundingClientRect();
    const slot = el._slot, r0 = slot.getBoundingClientRect();
    const a = el.animate([
      { transform: 'translate(-50%, -50%)' },
      { transform: `translate(-50%, -50%) translate(${r0.left - r1.left}px, ${r0.top - r1.top}px) scale(${r0.width / r1.width})` },
    ], { duration: 340, easing: 'cubic-bezier(.3,.8,.3,1)' });
    shade.hidden = true;
    a.onfinish = () => { el.classList.remove('open'); slot.remove(); el._slot = null; };
    openPoster = null; mode = 'hero'; place();
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
    el.addEventListener('click', (e) => { e.preventDefault(); if (el.classList.contains('open') && !el._justOpened) closeIt(); });
    el.addEventListener('dragstart', (e) => e.preventDefault());
  });

  // ---------- mute ----------
  try { if (localStorage.getItem('mute') === '1') { Voice.setMuted(true); muteBtn.setAttribute('aria-pressed', 'true'); } } catch {}
  muteBtn.addEventListener('click', () => {
    const m = !Voice.muted; Voice.setMuted(m); muteBtn.setAttribute('aria-pressed', String(m));
    try { localStorage.setItem('mute', m ? '1' : '0'); } catch {}
  });

  // bubble follows him every frame
  const tick = () => { if (mode === 'talking') place(); if (!bubble.hidden) placeBubble(); requestAnimationFrame(tick); };
  tick();
})();
