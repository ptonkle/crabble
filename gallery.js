// The Gallery's page side: it draws what the core's Gallery returns and
// decides nothing (crates/core/src/gallery/mod.rs). It holds its own core
// object, apart from the menus', so the menus can change round it.
//
// The core describes the controls (selects, toggles and buttons with their
// `gallery.` names); every change goes back through `act`. Each animation
// frame the page hands over the clock, asks for the frame and draws it
// through gl.js. The page's address can open the Gallery straight onto a
// model or a scene (`?model=tile&letter=Q&ms=0`): the core reads the
// query. When a frame with a view has been drawn, <body data-drawn="true">
// tells the screenshot tool (scripts/shot.py) it can take its picture.
import { Gallery } from "./pkg/tool_shim.js?v=4b4df8fe4ed2";
import { makeRenderer } from "./gl.js?v=4b4df8fe4ed2";

const el = (id) => document.getElementById(id);
const make = (tag, text) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  return node;
};
// each fetch is written out whole, so the build stamps its address
const text = async (response) => (await response).text();

export async function makeGallery(onClose) {
  const g = new Gallery();
  const shaders = {
    vertex: await text(fetch("data/shaders/scene.vert?v=4b4df8fe4ed2")),
    fragment: await text(fetch("data/shaders/scene.frag?v=4b4df8fe4ed2")),
    skyVertex: await text(fetch("data/shaders/sky.vert?v=4b4df8fe4ed2")),
    skyFragment: await text(fetch("data/shaders/sky.frag?v=4b4df8fe4ed2")),
  };
  const loaded = JSON.parse(
    g.load(
      await text(fetch("data/models.json?v=4b4df8fe4ed2")),
      await text(fetch("data/glyphs.json?v=4b4df8fe4ed2")),
      await text(fetch("data/scenes.json?v=4b4df8fe4ed2")),
      await text(fetch("data/look.json?v=4b4df8fe4ed2")),
      await text(fetch("data/gallery.json?v=4b4df8fe4ed2")),
    ),
  );
  const canvas = el("scene");
  const renderer = makeRenderer(canvas, JSON.parse(g.numbers()), shaders);
  let running = false;
  let ui = loaded.ok;

  function resize() {
    const [w, h] = g.resize(canvas.clientWidth, canvas.clientHeight, window.devicePixelRatio);
    if (renderer) renderer.resize(w, h);
  }

  function send(name, value) {
    show(JSON.parse(g.act(name, value)));
  }

  function control(item) {
    if (item.kind === "select") {
      const label = make("label");
      label.append(make("span", item.label));
      const select = make("select");
      select.id = item.id;
      item.options.forEach((o) => {
        const option = make("option", o.text);
        option.value = o.value;
        select.append(option);
      });
      select.value = item.value;
      select.addEventListener("change", () => send(item.id, select.value));
      label.append(select);
      return label;
    }
    const button = make("button", item.label);
    button.id = item.id;
    if (item.kind === "toggle") {
      button.setAttribute("aria-pressed", item.pressed ? "true" : "false");
      button.addEventListener("click", () => send(item.id, item.send));
    } else {
      button.addEventListener("click", () => send(item.name, item.value));
    }
    return button;
  }

  function show(result) {
    if (result.error) {
      el("gallery-message").textContent = result.error;
      return;
    }
    ui = result.ok;
    el("gallery-message").textContent = "";
    const panel = el("gallery");
    panel.hidden = !ui.open || ui.bare;
    el("gallery-heading").textContent = ui.heading;
    el("gallery-sub").textContent = ui.sub || "";
    el("gallery-items").replaceChildren(...ui.items.map(control));
    canvas.hidden = !ui.open;
    if (ui.open) {
      document.body.dataset.screen = "gallery";
      resize();
      if (!running) {
        running = true;
        requestAnimationFrame(frame);
      }
    } else if (running) {
      running = false;
      document.body.dataset.drawn = "false";
      onClose();
    }
  }

  function frame(now) {
    if (!running) return;
    g.advance(now);
    g.build_frame();
    const view = g.view();
    if (renderer) renderer.frame(view, g.solid(), g.solid_count(), g.clear(), g.clear_count());
    if (view.length) document.body.dataset.drawn = "true";
    requestAnimationFrame(frame);
  }

  window.addEventListener("resize", () => ui && ui.open && resize());
  if (loaded.error) el("gallery-message").textContent = loaded.error;

  return {
    // from the Title's entry
    open() {
      send("gallery.open", "");
    },
    // from the page's address; the core says whether it opens
    fromAddress(query) {
      send("gallery.query", query);
      return ui && ui.open;
    },
  };
}
