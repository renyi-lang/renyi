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
2. Done in stage 1: **exhaustiveness is the usefulness check** over the
   shapes of the arms (`crates/renyi_check/src/check.rs`, `missing`): a
   literal arm other than a Boolean never covers its type, the patterns
   inside `some`, `success`, `failure` and variant fields count, and a
   `failure(error: Oops)` arm covers that member of the error union
   alone. The message spells the missing cases as arms (`some(...)`,
   `failure(error: Ouch)`, `Circle(...)`). Tested in `rules.rs` and by
   the conformance case `match_nested.ry`.
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
7. Done in stage 1 (decision V7): **task independence is a checker
   rule**: a task of `run concurrently` may not `set` a mutable binding
   declared outside the block and may not read a binding declared by
   another task of the same block (`task-independence`; conformance
   case `task_independence.ry`).
8. **The boundary crashes where the design says a failure.** Done in
   stage 1: `std.sqlite` fails with the new `DbError.PermissionDenied` and
   `DbError.OverBudget` (`library/std/sqlite.ry`; `vm.rs`, `denied` and
   `over_budget`), and `filesystem.copy` and `move` compare the target
   with the write scope too (`grant.rs:525`, `target_effect_of`; tested in
   `crates/renyi_vm/tests/semantics.rs`). Closed by decision Y1
   (session 8): `serve` fails with `StartError.PermissionDenied(port)`
   or the new `OverBudget(port)` (`library/std/server.ry`; `vm.rs`,
   `denied` and `over_budget`; tested in
   `crates/renyi_vm/tests/network.rs`); a primitive that cannot fail
   (`filesystem.exists`, `environment.get`) crashes by rule, naming the
   function whose `needs` narrowed the grant, and containment is
   lexical by rule (no canonical path, no symlink handling), both in
   the reference's section 11.
9. Done in stage 1 (decision V2): **indentation carries no meaning,
   continuation lines included**; the sketch's section 1 states the
   rule and the lexer and the parser follow it (commit `8c775cd`).
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
    Closed by decision Y2 (session 8): `replay_with` checks the
    capabilities of the `grant` header against the effective grant
    before the calls (`vm.rs`; tested in `recording.rs`,
    `a_run_is_recorded_and_replayed`); the six fixtures fit their tests.
15. Done in stage 1: **the range loop has one spelling**, `for each x
    from 1 to 9`; `in` before `from` is `range-loop` (`parser.rs`,
    `loop_source`), so the formatter never rewrites it. Tested in
    `rules.rs` and by the conformance case `range_loop.ry`.
16. Done in stage 1: **the module header must match the path** (decisions
    G3, J17): the last segment is the file's stem and the segments before
    it its parent directories, else `module-name`
    (`crates/renyi_check/src/lib.rs`, `module_name_mismatch`); `.renyi`
    is read wherever `.ry` is: imports, the index and the git-base maps.
    The MCP `check` tool names a source given without a path after its
    header; the readability harness ignores the rule for its scratch
    file, whose name is the process id. Tested in `rules.rs` and by the
    conformance case `wrong_name.ry`.
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
    Greater` by decision V3. Tested in `crates/renyi_vm/tests/semantics.rs`.
    Decision Y3 (session 8) keeps the rest as the rule: collections
    compare and hash by structure and consult no declared `equals` or
    `hash`; `repeat`, `pad` and the rounding places crash on an
    `Integer` past a machine word; both in the reference (sections 5
    and 7).
18. Done in session 8 (decision Y4): **a statement loop marks the
    operand stack at its entry and `break` and `continue` unwind to the
    mark** (`Op::MarkStack`, `Op::UnwindStack`; `compile/mod.rs`,
    `enter_loop` and `leave_regions_of_loop`; tested in
    `crates/renyi_vm/tests/semantics.rs`), so an outcome nested in an
    expression leaves nothing behind; **a field's refinement condition
    sees its own field alone** and another field of the type is
    `refinement-field` (`check.rs` and `compiler/bodies.ry`,
    `check_condition_body` and `infer_name`; conformance case
    `refinement_field.ry`). `--redact` works by name by decision S3,
    which is the design, not a gap.

## 2. Promised compile-time checks that nobody wrote

Each is a sentence in the design that the checker does not act on.

1. Done in stage 1 (decision V5): nesting deeper than 4 is
   `nesting-depth` and a body over 60 lines is `body-length` (`check.rs`;
   decisions D4, O3).
2. Done in stage 1: a literal pattern, as the argument of `matches` or a
   `Pattern(...)` construction, is checked by the engine at compile time
   (`regex-invalid`; `check.rs`, `check_regex_literal`). A pattern built
   at run time still crashes when it is bad (`natives/regex.rs`).
3. Done in stage 1: a `Url` literal needs a scheme and a host, a `Path`
   literal is non-empty without control characters (`invalid-literal`;
   `check.rs`, `check_text_literal`); a `Date` built from literals needs
   a day its month has (`constraint-violation`; `check_date_literal`),
   and one built from run-time values fails with `ConstraintViolation`
   for such a day (`vm.rs`, `refined_record`).
4. Done in stage 1: a variant field whose JSON key is `kind` under
   `ToJson` or `FromJson` is `kind-field` (`world.rs`, `resolve_derives`).
5. Done in stage 1: every `see also:` name must be a definition of the
   module, `module.name` of an import or `Type.method`
   (`unknown-reference`; `world.rs`, `check_see_also`).
6. Done in stage 1 (decision V6): the first two tiers of `deprecated:`
   (decision C8c). Every call of a deprecated definition gets the
   `deprecated` warning with the replacement as its fix, `renyi check
   --strict` makes it an error, and the index drops deprecated
   definitions (tier 1). `renyi migrate` (tier 3) waits for the package
   manager.
7. Done in stage 1: the `guard-no-sink` warning when no sink after `only
   to` is in the grant.
8. Done in stage 1: `ability X where self can Y` is checked once every
   implementation is known: an implementation for a type without `Y` is
   `missing-ability` (`world.rs`, `check_requirements`), and a type
   parameter constrained to `X` has `Y` (`check.rs`, `has_ability`).
9. Done in stage 1 (decision V10): `Iterable of Item` is declared in the
   prelude with `to_list(self) returns List of Item`; `for each` and the
   queries walk any type that implements it (`check.rs`, `loop_items`;
   `vm.rs`, `IterInit`), and an implementation's method may not declare
   `needs` (`method-signature`).
10. Done in stage 1 (decision V6): a tool's parameters must have
    `FromJson` and its result `ToJson` (`tool-type`; `check.rs`,
    `check_tool_signature`).
11. Done in stage 1: every diagnostic carries a fix (decision D3). The
    fix-less helpers are gone from the lexer, the parser and the checker,
    so a new site cannot omit one, and both conformance runners require a
    `fix:` line after every diagnostic. The parser's `expected` fix is the
    Renyi spelling when the token is another language's (`def`, `null`,
    `than`, braces; decision C4); the lexer keeps `userName` as one name
    with `write `user_name`` as its fix, and `user_record` where a type is
    expected gets `write `UserRecord`` (decision C5); an unknown name,
    type, field, variant, module or ability suggests the closest one in
    scope, or the Renyi name of a foreign one (`null`, `True`, `print`,
    `Int`; `suggest.rs`).
12. Done in stage 1: the unread loop variable of a bare `count` query
    over one source gets the fix decision M2 names, `source.length()`
    (`check.rs`, `BindingKind::Count`).

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
  call; last-use moves exist only for the receiver of `change x to
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

Corrected at the end of stage 1 (session 7); kept as the record of what
was wrong.

- `docs/cheatsheet.md`: `only to` enforced (221-222), `expose as tool`
  publishes to agents (251-252), `deprecated:` warns callers (253) and
  "Indentation is never meaning" (4) are true since decisions V2 and V6;
  the `ability Describable` block (101-103) is no longer `public`, so it
  needs no `purpose:` and the checker accepts it.
- `docs/design/05-agent-tooling.md:75, 178-180`: the limits are enforced
  since V5; 253-254 now say that `renyi mcp` re-reads the served
  directory on every call and that a resident `World` is open item R5-5;
  the section 4 example's `text_hash` carries the `sha256:` prefix
  `own_text_hash` writes.
- `docs/design/04-stdlib-sketch.md:226`: `is` and `is not` use a declared
  `equals` since session 7 and the table says which operations keep the
  derived form; 229 no longer names `console.print` of non-text values
  (decision K11).
- `docs/design/02-syntax-sketch.md:65, 333`: R2-1 and R2-5 now cite J2
  and J10; 245 names `json.parse_with` (decision K5).
- `docs/design/01-decisions.md`: C2 is restored by V2, C3b is read with
  `Same` by V3, C4a's phrase table is placed by V4, C8c tier 1 is
  delivered by V6, and the freeze by decision is recorded as V1; H2, R1,
  R8 and U9 stand as the record of the measurement that preceded it.
- `docs/design/07-system-design.md:26`: `example:` is no longer called
  mandatory at the boundary (C8a makes `purpose:` mandatory, C8b makes
  `example:` a tested clause).
- `README.md:27`: `.renyi` equals `.ry` since `b997b93`;
  `crates/renyi/Cargo.toml:3` no longer says "building".

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

### Status at the end of session 7 (2026-10-06)

Stage 1 is done. Sections 1, 2, 3 and 5 are resolved except the residue
their entries name: 1.8 (the server and the primitives that cannot fail
crash on a scope denial; containment is lexical), 1.11 (a function value
stored in a collection is charged nowhere statically), 1.14 (the
recording's grant header is not compared with a test's `needs`), 1.17
(`Hash` implementations are never called; collections keep the derived
equality; `repeat`, `pad` and the rounding places go through `small()`),
1.18, 3.4 (`process` and `foreign` wait for M4) and 3.6 (the memory
budget has no syntax). Section 4 is stages 2 and 3; section 6 is
unchanged, plus R3-2 (constraints with type arguments) in the sketch.
The freeze entry is V11; the formal grammar is `docs/grammar.ebnf`
(decision V12, session 8) and the language reference `docs/reference.md`
(session 8); the sketch is the design record.

### Status at the end of session 8 (2026-10-06)

Stage 2 is under way in the order the owner set (decision W4: the lexer
and the parser first). The front end written in Renyi lives in
`compiler/` (W2): `ast.ry`, `lexer.ry`, `parser.ry` and the drivers
`parse.ry` and `tokens.ry`; the tree's JSON is the derived JSON of the
Renyi types (W1); `crates/renyi/tests/selfhost.rs` holds the Renyi
parser equal to the Rust one, byte for byte, over the corpus, the
conformance programs, the library declarations and the compiler itself
(W3). The first measurement of the VM on compiler-sized work: the lexer
and the parser take about 0.3 s on a 13-line program, 0.6 s on the
prelude's declarations and 3.9 s on the 2700-line parser (after the
lexer stopped slicing the source text with `drop` and `take`, which
copy the whole text per token: 79 s before); a list grown with `append`
in a loop is linear (80 000 appends in 0.26 s, the in-place update of
decision O1), and where the rest of the time goes has not been
profiled.

The checker written in Renyi followed in the same session (decisions W5
to W8): `lists.ry`, `report.ry`, `effects.ry`, `suggest.ry`, `types.ry`,
`refine.ry`, `declare.ry` (the port of `world.rs`), `bodies.ry` (the
port of `check.rs`) and the command line `checker.ry`; `renyi run
compiler/checker.ry --json <file>` prints what `renyi check --json`
prints, and the same test holds the two equal, with and without
`--strict`, over the corpus, the conformance programs and the
compiler's own sources (W7: 71 programs, every diagnostic, fix and exit
status equal). The references the VM's compiler needs are produced and
not yet judged (W8). The second measurement: the Renyi checker takes
about 1.5 s on a small program (the twelve library declaration files
lexed, parsed and declared each run), 32 s on the 7500-line `bodies.ry`
and 39 s on `checker.ry` with its imports; the judge takes about three
minutes on eight threads.

The profile (W6, the same session; decisions X1 to X4). Every number
above came from the development build: the release build runs the
parser on `parser.ry` in 1.0 s and the checker on `bodies.ry` in 5.5 s,
four to five times faster, so the development profile now optimizes
`renyi_vm` and the dependencies (X1). A sampling profiler built into the
VM (X4) showed, on the checker's run over `bodies.ry` (148 million
operations, 5.5 million calls, 3.9 million primitive calls): a tenth of
the time in `line_of` of `bodies.ry`, a linear scan of the line starts
per call (now a binary search); a tenth in the lexer's `slice`, which
cut a token one character at a time (now one `List.slice`, X2); the
rest spread over the VM's plain operations at about 35 ns each (`Load`
14 percent, `Field` 10, `Binary` 10, `Call` and `Return` 16, the
primitives `at`, `contains`, `length` and `append` 15 together), the
record field found by a linear search of its names on every access,
every `step` returning a 48-byte result and every `Value` 40 bytes
wide. After the source fixes and `slice`: 93 million operations, 3.3 s
in the release build (from 5.5 s); the parser 0.76 s (from 1.0 s); the
checker's judge 64 s in `cargo test` (from 170 s). The
owner chose to rewrite the interpreter loop and slim `Value` next (X3,
target 1.5 to 2 times), then the bytecode emitter with its file format
or loader.

The loop (X3) and `--profile` (X4), the same session. `vm.rs` runs the
loop of decision X3: the running frame's code, program counter and
base live in locals and nothing is returned per operation; a call to a
declared function leaves the arguments on the stack as the callee's
first locals; one handler stack serves every frame; `Op::Field`
remembers per site the index it found last; a primitive takes its
arguments as a slice of a buffer the VM reuses, so a call allocates
nothing; `Value` is 24 bytes (`Decimal` boxed); the declared `equals`,
`compare`, `to_text` and `to_list` of every type are found once at
compile time; two small Integers, two texts or two Booleans under an
operator take a fast path; a loop over a list walks the list itself.
Measured in the release build, best of five, against `f770f79` run
back to back: the parser on `parser.ry` 963 to 619 ms, the checker on
`lexer.ry` 686 to 433 ms and on `bodies.ry` 3650 to 2339 ms (1.56
times each, the operation counts unchanged at 17 and 93 million); a
counting loop 1475 to 794 ms (8.8 ns per operation), a calling loop
629 to 282 ms, a loop of primitive calls 868 to 422 ms; `cargo test`
1 m 45 s, the two judges 51 s of it (from 64 s). The profile now shows
the time spread over the plain operations at about 20 ns each (`Call`
13 percent, `Load` 11, `Field` 10, `Return` 10, `Binary` 6, the
primitive `at` 6, `Construct` 5 on `bodies.ry`), every allocation on
the system allocator, so the binary now allocates through `mimalloc`
(X6: the parser 811 to 496 ms, the checker on `lexer.ry` 510 to 389 ms,
on `bodies.ry` 2712 to 2138 ms, measured back to back against the same
commit without it; about 1.9, 1.8 and 1.7 times `f770f79` in all). The
owner's order for what follows: the residue of stage 1 (the status of
session 7 above: 1.8, 1.14, 1.17, 1.18, R3-2), then the bytecode
emitter, which writes a file the VM loads (X5).

The residue (decisions Y1 to Y4, the same session): 1.8, 1.14, 1.17 and
1.18 are closed as their entries now say (the server's `StartError`,
the crash rule for primitives that cannot fail, lexical containment;
the recording's grant header; the derived `Equal` and `Hash` of the
collections; the loop's stack mark and `refinement-field`); R3-2 waits
until after the emitter (Y4). Still open from the status of session 7:
1.11 (a function value stored in a collection is charged nowhere
statically), 3.4 and 3.6. Next: the bytecode emitter.

### The bytecode file (session 8, decisions Z1 to Z4)

The emitter's target exists before the emitter: a bytecode file is the
derived JSON of the types of `compiler/bytecode.ry` (Z1), `renyi
compile [--to <file.ryc>] <file.ry>` writes it and `run`, `record`,
`test` and `reproduce` load a `.ryc` file in place of a source (Z4),
through a loader hand-written over the VM's own JSON reader that refuses
what does not fit (Z2). Every `run` case of the conformance suite runs a
second time from its file (Z3). The spans a program keeps count
characters since this entry, as the syntax tree's JSON does.

### The emitter written in Renyi (session 8, decision Z3)

The emitter exists: `compiler/emit.ry` is the transcription of
`crates/renyi_vm/src/compile/` and `types.rs` over the world of
`declare.ry`, the tree and the references `bodies.ry` records;
`compiler/project.ry` holds what the command lines share (the files,
the imports, the library, the checked project); `renyi run
compiler/compile.ry [--to <file.ryc>] <file>` writes what `renyi
compile` writes. The judge in `crates/renyi/tests/selfhost.rs` holds
the two files equal, byte for byte, over the corpus, the conformance
programs and the compiler's own sources, and requires the Renyi
compiler to refuse what `renyi compile` refuses (Z3). With it, the
references of W8 have their judge, and the Rust VM runs what the Renyi
compiler emits: the toolchain written in Renyi is the lexer, the
parser, the checker and the emitter, with the Rust toolchain as stage
0. R3-2 (constraints with type arguments) is decision AB1, the
subsection below; next: the remaining performance items as a profile
calls for them.

### Constraints with type arguments (session 8, decision AB1)

Open item R3-2 is closed. A constraint names an ability with the type
arguments it declares (`for any Bag, Item where Bag can Iterable of
Item`), any types in scope, `type-arity` otherwise; a requirement
follows the same rule. The body walks the parameter and calls the
ability's methods with the arguments substituted; a call site matches
the arguments of the argument's implementation against the
constraint's and infers what they leave open, and a mismatch is
`missing-ability` with the arguments named. One implementation per
ability and type (`duplicate-implementation`). The prelude implements
`Iterable` for the collections, a range and a text, so that a generic
walker takes a list. Both checkers, the VM's dispatch on base values,
the grammar, the reference, the cheat sheet and four conformance cases
changed in one commit. Still open from section 7: 1.11, 3.4 and 3.6.
The judges of `selfhost.rs` run the front end from the bytecode `renyi
compile` writes of each driver when the test starts, and the Renyi
compiler writes the file it ran from (its fixed point). Next: the
remaining performance items as a profile calls for them, or stage 3.
