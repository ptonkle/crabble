# Second-order notebook

Stage 6, kept by the agent as the work happens, not afterwards. `B3 Build in self-questioning`.

The code and tests are the work; this file is the record about the work. It is where a decision the agent made alone, an assumption that turned out wrong, or something only the person can do becomes visible to the person. With the plan approved in advance, it is the main place those decisions surface before the person uses the tool. [`docs/procedure.md`](https://github.com/sgilson7/builder-setting/blob/main/docs/procedure.md#the-second-order-notebook) says why.

One row per assumption, surprise or open item. **Kind** is *divergence* (what was assumed, what turned out to be true, what changed), *finding*, or *worklist* (the person's to do). Number rows by round (R1.1, R1.2, then R2.1 after the first triage) so a later round does not renumber an earlier one. **Rule** is the rule the row produced for later sessions, and where it now lives (`CLAUDE.md`, a test, `tests/breaks.json`), or blank. **Status** is *done*, *open*, or *person*; every *person* row is named in `HANDOFF.md`. `scripts/notebook_check.py` checks the form of each row and that the notebook is not empty once `PLAN.md` holds a decision, not what the rows say.

| # | Kind | Assumed or found | What is true | What changed | Rule | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R1.1 | divergence | The journal is the template's own (`tools/journal/new_journal.py`). | The person's standing instructions use design-journal-kit's form (`../crabble-journal/`, a page per area, published as an artifact). The template's `scripts/journal_check.py` still runs in `check.sh`. | The journal is the kit's form; every entry also carries an `id` and a `data-kind`, and the first is a `reading` entry, so `journal_check.py` reads it. | `.claude/skills/design-journal/SKILL.md` | done |
| R1.2 | finding | `TRIAGE.md` is committed, as in the template. | The person's rule: todos are written to `TRIAGE.md` and never committed or pushed. | `TRIAGE.md` is in `.gitignore` and stays untracked; it holds the future ideas (leg IK first). | `.gitignore` | done |
| R1.3 | divergence | The description goes in a text fence, as in Honey Hole. | `journal_check.py` decides the brief is written by finding a `>` quote under the word-for-word heading. | The description and the answers are quoted line by line with `>`; the words are unchanged (`brief/description.md` keeps the original). |  | done |
| R1.4 | divergence | Three standard sets are 300 tiles. | A standard English set is 100 tiles with 2 blanks; with no blanks, three sets are 294 tiles. | The brief says 294; tuned later with the AI. | BRIEF.md, Tiles | done |
| R1.5 | finding | Scrabble's scoring includes the 50-point bonus for playing all seven tiles. | There are no racks and tiles go down one at a time, so the bonus has no meaning here. | No bonus. Scoring is letters, letter premiums and word premiums only. | BRIEF.md, Scoring | done |
| R1.6 | finding | The brief names the look only as "lightly underwater and cartoony". | The menus and HUD need a concrete palette and shapes, and must not be Builder's default pack. | Chose "tide pool": deep sea-blue rounded panels with a pale aqua rim, sea-glass aqua pill buttons, coral second accent, foam-white rounded text, slow bubbles; green kept for real words only. Shown in the journal for approval. | BRIEF.md, Style | open |
| R1.7 | finding | The brief says the crab walks sideways and WASD moves it, but not which way the crab faces. | Crabs walk sideways; the claws map to the mouse buttons, which reads best with the claws ahead of the camera. | The crab faces where the camera looks, claws forward; A and D scuttle sideways (the walk-sideways gait), W and S shuffle forward and back. To confirm in the journal. |  | person |
| R1.8 | finding | The brief gives sizes only as "tiles about as big as the crab's abdomen". | A crab's abdomen is the flap folded under its body, about 40-45% of the shell's width. | Scale: a tile is 200 x 200 x 45 mm on 210 mm squares (the board is 3150 mm across); the crab's shell is about 460 mm wide, so its abdomen is about one tile. Reach is about 210 mm beyond the claw tips. | model files; BRIEF.md | done |
| R1.9 | finding | "Do all the online syncing up the same" as Ape Race. | Ape Race lets each browser own its ape. A tile can be grabbed or placed by two browsers at once, and the person wants one answer everywhere. | Crabs sync as Ape Race's apes (owner sends its state at 20 Hz, host runs the AI). Tiles are the host's: grabs and placements are requests, the host settles who was first and everyone shows its answer. | DECISIONS.md, 2026-10-19 (online sync) | open |
| R1.10 | finding | The centre square is Scrabble's double word star. | The starting letter sits on it, and a premium counts for every word through it (Q7). | Every word through the centre is doubled, for the whole game. A balance point to watch with the AI. |  | open |
| R1.11 | finding | "Who owns a word" needed a precise rule. | The last player to add a letter to a run is the placer of the run's most recently placed tile. | Owner = the player who placed the latest tile in the word; the starting letter belongs to no one. Two placements at once are ordered by the host. | BRIEF.md, Words and ownership | done |
| R1.12 | finding | ENABLE covers the words players expect. | ENABLE is public domain and close to the North American tournament list for everyday words, but it predates some newer short words. | To check when the list is added: which common two-letter tournament words it lacks, listed for the person. |  | open |
| R1.13 | worklist | Anti-camping is settled. | The person first kept the points penalty after about 30 s without placing as a candidate, then settled it the same day: -2 points a second past 30 s idle, live "-2" popups, a "Place a tile!" countdown from 10 s before. | Recorded in BRIEF.md and DECISIONS.md; the countdown and popup are mocked up in the journal. Dropped: time-weighted ownership, random end time, heavy tiles. | BRIEF.md, Anti-camping | done |
| R1.14 | finding | "Starts the game at a corner of the board": on the corner square or beside it. | The corner squares are triple word squares and a crab standing on one would cover it. | Each crab starts on the sand just off its corner, facing the centre: red, yellow, blue, purple clockwise from the near left. |  | done |
| R1.15 | finding | The brief says tiles lie mostly in piles on mounds around the board, some alone. | The number and size of piles is not given. | First layout: 12 piles of about 20 tiles on low mounds around the board, and the rest (about 50) lying alone; about a third face down. Tuned with the AI. |  | open |
| R1.16 | finding | The word preview shows the words a placement forms and whether each is complete. | Scores are hidden until the end; points in the preview would be a running score. | The preview shows each word and a mark for real or not, no points. Real words read green with a tick, so colour is not the only cue. |  | open |
| R1.17 | finding | Holding Tab enlarges the overhead view. | The brief does not say whether the crab can still move. | The game goes on and the crab still moves; only the view changes. |  | open |
| R1.18 | worklist | Crab legs that place themselves dynamically (IK) are part of this round. | The person: "that animation can be left as a todo for later". | Written to `TRIAGE.md`'s future ideas (untracked, R1.2). |  | done |
| R1.19 | finding | The tone file bans "!" in every string. | The person's own words for the countdown are "place a tile!". | Strings under the `idle.` prefix may end with one "!", as Honey Hole's popups do; to be wired into `copy.rs`'s `ALLOWED` when the string is added. The penalty popup's number comes from the core (`{idle.points}`). | TONE.md, Project decisions | done |
| R1.20 | divergence | The template's page holds a `State` JSON and hands it back on every `act`. | Ape Race's menus need a stateful app with a clock; a state round-trip through the page is page-held state (Honey Hole's R1.20). | The shim holds one `Game` around `api::Session` and `app::App`: `new(strings, notes)`, `ui()`, `act(name, value)`, `resize`, `scene`, `net_event`, `net_code`, `net_pump`. `page.rs` is gone. | `crates/core/src/api.rs`; `shim_decides_nothing` | done |
| R1.21 | divergence | The template sends glossary definitions to the page as hover hints (D15). | GLOSSARY.md already says the page shows no definitions, as in Ape Race and Honey Hole. | No `Hint` in the view model and no glossary copy in `scripts/build.sh`; `GLOSSARY.md` and its copy tests stay. Recorded in BUILDER-SPEC.md's change table. | BUILDER-SPEC.md changes table | done |
| R1.22 | divergence | The template's strings file is a fixed struct (`title`, `intro`). | The menus need many strings by key. | `strings.rs` (Honey Hole's): a key-to-text map; the core lists every key it reads in `KEYS`, and a missing, empty or unused key is refused by name. `tests/breaks.json`'s rows for `page.rs` now break `strings.rs` and `app.rs`. | `crates/core/src/strings.rs` tests | done |
| R1.23 | worklist | Settings are kept in browser storage, as in Ape Race and Honey Hole. | Honey Hole stores them because its person chose it (its brief P3, its D17 change). Crabble's brief allows no storage (D17 stands). | Volume, mouse sensitivity, Invert mouse Y and the name last for this visit; the Settings glossary entry says so. To keep them between visits, the person says so and Honey Hole's `web/settings-store.js` and its `no_storage.rs` allowance come over. | GLOSSARY.md (Settings); `crates/core/tests/no_storage.rs` | person |
| R1.24 | divergence | What's New is marked "New" until this version's notes are read, as in Ape Race. | The mark needs the seen version kept between visits, which needs storage (R1.23). | Left out. The button item still carries `badge`, so it can return once storage is decided. |  | open |
| R1.25 | finding | Ape Race builds its patch notes into the core. | D5 puts reader-visible text in `data/` (Honey Hole's R1.25). | The page fetches `data/patch-notes.json`; the core checks each version, date, line and the newest-first order. `scripts/bump.sh` (Ape Race's) moves `VERSION` in `app.rs` and adds the entry; `tests/data_files.rs` fails when the newest entry is not `VERSION`. | `crates/core/tests/data_files.rs` | done |
| R1.26 | divergence | A refresh puts a player back in their lobby, as in Ape Race. | Ape Race keeps the room and the client id in session storage (its T72) and has an `Underway` message for a seat back mid-race; no storage here. | Honey Hole's way: a joiner's address keeps `#room=CODE`, so a refresh joins again (as a new client id). A host's refresh loses its lobby, and a joiner refreshing mid-game is refused as under way; `Underway` is not ported. | `room_hash` in `app/online_play.rs`; `tests/browser/online_game.py` | open |
| R1.27 | divergence | Ape Race's lobby comes over whole. | Its roster also carries ape kinds, palettes, costumes, the map, items, Equal Stats, Floor is Lava, weather, tournaments, the screech, calls and stall notices: game content. | The lobby carries the code (shown big, Copy code, Copy link, invite links), the pasted-code path, names, seat colours, Ready marks, the AI count, pings, the three refusals, Start and Rematch. One AI level only (the person, 2026-10-19), so no difficulty. | `crates/core/src/online/` | done |
| R1.28 | finding | Ape Race's labels ("Room code", "Play", "How to play") can be used. | GLOSSARY.md rules out "room" (Lobby) and "play" (Place). | "Code" for the room code and "Start" for Play. "How to play" is the person's own name for the screen: allowed in `copy.rs` with its reason. The trademarked board game's name is refused in every data string by a test (the person's rule). | `ALLOWED` and `no_reader_string_names_the_board_game_by_its_trademark` in `crates/core/tests/copy.rs` | done |
| R1.29 | finding | Each seat's colour is fixed to the player. | Colours go by lobby place (red, yellow, blue, purple), people first, then AI crabs, as Ape Race's seats do. | A person leaving the lobby moves the later seats' colours up one. The lobby names each colour in words beside its dot, so colour is not the only cue. | `SEAT_COLOURS` in `app/online_play.rs` | done |
| R1.30 | divergence | The stand-in Play draws through the WebGL renderer, as Ape Race's and Honey Hole's do. | The renderer (`web/gl.js`, `gfx/`) is being built in parallel; a second one would collide at the merge. | The stand-in projects its crabs and sand in the core with its own sine table (`standin/trig.rs`, `standin/draw.rs`) and the page replays flat polygons on a 2D canvas (`web/standin.js`). Both go when the real game draws through the renderer. | `crates/core/src/standin/` | open |
| R1.31 | finding | The stand-in needs AI crabs that play. | The real AI comes with the real game; the stand-in only needs them to move. | AI crabs walk to random points (`tuning.wander`). Online, only the host runs them and sends their states. | `standin::Game::step` | done |
| R1.32 | divergence | `?fast` alone lets the browser checks reach every screen. | It skips the 3-2-1 and leaves rule timings alone, so the end of a 4-minute game is out of reach. | A second test flag, `?short` (3 s) or `?short=N` (N seconds), sets the game's length; never set for players. The end-screen step and the online game check use it. | `crates/core/src/app/fast.rs` | done |
| R1.33 | finding | Two browsers can always connect. | The transport uses STUN only, no TURN (Ape Race's N2): two players both behind strict NATs cannot connect. | Kept as in Ape Race and Honey Hole. A TURN server would be a server the brief does not allow. | `web/online-config.js` | open |
| R1.34 | finding | Escape pauses every game. | Ape Race's pause stops the race offline and only opens the menu online. | The same: a Local game's clock and crabs stop under the pause menu; an online game runs on behind it. | `play_advance` in `app/play.rs` | done |
| R1.35 | finding | Each browser ends its own game at 0:00. | Clocks drift a little between browsers, and the host is to settle tiles. | The host's clock ends the game: its End is repeated until the next round; a joiner's clock holds at 0:00 until it arrives, or until the host leaves. | `Session::end_game`, `Game::hold_at_zero` | done |
| R1.36 | finding | Escape always reaches the page. | With the pointer locked the browser takes Escape to let the pointer go. | The page tells the core (`pointer.released`) and the core opens the pause menu. | `play_input` in `app/play.rs` | done |
| R1.37 | finding | Ape Race names AI seats "AI 1", "AI 2". | A number is a count on screen; the colour already tells them apart. | Each is "AI crab" beside its colour. |  | done |
| R1.38 | finding | `scripts/journal_check.py` finds the journal from any checkout. | From a git worktree it looked for `<worktree>-journal`. | It finds the checkout a worktree belongs to (`checkout`) and looks beside that. | `checkout` in `scripts/journal_check.py` | done |
| R1.39 | finding | The menus' rounded heavy face is on every machine. | Linux has no rounded system face, so the menus fall back to the system's bold sans (DejaVu Sans Bold in the screenshots). | Left. A vendored OFL face (Nunito, say) with a SHA256SUMS pin and a LICENSES row would fix it; it needs a download. | `--face` in `web/style.css` | open |
| R1.40 | finding | The vendored Trystero bundle could be any copy. | Ape Race's and Honey Hole's are the same file (sha256 6bfce15d...). | Copied with its SHA256SUMS; `scripts/build.sh` checks it and refuses an unpinned file beside it. Rooms are kept apart by appId `crabble-p2p`, password `crabble` and the protocol in the room name. | `scripts/build.sh`; `web/online-config.js` | done |
| R1.41 | divergence | The quick check also runs the online check, as Ape Race's does. | Honey Hole's quick check (this template's) has no online steps (its R1.1). | Not added: an `online` change runs the online core tests and the lobby's browser step; `./scripts/check-online.sh` stays a separate run. `scripts/timed-checks.sh online` logs it. | `scripts/timed-checks.sh` | open |
| R1.42 | finding | The How to play screen was in this round's brief. | The person asked for it during the port (2026-10-19), with no numbers and no exact rules. | A Title entry, five short lines and five controls; a browser step checks there are five lines and no digit. | `app/how.rs`; `step_how` | done |
| R1.43 | finding | The template's `check.sh` stages carry over. | Honey Hole's `check.sh`, the base of this one, had dropped the template's journal stage. | Restored as stage 10/10, last, so its failure does not hide the other stages (R1.45). | `scripts/check.sh` | done |
| R1.44 | finding | A lobby convenience that needs a seat id survives a new link. | Ape Race keys seats by a client id the page makes, not by the transport's peer id. | The same: `flag.client` from `crypto.randomUUID()`; a newer link from the same id takes over the seat. | `Session::on_hello` | done |
| R1.45 | worklist | The journal stage passes. | `scripts/journal_check.py` fails on the journal as it is: no opening `reading` entry, entries without an id or `data-kind`, and R1.7 (person) not named in it. The journal is not this port's to write. | Left for the journal's owner. Every other stage of `check.sh` passes. | `scripts/journal_check.py` | open |
| R1.46 | finding | Rematch must set every joiner who stayed Ready again, as Ape Race's does (its T56). | The break audit (`scripts/break_tests.py`) found the loop changed nothing: a game never clears a Ready mark, so those who stayed are still ready. | The loop is gone; the test `the_hosts_end_reaches_the_joiners_and_rematch_brings_everyone_back_ready` keeps the behaviour. Nine rows in `tests/breaks.json`, all caught. | `tests/breaks.json` | done |

Row ranges for parallel agents: R1.20-R1.59 the tooling and menu port, R1.60-R1.89 the model format, R1.90-R1.199 the model and design-material agents (each given its own range).

## Strings written by the agent

Each reader-visible string the agent wrote, with its key, for the person to read.

| Key | String |
| --- | --- |
| `title` | Crabble |
| `menu.local` | Local game |
| `menu.online` | Online game |
| `menu.how` | How to play |
| `menu.settings` | Settings |
| `menu.notes` | What's New |
| `how.goal` | Make words on the board with letter tiles. |
| `how.claws` | Each claw grabs a tile, places it on the board, or drops it. |
| `how.claim` | Adding the last letter to a word claims it. |
| `how.fake` | A word that is not a real word costs its owner points. |
| `how.idle` | Failing to place a tile for a while brings a penalty. |
| `how.move` | Move |
| `how.move_keys` | W A S D |
| `how.look` | Look |
| `how.look_keys` | Mouse |
| `how.claw` | Left and right claw |
| `how.claw_keys` | Left and right click |
| `how.jump` | Jump |
| `how.jump_keys` | Space |
| `how.overhead` | Overhead view |
| `how.overhead_keys` | Hold Tab |
| `menu.back` | Back |
| `setup.ai` | AI crabs |
| `local.start` | Start |
| `pause.heading` | Paused |
| `menu.resume` | Resume |
| `menu.quit` | Quit to Title |
| `end.heading` | Time's up |
| `end.rematch` | Rematch |
| `end.title` | Back to Title |
| `settings.heading` | Settings |
| `settings.volume` | Volume |
| `settings.mouse` | Mouse sensitivity |
| `settings.invert_y` | Invert mouse Y |
| `notes.heading` | What's New |
| `notes.minor` | Minor versions |
| `online.name` | Name |
| `name.default` | Crab |
| `online.host` | Host |
| `online.code` | Code |
| `online.join` | Join |
| `online.code_shape` | A code is {n} letters and digits. |
| `online.host_code` | Host by pasted code |
| `online.invite` | Invite code |
| `online.join_code` | Join by pasted code |
| `online.reply` | Reply code |
| `online.connect` | Connect |
| `online.next_invite` | Next invite |
| `online.copy` | Copy code |
| `online.copy_link` | Copy link |
| `online.copied` | Copied |
| `online.ready` | Ready |
| `online.unready` | Not ready |
| `online.leave` | Leave |
| `online.joining` | Joining |
| `lobby.heading` | Lobby |
| `lobby.colour` | Crab |
| `lobby.state` | Ready |
| `lobby.ping_head` | Ping |
| `lobby.you` | {name} (you) |
| `lobby.host` | Host |
| `lobby.ready` | Ready |
| `lobby.not_ready` | Not ready |
| `lobby.ai_crab` | AI crab |
| `lobby.ping` | {ms} ms |
| `colour.red` | Red |
| `colour.yellow` | Yellow |
| `colour.blue` | Blue |
| `colour.purple` | Purple |
| `online.refused.full` | That lobby is full. |
| `online.refused.underway` | That game is under way. |
| `online.refused.version` | That lobby runs another version. Reload the page to update. |
| `online.host_left` | The host closed the lobby. |
| `online.lost` | The host's connection dropped. Join again. |
| `online.no_bundle` | The online game could not load. Reload the page. |
| `online.no_relay` | No relay answered. Check the connection, then try again. |
| `online.no_path` | Could not reach another browser. Try again, or another network. |
| `online.signal_failed` | The connection could not be set up. Try again. |
| `online.wrong_code` | That code goes in the other box. |
| `online.bad_code` | That code is cut short. Copy it again. |
| `online.not_hosting` | Host first, then make the next invite. |
| `online.no_offer` | Make an invite first. |
| `patch-notes 0.1.0 [0]` | First version: the Title, a Local game, an Online game with its lobby, How to play, Settings and What's New. |
| `patch-notes 0.1.0 [1]` | The game itself is a stand-in for now: crabs walk the sand while the clock counts down. |
