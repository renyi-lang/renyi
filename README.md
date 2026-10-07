# Renyi

Renyi is the scripting language of AI agents: the language an agent
writes and a person reviews at a glance. A Renyi program declares what it
may do (`needs filesystem.read("data"), network.http("api.example")`),
the compiler refuses what is not declared, and the runtime admits only
that, for the program's dependencies too. Every run can be recorded,
replayed and narrated; the project map and the semantic diff are
commands; every error carries a fix. The syntax is regular English with
one spelling per concept, so that a reviewer reads a program in one
pass: effects in the type system, no exceptions, no null, no anonymous
functions, and documentation that is part of the grammar so that code
can be retrieved by meaning. It is a general-purpose language whose
first users are people who run automation with Claude Code or Codex and
will not run an agent's Python blind (`docs/design/08-positioning.md`,
decisions AH1 to AH4).

What Renyi leads with:

- **Effects are capabilities.** Every function declares what it needs,
  scoped to a path or a host, budgeted (`at most 60 per minute`) and guarded
  (`only to`); `main`'s declaration is the program's grant, and the VM's
  one boundary enforces it, dependencies included.
- **Runs are recorded, replayed and narrated.** `renyi record`, `run
  --replay`, `reproduce` under the run manifest, `--explain` through
  the program's own `purpose:` clauses.
- **The tooling an agent needs.** `renyi index` (the project map and the
  semantic diff with the version bump it forces), `renyi mcp` (the
  toolchain as MCP tools), `purpose:` as syntax, a fix on every error.
- **Package effects are computed, never widened silently.** `renyi add`,
  `update`, `audit`: a dependency's effects come from its sources, and a
  version that would let the program do more is refused.

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

Source files use the `.renyi` or `.ry` extension; the two are equivalent.

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
  its time went; the VM generates machine code for the hot code
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
  header (decision AF1); `renyi add`, `update
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
  (`std.process`, decision AE1) and C libraries through foreign modules
  (decision AF1; `renyi bind` writes one from a header); `run concurrently`
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
python tools/bench.py target/debug/renyi              # the benchmarks of bench/, with --interpret and against CPython
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

Next: M4 (provenance guards, the package manager, budgets in the
manifest).

## Working on this repository

`CLAUDE.md` holds the conventions for Claude Code sessions and
`docs/HANDOFF.md` the current state and next steps; start there.

## License

Apache-2.0. See `LICENSE`.
