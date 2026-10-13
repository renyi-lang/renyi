// The play page of the site (decision AW33, stage 33d). The toolchain's
// module for WebAssembly (crates/renyi_web, built by the Pages workflow
// and put beside the page by tools/site.py) checks and runs programs in
// the browser. The showcase game's sources are compiled here and played
// on the canvas: the program hands each frame to the page through
// screen.present and suspends, the page draws the shapes and resumes it
// with the keys held and the time elapsed. The playground runs what is
// written in it the same way, and a program of its own that presents
// frames plays on the same canvas. Nothing leaves the page: the module,
// the game's sources and this script are files of the site.
// tools/wasm_check.js drives the same module under node.

(() => {
  "use strict";

  const canvas = document.getElementById("game");
  const startButton = document.getElementById("game-start");
  const gameStatus = document.getElementById("game-status");
  const rate = document.getElementById("game-rate");
  const editor = document.getElementById("editor");
  const checkButton = document.getElementById("check");
  const runButton = document.getElementById("run");
  const argumentsField = document.getElementById("arguments");
  const output = document.getElementById("output");
  const runStatus = document.getElementById("run-status");
  if (!canvas || !startButton || !editor) return;

  // The statuses of the module's C ABI (crates/renyi_web/src/lib.rs).
  const FINISHED = 0;
  const EXITED = 3;
  const SUSPENDED = 6;
  const ENDINGS = ["finished", "main failed", "crashed", "exited", "does not check",
    "refused: main needs what the page cannot serve", "suspended"];
  // The logical screen of std.screen, drawn at the canvas's scale.
  const WIDTH = 320;
  const HEIGHT = 200;
  const SCALE = canvas.width / WIDTH;
  // A frame after a pause (the tab in the background) counts as this long.
  const LONGEST_FRAME_MS = 100;
  // The keys held, by the key of the event (a letter in lower case).
  const KEYS = {
    ArrowUp: "forward", w: "forward", ArrowDown: "backward", s: "backward",
    ArrowLeft: "turn_left", a: "turn_left", ArrowRight: "turn_right", d: "turn_right",
    " ": "fire", f: "fire", Escape: "quit", q: "quit",
  };
  const GAME = "assets/raycaster/";
  const context = canvas.getContext("2d");

  // The module behind a few functions. Every text crosses as bytes in
  // the module's memory; a view is taken after each allocation, which
  // may grow the memory and detach earlier views.
  class Toolchain {
    constructor(instance) {
      this.exports = instance.exports;
      this.encoder = new TextEncoder();
      this.decoder = new TextDecoder();
    }

    put(text) {
      const bytes = this.encoder.encode(text);
      const pointer = this.exports.renyi_alloc(bytes.length);
      new Uint8Array(this.exports.memory.buffer, pointer, bytes.length).set(bytes);
      return [pointer, bytes.length];
    }

    call(name, text) {
      const [pointer, length] = this.put(text);
      const result = this.exports[name](pointer, length);
      this.exports.renyi_free(pointer, length);
      return result;
    }

    read(pointer, length) {
      return this.decoder.decode(new Uint8Array(this.exports.memory.buffer, pointer, length));
    }

    file(name, text) {
      const [namePointer, nameLength] = this.put(name);
      const [textPointer, textLength] = this.put(text);
      this.exports.renyi_file(namePointer, nameLength, textPointer, textLength);
      this.exports.renyi_free(namePointer, nameLength);
      this.exports.renyi_free(textPointer, textLength);
    }

    clear() { this.exports.renyi_clear(); }
    argument(text) { this.call("renyi_argument", text); }
    check(name) { return this.call("renyi_check", name); }
    run(name) { return this.call("renyi_run", name); }

    resume(keys, elapsed) {
      return this.exports.renyi_resume(+keys.forward, +keys.backward, +keys.turn_left,
        +keys.turn_right, +keys.fire, +keys.quit, elapsed);
    }

    text() { return this.read(this.exports.renyi_text(), this.exports.renyi_text_len()); }
    frame() { return this.read(this.exports.renyi_frame(), this.exports.renyi_frame_len()); }
    exitCode() { return this.exports.renyi_exit_code(); }
  }

  async function load() {
    const imports = { env: { renyi_host_now_ms: () => Date.now() } };
    const response = await fetch("assets/renyi_web.wasm");
    if (!response.ok) throw new Error(`the module answered ${response.status}`);
    const streamed = "instantiateStreaming" in WebAssembly
      && (response.headers.get("content-type") || "").startsWith("application/wasm");
    const { instance } = streamed
      ? await WebAssembly.instantiateStreaming(response, imports)
      : await WebAssembly.instantiate(await response.arrayBuffer(), imports);
    return new Toolchain(instance);
  }

  // The game's sources, the main file first (the index tools/site.py writes).
  async function gameFiles() {
    const names = await (await fetch(`${GAME}index.json`)).json();
    return Promise.all(names.map(async (name) => [name, await (await fetch(GAME + name)).text()]));
  }

  let toolchain = null;
  let files = null;
  // Who started the run on the canvas: the game or the playground.
  let owner = null;
  let live = false;
  let keys = held();
  let last = 0;
  // The measure of the last second: the frames and the time in the VM.
  let frames = 0;
  let inside = 0;
  let since = 0;

  function held() {
    return { forward: false, backward: false, turn_left: false, turn_right: false,
      fire: false, quit: false };
  }

  // The frame as the module gives it: one shape per line.
  function draw(frame) {
    context.setTransform(SCALE, 0, 0, SCALE, 0, 0);
    context.fillStyle = "#000";
    context.fillRect(0, 0, WIDTH, HEIGHT);
    context.font = "8px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";
    context.textBaseline = "top";
    for (const line of frame.split("\n")) {
      const parts = line.split(" ");
      if (parts[0] === "column") {
        const [x, top, bottom, red, green, blue] = parts.slice(1).map(Number);
        context.fillStyle = `rgb(${red}, ${green}, ${blue})`;
        context.fillRect(x, top, 1, bottom - top + 1);
      } else if (parts[0] === "rectangle") {
        const [x, y, width, height, red, green, blue] = parts.slice(1).map(Number);
        context.fillStyle = `rgb(${red}, ${green}, ${blue})`;
        context.fillRect(x, y, width, height);
      } else if (parts[0] === "label") {
        const [x, y, red, green, blue] = parts.slice(1, 6).map(Number);
        context.fillStyle = `rgb(${red}, ${green}, ${blue})`;
        context.fillText(parts.slice(6).join(" "), x, y);
      }
    }
  }

  // What the program wrote: the game's last line under the canvas, the
  // playground's text in its output.
  function report(text) {
    if (!text) return;
    if (owner === "game") {
      const lines = text.trim().split("\n");
      gameStatus.textContent = lines[lines.length - 1];
    } else {
      output.textContent += text;
    }
  }

  function ended(status) {
    live = false;
    startButton.textContent = "Play";
    const ending = status === EXITED
      ? `exited with ${toolchain.exitCode()}`
      : ENDINGS[status] || `status ${status}`;
    if (owner === "game") {
      if (status !== FINISHED) gameStatus.textContent = `${gameStatus.textContent} (${ending})`;
    } else {
      runStatus.textContent = ending;
    }
  }

  // One frame: the run resumed with the keys held and the time since the
  // previous frame, the next frame drawn, the measure once a second.
  function tick(now) {
    if (!live) return;
    const elapsed = Math.max(1, Math.min(LONGEST_FRAME_MS, Math.round(now - last)));
    last = now;
    const before = performance.now();
    const status = toolchain.resume(keys, elapsed);
    inside += performance.now() - before;
    frames += 1;
    report(toolchain.text());
    if (status !== SUSPENDED) {
      ended(status);
      return;
    }
    draw(toolchain.frame());
    if (now - since >= 1000) {
      rate.textContent = `${Math.round((frames * 1000) / (now - since))} frames a second, `
        + `${(inside / frames).toFixed(2)} ms a frame in the VM`;
      frames = 0;
      inside = 0;
      since = now;
    }
    requestAnimationFrame(tick);
  }

  // A run started: ended at once, or suspended at its first frame and
  // then driven by the ticks.
  function begin(who, status) {
    owner = who;
    report(toolchain.text());
    if (status !== SUSPENDED) {
      ended(status);
      return;
    }
    live = true;
    keys = held();
    last = performance.now();
    frames = 0;
    inside = 0;
    since = last;
    draw(toolchain.frame());
    startButton.textContent = "Stop";
    if (who === "game") {
      gameStatus.textContent = "Playing: W and S or the arrows move, A and D turn, Space fires, Escape quits.";
    }
    canvas.focus({ preventScroll: true });
    requestAnimationFrame(tick);
  }

  function stop() {
    live = false;
    toolchain.clear();
    startButton.textContent = "Play";
    if (owner === "game") gameStatus.textContent = "Stopped.";
    else runStatus.textContent = "stopped";
  }

  startButton.addEventListener("click", () => {
    if (live) {
      stop();
      return;
    }
    toolchain.clear();
    for (const [name, text] of files) toolchain.file(name, text);
    toolchain.argument("play");
    gameStatus.textContent = "";
    rate.textContent = "";
    begin("game", toolchain.run(files[0][0]));
  });

  const keyOf = (event) => KEYS[event.key.length === 1 ? event.key.toLowerCase() : event.key];
  canvas.addEventListener("keydown", (event) => {
    const key = keyOf(event);
    if (!key || !live) return;
    keys[key] = true;
    event.preventDefault();
  });
  canvas.addEventListener("keyup", (event) => {
    const key = keyOf(event);
    if (!key) return;
    keys[key] = false;
    event.preventDefault();
  });
  canvas.addEventListener("blur", () => { keys = held(); });

  // The playground: the editor's text as the one file of the session,
  // named after its module.
  function stage() {
    if (live) stop();
    toolchain.clear();
    const found = /^\s*module\s+([A-Za-z_][A-Za-z0-9_]*)/.exec(editor.value);
    const name = `${found ? found[1] : "main"}.ry`;
    toolchain.file(name, editor.value);
    return name;
  }

  checkButton.addEventListener("click", () => {
    const name = stage();
    const status = toolchain.check(name);
    const text = toolchain.text();
    output.textContent = text;
    runStatus.textContent = status === FINISHED
      ? (text ? "checks, with warnings" : "checks clean")
      : ENDINGS[status];
  });

  runButton.addEventListener("click", () => {
    const name = stage();
    for (const argument of argumentsField.value.trim().split(/\s+/).filter(Boolean)) {
      toolchain.argument(argument);
    }
    output.textContent = "";
    runStatus.textContent = "";
    begin("playground", toolchain.run(name));
  });

  (async () => {
    try {
      [toolchain, files] = await Promise.all([load(), gameFiles()]);
    } catch (error) {
      const reason = error && error.message ? error.message : String(error);
      startButton.textContent = "Not available";
      gameStatus.textContent = `The toolchain's module could not be loaded (${reason}). `
        + "The site is built with it; a local build takes it with tools/site.py --module.";
      return;
    }
    startButton.textContent = "Play";
    startButton.disabled = false;
    checkButton.disabled = false;
    runButton.disabled = false;
    gameStatus.textContent = "Ready.";
  })();
})();
