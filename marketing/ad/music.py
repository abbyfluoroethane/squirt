#!/usr/bin/env python3
"""Synthesizes the 30 s soundtrack for the squirt ad (see SCRIPT.md, section 3).

Standard library + numpy only. Writes audio.wav next to this file:
30.000 s, 48 kHz, 16-bit stereo, master peak about -3 dBFS.

120 BPM (beat = 0.5 s, bar = 2.0 s). D minor, lifting to D major at 24.0.
"""
import os
import wave

import numpy as np

SR = 48000
DUR = 30.0
N = int(round(DUR * SR))
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "audio.wav")
rng = np.random.default_rng(20241108)
TWO_PI = 2.0 * np.pi


def T(t):
    """Seconds to sample index (sample-accurate)."""
    return int(round(t * SR))


def mtof(m):
    return 440.0 * 2.0 ** ((m - 69) / 12.0)


def smooth(x):
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


# ---------------------------------------------------------------- filters
def lp_fft(x, fc, order=1):
    """Causal one-pole low-pass (applied `order` times) via zero-padded FFT."""
    n = x.shape[-1]
    nf = 1 << int(np.ceil(np.log2(n + SR)))
    f = np.fft.rfftfreq(nf, 1.0 / SR)
    H = (1.0 / (1.0 + 1j * f / fc)) ** order
    return np.fft.irfft(np.fft.rfft(x, nf, axis=-1) * H, nf, axis=-1)[..., :n]


def hp_fft(x, fc, order=1):
    return x - lp_fft(x, fc, order)


def tv_lp(x, fc):
    """Time-varying one-pole low-pass (sample loop), fc per sample."""
    a = (1.0 - np.exp(-TWO_PI * fc / SR)).tolist()
    xs = x.tolist()
    y = [0.0] * len(xs)
    s = xs[0]
    for i, v in enumerate(xs):
        s += a[i] * (v - s)
        y[i] = s
    return np.array(y)


# ---------------------------------------------------------------- envelope
def adsr(ng, a, d, s, r, peak=1.0):
    """ADSR for a gate of ng samples plus a release of r seconds.
    Smooth attack, exponential decay, cosine release; at least 3 ms in / 5 ms out."""
    an = max(T(a), T(0.003))
    dn = T(d)
    rn = max(T(r), T(0.005))
    i = np.arange(ng)
    env = np.empty(ng)
    att = i < an
    env[att] = peak * smooth(i[att] / an)
    rest = ~att
    if dn > 0:
        env[rest] = s + (peak - s) * np.exp(-3.0 * (i[rest] - an) / dn)
    else:
        env[rest] = s
    level = env[-1] if ng else 0.0
    rel = level * 0.5 * (1.0 + np.cos(np.pi * np.arange(rn) / rn))
    return np.concatenate([env, rel])


# ---------------------------------------------------------------- buses
BUS_NAMES = ["pad", "pluck", "bass", "kick", "hat", "lead", "bell", "fx"]
B = {k: np.zeros((2, N)) for k in BUS_NAMES}
PLUCK_M = np.zeros(N)  # mono feed for the pluck delay
LEAD_M = np.zeros(N)  # mono feed for the lead delay


def put(bus, t, sig, pan=0.0, gain=1.0):
    i = T(t)
    if i >= N:
        return
    j = min(N, i + sig.shape[-1])
    seg = sig[..., : j - i] * gain
    if seg.ndim == 1:
        a = (pan + 1.0) * np.pi / 4.0
        B[bus][0, i:j] += seg * np.cos(a)
        B[bus][1, i:j] += seg * np.sin(a)
    else:
        B[bus][:, i:j] += seg


def put_mono(arr, t, sig, gain=1.0):
    i = T(t)
    if i >= N:
        return
    j = min(N, i + len(sig))
    arr[i:j] += sig[: j - i] * gain


# ---------------------------------------------------------------- voices
CHORDS = {
    "Dm": dict(pad=[50, 57, 62, 65], arp=[62, 65, 69, 74]),
    "Bb": dict(pad=[46, 53, 58, 62], arp=[62, 65, 70, 74]),
    "F": dict(pad=[53, 57, 60, 65], arp=[60, 65, 69, 72]),
    "C": dict(pad=[48, 55, 60, 64], arp=[60, 64, 67, 72]),
    "Gm": dict(pad=[55, 58, 62, 67], arp=[58, 62, 67, 70]),
    "D": dict(pad=[50, 57, 62, 66, 69], arp=[62, 66, 69, 74]),
    "G": dict(pad=[55, 59, 62, 67], arp=[59, 62, 67, 71]),
}


def pad_chord(t0, t1, name, att, rel, peak=1.0, sus=1.0, dec=1.0, gain=1.0):
    """Soft pad: each note is 3 detuned oscillators (sine + gentle harmonics),
    spread across the stereo field, with slow amplitude drift."""
    notes = CHORDS[name]["pad"]
    ng = T(t1) - T(t0)
    env = adsr(ng, att, dec, sus, rel, peak)
    n = len(env)
    t = np.arange(n) / SR
    L = np.zeros(n)
    R = np.zeros(n)
    harm = [1.0, 0.30, 0.10, 0.04]
    for m in notes:
        f = mtof(m)
        for cents, wl, wr in ((-9, 1.0, 0.25), (0, 0.7, 0.7), (9, 0.25, 1.0)):
            ff = f * 2.0 ** (cents / 1200.0)
            ph = rng.uniform(0, TWO_PI)
            lfo = 1.0 + 0.12 * np.sin(TWO_PI * rng.uniform(0.1, 0.3) * t + rng.uniform(0, TWO_PI))
            w = np.zeros(n)
            for h, a in enumerate(harm, 1):
                w += a * np.sin(TWO_PI * ff * h * t + ph * h)
            w *= lfo
            L += w * wl
            R += w * wr
    sc = gain * env / (len(notes) * 1.6)
    put("pad", t0, np.stack([L * sc, R * sc]))


def pluck(m, vel=1.0, tau=0.38, dur=1.5):
    """Felt-like pluck: soft attack, upper harmonics die faster than the fundamental."""
    n = T(dur)
    t = np.arange(n) / SR
    f = mtof(m)
    w = np.zeros(n)
    for h in range(1, 7):
        if f * h < 9000:
            w += (1.0 / h ** 1.3) * np.sin(TWO_PI * f * h * t) * np.exp(-t * (1.0 + 0.9 * (h - 1)) / tau)
    w *= smooth(t / 0.004) * smooth((n - np.arange(n)) / T(0.03))
    return w * vel


def lead_note(t0, m, dur, vel=1.0, att=0.08, rel=0.35):
    """Airy lead: two slightly detuned triangle oscillators with gentle vibrato."""
    ng = T(dur)
    env = adsr(ng, att, 0.25, 0.8, rel)
    n = len(env)
    t = np.arange(n) / SR
    f = mtof(m)
    vib = 1.0 + 0.0035 * smooth((t - 0.15) / 0.5) * np.sin(TWO_PI * 5.2 * t)
    ph = TWO_PI * np.cumsum(f * vib) / SR

    def tri(p):
        return (2.0 / np.pi) * np.arcsin(np.sin(p))

    w = tri(ph) + 0.7 * tri(ph * 1.003 + 1.0)
    sig = w * env * 0.6 * vel
    put("lead", t0, sig, pan=0.12)
    put_mono(LEAD_M, t0, sig)


def bell(t0, m, vel=1.0, tau=1.4, dur=4.0):
    n = T(dur)
    t = np.arange(n) / SR
    f = mtof(m)
    w = np.zeros(n)
    for p, a, d in ((1.0, 1.0, 1.0), (2.0, 0.25, 0.5), (3.0, 0.10, 0.3), (5.4, 0.04, 0.15)):
        w += a * np.sin(TWO_PI * f * p * t) * np.exp(-t / (tau * d))
    w *= smooth(t / 0.003) * smooth((n - np.arange(n)) / T(0.25))
    put("bell", t0, w * vel, pan=0.1)


def sec(t):
    """Section loudness for the rhythm section so each accent steps the level up."""
    return 0.5 if t < 12.0 else (0.75 if t < 18.0 else 1.0)


def kick_hit(t0, vel=1.0):
    """Soft low sine kick: quick pitch drop, no beater click (starts at zero, 3 ms ramp)."""
    n = T(0.5)
    t = np.arange(n) / SR
    f = 46.0 + 62.0 * np.exp(-t / 0.032)
    ph = TWO_PI * np.cumsum(f) / SR
    w = np.sin(ph) * np.exp(-t / 0.14) * smooth(t / 0.003) * smooth((n - np.arange(n)) / T(0.03))
    put("kick", t0, w * vel * (1.0 if vel >= 0.9 and t0 in (6.0, 12.0, 18.0, 24.0) else sec(t0)))


def bass_note(t0, m, dur, vel=1.0):
    ng = T(dur)
    env = adsr(ng, 0.008, 0.15, 0.75, 0.07)
    t = np.arange(len(env)) / SR
    f = mtof(m)
    w = np.sin(TWO_PI * f * t) + 0.2 * np.sin(TWO_PI * 2 * f * t) + 0.07 * np.sin(TWO_PI * 3 * f * t)
    put("bass", t0, w * env * vel * sec(t0))


def hat_hit(t0, vel=1.0, pan=0.0):
    n = T(0.06)
    t = np.arange(n) / SR
    w = rng.standard_normal(n) * np.exp(-t / 0.014) * smooth(t / 0.0015) * smooth((n - np.arange(n)) / T(0.01))
    put("hat", t0, w * vel, pan=pan)


# ---------------------------------------------------------------- arrangement
def build():
    # ---- pad (name, attack, release, peak, sustain, decay, gain)
    pads = [
        (0.0, 3.0, "Dm", 1.8, 0.9, 1.0, 1.0, 1.0, 0.70),
        (3.0, 6.0, "Bb", 1.2, 0.5, 1.0, 1.0, 1.0, 0.75),
        (6.0, 8.0, "Dm", 0.35, 0.8, 1.6, 1.0, 1.5, 0.85),
        (8.0, 10.0, "Bb", 0.9, 0.8, 1.0, 1.0, 1.0, 0.85),
        (10.0, 12.0, "F", 0.9, 0.6, 1.0, 1.0, 1.0, 0.85),
        (12.0, 14.0, "Gm", 0.3, 0.8, 1.6, 1.0, 1.5, 1.00),
        (14.0, 16.0, "Bb", 0.9, 0.8, 1.0, 1.0, 1.0, 1.00),
        (16.0, 18.0, "C", 0.9, 0.5, 1.0, 1.0, 1.0, 1.00),
        (18.0, 20.0, "Dm", 0.3, 0.8, 1.7, 1.0, 1.5, 1.10),
        (20.0, 22.0, "Gm", 0.9, 0.8, 1.0, 1.0, 1.0, 1.10),
        (22.0, 23.95, "Bb", 0.9, 0.05, 1.0, 1.0, 1.0, 1.10),  # holds Bb, ends exactly at 24.0
        (24.0, 26.0, "D", 0.25, 0.8, 1.8, 1.0, 1.8, 1.15),  # D major bloom
        (26.0, 28.0, "G", 0.8, 0.7, 1.0, 1.0, 1.0, 1.10),
        (28.0, 30.0, "D", 0.7, 0.3, 1.0, 1.0, 1.0, 1.00),
    ]
    for t0, t1, name, att, rel, peak, sus, dec, g in pads:
        pad_chord(t0, t1, name, att, rel, peak, sus, dec, g)

    # ---- intro pluck: every 2 beats from 1.0, high register
    for k, (t, m) in enumerate([(1.0, 81), (2.0, 77), (3.0, 74), (4.0, 82), (5.0, 77)]):
        p = pluck(m, 0.55, tau=0.45, dur=1.8)
        put("pluck", t, p, pan=0.2 * (-1) ** k)
        put_mono(PLUCK_M, t, p)

    # ---- bars: (start, chord, bass root midi)
    bars6 = [(6, "Dm", 38), (8, "Bb", 34), (10, "F", 41)]
    bars12 = [(12, "Gm", 43), (14, "Bb", 46), (16, "C", 48)]
    bars18 = [(18, "Dm", 38), (20, "Gm", 43), (22, "Bb", 46)]
    bars24 = [(24, "D", 38), (26, "G", 38)]

    # ---- pluck arpeggios, eighth notes, one chord per bar
    pat = [0, 1, 2, 3, 2, 1, 2, 3]
    vp = [1.0, 0.6, 0.7, 0.85, 0.65, 0.55, 0.72, 0.6]

    def arp(bars, sec_vel, last_end=None):
        for s, name, _ in bars:
            for k in range(8):
                t = s + 0.25 * k
                if last_end is not None and t >= last_end:
                    break
                m = CHORDS[name]["arp"][pat[k]]
                p = pluck(m, vp[k] * sec_vel)
                put("pluck", t, p, pan=0.3 * np.sin(1.7 * k + s))
                put_mono(PLUCK_M, t, p)

    arp(bars6, 0.75)
    arp(bars12, 0.85)
    arp(bars18, 0.9, last_end=23.0)
    arp(bars24[:1], 1.0)  # 24-26 only: D F# A D

    # ---- kick
    kicks = []
    t = 6.0
    while t < 18.0 - 1e-9:  # beats 1 and 3 (every 1.0 s)
        kicks.append((t, 0.9 if t in (6.0, 12.0) else 0.6))
        t += 1.0
    t = 18.0
    while t < 23.0 - 1e-9:  # fullest bar: every beat at low level, beat 1 a bit more
        beat1 = abs((t / 2.0) - round(t / 2.0)) < 1e-9
        kicks.append((t, 1.0 if t == 18.0 else (0.55 if beat1 else 0.4)))
        t += 0.5
    t = 24.0
    while t < 28.0 - 1e-9:
        kicks.append((t, 1.0 if t == 24.0 else 0.6))
        t += 1.0
    for t, v in kicks:
        kick_hit(t, v)

    # ---- hats from filtered noise, off-beat
    k = 0
    for t in np.arange(8.75, 12.0, 1.0):  # 8.0 section: every other off-beat
        hat_hit(float(t), 0.6, pan=0.2 * (-1) ** k)
        k += 1
    for t in np.arange(12.25, 18.0, 0.5):
        hat_hit(float(t), 0.8, pan=0.2 * (-1) ** k)
        k += 1
    for t in np.arange(18.25, 23.0, 0.5):
        hat_hit(float(t), 1.0, pan=0.2 * (-1) ** k)
        k += 1
    for t in np.arange(24.25, 28.0, 0.5):
        hat_hit(float(t), 0.7, pan=0.2 * (-1) ** k)
        k += 1

    # ---- bass
    for i, (s, name, r) in enumerate(bars6):
        bass_note(s, r, 0.9, 1.15 if i == 0 else 1.0)
        bass_note(s + 1.0, r, 0.9, 0.9)
    walk = [(0.0, 0, 0.45), (0.5, 12, 0.2), (1.0, 7, 0.45), (1.5, 12, 0.2), (1.75, 7, 0.2)]
    for s, name, r in bars12:
        for off, semi, d in walk:
            bass_note(s + off, r + semi, d, 1.1 if (s == 12 and off == 0) else 0.95)
    for s, name, r in bars18:
        for off, semi, d in walk:
            if s + off >= 23.0:
                break
            bass_note(s + off, r + semi, d, 1.1 if (s == 18 and off == 0) else 1.0)
    bass_note(23.0, 46, 0.97, 0.7)  # holds Bb under the filter sweep, ends before 24.0
    for t in (24.0, 25.0, 26.0, 27.0):
        bass_note(t, 38, 0.9, 1.15 if t == 24.0 else 0.95)

    # ---- lead
    lead_note(13.0, 74, 1.0)  # D
    lead_note(14.0, 77, 1.0)  # F
    lead_note(15.0, 81, 1.0)  # A
    lead_note(16.0, 79, 1.5)  # G
    lead_note(18.0, 77, 1.0)
    lead_note(19.0, 81, 1.0)
    lead_note(20.0, 79, 1.5)
    lead_note(21.5, 81, 0.5)
    lead_note(22.0, 77, 0.8, rel=0.3)
    lead_note(26.0, 81, 1.9, att=0.5, rel=0.1)  # sustained A

    # ---- bells
    bell(18.0, 81, 0.8)
    bell(20.0, 86, 0.8)
    bell(22.0, 89, 0.7)
    bell(24.0, 86, 1.0, tau=1.8, dur=4.5)
    bell(28.0, 74, 1.1, tau=3.0, dur=2.0)  # D5, rings out to 30.0

    # ---- FX: 17.6-18.0 riser, 23-24 sweep
    n = T(0.4)
    x = np.arange(n) / n
    nz = rng.standard_normal(n)
    fc = 400.0 * (9000.0 / 400.0) ** x
    r = tv_lp(tv_lp(nz, fc), fc)
    r *= (x ** 2) * smooth(np.arange(n) / T(0.005)) * smooth((n - np.arange(n)) / T(0.003))
    put("fx", 17.6, r * 3.0)

    n = T(1.0)
    x = np.arange(n) / n
    nz = rng.standard_normal(n)
    fc = 250.0 * (9000.0 / 250.0) ** (x ** 1.4)
    sw = tv_lp(tv_lp(nz, fc), fc)
    sw *= smooth(x) * smooth((n - np.arange(n)) / T(0.03))
    put("fx", 23.0, sw * 3.0, pan=0.0)

    return kicks


# ---------------------------------------------------------------- reverb / delay
def make_ir(seed):
    g = np.random.default_rng(seed)
    n = T(2.4)
    t = np.arange(n) / SR
    pre = T(0.015)
    noise = g.standard_normal(n) * np.exp(-6.91 * t / 2.2)
    ir = np.concatenate([np.zeros(pre), noise[: n - pre]])
    a = lp_fft(ir, 6000, 1)
    b = lp_fft(ir, 1800, 2)
    w = smooth(t / 1.6)
    ir = a * (1 - w) + b * w
    ir = hp_fft(ir, 180, 1)
    ir *= smooth((t - pre / SR) / 0.008)
    return ir / np.sqrt(np.sum(ir ** 2))


def reverb(send):
    nf = 1 << 21
    out = np.zeros_like(send)
    for c, seed in enumerate((11, 23)):
        irf = np.fft.rfft(make_ir(seed), nf)
        out[c] = np.fft.irfft(np.fft.rfft(send[c], nf) * irf, nf)[:N]
    return out


def pingpong(mono, delay=T(0.375), taps=6, fb=0.45, fc=4500):
    """Dotted-eighth stereo ping-pong delay (echoes only, each repeat darker)."""
    out = np.zeros((2, N))
    cur = mono.copy()
    for k in range(1, taps + 1):
        cur = lp_fft(cur, fc)
        sh = k * delay
        out[1 if k % 2 else 0, sh:] += cur[: N - sh] * fb ** k
    return out


def nrm(x):
    return x / (np.max(np.abs(x)) + 1e-12)


# ---------------------------------------------------------------- main
def main():
    kicks = build()

    # sidechain-ish duck for bass (and a little pad) on each kick
    duck = np.ones(N)
    ramp = T(0.003)
    for t, v in kicks:
        i = T(t)
        ln = T(0.5)
        j = np.arange(min(ln, N - i))
        c = 1.0 - 0.35 * v * np.exp(-j / (0.12 * SR)) * smooth(j / ramp)
        duck[i:i + len(j)] *= c

    pad = lp_fft(B["pad"], 2600, 2)
    pad = hp_fft(pad, 60, 1)
    # filter sweep on the pad, 22.5-24.0: darken, then open up to 24.0
    s0, s1 = T(22.5), T(24.0)
    ln = s1 - s0
    u = np.arange(ln) / SR
    fc = np.where(
        u < 0.5,
        6000.0 * (300.0 / 6000.0) ** smooth(u / 0.5),
        300.0 * (9000.0 / 300.0) ** (np.clip((u - 0.5) / 1.0, 0, 1) ** 1.3),
    )
    seg = pad[:, s0:s1]
    w = smooth(np.arange(ln) / T(0.1)) * (1.0 - smooth((np.arange(ln) - (ln - T(0.05))) / T(0.05)))
    for c in range(2):
        y = tv_lp(tv_lp(seg[c], fc), fc)
        pad[c, s0:s1] = seg[c] * (1 - w) + y * w
    pad = pad * duck ** 0.3

    bass = hp_fft(lp_fft(B["bass"], 350, 2), 35, 1) * duck
    kick = B["kick"]
    hats = lp_fft(hp_fft(B["hat"], 6500, 2), 12000, 1)
    pl = lp_fft(B["pluck"], 6000, 1)
    ld = lp_fft(B["lead"], 4200, 2)
    bl = B["bell"]
    fx = hp_fft(B["fx"], 150, 1)
    echo = pingpong(lp_fft(PLUCK_M, 6000, 1)) * 0.55 + pingpong(lp_fft(LEAD_M, 4200, 2), fb=0.35) * 0.30

    pad, bass, kick, hats, pl, ld, bl, fx = (nrm(x) for x in (pad, bass, kick, hats, pl, ld, bl, fx))
    echo = nrm(echo)

    g = dict(pad=0.34, bass=0.50, kick=0.45, hats=0.075, pl=0.26, ld=0.19, bl=0.17, fx=0.16, echo=0.14)
    dry = (pad * g["pad"] + bass * g["bass"] + kick * g["kick"] + hats * g["hats"]
           + pl * g["pl"] + ld * g["ld"] + bl * g["bl"] + fx * g["fx"] + echo * g["echo"])
    send = (pad * g["pad"] * 0.55 + pl * g["pl"] * 0.6 + ld * g["ld"] * 0.7 + bl * g["bl"] * 1.0
            + hats * g["hats"] * 0.3 + fx * g["fx"] * 0.6 + echo * g["echo"] * 0.6)
    wet = reverb(send)
    mix = dry + wet * 0.55

    mix = hp_fft(mix, 30, 1)
    mix = mix / np.max(np.abs(mix))
    mix = np.tanh(1.4 * mix)  # gentle soft limiting of transients
    m = T(0.3)  # master fade: last 0.3 s only
    mix[:, N - m:] *= (0.5 * (1.0 + np.cos(np.pi * np.arange(m) / (m - 1))))
    mix *= 10 ** (-3.0 / 20.0) / np.max(np.abs(mix))

    pcm = np.round(mix * 32767.0).astype("<i2")
    inter = np.empty(N * 2, dtype="<i2")
    inter[0::2] = pcm[0]
    inter[1::2] = pcm[1]
    with wave.open(OUT, "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(SR)
        wf.writeframes(inter.tobytes())
    print("wrote", OUT)


if __name__ == "__main__":
    main()
