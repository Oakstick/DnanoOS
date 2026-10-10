"""The recorded soundtracks' tempo and downbeat, for `Soundtrack::bpm` and `Soundtrack::downbeat`.

The onset strength is the spectral flux (the rise of the log magnitude spectrum, 200 frames a
second) less its local mean. The tempo is the lag whose autocorrelation, summed over its first four
multiples, is strongest between 60 and 200 bpm (checked on each third of the song too, so a song
that changes pace shows it); the beat phase is where the onsets gather on that grid, and of its
four beats the downbeat is the one the low band (under 200 Hz) leans on.

    python tools/track_tempo.py assets/music/*.ogg            # measure
    python tools/track_tempo.py assets/music/boss.ogg=93.4    # a phase for a tempo you chose

Needs numpy and soundfile. Octaves are ambiguous by ear and by numbers alike: the game keeps the
pulse between 80 and 160 bpm (a song measured at 186.8 is written as 93.4)."""
import sys

import numpy as np
import soundfile as sf

FPS = 200.0


def flux(path, below_hz=None):
    x, sr = sf.read(path, dtype='float32', always_2d=True)
    x = x.mean(axis=1)
    hop, n = int(sr / FPS), 2048
    win = np.hanning(n).astype(np.float32)
    frames = (len(x) - n) // hop
    freqs = np.fft.rfftfreq(n, 1 / sr)
    sel = freqs < below_hz if below_hz else np.ones_like(freqs, bool)
    out, prev = [], None
    for s in range(0, frames, 4000):
        idx = np.arange(n)[None, :] + hop * np.arange(s, min(frames, s + 4000))[:, None]
        mag = np.log1p(1000 * np.abs(np.fft.rfft(x[idx] * win, axis=1)))[:, sel]
        if prev is not None:
            mag = np.vstack([prev[None, :], mag])
        d = np.maximum(mag[1:] - mag[:-1], 0).sum(axis=1)
        if prev is None:
            d = np.concatenate([[0], d])
        prev = mag[-1]
        out.append(d)
    o = np.concatenate(out)
    w = int(FPS * 0.5)
    return np.maximum(o - np.convolve(o, np.ones(2 * w + 1) / (2 * w + 1), mode='same'), 0), len(x) / sr


def acf(o, lag):
    i, f = int(lag), lag - int(lag)
    a = o[:len(o) - i - 1]
    b = o[i:len(o) - 1] * (1 - f) + o[i + 1:] * f
    return float((a * b).sum() / len(a))


def tempo(o):
    bpms = np.arange(60, 200, 0.05)
    sc = np.array([sum(acf(o, FPS * 60 / b * m) for m in (1, 2, 3, 4)) / 4 for b in bpms])
    return bpms, sc


def comb(o, phase, period):
    xs = np.arange(phase, len(o) - 1, period)
    return np.interp(xs, np.arange(len(o)), o).sum()


def downbeat(path, o, bpm):
    period = FPS * 60 / bpm
    phase = max((period * k / 400 for k in range(400)), key=lambda p: comb(o, p, period))
    low, _ = flux(path, 200)
    bars = [phase + period * k for k in range(4)]
    return phase / FPS, bars[int(np.argmax([comb(low, b, period * 4) for b in bars]))] / FPS


for arg in sys.argv[1:]:
    path, _, chosen = arg.partition('=')
    o, dur = flux(path)
    if chosen:
        bpm = float(chosen)
    else:
        bpms, sc = tempo(o)
        picks = []
        for i in np.argsort(-sc):
            if all(abs(bpms[i] - q) > 2 for q in picks):
                picks.append(float(bpms[i]))
            if len(picks) == 4:
                break
        third = len(o) // 3
        thirds = [float(tempo(o[t * third:(t + 1) * third])[0][np.argmax(tempo(o[t * third:(t + 1) * third])[1])]) for t in range(3)]
        bpm = picks[0]
        while bpm > 160:
            bpm /= 2
        while bpm < 80:
            bpm *= 2
        print(f"{path}: {dur:.1f} s; candidates {[round(p, 2) for p in picks]}, by thirds {[round(t, 2) for t in thirds]}")
    beat, bar = downbeat(path, o, bpm)
    print(f"{path}: {bpm:.2f} bpm, first beat {beat:.3f} s, downbeat {bar:.3f} s")
