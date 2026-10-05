# Handoff

Last updated: 2026-10-05, end of session 4 (pre-test finished and decided,
M1 wrapped up, M2 type and effect checker green on the corpus, runtime
memory model, agent tooling, the three signature capabilities and the four
system-level commitments designed; the new syntax is in the grammar).
Branch: `main` is the only branch (owner's decision, 2026-10-05); the session
branches were fast-forwarded into it and deleted, and it is the GitHub default.

## Where the project stands

Milestones M0 (design), M1 (front end) and M2 (type and effect checker) are
done; the project map (`renyi index`) and M3 (the VM) are next. Design
decisions are recorded in sections 0 to P of `01-decisions.md`; the agent
tooling (project map, budgets, diffs, `renyi mcp`) is designed in
`05-agent-tooling.md` and the signature capabilities (recorded runs with
`replays` tests and narration, budgets in grants, provenance guards) in
`06-runtime-guarantees.md`, and the system-level commitments (capability-safe
packages, reproducibility, in-process sandboxing, checked live update) with
the trade-offs the language claims to resolve in `07-system-design.md`; the surface syntax and the standard
library are sketched in full, and the library also exists as declaration
files the compiler reads (`library/std/*.ry`). The corpus has its target 30
programs, passes the lint, is in canonical layout, and type- and
effect-checks without a diagnostic. The cheat sheet measures about 2770
tokens (byte estimate; 2890 after the grant and `replays` lines) against the
3000-token budget, so the sheet is nearly full. The Rust workspace has
three crates: `renyi_syntax` (lexer, parser, JSON encoder, formatter),
`renyi_check` (the checker) and the `renyi` binary with `check` (parse plus
type and effect check), `format`, `tokens` and `parse [--json]`. 77 tests,
clippy and fmt clean. Nothing runs yet: that is M3. The readability harness
has run once as a subagent pre-test (below); the live round still waits for
API keys.

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

## Done in session 4: M1 wrap-up and M2

1. `renyi parse --json` (`crates/renyi_syntax/src/json.rs`), and the
   formatter keeps comments inside multi-line expressions where they were
   written (lists, arguments, query clauses, `otherwise` lines, arms,
   fields); an `otherwise` wrapped around a query lays out like the query.
2. `repeat until` replaced `while` (M9) throughout the front end, the lint,
   the corpus and the documents; the parser reports foreign statement
   keywords (`while`, `else`, `def`, `var`, ...) with the Renyi form.
3. The standard library as declaration files, `library/std/*.ry`, parsed in
   a declaration-only mode (`parse_declarations`); a test keeps them in step
   with the sketch. Decisions N1 (methods may carry reserved-word names) and
   N2 (Path, Url and Pattern are plain Text subtypes) came out of this.
4. **M2, `crates/renyi_check`**: `world.rs` declares every module (library
   and user) into tables of types, abilities, implementations, functions and
   constants and resolves imports and `exposing`; `check.rs` checks bodies
   with bidirectional inference and unification variables (number literals
   stay open until something fixes their type, then default to Integer or
   Decimal), subtyping through `type X is Base`, implicit `maybe` wrapping,
   generics with `for any` instantiated per call and constraints checked
   when known, methods by receiver type (overloads such as `sum` chosen by
   the receiver), ability methods and derived abilities, record and variant
   constructions with compile-time refinement checks on literals
   (`refine.rs`, with the `regex` crate for `matches`), fallible calls that
   must be handled by `otherwise` or `match`, error unions that narrow after
   `failure(error: T)` arms, `match` exhaustiveness, queries with per-group
   terminals, effects with scoped capabilities (`effects.rs`, J11; library
   primitives are scoped at run time) and the effects of functions passed by
   name, and the rules J8, J9, J15, M2, M3, M4. `renyi check` runs it after
   parsing. Tests: the 30 corpus programs check cleanly; `tests/rules.rs`
   has 21 rule tests; the 40 judged pre-test samples agree with the checker
   (every sample judged correct checks cleanly, every one judged wrong for a
   type-level reason raises an error).
5. Corpus fixes the checker forced: `inventory_db.ry` exposes `FromRow`,
   `sales_report.ry` joins text by interpolation instead of `+`, `retry.ry`
   reads its loop counter. Decision N3: library error types derive `ToText`.

## Decisions taken in session 4 (entries M1 to M10 of `01-decisions.md`)

The pre-test results were put to the owner as questions; every answer is
recorded and applied:

- M1 `group by key` combines with any terminal clause (per-group `sum`, ...).
- M2 the loop variable of a bare `count` query is unused (J8 applies; fix
  `.length()`); the variable of a `first` query counts as read.
- M3 `otherwise` on a value that cannot fail is a compile error.
- M4 a named single argument is a compile error ("drop the name").
- M5 the readability scoring runs `renyi format` before the lint (harness
  default; `--no-format` keeps the strict tally).
- M6 the cheat sheet has a library section (names only); it measures about
  2770 tokens by two byte-based estimates because the tiktoken encodings
  could not be downloaded through the proxy; CI with network access must
  confirm the count.
- M7 Haiku 4.5 stays the floor model.
- M8 an MCP server for the toolchain is scheduled after M2.
- M9 `repeat until condition ... end` replaces `while` (85 reserved words,
  15 phrases); `while` and other foreign keywords at statement start get a
  diagnostic with the Renyi form. Lexer, parser, formatter, JSON, lint,
  cheat sheet, sketch and three corpus programs were changed together.
- M10 no bottom-tested loop for now.

## Decisions taken at the end of session 4 (entries O1 to O5)

The owner asked how memory is managed, what sets the language apart and
what it can offer agents, and took the recommendations as given:

- O1 memory is reference counting with in-place reuse of uniquely held
  values (the value graph is acyclic: immutable values, no references, no
  closures); supersedes E2's tracing collector. v1 runs green threads on
  one OS thread, so counts are not atomic.
- O2 the project map (`renyi index`, JSON, incremental by content hash) is
  the agent's view of a project: one record per definition with signature,
  purpose, declared and transitive effects and failures, edges and six
  metrics (lines, depth, branches, effects, fan-in/out, coverage).
- O3 module-level complexity budgets are reported by the map and gated in
  CI as warnings; thresholds after measuring the corpus.
- O4 `renyi index --diff` reports semantic changes (signatures, effects,
  failures, hashes) with the callers they reach; G1's version bump reads it.
- O5 the toolchain MCP server is `renyi mcp` (JSON-RPC over stdio), distinct
  from `renyi serve --mcp` of D6; `serde_json` is taken as a dependency.

## Decisions P1 to P4: the signature capabilities

The owner asked for at least one capability no other language has had and
chose three of four candidates (the fourth, nothing, was not chosen):

- P1 **recorded runs**: `renyi record` writes every effect call of a run;
  `test ... replays "path"` answers a test's effects from the recording,
  offline and deterministic; `renyi run --replay` re-executes a run;
  `--explain` narrates any run with the `purpose:` clauses (the
  self-narrating layer). M3.
- P2 **budgets in grants**: `needs network.http("host") at most 60 per
  minute` on `main` or a test; the runtime counts and fails the call over
  budget. M3.
- P3 **provenance guards**: `needs filesystem.read("secrets") only to
  network.http("host")`; values from a guarded capability carry their
  origin at run time and may leave only through the listed sinks; the
  answer to exfiltration by prompt injection. Design now, runtime in M4.
- P4 reserved words 88, phrases 17 (`only`, `per`, `replays`; `at most`,
  `only to`). The parser, formatter, JSON encoder, lint, checker
  (`grant-clause`: budgets and guards belong on `main` or a test), sketch
  and cheat sheet carry the syntax; no corpus program uses it yet.

## Decisions Q1 to Q4: system-level commitments

The owner asked for system-level innovations and took all four:

- Q1 **capability-safe packages**: computed effect manifests in the
  registry, `main`'s grant covers every dependency, no silent widening on
  update, scopes narrow dynamically through the grant stack. M3/M4.
- Q2 **reproducibility by construction**: a run manifest and `renyi
  reproduce`. M3/M4.
- Q3 **in-process sandboxing**: the grant is the isolation boundary; a host
  loads a module with a grant (plus a memory budget, R7-1). After Q1, M5.
- Q4 **checked live update**: swap changed definitions by hash between
  requests after the semantic diff passes; no hidden state to migrate. M5.

`07-system-design.md` section 1 states the eight trade-offs Renyi claims to
resolve; they are claims to test, and the readability round and M3 test
the first ones.

## Next steps

0. **Confirm the cheat-sheet token count** with `python3 tools/count_tokens.py`
   where the tiktoken encodings can be fetched (the gate falls back to a
   byte estimate offline and says so).
1. **Run the first live readability round** once the two API keys are present
   in the environment (a new session picks them up): `prepare`, then `run` for
   `claude-sonnet-5-5`, `claude-haiku-4-5-20251001` and one OpenAI model,
   `score --grader anthropic:claude-sonnet-5-5` (two graders where they
   disagree), judge the pending Complete and Write samples into
   `judgement.json` with reasons, `report`, and commit the run directory.
   Compare against the acceptance thresholds in `03-readability-test.md`; a
   failing threshold becomes a grammar question for the owner, not a silent
   change. `repeat until` (M9) gets its first measurement here.
2. **`renyi index`, the project map** (`05-agent-tooling.md`, sections 1
   to 4): a new crate `renyi_index` over `renyi_check::World`: the
   definition and module records, the six metrics with the definitions
   given there, content hashes (a 256-bit hash of the canonical text with
   the definition's own name removed and dependency names replaced by
   hashes; strongly connected components hashed together), the text form
   and `--json`. Measure the corpus and write the numbers into the design
   document as the first budget data (R5-1). About a day of work.
3. **M3: the bytecode VM** (`renyi run`, `renyi test`, `renyi record`), the
   owner's first demo, under decision O1 (reference counting, in-place
   reuse, last-use moves): run the thirty corpus programs; the ten reference
   outputs under `tests/readability/reference/` and the `example:` lines
   and `test` blocks of the corpus are the first conformance expectations,
   and the 40 judged pre-test samples are a second check (decision L2).
   Decimal128 arithmetic, the capability sandbox at startup, structured
   concurrency on one OS thread and the library primitives (console,
   filesystem, json, http, csv, sqlite, regex, time) are the bulk of it;
   the checker's `World` and `Ty` are the input. The primitive boundary
   carries the recorder, the replayer, the budget counters and the
   narration hook from the first version (`06-runtime-guarantees.md`,
   section 4); the five network programs of the corpus get recordings and
   `replays` tests. The grant stack (Q1, Q3) and the run manifest with
   `renyi reproduce` (Q2) belong to the same runtime.
4. **`renyi mcp`** (decision O5, `05-agent-tooling.md` section 7): the first
   seven tools, then `run`, `run_tests` and `diff` as M3 and `--diff` land.
5. **Complexity budgets and the semantic diff** (O3, O4), then M4 reads
   them from the manifest.
6. **M4 also carries the provenance guards** (P3, `06-runtime-guarantees.md`
   section 3): origin sets on heap values, the `only to` check at outgoing
   primitives, the `environment` default (R6-6); and the package manager
   with computed effect manifests, `renyi add`, `update --accept-effects`
   and `audit` (Q1, `07-system-design.md` section 2).
7. **M5**: the embedding API with grants and memory budgets (Q3), `renyi
   serve --watch` and checked swaps (Q4), the LSP, index and compiler API
   as planned.

## Known gaps and risks

- The standard library sketch is a first draft written from the corpus; its
  JSON derivation rules, SQLite type mapping and HTTP timeout default have
  not been validated against real data.
- 85 reserved words include common identifiers (`count`, `first`, `sum`,
  `set`, `type`, `test`, `check`, `run`, `group`, `power`, `tags`, `example`,
  `repeat`, `until`). The owner confirmed in round 2 (J3) that they stay
  reserved; the fix for a collision is a rename suggestion from the compiler.
- `repeat until` (M9) and the grant clauses `at most`, `only to` and
  `replays` (P1 to P3) have not been through a readability round yet; the
  pre-test samples were written with `while`. A drop of more than five
  points in the live round reverts a change (protocol rule).
- The cheat sheet is at about 2890 of 3000 tokens by byte estimate; the
  next grammar addition must make room, and the real tokenizer count is
  still owed.
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
- The checker is first-generation: inference is local to a body, `assign`
  treats type arguments covariantly (sound for immutable values), ability
  checks for `ToJson`/`FromJson`/`FromRow` are structural only (M4 validates
  derivations against real data), `Equal` is assumed for every data type,
  and a function's capabilities are checked at call sites only. Known gaps:
  no check that an `example:` argument is a literal, no check of `see also`
  references, no `deprecated` warnings, no unreachable-pattern detection,
  and `with` on a refined field re-checks only literals.
- J8 makes the counter of a counted loop (`for each attempt from 1 to
  attempts`) an error when the body never reads it; the corpus now reads
  it. Watch the live round for this cost.
- The ten reference outputs under `tests/readability/reference/` were derived
  by hand from the sketches (Decimal and Float rendering rules included); the
  VM at M3 is the first independent check of them, as it is of the 29
  Complete and Write verdicts in the pre-test's `judgement.json` files.
- The pre-test's Explain grades came from Sonnet subagents grading ten items
  per call and are uniformly high; the live round should use two graders.
- Decision O1 (reference counting) rests on the value graph being acyclic;
  any future feature that lets a value refer to itself (a `lazy` thunk over
  its own binding, mutable fields, closures) would need a cycle collector
  and must be checked against O1 first.

## Owner actions pending

- Set `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` in the cloud environment so the
  next session can run the readability round (decision L1).

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (for example deriving comparisons from `is`).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches over prose.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews.
