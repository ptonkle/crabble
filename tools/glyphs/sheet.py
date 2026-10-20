#!/usr/bin/env python3
"""Draws every glyph of data/glyphs.json on one sheet, to check them by eye.

    .venv/bin/python tools/glyphs/sheet.py OUT.png [--pieces]

Each glyph is drawn as the polygons the file holds, flat, in tile ink on
tile wood; --pieces gives each piece its own colour, to see how a glyph is
built. Headless Chromium through Playwright; no window opens. The tiles in
the 3D view are checked with scripts/shot.py (scene tiles-a-to-z).
"""
import argparse
import json
import pathlib

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
COLOURS = ["#c0392b", "#2471a3", "#229954", "#b7950b", "#7d3c98", "#d35400", "#17a589", "#566573"]


def svg(key, g, height, pieces):
    w = g["width"]
    polys = []
    for k, p in enumerate(g["polys"]):
        pts = " ".join(f"{x},{height - y}" for x, y in p)
        fill = COLOURS[k % len(COLOURS)] if pieces else "#2c1e16"
        opacity = ' fill-opacity="0.75"' if pieces else ""
        polys.append(f'<polygon points="{pts}" fill="{fill}"{opacity}/>')
    pad = 14
    box = f"{-pad} {-pad} {w + 2 * pad} {height + 2 * pad}"
    return (
        f'<figure><svg viewBox="{box}" height="150"><rect x="{-pad}" y="{-pad}" width="{w + 2 * pad}" '
        f'height="{height + 2 * pad}" rx="12" fill="#e8c88f"/>{"".join(polys)}</svg>'
        f"<figcaption>{key} &middot; {len(g['polys'])}</figcaption></figure>"
    )


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("out")
    ap.add_argument("--pieces", action="store_true")
    a = ap.parse_args()
    font = json.loads((ROOT / "data" / "glyphs.json").read_text())
    cells = "".join(svg(k, g, font["height"], a.pieces) for k, g in font["glyphs"].items())
    html = (
        "<!doctype html><html><body style='margin:0;background:#0d3550;font:600 16px system-ui;color:#f2fbfa'>"
        "<div style='display:flex;flex-wrap:wrap;gap:14px;padding:20px'>"
        f"{cells}</div><style>figure{{margin:0;text-align:center}}</style></body></html>"
    )
    with sync_playwright() as p:
        b = p.chromium.launch()
        page = b.new_page(viewport={"width": 1920, "height": 1080})
        page.set_content(html)
        page.screenshot(path=a.out, full_page=True)
        b.close()
    print(a.out)


if __name__ == "__main__":
    main()
