# Decisions

Dated entries. Each quotes the person's request word for word, then says what the agent decided and where it is recorded. Grants of autonomy go here too ("you may publish"), with their scope: this run only, or until withdrawn.

## 2026-10-19 (what to port)

> Copy the general ideas for menu layout from Ape Race, where there is a main menu, and you can laucnh into either local game or online game, do all the of the online syncing up the same. This is proven to be good.

> rely on ape race more than honey hole, it is a more complete game.

Standing rules from earlier games, which apply here too: a new slop game takes Ape Race's general machinery and not its look, wording, content or design habits; the Ape Race lobby counts as general machinery and comes over with all its conveniences; the `?fast` mode comes with the check tooling.

The agent decided:

- **From Ape Race:** the menu layout (Title, then Local game or Online game; Settings; What's New with version numbers and patch notes), the online lobby with every convenience (room code, copy code, copy link, join from an invite link, ready marks, adding AI seats, rematch), the online approach and transport (WebRTC between up to four browsers, introduced by room code over pinned Nostr relays or by a pasted code; a pure state machine in the core), test tags and the quick check, check visibility (nextest, progress bars, watch window, check times), the `?fast` mode for browser checks, and the design journal.
- **From Honey Hole:** the hand-written model file format (`data/models/*.json`), its renderer and Gallery, and the end-of-game tally.
- **Not ported:** Ape Race's or Honey Hole's names, screens' wording, palettes, menus' look, maps, items and tuning.

## 2026-10-19 (online sync)

> Online will be important to determine who picked up a given tile and who placed a given tile first. Make sure these states stay in sync.

The agent decided (SECOND-ORDER R1.9): crabs sync as Ape Race's apes do (each browser moves its own crab with no delay and sends its state about 20 times a second; the host runs the AI crabs). Tiles are the host's: a grab or a placement is a request to the host, which settles who was first, and every browser shows the host's answer. The full design goes in `PLAN.md` before any online code is written for play.

## 2026-10-19 (round 1 answers)

The questions, shortened, and the person's answers word for word, are in `brief/description.md` and quoted in `BRIEF.md`.

The agent decided:

- Q1: the board is 15 × 15.
- Q2: every placed tile shares a side with a tile already on the board.
- Q3: reach is about one tile's width from the crab (one value, tuned in play).
- Q4: an empty claw's click grabs the tile under the crosshair; a full claw's click places at the ghost when the crosshair is on a legal square, and otherwise drops the tile. Two tiles at most.
- Q5: a face-down tile carries no hidden state; its letter is drawn on its face and just can't be seen from above.
- Q6: a word is a maximal run of two or more letters; its owner is whoever last added a letter to it.
- Q7: a premium square counts for every word through it, every time.
- Q8: an invalid word at the end costs its owner its full score, premiums included.
- Q9: the word list is ENABLE. The person asked: "Yes, but really we can't use the tournament list? what is with that?" The tournament lists are not free to ship: the North American list (NWL, formerly TWL) belongs to NASPA and Collins Scrabble Words to HarperCollins, and both are licensed, not public. A game whose files are public cannot include either without a licence. ENABLE is in the public domain and close to the North American list for everyday words; it lacks some newer short words. The list is one data file, so a licensed list could be swapped in later (SECOND-ORDER R1.12).
- Q10: three standard sets to start, tuned later with the AI. Without blanks that is 294 tiles, not 300 (R1.4).
- Q11: a local game is you against 1–3 AI crabs; no split screen.
- Q12: crabs push each other a little; no stealing, no knocking out.
- Q13: the early design materials go in the design journal.

## 2026-10-19 (anti-camping)

The ideas offered and the person's answers, word for word:

> 1. Extending only earns the difference — "I don't think works because then if a single player spelled A into App into Apple, do they score for all the words? I like the stealing it's abig part of the game"
> 2. A held tile slips out after ~20 s — "there would be nothing to stop them from just dropping and picking it up constantly though"
> 3. Tiles left in claws at the end count against you — "nah that's weird that if you're at the end of the game you need to remember to drop everyting before the game ends"
> 4. Points penalty after 30 s without placing — "let's keep this idea, but think of more"
> 5. Words lock after ~30 s untouched — "no"

The agent decided: the points penalty after about 30 s without placing is the current candidate and stays open. Settled later the same day; see below.

## 2026-10-19 (look)

> Use the same graphical style from ape-race and Honey Hole, of 3d geometric shapes. Title card things should be "crabble" spelled out with scrabble tiles. The map should be just on a sandy seafloor udnerwater. the boar dshould be totally flat and horizontal but there can be small mounds and such around the outside wher you find the scrabble tiles.

> STyle should be lightly underwater and cartoony. Do not fall back to the Builder framework style.

The agent decided: the menus and HUD take a "tide pool" look (BRIEF.md, Style; SECOND-ORDER R1.6), shown in the design journal for approval before the HUD is built.

## 2026-10-19 (anti-camping, settled)

> 30 seconds without placing a tile is the penalty. -2 points every second you wait past 30 second ... Give the player live "-2" number popups to make this clear. Also if you're within 10 seconds of starting to accrue penalty, put a countdown in the corner of the screen that says something like "place a tile!"

The agent decided: the idle timer starts with the game and resets on each placement. From 20 s idle a corner countdown shows "Place a tile!" with the seconds left; past 30 s the player loses 2 points a second, with a "-2" popup each second that only that player sees. The three numbers live in tuning data. Dropped and not to be built: time-weighted ownership, a random end time, heavy tiles. The countdown and the popup are drawn in the journal (SECOND-ORDER R1.13, R1.19).

## 2026-10-19 (AI level)

> Just do a single medium level of ai for now, don't want to overdesign it if it turns out the game sucks

The agent decided: one AI level (medium), no difficulty picker in the local setup or the lobby, and a simple AI. It still takes visible thinking time before choosing a letter to grab and a square to place it on.

## 2026-10-19 (sounds)

> oh also do some basic sounds for picking up tiles, placing tiles, completing words, etc.

The agent decided: sounds are synthesised by the core as WAV bytes (Ape Race's approach, no audio files to license), from a small data file of sound recipes, and played by the page. The first set: grab, place, real word, drop, jump, land, idle tick, "-2", countdown, end, tally tick. Each is in the journal to listen to.

## 2026-10-19 (How to play, no trademark)

> DO NOT OVEREXPLAIN IN THIS. FOR EXAMPLE YOU DO NOT NEED TO NAME THE EXACT POINT LOSS OR TIMING ON WHEN YOU START LOSING POINTS. JUST SOMETHING LIKE "FAILING TO PLACE A FOR A WHILE TILE WILL RESULT IN A PENALTY" OR SOMETHING.

The agent decided: How to play is a Title entry with a handful of short lines and a controls list, no numbers. "Scrabble" never appears in player-visible text (on TONE.md's lint list).

## 2026-10-19 (crab size and held tiles)

> crabs are way too big compares to the tiles

> eh I was still imagining feeling more dwarfed by the tiles and board. Try three different smaller sizes for comparison.

> I like C, but let's shrink the tile specifically while you're holding it so that the crab to tile ratio is like A

> crab should also hold the tile up more at an angle such that it's easy to see what the tile is instead of holding it horizontally

The agent decided (SECOND-ORDER R1.8, R1.186): the crab has a 90 mm shell (size C), the camera 352 mm behind it and 196 mm up. A held tile is drawn at half size (100 mm), shrinking over 0.15 s when grabbed and growing back when let go; a placed tile and the ghost are always full size. A held tile is tipped 60 degrees up toward the camera so its letter reads. Reach is within one board square (210 mm) of the claw tips. Walk speed and jump height are to be retuned for the small crab (R1.187).

## 2026-10-19 (held tiles face up)

Beside the round 1 answer to Q5 ("there is no state change about them when you pick them up"):

> oh sorry when you pick it up it shoul dface up immediately

> yes they should flip it face up

The agent decided (SECOND-ORDER R1.188): a grabbed tile turns face up during the pickup shrink and is always held letter toward the camera. Face down only matters for tiles lying on the sand.

## 2026-10-19 (journal answers)

The journal's questions CR-Q1 to CR-Q11 and the person's answers, word for word:

> CR-Q1 (which way the crab faces) — "yes camera faces where camera looks"

> CR-Q2 (keep Settings between visits) — "yes"

> CR-Q3 (a letter in a real word and a fake one: green?) — "green whne only all real"

> CR-Q4 (premium square colours) — "sure"

> CR-Q5 (word outlines on the tile tops, fake words dashed) — "I think it's fine for now but keep in mind"

> CR-Q6 (ghost tile look) — "looks good"

> CR-Q7 (which title card) — "standing"

> CR-Q8 (the centre star) — "are center words normally doubled? if that's how it works in scrabble then ok"

> CR-Q9 (holding one tile: one claw up or both) — "only full clas rises"

> CR-Q10 (a game clock on screen) — "yes there needs to be an overall timer, and show the idle timer conditionaly below it"

> CR-Q11 (the word list) — "I don't want those stupid short nonwords that no one knows, so don't include beyond what is in ENABLE"

The agent decided:

- CR-Q1: the crab faces where the camera looks; A and D scuttle sideways, W and S shuffle forward and back (R1.7).
- CR-Q2: settings (volume, mouse sensitivity, invert mouse Y) and the online name are kept on this device between visits, as Honey Hole keeps them: one file, `web/settings-store.js`, under one key; the core writes and checks what is kept (R1.23; BUILDER-SPEC, the D17 change).
- CR-Q3: a letter is green only when every word through it is real (R1.180).
- CR-Q4: the premium colours stay: burnt orange 3W, shell pink 2W, sea blue 3L, pale aqua 2L, each labelled (R1.181).
- CR-Q5: the dashed outline for a word that is not real stays; a watch item, since its dashes look like the shared-letter dashes (R1.151, R1.153).
- CR-Q6: the sea-glass ghost with its aqua ring (R1.156).
- CR-Q7: the standing title card. Putting it on the Title screen in 3D is left for a later session (R1.189).
- CR-Q8: in Scrabble the centre star is a double word square that counts only for the first word; here every premium counts for every word through it (Q7), so the star doubles every word through the centre all game. Kept, the coordinator answering the person's question (R1.10).
- CR-Q9: holding one tile, only the claw with it is raised; the empty claw stays down (R1.98).
- CR-Q10: the match clock stays top right; the "Place a tile!" countdown appears directly below it, only in its 10 s warning window (R1.182).
- CR-Q11: plain ENABLE, with no added words (R1.12).


## Publishing (2026-10-19)

The person, word for word: "deploy to pages when you can we'll balance later", then "you can deploy, you ahve done it many times yourself". The agent may create the public repository ptonkle/crabble and publish to its GitHub Pages site with `./scripts/publish.sh` once `./scripts/check.sh` passes (BRIEF.md, Autonomy), as for Honey Hole.

## 2026-10-20 (idle grace, piles nearer, scenery: F9, F14)

The person, word for word: "ok execute all the todos except co op mode". Their todos, word for word: F9 "penalty timer should be 40 seconds too, and yes we can move the piles in a bit"; F14 "add more scenery". On the tile count (CR-Q12, R4.11): "don't recalculate tile count, it felt fine".

- The idle grace is 40 s (`data/tuning.json` `idle.grace_s`); the "Place a tile!" countdown still shows for its last 10 s, from 30 s (R9.1).
- The piles sit 0.8-1.0 m off the board's edge, three a side, the same on every side (`tools/piles.py`, R9.2).
- The tile count stays 294, three standard sets; R4.11 is closed (R9.3).

## 2026-10-20 (todos F5 and F11: renaming in the lobby, the AI's words)

> ok execute all the todos except co op mode

> (F5) add option to update name in once you've joined the online lobby

> (F11) cull exteremely uncommon words out of the list, i saw the ai using these a lot and it's just not fun

> (F11, decided) I guess let's just plan to cut them from the AI

The agent decided:

- F5: the Lobby already had Ape Race's name field (TRIAGE T35 there) once a seat is given, kept in Settings and passed on by the host to every seat. It now shows while a joiner waits for its seat too, and tests cover a rename reaching every seat with messages lost (SECOND-ORDER R12.1).
- F11: ENABLE still judges every word (CR-Q11 stands); the AI makes only words in its vocabulary, the 44,505 ENABLE words SCOWL ranks common (level 40, 35 for words of 3 letters or fewer), `data/words/common.txt`, made by `tools/words/common.py` (SECOND-ORDER R12.2-R12.5; the cutoff is the person's to check, R12.3).
## 2026-10-20 (word owners' marks, F8)

The person, word for word:

> the indicators of word owners needs a  ton of work, the higher floating things don't work at all. suggest alternatives. Especially around when two words use the same letter

> hmm maybe iut only needs to show your own info so like you don't need to know who owns every word, just what words you own

> and then in additoin to green word it's solid surrounding line for a completed word and dashed for an incomplete one on f8

Asked whether the Tab view should still show everyone's:

> f8 no you don't need to keep track of others words, it is hard enough to find where to place letters, we don't need more confusion.

The agent decided (SECOND-ORDER R7.1-R7.3): the floating strips on the tiles' tops are gone. Each word you own gets a line all the way round it in your colour, at the board: a strip on the board at the tiles' feet, a wall up their outer sides and a rim on their top edges, so it reads from behind the crab and from above. The line is solid when the word is real and dashed when it is not; letters still turn green only when every word through them is real. Where two of your words cross, their lines meet and neither runs through the other. Other players' words get no owner marks in any view: the 3D view, the corner overhead view and the Tab view. The alternating colours on shared letters are gone with them.
## 2026-10-20 (a livelier Title, F4)

The person, word for word (TRIAGE.md F4): "backgroudn on main screne should have crabs walkign about and just generally more interest", then, of the todos: "ok execute all the todos except co op mode".

The agent decided (SECOND-ORDER R11.1-R11.8): four crabs, one per seat colour, live on the Title's sand round the CRABBLE card. Red scuttles sideways right across in front of the card and back; yellow fusses with an S tile beside the menu, carrying it a short way, setting it down, picking it up again; blue peeks out from behind the E and ducks back; purple ambles on the left. Two small fish swim by behind the card now and then. All move slowly (90-150 mm a second against the game's 394), rest 3-8 s at each end, keep out of the menu's rectangle at 16:9, and use the game's own crab moves (`app/view3d.rs` `pose_crab`). Where each goes is `data/look.json` `title.crabs` and `title.fish`. Without WebGL the Title still shows the wordmark alone.
## 2026-10-20 (tile physics, F7)

> improve physics of scrabble tiles

(TRIAGE.md F7, 2026-10-19), then, for every todo then listed:

> ok execute all the todos except co op mode

The coordinator's brief for this todo: tiles tip and rotate when they land on an edge or off-centre on another tile, slide down mound slopes and off pile edges, topple when their support is pulled, crabs nudge loose tiles they walk into, piles look messy and settle believably, nothing jitters or sinks; still whole-number maths (a replayed `Match` comes out the same) with the host's resting poses winning online (R5.8); cheap with 294 tiles at 50 ticks a second.

The agent decided (SECOND-ORDER R8.*):

- **A tile awake is a light rigid box with a velocity and a spin** (`crates/core/src/sim/tumble.rs`), replacing the plane fitted under its edges (R2.3). It tips over an edge when its middle overhangs what it touches, slides down its own face by gravity less wood-on-wood friction, stops against the side of another tile, and turns over when it goes past upright (R8.1).
- **`tiles.steepest_deg` is gone**: a tile now leans as steeply as what holds it lets it (R8.2).
- Stage 2: crabs nudge loose tiles they walk into (R8.9).

## 2026-10-20 (blank ghost, F10)

> ghost tiles currenlty always show what's in your left hand I think. I guess just have it be blank always.

The agent decided (SECOND-ORDER R7.4): the ghost tile is blank whatever is held. Holding one tile, the word preview is as before. Holding a tile in each claw, the preview shows the words each claw's tile would make, left claw's first, each row starting with "L" or "R". The preview shows before any click, so it cannot know which claw will be clicked.

## 2026-10-20 (grab highlight, F6)

> highlight on the ground tiles whne they're grabbable

The agent decided (SECOND-ORDER R7.5): the core names the loose tile a free claw's click would grab now (`Match::grabbable`, from the same rule as a click). The view draws a see-through sea-glass collar round that tile, in the ghost ring's aqua.

## 2026-10-20 (crab motion: TRIAGE F1 and F12)

> it would be nice if the crabs kinda dynamically were able to place their legs to crawl things liek the pile of tiles with the legs being able to dynamically place themselves.

> make crab animation more dynamic

> ok execute all the todos except co op mode

The agent decided (SECOND-ORDER R10.*): the extra motion is presentation only, worked out by the core from what it already knows of each crab (its gait, speed, turn, time in the air, last grab or place), so a crab another browser moves shows it too and no rule changes. Its numbers are `data/look.json` `crab`. F12: the body bobs with each step, leans into a scuttle and banks into a turn; the crab stretches leaving the ground and squashes landing, harder the faster it came down; the eye stalks trail a move and nod on landing; the claws sway as it walks, squeeze after a grab and clack twice after a place; a crab standing still a while fidgets (looks about, clacks its empty claws, shifts its weight).
