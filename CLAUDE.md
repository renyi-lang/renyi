# Renyi: notes for Claude Code sessions

Read `docs/HANDOFF.md` first. It is the current state of the project and is
rewritten at the end of every session. Then read the seven design documents
under `docs/design/` in order.

## What this repository is

Renyi is a programming language. Its design (M0) is complete; the Rust front
end (M1: lexer, parser, formatter), the type and effect checker (M2) and
the VM (M3: `renyi run`, `record`, `run --replay`, `--explain`, `test` with
`replays`, budgets and scope checks, every library module including HTTP,
the server and SQLite; tasks run one after the other by decision S2)
exist. The owner (GitHub `skymanbp`) makes design decisions and reviews;
Claude writes the documents, the example corpus and the Rust
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
- `main` is the only branch (owner's decision, 2026-10-05). Commit and push
  there directly; do not create other branches, local or remote.
- Before ending a session, rewrite `docs/HANDOFF.md` so the next session can
  start without this conversation.

## Where things are

| Path | Content |
|------|---------|
| `docs/design/01-decisions.md` | design decisions with reasons: rounds 1 and 2 (sections 0 to J), library (K), readability (L), pre-test (M), checker (N), runtime and agent tooling (O), signature capabilities (P), system-level commitments (Q), the live round (R), runtime dependencies and concurrency (S) |
| `docs/design/02-syntax-sketch.md` | concrete syntax; section 18 tracks open items (round 2 is settled, new items start at R3-1) |
| `docs/design/03-readability-test.md` | protocol that freezes the grammar by measuring LLM comprehension |
| `docs/design/04-stdlib-sketch.md` | prelude, core modules and extension packages; the lint checks corpus calls against its `function` lines |
| `docs/design/05-agent-tooling.md` | the project map (`renyi index`), metrics, content hashes, budgets, diffs and the `renyi mcp` server |
| `docs/design/06-runtime-guarantees.md` | recorded runs and `replays` tests, narrated runs, budgets in grants (`at most`), provenance guards (`only to`) |
| `docs/design/07-system-design.md` | the trade-offs the language claims to resolve; capability-safe packages, reproducibility, in-process sandboxing, checked live update |
| `docs/cheatsheet.md` | the whole language on one page; token-budgeted |
| `examples/` | the corpus, one program per file, index in `examples/README.md` |
| `tools/count_tokens.py` | cheat-sheet budget gate |
| `tools/lint_examples.py` | regex-level checks for the corpus, including calls against the library sketch |
| `tests/readability/` | harness for the readability protocol: `run.py`, manifest, reference outputs, Write tasks |
| `crates/renyi_syntax/` | spans, diagnostics, lexer, AST, parser, JSON encoder, formatter; `tests/corpus.rs` runs the corpus through all of them, `tests/library.rs` parses the library declarations |
| `crates/renyi_check/` | the type and effect checker (M2): `world.rs` declares modules, `check.rs` checks bodies, `effects.rs` covers capabilities, `refine.rs` evaluates refinements on literals; `tests/corpus.rs` and `tests/rules.rs` |
| `library/std/` | the standard library as Renyi declaration files (one per module), compiled into the checker; kept in step with `04-stdlib-sketch.md` by a test |
| `crates/renyi_vm/` | the VM (M3): `compile/` lowers the checked tree to bytecode (`bytecode.rs`) through the checker's recorded references, `vm.rs` runs it and holds the primitive boundary (`call_native`: grant and budget checks, recording, replay, narration), `grant.rs` the effective grant and budget counters, `recording.rs` the recording format and the replay, `natives/` the library primitives, `runner.rs` runs `main`, examples and tests; `tests/corpus.rs` checks the ten reference outputs and every `example:` and `test` block, `tests/recording.rs` the boundary |
| `crates/renyi/` | the `renyi` binary: `check` (parse, type and effect check), `format`, `tokens`, `parse [--json]`, `index [--json \| --budgets]`, `run [options] <file> [arguments]`, `record [--to file] [options] <file> [arguments]`, `test [--strict] [--refresh name] [--explain] <file>...` |
