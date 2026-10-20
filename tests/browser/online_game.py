#!/usr/bin/env python3
"""An online game between two tabs through the game page (Honey Hole's
online_match.py and Ape Race's online_race.py, for Crabble's lobby).

Step 3 of scripts/check-online.sh. Two tabs, each in its own browser
context, load index.html with `?codes&ice=none`, so they connect by pasted
codes on host candidates alone: no relay, no STUN, and every off-origin
request is aborted and counted as a failure. For each engine:

- a code link: a tab hosting by code with `relay=none` (no relay
  contacted) shows the code big; Copy code and Copy link copy the code and
  the page's own address with `#room=CODE` (read from `body[data-copied]`,
  since a headless clipboard may refuse to be read); a new tab opening that
  link goes straight to the Lobby, keeps the code in its address and goes
  back to it after a refresh;
- A hosts by pasted code and shows an invite code; B pastes it, joins and
  shows a reply code; A pastes that back;
- B is seated in A's Lobby: both list both names, A's lists B's ping, the
  seats are Red then Yellow; B is not offered Start, and A's Start waits
  for B's Ready;
- B renames itself and A sees it; A adds an AI crab and B sees it;
- B presses Ready; A's Start comes on; A presses it; both reach Play with
  the clock running (`?fast`: no 3-2-1);
- B holds W and A sees B's crab move (`body[data-remote-moved="true"]`,
  from the core); A holds W and B sees A's;
- B's pause menu does not stop the online game's clock;
- the game (`?short=N`) ends on both; A is offered Rematch, B is not; A's
  Rematch takes both back to the Lobby, B still Ready;
- fails on any off-origin request, WebSocket, CSP violation, console error
  or uncaught page error.

WebKit's tabs connect only with loopback candidates (see online.py,
LOOPBACK_ONLY): the pasted codes are rewritten for it.

    python3 tests/browser/online_game.py

`ONLINE_SHOTS=DIR` also saves each tab's Lobby (DIR/lobby-<engine>-<tab>.png),
in Chromium and Firefox: a WebKit screenshot injects a style the CSP refuses.
"""
import engines
import json
import os
import pathlib
import sys

from playwright.sync_api import sync_playwright

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import online as transport  # noqa: E402  (the helpers of the transport check)

WAIT_MS = transport.WAIT_MS
STRINGS = json.loads((pathlib.Path(__file__).resolve().parents[2] / "data" / "strings.json").read_text())
check = transport.check
# the page's test flags (crates/core/src/app/fast.rs): no 3-2-1 (BUILDER_FAST=0
# for the players' timing), and a game of GAME_S seconds, to reach its end
GAME_S = 20
FLAGS = ("&fast" if os.environ.get("BUILDER_FAST") != "0" else "") + f"&short={GAME_S}"


def by_id(id_):
    return f'[id="{id_}"]'


class Tab:
    def __init__(self, browser, origin, name, seen, url=None):
        self.name = name
        self.context = transport.watched_context(browser, origin, seen)
        self.page = self.context.new_page()
        self.page.set_viewport_size({"width": 1920, "height": 1080})
        tag = f"[{name}] "
        self.page.on("console", lambda m: seen["console"].append(tag + m.text) if m.type == "error" else None)
        self.page.on("pageerror", lambda e: seen["errors"].append(tag + str(e)))
        self.page.on("websocket", lambda ws: seen["sockets"].append(ws.url))
        self.page.goto(url or origin + "/index.html?codes&ice=none" + FLAGS)
        self.page.wait_for_selector("body[data-ready='true']", timeout=WAIT_MS)

    def click(self, id_):
        self.page.click(by_id(id_), timeout=WAIT_MS)

    def code(self, id_):
        sel = by_id(id_)
        self.page.wait_for_function(
            "sel => { const e = document.querySelector(sel); return e && e.value; }",
            arg=sel,
            timeout=WAIT_MS,
        )
        return self.page.input_value(sel)

    def fill(self, id_, text):
        self.page.fill(by_id(id_), text)
        # the field sends its text when it loses the focus
        self.page.evaluate("() => document.activeElement && document.activeElement.blur()")

    def wait(self, selector):
        try:
            self.page.wait_for_selector(selector, state="attached", timeout=WAIT_MS)
            return True
        except Exception:
            return False

    def screen(self, name):
        return self.wait(f"body[data-screen='{name}']")

    def phase(self, name):
        return self.wait(f"body[data-phase='{name}']")

    def enabled(self, id_):
        return self.wait(by_id(id_) + ":not([disabled])")

    def has_text(self, texts):
        try:
            self.page.wait_for_function(
                "want => { const t = document.getElementById('menus').innerText;"
                " return want.every(w => t.includes(w)); }",
                arg=texts,
                timeout=WAIT_MS,
            )
            return True
        except Exception:
            return False

    def clock(self):
        loc = self.page.locator(".panel.clock h2")
        return loc.inner_text() if loc.count() else ""

    def csp(self):
        return self.page.evaluate("() => window.__builderCsp || []")


def room_link(browser, origin, seen):
    """Copy code and Copy link in a code Lobby, and the link joining."""
    query = "?codes&ice=none&relay=none" + FLAGS
    a = Tab(browser, origin, "A", seen, origin + "/index.html" + query)
    a.click("menu.online")
    a.click("online.host")
    check(a.screen("lobby"), "A hosts by code and is in the Lobby")
    a.page.wait_for_selector(by_id("online.copy.code"), timeout=WAIT_MS)
    code = a.page.text_content(by_id("online.copy.code"))
    check(len(code) == 6, f"the code is shown big ({code})")
    a.click("online.copy")
    check(a.page.get_attribute("body", "data-copied") == code, f"Copy code copies the code {code}")
    check(a.has_text([STRINGS["online.copied"]]), "Copy code reads Copied")
    a.click("online.copy_link")
    link = a.page.get_attribute("body", "data-copied") or ""
    want = origin + "/index.html#room=" + code
    check(link == want, f"Copy link copies the page's address with the code ({link})")
    base, _, frag = link.partition("#")
    b = Tab(browser, origin, "B", seen, base + query + "#" + frag)
    check(b.screen("lobby"), "opening the link goes straight to the Lobby")
    check(b.has_text([code]), "the linked Lobby shows the code")
    check(b.page.url.endswith("#room=" + code), "the joiner's address keeps the code")
    check("#" not in a.page.url, "the host's address keeps none")
    b.page.reload()
    b.page.wait_for_selector("body[data-ready='true']", timeout=WAIT_MS)
    check(b.screen("lobby") and b.has_text([code]), "a joiner's refresh goes back to the Lobby")
    csp = a.csp() + b.csp()
    a.context.close()
    b.context.close()
    return csp


def run_engine(p, engine, origin):
    transport.report.note(f"\n[{engine}] {origin}/index.html?codes&ice=none")
    seen = {"foreign": [], "console": [], "errors": [], "sockets": []}
    browser, _ = engines.launch(p, engine)
    paste = transport.loopback if engine in transport.LOOPBACK_ONLY else (lambda c: c)
    link_csp = room_link(browser, origin, seen)
    a = Tab(browser, origin, "A", seen)
    b = Tab(browser, origin, "B", seen)
    tabs = [a, b]

    def finish():
        csp = link_csp + [x for t in tabs for x in t.csp()]
        check(not csp, f"no Content-Security-Policy violation {csp}")
        check(not seen["foreign"], f"no off-origin request at any point {seen['foreign']}")
        check(not seen["sockets"], f"no WebSocket {seen['sockets']}")
        check(not seen["errors"], f"no uncaught page error {seen['errors']}")
        check(not seen["console"], f"no console error {seen['console']}")
        browser.close()

    for t in tabs:
        t.click("menu.online")
    check(a.screen("online") and b.screen("online"), "both tabs open Online game")
    a.fill("online.name", "Hana")
    b.fill("online.name", "Bea")

    a.click("online.host_code")
    check(a.screen("lobby"), "A hosts and is in the Lobby")
    invite = a.code("online.invite")
    check(invite.startswith("O"), "A shows an invite code")
    b.fill("online.invite", paste(invite))
    b.click("online.join_code")
    check(b.screen("lobby"), "B joins and is in the Lobby")
    reply = b.code("online.reply")
    check(reply.startswith("A"), "B shows a reply code")
    a.fill("online.reply", paste(reply))
    a.click("online.connect")

    ok = b.enabled("online.ready")
    check(ok, "B is seated in A's Lobby")
    if not ok:
        return finish()
    check(a.has_text(["Hana (you)", "Bea"]), "A's seats list both players")
    check(b.has_text(["Hana", "Bea (you)"]), "B's seats list both players")
    colours = a.page.locator("table.cells tr.tone td:first-child").all_inner_texts()
    check(colours == ["Red", "Yellow"], f"the seats are Red, then Yellow {colours}")
    check(b.page.locator(by_id("online.start")).count() == 0, "only the host is offered Start")
    check(a.page.is_disabled(by_id("online.start")), "A's Start waits for B's Ready")
    try:
        a.page.wait_for_function(
            "() => [...document.querySelectorAll('table.cells td')].some(td => / ms$/.test(td.textContent))",
            timeout=WAIT_MS,
        )
        pinged = True
    except Exception:
        pinged = False
    check(pinged, "A's Lobby shows B's ping")
    b.fill("online.name", "Bea Moss")
    check(a.has_text(["Bea Moss"]), "B renames itself in the Lobby and A sees it")
    a.page.select_option(by_id("online.ai"), "1")
    check(b.has_text([STRINGS["lobby.ai_crab"]]), "A's AI crab shows in B's Lobby")
    # not in WebKit: its screenshot injects a style the page's CSP refuses
    if os.environ.get("ONLINE_SHOTS") and engine != "webkit":
        out = pathlib.Path(os.environ["ONLINE_SHOTS"])
        out.mkdir(parents=True, exist_ok=True)
        for t in tabs:
            t.page.screenshot(path=str(out / f"lobby-{engine}-{t.name}.png"))
    b.click("online.ready")
    ok = a.enabled("online.start")
    check(ok, "A's Start comes on once B is Ready")
    if not ok:
        return finish()
    a.click("online.start")
    ok = a.screen("play") and b.screen("play")
    check(ok, "both tabs reach Play")
    if not ok:
        return finish()
    check(a.phase("running") and b.phase("running"), "both clocks run")

    for t, driver, other in ((a, b, "B's crab"), (b, a, "A's crab")):
        driver.page.keyboard.down("KeyW")
        moved = t.wait("body[data-remote-moved='true']")
        check(moved, f"{t.name} sees {other} move")
        driver.page.keyboard.up("KeyW")

    # the pause menu over B: the online game's clock runs on
    b.page.keyboard.press("Escape")
    b.page.wait_for_selector(".panel.pause", timeout=WAIT_MS)
    before = b.clock()
    try:
        b.page.wait_for_function(
            "t => { const h = document.querySelector('.panel.clock h2'); return h && h.textContent !== t; }",
            arg=before,
            timeout=WAIT_MS,
        )
    except Exception:
        pass
    after = b.clock()
    check(before != after, f"the online game runs on behind B's menu ({before} -> {after})")

    ok = a.phase("ended") and b.phase("ended")
    check(ok, "the game ends on both tabs")
    if not ok:
        return finish()
    check(a.page.locator(by_id("end.rematch")).count() == 1, "the host is offered Rematch")
    check(b.page.locator(by_id("end.rematch")).count() == 0, "a joiner is not")
    a.click("end.rematch")
    check(a.screen("lobby") and b.screen("lobby"), "Rematch takes both back to the Lobby")
    check(a.has_text(["Bea Moss", STRINGS["lobby.ready"]]) and a.enabled("online.start"), "B is back Ready")
    finish()


def main():
    if not (transport.DIST / "index.html").exists():
        sys.exit("dist/index.html is missing: run scripts/build.sh first")
    server = transport.serve()
    host, port = server.server_address[:2]
    origin = f"http://{host}:{port}"
    with sync_playwright() as p:
        transport.report = transport.progress.Report(transport.ENGINES, ["game"])
        try:
            for i, engine in enumerate(transport.ENGINES, 1):
                transport.report.engine_start(i, engine)
                transport.report.step_start(1, "a game between two tabs")
                run_engine(p, engine, origin)
                transport.report.step_end()
                transport.report.engine_end()
        finally:
            transport.report.close()
    server.shutdown()
    if transport.failures:
        print(f"\nONLINE GAME CHECK FAILED: {len(transport.failures)} problem(s)")
        for f in transport.failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE GAME CHECK PASSED in {', '.join(transport.ENGINES)}")


if __name__ == "__main__":
    main()
