#!/usr/bin/env python3
"""Phone and tablet stills of every screen, with touch emulated, for the
design journal and for checking that nothing scrolls sideways (F15). The
built page (dist/) in Chromium; saved as notes/shots/touch/<device>-<screen>.png.
Run ./scripts/build.sh first.

    .venv/bin/python scripts/touch-shots.py [--out DIR] [--only phone-landscape]

Each still's page width is checked against the viewport: a screen wider
than the phone is printed as WIDE.
"""
import argparse
import functools
import http.server
import pathlib
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[1]

# CSS size, pixel ratio (1, so the stills stay small in the repository;
# the controls and menus lay out the same); the phone is a 390 x 844
# handset, the tablet an 820 x 1180 one
DEVICES = {
    "phone-portrait": (390, 844, 1),
    "phone-landscape": (844, 390, 1),
    "tablet-portrait": (820, 1180, 1),
    "tablet-landscape": (1180, 820, 1),
}


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
    }

    def log_message(self, *args):
        pass


def serve():
    handler = functools.partial(Handler, directory=str(ROOT / "dist"))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


WIDE = "() => document.documentElement.scrollWidth > window.innerWidth + 1"


def screen(page, name):
    page.wait_for_selector(f"body[data-screen='{name}']", state="attached", timeout=10000)
    page.wait_for_timeout(400)


def click(page, action):
    page.tap(f"[id='{action}']")


def shots(page, base, out, device):
    taken = []

    def shot(name):
        path = out / f"{device}-{name}.png"
        page.screenshot(path=str(path))
        wide = page.evaluate(WIDE)
        taken.append((name, wide))
        print(f"  {path}{'  WIDE' if wide else ''}")

    def open_page(*flags):
        page.goto(base + "/index.html?" + "&".join(("fast",) + flags))
        page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        page.wait_for_timeout(600)

    open_page()
    shot("title")
    for entry, name in [("menu.how", "how"), ("menu.settings", "settings"), ("menu.local", "local"), ("menu.online", "online"), ("menu.notes", "notes")]:
        click(page, entry)
        screen(page, name)
        shot(name)
        page.evaluate("() => window.__game && document.getElementById('menu.back') && document.getElementById('menu.back').click()")
        screen(page, "title")
    # the lobby by pasted codes (no relay is reached)
    open_page("codes")
    click(page, "menu.online")
    screen(page, "online")
    if page.locator("[id='online.host']").count():
        click(page, "online.host")
        try:
            screen(page, "lobby")
            page.wait_for_timeout(800)
            shot("lobby")
        except Exception as e:
            print("  no lobby:", e)
    # a game: the touch controls over the view
    open_page()
    click(page, "menu.local")
    screen(page, "local")
    page.select_option("[id='local.ai']", "3")
    click(page, "local.start")
    screen(page, "play")
    page.wait_for_selector("body[data-touch='on']", state="attached", timeout=10000)
    page.wait_for_timeout(1500)
    shot("play")
    click(page, "touch.overhead")
    page.wait_for_timeout(600)
    shot("play-overhead")
    click(page, "touch.overhead")
    click(page, "touch.pause")
    page.wait_for_selector(".panel.pause", timeout=5000)
    shot("pause")
    # the tally, from a short game
    open_page("short")
    click(page, "menu.local")
    screen(page, "local")
    page.select_option("[id='local.ai']", "3")
    click(page, "local.start")
    page.wait_for_selector("body[data-phase='ended']", state="attached", timeout=30000)
    page.wait_for_selector(".panel.end .tally-col.winner", timeout=40000)
    page.wait_for_timeout(500)
    shot("tally")
    return taken


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", default=str(ROOT / "notes" / "shots" / "touch"))
    ap.add_argument("--only", action="append")
    a = ap.parse_args()
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    server = serve()
    base = f"http://127.0.0.1:{server.server_address[1]}"
    wide = []
    with sync_playwright() as p:
        browser = p.chromium.launch()
        for device, (w, h, dpr) in DEVICES.items():
            if a.only and device not in a.only:
                continue
            print(device)
            ctx = browser.new_context(viewport={"width": w, "height": h}, device_scale_factor=dpr, is_mobile=True, has_touch=True)
            page = ctx.new_page()
            wide += [f"{device}-{n}" for n, wd in shots(page, base, out, device) if wd]
            ctx.close()
        browser.close()
    server.shutdown()
    print("WIDE: " + ", ".join(wide) if wide else "no screen scrolls sideways")


if __name__ == "__main__":
    main()
