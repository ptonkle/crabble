#!/usr/bin/env python3
"""Makes the AI's vocabulary: the ENABLE words that are common (TRIAGE F11).

    python3 tools/words/common.py SCOWL_FINAL_DIR    writes data/words/common.txt
    python3 tools/words/pack.py                      then packs it

The person, F11: "cull exteremely uncommon words out of the list, i saw the
ai using these a lot and it's just not fun", then "I guess let's just plan to
cut them from the AI". So ENABLE (data/words/enable1.txt) still judges every
word, and the AI only makes words in this smaller list (SECOND-ORDER R12.2).

The commonness ranking is SCOWL's size levels (Kevin Atkinson's Spell
Checker Oriented Word Lists, wordlist.aspell.net; licence in LICENSES.md).
SCOWL_FINAL_DIR is the `final/` folder of the release tarball, e.g.
scowl-2020.12.07.tar.gz from downloads.sourceforge.net/wordlist/. A word's
level is the lowest level of the FILES (`english-words.*`, `american-words.*`,
`british-words.*`, `canadian-words.*`; British and Canadian give AXE and the
like, Australian is left out for its slang) that lists it in lowercase (so
names and abbreviations such as AA and XU, which SCOWL keeps in other
files, never count; a web word count would rank them high).

A word is kept when it is in ENABLE and its level is at most LEVEL; a word
of SHORT letters or fewer needs SHORT_LEVEL, as short words are where the
odd Scrabble words live (AA, JO, XU, QAT). The cutoffs were chosen by
sampling what each level keeps and cuts (SECOND-ORDER R12.3).
"""

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ENABLE = ROOT / "data/words/enable1.txt"
OUT = ROOT / "data/words/common.txt"
FILES = ("english-words", "american-words", "british-words", "canadian-words")
LEVEL = 40
SHORT = 3
SHORT_LEVEL = 35


def levels(final):
    out = {}
    for f in sorted(Path(final).iterdir()):
        name, _, level = f.name.rpartition(".")
        if name not in FILES or not level.isdigit():
            continue
        for w in f.read_text(encoding="latin-1").split():
            if w.isascii() and w.isalpha() and w.islower():
                out[w] = min(out.get(w, 99), int(level))
    if not out:
        sys.exit(f"no {' or '.join(FILES)} files in {final}")
    return out


def common(enable, lv):
    def keep(w):
        need = SHORT_LEVEL if len(w) <= SHORT else LEVEL
        return lv.get(w, 99) <= need

    return [w for w in enable if keep(w)]


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    enable = ENABLE.read_text(encoding="ascii").split()
    words = common(enable, levels(sys.argv[1]))
    OUT.write_text("".join(w + "\n" for w in words), encoding="ascii")
    print(f"{len(words)} of {len(enable)} ENABLE words -> {OUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
