#!/usr/bin/env python3
"""The frame time of a Local game (four crabs) on a phone-sized view with
touch emulated, at each pixel ratio and Quality setting, with and without
Chromium's CPU slowed four times (a stand-in for a phone's CPU; the GPU is
this machine's, so a phone's fill cost is not measured). F15,
SECOND-ORDER R13.12; numbers in MEASUREMENTS.md. Run ./scripts/build.sh
first.

    .venv/bin/python scripts/touch-frames.py
"""
import functools
import http.server
import pathlib
import sys
import threading

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1] / 'tests' / 'browser'))
import engines
from playwright.sync_api import sync_playwright
ROOT = pathlib.Path(__file__).resolve().parents[1]

class H(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript", ".json": "application/json"}
    def log_message(self, *a): pass
srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(H, directory=str(ROOT / "dist")))
threading.Thread(target=srv.serve_forever, daemon=True).start()
base = f"http://127.0.0.1:{srv.server_address[1]}/index.html?fast"
MEASURE = """async () => {
  const t = []; let last = performance.now();
  await new Promise(res => { const f = (now) => { t.push(now - last); last = now; if (t.length < 240) requestAnimationFrame(f); else res(); }; requestAnimationFrame(f); });
  t.sort((a,b)=>a-b);
  const avg = t.reduce((a,b)=>a+b,0)/t.length;
  return {avg: avg.toFixed(1), p50: t[t.length>>1].toFixed(1), p95: t[Math.floor(t.length*0.95)].toFixed(1)};
}"""
# (CSS width, height, pixel ratio, CPU slowdown, Quality)
cases = [(844, 390, 3, thr, q) for q in ["high", "auto", "low"] for thr in [1, 4]] + [(1920, 1080, 1, 1, "auto")]
with sync_playwright() as p:
    browser, _ = engines.launch(p, "chromium")
    for (w, h, d, thr, quality) in cases:
        ctx = browser.new_context(viewport={"width": w, "height": h}, device_scale_factor=d, has_touch=True, is_mobile=w < 1000)
        page = ctx.new_page()
        page.goto(base)
        page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        page.evaluate(f"() => window.__game.act('settings.quality', '{quality}')")
        cdp = ctx.new_cdp_session(page)
        page.evaluate("() => { document.getElementById('menu.local').click(); }")
        page.evaluate("() => { document.getElementById('local.ai').value='3'; document.getElementById('local.ai').dispatchEvent(new Event('change', {bubbles:true})); document.getElementById('local.start').click(); }")
        page.wait_for_selector("body[data-phase='running']", state="attached", timeout=20000)
        page.wait_for_timeout(1500)
        cdp.send("Emulation.setCPUThrottlingRate", {"rate": thr})
        r = page.evaluate(MEASURE)
        cdp.send("Emulation.setCPUThrottlingRate", {"rate": 1})
        size = page.evaluate("() => [document.getElementById('play').width, document.getElementById('play').height]")
        print(f"{w}x{h} dpr{d} cpu/{thr} quality {quality} buffer {size}: frame avg {r['avg']} ms p50 {r['p50']} p95 {r['p95']}", flush=True)
        ctx.close()
    browser.close()
