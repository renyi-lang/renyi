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

The design is complete (M0), the front end (M1), the type and effect
checker (M2) and the project map (`renyi index`) exist; the VM (M3) is
next. What exists:

- `docs/design/01-decisions.md`: every design decision taken so far, with the
  reasoning.
- `docs/design/02-syntax-sketch.md`: the concrete surface those decisions
  produce; `docs/design/04-stdlib-sketch.md`: the prelude and the standard
  library modules, which also exist as declaration files under
  `library/std/`.
- `docs/design/03-readability-test.md`: how the grammar is frozen by measuring
  LLM comprehension; the harness and a first pre-test round are under
  `tests/readability/`.
- `docs/design/05-agent-tooling.md`: the project map (`renyi index`, which
  exists) and the `renyi mcp` server for agents; `docs/design/06-runtime-guarantees.md`:
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
  failures, edges, metrics and content hash. Every example checks cleanly
  and is in canonical form.

```
cargo build
./target/debug/renyi check examples/hello.ry
./target/debug/renyi check --json examples/hello.ry
./target/debug/renyi format --check examples/*.ry
./target/debug/renyi index examples
cargo test
```

Next: the bytecode VM (M3), the first milestone that runs a program, which
carries recorded runs, budgets and the grant stack from its first version.

## Working on this repository

`CLAUDE.md` holds the conventions for Claude Code sessions and
`docs/HANDOFF.md` the current state and next steps; start there.

## License

Apache-2.0. See `LICENSE`.
