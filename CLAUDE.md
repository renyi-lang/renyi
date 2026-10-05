# Renyi: notes for Claude Code sessions

Read `docs/HANDOFF.md` first. It is the current state of the project and is
rewritten at the end of every session. Then read the four design documents
under `docs/design/` in order.

## What this repository is

Renyi is a programming language. Its design (M0) is complete; the Rust front
end (M1: lexer, parser, formatter) and the type and effect checker (M2) exist;
the VM (M3) does not yet. The owner (GitHub `skymanbp`) makes design decisions and
reviews; Claude writes the documents, the example corpus and the Rust
implementation.

## Conventions

- All documents, code comments and commit messages are in English (decision
  H5). Chat with the owner is in Chinese.
- Design decisions live in `docs/design/01-decisions.md` as append-only
  entries. A reversal is a new entry that names the entry it supersedes. Never
  edit an accepted entry.
- `docs/design/02-syntax-sketch.md` is the single source of truth for the
  surface syntax until the formal grammar exists, and
  `docs/design/04-stdlib-sketch.md` for library names. When an example needs
  something they do not define, extend the sketch in the same commit.
- Every grammar change must keep `docs/cheatsheet.md` under 3000 tokens. Run
  `python3 tools/count_tokens.py` (requires `pip install tiktoken`). It exits
  non-zero over budget.
- Every `.ry` file under `examples/` must pass `python3 tools/lint_examples.py`
  with zero problems.
- When a design question needs the owner, ask with AskUserQuestion in batches
  of four, recommended option first and labelled "(Recommended)". The owner
  asked for this format explicitly and answers quickly in it.
- The implementation language is Rust, one binary named `renyi` (decision H1).
  The Python scripts in `tools/` are development aids only. Before a commit
  that touches `crates/`: `cargo fmt`, `cargo clippy --all-targets` and
  `cargo test` must be clean. The corpus tests also require every example to
  be in canonical layout: run `cargo run -- format examples/*.ry` after
  editing an example.
- Work on a `claude/...` branch and push there. Push to `main` only when the
  owner asks.
- Before ending a session, rewrite `docs/HANDOFF.md` so the next session can
  start without this conversation.

## Where things are

| Path | Content |
|------|---------|
| `docs/design/01-decisions.md` | round-1 (sections 0 to I) and round-2 (section J) decisions with reasons |
| `docs/design/02-syntax-sketch.md` | concrete syntax; section 18 tracks open items (round 2 is settled, new items start at R3-1) |
| `docs/design/03-readability-test.md` | protocol that freezes the grammar by measuring LLM comprehension |
| `docs/design/04-stdlib-sketch.md` | prelude, core modules and extension packages; the lint checks corpus calls against its `function` lines |
| `docs/cheatsheet.md` | the whole language on one page; token-budgeted |
| `examples/` | the corpus, one program per file, index in `examples/README.md` |
| `tools/count_tokens.py` | cheat-sheet budget gate |
| `tools/lint_examples.py` | regex-level checks for the corpus, including calls against the library sketch |
| `tests/readability/` | harness for the readability protocol: `run.py`, manifest, reference outputs, Write tasks |
| `crates/renyi_syntax/` | spans, diagnostics, lexer, AST, parser, JSON encoder, formatter; `tests/corpus.rs` runs the corpus through all of them, `tests/library.rs` parses the library declarations |
| `crates/renyi_check/` | the type and effect checker (M2): `world.rs` declares modules, `check.rs` checks bodies, `effects.rs` covers capabilities, `refine.rs` evaluates refinements on literals; `tests/corpus.rs` and `tests/rules.rs` |
| `library/std/` | the standard library as Renyi declaration files (one per module), compiled into the checker; kept in step with `04-stdlib-sketch.md` by a test |
| `crates/renyi/` | the `renyi` binary: `check` (parse, type and effect check), `format`, `tokens`, `parse [--json]` |
