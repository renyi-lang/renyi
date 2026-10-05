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

Design is complete (M0) and the implementation has started (M1). What exists:

- `docs/design/01-decisions.md`: every design decision taken so far, with the
  reasoning, in three rounds.
- `docs/design/02-syntax-sketch.md`: the concrete surface those decisions
  produce; `docs/design/04-stdlib-sketch.md`: the prelude and the standard
  library modules.
- `docs/design/03-readability-test.md`: how the grammar is frozen by measuring
  LLM comprehension; the harness is under `tests/readability/`.
- `docs/cheatsheet.md`: the whole language on one page, kept under 3000 tokens
  (`python3 tools/count_tokens.py`).
- `examples/`: thirty example programs, checked by
  `python3 tools/lint_examples.py`.
- `crates/`: the Rust toolchain. `renyi check` reports lexer, parser and layout
  diagnostics as text or JSON; `renyi tokens` and `renyi parse` dump the token
  stream and the syntax tree. Every example parses cleanly.

```
cargo build
./target/debug/renyi check examples/hello.ry
./target/debug/renyi check --json examples/hello.ry
cargo test
```

Next: the formatter (`renyi format`), then the type and effect checker (M2)
and the bytecode VM (M3), which is the first milestone that runs a program.

## Working on this repository

`CLAUDE.md` holds the conventions for Claude Code sessions and
`docs/HANDOFF.md` the current state and next steps; start there.

## License

Apache-2.0. See `LICENSE`.
