// Runs a Renyi program through the web crate's module under node
// (decision AW29, stage 33a): the files given are handed to the module,
// the first being the main file, the program is checked, then run, and
// what it printed comes back on standard output. A run that presents
// frames (decision AW33) is driven as the page drives it: each frame is
// answered with the next move of `--script`, in the showcase game's own
// notation (a key and a frame count: F forward, B backward, L and R
// turn, S fire, Q quit, any other key waits; 64 ms a frame), then with
// `quit` until the run ends; the frames drawn, the shapes of the first
// and the time per frame in the module go to standard error. The exit
// status is the module's: 0 finished, 1 main failed, 2 crashed, 4 the
// program does not check, 5 main needs a capability the page cannot
// serve, or the code of `environment.exit`. CI runs it on the hello
// program of the corpus and on the showcase game's demo and play modes.
//
//   node tools/wasm_check.js <renyi_web.wasm> <main.ry> [<file.ry>...] [--script <moves>] [-- <argument>...]

"use strict";

const fs = require("fs");
const path = require("path");

const SUSPENDED = 6;
const FRAME_MS = 64;
const MOVES = { F: "forward", B: "backward", L: "turn_left", R: "turn_right", S: "fire", Q: "quit" };
// A run that ignores `quit` is given up after this many frames past the script.
const PATIENCE = 1000;

// The keys of one frame for a move's key: one held, or none.
function inputOf(key) {
  const input = { forward: 0, backward: 0, turn_left: 0, turn_right: 0, fire: 0, quit: 0 };
  if (key in MOVES) input[MOVES[key]] = 1;
  return input;
}

// The keys of every frame of a script: `F22` is `F` for 22 frames.
function expanded(script) {
  const keys = [];
  for (const move of script.split(/\s+/).filter(Boolean)) {
    const times = parseInt(move.slice(1), 10) || 1;
    for (let i = 0; i < times; i += 1) keys.push(move[0]);
  }
  return keys;
}

async function main() {
  const [wasmPath, ...rest] = process.argv.slice(2);
  if (!wasmPath || rest.length === 0) {
    process.stderr.write(
      "usage: node tools/wasm_check.js <renyi_web.wasm> <main.ry> [<file.ry>...] [--script <moves>] [-- <argument>...]\n"
    );
    return 64;
  }
  const split = rest.indexOf("--");
  const files = split < 0 ? rest : rest.slice(0, split);
  const args = split < 0 ? [] : rest.slice(split + 1);
  let script = [];
  const option = files.indexOf("--script");
  if (option >= 0) {
    script = expanded(files[option + 1] || "");
    files.splice(option, 2);
  }
  const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath), {
    env: { renyi_host_now_ms: () => Date.now() },
  });
  const exports = instance.exports;
  const memory = exports.memory;
  // a text written into the module's memory; the view is taken after the
  // allocation, which may grow the memory and detach earlier views
  const put = (text) => {
    const bytes = new TextEncoder().encode(text);
    const pointer = exports.renyi_alloc(bytes.length);
    new Uint8Array(memory.buffer, pointer, bytes.length).set(bytes);
    return [pointer, bytes.length];
  };
  const call = (name, text) => {
    const [pointer, length] = put(text);
    const status = exports[name](pointer, length);
    exports.renyi_free(pointer, length);
    return status;
  };
  const read = (pointer, length) =>
    new TextDecoder().decode(new Uint8Array(memory.buffer, pointer, length));
  const answer = () => read(exports.renyi_text(), exports.renyi_text_len());
  const frame = () => read(exports.renyi_frame(), exports.renyi_frame_len());
  for (const file of files) {
    const [name, nameLength] = put(path.basename(file));
    const [text, textLength] = put(fs.readFileSync(file, "utf8"));
    exports.renyi_file(name, nameLength, text, textLength);
    exports.renyi_free(name, nameLength);
    exports.renyi_free(text, textLength);
  }
  const mainName = path.basename(files[0]);
  const checked = call("renyi_check", mainName);
  process.stdout.write(answer());
  if (checked !== 0) {
    return checked;
  }
  for (const argument of args) {
    call("renyi_argument", argument);
  }
  let status = call("renyi_run", mainName);
  process.stdout.write(answer());
  // the frame loop, as the page drives it
  let frames = 0;
  let shapes = 0;
  let inside = 0;
  while (status === SUSPENDED) {
    if (frames === 0) shapes = frame().split("\n").length - 1;
    frames += 1;
    if (frames > script.length + PATIENCE) {
      process.stderr.write(`the run did not end ${PATIENCE} frames after the script\n`);
      return 70;
    }
    const input = inputOf(frames <= script.length ? script[frames - 1] : "Q");
    const before = performance.now();
    status = exports.renyi_resume(input.forward, input.backward, input.turn_left,
      input.turn_right, input.fire, input.quit, FRAME_MS);
    inside += performance.now() - before;
    process.stdout.write(answer());
  }
  if (frames > 0) {
    process.stderr.write(
      `${frames} frames drawn, ${shapes} shapes in the first, `
      + `${(inside / frames).toFixed(2)} ms a frame in the module\n`
    );
  }
  return status === 3 ? exports.renyi_exit_code() : status;
}

main().then(
  (status) => process.exit(status),
  (error) => {
    process.stderr.write(`${error && error.stack ? error.stack : error}\n`);
    process.exit(70);
  }
);
