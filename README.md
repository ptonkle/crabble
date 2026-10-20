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

## Models, the Gallery and stills

Every crab, tile, board, patch of seafloor, prop and the title card is a hand-written JSON file in `data/models/` (the format is at the top of `crates/core/src/model.rs`: millimetres, +y up, +z the model's front, +x its left; parts with pivots, flat faces, colours with a player slot, glow or alpha, anchors and labels, keyframed moves). Letters and digits are flat polygons in `data/glyphs.json`. Scenes in `data/scenes/` place models and a camera for the design journal (`crates/core/src/scene.rs`). The Gallery, on the Title, shows each model and scene.

`scripts/shot.py` takes stills of the built page in headless Chromium (no window opens), 1920 × 1080 by default, with the clock stopped so a still comes out the same every time:

```bash
./scripts/build.sh
.venv/bin/python scripts/shot.py tile.png --model tile --letter Q --view above
.venv/bin/python scripts/shot.py board.png --scene placing
.venv/bin/python scripts/shot.py walk.png --model crab --anim walk_side --colour blue --ms 250
.venv/bin/python scripts/shot.py jump.png --model crab --anim jump --grid ms=0,150,300,450
.venv/bin/python scripts/shot.py views.png --model tile --grid view=model,front,side,above
.venv/bin/python scripts/shot.py moves.png --model crab --grid anim=walk_side,walk_fwd --grid ms=0,200,400
.venv/bin/python tools/glyphs/sheet.py glyphs.png     # every glyph, flat, to check by eye
```

The crab lines need a crab model (`data/models/crab.json`). The same choices open the page in a browser: `index.html?model=tile&letter=Q&view=above&ms=0`. The keys are listed at the top of `crates/core/src/gallery/mod.rs`; `--query` passes any of them raw, `--controls` keeps the Gallery's panel on screen.

## Build, test and serve

```bash
./scripts/setup.sh              # once: Python venv with Playwright and three browsers
cargo test --workspace          # the core, boundary, copy and storage tests
./scripts/build.sh              # release WebAssembly build into dist/
python3 scripts/serve.py        # http://127.0.0.1:8000/
./scripts/check.sh              # everything
```

Needs Rust (pinned in `rust-toolchain.toml`), `wasm-bindgen-cli` 0.2.127, and Python 3.
