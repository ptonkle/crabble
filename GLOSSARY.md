# Glossary

The source of truth for what each word in this tool means. Strings, tests, the plan and the README use a term in the sense given here. The page shows no definitions on screen (as in Ape Race and Honey Hole); this file and its copy tests keep the words straight.

Write each entry as a `### Term` heading, then one or two plain sentences, then an optional `Not:` line listing words that must not be used for it. A number a rule decides is a `{placeholder}` the core fills, so the glossary cannot disagree with the rules. Keep entries in alphabetical order. `crates/core/tests/copy.rs` checks the format, the placeholders and the `Not:` words; `TONE.md` governs the prose.

<!--
An example entry. It sits inside this comment, so it does not count.

### Spout

Where slush comes out. Each line has up to {line.spouts} spouts.

Not: tap, dispenser, nozzle
-->

### AI crab

A crab the computer plays, by the same rules and inputs as a person. It takes time to choose which tile to grab and where to put it.

Not: bot, CPU, NPC

### Board

The square grid of {board.size} by {board.size} squares in the middle of the seafloor, flat and level, where tiles are placed.

Not: grid, mat

### Claw

One of a crab's two pincers, left or right. Each holds at most one tile; the left mouse button works the left claw and the right button the right.

Not: hand, pincer, nipper

### Code

The six letters and digits that name a lobby. The host shares it, or a link holding it, and the others join by it.

Not: room code, PIN, password

### Crab

The creature each player steers, in the player's colour. It walks, jumps, and grabs and places tiles with its claws.

Not: avatar, character

### Crosshair

The mark in the middle of the screen. The tile under it is the one a claw grabs, and the square under it is where a held tile goes.

Not: reticle, cursor

### Drop

Letting a held tile fall to the sand without placing it on the board. It can be picked up again.

Not: discard, throw

### Gallery

A screen for looking at each model on the sand under the water: turning, in its moves and colours, near or far, and at each scene.

Not: model viewer, showroom

### Game

One round of Crabble, {game.minutes} minutes long, from the start to the tally.

Not: match, round

### Ghost tile

A see-through copy of a held tile, shown on the square where a click would place it.

Not: preview tile, shadow

### Host

The player who opened the lobby. The host adds AI crabs, starts the game and offers the rematch, and the host's browser keeps the game's clock.

Not: server, leader

### How to play

The Title's screen of a few short lines on the game and its controls. It names no number and no exact rule.

Not: tutorial, manual

### Idle penalty

The points a player loses for going too long without placing a tile. It is taken off their total at the tally.

Not: fine, timeout

### Lobby

The room where players gather before an online game. The host shares its code or link, everyone readies up, and the host starts.

Not: waiting room, room

### Local game

A game on this computer against AI crabs.

Not: offline game, single player

### Mound

A low hump of sand around the board where tiles are found, mostly in piles.

Not: hill, dune

### Online game

A game with other people in other browsers, joined through a lobby.

Not: multiplayer game, network game

### Overhead view

The board seen from straight above, in the bottom-left corner: every placed tile, the premium squares, outlines and a see-through circle for each crab. Holding Tab enlarges it.

Not: minimap, map, radar

### Owner

The player who last added a letter to a word. A word's points go to its owner at the tally, or are taken from them if it is not a real word.

Not: author, holder

### Pasted code

A long invite code or reply code copied between two browsers by hand, to join a lobby without the relays. Offered to the automated checks.

Not: token

### Pile

A messy heap of tiles on a mound, some face down. Pull one from the bottom and the ones above may tumble.

Not: stack, heap

### Place

Putting a held tile on the board. A placed tile touches a tile already there, and it never moves again.

Not: play, lay, put down

### Premium square

A board square that multiplies points: double letter, triple letter, double word or triple word. It counts for every word through it.

Not: bonus square, multiplier

### Ready

A player's mark in the lobby that they are set to start. The host can start once every other player is ready.

Not: confirm, lock in

### Real word

A word in the word list. Its letters turn green on the board and in the overhead view.

Not: valid word, legal word

### Rematch

The host's choice at the end of an online game: everyone goes back to the lobby with the same seats, ready.

Not: replay, restart

### Scene

A still for the design journal, set out in a data file: models placed on the sand and a camera to see them by.

Not: diorama, setup

### Settings

The volume, the mouse's sensitivity and whether it tilts the camera the other way. They are kept on this device between visits, with your name.

Not: options, preferences

### Tally

The end of a game, when each player's words are counted up one by one and the scores are shown for the first time.

Not: results, scoreboard

### Tile

A wooden letter tile with its letter and point value on its face.

Not: piece, block, chip

### Title

The first screen: the game's name and its entries.

Not: main menu, home screen, start screen

### What's New

The list of the game's versions, newest first, each with what it changed.

Not: changelog, release notes

### Word

A row or column of {word.min_letters} or more letters with no gap. Every word on the board has an owner.

Not: run, chain

### Word preview

The small popup near the top of the screen that shows the words a held tile would form where it is aimed, and which of them are real words.

Not: tooltip, hint
