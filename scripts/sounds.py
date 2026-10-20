#!/usr/bin/env python3
"""Writes every sound to a directory to listen to, and checks each file.

    python3 scripts/sounds.py OUT_DIR [VOLUME]

The core renders the sounds (crates/core/examples/write_sounds.rs runs
`tool_core::sound`); this script computes nothing about them beyond reading
the files back. For each file it prints the length, the peak (dB below full
scale), how many samples sit at full scale (clipping), the DC offset (the
mean, as a percent of full scale), the first and last samples (a sound
should start and end at silence), and for a loop the jump across its seam.
It exits non-zero when a file clips, starts or ends away from silence, or
carries a DC offset over 1%.

When ffmpeg can encode audio, each file is also converted to a small .ogg
(or .mp3) beside it. Playwright's bundled ffmpeg has no audio encoder, so
without a system ffmpeg the .wav files are kept alone.
"""
import array
import math
import pathlib
import shutil
import subprocess
import sys
import wave

ROOT = pathlib.Path(__file__).resolve().parents[1]
FULL = 32767


def encoder():
    """An ffmpeg that can write .ogg or .mp3, and the extension and codec."""
    candidates = [shutil.which("ffmpeg")]
    candidates += sorted(str(p) for p in pathlib.Path.home().glob(".cache/ms-playwright/ffmpeg-*/ffmpeg-linux"))
    for exe in filter(None, candidates):
        try:
            listing = subprocess.run([exe, "-hide_banner", "-encoders"], capture_output=True, text=True).stdout
        except OSError:
            continue
        for codec, ext in (("libvorbis", "ogg"), ("libopus", "ogg"), ("libmp3lame", "mp3")):
            if codec in listing:
                return exe, ext, codec
    return None


def read(path):
    with wave.open(str(path)) as w:
        rate = w.getframerate()
        pcm = array.array("h", w.readframes(w.getnframes()))
    return rate, pcm


def db(peak):
    return -math.inf if peak == 0 else 20 * math.log10(peak / FULL)


def main(out, volume):
    out = pathlib.Path(out)
    run = subprocess.run(
        ["cargo", "run", "-q", "-p", "tool_core", "--example", "write_sounds", "--", str(out), str(volume)],
        cwd=ROOT, capture_output=True, text=True,
    )
    if run.returncode != 0:
        sys.exit(run.stderr)
    files = [pathlib.Path(p) for p in run.stdout.split()]
    enc = encoder()
    loops = {"ambience"}
    bad = []
    print(f"{'file':<16}{'ms':>6}{'peak dB':>9}{'clip':>6}{'dc %':>7}{'first':>7}{'last':>7}{'seam':>7}")
    for f in files:
        rate, pcm = read(f)
        n = len(pcm)
        peak = max((abs(s) for s in pcm), default=0)
        clip = sum(1 for s in pcm if abs(s) >= FULL)
        dc = 100 * (sum(pcm) / n) / FULL if n else 0
        name = f.stem
        is_loop = name.split("-")[0] in loops
        seam = abs(pcm[-1] - pcm[0]) if n else 0
        print(f"{name:<16}{1000 * n // rate:>6}{db(peak):>9.1f}{clip:>6}{dc:>7.2f}{pcm[0]:>7}{pcm[-1]:>7}"
              f"{seam if is_loop else '':>7}")
        quiet = 0.002 * FULL
        if clip or abs(dc) > 1 or (not is_loop and (abs(pcm[0]) > quiet or abs(pcm[-1]) > quiet)):
            bad.append(name)
        if enc:
            exe, ext, codec = enc
            subprocess.run([exe, "-y", "-loglevel", "error", "-i", str(f), "-c:a", codec, "-q:a", "4",
                            str(f.with_suffix("." + ext))], check=True)
    print(f"\n{len(files)} files in {out}" + (f", each also as .{enc[1]}" if enc else ", wav only (no audio encoder found)"))
    if bad:
        sys.exit("these files clip, carry a DC offset or do not start and end at silence: " + ", ".join(bad))


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 100)
