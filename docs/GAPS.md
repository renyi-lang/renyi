# Gap audit, 2026-10-06 (revision 9e24f68)

What the seven design documents and the decision record promise that the
implementation does not deliver, as it stands before the grammar freeze
and the self-hosting work. Four read-only audits ran in session 6, one per
area (the syntax sketch, the library sketch, the tooling and runtime
documents with the system design, and the decision record with the
milestones), each by a subagent that read its documents in full and
checked every promise against the crates with greps, code reading and
probe programs run on the release binary (the syntax audit alone wrote
about 140); the session then re-ran the probes behind every finding
marked "verified". The reports
themselves are in the session's lane, not in the repository; this file is
the consolidated list. Line numbers are of revision 9e24f68.

The audits found no missing library function (every one of the 189
declarations has a native) and the corpus, the tests, the lint and the
token gate all pass. The gaps are elsewhere: behaviour that contradicts
the design, checks the design promises that nobody wrote, surface syntax
that parses but does nothing, and three milestones not started.

## 1. Defects: behaviour that contradicts the design

Ordered by how much they matter to the language's promises.

1. Done in stage 1: **a `within` deadline fails instead of crashing.**
   `settle` hands an unhandled failure out of a function that declares
   it (`crates/renyi_vm/src/vm.rs:955`), so `otherwise` after the call
   runs and `main`'s own expiry ends the run with `TimedOut(after: 20ms)`
   (`crates/renyi_vm/tests/semantics.rs`,
   `an_expired_deadline_is_the_functions_failure`).
2. **Exhaustiveness has holes.** `check_exhaustive` returns true as soon as
   any arm records `Cover::Other` (`crates/renyi_check/src/check.rs:3633-3635`);
   literal patterns other than Boolean record `Other` (3452-3459), the
   inner patterns of `some`, `success`, `failure` and variant fields are
   discarded (3411-3451, 3592), and the error list is unused (3636), so
   `failure(error: Oops)` alone covers an `Oops or Ouch` union. Verified:
   integer-literal arms without `otherwise` check clean and crash at run
   time with "no arm of the match fits the value". Boolean and plain
   sum-type matches are checked.
3. Done in stage 1: **`with` runs the refinements again** (`vm.rs:1566`,
   `refined_record`, shared with construction), and an update whose new
   value is not a literal needs `otherwise` like a construction (decision
   U9; `missing-otherwise`, conformance case `with_refined.ry`). Tested
   in `crates/renyi_vm/tests/semantics.rs` (`an_update_keeps_the_refinements`)
   and `crates/renyi_check/tests/rules.rs`.
4. Done in stage 1: **privacy holds for functions, methods, constants and
   abilities** as it did for types: a call, a value or an `exposing` of a
   definition another module did not mark `public` is `private-name`
   (`check.rs`, `require_public`; `world.rs`, `resolve_imports`). Tested
   in `crates/renyi_check/tests/rules.rs`.
5. Done in stage 1: **a public constant of another module is read as
   `module.name`** (`check.rs`, `infer_member`; the VM compiles it to the
   constant's global, `compile/expr.rs`, `member`). Tested in `rules.rs`
   and `crates/renyi_vm/tests/semantics.rs`.
6. Done in stage 1: **an implementation's methods carry the ability's
   signature**: the parameters by name and type, the result and the
   failures, with `Self` read as the target (`world.rs`,
   `check_method_signature`; `method-signature`). Tested in `rules.rs`.
7. **Task independence is not enforced.** A `run concurrently` body is
   checked in the enclosing scope (`check.rs:1289-1298`); two tasks may
   `set` one outer mutable and a task may read a sibling's binding
   (verified: both check clean). The claim that a race cannot be written
   (decision E1, `07-system-design.md` row Concurrency) holds today only
   because tasks run one after the other (decision S2).
8. **The boundary crashes where the design says a failure.** Done in
   stage 1: `std.sqlite` fails with the new `DbError.PermissionDenied` and
   `DbError.OverBudget` (`library/std/sqlite.ry`; `vm.rs`, `denied` and
   `over_budget`), and `filesystem.copy` and `move` compare the target
   with the write scope too (`grant.rs:525`, `target_effect_of`; tested in
   `crates/renyi_vm/tests/semantics.rs`). Still open: the server and the
   primitives that cannot fail (`filesystem.exists`, `environment.get`)
   crash, and `StartError` has no `PermissionDenied` or `OverBudget`
   variant. Containment is lexical: no canonical path, no symlink
   handling (`grant.rs`, the path containment).
9. **Indentation is semantic for continuation lines**, against decision
   C2 and the cheat sheet's first paragraph: a statement continues only
   when the next line is indented deeper and starts with a continuation
   word (`crates/renyi_syntax/src/parser.rs:166-177, 303-315`); the lexer
   ends documentation clauses by indentation (`lexer.rs:307-326`). The
   sketch documents the rule (section 1); the decision record does not.
   Verified: `otherwise 0` at the statement's own column is "expected an
   expression, found `otherwise`".
10. Done in stage 1: **a method is declared in the module of its type**
    (decision K1): a `self:` function whose head type another module
    declares is `method-module` (`world.rs`, `declare_function`). Tested
    in `rules.rs`.
11. Done in stage 1: **a higher-order function declares only its own
    effects** (sketch section 3): a call through a function-typed
    parameter charges nothing to the function (`check.rs`,
    `call_function_type`), and the call site that passes a function
    covers its needs as before, whatever the parameter's type lists
    (decision B1); a function type with `needs` may be followed by
    another parameter (`parser.rs`, `capabilities`). At run time the
    passed function runs under the grant in force where it is called
    (decision Q1). Still open: a function value stored in a collection
    and called later is charged nowhere statically; the grant stack
    refuses it at run time. Tested in `rules.rs` and by the conformance
    case `higher_order.ry`.
12. Done in stage 1: **type parameters are in scope for body annotations**
    (`check.rs`, `resolve_in_body` takes the function's `for any`
    parameters). Tested in `rules.rs`.
13. Done in stage 1: **a let-bound numeric literal takes its own type**,
    Integer or Decimal, unless the binding is annotated (`check.rs`,
    `StmtKind::Let`), so `let whole be 3` then a Decimal comparison is
    `type-mismatch` (sketch section 7). Tested in `rules.rs`.
14. **Replay does not compare the grant header.** Done in stage 1: a
    replay counts every call against the budgets and refuses the
    recording that exceeds one (`vm.rs:646`; tested in
    `crates/renyi_vm/tests/recording.rs`, `a_replay_checks_the_budgets_too`).
    Still open: the recording's `grant` header is never compared with
    the test's `needs` (coverage is checked per recorded call).
15. **Two spellings of the range loop** are accepted (`for each x in from
    1 to 9` and `for each x from 1 to 9`) and the formatter rewrites one
    into the other (`parser.rs:1625-1637`, `format.rs:862-872`), against
    the one-spelling rule and the formatter's "never changes tokens".
16. **The module header need not match the path** (decisions G3, J17; only
    imports look a module up by its header), and `.renyi` is not accepted
    by imports, the index or the git-base maps (`crates/renyi_check/src/lib.rs:195`,
    `crates/renyi_index/src/lib.rs:217`, `crates/renyi/src/maps.rs:224, 245`)
    although `README.md` calls the two extensions equivalent.
17. **A user `Hash` implementation is never called**
    (`crates/renyi_vm/src/value.rs`; known), and collections keep the
    derived equality (`04-stdlib-sketch.md`, ability table). Done in
    stage 1: a declared `equals` decides `is` and `is not` (`vm.rs:1767`,
    `equal`); a `Float` result past its range or not a number is a crash
    (`vm.rs:2046`, `finite_float`, used by the arithmetic, `to_float` and
    JSON decoding); an `Integer` beyond i64 is answered by `List.at`
    (nothing), `take` and `drop` and their `Text` forms
    (`natives/prelude.rs:358`, `count`); `Text.split("")` gives the
    characters and `replace(old: "")` returns the text; `parse_instant`
    requires two-digit clock fields; `Ordering` stays `Less, Same,
    Greater` by decision V3. Still through `small()`: `repeat`, `pad` and
    the rounding places. Tested in `crates/renyi_vm/tests/semantics.rs`.
18. **Known and still open from `HANDOFF.md`:** `break` or `continue` as
    the outcome of an `if` or `match` expression nested in another
    expression leaves operands on the stack (probes gave correct totals;
    the residue is by reading); a refinement condition may read another
    field; `--redact` works by name only.

## 2. Promised compile-time checks that nobody wrote

Each is a sentence in the design that the checker does not act on.

1. Nesting deeper than 4 and bodies over about 60 lines as compile errors
   (decisions D4, O3; `05-agent-tooling.md:75, 178-180` say the language
   rejects them). Verified: six nested `if` blocks and an 85-line body
   check clean. The corpus's deepest definition is 3 and its longest body
   30 lines, so enforcing costs nothing today.
2. Regex literal patterns (decisions K3, N2): the error is computed and
   discarded, `let _ = error;` (`check.rs:1726-1728`), for `raw` literals
   only; a bad pattern crashes at run time (`natives/regex.rs:19-21`).
3. `Url` and `Path` literals (N2): plain `type X is Text`, nothing
   validates them; `Date(year: 2024, month: 2, day: 30)` checks clean
   while `library/std/time.ry:31` says `InvalidDate` names a day that does
   not exist.
4. A variant field named `kind` under `ToJson` or `FromJson` (decision
   K10): no rule in `resolve_derives` (`world.rs:681-725`); the encoder
   writes two `kind` keys.
5. `see also:` existence (decision C8b): never read by the checker.
6. Done in stage 1 (decision V6): the first two tiers of `deprecated:`
   (decision C8c). Every call of a deprecated definition gets the
   `deprecated` warning with the replacement as its fix, `renyi check
   --strict` makes it an error, and the index drops deprecated
   definitions (tier 1). `renyi migrate` (tier 3) waits for the package
   manager.
7. Done in stage 1: the `guard-no-sink` warning when no sink after `only
   to` is in the grant.
8. Ability requirements `ability X where self can Y`: parsed
   (`parser.rs:1054-1064`), never checked; an implementation whose type
   lacks `ToText` passes and runs.
9. The `Iterable` core ability of sketch section 5 is declared nowhere.
10. `expose as tool` parameter and return types are not checked as
    JSON-representable (decision D6).
11. Diagnostics without a `fix`: the parser has about 31 diagnostic sites
    and 3 fixes, `world.rs` 23 and 14; the generic "expected" parse errors
    carry none (decision D3). One-spelling fixes exist for statement-start
    foreign words and lexer symbols only: top-level `def` or `fn`, `else`,
    `null`, `is larger than`, `userName` and `user_record` get a bare
    "expected" (decisions C4, C5).
12. The fix text for an unread `count` loop variable differs from decision
    M2 (`check.rs:704-707`).

## 3. Surface syntax that parses but does nothing

These are in the cheat sheet or the sketch, so a program may use them
today and get no behaviour. Each needs a decision: implement before the
freeze, or take it out of the frozen surface until it exists.

1. **`only to` provenance guards** (decision P3; `06-runtime-guarantees.md`
   section 3). Done in stage 1 (decision V6): the VM tracks origins as a
   bit set on values (`Value::Guarded`), tags what enters through a
   guarded capability and what is computed from it, and refuses every
   outgoing call past the sinks with the prelude's `Guarded(origin, sink)`
   or a crash; a replay and a test's own grant enforce the same guards, a
   server does not send a guarded response (`crates/renyi_vm/tests/
   guards.rs`). The recording format is unchanged. What remains is in
   section 3.4 of the design document (`match` without an arm for
   `Guarded`, R6-8) and its open items.
2. **`expose as tool`** (decision D6; sketch section 15). Done in stage 1:
   `renyi tools [path]` prints the manifest with a JSON Schema per
   parameter and result, the purpose and the permissions
   (`crates/renyi_index/src/tools.rs`); `tool-type` rejects a parameter or
   result that JSON cannot carry. `serve --mcp` stays with M5.
3. **`deprecated:`** (item 2.6): the cheat sheet (line 253) says it warns
   existing callers.
4. **`process` and `foreign` capabilities**: in the tree
   (`crates/renyi_check/src/effects.rs:29-30`), scoped, accepted in
   grants (`needs process("git") at most 3 per run` checks clean), no
   library function needs them, no `std.process`, no FFI (decisions F0,
   F1; `std.process` is "Not in v1" in the library sketch).
5. **`lazy`** is reserved with no grammar rule (decision B4, as intended).
6. The memory budget of a grant (`at most 256 megabytes memory`, open item
   R7-1) has no syntax and no enforcement; the VM has no step or
   recursion limit either.

## 4. Milestones not started, or half done

- **M4** (library and package manager): the library half exists (12
  modules, 189 functions, natives for each). Absent: the registry, the
  lockfile, the project manifest, `renyi add`, `update --accept-effects`,
  `audit`, package effect manifests with package-named coverage errors,
  dependency hashes in the run manifest, the FFI generators, the `only to`
  runtime, budget thresholds read from the manifest (decision R7),
  `std.process`.
- **M5** (LSP, index, compiler API): the index half exists (`index`,
  `--budgets`, `--diff`, `mcp` with ten tools). Absent: the LSP, the
  embedding API with per-module grants and memory budgets (decision Q3:
  the nearest things are `Vm::begin_run`, `Vm::call_function` and
  `Options`, one grant per run), `renyi run --sandbox grant.json`, `renyi
  serve --watch` with checked swaps (decision Q4), the per-definition map
  refresh and a resident `World` (open item R5-5: every MCP tool call
  re-reads and recompiles the served directory).
- **M6** (AOT and WASM): not started; no `renyi build` (decision A1;
  `crates/renyi/Cargo.toml:3` says "building"); no WASI, browser or C API
  target (decision A5); only Windows has run the toolchain.
- **Concurrency**: tasks run one after the other (decision S2); decision
  E1's green threads and the performance side of the trade-off are not
  delivered; `within` is checked between statements and items and does
  not interrupt a blocking primitive (`vm.rs:1217-1240`).
- **Testing and CI** (decisions H6, D1). Done in stage 1 (decision V8):
  `.github/workflows/ci.yml` runs the format, clippy, test, corpus, lint
  and token gates and the conformance suite on every push;
  `tests/conformance/` holds the suite (the ten Predict references, one
  program per diagnostic of stage 1, one guarded run) with a runner that
  needs no Rust crate (`tools/conformance.py`). Still missing: golden
  files for parse trees and formatter output; property tests
  ("type-checked programs never crash the VM" is untested); the
  readability regression compares nothing to the thresholds and is not
  wired to anything; `renyi index --budgets` always exits 0.
- **Specification**: no formal grammar and no language reference; the
  hand-written parser is the only grammar (`CLAUDE.md` still says "until
  the formal grammar exists").
- **Recorded and narrated runs**: a server run cannot be recorded (`serve`
  loops; inbound requests are not effect calls; `http_service` has no
  fixture); narration prints every declared function, by bare name when
  there is no `purpose:`, in a format unlike the document's, without the
  `otherwise` that caught a failure, and without durations on replay.
- **Reproducibility blind spots**: `main`'s hash misses ability
  implementations reached by generic dispatch or interpolation
  (`crates/renyi_index/src/edges.rs:58-77`, `check.rs:1705`) and constants
  used only in refinement conditions (`lib.rs:275-293`), so `renyi
  reproduce` can accept such an edit; only standard output is compared;
  the toolchain is named by its version string.
- **Performance** (decision A4 names an anchor; no benchmark exists).
  Measured in session 6 on the release binary (medians of a Python-timed
  subprocess loop): `renyi version` 132 ms, `renyi run examples/hello.ry`
  100 ms; a trial-division prime count to 200,000 takes 2.50 s where
  CPython 3.13 takes 0.78 s, 3.2 times slower on integer loops. `Text.matches` compiles its pattern at every
  call; last-use moves exist only for the receiver of `set x to
  x.method(...)` (decision O1 promises them for arguments).
- **Index and MCP details**: no per-module metric maxima; `branches`
  counts every `and`/`or` (open item R5-3); the `paths` metric strips
  scopes while the module budget counts spellings; library references are
  appended with the toolchain string rather than replaced; `canonical` is
  true for a file that cannot be formatted; the `effects` tool leaves
  ability calls as leaves; `library_lookup` misses methods inside ability
  blocks; `project_map` has a boolean instead of a detail level; MCP path
  arguments are not confined to the served directory (verified: `check`
  and `diff` read files outside it); the MCP `diff` test never asserts its
  answer.
- **Library details**: `ToJson.to_json`, `FromJson.from_json` and
  `FromRow.from_row` are not declared as methods (the abilities are
  derive-only; `FromRow` lives in `std.sqlite`, not the prelude); `Bytes`
  is minimal with no `from_base64` (decision K4); `std.regex` has no
  example or test beyond `Text.matches`; JSON `Pair` encodes but does not
  decode; `std.random` is a clock-seeded xorshift with `% span` and a
  stale comment; the library test compares function names only, so a
  changed signature would pass.

## 5. Documents that claim what does not exist, or contradict each other

- `docs/cheatsheet.md`: `only to` enforced (221-222); `expose as tool`
  publishes to agents (251-252); `deprecated:` warns callers (253);
  "Indentation is never meaning" (4); the `public ability Describable`
  block (101-103) has no `purpose:` and the checker rejects it (verified:
  `purpose-missing`).
- `docs/design/05-agent-tooling.md:75, 178-180`: the limits are enforced;
  253-254: a resident `World`; the section 4 example's `text_hash` prefix
  is stale.
- `docs/design/04-stdlib-sketch.md:226`: `is`, `contains` and `index_of`
  use `equals`; 229: `console.print` of non-text values (decision K11 says
  `Text` only).
- `docs/design/02-syntax-sketch.md:65, 325`: R2-1 and R2-5 called open
  (settled as J2 and J10); 237: `json.parse(text: text, naming: CamelCase)`
  (decision K5 replaced it with `parse_with`).
- `docs/design/01-decisions.md`: C2 against the parser (item 1.9); C3b
  against the library (`Same`); C4a says the phrase table is printed in
  full in the cheat sheet (dropped by U8 and U9 to pay for other
  sentences; it survives in the sketch, section 17); C8c tier 1 against
  the index; H2, R1, R8 and U9 still freeze the grammar by measurement
  with two gating models, against the owner's decision of 2026-10-06 to
  freeze by decision (not yet recorded as an entry).
- `docs/design/07-system-design.md:26`: `example:` mandatory at the
  boundary (decisions C8a and C8b make it optional).
- `README.md:27`: `.renyi` equals `.ry`; `crates/renyi/Cargo.toml:3`:
  "building".

## 6. Open items in the design documents

R5-2 (local names in the content hash), R5-3 (what `branches` counts),
R5-4 (atomic reference counts under real concurrency), R5-5 (the
per-definition map refresh); R6-5 (more than 64 guarded origins), R6-6
(`environment` guarded by default), R6-7 (`guard.release`); R7-1 (memory
budget syntax), R7-2 (who runs the registry), R7-3 (hot swap on a running
task's stack), R7-4 (the grant stack's cost). The sketch's section 18
holds none.

## 7. Proposed order

Three stages, so that the grammar freeze, the formal grammar and the
self-hosted front end rest on an implementation that does what the
documents say.

1. **Before the freeze entry is written: make what exists true.** Fix the
   defects of section 1 (the deadline crash, exhaustiveness, `with`,
   privacy, constants, ability conformance, task independence, the
   boundary crashes and the copy/move hole, the parameter and literal
   rules, replay's budget check, the range-loop spelling, the header and
   extension rules). Write the checks of section 2 (most are a day each
   or less; the regex one is emitting an error already computed). Decide
   each item of section 3: `only to` and `deprecated` warnings are
   implementable now; `expose as tool` needs a `renyi tools` command
   that prints the schema; `process` and `foreign` stay M4. Correct the
   documents of section 5 and record the contradictions of the decision
   record as new entries (indentation continuation, `Same`, the phrase
   table, the freeze by decision). Add CI running `cargo fmt --check`,
   `cargo clippy --all-targets`, `cargo test`, the token gate and the
   lint, and turn the corpus's golden outputs into an
   implementation-independent conformance suite (the suite is what a
   self-hosted compiler will be tested against).
2. **The freeze, the specification and the ground for self-hosting.** The
   freeze entry; the formal grammar (EBNF) and the language reference
   derived from the sketch and the cheat sheet; a bytecode file format or
   a loader so that a compiler written in Renyi has something to emit;
   the performance items that a compiler needs (in-place collections,
   string building, the pattern cache); then the self-hosted lexer and
   parser, tested against `renyi parse --json` on the corpus, then the
   checker and the bytecode emitter, with the Rust toolchain as stage 0
   and the Rust VM as the runtime.
3. **The milestones**: M4 (packages, FFI, the `only to` runtime if not
   done in stage 1), M5 (embedding API, sandbox budgets, live update, LSP,
   resident `World`), M6 (`renyi build`, AOT and WASM), real concurrency.
