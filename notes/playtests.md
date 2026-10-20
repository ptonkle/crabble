# Playtests

Headless games of AI crabs, for balancing. Run with

    cargo run --release -p tool_core --example playtest -- GAMES [FIRST_SEED]

Each game is 4 AI crabs (the one medium level, `ai` in `data/tuning.json`), 4:00 on the shipped map, scored with `score::tally` on the final board (the numbers players see in the tally). `PLAYTEST_TRACE=<seat>` prints one crab's steps; `PLAYTEST_GIVEUPS=1` lists what each crab gave up on.

## 2026-10-19, 16 games (seeds 1-16), first AI numbers

Columns: grabs; tiles placed; placements whose every word was real / that made a fake word; steals (placements that grew another crab's real word); drops; goals given up (a tile it could not reach or aim at, a pile or square it came no closer to); the longest stretch without placing; idle penalty paid (points); real / fake words owned at the end; the tally's total.

seed | crab | grabs | placed | real/fake placements | steals | drops | gave up | longest idle s | idle paid | real/fake words | total
---|---|---|---|---|---|---|---|---|---|---|---
1 | 0 (won) | 11 | 11 | 10/1 | 7 | 0 | 4 | 33 | 4 | 5/1 | 17
1 | 1 | 12 | 12 | 9/3 | 4 | 0 | 3 | 33 | 6 | 4/3 | 5
1 | 2 | 10 | 9 | 7/2 | 6 | 0 | 10 | 52 | 56 | 5/1 | -12
1 | 3 | 10 | 9 | 7/2 | 3 | 0 | 6 | 40 | 22 | 3/1 | -14
2 | 0 (won) | 11 | 10 | 8/2 | 4 | 0 | 3 | 35 | 14 | 4/1 | 26
2 | 1 | 14 | 11 | 9/2 | 6 | 3 | 3 | 35 | 20 | 5/2 | 12
2 | 2 | 15 | 11 | 10/1 | 7 | 2 | 5 | 40 | 22 | 3/0 | 6
2 | 3 | 11 | 10 | 7/3 | 6 | 0 | 5 | 38 | 26 | 5/2 | 4
3 | 0 | 8 | 8 | 8/0 | 8 | 0 | 8 | 56 | 66 | 8/0 | 39
3 | 1 | 12 | 10 | 10/0 | 3 | 0 | 3 | 41 | 24 | 3/0 | -4
3 | 2 | 9 | 8 | 7/1 | 3 | 0 | 5 | 41 | 48 | 2/1 | -36
3 | 3 (won) | 13 | 12 | 11/1 | 6 | 0 | 2 | 36 | 10 | 7/1 | 49
4 | 0 | 8 | 8 | 7/1 | 2 | 0 | 7 | 37 | 18 | 3/1 | 0
4 | 1 | 10 | 9 | 9/0 | 5 | 0 | 6 | 42 | 46 | 6/0 | 16
4 | 2 (won) | 12 | 12 | 11/1 | 10 | 0 | 2 | 33 | 6 | 6/1 | 49
4 | 3 | 11 | 11 | 9/2 | 5 | 0 | 4 | 34 | 8 | 5/1 | 34
5 | 0 | 10 | 8 | 6/2 | 5 | 0 | 6 | 38 | 38 | 4/1 | -7
5 | 1 | 12 | 11 | 10/1 | 6 | 1 | 4 | 39 | 16 | 3/1 | 9
5 | 2 (won) | 11 | 10 | 10/0 | 6 | 0 | 4 | 35 | 8 | 7/0 | 77
5 | 3 | 13 | 13 | 13/0 | 4 | 0 | 2 | 38 | 16 | 8/0 | 25
6 | 0 | 7 | 7 | 5/2 | 3 | 0 | 10 | 43 | 52 | 5/2 | -18
6 | 1 (won) | 15 | 11 | 9/2 | 3 | 2 | 3 | 33 | 6 | 5/2 | 6
6 | 2 | 11 | 9 | 9/0 | 6 | 0 | 7 | 36 | 24 | 4/0 | 0
6 | 3 | 8 | 8 | 7/1 | 5 | 0 | 8 | 36 | 22 | 3/0 | -2
7 | 0 | 10 | 9 | 7/2 | 6 | 0 | 6 | 34 | 8 | 4/1 | 20
7 | 1 | 11 | 9 | 9/0 | 4 | 1 | 4 | 47 | 50 | 6/0 | -1
7 | 2 | 10 | 10 | 9/1 | 4 | 0 | 7 | 38 | 20 | 3/1 | -1
7 | 3 (won) | 11 | 11 | 9/2 | 6 | 0 | 2 | 34 | 10 | 7/1 | 49
8 | 0 | 10 | 10 | 7/3 | 5 | 0 | 5 | 29 | 0 | 6/2 | 19
8 | 1 (won) | 12 | 8 | 7/1 | 6 | 3 | 6 | 37 | 24 | 6/1 | 44
8 | 2 | 11 | 8 | 6/2 | 4 | 2 | 10 | 41 | 36 | 2/2 | -23
8 | 3 | 9 | 9 | 5/4 | 1 | 0 | 5 | 41 | 48 | 2/2 | -51
9 | 0 | 8 | 8 | 6/2 | 5 | 0 | 9 | 35 | 18 | 5/1 | 13
9 | 1 | 9 | 9 | 7/2 | 2 | 0 | 5 | 36 | 18 | 4/2 | -22
9 | 2 (won) | 14 | 13 | 12/1 | 5 | 0 | 2 | 35 | 8 | 5/1 | 46
9 | 3 | 12 | 11 | 9/2 | 8 | 1 | 5 | 35 | 20 | 6/1 | -2
10 | 0 | 11 | 11 | 10/1 | 4 | 0 | 6 | 36 | 20 | 4/0 | 17
10 | 1 (won) | 11 | 9 | 7/2 | 4 | 1 | 2 | 38 | 24 | 6/1 | 39
10 | 2 | 10 | 9 | 8/1 | 4 | 0 | 7 | 41 | 34 | 4/0 | 3
10 | 3 | 8 | 7 | 7/0 | 4 | 0 | 9 | 41 | 56 | 4/0 | 12
11 | 0 | 7 | 7 | 4/3 | 3 | 0 | 8 | 44 | 72 | 1/1 | -75
11 | 1 (won) | 11 | 11 | 11/0 | 6 | 0 | 2 | 34 | 6 | 5/0 | 43
11 | 2 | 11 | 11 | 11/0 | 3 | 0 | 6 | 43 | 54 | 6/0 | 15
11 | 3 | 10 | 10 | 10/0 | 5 | 0 | 2 | 39 | 32 | 7/0 | 28
12 | 0 (won) | 13 | 12 | 12/0 | 7 | 0 | 5 | 31 | 2 | 7/0 | 58
12 | 1 | 9 | 8 | 7/1 | 6 | 0 | 6 | 57 | 66 | 4/1 | -57
12 | 2 | 9 | 9 | 8/1 | 4 | 0 | 7 | 43 | 34 | 5/1 | 5
12 | 3 | 12 | 10 | 8/2 | 6 | 0 | 6 | 37 | 30 | 2/2 | -39
13 | 0 | 10 | 9 | 7/2 | 6 | 0 | 5 | 37 | 30 | 2/1 | -31
13 | 1 | 9 | 9 | 7/2 | 3 | 0 | 5 | 36 | 26 | 4/2 | 3
13 | 2 (won) | 9 | 9 | 9/0 | 6 | 0 | 7 | 34 | 16 | 5/0 | 30
13 | 3 | 10 | 10 | 10/0 | 5 | 0 | 11 | 78 | 94 | 7/0 | -5
14 | 0 | 11 | 8 | 6/2 | 5 | 3 | 6 | 36 | 30 | 5/2 | -33
14 | 1 | 10 | 8 | 6/2 | 4 | 0 | 9 | 57 | 58 | 3/2 | -55
14 | 2 (won) | 12 | 11 | 10/1 | 6 | 1 | 6 | 38 | 20 | 4/1 | 14
14 | 3 | 14 | 10 | 9/1 | 4 | 4 | 5 | 35 | 22 | 5/0 | 5
15 | 0 | 11 | 9 | 9/0 | 3 | 0 | 3 | 37 | 26 | 4/0 | 5
15 | 1 (won) | 13 | 13 | 13/0 | 5 | 0 | 1 | 34 | 8 | 7/0 | 20
15 | 2 | 11 | 9 | 8/1 | 6 | 0 | 9 | 43 | 38 | 6/1 | 4
15 | 3 | 11 | 9 | 7/2 | 3 | 2 | 6 | 46 | 50 | 6/2 | -18
16 | 0 (won) | 10 | 9 | 8/1 | 5 | 0 | 9 | 39 | 38 | 2/1 | -24
16 | 1 | 9 | 9 | 7/2 | 4 | 0 | 9 | 51 | 56 | 5/1 | -33
16 | 2 | 6 | 6 | 5/1 | 5 | 0 | 16 | 99 | 154 | 5/0 | -126
16 | 3 | 13 | 8 | 5/3 | 2 | 3 | 3 | 41 | 34 | 2/2 | -35

seed | on board | loose at end | held at end | words on board | longest real word
---|---|---|---|---|---
1 | 42 | 250 | 2 | 23 | HICKS
2 | 43 | 247 | 4 | 22 | ALANDS
3 | 39 | 251 | 4 | 22 | BATHES
4 | 41 | 252 | 1 | 23 | GROUT
5 | 43 | 248 | 3 | 24 | ESTOPS
6 | 36 | 254 | 4 | 21 | ABETS
7 | 40 | 252 | 2 | 23 | MOPED
8 | 36 | 256 | 2 | 23 | WAITS
9 | 42 | 251 | 1 | 25 | RAGAS
10 | 37 | 254 | 3 | 19 | THENS
11 | 40 | 254 | 0 | 20 | MORELS
12 | 40 | 250 | 4 | 22 | FATED
13 | 38 | 255 | 1 | 21 | THEMES
14 | 38 | 254 | 2 | 22 | METAL
15 | 41 | 249 | 4 | 26 | ADORED
16 | 33 | 258 | 3 | 18 | LOWSE

Means per crab per game:
- grabs 10.7, tiles placed 9.6 (real 8.3, fake 1.3), steals 4.8, drops 0.5, gave up 5.7
- words owned at the end: real 4.6, fake 0.9; idle penalty paid 30.8 (crabs that paid any: 98%)
- final score 2.2 (lowest -126, highest 77); longest idle 40.5 s
Means per game: 39.3 tiles on the board (the centre one counted), 252.2 still loose, 2.5 held, 22.1 words

### What it says

- **294 tiles is far too many for 4:00.** Four medium AI crabs put about 39 tiles on the board in a game (33-43, the centre one counted); about 252 stay loose and 2-3 are in claws at the end, so 13% of the tiles are used. The limit is the walk, not the supply: the piles are 2-4 m from the board's middle, about 6-10 s each way at 394 mm/s, so a trip for one or two tiles takes about 20-25 s with the thinking. Even one standard set (98 tiles) would leave over half on the sand. The tile count mostly decides how many letters are in view at a pile, so fewer tiles means less choice at each pile rather than running out. The person's call (SECOND-ORDER R4.11).
- **The idle penalty takes a large share.** Nearly every crab pays some (about 31 points a game on average, about what its real words earn); the longest wait without placing averages 40 s. With trips this long, 30 s of grace is tight for a crab that thinks before it chooses. The AI hurries from 20 s idle (places whatever it holds, fake or not), which is where most of its fake words come from (1.3 a game).
- **Steals are common.** Half of all placements grow another crab's real word: with tiles going on one at a time round one cluster in the middle, nearly every real placement extends a word another crab claimed last. Winning totals are 6 to 77; totals near 0 are common because the penalty and fake words cancel the words.
- **Words stay short.** The longest real words are 5 or 6 letters (HICKS, ALANDS, BATHES, ESTOPS).
- **Give-ups** (about 6 a crab a game) are mostly tiles: one inside a messy pile behind others, which the crosshair cannot pick out from where the crab can stand, or a pile it walks into and cannot climb out of (it then wanders a random way for 1.5 s).
