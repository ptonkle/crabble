#!/usr/bin/env python3
"""Packs the word list for the core (crates/core/src/words.rs).

    python3 tools/words/pack.py            writes data/words/enable1.bin
    python3 tools/words/pack.py --check    fails if the .bin is stale

Input: data/words/enable1.txt, ENABLE as published (public domain; see
LICENSES.md), one lowercase word a line, sorted, 172,820 words.

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
"""

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TXT = ROOT / "data/words/enable1.txt"
BIN = ROOT / "data/words/enable1.bin"
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


def main():
    words = TXT.read_text(encoding="ascii").split()
    out = pack(words)
    if "--check" in sys.argv:
        if not BIN.exists() or BIN.read_bytes() != out:
            sys.exit(f"{BIN.relative_to(ROOT)} is stale: run python3 tools/words/pack.py")
        print(f"{BIN.relative_to(ROOT)} matches {TXT.relative_to(ROOT)}")
        return
    BIN.write_bytes(out)
    print(f"{len(words)} words, {len(out)} bytes -> {BIN.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
