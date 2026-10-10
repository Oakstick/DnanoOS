/**
 * Procedural lo-fi backing track. Nothing is sampled: a scheduler walks a
 * four-bar loop of sixteenths and builds each hit out of oscillators and
 * noise, so the whole soundtrack costs a few kilobytes of code.
 *
 * What makes it read as lo-fi rather than as a chiptune is the treatment
 * around the notes. Everything runs through one gentle low-pass so the top
 * end is rolled off like a worn tape, the filter and the pad detune drift
 * under slow LFOs for warble, the offbeats are swung late, and a loop of
 * sparse crackle sits underneath. The chords are the joyful part: a plain
 * I-vi-IV-V in C, voiced with sevenths and ninths.
 */

/** Beats per minute. Slow enough to sit behind the game rather than drive it. */
const BPM = 78;
/** Sixteenths in the loop: four bars of four beats. */
const LOOP_STEPS = 64;
/** How far ahead notes are queued, and how often the queue is topped up. */
const LOOKAHEAD = 0.25;
const TICK_MS = 45;
/** How late an offbeat sixteenth lands, as a fraction of a sixteenth. */
const SWING = 0.34;

/** Semitones above middle C. */
const C4 = 261.626;
const hz = (semitones: number): number => C4 * Math.pow(2, semitones / 12);

interface Chord {
  /** Bass root, down in the second octave. */
  root: number;
  /** Upper voicing, sevenths and ninths rather than plain triads. */
  voicing: number[];
  /** Notes the melody may pick from over this chord. */
  scale: number[];
}

/** I - vi - IV - V in C major, a bar each. */
const PROGRESSION: Chord[] = [
  { root: -24, voicing: [4, 7, 11, 14], scale: [12, 16, 19, 21, 24] },   // Cmaj9
  { root: -15, voicing: [7, 12, 16, 19], scale: [12, 14, 16, 19, 21] },  // Am7
  { root: -19, voicing: [9, 12, 16, 21], scale: [12, 14, 17, 19, 21] },  // Fmaj9
  { root: -17, voicing: [5, 11, 14, 17], scale: [11, 14, 17, 19, 23] },  // G7
];

export class Music {
  private readonly ctx: AudioContext;
  private readonly bus: GainNode;
  private readonly gain: GainNode;
  private readonly tone: BiquadFilterNode;
  private readonly noise: AudioBuffer;
  private crackle: AudioBufferSourceNode | null = null;
  private timer: number | null = null;
  private step = 0;
  private nextTime = 0;
  private readonly stepDur = 60 / BPM / 4;

  constructor(ctx: AudioContext, destination: AudioNode, level = 0.5) {
    this.ctx = ctx;

    this.gain = ctx.createGain();
    this.gain.gain.value = level;
    this.gain.connect(destination);

    // One soft low-pass over everything is most of the lo-fi character.
    this.tone = ctx.createBiquadFilter();
    this.tone.type = 'lowpass';
    this.tone.frequency.value = 3600;
    this.tone.Q.value = 0.6;
    this.tone.connect(this.gain);

    this.bus = ctx.createGain();
    this.bus.gain.value = 1;
    this.bus.connect(this.tone);

    // Tape warble: the filter breathes, which makes the loop feel unsteady
    // in the way a worn cassette is, without bending the pitch audibly.
    const wobble = ctx.createOscillator();
    wobble.frequency.value = 0.23;
    const wobbleDepth = ctx.createGain();
    wobbleDepth.gain.value = 260;
    wobble.connect(wobbleDepth).connect(this.tone.frequency);
    wobble.start();

    this.noise = this.buildNoise();
    this.startCrackle();
  }

  /** Begins the loop, or does nothing if it is already running. */
  start(): void {
    if (this.timer !== null) return;
    this.nextTime = this.ctx.currentTime + 0.08;
    this.timer = window.setInterval(() => this.pump(), TICK_MS);
  }

  stop(): void {
    if (this.timer !== null) window.clearInterval(this.timer);
    this.timer = null;
  }

  /** Fades the music under, for a pause or a death, and back up again. */
  setLevel(level: number, seconds = 0.4): void {
    this.gain.gain.setTargetAtTime(level, this.ctx.currentTime, seconds / 3);
  }

  // ------------------------------------------------------------ scheduling

  private pump(): void {
    const until = this.ctx.currentTime + LOOKAHEAD;
    while (this.nextTime < until) {
      this.playStep(this.step, this.nextTime);
      this.nextTime += this.stepDur;
      this.step = (this.step + 1) % LOOP_STEPS;
    }
  }

  private playStep(step: number, at: number): void {
    const chord = PROGRESSION[Math.floor(step / 16)];
    const beat = step % 16;
    // Offbeat sixteenths land late; that lilt is the whole feel of the genre.
    const t = at + (beat % 4 === 2 ? this.stepDur * SWING : 0) + Math.random() * 0.006;

    if (beat === 0 || beat === 8) this.kick(t, beat === 0 ? 0.42 : 0.34);
    if (beat === 4 || beat === 12) this.snare(t);
    if (beat === 14 && Math.random() < 0.4) this.kick(t, 0.2);
    if (beat % 2 === 0) this.hat(t, beat % 4 === 0 ? 0.07 : 0.045);

    if (beat === 0) {
      this.bass(t, chord.root, this.stepDur * 3);
      this.keys(t, chord.voicing, 0.3);
    }
    if (beat === 6) this.keys(t, chord.voicing, 0.17);
    if (beat === 10) this.bass(t, chord.root + 7, this.stepDur * 2);
    if (beat === 14) this.bass(t, chord.root + 12, this.stepDur);

    // A sparse melody, picked fresh each bar so the loop never quite repeats.
    if ((beat === 2 || beat === 7 || beat === 11) && Math.random() < 0.45) {
      const note = chord.scale[Math.floor(Math.random() * chord.scale.length)];
      this.lead(t, note);
    }
  }

  // ---------------------------------------------------------------- voices

  /** Soft sine thump with the pitch dropping away under it. */
  private kick(t: number, vol: number): void {
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = 'sine';
    o.frequency.setValueAtTime(125, t);
    o.frequency.exponentialRampToValueAtTime(42, t + 0.11);
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.001, t + 0.22);
    o.connect(g).connect(this.bus);
    o.start(t);
    o.stop(t + 0.25);
  }

  /** Brushed backbeat: a band of noise, short and dry. */
  private snare(t: number): void {
    const src = this.ctx.createBufferSource();
    src.buffer = this.noise;
    src.playbackRate.value = 1.4;
    const f = this.ctx.createBiquadFilter();
    f.type = 'bandpass';
    f.frequency.value = 1700;
    f.Q.value = 0.8;
    const g = this.ctx.createGain();
    g.gain.setValueAtTime(0.22, t);
    g.gain.exponentialRampToValueAtTime(0.001, t + 0.13);
    src.connect(f).connect(g).connect(this.bus);
    src.start(t, Math.random() * 1.5, 0.2);
  }

  private hat(t: number, vol: number): void {
    const src = this.ctx.createBufferSource();
    src.buffer = this.noise;
    src.playbackRate.value = 2.4;
    const f = this.ctx.createBiquadFilter();
    f.type = 'highpass';
    // Stays under the tape filter above, or the hats vanish into it entirely.
    f.frequency.value = 3000;
    const g = this.ctx.createGain();
    g.gain.setValueAtTime(vol, t);
    g.gain.exponentialRampToValueAtTime(0.001, t + 0.05);
    src.connect(f).connect(g).connect(this.bus);
    src.start(t, Math.random() * 1.5, 0.08);
  }

  /** Round triangle bass, well below everything else. */
  private bass(t: number, semitones: number, dur: number): void {
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = 'triangle';
    o.frequency.value = hz(semitones);
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(0.2, t + 0.02);
    g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    o.connect(g).connect(this.bus);
    o.start(t);
    o.stop(t + dur + 0.05);
  }

  /** The chord itself: each note detuned a little, which is what blurs it. */
  private keys(t: number, voicing: number[], vol: number): void {
    for (const semitones of voicing) {
      const o = this.ctx.createOscillator();
      const g = this.ctx.createGain();
      o.type = 'triangle';
      o.frequency.value = hz(semitones);
      o.detune.value = (Math.random() - 0.5) * 14;
      const dur = 1.1 + Math.random() * 0.3;
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(vol / voicing.length, t + 0.05);
      g.gain.exponentialRampToValueAtTime(0.001, t + dur);
      o.connect(g).connect(this.bus);
      o.start(t);
      o.stop(t + dur + 0.05);
    }
  }

  private lead(t: number, semitones: number): void {
    const o = this.ctx.createOscillator();
    const g = this.ctx.createGain();
    o.type = 'sine';
    o.frequency.value = hz(semitones);
    o.detune.value = (Math.random() - 0.5) * 10;
    const dur = 0.4 + Math.random() * 0.4;
    g.gain.setValueAtTime(0.0001, t);
    g.gain.exponentialRampToValueAtTime(0.15, t + 0.03);
    g.gain.exponentialRampToValueAtTime(0.001, t + dur);
    o.connect(g).connect(this.bus);
    o.start(t);
    o.stop(t + dur + 0.05);
  }

  // ------------------------------------------------------------ vinyl bed

  /**
   * Three seconds of near-silence with scattered pops and a whisper of hiss,
   * looped forever. Real crackle is sparse impulses, not continuous noise,
   * which is why a plain noise source sounds like rain instead of vinyl.
   */
  private buildNoise(): AudioBuffer {
    const rate = this.ctx.sampleRate;
    const buf = this.ctx.createBuffer(1, rate * 3, rate);
    const d = buf.getChannelData(0);
    for (let i = 0; i < d.length; i++) d[i] = (Math.random() * 2 - 1) * 0.06;
    for (let n = 0; n < 420; n++) {
      const at = Math.floor(Math.random() * (d.length - 40));
      const amp = 0.25 + Math.random() * 0.6;
      for (let i = 0; i < 24; i++) d[at + i] += amp * (Math.random() * 2 - 1) * (1 - i / 24);
    }
    return buf;
  }

  private startCrackle(): void {
    const src = this.ctx.createBufferSource();
    src.buffer = this.noise;
    src.loop = true;
    const f = this.ctx.createBiquadFilter();
    f.type = 'bandpass';
    f.frequency.value = 3200;
    f.Q.value = 0.5;
    const g = this.ctx.createGain();
    g.gain.value = 0.05;
    src.connect(f).connect(g).connect(this.gain);
    src.start();
    this.crackle = src;
  }

  dispose(): void {
    this.stop();
    this.crackle?.stop();
    this.crackle = null;
  }
}
