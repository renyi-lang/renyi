# Handoff

Last updated: 2026-10-05, session 2 in progress.
Branches: `main` holds the session-1 handoff; `claude/renyi-language-design-hbrie3`
carries session 2.

## Where the project stands

Milestone M0 (design) is nearly done. Rounds 1 and 2 of design decisions are
complete and recorded; the surface syntax is sketched in full; the standard
library is sketched and every corpus call is checked against it. The corpus
has reached its target of 30 programs and passes the lint. The cheat sheet
measures 2451 tokens against a 3000-token budget. No compiler, parser or
runtime exists. Three library questions (R3-1 to R3-3 at the end of
`04-stdlib-sketch.md`) await the owner.

## Done in session 1

1. Asked the owner about 50 design questions in interactive batches; every
   answer is recorded in `docs/design/01-decisions.md`.
2. Designed the clause grammar, the three verbs (`let ... be`, `set ... to`,
   `is`), the six comparison phrases, multi-word keywords as single tokens,
   the unified `type` construct, three-tier deprecation, structured
   concurrency, and the effect/capability model; all in
   `docs/design/02-syntax-sketch.md`.
3. Wrote `docs/cheatsheet.md`, `tools/count_tokens.py`, `tools/lint_examples.py`.
4. Wrote 14 example programs covering the four target use cases (agent/API
   glue, data scripts, backend service, CLI). Index: `examples/README.md`.
5. Wrote the readability test protocol, `docs/design/03-readability-test.md`.
6. Added README and the Apache-2.0 license.

## Done in session 2

1. Grew the corpus from 14 to 30 programs (state machine, recursive tree,
   sets, text parsing, dates, SQLite, generic container with its own ability,
   cross-module import, embedded library, semver, pagination, Float
   statistics, topological sort, chat API call, recursive file walk, Markdown
   table). Index: `examples/README.md`.
2. Extended the syntax sketch where the programs needed it: block-string
   boundaries, interpolation rules (no string literal inside a hole), exposing
   a sum type exposes its variants, namespace names are taken locally, `can`
   after variants, generic ability implementations with `for any`, binding
   scope per scope, the collection and text methods in use, `otherwise fail`
   inside tests. Added open items R2-14 to R2-16.
3. Taught the lint about `"""` blocks, one-line `if ... end`, ability
   declarations with type parameters, the three new reserved words, and
   unused bindings (heuristic).
4. Round 2 decisions: asked the owner all 17 open items in four batches and
   recorded them as entries J1 to J18 of `01-decisions.md`. The owner took
   every recommendation except R2-6 (scoped capabilities are in v1). New
   reserved words: `raw`, `within`, `ignore` (84 in total). The sketch, the
   cheat sheet and six corpus programs were updated to match.
5. Wrote `docs/design/04-stdlib-sketch.md` (prelude types and methods, core
   abilities, built-in errors, `std.console`, `std.environment`, `std.time`,
   `std.random`, `std.filesystem`, `std.json`, `std.http`, `std.server`,
   `std.csv`, `std.sqlite`, `std.regex`) after four owner decisions (K1 to
   K4: methods are `self: Type` functions, non-2xx HTTP is a failure,
   `matches` in the prelude, minimal `Bytes`). The lint now rejects any corpus
   call not declared there.

## Next steps, in order

1. **Ask R3-1 to R3-3** (end of `04-stdlib-sketch.md`): map and set ordering,
   JSON encoding of variants with fields, whether `console.print` takes any
   `ToText` value. Record answers in `01-decisions.md` section K or a new L.
2. **Readability test harness.** Implement `docs/design/03-readability-test.md`
   under `tests/readability/`. It needs API access to at least three models
   from two vendors; confirm credentials with the owner first.
3. **M1 in Rust.** Cargo workspace with crates for lexer (phrase table, longest
   match), parser (clause grammar, continuation rules), formatter (canonical
   layout from sketch section 16), and a conformance test runner driven by the
   corpus. The owner chose M3 (a VM running the corpus) as the first demo.

## Known gaps and risks

- The standard library sketch is a first draft written from the corpus; its
  JSON derivation rules, SQLite type mapping and HTTP timeout default have
  not been validated against real data.
- 84 reserved words include common identifiers (`count`, `first`, `sum`,
  `set`, `type`, `test`, `check`, `run`, `group`, `power`, `tags`, `example`).
  The owner confirmed in round 2 (J3) that they stay reserved; the fix for a
  collision is a rename suggestion from the compiler.
- Scoped capabilities (J11) make the effect checker compare path prefixes and
  host names; the containment rules in sketch section 11 are the first draft
  and have not been exercised beyond three corpus programs.
- The token budget is measured with tiktoken `o200k_base` and `cl100k_base` as
  proxies; no tokenizer for Claude models is public.
- `tools/lint_examples.py` is regex-based; its block/`end` balance check is a
  heuristic and can miss errors.
- Decimal rendering: `rounded(2)` renders with exactly two decimals (`6.00`),
  now stated in the library sketch; the examples depend on that.
- The lint's call check is coarse: it accepts any name declared anywhere in
  the library sketch or the corpus, not per receiver type.
- The GitHub default branch was set automatically to the first pushed branch;
  the owner should switch it to `main` in the repository settings.

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (for example deriving comparisons from `is`).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches over prose.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews.
