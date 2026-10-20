#!/usr/bin/env python3
"""Stills of the 3D view for the design journal: the built page (dist/) in
headless Chromium (no window ever opens), opened straight onto a model or a
scene through the Gallery's address (crates/core/src/gallery/mod.rs), saved
as a PNG at 1920x1080. After Ape Race's scripts/shot.py. Run
./scripts/build.sh first.

    .venv/bin/python scripts/shot.py OUT.png --model tile --letter Q --view above
    .venv/bin/python scripts/shot.py OUT.png --scene placing
    .venv/bin/python scripts/shot.py OUT.png --model crab --anim walk_side --ms 250 --colour blue

The clock is stopped at --ms (default 0), so a still comes out the same
every time. Any Gallery key can also be passed raw: --query "yaw=135&pitch=20".

A grid of stills in one image: --grid KEY=V1,V2,... makes one column per
value; a second --grid makes the rows. Each cell is labelled.

    .venv/bin/python scripts/shot.py OUT.png --model crab --anim jump --grid ms=0,150,300,450
    .venv/bin/python scripts/shot.py OUT.png --model tile --grid view=model,front,side,above
    .venv/bin/python scripts/shot.py OUT.png --model crab --grid anim=walk_side,walk_fwd --grid ms=0,200,400

--controls keeps the Gallery's panel on screen (by default only the 3D
view shows); --size WxH changes the picture's size.
"""
import argparse
import base64
import functools
import http.server
import pathlib
import sys
import threading
import urllib.parse

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[1]
KEYS = ["model", "scene", "anim", "ms", "view", "yaw", "pitch", "zoom", "colour", "letter", "value", "ink", "face", "alpha"]


class Handler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".json": "application/json",
    }

    def log_message(self, *args):
        pass


def query(a, extra):
    pairs = [("ms", a.ms)]
    for k in KEYS:
        v = getattr(a, k, None)
        if v is not None and k != "ms":
            pairs.append((k, v))
    pairs = [(k, v) for k, v in pairs if k not in extra] + list(extra.items())
    q = urllib.parse.urlencode(pairs, safe=",")
    if a.query:
        q += "&" + a.query
    if not a.controls:
        q += "&bare"
    if not any(k in ("model", "scene") for k, _ in pairs) and "model=" not in a.query and "scene=" not in a.query:
        q += "&gallery"
    return q


def still(page, base, q, path, errors):
    page.goto(f"{base}?{q}")
    page.wait_for_selector("body[data-ready='true']", timeout=60000, state="attached")
    message = page.inner_text("#gallery-message")
    if message:
        errors.append(f"{q}: {message}")
    page.wait_for_selector("body[data-drawn='true']", timeout=60000, state="attached")
    # two more frames, so what is on screen is this address's frame
    page.evaluate("() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))")
    return page.screenshot(path=path) if path else page.screenshot()


def grid_page(cells, cols, w, h):
    imgs = "".join(
        f"<figure><img src='data:image/png;base64,{base64.b64encode(png).decode()}'><figcaption>{label}</figcaption></figure>"
        for label, png in cells
    )
    return (
        "<!doctype html><html><body style='margin:0;background:#0d3550'>"
        f"<div style='display:grid;grid-template-columns:repeat({cols},1fr);gap:8px;padding:8px;width:{w - 16}px'>"
        f"{imgs}</div><style>figure{{margin:0;position:relative}}img{{width:100%;display:block;border-radius:10px}}"
        "figcaption{position:absolute;left:10px;top:8px;padding:2px 10px;border-radius:999px;"
        "background:rgba(13,53,80,.8);color:#f2fbfa;font:700 18px system-ui}</style></body></html>"
    )


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("out")
    for k in KEYS:
        ap.add_argument(f"--{k}", default="0" if k == "ms" else None)
    ap.add_argument("--query", default="", help="more Gallery keys, raw: yaw=135&pitch=20")
    ap.add_argument("--grid", action="append", default=[], help="KEY=V1,V2,...: columns, then rows")
    ap.add_argument("--controls", action="store_true", help="keep the Gallery's panel on screen")
    ap.add_argument("--size", default="1920x1080")
    a = ap.parse_args()
    w, h = (int(x) for x in a.size.split("x"))
    if not (ROOT / "dist" / "index.html").exists():
        sys.exit("no dist/: run ./scripts/build.sh first")
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(ROOT / "dist")))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    base = f"http://127.0.0.1:{server.server_address[1]}/index.html"
    errors = []
    with sync_playwright() as p:
        b = p.chromium.launch(headless=True, args=["--use-angle=swiftshader", "--enable-unsafe-swiftshader"])
        page = b.new_page(viewport={"width": w, "height": h})
        page.on("pageerror", lambda e: errors.append(str(e)))
        page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
        if not a.grid:
            still(page, base, query(a, {}), a.out, errors)
        else:
            axes = [(g.split("=", 1)[0], g.split("=", 1)[1].split(",")) for g in a.grid[:2]]
            cols_key, cols = axes[0]
            rows_key, rows = axes[1] if len(axes) > 1 else (None, [None])
            cells = []
            for r in rows:
                for c in cols:
                    extra = {cols_key: c}
                    if rows_key:
                        extra[rows_key] = r
                    label = " · ".join(f"{k} {v}" for k, v in extra.items())
                    cells.append((label, still(page, base, query(a, extra), None, errors)))
            # as tall as the grid: the full-page picture is cut to it
            sheet = b.new_page(viewport={"width": w, "height": 1})
            sheet.set_content(grid_page(cells, len(cols), w, h))
            sheet.wait_for_load_state()
            sheet.screenshot(path=a.out, full_page=True)
        b.close()
    server.shutdown()
    print(a.out)
    for e in errors:
        print("ERROR:", e)
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
