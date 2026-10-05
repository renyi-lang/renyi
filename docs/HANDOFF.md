# Handoff

Last updated: 2026-10-05, session 2 in progress.
Branches: `main` holds the session-1 handoff; `claude/renyi-language-design-hbrie3`
carries session 2.

## Where the project stands

Milestone M0 (design) is about three quarters done. Round 1 of design
decisions is complete and recorded. The surface syntax is sketched in full. The
corpus has reached its target of 30 programs and passes the lint. The cheat
sheet measures 2249 tokens against a 3000-token budget. No compiler, parser or
runtime exists.

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
3. Taught the lint about `"""` blocks, one-line `if ... end`, and ability
   declarations with type parameters.

## Next steps, in order

1. **Round 2 decisions.** Ask the owner the open items R2-1 to R2-16 from
   section 18 of the syntax sketch, four at a time. The ones that change the
   most code: R2-11 (upward subtyping for `type X is Base`), R2-1 (map
   literals with braces), R2-2 (whether documentation clause heads such as
   `purpose`, `tags`, `example` are reserved identifiers), R2-13 (contracts on
   parameters), R2-14 (interpolation in every literal versus a raw form).
   Record answers as new entries in `01-decisions.md`.
2. **Standard library sketch.** The corpus uses `std.console`,
   `std.environment`, `std.filesystem`, `std.json`, `std.http`, `std.csv`,
   `std.server`, `std.time`, `std.random`, `std.sqlite` with ad hoc function
   and method names, plus the list, map, set, text and number methods listed
   in sketch section 7. Collect them into `docs/design/04-stdlib-sketch.md` so
   the names stay consistent and the lint can check them.
3. **Readability test harness.** Implement `docs/design/03-readability-test.md`
   under `tests/readability/`. It needs API access to at least three models
   from two vendors; confirm credentials with the owner first.
4. **M1 in Rust.** Cargo workspace with crates for lexer (phrase table, longest
   match), parser (clause grammar, continuation rules), formatter (canonical
   layout from sketch section 16), and a conformance test runner driven by the
   corpus. The owner chose M3 (a VM running the corpus) as the first demo.

## Known gaps and risks

- The standard library surface is invented per example and not yet specified
  (see step 2). New in session 2: `std.time` (`Date`, `Weekday`,
  `InvalidDate`, `time.today`, `time.parse_date`, `plus_days`, `weekday`),
  `std.sqlite` (`Connection`, `DbError`, `open`, `query`, `execute`,
  `integer`, `text`, `close`, derived `FromRow`), `std.filesystem`
  (`inspect`, `list`, `Entry` with `File(size)`, `Directory`, `Other`,
  `path.name()`), `std.http` (`post` with `url`, `body`, `headers`;
  `response.status`), `std.json.render`, `std.environment.get`.
- 81 reserved words include common identifiers (`count`, `first`, `sum`,
  `set`, `type`, `test`, `check`, `run`, `group`, `power`, `tags`, `example`).
  If the corpus shows frequent collisions, revisit R2-2 and the phrase table.
- The token budget is measured with tiktoken `o200k_base` and `cl100k_base` as
  proxies; no tokenizer for Claude models is public.
- `tools/lint_examples.py` is regex-based; its block/`end` balance check is a
  heuristic and can miss errors.
- Decimal rendering: the sketch assumes `rounded(2)` renders with exactly two
  decimals (`6.00`); the examples depend on that.
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
