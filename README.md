# Crabble

A speed word game played by crabs on the seafloor, in 3D in the browser. Steer your crab round the board, pull letter tiles out of messy piles with your claws, and place them as fast as you can: there are no turns. The last crab to add a letter to a word owns it. After 4 minutes the words are tallied, and a word that is not real costs its owner its points. Play against 1 to 3 AI crabs, or online with up to four players. Mouse and keyboard, desktop only.

The design is in [`BRIEF.md`](BRIEF.md), decisions in [`DECISIONS.md`](DECISIONS.md), and what the agent decided alone in [`SECOND-ORDER.md`](SECOND-ORDER.md).

## Controls

| Action | Key |
| --- | --- |
| Move | W A S D |
| Look | Mouse |
| Left claw: grab, place or drop | Left click |
| Right claw: grab, place or drop | Right click |
| Jump | Space |
| Big overhead view | Hold Tab |

## Build, test and serve

```bash
./scripts/setup.sh              # once: Python venv with Playwright and three browsers
cargo test --workspace          # the core, boundary, copy and storage tests
./scripts/build.sh              # release WebAssembly build into dist/
python3 scripts/serve.py        # http://127.0.0.1:8000/
./scripts/check.sh              # everything
```

Needs Rust (pinned in `rust-toolchain.toml`), `wasm-bindgen-cli` 0.2.127, and Python 3.
