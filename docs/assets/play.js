// The pages that play (decision AW33, stage 33d; the game's own page and
// the art, decision AW35, stage 36d). The toolchain's module for
// WebAssembly (crates/renyi_web, built by the Pages workflow and put
// beside the pages by tools/site.py) checks and runs programs in the
// browser. On the game's page the showcase game's sources are compiled
// and played on the canvas: the program hands each frame to the page
// through screen.present and suspends, the page draws the shapes and
// resumes it with the keys held and the time elapsed; a strip or a
// picture is drawn from the art's images, shaded, and the sounds the
// program asked for through screen.play are played. On the play page the
// playground runs what is written in it the same way, and a program of
// its own that presents frames plays on the canvas under the output.
// Nothing leaves the page: the module, the game's sources, the art and
// this script are files of the site. tools/wasm_check.js drives the same
// module under node.

(() => {
  "use strict";

  // The game's page has the canvas `game` and its controls; the play
  // page has the editor and the canvas `screen` under its output.
  const canvas = document.getElementById("game") || document.getElementById("screen");
  const startButton = document.getElementById("game-start");
  const fullButton = document.getElementById("game-full");
  const soundBox = document.getElementById("game-sound");
  const gameStatus = document.getElementById("game-status");
  const rate = document.getElementById("game-rate");
  const editor = document.getElementById("editor");
  const checkButton = document.getElementById("check");
  const runButton = document.getElementById("run");
  const argumentsField = document.getElementById("arguments");
  const output = document.getElementById("output");
  const runStatus = document.getElementById("run-status");
  const isGamePage = Boolean(startButton && gameStatus && rate);
  if (!canvas || (!isGamePage && !editor)) return;

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
  // The shades an image is kept in: a shade in percent is rounded to one
  // of these levels, the last the image as it is.
  const LEVELS = 16;
  // The keys held, by the key of the event (a letter in lower case).
  const KEYS = {
    ArrowUp: "forward", w: "forward", ArrowDown: "backward", s: "backward",
    ArrowLeft: "turn_left", a: "turn_left", ArrowRight: "turn_right", d: "turn_right",
    " ": "fire", f: "fire", Escape: "quit", q: "quit",
  };
  const GAME = "assets/raycaster/";
  const ART = "assets/art/";
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
    sounds() { return this.read(this.exports.renyi_sounds(), this.exports.renyi_sounds_len()); }
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

  // The art: every image decoded, every sound's bytes kept until a
  // click makes the audio context that decodes them (the index
  // tools/site.py writes beside the files). A few files are fetched at
  // a time and a failure is tried again: two hundred at once overflow a
  // small server's queue (the one `python -m http.server` keeps, which
  // a local build is seen with), and a file that still fails is logged
  // and left out.
  const images = new Map();
  const soundBytes = new Map();
  const AT_ONCE = 6;
  const TRIES = 3;
  const pause = (ms) => new Promise((resolve) => { setTimeout(resolve, ms); });
  async function each(items, work) {
    const queue = [...items];
    const workers = [];
    for (let i = 0; i < AT_ONCE; i += 1) {
      workers.push((async () => {
        while (queue.length > 0) {
          const item = queue.shift();
          for (let attempt = 1; attempt <= TRIES; attempt += 1) {
            try {
              await work(item);
              break;
            } catch (error) {
              if (attempt === TRIES) console.warn(`the art's ${item} could not be loaded: ${error}`);
              else await pause(200 * attempt);
            }
          }
        }
      })());
    }
    await Promise.all(workers);
  }
  async function art() {
    const index = await (await fetch(`${ART}index.json`)).json();
    await each(index.images, async (name) => {
      const image = new Image();
      image.src = `${ART}${name}.png`;
      await image.decode();
      images.set(name, image);
    });
    await each(index.sounds, async (name) => {
      const response = await fetch(`${ART}${name}.wav`);
      if (!response.ok) throw new Error(`answered ${response.status}`);
      soundBytes.set(name, await response.arrayBuffer());
    });
  }

  // An image at a shade: the image itself at full shade, else a copy
  // darkened once and kept, its transparent pixels left transparent.
  const shaded = new Map();
  const missing = new Set();
  function source(name, shade) {
    const image = images.get(name);
    if (!image) {
      if (!missing.has(name)) {
        missing.add(name);
        console.warn(`the frame names an image the art does not have: ${name}`);
      }
      return null;
    }
    const level = Math.max(0, Math.min(LEVELS - 1, Math.round((shade * (LEVELS - 1)) / 100)));
    if (level === LEVELS - 1) return image;
    const key = `${name}/${level}`;
    let copy = shaded.get(key);
    if (!copy) {
      copy = document.createElement("canvas");
      copy.width = image.width;
      copy.height = image.height;
      const painter = copy.getContext("2d");
      painter.drawImage(image, 0, 0);
      painter.globalCompositeOperation = "source-atop";
      painter.fillStyle = `rgba(0, 0, 0, ${(1 - level / (LEVELS - 1)).toFixed(3)})`;
      painter.fillRect(0, 0, copy.width, copy.height);
      shaded.set(key, copy);
    }
    return copy;
  }

  // The sounds: an audio context made at the first click, which the
  // browser allows, the sounds decoded into it then.
  let audio = null;
  const buffers = new Map();
  async function unlockAudio() {
    if (!audio) {
      const Context = window.AudioContext || window.webkitAudioContext;
      if (!Context) return;
      audio = new Context();
      await Promise.all([...soundBytes].map(async ([name, bytes]) => {
        try {
          buffers.set(name, await audio.decodeAudioData(bytes.slice(0)));
        } catch (error) {
          console.warn(`the sound ${name} does not decode: ${error}`);
        }
      }));
    }
    if (audio.state === "suspended") await audio.resume();
  }

  function play(names) {
    if (!audio || (soundBox && !soundBox.checked)) return;
    for (const name of names.split("\n")) {
      if (!name) continue;
      const buffer = buffers.get(name);
      if (!buffer) {
        if (!missing.has(name)) {
          missing.add(name);
          console.warn(`the program names a sound the art does not have: ${name}`);
        }
        continue;
      }
      const node = audio.createBufferSource();
      node.buffer = buffer;
      node.connect(audio.destination);
      node.start();
    }
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

  // The frame as the module gives it: one shape per line. A strip is one
  // column of an image, taken modulo its width, stretched between two
  // rows; a picture an image stretched to a size; both shaded. An image
  // the art does not have is drawn in magenta, so that the name shows.
  function draw(frame) {
    context.setTransform(SCALE, 0, 0, SCALE, 0, 0);
    context.imageSmoothingEnabled = false;
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
      } else if (parts[0] === "strip") {
        const [x, top, bottom, column, shade] = parts.slice(1, 6).map(Number);
        const image = source(parts.slice(6).join(" "), shade);
        if (!image) {
          context.fillStyle = "#f0f";
          context.fillRect(x, top, 1, bottom - top + 1);
          continue;
        }
        const at = ((column % image.width) + image.width) % image.width;
        context.drawImage(image, at, 0, 1, image.height, x, top, 1, bottom - top + 1);
      } else if (parts[0] === "picture") {
        const [x, y, width, height, shade] = parts.slice(1, 6).map(Number);
        const image = source(parts.slice(6).join(" "), shade);
        if (!image) {
          context.fillStyle = "#f0f";
          context.fillRect(x, y, width, height);
          continue;
        }
        context.drawImage(image, x, y, width, height);
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
    const ending = status === EXITED
      ? `exited with ${toolchain.exitCode()}`
      : ENDINGS[status] || `status ${status}`;
    if (owner === "game") {
      startButton.textContent = "Play again";
      if (status !== FINISHED) gameStatus.textContent = `${gameStatus.textContent} (${ending})`;
    } else {
      runStatus.textContent = ending;
    }
  }

  // One frame: the run resumed with the keys held and the time since the
  // previous frame, the next frame drawn, its sounds played, the measure
  // once a second.
  function tick(now) {
    if (!live) return;
    const elapsed = Math.max(1, Math.min(LONGEST_FRAME_MS, Math.round(now - last)));
    last = now;
    const before = performance.now();
    const status = toolchain.resume(keys, elapsed);
    inside += performance.now() - before;
    frames += 1;
    report(toolchain.text());
    play(toolchain.sounds());
    if (status !== SUSPENDED) {
      ended(status);
      return;
    }
    draw(toolchain.frame());
    if (owner === "game" && now - since >= 1000) {
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
    play(toolchain.sounds());
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
    canvas.hidden = false;
    draw(toolchain.frame());
    if (who === "game") {
      startButton.textContent = "Stop";
      gameStatus.textContent = "Playing.";
    }
    canvas.focus({ preventScroll: true });
    requestAnimationFrame(tick);
  }

  function stop() {
    live = false;
    toolchain.clear();
    if (owner === "game") {
      startButton.textContent = "Play";
      gameStatus.textContent = "Stopped.";
    } else {
      runStatus.textContent = "stopped";
    }
  }

  // The keys: on the game's page the whole window listens while the game
  // runs; on the play page the canvas, once focused, so that the editor
  // keeps its keys.
  const keyOf = (event) => KEYS[event.key.length === 1 ? event.key.toLowerCase() : event.key];
  const keyTarget = isGamePage ? window : canvas;
  keyTarget.addEventListener("keydown", (event) => {
    const key = keyOf(event);
    if (!key || !live || event.target instanceof HTMLInputElement) return;
    keys[key] = true;
    event.preventDefault();
  });
  keyTarget.addEventListener("keyup", (event) => {
    const key = keyOf(event);
    if (!key) return;
    keys[key] = false;
    event.preventDefault();
  });
  window.addEventListener("blur", () => { keys = held(); });

  if (isGamePage) {
    startButton.addEventListener("click", async () => {
      if (live) {
        stop();
        return;
      }
      startButton.disabled = true;
      await unlockAudio();
      startButton.disabled = false;
      toolchain.clear();
      for (const [name, text] of files) toolchain.file(name, text);
      toolchain.argument("play");
      gameStatus.textContent = "";
      rate.textContent = "";
      begin("game", toolchain.run(files[0][0]));
    });
    if (fullButton) {
      fullButton.addEventListener("click", () => {
        const stage = canvas.parentElement;
        if (document.fullscreenElement) {
          document.exitFullscreen();
        } else if (stage.requestFullscreen) {
          stage.requestFullscreen();
        }
      });
    }
  }

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

  if (editor) {
    checkButton.addEventListener("click", () => {
      const name = stage();
      const status = toolchain.check(name);
      const text = toolchain.text();
      output.textContent = text;
      runStatus.textContent = status === FINISHED
        ? (text ? "checks, with warnings" : "checks clean")
        : ENDINGS[status];
    });

    runButton.addEventListener("click", async () => {
      const name = stage();
      for (const argument of argumentsField.value.trim().split(/\s+/).filter(Boolean)) {
        toolchain.argument(argument);
      }
      output.textContent = "";
      runStatus.textContent = "";
      await unlockAudio();
      begin("playground", toolchain.run(name));
    });
  }

  (async () => {
    const say = (text) => {
      if (isGamePage) gameStatus.textContent = text;
      else runStatus.textContent = text;
    };
    try {
      [toolchain, files] = await Promise.all([load(), isGamePage ? gameFiles() : null, art()]);
    } catch (error) {
      const reason = error && error.message ? error.message : String(error);
      if (isGamePage) startButton.textContent = "Not available";
      say(`The toolchain's module could not be loaded (${reason}). `
        + "The site is built with it; a local build takes it with tools/site.py --module.");
      return;
    }
    if (isGamePage) {
      startButton.textContent = "Play";
      startButton.disabled = false;
    } else {
      checkButton.disabled = false;
      runButton.disabled = false;
    }
    say("Ready.");
  })();
})();
