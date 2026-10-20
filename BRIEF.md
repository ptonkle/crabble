# Brief

Stage 1. The person's own words come first; the rest is written by the agent from them and read by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The description, word for word

Given 2026-10-19 (also kept as `brief/description.md`). Later requests are numbered in `TRIAGE.md`.

> New game idea: this game is called Crabble and the idea is the players each control a crab and the goal is to more or less play speed scrabble.
>
> There are no turns, players can place tiles as fast as they can. Tiles are in piles around the board which are lightly physics-simulated, i.e. you can pull a piece out from the bottom and the top ones might tumble down, etc., the piles are messy, not all pieces are necessarily right side up, etc.. Tiles should be about as big as the crab's abdomen. you can press right click to grab a tile with your right claw and left click for left claw. When placing the tiles, you should see a ghost version of the tile indication the position it wil lbe placed if you click, as well as a dynamic popup near the top that indicates the word(s) you will be forming as well as making it clear if that completes the word. Tiles cannot be moved/removed after they are placed. Distribution of letters is the same as in normal scrabble. But there will need to be more tiles probably to fill the whole game time. You can also drop the tiles without putting them on the board with the mouse buttons too.
> Let's say no blank tiles for now. 
>
> Use the same graphical style from ape-race and Honey Hole, of 3d geometric shapes. Title card things should be "crabble" spelled out with scrabble tiles. The map should be just on a sandy seafloor udnerwater. the boar dshould be totally flat and horizontal but there can be small mounds and such around the outside wher you find the scrabble tiles. Tiles mostly exist in piles but some can be found alone. 
>
> Online will be important to determine who picked up a given tile and who placed a given tile first. Make sure these states stay in sync.
>
> View: Ths view is 3rd person from behind the crab. Your mouse is snapped to the center and can be used to look around. the piece you pick up is the one your crosshair is looking at.
>
> There is also a picture in picture in the bottom left of the overhead view of the board. This hsould show the current state of the board clearly, with icons of each crab's position and what tiles have been placed. The purpose of this is to be the clear place to look to see the existing board, double/triple word/letter scores, etc. The crab icons can just be circles that are slkightly transparent so that they don't block the information. The color outline to indicate word ownership in the view should surround the whole word, not letter by letter. There likely will be some difficulty in replaying who owns words since there can be shared letters and such but do your best and we can make it more precise later.
>
> Copy the general ideas for menu layout from Ape Race, where there is a main menu, and you can laucnh into either local game or online game, do all the of the online syncing up the same. This is proven to be good. 
>
> STyle should be lightly underwater and cartoony. Do not fall back to the Builder framework style.
>
> Each player starts the game at a corner of the board with no tiles. games last 4 minutes. Scoring rules are the same as scrabble. Tally up points only at the same a la Honey Hole, do not show scores while in the game. the player does not score points until the end of the game, and who owns a word is only determined at the end of the game by the player who added the last letter to that word. Since tiles are placed one at a time, there is not validation that it forms a word with any individual placement. However when you add a tile to a word you claim "ownership" of that word. If that word is not a real word by the end of the game, you LOSE as many points as the tiles are worth instead of gaining that many. Ownership of the word should be indicated by a colored border around each of the tiles in that word both in the 3d model and the overhead view. Since a single letter can be owned by multiple players, use alternating colors around the edges. The text color of the tiles in both should also turn green when the word is complete. The letter at the middle of the board in the beginning is random
>
> Controls: 
> - WASD to move around
> - mouse for camera
> - right click to pick up in right claw, left click to pick up in left claw
> - space to jump
> - hold tab to bring up the overheard view to fill 90% of the screen.
>
> Jumping is a bit floaty, and it would be nice if the crabs kinda dynamically were able to place their legs to crawl things liek the pile of tiles with the legs being able to dynamically place themselves. But that animation can be left as a todo for later.
>
> Each player is assigned a color in the lobby, red, yellow, blue, or purple.
>
> Develop a basic ai to play the game too, mostly just so the ai can be used in automated testing and balancing. Be sure that the ai needs to spend some time thinking about what letters to grab or where to put them, they do not know those things instantly.
>
> We need some way to make sure players don't just camp with the letter 6 to steal other players' words. I still need to figure out the details of this, ideas may be that if you don't place a letter for 30 seconds you suffer a points penalty or something like that. Suggest ideas. 
>
> Early design materials: crab models, tile models, board model, map model, crab animations, birds-eye view, how to tell which player owns the word and if it's completed, other ideas.

The answers to the first questions, 2026-10-19 (the questions shortened, the answers word for word):

> The agent's questions, shortened, then the person's answer word for word.
>
> 1. Board size, 15×15 to start? — "yes start with that size"
> 2. Every tile must touch an existing tile? — "yes"
> 3. Placing reach? — "within a short distance, probably not more than a tile's distance away"
> 4. Empty claw grabs the aimed tile; full claw places at the ghost when aimed at the board, else drops; so two tiles at most? — "yes"
> 5. Face-down tiles hidden until picked up? — "The letter on the tile renders, it's just you can't see it from above so in that way they are hidden. there is no state change about them when you pick them up."
> 6. A word is only the longest unbroken run of 2+ letters; CAT→CATS hands it to whoever added the S? — "yes"
> 7. A premium square counts for every word through it? — "yes"
> 8. An invalid word loses its full score, premiums included? — "yes"
> 9. ENABLE word list? — "Yes, but really we can't use the tournament list? what is with that?"
> 10. Start with 3 standard sets (300 tiles), tune with AI? — "sounds good"
> 11. Local game is you against 1–3 AI crabs, no split-screen? — "correct"
> 12. Crab contact? — "push each other a bit yes but no stealing or knocking out yet. That may come later though"
> 13. Early design materials into a design journal? — "yes into the design journal"
>
> Anti-camping ideas offered and the person's answers:
>
> 1. Extending only earns the difference — "I don't think works because then if a single player spelled A into App into Apple, do they score for all the words? I like the stealing it's abig part of the game"
> 2. A held tile slips out after ~20 s — "there would be nothing to stop them from just dropping and picking it up constantly though"
> 3. Tiles left in claws at the end count against you — "nah that's weird that if you're at the end of the game you need to remember to drop everyting before the game ends"
> 4. Points penalty after 30 s without placing — "let's keep this idea, but think of more"
> 5. Words lock after ~30 s untouched — "no"

Later the same day: "rely on ape race more than honey hole, it is a more complete game."

## The tool

**Crabble** is speed Scrabble played by crabs, in 3D in the browser. Two to four players each steer a crab around a Scrabble board on the seafloor, pick letter tiles out of messy piles with their claws and put them on the board as fast as they can. There are no turns. A game lasts 4 minutes; scores stay hidden until a tally at the end. Played with a mouse and keyboard on a desktop browser, locally against 1 to 3 AI crabs, or online with up to four players.

### Look

- 3D low-poly geometric shapes, as in Ape Race and Honey Hole. Lightly underwater and cartoony: a sandy seafloor, soft blue-green water, light caustics.
- The board is perfectly flat and horizontal. Small mounds lie around the outside, where the tiles are found: mostly in messy piles, some alone.
- Models are hand-written data files under `data/models/` (Honey Hole's format), not meshes built in Rust code. Limbs and bodies are tapered many-sided shapes, never boxes by default; a real creature is checked against its real anatomy from above, the side and 3/4.
- The title card spells CRABBLE in Scrabble tiles.
- Menus and HUD have their own underwater, cartoony look (see Style), never Builder's default style pack.

### Board

- 15 × 15 squares with the standard Scrabble premium layout: double and triple letter, double and triple word.
- At the start the centre square holds one random letter tile. Every tile placed after it must touch a tile already on the board (share a side).
- Placed tiles never move and are never taken off.

### Tiles

- Wooden Scrabble tiles, each showing its letter and point value, about as big as a crab's abdomen.
- The letters and values follow standard English Scrabble, with no blank tiles. A game starts with three standard sets (3 × 98 = 294 tiles, without the blanks); the number is tuned later with the AI.
- Tiles lie in piles around the board, lightly physics-simulated: pull one out from the bottom and the ones above may tumble. Piles are messy, and some tiles lie face down.
- A face-down tile has no hidden state: its letter is drawn on its face, which just can't be seen from above. Nothing about a tile changes when it is picked up.

### Crabs and controls

- Each player is a crab, in the colour of their lobby seat: red, yellow, blue or purple.
- Each crab starts at a corner of the board with empty claws.
- Crabs push each other a little when they bump. There is no stealing a held tile and no knocking out (maybe later).
- Jumping is a bit floaty.

| Key | Does |
| --- | --- |
| W A S D | move |
| Mouse | look (the pointer is locked to the centre; a crosshair marks it) |
| Left click | left claw: grab, place or drop |
| Right click | right claw: grab, place or drop |
| Space | jump |
| Tab (hold) | the overhead view fills 90% of the screen |

**A claw's click.** With the claw empty, a click grabs the tile under the crosshair, if one is in reach. With a tile in it, a click places the tile at the ghost when the crosshair is on the board at a legal square, and otherwise drops it on the floor. A crab holds at most two tiles, one per claw.

**Reach.** A tile can be grabbed or placed only within a short distance of the crab: no more than about one tile's width away.

### View

- Third person, from behind the crab. The mouse turns the camera; the crosshair picks the tile to grab.
- **Ghost tile:** while a claw holds a tile and the crosshair is on a legal square, a see-through copy of the tile shows where a click will put it.
- **Word preview:** a small popup near the top of the screen shows the word or words that placement would form, and makes it clear whether each would be a complete word.
- **Overhead view:** a picture-in-picture in the bottom-left corner shows the board from above: every placed tile, the premium squares, and each crab as a slightly see-through circle in its colour, so the circles never hide the board. Holding Tab enlarges it to fill 90% of the screen.

### How to play

A "How to play" entry on the main menu (asked 2026-10-19): "DO NOT OVEREXPLAIN IN THIS. FOR EXAMPLE YOU DO NOT NEED TO NAME THE EXACT POINT LOSS OR TIMING ON WHEN YOU START LOSING POINTS. JUST SOMETHING LIKE "FAILING TO PLACE A FOR A WHILE TILE WILL RESULT IN A PENALTY" OR SOMETHING." A handful of short lines: the goal, grabbing and placing with each claw, the last letter added claims a word, fake words cost points, place tiles regularly or be penalised, and the controls. No numbers or exact rules. No player-visible text says "Scrabble"; it says "tiles" or "word tiles".

### Words and ownership

- A **word** is a maximal unbroken run of two or more letters along a row or a column.
- The word's **owner** is the player who last added a letter to it. Extending CAT to CATS hands the word to whoever placed the S. Ownership is settled at the end of the game.
- Each placement claims the words it touches. There is no check when a tile is placed that it makes a word.
- Each player sees outlines round their own words only, in their colour, at the board in 3D and in the overhead view (Tab too): solid for a real word, dashed for one that is not. Two of your words that cross share one outline. Other players' words get no owner marks (the person, F8, 2026-10-20; `DECISIONS.md`).
- A word that is in the word list turns its letters' text green, on the board and in the overhead view.
- The word list is ENABLE (public domain); see `DECISIONS.md` on why not the tournament lists.

### Scoring

- Scrabble's scoring: a word scores the sum of its letters' values, with double and triple letter squares applied to their letters and double and triple word squares multiplying the word.
- A premium square counts for every word through it, every time.
- No scores are shown during the game. At the end, points are tallied up one by one, as in Honey Hole's postgame.
- A real word scores its points for its owner. A word that is not real at the end costs its owner its full score (premiums included) instead.

### Anti-camping

Settled 2026-10-19, word for word:

> 30 seconds without placing a tile is the penalty. -2 points every second you wait past 30 second ... Give the player live "-2" number popups to make this clear. Also if you're within 10 seconds of starting to accrue penalty, put a countdown in the corner of the screen that says something like "place a tile!"

- Each player's idle timer starts at the start of the game and resets each time they place a tile.
- From 20 s idle, a countdown appears in a corner of the screen: "Place a tile!" and the seconds left.
- Past 30 s idle, the player loses 2 points a second, and a "-2" popup shows each second. Only the idle player sees their own popups; scores stay hidden otherwise.
- The numbers (30 s, the 10 s warning, 2 points a second) are tuning data, not code.

### Sound

Asked 2026-10-19: "oh also do some basic sounds for picking up tiles, placing tiles, completing words, etc." Basic synthesised sounds: grabbing a tile, placing it (a wooden clack), completing a real word, dropping a tile, jumping and landing, the idle countdown and its "-2", the start countdown and the end, and the tally's points.

### Online

- The menus follow Ape Race: a main menu, then a local game or an online game, and Ape Race's lobby with its conveniences (room code, copy code, copy link, joining from an invite link).
- The online syncing follows Ape Race's approach and transport (WebRTC over pinned Nostr relays). Every browser must agree on who picked up a given tile and who placed a given tile first; the tile states must stay in sync.

### AI crabs

- A basic AI plays the game, mainly for automated testing and balancing. It plays by the same rules and inputs as a person.
- The AI must take time to think about which letters to grab and where to put them; it does not know either instantly.
- One AI level only, medium, and no difficulty picker. The person, 2026-10-19: "Just do a single medium level of ai for now, don't want to overdesign it if it turns out the game sucks". The AI stays simple, but its thinking time stays visible.

### Later

- Crab legs that place themselves dynamically (IK), so a crab crawls over a pile of tiles (`TRIAGE.md`, future ideas).
- Stealing held tiles and knocking crabs out.
- Blank tiles.

## Done means

The person checks these before a first public release. Each is checkable by playing or by a named test.

- A local game against 1–3 AI crabs and an online game of up to four players both run a full 4-minute game to the end-of-game tally, in Chromium, Firefox and WebKit.
- The board is 15 × 15 with the standard premium layout (a core test checks every premium square), and starts with one random letter in the centre.
- The tile set is three standard sets without blanks, 294 tiles, with standard values (core test).
- A tile can be placed only touching a tile already on the board, within reach of the crab; a placed tile never moves (core tests).
- An empty claw's click grabs the tile under the crosshair in reach; a full claw's click places at the ghost or drops (core tests).
- Words, owners and scores follow the rules above, worked out by hand for a set of boards: extension hands over ownership, premiums count for every word through them, invalid words subtract their full score (core tests).
- The idle penalty: from 20 s idle the corner countdown shows; past 30 s each second costs 2 points with a "-2" popup; a placement resets it (core tests).
- Every crab, tile, board and seafloor model is loaded from its own file under `data/models/`.
- Outlines round your own words (solid when real, dashed when not) and green text for real words show both in 3D and in the overhead view.
- Online, two browsers grabbing the same tile at once end with one owner on every screen; two placing on the same square end with one tile (online tests).
- AI crabs play a whole game headless in the core for balancing.
- `./scripts/check.sh` passes, and the online check passes.

## Decisions that stay with the person

- The anti-camping numbers.
- Any balance number: game length, tile count, reach, penalty sizes, jump height.
- The look of every model and screen, approved through the design journal.
- The word list.
- Whether and where to publish.

## Accessibility

- Players are shown by colour and by name; in the overhead view the crab circles may also take a letter or shape (to settle in the journal).
- A real word is shown by green text and also by its outline style, never by colour alone (to settle in the journal).
- Mouse and keyboard only. No gamepad or touch at first.

## Style

Not the template's `default` look. Chosen by the agent from the person's direction ("lightly underwater and cartoony"; SECOND-ORDER R1.6), to be approved in the journal:

- **World:** low-poly 3D in flat colours, sandy seafloor, soft blue-green water fog, light caustic ripples on the sand, a few rocks, shells and weeds around the mounds.
- **Menus and HUD:** "tide pool". Deep sea-blue panels with rounded, bubble-like corners and a pale aqua rim; sea-glass aqua buttons shaped like pills; coral as the second accent; foam white text in a rounded heavy face; a few slow bubbles rising behind the menus. Green is kept for real words and is not used as a UI accent. No cream, no gold, no ink outlines.

## Words

Terms the person uses, to go into `GLOSSARY.md` before any string uses them: crab, claw (left, right), tile, pile, mound, board, premium square (double/triple letter/word), word, owner, ghost tile, word preview ("dynamic popup"), overhead view ("picture in picture", "bird's-eye view"), crosshair, lobby, local game, online game, tally.

## Autonomy

The plan is approved in advance for the early work the person asked for: the repository setup, the tooling and menu port, the model format and the early design materials in the design journal. Where this brief is silent, the agent decides, records the decision in `SECOND-ORDER.md` and continues; a big design question that only the person can settle is asked, not guessed. The agent may not loosen a Builder requirement or a boundary test, and may not add a server, an account, analytics or any off-origin request except the pinned online relays. Since 2026-10-19 the agent may create the public repository ptonkle/crabble and publish it and its GitHub Pages site with `./scripts/publish.sh` once `./scripts/check.sh` passes (`DECISIONS.md`).

## Journal

`default`

The journal is `../crabble-journal/`, its own git repository, never pushed, published as a private artifact (design-journal-kit's form, `.claude/skills/design-journal/`).

## Allowed core dependencies

`serde`, `serde_json` (reference default). Add others only here, with the reason.
