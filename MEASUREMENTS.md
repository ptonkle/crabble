# Measurements

Stage 4, recorded by the agent before building and again at handoff. `B2 Require step-by-step justification`.

The state measured rather than recalled, so later changes can be compared with it. A probe prints numbers and asserts loose bounds at most; record its output here with the command that produced it.

| What | Command | Result | When |
| --- | --- | --- | --- |
| Toolchain | `rustc --version; wasm-bindgen --version` | | |
| Core tests | `./scripts/count-tests.sh` | | |
| Full check | `./scripts/check.sh` | | |
| WebAssembly size | printed by `./scripts/build.sh` | | |
| Breaks caught | `python3 scripts/break_tests.py` | | |
| Frame time on a phone-sized view (F15, R13.12), before the Quality setting | `scripts/touch-frames.py` (then without the setting): 844 x 390 with touch, Chromium on this machine's GPU (d3d12), four crabs, 240 frames | pixel ratio 3 (2532 x 1170): mean 19.7 ms, 95th 33.4; with the CPU slowed 4x mean 38.4, 95th 83.4. Ratio 2: 16.6 / 18.2 slowed. Ratio 1: 16.8 / 25.0 slowed. 1920 x 1080 at 1: 17.7 | 2026-10-20 |
| Frame time with the Quality setting | `.venv/bin/python scripts/touch-frames.py` | 844 x 390 at ratio 3: High (2532 x 1170) mean 16.7 ms, slowed 4x 21.6 (95th 33.4); Auto (1688 x 780) 17.0, slowed 22.2 (95th 49.9); Low (844 x 390) 16.6, slowed 28.7 (95th 66.7). The runs swing by several ms from one to the next: the GPU here hides the fill cost, and the slowed CPU (the core's frame build) dominates whatever the ratio | 2026-10-20 |

## Not measured

Devices, browsers and conditions no measurement here covers.

- A real phone or tablet: its GPU's fill rate, thermal slowdown, and the browser's bars (F15, R13.10, R13.12).
