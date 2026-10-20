// voice.js — Animal-Crossing-style babble. Deterministic: the same text always
// sounds the same. Each letter is a short pitched beep; vowels ring longer.
// Exposes: Voice.say(text, {onChar, onMouth, onEnd}) → stop(); Voice.unlock(); Voice.muted.
window.Voice = (() => {
  let ctx = null, master = null, muted = false;
  const unlock = () => {
    if (!ctx) {
      ctx = new (window.AudioContext || window.webkitAudioContext)();
      master = ctx.createGain(); master.gain.value = muted ? 0 : 0.35;
      // soften the top end so the beeps sit back a bit
      const lp = ctx.createBiquadFilter(); lp.type = 'lowpass'; lp.frequency.value = 3400; lp.Q.value = 0.7;
      master.connect(lp).connect(ctx.destination);
    }
    if (ctx.state === 'suspended') ctx.resume();
    return ctx;
  };
  const setMuted = (m) => { muted = m; if (master) master.gain.setTargetAtTime(m ? 0 : 0.35, ctx.currentTime, 0.02); };

  const VOWELS = 'aeiouy';
  const BASE = 196;                  // his pitch, roughly a G3
  const hash = (c) => { let h = 2166136261; for (const ch of c) h = Math.imul(h ^ ch.charCodeAt(0), 16777619); return (h >>> 0); };

  // one beep: saw → formant-ish bandpass → gain envelope
  const beep = (t, ch, prev) => {
    const h = hash(ch);
    const semis = (h % 9) - 4;                               // ±4 semitones around base
    const f = BASE * Math.pow(2, semis / 12) * (VOWELS.includes(ch) ? 1 : 1.18);
    const vowel = VOWELS.includes(ch);
    const dur = vowel ? 0.085 : 0.055;
    const osc = ctx.createOscillator(); osc.type = 'sawtooth';
    osc.frequency.setValueAtTime(prev ? prev * 0.9 + f * 0.1 : f, t);
    osc.frequency.exponentialRampToValueAtTime(f, t + 0.02);
    osc.frequency.exponentialRampToValueAtTime(f * (vowel ? 0.94 : 1.03), t + dur);
    const bp = ctx.createBiquadFilter(); bp.type = 'bandpass';
    bp.frequency.value = vowel ? 620 + (h % 400) : 1400 + (h % 900); bp.Q.value = vowel ? 1.1 : 2.2;
    const g = ctx.createGain();
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(vowel ? 1 : 0.55, t + 0.008);
    g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
    osc.connect(bp).connect(g).connect(master);
    osc.start(t); osc.stop(t + dur + 0.01);
    return { f, dur };
  };

  let current = null;
  const say = (text, { onChar, onMouth, onEnd } = {}) => {
    stop();
    unlock();
    const chars = [...text];
    let t = ctx.currentTime + 0.05, prev = null;
    const timers = [];
    const start = performance.now(), t0 = t;
    chars.forEach((ch, i) => {
      const c = ch.toLowerCase();
      const isLetter = /[a-z]/.test(c);
      let step;
      if (isLetter) {
        const b = beep(t, c, prev); prev = b.f;
        step = b.dur + 0.012;
        const at = (t - t0) * 1000, vowel = VOWELS.includes(c);
        timers.push(setTimeout(() => { onChar && onChar(i); onMouth && onMouth(vowel || i % 2 === 0); }, at));
        timers.push(setTimeout(() => { onMouth && onMouth(false); }, at + b.dur * 1000 * 0.9));
      } else {
        step = ch === ' ' ? 0.07 : /[,.!?…]/.test(ch) ? 0.22 : 0.03;
        const at = (t - t0) * 1000;
        timers.push(setTimeout(() => { onChar && onChar(i); onMouth && onMouth(false); }, at));
      }
      t += step;
    });
    timers.push(setTimeout(() => { onMouth && onMouth(false); current = null; onEnd && onEnd(); }, (t - t0) * 1000 + 40));
    current = { timers };
    return stop;
  };
  const stop = () => { if (!current) return; current.timers.forEach(clearTimeout); current = null; };

  return { say, stop, unlock, setMuted, get muted() { return muted; } };
})();
