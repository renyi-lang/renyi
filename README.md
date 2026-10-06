# Renyi

Renyi is a statically typed programming language whose syntax is regular
English. It is designed to be read and written by LLM agents and reviewed by
people: one spelling per concept, effects in the type system, no exceptions,
no null, no anonymous functions, and documentation that is part of the grammar
so that code can be retrieved by meaning.

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
checker (M2), the project map (`renyi index`) and the VM (M3) exist. What
exists:

- `docs/design/01-decisions.md`: every design decision taken so far, with the
  reasoning.
- `docs/design/02-syntax-sketch.md`: the concrete surface those decisions
  produce; `docs/design/04-stdlib-sketch.md`: the prelude and the standard
  library modules, which also exist as declaration files under
  `library/std/`.
- `docs/design/03-readability-test.md`: how the grammar is frozen by measuring
  LLM comprehension; the harness and a first pre-test round are under
  `tests/readability/`.
- `docs/design/05-agent-tooling.md`: the project map (`renyi index`) and the
  `renyi mcp` server for agents, both of which exist; `docs/design/06-runtime-guarantees.md`:
  recorded and replayable runs, budgets in grants, provenance guards;
  `docs/design/07-system-design.md`: the trade-offs the language claims to
  resolve, capability-safe packages, reproducibility, in-process sandboxing
  and checked live update.
- `docs/cheatsheet.md`: the whole language on one page, kept under 3000 tokens
  (`python3 tools/count_tokens.py`).
- `examples/`: thirty example programs, checked by
  `python3 tools/lint_examples.py`.
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
  compares the outcome and the output, and `--explain` narrates a run
  through the `purpose:` clauses it passes; `renyi test`
  runs every `example:` line and `test` block, a `replays` test from its
  recording (`--strict`, `--refresh`). Every example checks cleanly, is in
  canonical form, and its examples and tests pass, the five that reach
  the network and the one on SQLite from recordings under
  `examples/fixtures/`; the ten programs with a reference output print
  it. The VM covers the whole library: the
  prelude, console, environment, time, random, filesystem, JSON, CSV,
  regular expressions, the HTTP client (`ureq`), the HTTP server (over
  `std::net`) and SQLite (`rusqlite`, compiled in); `run concurrently`
  runs its tasks one after the other (decision S2).

```
cargo build
./target/debug/renyi check examples/hello.ry
./target/debug/renyi check --json examples/hello.ry
./target/debug/renyi format --check examples/*.ry
./target/debug/renyi index examples
./target/debug/renyi index --diff HEAD examples   # what changed since the last commit, and the version bump
./target/debug/renyi run examples/hello.ry Renyi
./target/debug/renyi run --explain examples/statistics.ry 2 4 4 4 5 5 7 9
./target/debug/renyi record --to hello.json examples/hello.ry Renyi
./target/debug/renyi run --replay hello.json examples/hello.ry
./target/debug/renyi reproduce hello.json
./target/debug/renyi test examples/invoice.ry
./target/debug/renyi mcp examples        # the toolchain for an agent host, over standard input and output
cargo test
```

Next: M4 (provenance guards, the package manager, budgets in the
manifest).

## Working on this repository

`CLAUDE.md` holds the conventions for Claude Code sessions and
`docs/HANDOFF.md` the current state and next steps; start there.

## License

Apache-2.0. See `LICENSE`.
