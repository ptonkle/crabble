# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file, not from memory.

## What this is

Crabble: speed Scrabble played by crabs on the seafloor, in 3D in the browser, for 2 to 4 players (local against AI crabs, or online). It lives in `slop/crabble/`; its design journal is `../crabble-journal/`. This round sets up the repository, the tooling and menus, the model format and the early design materials.

## Rules that hold it up

The rules that break silently if a later change ignores them, and the test that guards each.

## Shape of the code

Where each kind of thing lives, in a short table.

## Commands

Build, test, check, publish, and the probes, with what each prints when it passes.

## What will bite within the hour

What a new session gets wrong first.

## The journal

Where the design journal is, its link if it is published, and its newest entry.

## Open for the person

- **R7.9:** try colourblind mode (Settings): its seat colours, symbols, thicker outlines and stronger premium squares.
- **R19.6:** switch colourblind mode on mid-game (pause, Settings) and say whether it now reads clearly, and whether the seat colours should move further from the usual ones.

- **R4.11:** decide the tile count (and maybe the piles' distance or the idle grace): four medium AI crabs use about 39 of the 294 tiles in 4:00 (`notes/playtests.md`).
- **R13.10:** play a game on a real phone and a tablet (F15): walk and look at once, both claws, the overhead tap, the pause, held upright and on its side; the stick's dead zone and the drag's turn (R13.5, R13.6) are guesses until then.
- **R12.3:** read the AI's vocabulary samples (what the cutoff keeps and cuts, every two-letter word it keeps) and say whether the cutoff is right; and the cost in R12.5 (the AI placed fewer tiles and dropped more; mostly won back in R14.*, see R14.2).
- **R20.6:** the AI no longer makes any word it does not know (CR-Q15), and pays for it in idle penalty: 8.8 to 27.4 points a crab a game, final score 40.0 to 25.3. Fine, or should the AI look at the board more often, or the tiles in view, the piles or the grace change (`notes/playtests.md`)?
- **R1.130:** approve the world's look: the board's premium palette and labels, the mounds, props and tile piles, the tile's grain.

## Mistakes so far

By system, from `SECOND-ORDER.md`, with the rule each one produced.

## What passed

The last lines of `./scripts/check.sh` and `./scripts/count-tests.sh`, with the date and commit.

2026-10-19, the tooling and menu port (branch of this worktree), three engines:

- `./scripts/check.sh`: stages 1 to 9 pass (format, lint, core and boundary tests, tag guard, release build, import audit, art, notebook, browser check: 159 checks in Chromium, Firefox and WebKit). Stage 10, the journal check, fails on the journal's own entries (SECOND-ORDER R1.45), which this port does not write.
- `./scripts/check-online.sh`: passes in Chromium, Firefox and WebKit (transport 64 checks, a game between two tabs 120 checks).
- `./scripts/count-tests.sh`: 98 passed, 0 failed, 0 ignored.
- Times in `notes/check-times.md` (the full check's row reads FAIL for stage 10 alone; its stages are in the log above it).

After merging main (the model format, renderer, glyphs, scenes, Gallery and sounds), 2026-10-19:

- `CI=1 ./scripts/check.sh`: stages 1 to 9 pass (202 cargo tests; the browser check 195 checks in Chromium, Firefox and WebKit, the Gallery and the sounds steps among them); stage 10 waits on the journal (R1.45).
- `./scripts/check-online.sh`: passes in three engines (64 and 120 checks).
- `scripts/break_tests.py`: 53 of 53 breaks caught.

2026-10-20, F15 (phone and tablet), merged with main at 069ad8c:

- `./scripts/quick-check.sh main`: passes (Chromium). The `touch`, `settings` and `how` browser steps pass in Chromium, Firefox and WebKit; WebKit and Firefox each pass every step alone. In one three-engine run Firefox's `sounds` step times out after Chromium, on main too (SECOND-ORDER R13.13).
- `scripts/break_tests.py` on F15's 13 breaks: 13 caught.

## The next action

One action.
