# Renyi: notes for Claude Code sessions

Read `docs/HANDOFF.md` first. It is the current state of the project and is
rewritten at the end of every session. Then read the four design documents
under `docs/design/` in order.

## What this repository is

Renyi is a programming language in its design phase (milestone M0). No
compiler exists yet. The owner (GitHub `skymanbp`) makes design decisions and
reviews; Claude writes the documents, the example corpus and, from M1 on, the
Rust implementation.

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
  The Python scripts in `tools/` are development aids only.
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
| `tools/lint_examples.py` | regex-level checks for the corpus |
