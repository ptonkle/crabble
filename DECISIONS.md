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
