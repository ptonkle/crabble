# Plan

Stage 2, written by the agent from `BRIEF.md`. `B2 Require step-by-step justification`.

Each decision in build order, with the reason for it and the test that will show it holds. The person reads this at stage 3 (`C3 Interrogate the claim`) and accepts, revises, or answers the open questions.

## Gameplay

Approved in advance (2026-10-19: "Yes go ahead with gameplay build"), with the person's numbers: "8 seconds to cross the board, jump just high enough to jump on top of tiles on the board." The online sync and the AI crabs are later sessions; this build leaves them a seam and builds neither. In code the simulation is `sim::Match` (the person's name for it); on screen it is a *game* (GLOSSARY.md).

### Architecture contracts

| # | Decision | Reason | Test that shows it holds |
| --- | --- | --- | --- |
| 1 | **Units.** The simulation works in the renderer's units: Q16.16 metres (`gfx::fx`), angles in 65536 a turn, y up, x east, z toward the board's near side. Tuning is written in mm, mm/s and ms (`data/tuning.json`) and converted once on load. | No floats in the core (D2); one unit system for physics, aim rays and drawing, so nothing is converted per frame. | `tuning_file_is_complete_and_valid`; boundary scan |
| 2 | **Fixed tick of 20 ms.** `App::advance` turns the page's clock into whole ticks; leftover time waits for the next frame. | Floaty jumps and tumbling tiles need a steady step; whole ticks keep the simulation the same on every machine. | `the_clock_runs_whole_ticks_and_keeps_the_rest` |
| 3 | **One `Match`** (`crates/core/src/sim/`) holds the whole game: the ground, the board, every tile, every crab, the clock, the idle timers, the events. `Match::step(&mut self, inputs: &[SeatInput])` moves it one tick. The same seed and the same inputs give the same game. | Tests, AI play-throughs, replays of bugs and the online host all run the same thing. | `two_games_with_the_same_inputs_stay_identical` |
| 4 | **`SeatInput`, one per seat:** the movement keys held (W A S D), Space pressed, the look (absolute yaw and pitch) and the two claws' clicks this tick. People and AI crabs both make `SeatInput`; the AI gets no other door. Remote crabs (online) take no input and are placed from their owner's states. | One path for every crab, so an AI plays by a person's rules (BRIEF.md, AI crabs). | the sim tests drive `SeatInput` only |
| 5 | **Every tile has an id** (`TileId`, 0 to 293, fixed for the game) and one state: *loose* (resting or tumbling, with its pose), *held* (seat and claw) or *placed* (square, by whom, and its place order). | The online host and its joiners name tiles by id; ownership needs the placing order. | `tile_ids_stay_fixed_and_every_tile_has_one_state` |
| 6 | **Grab and place are claims a referee settles; first claim wins.** A click becomes a `Claim` (`Grab { tile, seat, claw }`, `Place { tile, seat, square }`). `Match` settles claims in the order they arrive (within a tick, in seat order): a claim on a tile no longer loose, or a square already filled, or no longer touching, is refused. In a local game the `Match` is its own referee and settles at once. Online, the host's `Match` is the referee: a joiner's `Match` runs with `Referee::Remote`, queues its claims (`take_claims`) for the app to send, and applies the host's verdicts (`apply_verdict`); the host feeds joiners' claims in with `submit_claim`. Not built here: the messages. | The brief: "who picked up a given tile and who placed a given tile first" must agree everywhere (DECISIONS.md, online sync). | `two_claims_on_one_tile_the_first_wins`; `two_placements_on_one_square_the_first_wins`; `a_remote_referee_queues_claims_and_applies_verdicts` |
| 7 | **Events.** Each tick appends `Event`s: Grabbed, Placed, Dropped, Refused, Jumped, Landed, IdleWarning, IdlePenalty, Ended. Sounds, popups and (later) the AI and online messages read events; nothing else watches the state for changes. A drop is an event, not a claim: a held tile is always yours to drop. | One place where things happen. | event tests per system |
| 8 | **The ground** is a height grid built at the start from the drawn seafloor, mounds and board (their up-facing faces, `sim/ground.rs`), so a crab stands where the sand is drawn. Tiles' tops are added to it where a crab stands. A crab cannot walk up a step higher than `crab.step_mm`; it jumps onto it. | The map is data (`data/scenes/seafloor.json`); a second hand-written ground would drift from it. | `a_crab_stands_on_the_drawn_ground`; `a_lying_tile_needs_a_jump` |
| 9 | **Tiles are light rigid boxes** (200 × 200 × 45 mm). At rest they sleep. A tile that loses what held it up (the tile under it grabbed) wakes, falls, and settles on the plane fitted to what is under it; steeper than `tiles.repose_deg` it slides downhill and settles again; then it sleeps. All integer, so deterministic. Only the referee runs tile physics; online, joiners will be sent where tiles come to rest. | "Pull a piece out from the bottom and the top ones might tumble down"; deterministic for the online host. | `pulling_the_bottom_tile_tumbles_the_one_above`; `a_settled_tile_sleeps_and_never_sinks` |
| 10 | **Aim.** The camera's centre ray (from the seat's yaw and pitch, the crab's place and `camera.*` in tuning) picks the loose tile it hits first, or the board square it hits. Reach is `crab.reach_mm` past the claw tips. | The crosshair is the rule (BRIEF.md, View); the AI aims the same way. | `the_crosshair_grabs_the_tile_it_hits_within_reach` |
| 11 | **Words and ownership** (`sim/words.rs`): a word is a maximal run of two or more letters; its owner is whoever placed its latest letter (by place order). Real words come from ENABLE (`data/words.txt`), loaded once. Scoring follows the brief: letter values, premiums for every word through them, fake words negative, the idle penalty. | BRIEF.md, Words and Scoring. | hand-worked boards in `sim/words.rs` tests |
| 12 | **Drawing.** The core poses everything; the page replays. A frame is one or two passes (the main view, the overhead view), each a viewport, its view values, the static layers it draws and its own moving triangles. Static layers (the world; the tiles at rest, in buckets by id; the word marks) carry a version, and the page uploads one only when its version changes. | 60 000 triangles of seafloor, board and tiles are too many to send every frame; the crabs and held tiles are a few thousand. | `a_static_layer_keeps_its_version_until_it_changes` |
| 13 | **HUD in panels.** The clock, the idle countdown under it, the "-2" popups, the word preview and the tally are core panels; the crosshair is a page element the core turns on. No scores during play. | D3: the page holds no rule; KISS text (TONE.md). | browser check steps |
| 14 | **Tuning.** Walk 394 mm/s (the 3150 mm board in 8 s); a jump whose apex is 65 mm (just over a 45 mm tile on the board) with 700 ms in the air, the gravity worked out from those two; reach, step height, push, the idle numbers and the tile physics in `data/tuning.json`. | The person's numbers (R1.187); every balance number is theirs to change. | `tuning_file_is_complete_and_valid` |

### Build order

1. **Stage 1, a playable walk.** `sim::Match` with the ground, the crabs (WASD, mouse look, floaty jump, landing on tiles and mounds, gentle pushes) and the clock; the match screen replaces the stand-in: the seafloor map, the board, the piles (still), the size-C crab, the crosshair and pointer lock. AI seats stand still. Local game with 1 to 3 AI crabs.
2. **Stage 2, grab, place, drop.** Tile ids and states, the physics, claims and the local referee, aim, the held tile's look (half size, face up, tipped 60°), the ghost, placing and dropping, the random centre letter, 294 tiles. The grab and place moves retuned for the small crab (R1.184), and the held tile kept out of the board's rim at the corner.
3. **Stage 3, words and the HUD.** Words, owners and marks on the 3D board and in the overhead view (Tab: 90% of the screen); the word preview; the idle countdown and its "-2" popups.
4. **Stage 4, the end.** The tally, one line at a time (Honey Hole's), the winner, Rematch or back.

## Open questions for the person

Numbered, so they can be answered by number.

1. None for this build; decisions made alone are rows in `SECOND-ORDER.md`.

## Status

- [x] Read by the person on: 2026-10-19 (gameplay approved in advance: "Yes go ahead with gameplay build")
