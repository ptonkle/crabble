#!/usr/bin/env python3
"""Packs the word lists for the core (crates/core/src/words.rs).

    python3 tools/words/pack.py            writes data/words/enable1.bin and common.bin
    python3 tools/words/pack.py --check    fails if a .bin is stale

Inputs, one lowercase word a line, sorted: data/words/enable1.txt, ENABLE as
published (public domain; see LICENSES.md), 172,820 words, which judges
every word; and data/words/common.txt, the AI's vocabulary, a subset of it
made by tools/words/common.py.

The packed form is front coded in blocks, so the core can binary search it
where it lies in the WebAssembly module, with no parsing at load:

    "ENB1"                      magic
    u32 LE  words
    u32 LE  words a block (the last block may hold fewer)
    u32 LE  blocks
    u32 LE  x blocks: each block's start, from the start of the body
    body:   per word, one byte (the letters it shares with the word before,
            0 at each block's start) then its other letters, a to z

A letter is a byte of 97 or more, a shared count is below 32, so a word
ends at the next byte below 97 or at the end of the body.

The AI's vocabulary is one bit per ENABLE word, in ENABLE's order, so it can
only hold ENABLE words and the core finds a word's bit where it finds the
word (about 21 KB, where the words front coded would take 148 KB):

    "ENC1"                      magic
    u32 LE  words in ENABLE (the bits)
    u32 LE  words kept (the bits set)
    bits:   word k is bit k % 8 of byte k / 8
"""

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORDS = ROOT / "data/words"
BLOCK = 64


def pack(words):
    assert words == sorted(set(words)), "the list must be sorted with no repeats"
    for w in words:
        assert w.isascii() and w.isalpha() and w.islower() and len(w) < 32, w
    body = bytearray()
    starts = []
    prev = ""
    for k, w in enumerate(words):
        if k % BLOCK == 0:
            starts.append(len(body))
            prev = ""
        p = 0
        while p < min(len(w), len(prev)) and w[p] == prev[p]:
            p += 1
        body.append(p)
        body += w[p:].encode("ascii")
        prev = w
    head = b"ENB1" + struct.pack("<III", len(words), BLOCK, len(starts))
    head += b"".join(struct.pack("<I", s) for s in starts)
    return head + bytes(body)


def pack_common(enable, common):
    assert common == sorted(set(common)), "the vocabulary must be sorted with no repeats"
    keep = set(common)
    assert keep <= set(enable), f"not in ENABLE: {sorted(keep - set(enable))[:10]}"
    bits = bytearray((len(enable) + 7) // 8)
    for k, w in enumerate(enable):
        if w in keep:
            bits[k // 8] |= 1 << (k % 8)
    return b"ENC1" + struct.pack("<II", len(enable), len(common)) + bytes(bits)


def main():
    enable = (WORDS / "enable1.txt").read_text(encoding="ascii").split()
    common = (WORDS / "common.txt").read_text(encoding="ascii").split()
    outs = [
        ("enable1.bin", "enable1.txt", pack(enable), len(enable)),
        ("common.bin", "common.txt", pack_common(enable, common), len(common)),
    ]
    stale = False
    for name, src, out, n in outs:
        path = WORDS / name
        if "--check" in sys.argv:
            if not path.exists() or path.read_bytes() != out:
                print(f"data/words/{name} is stale: run python3 tools/words/pack.py")
                stale = True
            else:
                print(f"data/words/{name} matches data/words/{src}")
            continue
        path.write_bytes(out)
        print(f"{n} words, {len(out)} bytes -> data/words/{name}")
    if stale:
        sys.exit(1)


if __name__ == "__main__":
    main()
