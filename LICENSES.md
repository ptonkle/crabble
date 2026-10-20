# Licences

Every file in this repository that is not code written for it: its source, its author, its terms. The code is under the repository's licence.

## Assets

| Path | Source | Terms |
| --- | --- | --- |
| `web/art/` | drawn by `tools/art/art.py` from TikZ in this repository | this repository's licence |

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
