// The page draws what the core returns and decides nothing.
//
// The core describes the screen (its name, panels and their items), and this
// file builds the DOM from that description without knowing what any panel
// means. Every control sends its action name and value back through `act`,
// the core's one door for buttons, settings, keys and screens; the core
// answers with the next view model. No state lives here: what is drawn is
// what the core last returned. (Honey Hole's page, with its 2D layer and
// settings store left out.) The game's 3D view is drawn through gl.js
// from the frames the core builds (crates/core/src/app/view3d.rs). The
// Gallery has its own core object and page module (web/gallery.js); the
// menus open it and it hands back the Title.
import init, { Game } from "./pkg/tool_shim.js";
import { makeLayeredRenderer } from "./gl.js";
import { startNet } from "./net.js";
import { makeGallery } from "./gallery.js";
import { loadSounds } from "./audio.js";
import { loadSettings, saveSettings } from "./settings-store.js";

const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
};

let game = null;
let ui = null;
let panelsText = "";
// the view model's text as last drawn: an unchanged one is not drawn again
let uiText = "";
// the Gallery (web/gallery.js), and whether it is showing
let gallery = null;
let galleryOpen = false;
// the game's renderer (null without WebGL 2)
let renderer = null;

// ---- talking to the core ----

// What each menu element does, by its id: { click, change } handlers, made
// afresh with each build. The menus listen once, on their box, and look the
// target up here, so an element kept across a rebuild (drawMenus patches the
// old tree to match the new one) does the latest thing.
let handlers = {};

function on(id, type, fn) {
  if (!handlers[id]) handlers[id] = {};
  handlers[id][type] = fn;
}

function dispatch(e) {
  for (let t = e.target; t && t.id !== "menus"; t = t.parentElement) {
    const h = t.id && handlers[t.id] && handlers[t.id][e.type];
    if (h) {
      h(t, e);
      return;
    }
  }
}

function send(name, value) {
  draw(JSON.parse(game.act(name, value)));
}

// ---- items, one builder per type ----

function button(item) {
  const b = make("button", item.quiet ? "button quiet" : "button", item.text);
  b.type = "button";
  b.id = item.id;
  b.disabled = item.disabled === true;
  if (item.badge) b.append(make("span", "badge", item.badge));
  on(item.id, "click", () => send(item.id, ""));
  return b;
}

function slider(item) {
  const wrap = make("div", "field slider");
  const label = make("label", "", item.label);
  const input = make("input");
  input.type = "range";
  input.id = item.id;
  input.setAttribute("min", item.min);
  input.setAttribute("max", item.max);
  input.setAttribute("step", item.step);
  input.value = item.value;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  const shown = make("span", "slider-value", item.text);
  shown.id = item.id + ".text";
  wrap.append(label, input, shown);
  return wrap;
}

function toggle(item) {
  const wrap = make("label", "field toggle");
  const input = make("input");
  input.type = "checkbox";
  input.id = item.id;
  input.checked = item.value;
  on(item.id, "change", (t) => send(item.id, t.checked ? "on" : "off"));
  wrap.append(input, make("span", "", item.label));
  return wrap;
}

function select(item) {
  const wrap = make("div", "field");
  const label = make("label", "", item.label);
  const input = make("select");
  item.options.forEach((o) => {
    const opt = make("option", "", o.text);
    opt.value = o.value;
    opt.disabled = o.disabled === true;
    input.append(opt);
  });
  input.value = item.value;
  input.id = item.id;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  wrap.append(label, input);
  return wrap;
}

function text(item) {
  const wrap = make("div", "field text");
  const label = make("label", "", item.label);
  const input = make("input");
  input.type = "text";
  input.id = item.id;
  input.value = item.value;
  input.maxLength = item.max;
  input.spellcheck = false;
  input.autocomplete = "off";
  if (item.placeholder) input.placeholder = item.placeholder;
  label.htmlFor = item.id;
  on(item.id, "change", (t) => send(item.id, t.value));
  wrap.append(label, input);
  return wrap;
}

function lines(item) {
  const box = make("ul", "lines");
  item.lines.forEach((l) => box.append(make("li", "", l)));
  return box;
}

function list(item) {
  const table = make("table", "pairs");
  item.rows.forEach((r) => {
    const tr = make("tr");
    tr.append(make("th", "", r[0]), make("td", "", r[1]));
    table.append(tr);
  });
  return table;
}

function table(item) {
  const box = make("table", "cells");
  const head = make("tr");
  item.head.forEach((h) => head.append(make("th", "", h)));
  box.append(head);
  item.rows.forEach((r) => {
    const tr = make("tr", (r.marked ? "marked" : "") + (r.tone ? " tone tone-" + r.tone : ""));
    r.cells.forEach((c) => tr.append(make("td", "", c)));
    box.append(tr);
  });
  return box;
}

// Each version, its date and its lines (a minor version drawn smaller).
function notes(item) {
  const box = make("div", "notes");
  box.id = item.id;
  item.entries.forEach((e) => {
    const sec = make("section", e.minor ? "note minor" : "note");
    const head = make("h3", "note-head");
    head.append(make("span", "note-version", e.version), make("span", "note-date", e.date));
    const ul = make("ul");
    e.lines.forEach((l) => ul.append(make("li", "", l)));
    sec.append(head, ul);
    box.append(sec);
  });
  return box;
}

// Puts `text` on the clipboard. Without the clipboard (an insecure page, a
// refusal), the text is copied through a scratch field; if that fails too,
// the shown code, if any, is left selected for the reader to copy.
function clip(text, shown) {
  const fallback = () => {
    const box = make("textarea", "copy-scratch");
    box.value = text;
    box.readOnly = true;
    document.body.append(box);
    box.select();
    let done = false;
    try {
      done = document.execCommand("copy");
    } catch (e) {
      // the selection below is the last resort
    }
    box.remove();
    const again = shown && el(shown.id);
    if (!done && again) window.getSelection().selectAllChildren(again);
  };
  if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).catch(fallback);
  else fallback();
}

// A Copy button, and the text it copies shown big beside it when the core
// gives one (the code). The core says what to copy; the clipboard is the
// page's job. `body[data-copied]` holds what was copied, for the online
// check (a headless clipboard may refuse to be read).
function copy(item) {
  const b = make("button", "button quiet", item.text);
  b.type = "button";
  b.id = item.id;
  b.disabled = item.disabled === true;
  const wrap = make("div", "field copy-field");
  const code = item.value ? make("span", "copy-code", item.value) : null;
  on(item.id, "click", () => {
    document.body.dataset.copied = item.copy;
    clip(item.copy, code);
    send(item.id, "");
  });
  if (!code) return b;
  code.id = item.id + ".code";
  wrap.append(make("label", "", item.label), code, b);
  return wrap;
}

const builders = { button, slider, toggle, select, text, lines, list, table, notes, copy };

// The panels as the core gives them, built as a fresh tree off the page.
function buildMenus() {
  handlers = {};
  const box = make("div");
  (ui.panels || []).forEach((p) => {
    const panel = make("section", "panel " + p.kind);
    if (p.heading) {
      const h = make("h2", "", p.heading);
      if (p.click) {
        h.id = p.click;
        on(p.click, "click", () => send(p.click, ""));
      }
      panel.append(h);
    }
    if (p.sub) panel.append(make("p", "sub", p.sub));
    p.items.forEach((item) => panel.append(builders[item.type](item)));
    box.append(panel);
  });
  return box;
}

// ---- patching: keep every element that is still there ----

// Two nodes are the same element if they have the same tag and id.
function same(a, b) {
  return a.nodeType === b.nodeType && a.nodeName === b.nodeName && (a.nodeType !== Node.ELEMENT_NODE || a.id === b.id);
}

// Makes `old` (on the page) match `fresh` (just built), keeping each element
// that is still there: a rebuilt element under the mouse between press and
// release loses the click, a rebuilt select closes while open, and a rebuilt
// control loses the focus.
function patch(old, fresh) {
  if (old.nodeType !== Node.ELEMENT_NODE) {
    if (old.nodeValue !== fresh.nodeValue) old.nodeValue = fresh.nodeValue;
    return;
  }
  [...old.attributes].forEach((a) => {
    if (!fresh.hasAttribute(a.name)) old.removeAttribute(a.name);
  });
  [...fresh.attributes].forEach((a) => {
    if (old.getAttribute(a.name) !== a.value) old.setAttribute(a.name, a.value);
  });
  patchChildren(old, fresh);
  // the live state, after the children (a select's value needs its options)
  if (old.tagName === "INPUT" || old.tagName === "SELECT" || old.tagName === "TEXTAREA") {
    // a field being typed in keeps its text until it sends it (the Lobby
    // redraws many times a second)
    if (old.value !== fresh.value && old !== editing) old.value = fresh.value;
    if (old.checked !== fresh.checked) old.checked = fresh.checked;
  }
  if (old.disabled !== undefined && old.disabled !== fresh.disabled) old.disabled = fresh.disabled;
}

function patchChildren(old, fresh) {
  const want = [...fresh.childNodes];
  want.forEach((f, i) => {
    const o = old.childNodes[i];
    if (!o) old.append(f);
    else if (same(o, f)) patch(o, f);
    else old.replaceChild(f, o);
  });
  while (old.childNodes.length > want.length) old.lastChild.remove();
}

// An open dropdown: the browser closes a select's list when the page around
// it changes, so the menus wait while one is open, and catch up when it
// closes.
let openSelect = null;

// The text field typed in since it last sent its text, if any.
let editing = null;

function selectClosed() {
  if (!openSelect) return;
  openSelect = null;
  drawMenus();
}

function watchSelects() {
  document.addEventListener("pointerdown", (e) => {
    const s = e.target.closest && e.target.closest("#menus select");
    if (s && s !== openSelect) openSelect = s;
    else selectClosed();
  }, true);
  // after the change is sent (a redraw first would put back the old value)
  document.addEventListener("change", () => setTimeout(selectClosed, 0), true);
  document.addEventListener("focusout", (e) => {
    if (e.target === openSelect) selectClosed();
  }, true);
  document.addEventListener("keydown", (e) => {
    if (!openSelect) {
      if (e.target.matches && e.target.matches("#menus select") && (e.key === " " || e.key === "ArrowDown" && e.altKey)) openSelect = e.target;
    } else if (e.key === "Escape" || e.key === "Enter" || e.key === "Tab") {
      setTimeout(selectClosed, 0);
    }
  }, true);
}

function drawMenus() {
  if (openSelect) {
    if (openSelect.isConnected) return;
    openSelect = null;
  }
  const text = JSON.stringify(ui.panels || []);
  if (text === panelsText) return;
  panelsText = text;
  const menus = el("menus");
  const kept = document.activeElement;
  const focused = kept && menus.contains(kept) ? kept.id : "";
  const caret = focused && kept.selectionStart !== undefined ? kept.selectionStart : null;
  patchChildren(menus, buildMenus());
  // an element that had to be replaced gets the focus back (and a text
  // field its caret); when it is gone (a new screen), the screen's first
  // control takes the focus, so the keyboard carries on from there (not
  // while the core captures the keys)
  if (!focused || kept.isConnected || ui.capture) return;
  const again = el(focused) || menus.querySelector("button, input, select");
  if (!again) return;
  again.focus();
  if (caret !== null && again.setSelectionRange && again.type === "text") again.setSelectionRange(caret, caret);
}

// ---- drawing the view model ----

// A data attribute, written only when it changes.
function mark(name, value) {
  const text = String(value);
  if (document.body.dataset[name] !== text) document.body.dataset[name] = text;
}

function draw(result) {
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  ui = result.ok;
  // the settings as the core wrote them, kept for the next visit
  saveSettings(ui.settings);
  // drawn from an action's answer: the next frame's text is drawn afresh
  uiText = "";
  document.title = ui.title;
  mark("screen", ui.screen);
  mark("phase", ui.phase);
  mark("moved", ui.moved);
  mark("capture", ui.capture ? "on" : "");
  mark("scene", ui.scene ? "on" : "");
  mark("crosshair", ui.crosshair ? "on" : "");
  // the menu is up or the game is over: the pointer goes back to the reader
  if (!ui.capture && document.pointerLockElement) document.exitPointerLock();
  const version = el("version");
  if (version.textContent !== ui.version) version.textContent = ui.version;
  version.disabled = !ui.version_opens;
  el("message").textContent = "";
  drawMenus();
  if (ui.scene) resize();
  // the Title's Gallery entry: the Gallery draws itself
  if (ui.screen === "gallery" && gallery && !galleryOpen) {
    galleryOpen = true;
    gallery.open();
  }
}

// ---- input ----

// A key typed into a text control belongs to that control.
function typing(e) {
  const t = e.target;
  return t.tagName === "SELECT" || t.tagName === "TEXTAREA" || (t.tagName === "INPUT" && t.type === "text");
}

// Up and Down move the focus between a menu's controls, as Tab and
// Shift+Tab do, while a button or a toggle has it.
function arrowFocus(e) {
  const t = e.target;
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
  if (!(t.tagName === "BUTTON" || (t.tagName === "INPUT" && t.type === "checkbox"))) return;
  const all = [...el("menus").querySelectorAll("button:not([disabled]), input:not([disabled]), select:not([disabled])")];
  const next = all[all.indexOf(t) + (e.key === "ArrowDown" ? 1 : -1)];
  if (!next) return;
  e.preventDefault();
  next.focus();
}

function bindInput() {
  ["click", "change"].forEach((type) => el("menus").addEventListener(type, dispatch));
  el("version").addEventListener("click", () => send("version", ""));
  // every key goes to the core, which decides what it does; while the core
  // captures input (a game), the browser does nothing with it (Tab, Space)
  window.addEventListener("keydown", (e) => {
    if (ui && ui.capture) e.preventDefault();
    if (e.repeat || typing(e)) return;
    send("key", e.code);
  });
  window.addEventListener("keyup", (e) => {
    if (typing(e)) return;
    send("key.up", e.code);
  });
  window.addEventListener("blur", () => send("keys.release", ""));
  const scene = el("play");
  // the mouse turns the camera while the pointer is locked to the view
  window.addEventListener("mousemove", (e) => {
    if (ui && ui.capture && document.pointerLockElement === scene) send("mouse.move", e.movementX + "," + e.movementY);
  });
  // a click locks the pointer; once locked, each button goes to the core
  // (the claws)
  window.addEventListener("mousedown", (e) => {
    if (ui && ui.capture && document.pointerLockElement === scene) send("mouse.down", String(e.button));
  });
  scene.addEventListener("mousedown", () => {
    if (ui && ui.capture && document.pointerLockElement !== scene && scene.requestPointerLock) scene.requestPointerLock();
  });
  scene.addEventListener("contextmenu", (e) => {
    if (ui && ui.capture) e.preventDefault();
  });
  // the browser let the pointer go (its own Escape): the core opens the menu
  document.addEventListener("pointerlockchange", () => {
    if (!document.pointerLockElement && ui && ui.capture) send("pointer.released", "");
  });
  watchSelects();
  el("menus").addEventListener("keydown", arrowFocus);
  document.addEventListener("input", (e) => {
    if (e.target.matches && e.target.matches("#menus input[type=text]")) editing = e.target;
  }, true);
  // before the change is sent, so the core's answer is drawn into it
  document.addEventListener("change", (e) => {
    if (e.target === editing) editing = null;
  }, true);
}

// ---- the 3D view ----

// The canvas's size goes to the core, which says how many pixels to draw.
function resize() {
  const canvas = el("play");
  const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
  if (renderer && (canvas.width !== w || canvas.height !== h)) renderer.resize(w, h);
}

// The core's frame: the static layers whose signature changed, then each
// pass (the main view, the overhead view).
function drawFrame() {
  game.build_frame();
  const info = JSON.parse(game.frame_info());
  renderer.layers(info.layers, (k) => game.layer(k));
  if (!info.passes.length) renderer.blank();
  info.passes.forEach((p, k) =>
    renderer.pass({
      viewport: p.viewport,
      layers: p.layers,
      view: game.pass_view(k),
      solid: game.pass_solid(k),
      solidCount: game.pass_solid_count(k),
      clear: game.pass_clear(k),
      clearCount: game.pass_clear_count(k),
    }),
  );
  mark("drawn", info.passes.length ? "true" : "false");
}

// Each frame: the 3D view when the core says there is one, and the view
// model again while it is live (a game's clock moves). The clock goes to
// the core through web/net.js's pump.
function frame() {
  if (ui && ui.scene && renderer) drawFrame();
  if (ui && ui.live) {
    const text = game.ui();
    if (text !== uiText) {
      draw(JSON.parse(text));
      uiText = text;
    }
  }
  requestAnimationFrame(frame);
}

async function main() {
  await init();
  const strings = await (await fetch("data/strings.json")).text();
  const notes = await (await fetch("data/patch-notes.json")).text();
  game = new Game(strings, notes);
  // the settings from an earlier visit, checked by the core, before the
  // first draw writes the defaults back
  loadSettings(game);
  bindInput();
  draw(JSON.parse(game.ui()));
  // the game's files, each checked by the core (refused by name)
  const files = [
    ["data.tuning", await (await fetch("data/tuning.json")).text()],
    ["data.sounds", await (await fetch("data/sounds.json")).text()],
    ["data.models", await (await fetch("data/models.json")).text()],
    ["data.glyphs", await (await fetch("data/glyphs.json")).text()],
    ["data.look", await (await fetch("data/look.json")).text()],
    ["data.map", await (await fetch("data/scenes/seafloor.json")).text()],
  ];
  files.forEach(([name, text]) => {
    const result = JSON.parse(game.act(name, text));
    if (result.error) el("message").textContent = result.error;
  });
  renderer = makeLayeredRenderer(el("play"), JSON.parse(game.numbers()), {
    vertex: await (await fetch("data/shaders/scene.vert")).text(),
    fragment: await (await fetch("data/shaders/scene.frag")).text(),
    skyVertex: await (await fetch("data/shaders/sky.vert")).text(),
    skyFragment: await (await fetch("data/shaders/sky.frag")).text(),
  });
  // this page's seed and id, for codes and games: the core draws no random
  // number of its own
  send("flag.seed", String(crypto.getRandomValues(new Uint32Array(1))[0]));
  send("flag.client", crypto.randomUUID());
  const query = new URLSearchParams(location.search);
  // `?fast`: the browser checks' fast mode (no countdown), and `?short` (or
  // `?short=N` seconds): a short game (crates/core/src/app/fast.rs); never set for players
  ["fast", "short"].forEach((f) => {
    if (query.has(f)) send("flag." + f, query.get(f) || "on");
  });
  // `?codes`: the pasted-code path to online play, for the automated
  // online check (scripts/check-online.sh); players use codes and links
  if (query.has("codes")) send("flag.codes", "on");
  // invite links: the page's own address, and the lobby to join when opened
  // from one (`#room=CODE`; the core keeps it in a joiner's address)
  send("flag.page_url", location.origin + location.pathname);
  const linked = new URLSearchParams(location.hash.slice(1)).get("room");
  if (linked) send("flag.room_link", linked);
  startNet(game, draw);
  // the sounds the core names (web/audio.js; they start on the first click
  // or key)
  await loadSounds();
  // the Gallery: back from it goes to the Title; the page's address may
  // open it straight onto a model or a scene
  gallery = await makeGallery(() => {
    galleryOpen = false;
    send("menu.back", "");
  });
  if (gallery.fromAddress(location.search)) {
    galleryOpen = true;
    send("menu.gallery", "");
  }
  window.addEventListener("resize", () => ui && ui.scene && resize());
  requestAnimationFrame(frame);
  document.body.dataset.ready = "true";
}

main();
