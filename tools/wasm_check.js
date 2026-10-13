// Runs a Renyi program through the web crate's module under node
// (decision AW29, stage 33a): the files given are handed to the module,
// the first being the main file, the program is checked, then run, and
// what it printed comes back on standard output. The exit status is the
// module's: 0 finished, 1 main failed, 2 crashed, 4 the program does not
// check, 5 main needs a capability the page cannot serve, or the code of
// `environment.exit`. CI runs it on the hello program of the corpus.
//
//   node tools/wasm_check.js <renyi_web.wasm> <main.ry> [<file.ry>...] [-- <argument>...]

"use strict";

const fs = require("fs");
const path = require("path");

async function main() {
  const [wasmPath, ...rest] = process.argv.slice(2);
  if (!wasmPath || rest.length === 0) {
    process.stderr.write(
      "usage: node tools/wasm_check.js <renyi_web.wasm> <main.ry> [<file.ry>...] [-- <argument>...]\n"
    );
    return 64;
  }
  const split = rest.indexOf("--");
  const files = split < 0 ? rest : rest.slice(0, split);
  const args = split < 0 ? [] : rest.slice(split + 1);
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
  const answer = () =>
    new TextDecoder().decode(
      new Uint8Array(memory.buffer, exports.renyi_text(), exports.renyi_text_len())
    );
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
  const status = call("renyi_run", mainName);
  process.stdout.write(answer());
  return status === 3 ? exports.renyi_exit_code() : status;
}

main().then(
  (status) => process.exit(status),
  (error) => {
    process.stderr.write(`${error && error.stack ? error.stack : error}\n`);
    process.exit(70);
  }
);
