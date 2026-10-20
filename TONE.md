# Tone

The rules every reader-visible string follows: the text in `data/`, the definitions in `GLOSSARY.md`, and this project's `README.md`. Written by the person (or adopted as given), read by the agent before writing any string. `crates/core/tests/copy.rs` checks the word lists below; a reviewer checks the rest.

## Rules

1. **Lead with the point.** The first words of a string say what the reader needs: the result, the action, the reason.
2. **Plain words, short sentences.** Under 25 words a sentence. One idea a sentence.
3. **One name for each thing.** A term means what `GLOSSARY.md` says and is not renamed afterwards. The words on an entry's `Not:` line are never used for it.
4. **Numbers come from the core.** A count or limit a rule decides is filled in by the core, never typed into a string.
5. **A refusal names what is in the way and the next step.** No apology, no "something went wrong".
6. **No universals the tool cannot back.** *Every*, *always*, *never*, *only* and *nothing* state a rule of the tool, and a test shows it.
7. **Colour is never the only cue.** No string names a thing by its colour alone.
8. **No hype and no stock phrasing.** The lists below are the ones a test can check. Lists of three written for rhythm, slogans and closing flourishes are for the reviewer.

## Project decisions

- **Labels and short calls (KISS).** The person's standing rule from Honey Hole, 2026-10-18: "Keep it simple, stupid ... This is a game, there should be skill expression, you don't need to tell the players exactly what is happening and what to do at all times." On screen: labels and short calls; no rules, counts, controls or consequences unless the player can't play without them. Spec details go in code comments and docs, not in strings.
- **The idle countdown may end with "!".** The person's words, 2026-10-19: put a countdown in the corner that "says something like "place a tile!"". A string under the `idle.` prefix may end with one "!" and have none elsewhere (SECOND-ORDER R1.19). Every other string keeps rule 8.
- **No running score.** Scores are hidden until the tally, so no string during a game names a player's points, apart from the idle player's own "-2" popups.
- **No trademark.** No player-visible text says "Scrabble" (the person, 2026-10-19); say "tiles" or "word tiles". It is on the word list below so the copy test catches it.
- **Playful, not punny.** Light sea words where they fit a label (the title, a call like "Tally time"), never a pun in a control or a refusal.

## Word lists for the lint

Phrases on these lists fail `no_text_uses_a_phrase_the_tone_file_rules_out`. Add a project's own phrases at the end. A phrase that is right in one file goes in `ALLOWED` in `copy.rs` with its reason.

<!-- lint:start -->
Stock words:

- delve
- tapestry
- crucially
- robust
- leverage
- underscore
- landscape
- realm
- deep dive
- key takeaway
- seamless
- game-changer
- elevate
- empower

Announcing a point:

- it is worth noting
- it's worth noting
- it is important to

Contrast formulas:

- not only
- not just
- isn't just
- more than just

Filler:

- simply
- really
- actually

Exclamation:

- !
Trademark:

- scrabble
<!-- lint:end -->
