# Handoff

Last updated: 2026-10-05, end of session 5 (the first live readability
round run, scored, judged and adjudicated; `renyi index` built and measured;
`main` made the only branch). Branch: `main` is the only branch (owner's
decision, 2026-10-05); commit and push there directly.

## Where the project stands

Milestones M0 (design), M1 (front end) and M2 (type and effect checker) are
done, and the project map (`renyi index`, decision O2) exists; M3 (the VM)
is next. Design decisions are in sections 0 to Q of `01-decisions.md`; the
agent tooling in `05-agent-tooling.md`, the signature capabilities in
`06-runtime-guarantees.md`, the system-level commitments in
`07-system-design.md`. The corpus has 30 programs, passes the lint, is in
canonical layout and checks cleanly. The cheat sheet measures 2877 of 3000
tokens with tiktoken (the gate's maximum over `o200k_base` and
`cl100k_base`). The Rust workspace has four crates: `renyi_syntax`,
`renyi_check`, `renyi_index` and the `renyi` binary with `check`, `format`,
`tokens`, `parse [--json]`, `index [--json]` and `version`; 91 tests, clippy
and fmt clean on Windows and Linux. Nothing runs yet: that is M3.

The first live readability round (`tests/readability/2026-10-05-1623155/`,
its `notes.md` has the method, every deviation, the results and the
analysis) ran Sonnet 5.5, Haiku 4.5 and gpt-5.4-mini with five samples per
prompt. Results by the four-of-five rule against the thresholds (Predict and
Explain at least 90, Complete at least 80, Write at least 70, every model;
the protocol tally formats before the lint, the strict one does not):

| Label | Predict | Explain | Complete (protocol / strict) | Write (protocol / strict) |
|-------|---------|---------|----------|-------|
| claude-sonnet-5-5 | 90% | 97% | 79% / 63% | 40% / 10% |
| claude-haiku-4-5-20251001 | 30% | 90% | 37% / 32% | 0% / 0% |
| gpt-5.4-mini | 80% | 97% | 32% / 32% | 0% / 0% |

No model meets every threshold, so the grammar is not frozen; the failing
thresholds are questions for the owner (below), not silent changes. Sonnet
passes Predict and Explain and misses Complete by one item; `purpose-missing`
is 28 percent of its Write violations, over the protocol's quarter rule.

## Done in session 5

1. **Branch consolidation.** The session branches were fast-forwarded into
   `main` and deleted; `main` is the GitHub default and the only branch.
   `CLAUDE.md` says so.
2. **Windows build.** `.gitattributes` pins LF (the checkout had CRLF, which
   the layout check rejects); 485 files were rewritten to LF once. `cargo
   test`, clippy and fmt are clean. The token count was confirmed with real
   tiktoken encodings.
3. **`renyi index`** (`crates/renyi_index`): definition and module records,
   the six metrics, content hashes (SHA-256 over canonical text with the own
   name removed and references replaced by dependency hashes; strongly
   connected components hashed together), text and `--json` forms. The
   checker records every reference it resolves (`renyi_check::Reference`)
   and `check_project` checks a project as a whole. The corpus is measured
   in `05-agent-tooling.md` section 5 (R5-1): proposed budget thresholds 10
   public definitions per module, 5 transitive effect paths per module,
   fan-out 7, awaiting the owner.
4. **The live readability round** (step 1 of the previous handoff). The
   harness gained, each change forced by the round: run selection by newest
   `meta.json` and `--run`; retries with backoff; `--temperature none`
   (Sonnet 5.5 rejects the field); imported corpus modules shown in the
   prompts; `renyi check` after the lint for Complete and Write; LF scratch
   files on Windows, one per process (three `score` processes run at once
   shared one file and crashed); Explain graded against author's
   descriptions of every program
   (`tests/readability/reference/<program>.explain.txt`, written this
   session) with a rubric, two graders, adjudication of disagreements over
   one point and refusals deciding nothing; the grade parsed as the last
   standalone digit (the first digit was the 1 of "18" when the grader
   reasoned first); grades cached with the grader's answer. The 182 lint-
   and check-clean Complete and Write samples were judged by ten subagents
   with reasons (`outputs/<label>/judgement.json`, decision L2): 83 of 91
   distinct programs pass. The 49 Explain samples the graders disagreed on
   were adjudicated by hand in the same file (36 pass); `shipping_rules`
   fails Explain on every model because its `weight_steps` purpose line
   said "whole kilograms" where the code counts started kilograms.
5. **Cheat sheet**: states that a derived `ToText` prints a variant as its
   bare name (Sonnet's only Predict failure came from doubting it).
6. **Corpus**: the purpose line of `weight_steps` in `shipping_rules.ry`
   now says "started kilograms beyond the first" (the round's stored
   prompts keep the old wording).

## Owner actions pending

- **Rotate `ANTHROPIC_API_KEY` and `OPENAI_API_KEY`.** Session 5 printed
  both values into a tool result once while checking that the variables
  were set (not committed, but in the session transcript). Rotate both and
  set the new values in the environment.
- Answer the question batches of session 5 (below); if the session ended
  before they were asked, the next session asks them first.

## Questions for the owner from the round (session 5)

Each is a measured cost, with the recommendation first:

1. Haiku 4.5 as the floor model (M7) fails Predict at 30 percent from
   arithmetic and attention errors, not grammar; the thresholds say "every
   model". Gate the freeze on Sonnet-class models and keep Haiku as a
   reported trend, or keep "every model".
2. The reserved word `count` as a local name costs Sonnet two Complete items
   (17 of 19 without it, 89 percent, above the threshold); `sorted` and
   `first` cost the small models more. Make the query words contextual
   (reserved only inside a query) or keep J3.
3. `purpose:` on public types and on the module is the largest single Write
   rule for Sonnet and gpt (28 and 25 percent of violations; the protocol's
   quarter rule). Keep it (and say it louder in the cheat sheet) or require
   it on functions only.
4. A named single argument (M4) is the largest checker rule for Haiku (92
   samples) and gpt (77). Accept the named form when the name matches the
   parameter (the formatter drops it) or keep the error.
5. Decimal `is` and scale: `0.00 is 0` and `32.0 is 32` are undefined in the
   documents; the judges and the checker's refinement evaluation assume
   IEEE value equality. State it, or make scale significant.
6. `ignore pop(stack)` on an immutable value silently does nothing (three
   judged failures). Restrict `ignore` to calls with effects (a discarded
   pure result is dead code) or leave J15 as it is.
7. R5-1 budget thresholds (10 / 5 / 7) as the first defaults.
8. A fourth model: `gpt-5.5` rejects temperature 0 and was not run; run it
   at its default temperature as Sonnet was, or leave three models.

## Done in sessions 1 to 4 (condensed)

- Session 1: ~50 design decisions in interactive batches (`01-decisions.md`
  sections 0 to I), the clause grammar and the three verbs, the cheat sheet
  with its token gate, the lint, 14 example programs, the readability
  protocol (`03-readability-test.md`), README and license.
- Session 2: the corpus grown to 30 programs; round-2 decisions J1 to J18;
  the library sketch (`04-stdlib-sketch.md`) with decisions K1 to K11; the
  readability harness with the file provider; M1 started: lexer, parser,
  formatter, `renyi check`, `tokens`, `parse`, corpus conformance tests.
- Sessions 3 and 4: the subagent pre-test (`2026-10-05-e41258c/`, one sample
  per item, two Claude models) and decisions M1 to M10 from it (`group by`
  with any terminal, `count` leaves its loop variable unused, superfluous
  `otherwise` and a named single argument are errors, format before lint,
  the cheat sheet's library section, Haiku stays the floor model, an MCP
  server after M2, `repeat until` replaces `while`, no bottom-tested loop);
  `renyi parse --json`; the standard library as declaration files
  (`library/std/*.ry`, decisions N1 to N3); **M2, `crates/renyi_check`**
  (world, bodies with bidirectional inference, subtyping, implicit `maybe`,
  generics, abilities, refinements on literals, fallible calls, error unions,
  exhaustiveness, queries, scoped effects; 21 rule tests; the 40 judged
  pre-test samples agree with the checker); decisions O1 to O5 (reference
  counting with in-place reuse supersedes the tracing collector; the project
  map; budgets; the semantic diff; `renyi mcp`), P1 to P4 (recorded runs
  and `replays` tests, budgets in grants, provenance guards, the reserved
  words `only`, `per`, `replays` and phrases `at most`, `only to`) and Q1 to
  Q4 (capability-safe packages, reproducibility, in-process sandboxing,
  checked live update), each with its design document.

## Next steps

1. **Close the round with the owner**: ask the eight questions above
   (batches of four, recommended option first); record the answers as
   decision entries (section R of `01-decisions.md`); apply any grammar
   change to the sketch, the cheat sheet (token gate), the lint, the parser,
   the checker and the corpus together; re-run the readability round on the
   changed grammar (a change that lowers a passing rate by more than five
   points is reverted, protocol rule). `renyi index --budgets` can take the
   R5-1 defaults once confirmed.
2. **M3: the bytecode VM** (`renyi run`, `renyi test`, `renyi record`), the
   owner's first demo, under decision O1 (reference counting, in-place
   reuse, last-use moves). Plan: a new crate `renyi_vm` over
   `renyi_check::World` and `Ty`; values (Integer i64 with overflow crash,
   Decimal as IEEE decimal128 through the `dec` crate, libdecnumber
   bindings, since `rust_decimal` has 28 digits and is not J16; Float f64
   without NaN; Text, Bytes, List, Map and Set with insertion order, records,
   variants, maybe, functions by name); a compiler from the checked AST to
   bytecode with last-use marks; green threads on one OS thread for `run
   concurrently`, `concurrently` queries and `within`; the capability
   sandbox at startup (`--deny`, `--allow-host`, `--allow-read`, the grant
   stack of Q1 with scope checks at primitives); the primitive boundary
   carrying the recorder, the replayer, the budget counters and the
   narration hook from the first version (`06-runtime-guarantees.md`
   section 4); the library primitives (console, environment, time, random,
   filesystem, json, csv, http, server, sqlite, regex). Conformance: the
   thirty corpus programs, the ten reference outputs, every `example:` line
   and `test` block, and the 182 judged samples of the live round plus the
   40 of the pre-test (decision L2: the VM re-runs every verdict and the two
   sets are compared). The five network programs get recordings and
   `replays` tests.
3. **`renyi mcp`** (O5, `05-agent-tooling.md` section 7), then budgets and
   the semantic diff (O3, O4), then M4 (provenance guards, package manager)
   and M5 (embedding API, `serve --watch`, LSP) as before.

## Known gaps and risks

- The live round's Explain grades come from two models with a rubric and
  author's descriptions; the 49 disagreements over one point were
  adjudicated by Claude in the session (reasons in `judgement.json`).
  Haiku's Explain pass sits exactly at the 90 percent threshold and rests on
  those adjudications. The descriptions were written by Claude from the
  programs; the owner may want to spot-check a few.
- The Complete and Write verdicts come from Claude Code subagents (decision
  L2); three semantic assumptions are recorded in their reasons (Decimal
  `is` ignores scale, a sliding window excludes its far edge,
  `EmergencyCleared` outside an emergency gives `Red`). The VM re-runs them
  at M3.
- The ten Predict reference outputs were derived by hand (and re-derived
  this session for the seven items Haiku failed); the VM is the first
  independent check.
- Sonnet 5.5 cannot be sampled at temperature 0 (the field is rejected), so
  its five samples vary; the protocol's "temperature 0 where allowed" is
  met, but Sonnet's rates carry more sampling noise than the others'.
- The standard library sketch is a first draft from the corpus; JSON
  derivation rules, the SQLite type mapping and HTTP defaults are not
  validated against real data.
- 88 reserved words include common identifiers (`count`, `first`, `sum`,
  `sorted`, ...); the round measured their cost (question 2 above).
- The grant clauses (`at most`, `only to`, `replays`) have not been through
  a readability round; no corpus program uses them yet.
- `tools/lint_examples.py` is regex-based; its block balance and unused
  binding checks are heuristics (they caught real errors in the round but
  also decide samples before the checker sees them).
- The checker is first-generation (local inference, covariant type
  arguments, structural ability checks, capabilities checked at call sites
  only; no `example:` literal check, no `see also` check, no `deprecated`
  warnings, no unreachable-pattern detection).
- Decision O1 (reference counting) rests on the value graph being acyclic;
  any feature that lets a value refer to itself must be checked against it.

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (for example deriving comparisons from `is`).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches of four, recommended
  option first, over prose.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews. `main` is the only branch.
