# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `web/art/` | drawn by `tools/art/art.py` from TikZ in this repository | this repository's licence |
| `data/words/enable1.txt` | ENABLE (Enhanced North American Benchmark Lexicon), 172,820 words, from https://norvig.com/ngrams/enable1.txt, unchanged; sha256 `61ba1392a5b6199dd161fae7b483cd262fb8a88c0d49bd2c9ecb6753e755697f` | public domain |
| `data/words/enable1.bin` | packed from `enable1.txt` by `tools/words/pack.py` | public domain |
| `data/words/common.txt`, `data/words/common.bin` | the AI's vocabulary: the ENABLE words that SCOWL (Spell Checker Oriented Word Lists, Kevin Atkinson, http://wordlist.aspell.net/) 2020.12.07 lists at size 40 or below (35 for words of 3 letters or fewer), chosen by `tools/words/common.py` from the `english-words`, `american-words`, `british-words` and `canadian-words` files of `scowl-2020.12.07.tar.gz` (https://downloads.sourceforge.net/wordlist/scowl-2020.12.07.tar.gz, sha256 `5587667caa20c4891390c2d42dbb4d5c4c3f41bee77af1457ece3ba23fb859cc`); the tarball is not kept here; packed by `tools/words/pack.py` | the words are ENABLE's (public domain); the selection is SCOWL's: "Copyright 2000-2018 by Kevin Atkinson. Permission to use, copy, modify, distribute and sell these word lists, the associated scripts, the output created from the scripts, and its documentation for any purpose is hereby granted without fee, provided that the above copyright notice appears in all copies and that both that copyright notice and this permission notice appear in supporting documentation. Kevin Atkinson makes no representations about the suitability of this array for any purpose. It is provided \"as is\" without express or implied warranty." SCOWL's levels draw on Moby Words II and Brian Kelk's UK English Wordlist, both public domain. Norvig's `count_1w.txt` was not used: norvig.com puts only its code under MIT, and the counts come from the Google Web Trillion Word Corpus, distributed by the LDC under its own licence (SECOND-ORDER R12.2). |

## Vendored code

| Path | Project and version | Author | Licence | Checksum |
| --- | --- | --- | --- | --- |
| `web/vendor/trystero-nostr-0.25.4.js` | [Trystero](https://github.com/dmotz/trystero) 0.25.4, Nostr strategy, bundled with its dependencies | Dan Motzenbecker | MIT | `web/vendor/SHA256SUMS` |

## Code taken from other repositories

| From | What | Licence |
| --- | --- | --- |
| Ape Race and Honey Hole (the person's own projects) | the online transport (`web/online.js`, `web/net.js`, `web/net-clock.js`, `web/online-config.js`, `web/nettest.*`), the online protocol's design, the page's menu builder, the check tooling (`scripts/tags.py`, `checkrun.py`, `check-dashboard.py`, `watch-check.sh`, `timed-checks.sh`, `quick-check.sh`, `bump.sh`, `tests/browser/*`) | the person's |
| Honey Hole (the person's own project) | `crates/core/src/sound/decimal.rs` and `read.rs` unchanged; the recipe format and its checks in `recipe.rs` | the person's own code |
| Ape Race (the person's own project) | the WAV writer in `crates/core/src/sound/wav.rs`; the synthesiser's approach in `synth.rs` | the person's own code |

A file with no licence, a likeness, a recording or any third-party picture is a worklist row in `SECOND-ORDER.md` for the person to decide, and stays out of git until they do.
