# Handoff

Last updated: 2026-10-06, session 6 (the VM: `renyi run` and `renyi test`,
then the primitive boundary: `renyi record`, `run --replay`, `--explain`,
`replays` tests, budgets and scope checks). Branch: `main` is the only
branch (owner's decision, 2026-10-05); commit and push there directly.

## Where the project stands

Milestones M0 (design), M1 (front end) and M2 (type and effect checker) are
done; the project map (`renyi index`, decision O2) exists with its budget
report (`--budgets`, decisions O3 and R7); M3 (the VM) runs programs and
implements decisions P1 and P2 (recorded runs, budgets) at its primitive
boundary; what M3 still lacks is the network, server and SQLite primitives
and real concurrency. Design decisions are in sections 0 to R of `01-decisions.md`;
the agent tooling in `05-agent-tooling.md`, the signature capabilities in
`06-runtime-guarantees.md`, the system-level commitments in
`07-system-design.md`. The corpus has 30 programs, passes the lint, is in
canonical layout, checks cleanly, has nothing over budget, and its 82
`example:` lines and `test` blocks pass on the VM. The cheat sheet measures
2977 of 3000 tokens (unchanged this session). The Rust workspace has five
crates: `renyi_syntax`, `renyi_check`, `renyi_index`, `renyi_vm` and the
`renyi` binary with `check`, `format`, `tokens`, `parse [--json]`, `index
[--json | --budgets]`, `run [options] <file> [arguments]`, `record [--to
file] [options] <file> [arguments]`, `test [--strict] [--refresh name]
[--explain] <file>...` and `version`; 119 tests, clippy and fmt clean on
Windows.

The first live readability round (`tests/readability/2026-10-05-1623155/`)
stands as session 5 left it: Sonnet 5.5 passes Predict (90) and Explain
(97), misses Complete by one item (79) and Write by three (40); Haiku 4.5
and gpt-5.4-mini are below on all but Explain. Since decision R1 only the
large model of each vendor gates the freeze. The grammar is not frozen; the
owner's decisions R1 to R8 are applied; the cheat sheet changes of R3, R5
and R6 are unmeasured (round 2, below).

## The VM as it exists (`crates/renyi_vm`)

- **Pipeline.** `renyi run file.ry` reads the file and its imports,
  `check_project`s them (an error stops here, printed as `check` prints
  it), `compile_project` lowers every non-library body to bytecode, and
  `run_main` runs `main` with the command-line arguments. `renyi test`
  compiles the same way and runs every `example:` line and `test` block in
  source order, one line each, then `N passed, M failed[, K skipped]`;
  exit 1 when any fails. A failing `main` prints `main failed with <error>`
  (exit 1); a crash prints the message and `file:line` (exit 2);
  `environment.exit(code)` exits with the code.
- **Compiler** (`compile/`). One `Code` per function body, test, constant,
  `example:` line (one for the call, one for the expected value or the
  `fails with` pattern) and refinement condition. Names are never resolved
  by the compiler: it reads the references the checker recorded
  (`Target::Function`, `AbilityMethod`, `Constant`, `Type`, `Variant`,
  `Number` for the type of every numeric literal and of every `sum`,
  `Result` for a call whose result type only the context decides, such as
  `json.parse`). Locals are slots on the value stack; `set x to
  x.method(...)` loads the receiver with `LoadMove` when the arguments do
  not read `x`, so lists, maps and sets grow in place (decision O1).
- **Failures.** A fallible call that fails leaves a `Failure` value. The
  value of an `otherwise` and the subject of a `match` with
  `success`/`failure` arms are *handled regions* (`PushHandler` /
  `PopHandler`); a failure inside one unwinds the operand stack to the
  region's start and jumps to the handler, where the fallback outcome runs
  (`otherwise fail` passes the same failure on). Outside any region a
  failure is the result of an example or test, and a crash
  (`unhandled failure: ...`) in a function, which the checker should have
  prevented. Constructing a refined type runs its compiled conditions;
  one that does not hold gives `Failure(ConstraintViolation(type_name,
  detail))` with the condition's source text as the detail.
- **Values** (`value.rs`). `Integer` is `i64` with a `BigInt` spill
  (decision B9), `Decimal` an emulation of IEEE decimal128 (34 digits,
  round half even, exponent kept, so `9.50 + 5.00` prints `14.50`; division
  exact when it terminates, else rounded; decision J16, R5: equality by
  value), `Float` is `f64`. `maybe` is the value or `Nothing`. Records,
  variants, lists, maps and sets (insertion order, K9) are `Rc`; a subtype
  value (`Path`, `Permission`) is its base value. `Date` is the library's
  record, `Instant` and `Duration` are milliseconds.
- **Derived abilities** (`render.rs`). `ToText`: a declared `to_text` of
  the value's type (a user implementation or a library primitive such as
  `Date`'s) when there is one, else a variant as its bare name, a record in
  constructor form with nested text quoted, lists `[1, 2]`, maps `{k: v}`,
  sets `[a, b].to_set()`, `nothing`. `Compare`: a declared `compare`, else
  records by their `can Compare by` fields or every field, variants by
  position then fields; `Equal` and `Hash` are structural.
- **Natives** (`natives/`). All of the prelude; `std.console`,
  `std.environment`, `std.time` (civil-date arithmetic, ISO instants),
  `std.random` (xorshift, seeded from the clock), `std.filesystem`,
  `std.json` (own reader and writer; decoding by the context type with the
  derivation rules of `04-stdlib-sketch.md` section 7, `as` names and
  `Naming`; `Constraint` errors from refinements), `std.csv` (RFC 4180
  cells, rows with line numbers), `std.regex` and `Text.matches` on the
  `regex` crate. `std.http`, `std.server` and `std.sqlite` have no
  primitives: a live call crashes with `... is not available in this build
  of the VM` (a replayed one is answered from the recording).
- **The primitive boundary** (`Vm::call_native`, `grant.rs`,
  `recording.rs`; `06-runtime-guarantees.md` sections 1, 2 and 4). A
  primitive with `needs` passes one function. Its *effect* is its declared
  capability plus the scope its argument names (the normalised path, the
  host of the URL, the variable). `begin_run` sets the run's grant: the
  `needs` of `main` or of the test, narrowed by `--deny` (which refuses to
  start when any function needs the capability), `--allow-host`,
  `--allow-read`, `--allow-write` (an ancestor such as `filesystem` is
  split into its children first), with one counter per `at most` budget,
  declared or added by `--at-most`. An effect outside the grant fails with
  `FileError.PermissionDenied` or `HttpError.HostNotAllowed`, past a budget
  with `OverBudget` (added to both error types this session), and crashes
  when the primitive cannot fail. Live, the call runs and is appended to
  the recording (`Call { capability, primitive, arguments by name,
  outcome success/failure, duration_ms, at_ms }`, JSON by the `ToJson`
  rules); replaying, the entry with the same primitive and arguments
  answers it (identical entries in sequence order), its outcome decoded by
  the declared result and error types (`FunctionMeta.returns`/`fails`; a
  type-parameter result uses the context type the checker now records for
  effectful primitives), nothing is written or sent, and a call the
  recording lacks crashes naming the call and the nearest recorded one.
  `--explain` narrates on stderr: `Purpose. (module.name, param: value)`
  on entry, `-> value` on exit, and each effect as `console "text"` or
  `capability name(args) -> result[, N ms]`, indented by call depth.
- **Not yet.** `run concurrently` and `concurrently` queries run their
  tasks one after the other (`within` sets a deadline that is checked
  between statements or items and fails with `TimedOut`); no green
  threads. The grant is the program's or the test's: the grant stack of
  decision Q1 (intersection along the call chain) is not built, so a
  function's own narrower `needs` scope is checked by the checker only.

## Done in session 6

1. **`crates/renyi_vm`** (about 8300 lines with tests): `bytecode.rs`,
   `value.rs`, `integer.rs`, `decimal.rs`, `types.rs`, `compile/{mod, expr,
   stmt, pattern, query}.rs`, `vm.rs`, `render.rs`, `natives/{mod, prelude,
   system, time, filesystem, json, csv, regex}.rs`, `runner.rs`;
   `tests/corpus.rs` runs the ten Predict programs of the readability
   manifest against `tests/readability/reference/*.out` (all ten match: the
   hand-derived references are confirmed), every `example:` and `test` of
   the corpus (82 items, all pass), and a failing and a crashing `main`.
2. **Checker changes the VM needed**, each recorded as a reference so the
   map is unaffected: `Target::Number(NumberKind)` on every numeric literal
   and on a `sum` query (the literal takes the type its context expects, so
   `Number(value: 4)` carries a Decimal); `Target::Result(Ty)` at a call
   whose return type mentions a type parameter no argument mentions
   (`json.parse`, `sqlite.query`), resolved at the end of the body;
   `Target::Variant`/`Target::Type` on variant patterns;
   `BodyLocation::Example` for the references of `example:` lines and
   `BodyLocation::Condition` for refinement conditions.
3. **Refinement conditions are now checked** (`check_type_conditions`): a
   subtype's `where value ...` and every field condition of a record or a
   variant is checked as a small body over the fields (previously they were
   not checked at all; a condition calling an unknown method passed). The
   corpus and the library needed no change; `rules.rs` has a test.
4. **`renyi run` and `renyi test`** in `crates/renyi/src/main.rs`.
5. `CLAUDE.md`, `README.md` and the status line of `06-runtime-guarantees.md`
   say what runs.
6. **The primitive boundary** (second commit of the session): `grant.rs`
   (effective grant, budget counters, `effect_of`, `parse_capability`,
   path and host normalisation), `recording.rs` (the format of section
   1.1 with `at_ms`, parsing, the replay's matching), `Vm::call_native`
   with the scope check, the budgets, recording, replay and narration;
   `runner.rs` runs `main` and each test under its own grant, loads a
   `replays` fixture beside the test's source file, `--strict` fails a
   test that leaves recorded calls unused, `--refresh NAME` re-records one
   test's fixture (`TestOutcome::Recorded`); the binary gained `record`,
   `--replay`, `--explain`, `--deny`, `--allow-host`, `--allow-read`,
   `--allow-write`, `--at-most`, `--strict`, `--refresh`.
   `tests/recording.rs` records and replays a run with console, time,
   random and filesystem effects, stops a replay at an edited entry,
   refuses a recording the grant does not cover, enforces `at most 2 per
   run` and a path scope, writes and replays a `replays` fixture offline,
   and checks the narration of a run. Checker: `records_result` also
   records the context type of an effectful library call whose result
   mentions a type parameter (`random.choice`), so a replay can decode it;
   `effects::scope_contains` is public. Library: `OverBudget` added to
   `FileError` and `HttpError` (sketch and `library/std`).

## Done in session 5 (condensed)

Branch consolidation into `main`; the Windows build (LF pinned by
`.gitattributes`); `renyi index` with metrics, content hashes and
`--budgets`; the first live readability round (harness fixes 1 to 9, 182
judged Complete and Write samples, 49 adjudicated Explain samples, the
results table above and the analysis in the round's `notes.md`); cheat
sheet clarifications (derived `ToText`, `purpose:`, `ignore`, `is` on
numbers); the `weight_steps` purpose line; decisions R1 to R8 applied (the
protocol gates on large models; J3 and M4 stand; `purpose:` stays; `is` on
numbers compares values of one type; `ignore` only on calls with effects,
the checker's `ignore-pure`; budgets 10 / 5 / 7; gpt-5.5 joins round 2);
`ce.toml` at the repository root (untracked, in `.git/info/exclude`) sets
CodeEraser's guard to `warn` because its 750-line budget refused appends to
`01-decisions.md` and `02-syntax-sketch.md` (owner's decision; recreate it
on a fresh clone).

## Owner actions pending

- **Rotate `ANTHROPIC_API_KEY` and `OPENAI_API_KEY`.** Session 5 printed
  both values into a tool result once while checking that the variables
  were set (not committed, but in the session transcript). Rotate both and
  set the new values in the environment.

## Done in sessions 1 to 4 (condensed)

- Session 1: ~50 design decisions in interactive batches (`01-decisions.md`
  sections 0 to I), the clause grammar and the three verbs, the cheat sheet
  with its token gate, the lint, 14 example programs, the readability
  protocol (`03-readability-test.md`), README and license.
- Session 2: the corpus grown to 30 programs; round-2 decisions J1 to J18;
  the library sketch (`04-stdlib-sketch.md`) with decisions K1 to K11; the
  readability harness with the file provider; M1 started: lexer, parser,
  formatter, `renyi check`, `tokens`, `parse`, corpus conformance tests.
- Sessions 3 and 4: the subagent pre-test (`2026-10-05-e41258c/`) and
  decisions M1 to M10 from it; `renyi parse --json`; the standard library
  as declaration files (`library/std/*.ry`, decisions N1 to N3); **M2,
  `crates/renyi_check`** (world, bodies with bidirectional inference,
  subtyping, implicit `maybe`, generics, abilities, refinements on literals,
  fallible calls, error unions, exhaustiveness, queries, scoped effects);
  decisions O1 to O5 (reference counting with in-place reuse; the project
  map; budgets; the semantic diff; `renyi mcp`), P1 to P4 (recorded runs
  and `replays` tests, budgets in grants, provenance guards, the reserved
  words `only`, `per`, `replays`) and Q1 to Q4 (capability-safe packages,
  reproducibility, in-process sandboxing, checked live update), each with
  its design document.

## Next steps

1. **The rest of M3**, in this order: (a) `std.http` (a small client over
   `std::net` or a crate the owner accepts; `HostNotAllowed` and
   `OverBudget` already come from the boundary), `std.server` and
   `std.sqlite` primitives, with recordings and `replays` tests for the
   five network programs (`weather`, `concurrent_fetch`, `pagination`,
   `assistant`, `currency_tool`) and the two database ones; the owner
   decides the dependencies (ask in one batch: an HTTP client crate or a
   hand-written one, `rusqlite` with the bundled SQLite or none yet, TLS);
   (b) the grant stack of decision Q1: intersect the run's grant with the
   `needs` scopes along the call chain, so that a function declared
   `needs filesystem.read("data")` cannot read elsewhere even when `main`
   may; (c) real concurrency for `run concurrently` and `concurrently`
   queries (green threads on one OS thread, or an explicit decision to keep
   them sequential with deadlines); (d) decision L2: re-run the 182 judged
   Complete and Write samples of the live round and the 40 of the pre-test
   on the VM and compare with the subagents' verdicts (the
   `judgement.json` files hold the verdicts; the VM now decides `is` on
   Decimals by value, R5); (e) open items R6-1 to R6-4 of
   `06-runtime-guarantees.md` (binary bodies, query narration, redaction,
   budgets as data) when the network primitives make them concrete.
2. **Readability round 2** on the revised cheat sheet, with gpt-5.5 at its
   default temperature as the fourth model (R8) and the R1 gating; the
   protocol reverts a change that lowers a passing rate by more than five
   points. The round costs API calls; the VM can replace the subagent
   judges for Complete and Write once (e) above has shown it agrees with
   them.
3. **`renyi mcp`** (O5, `05-agent-tooling.md` section 7), then the semantic
   diff (O4), then M4 (provenance guards, package manager, budgets in the
   manifest) and M5 (embedding API, `serve --watch`, LSP) as before.

## Known gaps and risks

- **VM.** `break` or `continue` as the outcome of an `if` or `match`
  *expression* nested inside another expression leaves that expression's
  partial operands on the stack (statements and loop bodies are clean); no
  corpus program does this. `Equal` and `Hash` are always structural: a
  user implementation of either is not called (the sketch derives `Equal`
  for every data type; `Hash` implementations are not in the corpus). A
  value's `IsType` test for a builtin (`when failure(error: Text)`) is by
  kind. `random` is seeded from the clock; a run is reproducible through
  its recording only. `Float.to_text` is Rust's shortest round-trip form.
  `Text.matches` compiles its pattern at every call. Durations print as
  `1.5s` / `250ms` (no decision covers the format). A refinement condition
  on a library type (`Date`, `Port`) runs on construction but its
  references are not recorded (the checker does not walk library bodies);
  only literals and local names occur there today.
- **Boundary.** A scope denial of a primitive that cannot fail
  (`filesystem.exists`, `environment.get` under `environment("HOME")`)
  is a crash, since the module has no error to return. A budget on a
  parent (`filesystem at most 10 per run`) stays one counter, but
  `--allow-read` splitting the parent into children copies the budget
  spelling onto each child in the recording's `grant` header. The
  replay matches arguments by their JSON, so a `Float` argument that
  prints differently after a change in `float_text` would not match. A
  replayed `--explain` shows no durations. `renyi record` of a program
  that reads stdin records the lines read (`console.read_line`), as
  designed. The recording's `revision` is `git rev-parse --short HEAD` of
  the program's directory.
- **Checker.** First-generation (local inference, covariant type arguments,
  structural ability checks, capabilities checked at call sites only; no
  `example:` literal check, no `see also` check, no `deprecated` warnings,
  no unreachable-pattern detection). The `ignore-pure` rule counts a call
  with effects anywhere inside the ignored expression. The new condition
  bodies bind every field of the record or variant, which the sketch
  (section 4: "over the field name") does not promise; a condition that
  reads another field type-checks and runs.
- **Readability.** The live round's Explain grades rest on two grader
  models and 49 adjudications by Claude; Haiku's Explain sits at exactly 90.
  The Complete and Write verdicts come from subagents (L2) until step 1(e)
  re-runs them. Sonnet 5.5 cannot be sampled at temperature 0, so its rates
  carry more sampling noise. The grant clauses (`at most`, `only to`,
  `replays`) have not been through a round; no corpus program uses them.
- **Library.** The standard library sketch is a first draft from the
  corpus; the JSON derivation rules are now exercised by the VM's decoder
  but not against real data; the SQLite type mapping and HTTP defaults are
  unvalidated.
- 88 reserved words include common identifiers; the round measured their
  cost and the owner kept them (R2). `tools/lint_examples.py` is
  regex-based. Decision O1 rests on the value graph being acyclic.

## Owner preferences observed

- Wants principled reasoning, asked explicitly for a mathematical angle on
  syntax; accepts recommendations readily but counters with concrete
  alternatives (deriving comparisons from `is`; typed comparison for `is`
  on numbers; keeping J3 and M4 against the measured cost).
- Chat in Chinese; all artifacts in English.
- Prefers questions as interactive option batches of four, recommended
  option first, over prose; answers within minutes.
- Implementation is to be written mainly by Claude in sessions; the owner
  reviews. `main` is the only branch.
