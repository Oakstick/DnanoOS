/**
 * Procedural Web Audio. No sample files needed for the scaffold; swap any
 * play* body for buffer playback later.
 */
export type Sfx =
  | 'nodule' | 'charge' | 'eat' | 'death' | 'clear' | 'start'
  | 'slash' | 'hurt' | 'fuse' | 'explode' | 'arrow' | 'arrowHit' | 'win' | 'jump' | 'heal';

const MASTER_VOLUME = 0.5;

export class AudioManager {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private ambientOsc: OscillatorNode | null = null;
  private ambientGain: GainNode | null = null;
  private frenzyOsc: OscillatorNode | null = null;
  private pickupPitch = 0;
  muted = false;

  constructor() {
    try {
      this.muted = localStorage.getItem('vh.muted') === '1';
    } catch {
      /* storage unavailable */
    }
  }

  /** Must be called from a user gesture. */
  unlock(): void {
    if (this.ctx) {
      if (this.ctx.state === 'suspended') void this.ctx.resume();
      return;
    }
    this.ctx = new AudioContext();
    this.master = this.ctx.createGain();
    this.master.gain.value = this.muted ? 0 : MASTER_VOLUME;
    this.master.connect(this.ctx.destination);
    this.startAmbient();
  }

  toggleMute(): boolean {
    this.muted = !this.muted;
    if (this.master) this.master.gain.value = this.muted ? 0 : MASTER_VOLUME;
    try {
      localStorage.setItem('vh.muted', this.muted ? '1' : '0');
    } catch {
      /* ignore */
    }
    return this.muted;
  }

  private startAmbient(): void {
    if (!this.ctx || !this.master) return;
    this.ambientOsc = this.ctx.createOscillator();
    this.ambientOsc.type = 'sine';
    this.ambientOsc.frequency.value = 42;
    this.ambientGain = this.ctx.createGain();
    this.ambientGain.gain.value = 0.12;
    const lfo = this.ctx.createOscillator();
    lfo.frequency.value = 0.15;
    const lfoGain = this.ctx.createGain();
    lfoGain.gain.value = 6;
    lfo.connect(lfoGain).connect(this.ambientOsc.frequency);
    this.ambientOsc.connect(this.ambientGain).connect(this.master);
    this.ambientOsc.start();
    lfo.start();
  }

  /** 0..1 how close the nearest enemy is; raises the ambient pulse. */
  setThreat(level: number): void {
    if (!this.ctx || !this.ambientGain || !this.ambientOsc) return;
    const now = this.ctx.currentTime;
    this.ambientGain.gain.setTargetAtTime(0.12 + level * 0.25, now, 0.1);
    this.ambientOsc.frequency.setTargetAtTime(42 + level * 30, now, 0.1);
  }

  setFrenzy(active: boolean): void {
    if (!this.ctx || !this.master) return;
    if (active && !this.frenzyOsc) {
      this.frenzyOsc = this.ctx.createOscillator();
      this.frenzyOsc.type = 'triangle';
      this.frenzyOsc.frequency.value = 220;
      const lfo = this.ctx.createOscillator();
      lfo.frequency.value = 7;
      const lg = this.ctx.createGain();
      lg.gain.value = 40;
      lfo.connect(lg).connect(this.frenzyOsc.frequency);
      const g = this.ctx.createGain();
      g.gain.value = 0.08;
      this.frenzyOsc.connect(g).connect(this.master);
      this.frenzyOsc.start();
      lfo.start();
    } else if (!active && this.frenzyOsc) {
      this.frenzyOsc.stop();
      this.frenzyOsc.disconnect();
      this.frenzyOsc = null;
    }
  }

  /**
   * Nugget reward: a coin. Struck metal is inharmonic, so the clink is three
   * partials at bell ratios rather than a chord, decaying almost at once. Over
   * it go the two rising notes an arcade coin is made of, a fourth apart, with
   * a thin shimmer on the second for the ring. The whole thing creeps up in
   * pitch as the hoard grows, so a run of nuggets builds instead of repeating.
   */
  playReward(step: number): void {
    if (!this.ctx || !this.master) return;
    const t = this.ctx.currentTime;
    const base = 988 * Math.pow(1.018, Math.min(step, 24));

    // The clink of metal on metal.
    this.blip(t, base * 1.0, 0.06, 'sine', 0.10);
    this.blip(t, base * 2.76, 0.05, 'sine', 0.07);
    this.blip(t, base * 5.4, 0.04, 'sine', 0.035);

    // The coin itself: two notes, the second held.
    this.blip(t, base, 0.09, 'square', 0.15);
    this.blip(t + 0.07, base * 1.335, 0.24, 'square', 0.14);

    // Shimmer riding on the held note.
    this.blip(t + 0.07, base * 2.67, 0.26, 'sine', 0.045);
    this.blip(t + 0.09, base * 4.0, 0.16, 'sine', 0.025);
  }

  play(sfx: Sfx): void {
    if (!this.ctx || !this.master) return;
    const t = this.ctx.currentTime;
    switch (sfx) {
      case 'nodule':
        this.pickupPitch = (this.pickupPitch + 1) % 2;
        this.blip(t, 520 + this.pickupPitch * 90, 0.05, 'square', 0.12);
        break;
      case 'charge':
        this.sweep(t, 200, 900, 0.35, 'sawtooth', 0.2);
        break;
      case 'heal':
        // A warm rising third, the sound of catching your breath.
        this.blip(t, 523, 0.14, 'sine', 0.18);
        this.blip(t + 0.09, 659, 0.16, 'sine', 0.18);
        this.blip(t + 0.19, 784, 0.26, 'sine', 0.16);
        break;
      case 'eat':
        this.sweep(t, 900, 200, 0.25, 'square', 0.2);
        break;
      case 'death':
        this.sweep(t, 400, 40, 1.1, 'sawtooth', 0.3);
        break;
      case 'slash':
        this.sweep(t, 1400, 500, 0.12, 'sawtooth', 0.12);
        break;
      case 'hurt':
        this.sweep(t, 300, 120, 0.3, 'square', 0.22);
        break;
      case 'fuse':
        this.blip(t, 1200, 0.04, 'square', 0.08);
        break;
      case 'arrow':
        this.sweep(t, 2400, 900, 0.14, 'triangle', 0.14);
        break;
      case 'jump':
        this.sweep(t, 300, 700, 0.16, 'triangle', 0.16);
        break;
      case 'arrowHit':
        this.blip(t, 180, 0.08, 'square', 0.18);
        break;
      case 'explode':
        this.sweep(t, 160, 25, 0.9, 'sawtooth', 0.6);
        this.sweep(t, 900, 60, 0.25, 'square', 0.3);
        break;
      case 'clear':
        [0, 0.12, 0.24, 0.36].forEach((d, i) => this.blip(t + d, 440 * Math.pow(1.25, i), 0.1, 'triangle', 0.2));
        break;
      case 'win':
        [0, 0.15, 0.3, 0.45, 0.75].forEach((d, i) => this.blip(t + d, [523, 659, 784, 1047, 1319][i], 0.3, 'triangle', 0.25));
        break;
      case 'start':
        [0, 0.15].forEach((d, i) => this.blip(t + d, 330 + i * 110, 0.12, 'triangle', 0.2));
        break;
    }
  }

  private blip(t: number, freq: number, dur: number, type: OscillatorType, vol: number): void {
    if (!this.ctx || !this.master) return;
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = type;
    o.frequency.value = freq;
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    o.connect(g).connect(this.master);
    o.start(t);
    o.stop(t + dur + 0.02);
  }

  private sweep(t: number, from: number, to: number, dur: number, type: OscillatorType, vol: number): void {
    if (!this.ctx || !this.master) return;
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = type;
    o.frequency.setValueAtTime(from, t);
    o.frequency.exponentialRampToValueAtTime(to, t + dur);
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    o.connect(g).connect(this.master);
    o.start(t);
    o.stop(t + dur + 0.02);
  }
}
