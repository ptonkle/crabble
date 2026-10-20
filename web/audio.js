// Sounds: plays the WAV files the core renders and decides nothing.
//
// The core (crates/core/src/sound/) reads the recipes in data/sounds.json
// and hands back each sound as complete WAV bytes at a volume and a seed;
// this file decodes and plays them. It holds no frequencies, lengths or
// levels. A sound heard again and again (a grab, a placement, a tally tick)
// varies by the seed the caller passes; the caller gets that from the core.
//
// Browsers start sound only after a click or a key press, so the
// AudioContext starts on the first one; a sound asked for before then is
// dropped, not played late.
//
//   await loadSounds();                  once, after the module's init()
//   playSound("place", volume, seed);    a sound that plays once
//   startLoop("ambience", volume);       a loop, until stopLoop("ambience")
//
// `volume` is a percent; `soundVolume()` is the core's default until a
// setting gives another.
import { sounds, sound } from "./pkg/tool_shim.js";

let file = "";
let info = null;
let ctx = null;
const loops = new Map();

function mark(name, value) {
  document.body.dataset[name] = value;
}

function wake() {
  if (!ctx) {
    const Context = window.AudioContext || window.webkitAudioContext;
    if (!Context) {
      mark("audio", "unavailable");
      return;
    }
    ctx = new Context();
  }
  if (ctx.state === "suspended") ctx.resume();
  mark("audio", "started");
}

// Checks the sounds file through the core and waits for the first click or
// key. Returns the core's answer ({ok: {sounds, loops, volume}} or {error}).
export async function loadSounds() {
  file = await (await fetch("data/sounds.json")).text();
  const result = JSON.parse(sounds(file));
  if (result.error) {
    mark("audio", "refused");
    return result;
  }
  info = result.ok;
  ["pointerdown", "keydown"].forEach((type) => window.addEventListener(type, wake, true));
  mark("audio", "waiting");
  return result;
}

export function soundVolume() {
  return info ? info.volume : 0;
}

async function source(key, volume, seed) {
  if (!ctx || !info) return null;
  const bytes = sound(file, key, volume, seed);
  if (!bytes.length) return null;
  const src = ctx.createBufferSource();
  src.buffer = await ctx.decodeAudioData(bytes.buffer);
  src.connect(ctx.destination);
  return src;
}

export async function playSound(key, volume, seed) {
  const src = await source(key, volume, seed);
  if (!src) return;
  src.start();
  mark("lastSound", key);
}

export async function startLoop(key, volume) {
  if (loops.has(key)) return;
  loops.set(key, null);
  const src = await source(key, volume, 0);
  if (!src || !loops.has(key)) {
    loops.delete(key);
    return;
  }
  src.loop = true;
  src.start();
  loops.set(key, src);
  mark("audioLoops", [...loops.keys()].join(" "));
}

export function stopLoop(key) {
  const src = loops.get(key);
  loops.delete(key);
  if (src) src.stop();
  mark("audioLoops", [...loops.keys()].join(" "));
}
