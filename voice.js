// voice.js — Animal-Crossing-style babble, one sound per letter.
// Vowels are little formant vowels (each has its own shape), consonants are clicks,
// hisses or hums by family, and punctuation bends the melody. Deterministic: the
// same text always sings the same tune.
// API: Voice.say(text, {onChar, onMouth, onEnd}) → stop; Voice.stop(); Voice.unlock(); Voice.setMuted(b); Voice.muted
window.Voice = (() => {
  let ctx = null, master = null, muted = false;
  const CFG = { base: 196, rate: 1.0, gain: 0.4 };          // base pitch (Hz), speed, loudness

  const unlock = () => {
    if (!ctx) {
      ctx = new (window.AudioContext || window.webkitAudioContext)();
      master = ctx.createGain(); master.gain.value = muted ? 0 : CFG.gain;
      const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 4200; lp.Q.value = 0.6;
      const comp = ctx.createDynamicsCompressor(); comp.threshold.value = -18; comp.ratio.value = 4; comp.attack.value = 0.003; comp.release.value = 0.08;
      master.connect(lp).connect(comp).connect(ctx.destination);
    }
    if (ctx.state === 'suspended') ctx.resume();
    return ctx;
  };
  const setMuted = (m) => { muted = m; if (master) master.gain.setTargetAtTime(m ? 0 : CFG.gain, ctx.currentTime, 0.02); };

  // ---------- letter families ----------
  // vowel formants (F1, F2) in Hz, roughly: a=ah, e=eh, i=ee, o=oh, u=oo, y=ih
  const VOWEL = { a: [730, 1090], e: [530, 1840], i: [270, 2290], o: [570, 840], u: [300, 870], y: [390, 1990] };
  const PLOSIVE = 'pbtdkgq', FRIC = 'sfhzxcvj', NASAL = 'mnlrw';
  const fam = (c) => VOWEL[c] ? 'v' : PLOSIVE.includes(c) ? 'p' : FRIC.includes(c) ? 'f' : NASAL.includes(c) ? 'n' : null;

  const hash = (s) => { let h = 2166136261; for (const ch of s) h = Math.imul(h ^ ch.charCodeAt(0), 16777619); return (h >>> 0) / 4294967296; };
  const st = (n) => Math.pow(2, n / 12);                     // semitones → ratio

  let noiseBuf = null;
  const noise = () => {
    if (!noiseBuf) { noiseBuf = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate); const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1; }
    const s = ctx.createBufferSource(); s.buffer = noiseBuf; return s;
  };
  const env = (g, t, a, peak, dur) => { g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(peak, t + a); g.gain.exponentialRampToValueAtTime(0.0001, t + dur); };

  // one vowel: pulse-ish source → two formant bandpasses in parallel
  const vowel = (t, c, f0, dur, loud) => {
    const [F1, F2] = VOWEL[c];
    const osc = ctx.createOscillator(); osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(f0 * 1.04, t);
    osc.frequency.exponentialRampToValueAtTime(f0, t + dur * 0.35);
    osc.frequency.exponentialRampToValueAtTime(f0 * 0.965, t + dur);
    const mix = ctx.createGain(); mix.gain.value = 1;
    for (const [F, q, w] of [[F1, 7, 1.0], [F2, 9, 0.55]]) {
      const bp = ctx.createBiquadFilter(); bp.type = 'bandpass'; bp.frequency.value = F; bp.Q.value = q;
      const g = ctx.createGain(); g.gain.value = w;
      osc.connect(bp).connect(g).connect(mix);
    }
    const g = ctx.createGain(); env(g, t, 0.012, loud, dur);
    mix.connect(g).connect(master);
    osc.start(t); osc.stop(t + dur + 0.02);
  };
  // plosive: a click of noise plus a tiny low thump
  const plosive = (t, c, f0, loud) => {
    const n = noise(); const hp = ctx.createBiquadFilter(); hp.type = 'bandpass'; hp.frequency.value = 'ptk'.includes(c) ? 2600 : 1200; hp.Q.value = 1.2;
    const g = ctx.createGain(); env(g, t, 0.002, loud * 0.5, 0.028);
    n.connect(hp).connect(g).connect(master); n.start(t); n.stop(t + 0.04);
    const o = ctx.createOscillator(); o.type = 'sine'; o.frequency.setValueAtTime(f0 * 0.9, t); o.frequency.exponentialRampToValueAtTime(f0 * 0.5, t + 0.04);
    const g2 = ctx.createGain(); env(g2, t, 0.003, loud * 0.5, 0.045);
    o.connect(g2).connect(master); o.start(t); o.stop(t + 0.06);
  };
  // fricative: a hiss, brighter for s/z/c/x, softer for f/h/v/j
  const fric = (t, c, loud, dur) => {
    const n = noise(); const bp = ctx.createBiquadFilter(); bp.type = 'bandpass';
    bp.frequency.value = 'szcx'.includes(c) ? 5200 : 'h' === c ? 1500 : 3000; bp.Q.value = 0.8;
    const g = ctx.createGain(); env(g, t, 0.008, loud * ('h' === c ? 0.25 : 0.4), dur);
    n.connect(bp).connect(g).connect(master); n.start(t); n.stop(t + dur + 0.02);
  };
  // nasal / liquid: a soft hum an octave down, slightly closed
  const nasal = (t, c, f0, loud, dur) => {
    const o = ctx.createOscillator(); o.type = 'triangle'; o.frequency.setValueAtTime(f0, t); o.frequency.exponentialRampToValueAtTime(f0 * 1.03, t + dur);
    const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 'mn'.includes(c) ? 700 : 1400;
    const g = ctx.createGain(); env(g, t, 0.01, loud * 0.7, dur);
    o.connect(lp).connect(g).connect(master); o.start(t); o.stop(t + dur + 0.02);
  };

  // ---------- prosody ----------
  // split into sentences, each ending in . ! ? … or the end; a comma is a dip + pause inside one
  const plan = (text) => {
    const chars = [...text], out = [];
    const seed = hash(text);
    let sentenceStart = 0;
    const flush = (end, ender) => {
      const slice = out.slice(sentenceStart, end);
      const n = slice.length || 1;
      slice.forEach((ev, k) => {
        const p = k / Math.max(1, n - 1);                      // 0..1 through the sentence
        let bend = -1.2 * p;                                   // statements drift down
        let loud = 1;
        if (ender === '?') bend = p > 0.6 ? (p - 0.6) / 0.4 * 5.5 : -0.5 * p;
        if (ender === '!') { bend = 2 + (p > 0.75 ? (p - 0.75) / 0.25 * 2.5 : 0) - (p > 0.95 ? 3 : 0); loud = 1.18; }
        if (ender === '…') bend = -3 * p;
        ev.bend += bend; ev.loud *= loud;
      });
      sentenceStart = out.length;
    };
    let wordStart = true, prevComma = false;
    for (let i = 0; i < chars.length; i++) {
      const raw = chars[i], c = raw.toLowerCase();
      const f = fam(c);
      if (f) {
        const wob = (hash(text + i) - 0.5) * 1.6;                 // seeded wobble, ±0.8 semitone
        let bend = wob + (wordStart ? 1.2 : 0) + (prevComma ? -0.8 : 0);
        out.push({ i, c, f, bend, loud: wordStart ? 1.08 : 1 });
        wordStart = false; prevComma = false;
      } else {
        out.push({ i, c, f: null, pause: c === ' ' ? 0.055 : c === ',' ? 0.16 : c === '.' || c === '!' || c === '?' ? 0.26 : c === '…' ? 0.4 : 0.02 });
        if (c === ' ') wordStart = true;
        if (c === ',') { prevComma = true; wordStart = true; }
        if (c === '.' || c === '!' || c === '?' || c === '…') { flush(out.length - 1, c); wordStart = true; }
        if (c === '.' && chars[i + 1] === '.' ) { /* "..." handled as periods with pauses */ }
      }
    }
    flush(out.length, null);
    return { events: out, seed };
  };

  let current = null;
  const say = (text, { onChar, onMouth, onEnd } = {}) => {
    stop(); unlock();
    const { events, seed } = plan(text);
    const base = CFG.base * st((seed - 0.5) * 1.5);            // each line sits a hair higher or lower
    const timers = [];
    let t = ctx.currentTime + 0.05; const t0 = t;
    const r = 1 / CFG.rate;
    events.forEach((ev) => {
      const at = (t - t0) * 1000;
      if (!ev.f) {
        timers.push(setTimeout(() => { onChar && onChar(ev.i); onMouth && onMouth(false); }, at));
        t += ev.pause * r; return;
      }
      const f0 = base * st(ev.bend), loud = ev.loud;
      let dur;
      if (ev.f === 'v') { dur = 0.082 * r; vowel(t, ev.c, f0, dur, loud); }
      else if (ev.f === 'p') { dur = 0.05 * r; plosive(t, ev.c, f0, loud); }
      else if (ev.f === 'f') { dur = 0.06 * r; fric(t, ev.c, loud, dur); }
      else { dur = 0.055 * r; nasal(t, ev.c, f0 * 0.5, loud, dur); }
      const open = ev.f === 'v' || ev.f === 'n';
      timers.push(setTimeout(() => { onChar && onChar(ev.i); onMouth && onMouth(open); }, at));
      if (open) timers.push(setTimeout(() => { onMouth && onMouth(false); }, at + dur * 1000 * 0.9));
      t += dur + 0.008 * r;
    });
    timers.push(setTimeout(() => { onMouth && onMouth(false); current = null; onEnd && onEnd(); }, (t - t0) * 1000 + 40));
    current = { timers };
    return stop;
  };
  const stop = () => { if (!current) return; current.timers.forEach(clearTimeout); current = null; };

  return { say, stop, unlock, setMuted, get muted() { return muted; }, config: CFG };
})();
