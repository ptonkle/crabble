"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. Drive the tool here the way a person would, and assert what the page
shows with `check(condition, "what this shows")`. Assert the effect a person
would see (the text drawn, the file saved), not the markup that should cause it.

Expected values in these checks are worked out by hand from the brief (C4),
never copied from the tool's own output. Wait on a selector or a data-*
attribute the page sets, never on a fixed time: on CI the page may draw with
a CPU and run slowly.
"""

import json
import pathlib

DATA = pathlib.Path(__file__).resolve().parents[2] / "data"
STRINGS = json.loads((DATA / "strings.json").read_text())
GALLERY = json.loads((DATA / "gallery.json").read_text())["strings"]
TILE = json.loads((DATA / "models" / "tile.json").read_text())


def run(page, check, engine):
    check(page.inner_text("#title") == STRINGS["title"], "the title from data/strings.json is drawn")
    check(page.inner_text("#intro p") == STRINGS["intro"], "the intro from data/strings.json is drawn")
    check(page.get_attribute("body", "data-screen") == "start", "the core's start screen is shown")
    check(page.inner_text("#message") == "", "no error message is shown")

    # the Title's Gallery entry opens the Gallery, which draws the 3D view
    page.click(f"#intro button:has-text('{STRINGS['gallery']}')")
    page.wait_for_selector("body[data-screen='gallery']", state="attached", timeout=30000)
    page.wait_for_selector("body[data-drawn='true']", state="attached", timeout=30000)
    check(page.is_visible("#gallery"), "the Gallery's controls show")
    check(page.inner_text("#gallery-message") == "", "the Gallery's files were all accepted")
    check(not page.is_visible("main"), "the Title is hidden behind the Gallery")

    # choosing the tile shows its name, its size and a letter to choose
    page.select_option("[id='gallery.model']", "tile")
    page.wait_for_selector(f"#gallery-heading:has-text('{TILE['name']}')", timeout=10000)
    check(page.inner_text("#gallery-sub") == "200 × 200 × 45 mm", "the tile is 200 by 200 by 45 mm")
    page.select_option("[id='gallery.letter']", "Q")
    check(page.input_value("[id='gallery.letter']") == "Q", "a letter can be chosen for the tile")
    check(page.inner_text("#gallery-message") == "", "no Gallery control was refused")

    # Back returns to the Title
    page.click(f"[id='gallery.close']:has-text('{GALLERY['back']}')")
    page.wait_for_selector("body[data-screen='start']", state="attached", timeout=10000)
    check(page.is_visible("main"), "Back returns to the Title")
    check(not page.is_visible("#gallery"), "the Gallery's controls are gone")
