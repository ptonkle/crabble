# Playtests

Headless games of AI crabs, for balancing. Run with

    cargo run --release -p tool_core --example playtest -- GAMES [FIRST_SEED]

Each game is 4 AI crabs (the one medium level, `ai` in `data/tuning.json`), 4:00 on the shipped map, scored with `score::tally` on the final board (the numbers players see in the tally). `PLAYTEST_TRACE=<seat>` prints one crab's steps; `PLAYTEST_GIVEUPS=1` lists what each crab gave up on. `PLAYTEST_TIME=1` prints each crab's time in each step (by claws held and whether late); `PLAYTEST_BOARD=1` prints the final board.

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

## 2026-10-20, 16 games (seeds 1-16), piles nearer and 40 s grace (F9)

The piles moved in (`tools/piles.py`): twelve piles, three on each side of the board, their middles 0.8-1.0 m off its edge (were 2-4 m from the middle), each on the inner slope of a long ridge, the same three on every side so each corner start is as far from its piles as any other. Still 294 tiles. Idle grace 40 s (was 30 s); the AI still hurries from 20 s (`ai.hurry_ms`, unchanged). Same columns as above.

seed | crab | grabs | placed | real/fake placements | steals | drops | gave up | longest idle s | idle paid | real/fake words | total
---|---|---|---|---|---|---|---|---|---|---|---
1 | 0 | 12 | 12 | 11/1 | 3 | 0 | 10 | 47 | 12 | 7/1 | 36
1 | 1 | 15 | 14 | 12/2 | 9 | 0 | 7 | 33 | 0 | 8/1 | 31
1 | 2 (won) | 14 | 13 | 12/1 | 6 | 0 | 6 | 30 | 0 | 8/1 | 73
1 | 3 | 16 | 15 | 13/2 | 5 | 0 | 5 | 30 | 0 | 7/1 | 62
2 | 0 (won) | 15 | 13 | 11/2 | 5 | 0 | 5 | 34 | 0 | 7/1 | 54
2 | 1 | 11 | 9 | 7/2 | 4 | 1 | 13 | 44 | 6 | 8/1 | 30
2 | 2 | 16 | 15 | 13/2 | 7 | 0 | 7 | 36 | 0 | 6/0 | 46
2 | 3 | 14 | 9 | 7/2 | 5 | 3 | 11 | 31 | 0 | 3/0 | 35
3 | 0 | 15 | 9 | 5/4 | 4 | 5 | 9 | 35 | 0 | 2/4 | -14
3 | 1 | 18 | 12 | 12/0 | 7 | 4 | 7 | 35 | 0 | 7/0 | 54
3 | 2 (won) | 14 | 13 | 12/1 | 8 | 0 | 5 | 31 | 0 | 7/1 | 75
3 | 3 | 15 | 14 | 12/2 | 5 | 1 | 7 | 33 | 0 | 5/1 | 31
4 | 0 | 15 | 12 | 12/0 | 6 | 2 | 7 | 53 | 24 | 5/0 | 53
4 | 1 | 13 | 12 | 12/0 | 5 | 1 | 7 | 35 | 0 | 5/0 | 37
4 | 2 (won) | 17 | 12 | 12/0 | 9 | 4 | 6 | 40 | 0 | 8/0 | 85
4 | 3 | 14 | 12 | 12/0 | 6 | 1 | 5 | 39 | 0 | 6/0 | 69
5 | 0 (won) | 19 | 18 | 17/1 | 7 | 0 | 4 | 35 | 0 | 7/1 | 64
5 | 1 | 14 | 12 | 11/1 | 8 | 0 | 10 | 37 | 0 | 5/1 | 55
5 | 2 | 13 | 11 | 9/2 | 9 | 1 | 9 | 30 | 0 | 5/1 | 45
5 | 3 | 17 | 15 | 15/0 | 6 | 0 | 4 | 33 | 0 | 7/0 | 57
6 | 0 | 16 | 16 | 13/3 | 8 | 0 | 3 | 29 | 0 | 8/3 | 41
6 | 1 (won) | 14 | 14 | 14/0 | 7 | 0 | 6 | 41 | 2 | 7/0 | 51
6 | 2 | 17 | 14 | 12/2 | 5 | 2 | 6 | 32 | 0 | 7/1 | 38
6 | 3 | 10 | 10 | 8/2 | 4 | 0 | 8 | 50 | 18 | 4/2 | -20
7 | 0 | 17 | 13 | 12/1 | 4 | 3 | 4 | 31 | 0 | 6/1 | 36
7 | 1 | 13 | 12 | 11/1 | 3 | 0 | 7 | 27 | 0 | 8/1 | 51
7 | 2 | 11 | 9 | 7/2 | 2 | 2 | 9 | 37 | 0 | 5/1 | 13
7 | 3 (won) | 17 | 13 | 12/1 | 10 | 3 | 4 | 38 | 0 | 9/0 | 103
8 | 0 | 15 | 14 | 13/1 | 7 | 0 | 7 | 34 | 0 | 7/1 | 36
8 | 1 | 17 | 13 | 12/1 | 6 | 3 | 5 | 38 | 0 | 3/1 | 24
8 | 2 | 13 | 12 | 11/1 | 9 | 0 | 6 | 36 | 0 | 7/1 | 54
8 | 3 (won) | 18 | 15 | 13/2 | 8 | 3 | 5 | 30 | 0 | 8/2 | 99
9 | 0 | 12 | 10 | 10/0 | 7 | 0 | 13 | 60 | 38 | 5/0 | -6
9 | 1 (won) | 14 | 13 | 12/1 | 7 | 0 | 6 | 30 | 0 | 8/0 | 72
9 | 2 | 20 | 15 | 13/2 | 4 | 4 | 3 | 29 | 0 | 8/1 | 50
9 | 3 | 11 | 10 | 10/0 | 8 | 1 | 11 | 61 | 42 | 4/0 | -14
10 | 0 | 10 | 10 | 9/1 | 7 | 0 | 11 | 37 | 0 | 5/1 | 35
10 | 1 | 10 | 10 | 10/0 | 7 | 0 | 13 | 46 | 18 | 5/0 | 25
10 | 2 (won) | 15 | 14 | 14/0 | 4 | 0 | 5 | 33 | 0 | 9/0 | 67
10 | 3 | 9 | 9 | 9/0 | 5 | 0 | 14 | 57 | 32 | 4/0 | 2
11 | 0 (won) | 13 | 10 | 8/2 | 7 | 1 | 10 | 35 | 0 | 7/1 | 63
11 | 1 | 12 | 12 | 11/1 | 3 | 0 | 8 | 29 | 0 | 3/1 | 13
11 | 2 | 13 | 11 | 8/3 | 5 | 0 | 11 | 42 | 2 | 5/4 | 12
11 | 3 | 17 | 15 | 12/3 | 5 | 0 | 4 | 32 | 0 | 8/2 | 32
12 | 0 | 13 | 11 | 10/1 | 5 | 0 | 7 | 35 | 0 | 5/1 | 22
12 | 1 | 9 | 9 | 8/1 | 2 | 0 | 10 | 47 | 14 | 3/0 | 0
12 | 2 (won) | 15 | 14 | 12/2 | 10 | 1 | 5 | 34 | 0 | 8/2 | 88
12 | 3 | 10 | 9 | 7/2 | 6 | 0 | 13 | 37 | 0 | 3/2 | 3
13 | 0 | 12 | 12 | 9/3 | 8 | 0 | 5 | 29 | 0 | 5/2 | 36
13 | 1 | 18 | 14 | 12/2 | 6 | 2 | 3 | 31 | 0 | 3/1 | 6
13 | 2 | 13 | 11 | 10/1 | 6 | 2 | 5 | 33 | 0 | 5/1 | 13
13 | 3 (won) | 15 | 14 | 14/0 | 8 | 0 | 6 | 33 | 0 | 10/0 | 89
14 | 0 (won) | 14 | 12 | 11/1 | 7 | 0 | 7 | 38 | 0 | 6/1 | 47
14 | 1 | 16 | 16 | 15/1 | 7 | 0 | 3 | 33 | 0 | 10/1 | 45
14 | 2 | 14 | 14 | 11/3 | 7 | 0 | 5 | 33 | 0 | 4/2 | 19
14 | 3 (won) | 15 | 12 | 11/1 | 8 | 1 | 3 | 37 | 0 | 5/1 | 47
15 | 0 | 16 | 13 | 11/2 | 6 | 2 | 4 | 37 | 0 | 5/2 | 25
15 | 1 | 13 | 13 | 8/5 | 6 | 0 | 9 | 39 | 0 | 2/2 | 1
15 | 2 | 18 | 17 | 14/3 | 10 | 0 | 4 | 27 | 0 | 9/3 | 41
15 | 3 (won) | 13 | 10 | 7/3 | 5 | 2 | 7 | 31 | 0 | 6/2 | 44
16 | 0 | 13 | 13 | 12/1 | 6 | 0 | 9 | 31 | 0 | 4/0 | 35
16 | 1 | 9 | 9 | 9/0 | 5 | 0 | 12 | 39 | 0 | 6/0 | 60
16 | 2 (won) | 15 | 15 | 14/1 | 11 | 0 | 5 | 29 | 0 | 11/0 | 99
16 | 3 | 13 | 12 | 12/0 | 6 | 0 | 9 | 51 | 22 | 5/0 | 14

seed | on board | loose at end | held at end | words on board | longest real word
---|---|---|---|---|---
1 | 55 | 236 | 3 | 34 | FLOP
2 | 47 | 241 | 6 | 26 | DOJOS
3 | 49 | 241 | 4 | 27 | BAITS
4 | 49 | 242 | 3 | 24 | GRIND
5 | 57 | 231 | 6 | 27 | RIFER
6 | 55 | 238 | 1 | 32 | KAES
7 | 48 | 243 | 3 | 31 | SOWED
8 | 55 | 236 | 3 | 30 | JOWED
9 | 49 | 241 | 4 | 26 | NORIAS
10 | 44 | 249 | 1 | 24 | AMUSED
11 | 49 | 239 | 6 | 31 | RIDE
12 | 44 | 247 | 3 | 24 | CARTE
13 | 52 | 239 | 3 | 27 | COPENS
14 | 55 | 235 | 4 | 30 | GRINDS
15 | 54 | 237 | 3 | 31 | TORAH
16 | 50 | 243 | 1 | 26 | SIREE

Means per crab per game:
- grabs 14.1, tiles placed 12.4 (real 11.1, fake 1.4), steals 6.3, drops 0.9, gave up 7.1
- words owned at the end: real 6.1, fake 1.0; idle penalty paid 3.6 (crabs that paid any: 19%)
- final score 40.5 (lowest -20, highest 103); longest idle 36.6 s
Means per game: 50.8 tiles on the board (the centre one counted), 239.9 still loose, 3.4 held, 28.1 words

### Against the first run

| per crab per game | 2026-10-19 | 2026-10-20 | change |
|---|---|---|---|
| tiles placed | 9.6 | 12.4 | +29% |
| real / fake placements | 8.3 / 1.3 | 11.1 / 1.4 | |
| idle penalty paid | 30.8 (98% paid some) | 3.6 (19% paid some) | -88% |
| longest idle | 40.5 s | 36.6 s | |
| final score | 2.2 (-126 to 77) | 40.5 (-20 to 103) | |
| gave up | 5.7 | 7.1 | +1.4 |
| **per game** | | | |
| tiles on the board | 39.3 (13% of 294) | 50.8 (17%) | +29% |
| words on the board | 22.1 | 28.1 | |

- **The nearer piles make the pace; the longer grace takes the penalty away.** The same map with the old 30 s grace (a separate 16-game run, otherwise identical) places exactly as many tiles (12.4 a crab) and pays 16.1 points of idle penalty a crab (78% paid some): the shorter walk accounts for about half of the fall in penalty (30.8 to 16.1), the extra 10 s of grace for the rest (16.1 to 3.6). The AI's play does not change with the grace (it hurries from its own 20 s), so the tile counts are the piles' alone.
- **Scores are now mostly positive.** The penalty no longer cancels the words: the mean final score goes from 2 to 41 and the lowest from -126 to -20. A crab that pays much penalty now is one that got stuck (seed 16 crab 3: 51 s without placing, 22 points).
- **Most tiles still stay loose** (about 240 of 294). The tile count stays 294: the person, 2026-10-20, "don't recalculate tile count, it felt fine".
- **Give-ups rose a little** (5.7 to 7.1 a crab): the piles sit on a ridge's slope and spread along it, so more tiles lean behind others. Worth watching, not a blocker.
## 2026-10-20, 16 games (seeds 1-16), the AI keeps to common words (TRIAGE F11)

The AI now makes only words in its vocabulary (`data/words/common.txt`, 44,505 of ENABLE's 172,820; SECOND-ORDER R12.2-R12.4); ENABLE still judges every word. Same seeds and tuning as the first numbers, so "before" is that run (the playtest now also lists each game's real words outside the vocabulary, the last column).

Uncommon real words on the board at the end:

seed | before | after
---|---|---
1 | REE SI KA SH KOR AHA ANAS HEP NAH | AA
2 | AXE LOTA ETERNE GADI XU NETT AXE TOEA ALANDS EX PE | TA
3 | AWA BEY AGAR AE REX EFT FAX XI TI AI GOA AWEE | AA
4 | GIB WOP TARE LOWE JOEYS MOA DE TOM CRAW REM AE GROUT LI OD | AL LO
5 | AA TAO AAH ESTOPS AR GAMY LI NE OAT AAS JO HORAL AI MIDIS | 
6 | AY NEE ED NE OS TOG HIDER LIER CERE OM JINS YO | TORR JO OD LIRE
7 | PSI NAW MM OP FETAS DAL DOM AAL TAOS UN TA AI | 
8 | AW KAE YAH ED TAV OE TI AI TOD QAT ES | 
9 | GAGE AA WO AE OE PIU SOY SI AA ED RAGAS OE LIB | 
10 | TAE KHIS KOP THENS JOES GLOP LEX RET | AW
11 | ER KAB XI LI ES TERN MORELS DEX RITZ EL | 
12 | GOY ER WOS GID TOR YID DOW AA REF EX | AA
13 | AR GOX OS PADI LO DIS HAP OE HOYS OES | 
14 | PECS AA EN NOMA NU SRI AE | 
15 | AA AIN EN UH BAH ED RIN ED TAE DUI NE NU | 
16 | LOWSE ALIF DAGO GUL DAG HOD TAE LI MUS | FADO

Means, before and after:

measure | before | after
---|---|---
words on the board a game | 22.1 | 19.7
of them real but uncommon | 10.9 | 0.8
real / fake words owned a crab | 4.6 / 0.9 | 3.9 / 1.0
tiles placed a crab (real / fake placements) | 9.6 (8.3 / 1.3) | 8.6 (6.9 / 1.7)
steals a crab | 4.8 | 3.5
drops a crab | 0.5 | 2.2
idle penalty paid a crab | 30.8 | 41.8
final score a crab (lowest, highest) | 2.2 (-126, 77) | -20.3 (-302, 40)
tiles on the board a game | 39.3 | 35.4
longest real words | HICKS, ALANDS, BATHES, ESTOPS | SCANTS, GRATES, ABATE, DIRER

### What it says

- **The odd words are gone.** About half the words on the board were real but uncommon (AA, XU, QAT, KOR, TAE); now under one a game. The few left are made when a hurried crab places what it holds (fake to it) and the word happens to be real, or by a cross word it read off the board.
- **It costs the AI tempo.** With a quarter of ENABLE to work with, fewer letters in view make a word it knows, so it places 10% fewer tiles, drops more letters it cannot use (2.2 against 0.5) and waits longer, paying about 11 more points of idle penalty; its hurried placements are fake a little more often (1.7 against 1.3). Final scores fall by about 22 a crab. A balance call for later (the idle penalty and the AI's hurry at 20 s, R4.*), not retuned here.

## 2026-10-20, the scenery (F14) and the AI's paths

16 games (seeds 1-16) on the map with the F14 scenery and on the same map without it (main merged: the tile physics and AI of F7 and F8 are in both, so these are not comparable with the runs above): every number the same, 10.9 tiles placed a crab, idle penalty paid 3.6, 44.6 tiles on the board a game. The scenery stands past the ridges, where the AI does not walk.

## 2026-10-20, 16 games (seeds 1-16), letter choice that knows the vocabulary (F11 follow-up, R14.*)

After F11 the AI dropped tiles eight times as often (4.0 a crab against 0.5). Its letter choice now knows its vocabulary (SECOND-ORDER R14.1-R14.3): a held letter with no word counts in what a letter in view is worth (a letter that opens a word for it is worth both); with two letters it places first the one after which the other has a word; with both claws full and no word it knows, it places a real word it does not know before it drops (its last resort); in a hurry it places a real word before a fake one; a letter it let go it does not fetch straight back; and a second look whose letter lost its word is a first look again (R14.3). Rules, tuning, thinking times and the tile count are unchanged. "Before" is main with F9 and F11 (`cargo run --release -p tool_core --example playtest -- 16`).

| per crab per game | before (main) | after | change |
|---|---|---|---|
| grabs | 15.7 | 13.6 | |
| tiles placed (real / fake placements) | 10.9 (8.8 / 2.1) | 12.0 (11.1 / 0.9) | +10% |
| drops | 4.0 | 0.8 | -80% |
| gave up | 7.0 | 7.3 | |
| steals | 4.7 | 5.3 | |
| real / fake words owned | 5.0 / 1.4 | 6.2 / 0.8 | |
| idle penalty paid (crabs that paid any) | 3.6 (20%) | 3.3 (11%) | |
| final score (lowest, highest) | 24.5 (-33, 75) | 37.7 (-40, 109) | +13 |
| longest idle | 37.4 s | 36.4 s | |
| **per game** | | | |
| tiles on the board | 44.6 | 49.1 | +10% |
| words on the board (real but uncommon) | 25.4 (1.0) | 27.6 (4.5) | |

The same 16 games with one part taken out at a time:

| | tiles placed | fake placements | drops | uncommon words a game |
|---|---|---|---|---|
| all of it | 12.0 | 0.9 | 0.8 | 4.5 |
| no real-word last resort | 11.2 | 1.0 | 3.9 | 3.4 |
| no opening a word for a held letter, no two-letter look-ahead | 11.6-11.9 | 0.9-1.0 | 0.8 | 5.2-5.3 |

### What it says

- **The drops and the fakes are back to before F11.** Drops 0.8 (0.9 before F11), fake placements 0.9 (1.4 before F11), tiles placed 12.0 (12.4 before F11). The real-word last resort takes the drops away (3.9 without it); a real word before a fake one in a hurry halves the fakes.
- **The cost is odd short words again, though far fewer.** Real but uncommon words on the board go from 1.0 to 4.5 a game (10.9 before F11): mostly two-letter Scrabble words (AA, AE, OE, XI, NA, ER), made when both claws hold letters with no common word or when the crab is hurried. Leaving the two-letter ones out of the last resort brings drops back to 2.4 and leaves the uncommon count where it is (5.2: the hurried placements make them anyway). The person's call (R14.2).
- **Opening a word for a held letter barely moves the numbers.** In most looks with a stuck letter (443 of about 670 in 16 games) no letter in view makes a word it knows at all, so there is nothing to open; where it differs it changes the square more than the letter. Kept: it is what a player would do, and it costs nothing measurable.
- **A loop came to light and is fixed.** With fewer drops, crabs hold a letter longer, and a second look whose letter had lost its word (the board moved on) sent the crab back to the pile to look again, over and over: in a first build one crab went 165 s without placing (250 idle points). On main it never showed (longest idle 64 s).

seed | crab | grabs | placed | real/fake placements | steals | drops | gave up | longest idle s | idle paid | real/fake words | total
---|---|---|---|---|---|---|---|---|---|---|---
1 | 0 | 11 | 11 | 10/1 | 3 | 0 | 13 | 97 | 112 | 7/0 | -40
1 | 1 (won) | 13 | 13 | 13/0 | 4 | 0 | 5 | 33 | 0 | 9/0 | 58
1 | 2 | 15 | 14 | 14/0 | 5 | 0 | 7 | 37 | 0 | 5/0 | 43
1 | 3 | 15 | 14 | 13/1 | 4 | 0 | 6 | 37 | 0 | 8/1 | 48
2 | 0 | 12 | 11 | 10/1 | 2 | 1 | 10 | 39 | 0 | 5/1 | 41
2 | 1 | 10 | 10 | 8/2 | 6 | 0 | 12 | 48 | 16 | 6/2 | 11
2 | 2 | 11 | 10 | 9/1 | 4 | 0 | 8 | 35 | 0 | 5/1 | 33
2 | 3 (won) | 14 | 13 | 13/0 | 5 | 1 | 7 | 35 | 0 | 7/0 | 46
3 | 0 | 10 | 10 | 9/1 | 6 | 0 | 12 | 40 | 0 | 3/1 | 23
3 | 1 (won) | 14 | 13 | 13/0 | 6 | 0 | 6 | 33 | 0 | 6/0 | 70
3 | 2 | 12 | 11 | 10/1 | 5 | 0 | 9 | 38 | 0 | 7/1 | 54
3 | 3 | 15 | 12 | 11/1 | 5 | 1 | 7 | 37 | 0 | 5/1 | 24
4 | 0 | 11 | 9 | 7/2 | 3 | 2 | 11 | 46 | 12 | 4/1 | 9
4 | 1 (won) | 12 | 11 | 11/0 | 4 | 0 | 5 | 34 | 0 | 8/0 | 47
4 | 2 | 18 | 14 | 13/1 | 7 | 3 | 3 | 31 | 0 | 7/1 | 45
4 | 3 | 17 | 11 | 9/2 | 4 | 5 | 6 | 39 | 0 | 6/1 | 41
5 | 0 | 15 | 14 | 14/0 | 3 | 0 | 6 | 31 | 0 | 5/0 | 30
5 | 1 | 13 | 13 | 12/1 | 4 | 0 | 7 | 39 | 0 | 5/1 | 22
5 | 2 | 16 | 14 | 14/0 | 8 | 1 | 5 | 32 | 0 | 8/0 | 52
5 | 3 (won) | 14 | 12 | 12/0 | 10 | 1 | 6 | 35 | 0 | 8/0 | 57
6 | 0 | 15 | 13 | 12/1 | 5 | 2 | 3 | 29 | 0 | 5/1 | 34
6 | 1 | 15 | 9 | 5/4 | 7 | 4 | 7 | 40 | 0 | 4/3 | -4
6 | 2 (won) | 10 | 10 | 9/1 | 6 | 0 | 9 | 37 | 0 | 6/1 | 60
6 | 3 | 15 | 12 | 12/0 | 6 | 2 | 8 | 35 | 0 | 6/0 | 39
7 | 0 (won) | 14 | 12 | 12/0 | 6 | 0 | 7 | 38 | 0 | 9/0 | 109
7 | 1 | 13 | 11 | 9/2 | 4 | 0 | 8 | 31 | 0 | 3/0 | 31
7 | 2 | 15 | 15 | 13/2 | 7 | 0 | 5 | 38 | 0 | 5/2 | 30
7 | 3 | 15 | 12 | 12/0 | 8 | 2 | 5 | 34 | 0 | 7/0 | 48
8 | 0 (won) | 11 | 11 | 10/1 | 5 | 0 | 11 | 30 | 0 | 7/1 | 53
8 | 1 | 10 | 10 | 8/2 | 2 | 0 | 11 | 37 | 0 | 5/2 | 26
8 | 2 | 16 | 14 | 13/1 | 5 | 1 | 5 | 32 | 0 | 5/1 | 24
8 | 3 | 14 | 11 | 9/2 | 3 | 2 | 7 | 36 | 0 | 6/1 | 34
9 | 0 | 15 | 13 | 11/2 | 5 | 2 | 9 | 32 | 0 | 4/2 | 20
9 | 1 (won) | 15 | 14 | 13/1 | 8 | 0 | 5 | 33 | 0 | 7/1 | 49
9 | 2 | 18 | 13 | 11/2 | 10 | 4 | 5 | 33 | 0 | 7/1 | 48
9 | 3 | 15 | 13 | 13/0 | 7 | 1 | 6 | 42 | 4 | 5/0 | 32
10 | 0 | 15 | 14 | 14/0 | 5 | 0 | 6 | 26 | 0 | 4/0 | 20
10 | 1 | 12 | 11 | 9/2 | 8 | 0 | 8 | 36 | 0 | 6/2 | 56
10 | 2 | 14 | 13 | 12/1 | 6 | 0 | 5 | 29 | 0 | 7/1 | 29
10 | 3 (won) | 14 | 13 | 13/0 | 7 | 0 | 8 | 33 | 0 | 9/0 | 74
11 | 0 | 13 | 11 | 11/0 | 5 | 1 | 6 | 31 | 0 | 6/0 | 48
11 | 1 | 12 | 11 | 10/1 | 3 | 0 | 10 | 58 | 36 | 3/1 | -30
11 | 2 (won) | 14 | 12 | 12/0 | 6 | 1 | 5 | 37 | 0 | 10/0 | 86
11 | 3 | 16 | 12 | 11/1 | 3 | 4 | 6 | 31 | 0 | 7/1 | 45
12 | 0 | 13 | 13 | 11/2 | 7 | 0 | 7 | 29 | 0 | 6/1 | 17
12 | 1 | 10 | 10 | 8/2 | 5 | 0 | 11 | 44 | 10 | 4/2 | 14
12 | 2 (won) | 12 | 10 | 10/0 | 3 | 1 | 12 | 50 | 20 | 6/0 | 22
12 | 3 (won) | 15 | 13 | 11/2 | 6 | 0 | 7 | 38 | 0 | 5/3 | 22
13 | 0 (won) | 12 | 11 | 11/0 | 3 | 0 | 8 | 38 | 0 | 7/0 | 48
13 | 1 | 13 | 11 | 11/0 | 3 | 0 | 6 | 35 | 0 | 7/0 | 33
13 | 2 (won) | 17 | 15 | 15/0 | 4 | 1 | 3 | 30 | 0 | 7/0 | 48
13 | 3 | 11 | 10 | 8/2 | 6 | 0 | 9 | 35 | 0 | 7/2 | 38
14 | 0 | 14 | 13 | 12/1 | 3 | 0 | 11 | 41 | 0 | 6/1 | 39
14 | 1 | 9 | 9 | 9/0 | 5 | 0 | 10 | 36 | 0 | 5/0 | 32
14 | 2 | 13 | 12 | 12/0 | 6 | 0 | 6 | 31 | 0 | 8/0 | 59
14 | 3 (won) | 16 | 14 | 14/0 | 6 | 2 | 3 | 27 | 0 | 9/0 | 74
15 | 0 | 10 | 10 | 9/1 | 5 | 0 | 11 | 34 | 0 | 8/1 | 33
15 | 1 (won) | 13 | 12 | 12/0 | 6 | 0 | 6 | 33 | 0 | 8/0 | 66
15 | 2 | 15 | 13 | 10/3 | 5 | 2 | 8 | 32 | 0 | 4/2 | 21
15 | 3 | 15 | 12 | 12/0 | 4 | 2 | 7 | 33 | 0 | 5/0 | 25
16 | 0 | 15 | 12 | 10/2 | 6 | 2 | 6 | 28 | 0 | 7/1 | 23
16 | 1 | 11 | 10 | 9/1 | 5 | 0 | 12 | 32 | 0 | 3/1 | 19
16 | 2 | 17 | 15 | 14/1 | 5 | 0 | 3 | 34 | 0 | 7/1 | 46
16 | 3 (won) | 15 | 14 | 14/0 | 10 | 0 | 6 | 37 | 0 | 8/0 | 59

seed | on board | loose at end | held at end | words on board | longest real word | uncommon real words
---|---|---|---|---|---|---
1 | 53 | 239 | 2 | 30 | TWEET | AI NA ER
2 | 45 | 248 | 1 | 27 | SLITS | OP EMF ISM
3 | 47 | 243 | 4 | 24 | GEEKS | AA LITS BIZ LIB AA AI
4 | 46 | 245 | 3 | 28 | TOTED | COR AE OE
5 | 54 | 237 | 3 | 27 | RATEL | TA AA LI RATEL AE LAS
6 | 45 | 246 | 3 | 26 | TONES | JIN EMU XI EMU ER ES
7 | 51 | 238 | 5 | 26 | AMENDS | RIF
8 | 47 | 245 | 2 | 28 | GRATED | TITI XI RET VIS REX
9 | 54 | 237 | 3 | 27 | CORED | NORIA AIT DE AE LO REG EL
10 | 52 | 238 | 4 | 29 | ORATED | ORATED AA AA ER AE EF
11 | 47 | 244 | 3 | 28 | MILKS | LO OAT VOX LA AE MI
12 | 47 | 244 | 3 | 27 | MAIDENS | UM AB NA OE NA
13 | 48 | 241 | 5 | 30 | SPATES | OW OY MENO TI
14 | 49 | 243 | 2 | 29 | AGONE | LI EMU AGONE TOR ER
15 | 48 | 244 | 2 | 28 | ABATED | EN LI NA GOR
16 | 52 | 237 | 5 | 28 | RIDERS | LO AL

Means per crab per game:
- grabs 13.6, tiles placed 12.0 (real 11.1, fake 0.9), steals 5.3, drops 0.8, gave up 7.3
- words owned at the end: real 6.2, fake 0.8; idle penalty paid 3.3 (crabs that paid any: 11%)
- final score 37.7 (lowest -40, highest 109); longest idle 36.4 s
Means per game: 49.1 tiles on the board (the centre one counted), 241.8 still loose, 3.1 held, 27.6 words (4.5 of them real but uncommon)

## 2026-10-20, 16 games (seeds 1-16), only words it knows, in a hurry too (R14.2 closed, R20.*)

The person played: "ai is still playing bs words" (CR-Q15). The AI now places only placements whose every word is in its vocabulary (`words::is_common`), always: no real-word last resort before a drop, no fake or uncommon word in a hurry (SECOND-ORDER R20.1-R20.6). Both claws full with no word: it drops the rarer letter. Late with no word: it walks to the board to see it as it is now (it remembers the board only from its last time beside it), places a word there if one fits, and otherwise fetches a partner and pays the idle penalty. A held letter with nothing in view that goes with it sends it to another pile. A tile it gave up on it no longer grabs again when its crosshair lands there. Rules, tuning, thinking times and the vocabulary are unchanged. "Before" is main at b2c5a03 (the physics work since the R14 run changed the map's piles, so its numbers differ from that run's). The playtest now also counts placements that made a real but uncommon word, and lists the short words the placements made.

| per crab per game | before (main) | after | change |
|---|---|---|---|
| grabs | 16.2 | 16.5 | |
| tiles placed | 13.1 | 11.8 | -10% |
| fake placements | 0.9 | 0.0 | |
| placements making a real but uncommon word | 2.8 | 0.0 | |
| drops | 1.9 | 3.5 | |
| gave up | 4.6 | 3.9 | |
| steals | 5.5 | 5.5 | |
| real / fake words owned | 7.0 / 0.7 | 6.3 / 0.0 | |
| idle penalty paid (crabs that paid any) | 8.8 (8%) | 27.4 (58%) | +18.6 |
| final score (lowest, highest) | 40.0 (-277, 111) | 25.3 (-164, 92) | -14.7 |
| longest idle | 36.7 s | 49.9 s | |
| **per game** | | | |
| tiles on the board | 53.6 | 48.0 | |
| words on the board (real but uncommon) | 30.8 (6.8) | 25.1 (0.0) | |

Built up a part at a time (the same 16 games):

| | tiles placed | drops | idle paid | final score |
|---|---|---|---|---|
| before | 13.1 | 1.9 | 8.8 | 40.0 |
| only known words; late with no word, a partner from a pile | 10.2 | 9.2 | 69.1 | -22.8 |
| + full or late, it goes to look at the board first | 11.0 | 6.4 | 39.3 | 8.7 |
| + a tile it gave up on is not grabbed again off the crosshair | 11.3 | 6.3 | 27.6 | 21.6 |
| + nothing here for a held letter: another pile | 11.8 | 3.5 | 27.4 | 25.3 |

### What it says

- **No odd words at all.** 0 fake placements and 0 real but uncommon words in 16 games (6.8 a game before: AA, AE, OE, XI, NA, ER, EF, TA and the like).
- **The cost is the idle penalty.** 27 points a crab a game against 9, and more than half the crabs now pay some; the score falls 15. The worst crab paid 182 (seed 6, crab 1). A crab whose letters fit nowhere it knows walks between the piles and the board swapping letters; before, it placed one anyway. The first build showed why: a crab at a pile far from the board planned on the board as it last saw it (the centre tile alone), swapped letters at that pile all game and placed none (seed 12, 400 points).
- **A loop came to light and is fixed.** A crab that gave up on a tile it could not reach grabbed it again whenever the crosshair landed on it (it switches to a tile of the same letter under the crosshair): seed 1's crab 3 tried the same tile every 6 s for the last two minutes of its game. It was there before, hidden by hurried placements.
- **What would bring the idle cost down** (not done here): looking at the board on the way back from every pile, not only when late; a tile count or piles that offer more letters in view; or a longer grace. The person's call (R20.6).

seed | crab | grabs | placed | real/fake placements | steals | drops | gave up | longest idle s | idle paid | real/fake words | total
---|---|---|---|---|---|---|---|---|---|---|---
1 | 0 | 18 | 13 | 13/0 | 6 | 4 | 3 | 39 | 0 | 9/0 | 83
1 | 1 (won) | 20 | 16 | 16/0 | 3 | 2 | 1 | 33 | 0 | 11/0 | 92
1 | 2 | 16 | 12 | 12/0 | 2 | 3 | 3 | 38 | 0 | 5/0 | 29
1 | 3 | 13 | 9 | 9/0 | 3 | 3 | 5 | 60 | 64 | 6/0 | -6
2 | 0 (won) | 12 | 10 | 10/0 | 5 | 2 | 4 | 36 | 0 | 7/0 | 72
2 | 1 | 17 | 13 | 13/0 | 7 | 2 | 3 | 39 | 0 | 7/0 | 51
2 | 2 | 15 | 12 | 12/0 | 4 | 2 | 4 | 38 | 0 | 6/0 | 58
2 | 3 | 14 | 10 | 10/0 | 4 | 2 | 5 | 70 | 60 | 4/0 | -34
3 | 0 | 16 | 13 | 13/0 | 4 | 1 | 4 | 27 | 0 | 7/0 | 66
3 | 1 (won) | 17 | 14 | 14/0 | 10 | 1 | 2 | 27 | 0 | 10/0 | 79
3 | 2 | 21 | 16 | 16/0 | 4 | 4 | 1 | 34 | 0 | 7/0 | 59
3 | 3 | 15 | 10 | 10/0 | 7 | 3 | 3 | 60 | 38 | 4/0 | -3
4 | 0 | 19 | 12 | 12/0 | 8 | 6 | 3 | 50 | 18 | 6/0 | 31
4 | 1 (won) | 17 | 13 | 13/0 | 4 | 3 | 4 | 42 | 2 | 6/0 | 45
4 | 2 | 16 | 11 | 11/0 | 6 | 4 | 6 | 85 | 106 | 7/0 | -60
4 | 3 | 17 | 11 | 11/0 | 8 | 5 | 5 | 52 | 28 | 4/0 | 1
5 | 0 | 13 | 12 | 12/0 | 7 | 0 | 4 | 45 | 8 | 8/0 | 47
5 | 1 | 18 | 12 | 12/0 | 6 | 5 | 5 | 64 | 56 | 7/0 | -10
5 | 2 (won) | 17 | 12 | 12/0 | 6 | 5 | 0 | 34 | 0 | 6/0 | 69
5 | 3 | 21 | 14 | 14/0 | 7 | 5 | 1 | 56 | 32 | 5/0 | -6
6 | 0 | 21 | 18 | 18/0 | 9 | 1 | 1 | 23 | 0 | 7/0 | 56
6 | 1 | 17 | 5 | 5/0 | 3 | 10 | 4 | 124 | 182 | 3/0 | -164
6 | 2 | 17 | 13 | 13/0 | 6 | 2 | 4 | 45 | 8 | 6/0 | 34
6 | 3 (won) | 17 | 15 | 15/0 | 5 | 1 | 2 | 31 | 0 | 11/0 | 85
7 | 0 | 15 | 12 | 12/0 | 7 | 2 | 6 | 63 | 46 | 8/0 | 38
7 | 1 | 12 | 10 | 10/0 | 6 | 1 | 7 | 32 | 0 | 5/0 | 41
7 | 2 (won) | 16 | 14 | 14/0 | 6 | 2 | 3 | 34 | 0 | 7/0 | 64
7 | 3 | 16 | 13 | 13/0 | 7 | 2 | 3 | 29 | 0 | 4/0 | 28
8 | 0 | 14 | 10 | 10/0 | 4 | 3 | 9 | 71 | 72 | 5/0 | -34
8 | 1 | 15 | 8 | 8/0 | 5 | 6 | 6 | 72 | 66 | 5/0 | -1
8 | 2 (won) | 18 | 13 | 13/0 | 5 | 4 | 2 | 53 | 26 | 9/0 | 69
8 | 3 | 20 | 11 | 11/0 | 3 | 8 | 2 | 64 | 48 | 7/0 | -1
9 | 0 (won) | 12 | 11 | 11/0 | 6 | 1 | 10 | 92 | 102 | 10/0 | -6
9 | 1 | 17 | 10 | 10/0 | 5 | 6 | 6 | 84 | 86 | 5/0 | -38
9 | 2 | 18 | 8 | 8/0 | 2 | 9 | 5 | 100 | 122 | 2/0 | -96
9 | 3 | 14 | 7 | 7/0 | 4 | 6 | 5 | 84 | 122 | 4/0 | -96
10 | 0 | 14 | 10 | 10/0 | 6 | 3 | 6 | 51 | 26 | 5/0 | 11
10 | 1 (won) | 15 | 12 | 12/0 | 8 | 3 | 5 | 45 | 8 | 9/0 | 61
10 | 2 | 17 | 14 | 14/0 | 5 | 2 | 2 | 30 | 0 | 5/0 | 46
10 | 3 | 16 | 12 | 12/0 | 6 | 3 | 4 | 48 | 16 | 6/0 | 49
11 | 0 (won) | 19 | 13 | 13/0 | 4 | 4 | 4 | 43 | 6 | 7/0 | 82
11 | 1 | 16 | 13 | 13/0 | 8 | 2 | 4 | 36 | 0 | 6/0 | 59
11 | 2 | 20 | 11 | 11/0 | 6 | 8 | 1 | 37 | 0 | 6/0 | 76
11 | 3 | 18 | 11 | 11/0 | 6 | 5 | 2 | 50 | 20 | 5/0 | 33
12 | 0 | 14 | 7 | 7/0 | 3 | 6 | 10 | 69 | 62 | 2/0 | -48
12 | 1 | 15 | 8 | 8/0 | 4 | 6 | 6 | 53 | 36 | 7/0 | 1
12 | 2 | 20 | 13 | 13/0 | 3 | 5 | 3 | 53 | 26 | 7/0 | 33
12 | 3 (won) | 18 | 14 | 14/0 | 6 | 3 | 5 | 30 | 0 | 9/0 | 49
13 | 0 | 17 | 13 | 13/0 | 5 | 2 | 1 | 34 | 0 | 6/0 | 42
13 | 1 (won) | 16 | 12 | 12/0 | 6 | 2 | 4 | 41 | 0 | 9/0 | 66
13 | 2 | 17 | 16 | 16/0 | 6 | 1 | 2 | 26 | 0 | 6/0 | 28
13 | 3 | 10 | 7 | 7/0 | 5 | 1 | 6 | 75 | 104 | 4/0 | -65
14 | 0 | 14 | 11 | 11/0 | 4 | 2 | 4 | 33 | 0 | 3/0 | 38
14 | 1 | 16 | 14 | 14/0 | 4 | 2 | 3 | 34 | 0 | 6/0 | 53
14 | 2 (won) | 16 | 14 | 14/0 | 7 | 1 | 1 | 32 | 0 | 8/0 | 67
14 | 3 | 15 | 10 | 10/0 | 7 | 3 | 7 | 78 | 74 | 6/0 | -13
15 | 0 | 19 | 11 | 11/0 | 2 | 7 | 2 | 39 | 0 | 4/0 | 26
15 | 1 (won) | 20 | 13 | 13/0 | 8 | 6 | 2 | 48 | 16 | 7/0 | 58
15 | 2 | 18 | 12 | 12/0 | 8 | 5 | 3 | 43 | 6 | 6/0 | 41
15 | 3 | 15 | 11 | 11/0 | 6 | 3 | 6 | 58 | 34 | 8/0 | 17
16 | 0 (won) | 17 | 13 | 13/0 | 6 | 3 | 4 | 42 | 4 | 6/0 | 65
16 | 1 | 16 | 12 | 12/0 | 5 | 3 | 4 | 40 | 0 | 7/0 | 38
16 | 2 | 18 | 12 | 12/0 | 5 | 4 | 2 | 50 | 20 | 6/0 | 32
16 | 3 | 16 | 10 | 10/0 | 7 | 5 | 4 | 44 | 6 | 6/0 | 34

seed | on board | loose at end | held at end | words on board | longest real word | uncommon real words
---|---|---|---|---|---|---
1 | 51 | 238 | 5 | 31 | MADS | 
2 | 46 | 243 | 5 | 24 | TOOLS | 
3 | 54 | 233 | 7 | 28 | MARKS | 
4 | 48 | 242 | 4 | 23 | RUSES | 
5 | 51 | 239 | 4 | 26 | GOODY | 
6 | 52 | 235 | 7 | 27 | SCORES | 
7 | 50 | 241 | 3 | 24 | MOWED | 
8 | 43 | 247 | 4 | 26 | WHEAT | 
9 | 37 | 254 | 3 | 21 | METER | 
10 | 49 | 242 | 3 | 25 | AMUSED | 
11 | 49 | 239 | 6 | 24 | TWEETS | 
12 | 43 | 246 | 5 | 25 | BEEN | 
13 | 49 | 239 | 6 | 25 | SONGS | 
14 | 50 | 240 | 4 | 23 | FARED | 
15 | 48 | 242 | 4 | 25 | TOWED | 
16 | 48 | 241 | 5 | 25 | GLADS | 
Means per crab per game:
- grabs 16.5, tiles placed 11.8 (real 11.8, fake 0.0; real but uncommon 0.0), steals 5.5, drops 3.5, gave up 3.9
- words owned at the end: real 6.3, fake 0.0; idle penalty paid 27.4 (crabs that paid any: 58%)
- final score 25.3 (lowest -164, highest 92); longest idle 49.9 s
Means per game: 48.0 tiles on the board (the centre one counted), 241.3 still loose, 4.7 held, 25.1 words (0.0 of them real but uncommon)

## 2026-10-20, Stage 2: short words, and the idle cost (R20.6-R20.12)

### The short words the AI makes

The 2- and 3-letter real words the AI's placements made in the 16 games (seeds 1-16), with how often. Main before Stage 1 (it still made real words it did not know, in its last resort and in a hurry):

> AA 8, AD 6, AE 7, AG 2, AH 2, AI 7, AL 4, AM 6, AN 12, AR 2, AS 7, AT 15, AW 2, AX 4, BA 1, BE 7, BI 1, DE 1, DO 10, ED 1, EF 1, EH 3, EL 2, EM 12, EN 2, ER 5, ES 1, ET 12, FA 1, GO 8, HA 3, HE 6, HI 3, HO 1, ID 17, IF 7, IN 9, IS 8, IT 17, JO 2, LA 5, LI 7, LO 5, MA 2, ME 8, MM 1, MO 1, MU 8, MY 1, NA 2, NE 7, NO 13, NU 2, OE 5, OF 7, OH 2, ON 12, OR 25, OS 1, OW 5, OX 7, OY 2, PA 7, PI 5, RE 30, SH 1, SI 2, SO 6, TA 2, TI 3, TO 9, UM 2, UP 6, US 10, UT 4, WE 11, WO 1, XI 1, XU 3, YA 3, YO 2, AAL 1, AAS 1, ADD 1, ADO 1, ADS 1, AGE 1, AGO 3, AID 4, AIN 1, AIR 2, AIT 2, ALE 2, ALL 1, AMI 1, AMP 1, AMU 2, ANI 1, ANY 1, ARE 5, ASK 1, ATE 2, AWE 1, AXE 1, BAD 1, BEE 2, BEL 1, BET 3, BID 1, BIN 1, BIT 1, BOA 1, BUS 1, CAT 1, COR 3, COW 1, COX 1, CUR 1, DAL 1, DID 1, DIS 1, DOE 3, DOG 2, DOR 1, DOT 1, EAT 4, EGO 1, EME 1, ERA 2, ETA 1, EWE 3, FAD 1, FEM 1, GAS 1, GEM 2, GET 1, GIT 1, GNU 1, GOD 1, GOO 2, HAE 1, HAG 1, HAH 1, HAT 3, HEM 1, HER 3, HIM 1, HIS 1, HIT 1, HOE 2, IFS 1, INN 2, INS 1, ION 3, IRE 3, ITS 1, JAM 2, JAR 1, JOY 1, JUT 1, KID 1, KIN 1, LAD 1, LAW 2, LED 1, LET 2, LID 3, LIE 2, LIS 1, LIT 2, LOG 1, LOT 1, MAN 1, MET 3, MEW 1, MOO 1, MOW 1, MUG 3, MUM 1, NAB 1, NEE 1, NET 3, NIT 2, NOD 3, NOR 3, NOW 1, NUS 1, ODE 1, OHM 1, ONE 5, ORA 1, ORC 1, ORE 17, OUT 1, OWE 5, OXO 1, PAN 2, PAP 1, PAR 1, PAS 1, PAT 1, PAW 1, PIN 2, PIS 1, PIU 1, RAM 1, RAN 2, RAT 2, RED 6, REE 1, REG 1, REI 1, REM 1, RET 2, RIA 1, RID 1, RUM 1, RYA 1, SET 1, SIN 1, SIR 1, SOL 1, SON 2, SOU 1, TAN 3, THE 1, TIE 2, TIT 4, TOE 1, TON 1, TOW 1, TOY 1, TUG 1, TUP 1, USE 3, VAN 1, VAR 1, VAT 2, VIA 1, VIS 1, VOE 1, WAD 1, WED 1, WEE 2, WET 3, WIN 1, YAR 1, YES 1, YET 1, YUP 1

After Stage 1 (only words it knows):

> AD 9, AH 2, AM 4, AN 14, AS 7, AT 19, AX 4, BE 9, BY 3, DO 9, EH 6, EM 12, GO 14, HA 5, HE 13, HI 5, HO 4, ID 8, IF 7, IN 13, IS 7, IT 21, MA 8, ME 7, MU 1, MY 1, NO 16, OF 5, OH 3, ON 14, OR 27, OX 5, PA 7, PI 4, RE 40, SO 12, TO 12, UP 10, US 4, WE 17, ADD 1, ADO 3, ADS 1, AGO 3, AID 2, AMP 2, AND 1, ANY 1, ARE 11, ATE 3, AWE 3, AXE 1, BED 2, BEE 2, BEG 2, BET 2, BIN 1, BIT 1, BYE 1, CAM 1, CAN 2, CAT 2, CHI 1, CON 1, COX 1, CUP 3, DAD 1, DOE 4, DOS 2, DOT 1, EAR 1, EAT 4, EGO 2, EON 1, EWE 3, FAN 2, FIN 1, FIT 1, FOE 1, FOR 3, GAS 3, GEM 3, GIN 1, GOO 4, GOT 2, HAD 1, HAT 1, HAY 1, HEM 2, HEN 1, HER 2, HEY 1, HID 1, HIM 1, HIP 1, HIS 1, HIT 2, HOG 2, INN 4, ION 4, IRE 7, ITS 1, KID 1, KIT 2, LAD 3, LAW 1, LAX 1, LIT 4, LYE 1, MAD 1, MAN 3, MAR 1, MAS 1, MAT 2, MEN 3, MET 2, MOO 1, NIT 2, NOR 4, NOT 1, OFF 1, OHM 1, ONE 3, ORE 16, OWE 3, PAD 1, PAL 2, PAR 1, PAS 1, PAT 3, PAY 1, PIE 1, PIG 1, PIN 1, PUP 1, RAM 1, RAN 3, RAT 2, RED 1, REV 4, ROB 1, SOB 1, SOD 2, SON 5, TEN 1, THE 1, TIN 2, TIT 3, TOE 6, TOO 4, TOW 1, USE 3, VAN 2, WAD 1, WAR 1, WAS 2, WED 2, WEE 2, WET 4, WHO 1, WIN 1, WIT 2, WON 1, WOO 1

Words in there a normal player would not know as words: EM (12 times), RE (40, the commonest short word the AI made), PI (4), MU (1); and 3-letter COX. Looking down the whole vocabulary's 518 words of 3 letters or fewer for the same kind turned up 23, now cut (`SHORT_CUT` in `tools/words/common.py`; SECOND-ORDER R20.7):

- letters' names: EM, MU, PI, RE, CHI
- abbreviations read as words: ID, HO
- plurals of short words: DOS, HES, MAS, PAS, PIS
- dated or trade words: COX, CUR, FEN, OVA, PAP, QUA, SAC, SIC, SOP, SUP, WOT

The two-letter words it knows, before (40): AD AH AM AN AS AT AX BE BY DO EH EM GO HA HE HI HO ID IF IN IS IT MA ME MU MY NO OF OH ON OR OX PA PI RE SO TO UP US WE. After (34): AD AH AM AN AS AT AX BE BY DO EH GO HA HE HI IF IN IS IT MA ME MY NO OF OH ON OR OX PA SO TO UP US WE. The vocabulary goes from 44,505 words to 44,482. After Stage 2 the short words made in the same 16 games:

> AD 16, AH 1, AM 14, AN 17, AS 8, AT 21, AX 1, BE 13, BY 2, DO 16, EH 4, GO 18, HA 9, HE 14, HI 7, IF 6, IN 25, IS 10, IT 31, MA 8, ME 19, MY 4, NO 9, OF 10, OH 4, ON 10, OR 44, OX 3, PA 18, SO 9, TO 10, UP 2, US 10, WE 17, ADO 7, AGO 6, AMP 1, AND 2, ANT 2, ASK 1, ASS 1, ATE 6, AWE 4, AXE 1, BAD 2, BED 3, BEE 3, BEG 2, BET 1, BIB 1, BIN 1, BIT 3, BUS 1, CAD 1, CAM 2, CAN 2, CAT 2, CON 1, DAD 1, DAM 1, DIE 1, DIN 1, DOE 5, DOG 2, DON 1, DOT 1, EAT 3, EGO 2, EON 1, EWE 1, EYE 1, FAD 2, FAN 1, FAT 2, FIN 1, FOR 1, FOX 1, GAS 1, GOD 1, GOO 2, GOT 2, HAD 2, HAG 1, HAM 1, HAS 3, HAT 2, HAY 1, HEM 2, HEN 1, HER 3, HEW 2, HEY 1, HIM 1, HIS 5, HIT 2, INK 1, INN 5, ION 1, IRE 1, ITS 1, JAM 1, KIN 1, KIT 2, LAD 2, LIT 2, MAD 1, MAN 2, MAT 1, MAY 2, MEN 6, MET 4, MEW 2, NIT 3, NOD 1, NOR 3, NOT 4, NOW 1, OFF 1, OHM 1, ONE 3, ORE 24, OWE 3, PAD 2, PAL 2, PAN 1, PAR 1, PAT 2, PIN 1, PIT 1, PUS 1, RAG 1, RAM 3, RAN 2, RAT 2, RIM 1, SAT 1, SET 1, SHE 1, SIT 1, SON 5, SPA 1, THE 2, TIN 3, TIT 1, TOE 3, TON 2, TOT 1, TOW 2, TOY 1, USE 5, VAN 1, VAT 2, WAS 1, WEB 1, WED 1, WEE 3, WET 4, WIN 3, WIT 1, WON 1, YAM 1

### The idle cost

The coordinator's direction: bring the idle penalty down without letting a word it does not know back in; look at the board more, prefer letters that make words it knows; grace, tile count and piles unchanged.

**16 games are too few.** Seeds 1-16 gave Stage 1 27.4 idle points a crab; seeds 1-64 give 53.1. Single changes moved the 16-game mean 10-15 points either way by chance, so every number below is 64 games (seeds 1-64), with the 16-game one beside it for comparison with Stage 1's table.

| per crab per game, 64 games | main before Stage 1 | Stage 1 | Stage 2 |
|---|---|---|---|
| tiles placed | 13.5 | 10.7 | 11.0 |
| fake placements | 1.0 | 0.0 | 0.0 |
| placements making a real but uncommon word | 3.1 | 0.0 | 0.0 |
| drops | 2.0 | 4.8 | 4.7 |
| idle penalty paid (crabs that paid any) | 2.8 (7%) | 53.1 (68%) | 44.8 (68%) |
| final score (lowest, highest) | 45.6 (-277, 129) | -6.7 (-400, 124) | 1.4 (-368, 115) |
| real but uncommon words on the board a game | 7.3 | 0.0 | 0.0 |
| seeds 1-16: idle paid, tiles placed, final score | 8.8, 13.1, 40.0 | 27.4, 11.8, 25.3 | 23.3, 12.1, 28.6 |

What Stage 2 changed, and what it measured (64 games, idle paid / final score):

| | idle | score |
|---|---|---|
| Stage 1 | 53.1 | -6.7 |
| + the shorter vocabulary, a late empty-handed crab taking the best letter in view (not the nearest blind), the centre tile always a letter with a two-letter word (R20.8; three games of 64 had a centre C, L or the like and no crab ever placed) | 51.5 | -6.2 |
| + of two words, the near one: `place_walk_cost_per_m` 9 points a metre to the square (R20.9; measured on 16 games only: idle 43.7 to 27.0) | | |
| + a placement worth `open_square_pts` 10 more for each square beside it still open to a word it knows (R20.10) | 44.8 | 1.4 |

Tried and left out, each worse or no better:

- **Looking at the board after every pile trip** (the coordinator's first option), not only when full or late: idle 34.4 against 27.0 (16 games). It sends a crab holding one letter with no word to the board to look, where it would have fetched a partner.
- **Dropping a lone letter with no word at once** when its view of the board is fresh: idle 58.5 against 43.7 (16 games); it threw away letters that a partner would have made good.
- **Letting go of a letter it kept last time** (so one claw is not held by a letter all game): 45.1 against 44.8 (64 games).
- `open_square_pts` 3: 49.1; 20: 47.4 (64 games).
- The old vocabulary (Stage 1's) with the rest of Stage 2: 50.2 against 51.5 before the open squares (64 games): the shorter list costs about nothing.

### What it says

- **Stage 2 wins back about 8 of the 50 points.** The rule (only words it knows) costs about 42 idle points a crab a game against main before Stage 1, and the final score falls from 45.6 to 1.4.
- **Where it goes:** a crab whose letters fit nowhere it knows shuttles between piles and the board. The boards stay small (45 tiles against 55) and some close up: in seed 48 the board was 12 tiles with every open square needing a word like "?FOX" or "EH?", and all four crabs paid over 250.
- **What would help most next** (not done: the coordinator kept the grace, the tile count and the piles): more letters in view at a pile (the tile count or how the piles lie), or a longer grace; or a shorter `hurry_ms` with nothing else to do. R20.6.
