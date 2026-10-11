# Renyi

**Renyi is the scripting language of AI agents: the agent writes it, you
review it at a glance, and the program can only do what it declares.**

## Why Renyi

Agents now write most of the code that runs on your behalf. The code they
write in today's languages has to be read line by line, or run blind.
Renyi is designed around that division of labour: the agent writes, the
person reviews, and the language makes the review a glance.

1. **What a program may do is in its signature.** Every function declares
   its effects (`needs filesystem.read("data"), network.http("api.example")`),
   scoped to a path or a host, budgeted (`at most 60 per minute`) and
   guarded (`only to`). The compiler refuses what is not declared; the
   runtime admits only that, for the program's dependencies too. The
   signature of `main` is the whole program's grant.
2. **Every run is evidence.** A run can be recorded, replayed offline and
   reproduced under its manifest (toolchain, code hash, dependencies,
   grant, arguments, outcome); `--explain` narrates it in the words the
   program's author wrote. You know what a run did, as data.
3. **Dependencies cannot widen what you allowed.** A package's effects are
   computed from its sources, never written by hand; an update that would
   let the program do more is refused, not noticed later.
4. **The tooling an agent needs is built in.** The project map and the
   semantic diff are commands (`renyi index`, `--diff`), the toolchain is an
   MCP server (`renyi mcp`), documentation is syntax (`purpose:`), and every
   error carries a fix, so the agent's next attempt is right.
5. **Read in one pass, by anyone.** Regular English with one spelling per
   concept, one canonical layout, no exceptions, no null, no anonymous
   functions, no operator overloading. A reviewer who has never seen Renyi
   reads what a program touches from its signature.

```
public function active_adult_emails(path: Path)
  returns List of Email
  or fails with FileError or JsonError
  needs filesystem.read
  purpose: Read users from a JSON file and return the e-mails of active users who are at least 18.

  let text be filesystem.read_text(path) otherwise fail
  let users: List of User be json.parse(text) otherwise fail
  let emails be
    for each user in users
    where user.is_active and user.age is at least 18
    sorted by user.name
    collect user.email
  return emails
end
```

Renyi is a general-purpose language; its first users are people who run
automation with Claude Code, Codex and their like, and will not run an
agent's Python blind. The design record is
`docs/design/08-positioning.md` (decisions AH1 to AH4).

## Against the alternatives

Each row is a claim the implementation keeps and the conformance suite
tests. The other languages were built for other jobs and do them well;
the comparison is for one job, a program an agent wrote and a person
must trust.

| For a program an agent wrote | Renyi | Python | TypeScript / Deno | Rust | Shell |
|---|---|---|---|---|---|
| What it may do, visible before it runs | in every signature, scoped and budgeted, checked by the compiler | not declared; a sandbox limits the whole process | `--allow-*` flags for the whole process | not declared; the type system is about memory, not effects | not declared |
| Its dependencies' effects | computed from their sources, locked, refused when they widen | not declared | not declared | not declared | not declared |
| What a run did | recorded, replayed, reproduced by the runtime | external tooling | external tooling | external tooling | `set -x` |
| What an error means | a fix on every diagnostic, by rule | a traceback | a message | a good message, for a programmer | an exit code |
| Reviewable without training | regular English, one spelling per concept | familiar to programmers | familiar to programmers | expert reading | terse |
| Speed on integer loops | machine code in the process | CPython | V8 | native, the fastest | n/a |

Python and TypeScript are the languages agents write best today, and
nothing in them says what a script will touch before it runs. Rust gives
guarantees about memory and data races at a cost in time and expertise
that a script does not repay, and says nothing about effects. Shell is
what agents reach for first, and it is the hardest to review of all.

## The gap

Between "the agent wrote it" and "I ran it" there is a review that nobody
has time for. Sandboxes bound the process from outside and tell you
nothing about the program; type systems check shapes, not effects;
recordings and audit trails are bolted on afterwards, if at all. Renyi
closes that gap inside the language: the effects are types, the grant is
a signature, the run is data, and the syntax exists so that a person can
check all three in the time it takes to read a function header. That is
the whole design; everything else follows from it.

Source files use the `.renyi` or `.ry` extension; the two are equivalent.

## Install

Every release (decisions AI1 to AI4; `v0.1.0` is the first) carries
binaries for Linux (x86_64), macOS (Apple silicon) and Windows (x86_64).
On Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh
```

On Windows, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex
```

Both verify the archive's checksum; `RENYI_VERSION=v0.1.0` picks a
version and `RENYI_INSTALL_DIR` a directory. With a Rust toolchain,
`cargo install renyi` builds the same binary from crates.io, and `cargo
install --git https://github.com/renyi-lang/renyi renyi` from the
repository. The VS Code extension (syntax highlighting and the client of
the language server, `renyi lsp`; `editors/vscode/`) is attached to every
release as a `.vsix`. `docs/RELEASE.md` is the
release procedure, and the documentation site is
<https://renyi-lang.org>. An agent starts from `starter/` (decision AI3): a
skill file, the MCP configuration and five workflows that do real work,
with their tests and recordings.

## Status

The design is complete (M0); the front end (M1), the type and effect
checker (M2), the project map (`renyi index`), the VM (M3) and packages
(the first slice of M4, decision AC1) exist. What exists:

- `docs/design/01-decisions.md`: every design decision taken so far, with the
  reasoning.
- `docs/design/02-syntax-sketch.md`: the concrete surface those decisions
  produce; `docs/design/04-stdlib-sketch.md`: the prelude and the standard
  library modules, which also exist as declaration files under
  `library/std/`.
- `docs/design/03-readability-test.md`: how LLM comprehension of the grammar is
  measured (the grammar is frozen by decision V11); the harness and the rounds are under
  `tests/readability/`.
- `docs/design/05-agent-tooling.md`: the project map (`renyi index`) and the
  `renyi mcp` server for agents, both of which exist; `docs/design/06-runtime-guarantees.md`:
  recorded and replayable runs, budgets in grants, provenance guards;
  `docs/design/07-system-design.md`: the trade-offs the language claims to
  resolve, capability-safe packages, reproducibility, in-process sandboxing
  and checked live update.
- `docs/cheatsheet.md`: the whole language on one page, kept under 3000 tokens
  (`python3 tools/count_tokens.py`).
- `docs/grammar.ebnf`: the formal grammar of the syntax level, W3C EBNF over
  the lexer's tokens; a test in the syntax crate interprets it and checks that
  it accepts exactly what the parser accepts.
- `docs/reference.md`: the language reference, normative: one section per
  construct with the grammar excerpt, the meaning, the static rules with their
  diagnostic codes and the run-time behaviour; a test holds its excerpts to
  the grammar file and its list of diagnostic codes to the crates.
- `examples/`: thirty example programs, checked by
  `python3 tools/lint_examples.py`.
- `compiler/`: the compiler written in Renyi itself, run by the Rust VM:
  the syntax tree as Renyi types (whose derived JSON is what `renyi parse
  --json` prints), the lexer, the parser, the type and effect checker and
  the bytecode emitter; `renyi run compiler/parse.ry <file>` prints the
  same tree and the same diagnostics as `renyi parse --json <file>`,
  byte for byte on every program of the corpus, the conformance suite,
  the library and the compiler itself, `renyi run compiler/checker.ry --json <file>` prints
  the same diagnostics as `renyi check --json <file>`, with and without
  `--strict`, on every program of the corpus, the conformance suite and
  the compiler itself, and `renyi run compiler/compile.ry --to
  <file.ryc> <file>` writes the same bytecode file as `renyi compile`,
  byte for byte, on the same programs; a test holds all three pairs
  equal.
- `crates/`: the Rust toolchain. `renyi check` reports lexer, parser, type,
  effect and layout diagnostics as text or JSON, each with a suggested fix;
  `renyi format` rewrites files in the canonical layout; `renyi tokens` and
  `renyi parse [--json]` dump the token stream and the syntax tree;
  `renyi index [--json]` prints the project map: one record per definition
  with its signature, purpose, declared and transitive effects and
  failures, edges, metrics and content hash, `renyi index --diff <map or
  revision>` what changed since a saved map or a git revision, per
  definition, with what each change reaches and the version bump it
  forces; `renyi run` checks a program
  and runs its `main` on the bytecode VM under the grant `main` declares,
  narrowed by `--deny`, `--allow-host`, `--allow-read`, `--allow-write`
  and `--at-most`, and inside each function by that function's own
  `needs` (the grant stack of decision Q1); `renyi record` writes a
  recording of every effect of a run with the run manifest in its header
  (`--redact` keeps a secret out of it), `renyi run --replay` re-executes
  one offline, `renyi reproduce` replays one under its manifest and
  compares the outcome and the output, `--explain` narrates a run
  through the `purpose:` clauses it passes, and `--profile` reports where
  its time went; `renyi serve --watch` runs a service and, when a file
  of its project changes and the program checks clean, runs `main`
  again between two requests with the listening socket kept open
  (decision AO1); `renyi run --sandbox grant.json` runs a program under
  a grant narrower than its `main` declares, with a memory budget, and a
  Rust host loads a module the same way through `renyi::Sandbox` and
  calls its public functions (decision AP1; the guide is
  `docs/embedding.md`); the VM generates machine code for the hot code
  objects inside the binary (decision AG1: integer loops run in
  registers, an Integer that leaves the machine word goes back to the
  interpreter, which has the big ones), and `--interpret` keeps a run
  on the interpreter; `renyi test`
  runs every `example:` line and `test` block, a `replays` test from its
  recording (`--strict`, `--refresh`); `renyi compile` writes a program
  as a bytecode file (`.ryc`, the derived JSON of the types of
  `compiler/bytecode.ry`), which `run`, `record`, `test` and
  `reproduce` load in place of the source; `renyi bind <header.h>
  --module <name> --library <names>` writes a foreign module from a C
  header (decision AF1) and `renyi bind --python <package>` a Python
  module from a package (decisions AM1 and AM2); `renyi add`, `update
  [--accept-effects]`, `audit`, `fetch` and `publish` manage a project's
  dependencies (decision AC1: `renyi.json` names them and a registry,
  a directory or a URL; `renyi.lock.json` pins each version's hash;
  every package's files are verified against their hashes and its
  effect manifest recomputed from its sources, so a package cannot
  understate what it does, and a version whose effects widen is never
  taken silently). Every example checks cleanly, is in
  canonical form, and its examples and tests pass, the five that reach
  the network and the one on SQLite from recordings under
  `examples/fixtures/`; the ten programs with a reference output print
  it. The VM covers the whole library: the
  prelude, console, environment, time, random, filesystem, JSON, CSV,
  regular expressions, the HTTP client (`ureq`), the HTTP server (over
  `std::net`), SQLite (`rusqlite`, compiled in) and other programs
  (`std.process`, decision AE1), C libraries through foreign modules
  (decision AF1; `renyi bind` writes one from a header) and Python
  modules through the bridge (decisions AJ2, AJ3 and AL1 to AL4; `renyi
  bind --python` writes one from a package, AM1 and AM2; the guide is
  `docs/python.md`); `run concurrently`
  runs its tasks one after the other (decision S2); the binary
  allocates through `mimalloc` (decision X6). `bench/` holds six
  benchmarks, four of them with CPython twins, and `tools/bench.py`
  times them (decision AG4; CI prints the numbers).

```
cargo build
./target/debug/renyi check examples/hello.ry
./target/debug/renyi check --json examples/hello.ry
./target/debug/renyi format --check examples/*.ry
./target/debug/renyi index examples
./target/debug/renyi index --diff HEAD examples   # what changed since the last commit, and the version bump
./target/debug/renyi run examples/hello.ry Renyi
./target/debug/renyi run --explain examples/statistics.ry 2 4 4 4 5 5 7 9
./target/debug/renyi run --profile compiler/parse.ry compiler/parser.ry   # where the VM's time goes
./target/debug/renyi run --interpret bench/primes.ry   # the interpreter alone; by default the hot code runs as machine code
python tools/bench.py target/debug/renyi              # the benchmarks of bench/, with --interpret, from their images and against CPython
./target/debug/renyi record --to hello.json examples/hello.ry Renyi
./target/debug/renyi run --replay hello.json examples/hello.ry
./target/debug/renyi reproduce hello.json
./target/debug/renyi test examples/invoice.ry
./target/debug/renyi compile --to hello.ryc examples/hello.ry && ./target/debug/renyi run hello.ryc Renyi   # from the bytecode file
./target/debug/renyi mcp examples        # the toolchain for an agent host, over standard input and output
(cd tests/conformance/packages/project && ../../../../target/debug/renyi audit)   # a dependency's effects against every main
./target/debug/renyi run compiler/parse.ry examples/hello.ry   # the parser written in Renyi, on the VM
./target/debug/renyi run compiler/checker.ry --json examples/hello.ry   # the checker written in Renyi, on the VM
./target/debug/renyi run compiler/compile.ry --to hello.ryc examples/hello.ry   # the compiler written in Renyi, on the VM
cargo test
```

Release 0.1.0 is out (2026-10-07; decisions AI1 to AI5, the
procedure in `docs/RELEASE.md`); after it came the registration API for
Rust natives (decisions AJ1 and AK1 to AK4; `docs/extensions.md`), the
Python bridge (decisions AJ2, AJ3 and AL1 to AL4; `docs/python.md`), the
resident world, the language server, the watch of `serve` and the
embedding API (M5, decisions AN1 to AP2), and the rounds on the VM's
speed and the front end's (decisions AQ to AU47). `docs/ROADMAP.md` is
the map of every stage, numbered and grouped by track, with what comes
next: the compiler written in Renyi brought within three times the Rust
front end's time, when the binary switches to it.

## Working on this repository

`CLAUDE.md` holds the conventions for Claude Code sessions and
`docs/HANDOFF.md` the current state and next steps; start there.

## License

Apache-2.0. See `LICENSE`.
