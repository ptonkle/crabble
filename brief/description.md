# The person's description and answers, word for word

Given 2026-10-19.

## Description

```text
New game idea: this game is called Crabble and the idea is the players each control a crab and the goal is to more or less play speed scrabble.

There are no turns, players can place tiles as fast as they can. Tiles are in piles around the board which are lightly physics-simulated, i.e. you can pull a piece out from the bottom and the top ones might tumble down, etc., the piles are messy, not all pieces are necessarily right side up, etc.. Tiles should be about as big as the crab's abdomen. you can press right click to grab a tile with your right claw and left click for left claw. When placing the tiles, you should see a ghost version of the tile indication the position it wil lbe placed if you click, as well as a dynamic popup near the top that indicates the word(s) you will be forming as well as making it clear if that completes the word. Tiles cannot be moved/removed after they are placed. Distribution of letters is the same as in normal scrabble. But there will need to be more tiles probably to fill the whole game time. You can also drop the tiles without putting them on the board with the mouse buttons too.
Let's say no blank tiles for now. 

Use the same graphical style from ape-race and Honey Hole, of 3d geometric shapes. Title card things should be "crabble" spelled out with scrabble tiles. The map should be just on a sandy seafloor udnerwater. the boar dshould be totally flat and horizontal but there can be small mounds and such around the outside wher you find the scrabble tiles. Tiles mostly exist in piles but some can be found alone. 

Online will be important to determine who picked up a given tile and who placed a given tile first. Make sure these states stay in sync.

View: Ths view is 3rd person from behind the crab. Your mouse is snapped to the center and can be used to look around. the piece you pick up is the one your crosshair is looking at.

There is also a picture in picture in the bottom left of the overhead view of the board. This hsould show the current state of the board clearly, with icons of each crab's position and what tiles have been placed. The purpose of this is to be the clear place to look to see the existing board, double/triple word/letter scores, etc. The crab icons can just be circles that are slkightly transparent so that they don't block the information. The color outline to indicate word ownership in the view should surround the whole word, not letter by letter. There likely will be some difficulty in replaying who owns words since there can be shared letters and such but do your best and we can make it more precise later.

Copy the general ideas for menu layout from Ape Race, where there is a main menu, and you can laucnh into either local game or online game, do all the of the online syncing up the same. This is proven to be good. 

STyle should be lightly underwater and cartoony. Do not fall back to the Builder framework style.

Each player starts the game at a corner of the board with no tiles. games last 4 minutes. Scoring rules are the same as scrabble. Tally up points only at the same a la Honey Hole, do not show scores while in the game. the player does not score points until the end of the game, and who owns a word is only determined at the end of the game by the player who added the last letter to that word. Since tiles are placed one at a time, there is not validation that it forms a word with any individual placement. However when you add a tile to a word you claim "ownership" of that word. If that word is not a real word by the end of the game, you LOSE as many points as the tiles are worth instead of gaining that many. Ownership of the word should be indicated by a colored border around each of the tiles in that word both in the 3d model and the overhead view. Since a single letter can be owned by multiple players, use alternating colors around the edges. The text color of the tiles in both should also turn green when the word is complete. The letter at the middle of the board in the beginning is random

Controls: 
- WASD to move around
- mouse for camera
- right click to pick up in right claw, left click to pick up in left claw
- space to jump
- hold tab to bring up the overheard view to fill 90% of the screen.

Jumping is a bit floaty, and it would be nice if the crabs kinda dynamically were able to place their legs to crawl things liek the pile of tiles with the legs being able to dynamically place themselves. But that animation can be left as a todo for later.

Each player is assigned a color in the lobby, red, yellow, blue, or purple.

Develop a basic ai to play the game too, mostly just so the ai can be used in automated testing and balancing. Be sure that the ai needs to spend some time thinking about what letters to grab or where to put them, they do not know those things instantly.

We need some way to make sure players don't just camp with the letter 6 to steal other players' words. I still need to figure out the details of this, ideas may be that if you don't place a letter for 30 seconds you suffer a points penalty or something like that. Suggest ideas. 

Early design materials: crab models, tile models, board model, map model, crab animations, birds-eye view, how to tell which player owns the word and if it's completed, other ideas.
```

## Answers to the first questions (2026-10-19)

The agent's questions, shortened, then the person's answer word for word.

1. Board size, 15×15 to start? — "yes start with that size"
2. Every tile must touch an existing tile? — "yes"
3. Placing reach? — "within a short distance, probably not more than a tile's distance away"
4. Empty claw grabs the aimed tile; full claw places at the ghost when aimed at the board, else drops; so two tiles at most? — "yes"
5. Face-down tiles hidden until picked up? — "The letter on the tile renders, it's just you can't see it from above so in that way they are hidden. there is no state change about them when you pick them up."
6. A word is only the longest unbroken run of 2+ letters; CAT→CATS hands it to whoever added the S? — "yes"
7. A premium square counts for every word through it? — "yes"
8. An invalid word loses its full score, premiums included? — "yes"
9. ENABLE word list? — "Yes, but really we can't use the tournament list? what is with that?"
10. Start with 3 standard sets (300 tiles), tune with AI? — "sounds good"
11. Local game is you against 1–3 AI crabs, no split-screen? — "correct"
12. Crab contact? — "push each other a bit yes but no stealing or knocking out yet. That may come later though"
13. Early design materials into a design journal? — "yes into the design journal"

Anti-camping ideas offered and the person's answers:

1. Extending only earns the difference — "I don't think works because then if a single player spelled A into App into Apple, do they score for all the words? I like the stealing it's abig part of the game"
2. A held tile slips out after ~20 s — "there would be nothing to stop them from just dropping and picking it up constantly though"
3. Tiles left in claws at the end count against you — "nah that's weird that if you're at the end of the game you need to remember to drop everyting before the game ends"
4. Points penalty after 30 s without placing — "let's keep this idea, but think of more"
5. Words lock after ~30 s untouched — "no"

## Anti-camping, settled (2026-10-19)

The person's answer, word for word:

```text
30 seconds without placing a tile is the penalty. -2 points every second you wait past 30 second ... Give the player live "-2" number popups to make this clear. Also if you're within 10 seconds of starting to accrue penalty, put a countdown in the corner of the screen that says something like "place a tile!"
```
