# Handoff

Last updated: 2026-10-05, session 4 (the readability pre-test is finished and
its results are with the owner as questions).
Branches: `main` holds the session-1 handoff; `claude/renyi-language-design-hbrie3`
carries session 2; `claude/nifty-knuth-r5sntt` carries sessions 3 and 4 on top
of it. The owner decides when `main` moves.

## Where the project stands

Milestone M0 (design) is complete and M1 (front end) is essentially done.
Three rounds of design decisions are recorded; the surface syntax and the
standard library are sketched in full, and no design question is open. The
corpus has its target 30 programs, passes the lint, and is in canonical
layout. The cheat sheet measures 2451 tokens against a 3000-token budget. The
Rust workspace under `crates/` has the lexer, parser and formatter with
`renyi check`, `renyi format`, `renyi tokens` and `renyi parse` (also
`--json`); every corpus program lexes, parses, formats and encodes cleanly
(45 tests, clippy and fmt clean). Nothing type-checks or runs yet: that is M2
and M3. The readability harness has run once as a subagent pre-test (below);
the live round still waits for API keys.

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
   call not declared there. Three follow-up questions (R3-1 to R3-3) were
   answered and recorded as K9 to K11.
6. Wrote the readability test harness under `tests/readability/` (`run.py`
   with `prepare`, `run`, `score`, `report`; `manifest.json`; ten reference
   outputs for the deterministic programs, which double as the first
   conformance expectations; ten fresh Write tasks). Smoke-tested with the
   `file` provider. The owner will set `ANTHROPIC_API_KEY` and
   `OPENAI_API_KEY` in the cloud environment (decision L1) and Claude judges
   Complete and Write samples before M3 (decision L2).
7. Started M1: a Cargo workspace (`crates/renyi_syntax`, `crates/renyi`) with
   spans, diagnostics (text and JSON), layout checks, the complete lexer
   (phrase table with longest match, dot members, interpolation holes, block
   and raw text, clause text) with unit tests, the `renyi check` and
   `renyi tokens` commands, and a corpus conformance test that lexes all
   thirty programs. `cargo test`, `cargo clippy --all-targets` and
   `cargo fmt --check` are clean.
8. Wrote the AST (`ast.rs`) and the parser (`parser.rs`): clause grammar,
   statements, expressions with the precedence of sketch section 7, queries,
   patterns, `if` and `match` as statements and expressions, the
   indentation-sensitive continuation rules (now stated in sketch section 1),
   error recovery to the next line. `renyi check` runs the parser; `renyi
   parse` dumps the tree. All thirty corpus programs parse without a
   diagnostic, and the corpus test also checks that every public item has a
   `purpose:` clause.
9. Wrote the formatter (`format.rs`): a document model (text, soft and hard
   line breaks, nesting, groups, ordered alternatives, wrapped words) printed
   at 100 columns, with the layout rules of sketch section 16 (which gained
   the rules the implementation forced: one-line `if`, arm bodies, example
   breaking, prose wrapping, blank lines). `renyi format [--check]` rewrites
   files. Corpus tests: formatting is idempotent, preserves the syntax tree
   (spans aside), stays within 100 columns, and every corpus file is in
   canonical form (eight files were reformatted by the tool).

## Done in sessions 3 and 4: the readability pre-test

The owner asked whether the readability test needs API keys at all and
approved a free pre-test: fresh Claude Code subagents (Haiku and Sonnet) that
read only the cheat sheet and one prompt file, one sample per item. Run
directory: `tests/readability/2026-10-05-e41258c/`; its `notes.md` holds the
method, every deviation from the protocol, the results and the analysis of
every failing sample. Session 3 collected the Haiku samples and part of the
Sonnet ones and fixed an answer-key leak in `prepare`; session 4 collected
the rest, judged every lint-clean Complete and Write sample with a reason
(`outputs/<label>/judgement.json`), graded the sixty explanations with
Sonnet subagents, and ran `score` and `report`.

Results against the thresholds (Predict and Explain 90, Complete 80, Write 70):

| Label | Predict | Explain | Complete | Write (strict / after `renyi format`) |
|-------|---------|---------|----------|------|
| agent-sonnet | 100% | 100% | 74% | 50% / 70% |
| agent-haiku | 80% | 97% | 37% | 0% / 0% |

Sonnet's failures are mostly invented library names (the cheat sheet lists
none), plus one each of `group by ... sum`, an unused `count` loop variable,
and a superfluous `otherwise`. Haiku's are the rules the cheat sheet states
outright (`then`, `end`, reserved words, single-letter names). Session 4
also fixed two lint false positives found by the scoring, added
`score --format` and `--scores` to the harness, and stated the derived
`ToText` rule for variants in the library sketch.

Session 4 also added `renyi parse --json` (`crates/renyi_syntax/src/json.rs`).

## Questions put to the owner at the end of session 4

Asked with AskUserQuestion; the answers, once given, become decision entries
(section M of `01-decisions.md`) and sketch or cheat-sheet changes:

1. Query grammar: let `group by key` combine with `sum`, `count`, `first`,
   `any`, `all` and `collect` per group (both models wrote
   `group by sale.region sum sale.amount`; the parser already accepts it)?
2. J8 for query variables: is the loop variable of a bare `count` query
   (`for each line in order.lines count`) unused (fix: `order.lines.length()`)
   or consumed by `count`? (`first` returns the variable, so it counts as read.)
3. `otherwise` on a value that cannot fail (`Port(8080) otherwise crash`,
   `line.split(" ") otherwise fail`): error with the fix "remove otherwise",
   warning, or allowed?
4. A named single argument (`column_widths(table: table)`): error with the fix
   "drop the name", or accepted and normalized by the formatter?
5. Protocol: should Write and Complete scoring run `renyi format` before the
   lint (line width was 60% of Sonnet's Write violations, all fixable)?
6. Cheat sheet: add a library section (prelude text, list and map methods,
   module names) with the remaining budget (about 550 tokens)?
7. Haiku as the floor model (decision L1): keep it and iterate the cheat
   sheet, or move the floor up?

## Next steps

0. **Record the owner's answers** to the questions above as decisions, apply
   them (sketch, cheat sheet within budget, lint, the checker's rules), and
   re-judge the affected pre-test samples if a rule changed.
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
- `tools/lint_examples.py` is regex-based; its block/`end` balance and
  unused-binding checks are heuristics and can miss errors or flag correct
  code (two false positives were fixed in session 4; the whitespace and
  foreign-keyword checks now skip comments and clause text).
- Decimal rendering: `rounded(2)` renders with exactly two decimals (`6.00`),
  now stated in the library sketch; the examples depend on that.
- The lint's call check is coarse: it accepts any name declared anywhere in
  the library sketch or the corpus, not per receiver type.
- The parser accepts the corpus but has only been tested against it and its
  unit tests; malformed input beyond the unit tests may still cascade into
  several diagnostics for one mistake.
- The ten reference outputs under `tests/readability/reference/` were derived
  by hand from the sketches (Decimal and Float rendering rules included); the
  VM at M3 is the first independent check of them, as it is of the 29
  Complete and Write verdicts in the pre-test's `judgement.json` files.
- The pre-test's Explain grades came from Sonnet subagents grading ten items
  per call and are uniformly high; the live round should use two graders.
- The GitHub default branch was set automatically to the first pushed branch;
  the owner should switch it to `main` in the repository settings.

## Owner actions pending

- Answer the seven questions above (or say which to defer).
- Set `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` in the cloud environment so the
  next session can run the readability round (decision L1).
- Switch the GitHub default branch to `main`, and say when `main` should be
  updated from the session branch.

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (for example deriving comparisons from `is`).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches over prose.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews.
