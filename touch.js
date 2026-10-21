// The touch controls on a phone or tablet (F15). The core lays them out
// (`Ui::touch`, crates/core/src/app/touch.rs), moves the stick's ring and
// knob, and turns the touches into the crab's input; this file only puts
// each control where the core says and reports each finger on it through
// `act`, as "down,x,y", "move,x,y" or "up" (CSS pixels). Pointer events,
// not touch events, so a check can make them in every engine.

const make = (tag, cls) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  return node;
};

// a rect from the core: left, top, width, height
const place = (node, [left, top, width, height]) => {
  node.style.left = left + "px";
  node.style.top = top + "px";
  node.style.width = width + "px";
  node.style.height = height + "px";
};

export function makeTouch(box, send) {
  let drawn = "";
  // each control's elements by id, kept while the layout changes (a thumb
  // on the stick stays on it while the overhead view grows)
  const nodes = new Map();
  // each finger down: pointerId -> the control's id
  const held = new Map();
  const at = (e) => e.clientX + "," + e.clientY;

  function create(c) {
    const node = make(c.kind === "button" ? "button" : "div", "touch-" + c.kind);
    node.id = c.id;
    if (c.kind === "button") node.type = "button";
    const rec = { node, ring: null, knob: null };
    if (c.ring) {
      rec.ring = make("div", "touch-ring");
      rec.knob = make("div", "touch-knob");
      box.append(rec.ring, rec.knob);
    }
    node.addEventListener("pointerdown", (e) => {
      e.preventDefault();
      try {
        node.setPointerCapture(e.pointerId);
      } catch (err) {
        // a made-up pointer (the checks) cannot be captured
      }
      held.set(e.pointerId, c.id);
      node.classList.add("pressed");
      send(c.id, "down," + at(e));
    });
    node.addEventListener("pointermove", (e) => {
      if (held.get(e.pointerId) !== c.id) return;
      e.preventDefault();
      send(c.id, "move," + at(e));
    });
    const up = (e) => {
      if (held.get(e.pointerId) !== c.id) return;
      held.delete(e.pointerId);
      node.classList.remove("pressed");
      send(c.id, "up");
    };
    node.addEventListener("pointerup", up);
    node.addEventListener("pointercancel", up);
    node.addEventListener("contextmenu", (e) => e.preventDefault());
    box.append(node);
    nodes.set(c.id, rec);
    return rec;
  }

  function remove(id) {
    const rec = nodes.get(id);
    [rec.node, rec.ring, rec.knob].forEach((n) => n && n.remove());
    nodes.delete(id);
    [...held.entries()].filter(([, h]) => h === id).forEach(([p]) => held.delete(p));
  }

  return {
    // the core's controls, or none: changed only where they changed
    draw(touch) {
      const list = touch ? touch.controls : [];
      const text = JSON.stringify(list);
      if (text === drawn) return;
      drawn = text;
      const want = new Set(list.map((c) => c.id));
      [...nodes.keys()].filter((id) => !want.has(id)).forEach(remove);
      list.forEach((c) => {
        const rec = nodes.get(c.id) || create(c);
        rec.node.setAttribute("aria-label", c.label);
        if (c.kind === "button" && rec.node.textContent !== c.label) rec.node.textContent = c.label;
        rec.node.classList.toggle("on", c.on === true);
        place(rec.node, c.rect);
        if (rec.ring) place(rec.ring, c.ring);
        if (rec.knob) place(rec.knob, c.knob);
      });
    },
  };
}
