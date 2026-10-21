"""The project's own browser interactions, run by check.py in every engine.

check.py has already loaded the page, waited for it to report ready, and let
it idle. Drive the game here the way a person would, and assert what the page
shows with `check(condition, "what this shows")`. Assert the effect a person
would see (the text drawn), not the markup that should cause it.

Expected values in these checks are worked out by hand from the brief (C4),
never copied from the game's own output. Wait on a selector or a data-*
attribute the page sets, never on a fixed time: on CI the page may draw with
a CPU and run slowly.

The interactions are steps, `step_<name>(page, check)`, listed in STEPS. Each
starts and ends on the Title, so any of them can run alone after `boot`.
tests/tags.toml tags each step ([browser]); scripts/quick-check.sh runs only
the steps a change needs, through BUILDER_STEPS. A new step goes in STEPS and
in [browser], or `scripts/tags.py guard` fails.

The page opens with `?fast` (crates/core/src/app/fast.rs): no 3-2-1 to wait
through. BUILDER_FAST=0 checks the players' timing.
"""

import json
import os
import pathlib

DATA = pathlib.Path(__file__).resolve().parents[2] / "data"
STRINGS = json.loads((DATA / "strings.json").read_text())
GALLERY = json.loads((DATA / "gallery.json").read_text())["strings"]
TILE = json.loads((DATA / "models" / "tile.json").read_text())
VERSION = "v" + json.loads((DATA / "patch-notes.json").read_text())[0]["version"]

FAST = os.environ.get("BUILDER_FAST") != "0"
FLAGS = ["fast"] if FAST else []


def address(base, *more):
    """`base` (the page's URL without a query) with the test flags and `more`
    (such as "short")."""
    query = "&".join(FLAGS + list(more))
    return base + ("?" + query if query else "")


def reopen(page, *more):
    """Opens the page again with the test flags and `more`, and waits for it."""
    page.goto(address(page.url.split("?")[0].split("#")[0], *more))
    page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)


def by_id(action):
    """A selector for an element whose id is a dotted action name."""
    return f"[id='{action}']"


def screen(page, name, timeout=5000):
    page.wait_for_selector(f"body[data-screen='{name}']", state="attached", timeout=timeout)
    return page.get_attribute("body", "data-screen") == name


def phase(page, name, timeout=10000):
    try:
        page.wait_for_selector(f"body[data-phase='{name}']", state="attached", timeout=timeout)
        return True
    except Exception:
        return False


def focused(page):
    return page.evaluate("() => document.activeElement && document.activeElement.id")


HAS_WEBGL2 = "() => !!document.createElement('canvas').getContext('webgl2')"

# The 3D view shrunk to 64 by 36, as a list of RGBA values.
VIEW_PIXELS = """() => {
  const small = document.createElement('canvas');
  small.width = 64;
  small.height = 36;
  const g = small.getContext('2d');
  g.drawImage(document.getElementById('play'), 0, 0, 64, 36);
  return Array.from(g.getImageData(0, 0, 64, 36).data);
}"""

# Run before the page's own scripts: the browser has no WebGL at all.
NO_WEBGL = """(() => {
  const get = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (kind, ...rest) {
    return kind === 'webgl2' || kind === 'webgl' ? null : get.call(this, kind, ...rest);
  };
})();"""


def wordmark_shown(page):
    """Whether the name is drawn as the big text wordmark, not only kept
    for screen readers."""
    box = page.locator(".panel.title h2").bounding_box()
    return box is not None and box["height"] > 40 and page.is_visible(".panel.title h2")


def step_boot(page, check):
    check(screen(page, "title"), "the page opens on the Title")
    check(page.text_content(".panel.title h2") == "Crabble", "the Title holds the game's name")
    if page.evaluate(HAS_WEBGL2):
        # the 3D title card (CR-Q7, standing) is drawn behind the menu, and
        # moves a little; the wordmark is not shown over it
        page.wait_for_selector("body[data-scene='on'][data-drawn='true']", state="attached", timeout=20000)
        check(page.is_visible("#play"), "the title card is drawn behind the Title")
        check(not wordmark_shown(page), "the text wordmark gives way to the title card")
        # the view's pixels, small (a screenshot would add a style the
        # page's policy refuses in WebKit)
        first = page.evaluate(VIEW_PIXELS)
        page.wait_for_timeout(800)
        check(len(set(first)) > 1, "the title card is drawn, not a blank view")
        check(page.evaluate(VIEW_PIXELS) != first, "the title card moves gently")
    else:
        check(wordmark_shown(page), "without WebGL the Title shows the wordmark")
    menu = page.locator(".panel.menu .button").all_inner_texts()
    want = ["Local game", "Online game", "How to play", "Settings", "What's New", "Gallery"]
    check(menu == want, f"the Title's entries are Local game, Online game, How to play, Settings, What's New, Gallery {menu}")
    check(page.inner_text("#version") == VERSION, "the version shows in the corner")
    check(page.inner_text("#message") == "", "no error message is shown")
    check(page.locator(".bubbles span").count() > 0, "bubbles rise behind the menus")


def step_how(page, check):
    page.click(by_id("menu.how"))
    check(screen(page, "how"), "How to play opens from the Title")
    lines = page.locator(".panel.how .lines li").all_inner_texts()
    check(len(lines) == 5, f"How to play has five short lines {lines}")
    check(not any(c.isdigit() for c in "".join(lines)), "and no numbers in them")
    keys = page.locator(".panel.how .pairs td").all_inner_texts()
    check("Hold Tab" in keys and "W A S D" in keys, f"the controls list the keys {keys}")
    page.keyboard.press("Escape")
    check(screen(page, "title"), "Escape on How to play goes to the Title")


def step_settings(page, check):
    page.click(by_id("menu.settings"))
    check(screen(page, "settings"), "Settings opens from the Title")
    check(page.inner_text(by_id("settings.volume.text")) == "80%", "the volume starts at 80%")
    page.locator(by_id("settings.volume")).focus()
    page.keyboard.press("ArrowLeft")
    page.wait_for_function("() => document.getElementById('settings.volume.text').textContent === '75%'", timeout=5000)
    check(page.inner_text(by_id("settings.volume.text")) == "75%", "one step down on the slider sets 75%")
    check(focused(page) == "settings.volume", "the slider keeps the focus while the menus redraw")
    check(page.inner_text(by_id("settings.mouse.text")) == "100%", "the mouse sensitivity starts at 100%")
    page.click(by_id("settings.invert_y"))
    page.wait_for_function("() => document.getElementById('settings.invert_y').checked", timeout=5000)
    check(page.is_checked(by_id("settings.invert_y")), "Invert mouse Y ticks")
    page.locator(by_id("menu.back")).focus()
    page.keyboard.press("Enter")
    check(screen(page, "title"), "Back goes to the Title")
    check(focused(page) == "menu.local", "the new screen's first button takes the focus")
    page.click(by_id("menu.settings"))
    screen(page, "settings")
    check(page.inner_text(by_id("settings.volume.text")) == "75%", "the volume set earlier is kept for the visit")
    # put back for the steps after this one
    page.click(by_id("settings.invert_y"))
    page.keyboard.press("Escape")
    check(screen(page, "title"), "Escape on Settings goes to the Title")
    page.locator(by_id("menu.local")).focus()
    page.keyboard.press("ArrowDown")
    check(focused(page) == "menu.online", "Down moves the focus from Local game to Online game")


def step_notes(page, check):
    page.click(by_id("menu.notes"))
    check(screen(page, "notes"), "What's New opens from the Title")
    check(page.inner_text(".note-version") == VERSION, "the newest notes are this version's")
    page.click(by_id("menu.back"))
    screen(page, "title")
    page.click("#version")
    check(screen(page, "notes"), "the version in the corner opens What's New")
    page.click(by_id("menu.back"))
    screen(page, "title")


def step_local(page, check):
    page.click(by_id("menu.local"))
    check(screen(page, "local"), "Local game opens its setup")
    options = page.eval_on_selector(by_id("local.ai"), "s => [...s.options].map(o => o.value)")
    check(options == ["1", "2", "3"], f"one to three AI crabs {options}")
    page.select_option(by_id("local.ai"), "3")
    page.click(by_id("local.start"))
    check(screen(page, "play"), "Start opens Play")
    check(phase(page, "running"), "fast mode skips the 3-2-1")
    check(page.inner_text(".panel.clock h2") in ("4:00", "3:59"), "the game clock starts at 4:00")
    check(page.is_visible("#play"), "the seafloor is drawn")
    page.keyboard.down("KeyW")
    try:
        page.wait_for_selector("body[data-moved='true']", state="attached", timeout=10000)
        moved = True
    except Exception:
        moved = False
    page.keyboard.up("KeyW")
    check(moved, "W walks the crab")
    page.keyboard.press("Escape")
    page.wait_for_selector(".panel.pause", timeout=5000)
    buttons = page.locator(".panel.pause .button").all_inner_texts()
    check(buttons == ["Resume", "Settings", "Quit to Title"], f"Escape opens the pause menu {buttons}")
    page.click(by_id("menu.resume"))
    page.wait_for_selector(".panel.pause", state="detached", timeout=5000)
    check(True, "Resume closes it")
    page.keyboard.press("Escape")
    page.click(by_id("menu.quit"))
    check(screen(page, "title"), "Quit to Title goes to the Title")


def step_end(page, check):
    # a short game (`?short`), to reach the end without four minutes
    reopen(page, "short")
    page.click(by_id("menu.local"))
    page.click(by_id("local.start"))
    check(phase(page, "ended", timeout=15000), "a short game ends")
    check(page.inner_text(".panel.end h2") == "Time's up", "the end says time's up")
    # the tally: a column a crab, revealed one by one, then the winner (the
    # core's tests check the words and points; AI crabs may place some here)
    cols = page.locator(".panel.end .tally-col").count()
    check(cols >= 2, f"the tally has a column a crab ({cols})")
    page.wait_for_selector(".panel.end .tally-col.winner", timeout=30000)
    totals = page.locator(".panel.end .tally-total").all_inner_texts()
    check(len(totals) == cols, f"each column shows its total {totals}")
    winner = page.inner_text(".panel.end .lines")
    check(winner == "A tie" or winner.endswith(" wins"), f"the winner line names the winner {winner!r}")
    buttons = page.locator(".panel.end .button").all_inner_texts()
    check(buttons == ["Back to Title"], f"a Local game's end offers Back to Title {buttons}")
    page.click(by_id("end.title"))
    check(screen(page, "title"), "Back to Title goes to the Title")
    reopen(page)


def step_centre(page, check):
    """The centre letter of a later game on the same page is drawn as dealt,
    and placing a tile beside it leaves it be (SECOND-ORDER R21.1). The page
    keeps each static layer until the core's signature for it changes; the
    next game's tiles lie where the last game's lay, so the centre's layer
    (the last tile bucket, before the word marks) must change with its
    letter. `?spot=board` hands the crab a tile and aims it at the square
    right of the centre, so the word preview's word starts with the
    centre's letter."""
    reopen(page, "spot=board")
    seen = []
    for _ in range(5):
        page.click(by_id("menu.local"))
        page.click(by_id("local.start"))
        phase(page, "running")
        page.wait_for_selector(".panel.preview td", timeout=10000)
        word = page.locator(".panel.preview td").first.inner_text()
        layers = json.loads(page.evaluate("() => window.__game.frame_info()"))["layers"]
        seen.append((word[0], layers[-2]))
        if len(seen) > 1 and seen[-1][0] != seen[-2][0]:
            break
        page.keyboard.press("Escape")
        page.click(by_id("menu.quit"))
        screen(page, "title")
    (was, before), (now, after) = seen[-2], seen[-1]
    check(was != now, f"two games in a row with other centre letters ({was}, {now})")
    check(before != after, f"the centre's layer is drawn again for the new letter ({was} to {now})")
    # place the held tile beside the centre: the preview goes (the claw is
    # empty)
    page.evaluate("() => window.__game.act('mouse.down', '0')")
    page.wait_for_selector(".panel.preview", state="detached", timeout=5000)
    check(True, "the tile is placed beside the centre")
    page.keyboard.press("Escape")
    page.click(by_id("menu.quit"))
    check(screen(page, "title"), "back to the Title")
    reopen(page)


def step_online(page, check):
    # the Online game screen only: Host or Join would reach the relays, which
    # scripts/check-online.sh covers by pasted codes instead
    page.click(by_id("menu.online"))
    check(screen(page, "online"), "Online game opens from the Title")
    page.fill(by_id("online.code"), "ABC")
    page.evaluate("() => document.activeElement.blur()")
    page.click(by_id("online.join"))
    page.wait_for_selector(".panel.online .sub", timeout=5000)
    check(page.inner_text(".panel.online .sub") == "A code is 6 letters and digits.", "a short code is refused by name")
    check(screen(page, "online"), "and the screen stays")
    page.keyboard.press("Escape")
    check(screen(page, "title"), "Escape goes to the Title")


def step_gallery(page, check):
    # the Title's Gallery entry opens the Gallery, which draws the 3D view
    page.click(by_id("menu.gallery"))
    page.wait_for_selector("body[data-screen='gallery']", state="attached", timeout=30000)
    page.wait_for_selector("body[data-drawn='true']", state="attached", timeout=30000)
    check(page.is_visible("#gallery"), "the Gallery's controls show")
    check(page.inner_text("#gallery-message") == "", "the Gallery's files were all accepted")
    check(not page.is_visible("#menus"), "the Title is hidden behind the Gallery")
    # choosing the tile shows its name, its size and a letter to choose
    page.select_option("[id='gallery.model']", "tile")
    page.wait_for_selector(f"#gallery-heading:has-text('{TILE['name']}')", timeout=10000)
    check(page.inner_text("#gallery-sub") == "200 × 200 × 45 mm", "the tile is 200 by 200 by 45 mm")
    page.select_option("[id='gallery.letter']", "Q")
    check(page.input_value("[id='gallery.letter']") == "Q", "a letter can be chosen for the tile")
    check(page.inner_text("#gallery-message") == "", "no Gallery control was refused")
    # Back returns to the Title
    page.click(f"[id='gallery.close']:has-text('{GALLERY['back']}')")
    check(screen(page, "title", timeout=10000), "Back returns to the Title")
    check(page.is_visible("#menus"), "the Title shows again")
    check(not page.is_visible("#gallery"), "the Gallery's controls are gone")


def step_sounds(page, check):
    # the page loads the sounds the core checked, and plays the ones it
    # names: a short Local game names the start and the end
    check(page.get_attribute("body", "data-audio") in ("waiting", "started"), "the sounds file is accepted")
    reopen(page, "short")
    page.click(by_id("menu.local"))
    page.click(by_id("local.start"))
    check(phase(page, "ended", timeout=15000), "a short game ends")
    # the end's sound, then the tally's: last of all the winner's fanfare
    page.wait_for_selector("body[data-last-sound='fanfare']", state="attached", timeout=60000)
    check(page.get_attribute("body", "data-last-sound") == "fanfare", "the end of the game and its tally play their sounds")
    page.click(by_id("end.title"))
    reopen(page)


def step_fallback(page, check):
    # a browser without WebGL: the Title falls back to the text wordmark
    # and its entries still work (a page of its own, closed after)
    other = page.context.new_page()
    errors = []
    other.on("pageerror", lambda e: errors.append(str(e)))
    other.add_init_script(NO_WEBGL)
    other.goto(page.url)
    other.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    check(screen(other, "title"), "without WebGL the page still opens on the Title")
    check(wordmark_shown(other) and other.inner_text(".panel.title h2") == "Crabble", "without WebGL the Title shows the wordmark")
    check(other.get_attribute("body", "data-scene") != "on", "without WebGL no 3D view is drawn")
    other.click(by_id("menu.how"))
    check(screen(other, "how"), "without WebGL the Title's entries still open")
    check(not errors, f"without WebGL no page error {errors}")
    other.close()

# A finger's touch on a control, as pointer events (web/touch.js listens to
# pointer events, which every engine can make; Playwright's touchscreen only
# taps). `moves` are (dx, dy) steps from the start, in CSS pixels.
DRAG = """([sel, x, y, moves, up]) => {
  const node = document.querySelector(sel);
  const at = (type, cx, cy) => node.dispatchEvent(new PointerEvent(type, {
    pointerId: 41, pointerType: 'touch', isPrimary: true, clientX: cx, clientY: cy,
    bubbles: true, cancelable: true, buttons: type === 'pointerup' ? 0 : 1 }));
  at('pointerdown', x, y);
  moves.forEach(([dx, dy]) => at('pointermove', x + dx, y + dy));
  if (up) at('pointerup', x + (moves.length ? moves[moves.length - 1][0] : 0), y + (moves.length ? moves[moves.length - 1][1] : 0));
}"""

WIDE = "() => document.documentElement.scrollWidth > window.innerWidth + 1 || document.body.scrollWidth > window.innerWidth + 1"


# The middle of the 3D view (your own crab stands there, under the
# crosshair), 32 by 18 pixels from a 128 by 72 shrink: how many are
# vermilion (no blue at all, colourblind mode's first seat) and how many
# white (the symbol on the shell).
CRAB_PIXELS = """() => {
  const small = document.createElement('canvas');
  small.width = 128;
  small.height = 72;
  const g = small.getContext('2d');
  g.drawImage(document.getElementById('play'), 0, 0, 128, 72);
  const d = g.getImageData(48, 36, 32, 18).data;
  let orange = 0, white = 0;
  for (let i = 0; i < d.length; i += 4) {
    const [r, gr, b] = [d[i], d[i + 1], d[i + 2]];
    if (r > 120 && b < r * 0.1 && gr < r * 0.75) orange++;
    if (r > 210 && gr > 210 && b > 210) white++;
  }
  return [orange, white];
}"""


def colourblind_from_pause(page, on):
    """Pause, open Settings over the game, set colourblind mode `on`, back
    to the game."""
    page.keyboard.press("Escape")
    page.wait_for_selector(".panel.pause", timeout=5000)
    page.click(by_id("menu.settings"))
    page.wait_for_selector(".panel.settings", timeout=5000)
    if page.is_checked(by_id("settings.colourblind")) != on:
        page.click(by_id("settings.colourblind"))
    page.wait_for_selector(f"body[data-colourblind='{'on' if on else ''}']", state="attached", timeout=5000)
    page.click(by_id("menu.back"))
    page.wait_for_selector(".panel.pause", timeout=5000)
    page.click(by_id("menu.resume"))
    page.wait_for_selector(".panel.pause", state="detached", timeout=5000)


def step_colourblind(page, check):
    # the person, R19: "colorblind mode doesn't seem to do literally
    # anything"; it is switched mid-game from the pause menu's Settings, and
    # shows on the game at once
    page.click(by_id("menu.local"))
    page.select_option(by_id("local.ai"), "1")
    page.click(by_id("local.start"))
    check(phase(page, "running"), "the game starts")
    check(page.get_attribute("body", "data-colourblind") == "", "colourblind mode starts off")
    webgl = page.evaluate(HAS_WEBGL2)
    if webgl:
        page.wait_for_selector("body[data-drawn='true']", state="attached", timeout=20000)
        page.wait_for_timeout(300)
        off = page.evaluate(CRAB_PIXELS)
    colourblind_from_pause(page, True)
    check(page.get_attribute("body", "data-colourblind") == "on", "Settings over the game switches colourblind mode on")
    check(screen(page, "play"), "Back from Settings over the game goes back to the game")
    if webgl:
        page.wait_for_timeout(300)
        on = page.evaluate(CRAB_PIXELS)
        # by hand: your crab fills much of the middle; its usual red has blue
        # in it, the colourblind vermilion none, and only colourblind mode
        # puts a white symbol on its shell
        check(on[0] > off[0] + 20, f"your crab turns vermilion at once (orange pixels {off[0]} then {on[0]})")
        check(on[1] >= off[1] + 3, f"a symbol shows on your crab's shell (white pixels {off[1]} then {on[1]})")
    colourblind_from_pause(page, False)
    check(page.get_attribute("body", "data-colourblind") == "", "and off again")
    if webgl:
        page.wait_for_timeout(300)
        back = page.evaluate(CRAB_PIXELS)
        check(back[0] < on[0] - 20, f"your crab's colour goes back (orange pixels {back[0]})")
    page.keyboard.press("Escape")
    page.click(by_id("menu.quit"))
    check(screen(page, "title"), "Quit to Title goes to the Title")


def step_touch(page, check):
    # a phone on its side, with touch (Firefox has no mobile mode, so only
    # touch there); a context of its own, closed after
    browser = page.context.browser
    mobile = browser.browser_type.name != "firefox"
    ctx = browser.new_context(viewport={"width": 844, "height": 390}, has_touch=True, is_mobile=mobile)
    errors = []
    other = ctx.new_page()
    other.on("pageerror", lambda e: errors.append(str(e)))
    other.goto(address(page.url.split("?")[0].split("#")[0]))
    other.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
    # the menus fit a phone held upright with no sideways scroll
    other.set_viewport_size({"width": 390, "height": 844})
    wide = []
    for entry, name in [("menu.how", "how"), ("menu.settings", "settings"), ("menu.local", "local"), ("menu.online", "online"), ("menu.notes", "notes")]:
        other.tap(by_id(entry))
        screen(other, name)
        if other.evaluate(WIDE):
            wide.append(name)
        if name == "how":
            keys = other.locator(".panel.how .pairs th").all_inner_texts()
            check("Touch screen" in keys, f"How to play has a touch line {keys}")
        if name == "settings":
            options = other.eval_on_selector(by_id("settings.quality"), "s => [...s.options].map(o => o.textContent)")
            check(options == ["Auto", "High", "Low"], f"Settings offers the quality Auto, High or Low {options}")
            label = other.inner_text("label[for='settings.mouse']")
            check(label == "Look sensitivity", f"on a touch screen the sensitivity is the look's {label!r}")
        other.tap(by_id("menu.back"))
        screen(other, "title")
    check(not wide and not other.evaluate(WIDE), f"no menu scrolls sideways on a phone {wide}")
    check(other.get_attribute("body", "data-touch") == "on", "a tap turns the touch controls on")
    other.set_viewport_size({"width": 844, "height": 390})
    other.tap(by_id("menu.local"))
    screen(other, "local")
    other.tap(by_id("local.start"))
    check(phase(other, "running"), "a Local game starts on a tap")
    other.wait_for_selector("body[data-controls='on']", state="attached", timeout=10000)
    for c in ["touch.stick", "touch.left", "touch.right", "touch.jump", "touch.overhead", "touch.pause"]:
        check(other.is_visible(by_id(c)), f"the {c} control shows")
    boxes = {c: other.locator(by_id(c)).bounding_box() for c in ["touch.left", "touch.right", "touch.jump"]}
    check(all(b["x"] > 844 / 2 and b["y"] > 390 / 2 for b in boxes.values()), f"the claws and the jump sit by the right thumb {boxes}")
    check(boxes["touch.left"]["x"] < boxes["touch.right"]["x"], "the left claw is left of the right claw")
    check(not other.evaluate("() => !!document.pointerLockElement"), "no pointer lock on touch")
    # the stick walks the crab: a thumb down on its zone and pushed up
    zone = other.locator(by_id("touch.stick")).bounding_box()
    x, y = zone["x"] + zone["width"] * 0.4, zone["y"] + zone["height"] * 0.6
    other.evaluate(DRAG, [f"[id='touch.stick']", x, y, [[0, -20], [0, -60], [0, -90]], False])
    try:
        other.wait_for_selector("body[data-moved='true']", state="attached", timeout=10000)
        moved = True
    except Exception:
        moved = False
    other.evaluate(DRAG, [f"[id='touch.stick']", x, y - 90, [], True])
    check(moved, "the thumbstick walks the crab")
    # a drag on the right half turns the camera: no error, and the
    # crosshair stays
    look = other.locator(by_id("touch.look")).bounding_box()
    other.evaluate(DRAG, [f"[id='touch.look']", look["x"] + look["width"] * 0.3, look["y"] + 60, [[20, 0], [60, 10]], True])
    check(other.get_attribute("body", "data-crosshair") == "on", "a drag to look keeps the crosshair")
    # a tap on the overhead view enlarges it, another shrinks it
    other.tap(by_id("touch.overhead"))
    other.wait_for_selector("body:not([data-crosshair='on'])", state="attached", timeout=5000)
    check(other.locator(by_id("touch.overhead")).bounding_box()["height"] > 300, "a tap on the overhead view enlarges it")
    other.tap(by_id("touch.overhead"))
    other.wait_for_selector("body[data-crosshair='on']", state="attached", timeout=5000)
    check(other.locator(by_id("touch.overhead")).bounding_box()["height"] < 200, "another tap shrinks it")
    # held upright: a gentle call to turn the screen; on its side, none
    other.set_viewport_size({"width": 390, "height": 844})
    other.wait_for_selector(".panel.turn", timeout=5000)
    check(other.is_visible(".panel.turn"), "held upright, the game asks to turn the screen")
    other.set_viewport_size({"width": 844, "height": 390})
    other.wait_for_selector(".panel.turn", state="detached", timeout=5000)
    check(True, "on its side, the call goes")
    # the pause button opens the pause menu, and the controls go
    other.tap(by_id("touch.pause"))
    other.wait_for_selector(".panel.pause", timeout=5000)
    check(other.get_attribute("body", "data-controls") != "on", "the pause button opens the menu and the controls go")
    other.tap(by_id("menu.resume"))
    other.wait_for_selector("body[data-controls='on']", state="attached", timeout=5000)
    check(True, "Resume brings the controls back")
    check(not errors, f"no page error on touch {errors}")
    ctx.close()


STEPS = [
    ("boot", step_boot),
    ("fallback", step_fallback),
    ("how", step_how),
    ("settings", step_settings),
    ("notes", step_notes),
    ("local", step_local),
    ("colourblind", step_colourblind),
    ("end", step_end),
    ("centre", step_centre),
    ("online", step_online),
    ("gallery", step_gallery),
    ("sounds", step_sounds),
    ("touch", step_touch),
]


def chosen_steps():
    """The steps this run takes: all, or `boot` and those in BUILDER_STEPS."""
    wanted = os.environ.get("BUILDER_STEPS")
    chosen = None if wanted is None else set(wanted.split()) | {"boot"}
    return [(n, s) for n, s in STEPS if chosen is None or n in chosen]


def run(page, check, engine, report, first=1):
    """Runs the chosen steps; `report` (tests/browser/progress.py) shows each
    one start and end. `first` is the first step's number in the engine's
    count (the check's own load step comes before)."""
    steps = chosen_steps()
    skipped = [n for n, _ in STEPS if n not in dict(steps)]
    if skipped:
        report.note(f"  (skipping {len(skipped)} steps not picked: {', '.join(skipped)})")
    for k, (name, step) in enumerate(steps, first):
        report.step_start(k, name)
        try:
            step(page, check)
        except BaseException:
            report.step_end(error=True)
            raise
        report.step_end()
