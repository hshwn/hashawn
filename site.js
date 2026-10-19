// hashawn.com — stickers you can press, drag and peel open.
(() => {
  const sheet = document.getElementById('sheet');
  const stickers = [...document.querySelectorAll('.sticker')];
  const phone = () => matchMedia('(max-width: 720px)').matches;
  const DRAG = 5; // px before a press becomes a drag

  // remembered positions (per viewer, best effort)
  let saved = {};
  try { saved = JSON.parse(localStorage.getItem('stickers') || '{}'); } catch {}
  const remember = (el) => {
    try {
      saved[el.dataset.id] = { dx: el._dx, dy: el._dy };
      localStorage.setItem('stickers', JSON.stringify(saved));
    } catch {}
  };

  stickers.forEach((el) => {
    const s = saved[el.dataset.id];
    el._dx = (s && !phone()) ? s.dx : 0;
    el._dy = (s && !phone()) ? s.dy : 0;
    el.style.setProperty('--dx', `${el._dx}px`);
    el.style.setProperty('--dy', `${el._dy}px`);

    let start = null, moved = false, ox = 0, oy = 0;

    el.addEventListener('pointerdown', (e) => {
      if (e.button !== 0) return;
      start = { x: e.clientX, y: e.clientY };
      ox = el._dx; oy = el._dy; moved = false;
      el.classList.remove('wobble');
      el.classList.add('pressed');
      stickers.forEach((o) => o.classList.remove('top'));
      el.classList.add('top');
      el.setPointerCapture(e.pointerId);
    });

    el.addEventListener('pointermove', (e) => {
      if (!start || phone()) return;
      const dx = e.clientX - start.x, dy = e.clientY - start.y;
      if (!moved && Math.hypot(dx, dy) < DRAG) return;
      if (!moved) { moved = true; el.classList.add('dragging'); el.classList.remove('pressed'); }
      // 1:1 with the pointer, no easing
      el._dx = ox + dx; el._dy = oy + dy;
      el.style.setProperty('--dx', `${el._dx}px`);
      el.style.setProperty('--dy', `${el._dy}px`);
    });

    const end = (e) => {
      if (!start) return;
      start = null;
      el.classList.remove('pressed', 'dragging');
      el.classList.add('wobble');
      el.addEventListener('animationend', () => el.classList.remove('wobble'), { once: true });
      if (moved) { remember(el); el._swallow = true; }
    };
    el.addEventListener('pointerup', end);
    el.addEventListener('pointercancel', end);

    // a drag must not count as a click
    el.addEventListener('click', (e) => {
      if (el._swallow) { e.preventDefault(); el._swallow = false; }
    });
    el.addEventListener('dragstart', (e) => e.preventDefault());
  });

  document.querySelectorAll('.poster').forEach((el) => {
    el.addEventListener('click', (e) => { e.preventDefault(); open(el); });
  });

  // lightbox
  const box = document.getElementById('lightbox');
  const img = box && box.querySelector('img');
  const cap = box && box.querySelector('.cap');
  const open = (el) => {
    if (!box) { location.href = el.getAttribute('href'); return; }
    img.src = el.getAttribute('href');
    img.alt = el.querySelector('img').alt;
    cap.textContent = el.dataset.title || '';
    box.hidden = false;
    document.body.style.overflow = 'hidden';
  };
  const close = () => { box.hidden = true; img.src = ''; document.body.style.overflow = ''; };
  if (box) {
    box.addEventListener('click', close);
    addEventListener('keydown', (e) => { if (e.key === 'Escape' && !box.hidden) close(); });
  }

})();
