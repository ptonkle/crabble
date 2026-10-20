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

- **R1.7:** which way the crab faces and how WASD moves it (the default: claws toward the camera's view, A/D scuttle sideways).
- **R1.23:** whether Settings are kept between visits in browser storage, as in Honey Hole (now they last for the visit).

## Mistakes so far

By system, from `SECOND-ORDER.md`, with the rule each one produced.

## What passed

The last lines of `./scripts/check.sh` and `./scripts/count-tests.sh`, with the date and commit.

2026-10-19, the tooling and menu port (branch of this worktree), three engines:

- `./scripts/check.sh`: stages 1 to 9 pass (format, lint, core and boundary tests, tag guard, release build, import audit, art, notebook, browser check: 159 checks in Chromium, Firefox and WebKit). Stage 10, the journal check, fails on the journal's own entries (SECOND-ORDER R1.45), which this port does not write.
- `./scripts/check-online.sh`: passes in Chromium, Firefox and WebKit (transport 64 checks, a game between two tabs 120 checks).
- Times in `notes/check-times.md`.

## The next action

One action.
