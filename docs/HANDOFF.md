# Handoff

Last updated: 2026-10-06, session 8 (stage 2 of the gap audit of
`docs/GAPS.md`: the formal grammar `docs/grammar.ebnf` with decision V12;
the language reference `docs/reference.md`, normative, held to the
grammar file and the crates by a test; the sketch retired to the design
record; the front end written in Renyi under `compiler/`, decisions W1
to W4, held equal to the Rust parser by `crates/renyi/tests/selfhost.rs`;
then the checker written in Renyi, decisions W5 to W8, held equal to
`renyi check --json` by the same test; then the profile of the VM, W6,
with decisions X1 to X4: the development profile optimizes the VM,
`List.slice`, the interpreter loop rewritten, `renyi run --profile`;
then X5, the emitter writes a bytecode file, and X6, mimalloc; then the
residue of stage 1, decisions Y1 to Y4; then the bytecode file itself,
decisions Z1 to Z4, with `renyi compile` and the loader).
Branch: `main` is the only branch (owner's decision, 2026-10-05); commit
and push there directly.

## Where the project stands

Milestones M0 (design), M1 (front end), M2 (type and effect checker) and
M3 (the VM: `renyi run`, `record`, `run --replay`, `reproduce`,
`--explain`, `test` with `replays`, budgets, the grant stack of Q1, the
run manifest of Q2, every library module, the `only to` guards of P3,
tasks one after the other by S2) are done, with the project map (`renyi
index`, `--budgets`, `--diff`), `renyi tools` and `renyi mcp` on top. M4
to M6 are not started (`docs/GAPS.md`, section 4). Design decisions are
in sections 0 to Z of `01-decisions.md`; the agent tooling in
`05-agent-tooling.md`, the signature capabilities in
`06-runtime-guarantees.md`, the system-level commitments in
`07-system-design.md`; the open items in section 18 of the sketch (R3-1,
`Iterable`) and section 6 of `docs/GAPS.md`.

The owner's direction since 2026-10-06 (decision V1): the readability
scores are no longer the gate; the aim is a self-hosted, independent
language that people use; the grammar freezes by decision, not by
measurement; the front end is written in Renyi first with the Rust VM as
the runtime; the repository stays private until the owner says
otherwise; the first users are people building agent workflows and
learners. Before any of that the design was audited against the
implementation (`docs/GAPS.md`, session 6) and the gaps filled (stage 1
of its section 7, session 7). The grammar is frozen by decision V11 at
commit `dc58bb3`: a change to the surface is a new decision entry first,
then the reference, the grammar, the cheat sheet, the formatter and the
conformance suite in one commit. Since session 8 the normative text is
`docs/reference.md` with `docs/grammar.ebnf` (`crates/renyi_syntax/tests/grammar.rs`
and `tests/reference.rs` hold them to the parser and to each other); the
sketch is the design record.

Stage 2 of `docs/GAPS.md` section 7 is under way in the order the owner
set (W4, W5): the lexer, the parser and the checker written in Renyi
exist under `compiler/` (the next two sections) and the VM is profiled
and its loop rewritten (the section after them, decisions X1 to X6);
the residue of stage 1 is done (decisions Y1 to Y4, the section after
the profile's); the bytecode file exists with `renyi compile` and the
loader (decisions Z1 to Z4, the section after the residue's); the
emitter written in Renyi, which writes that file, is next.

The corpus has 30 programs, passes the lint, is in canonical layout,
checks cleanly, has nothing over budget, and its `example:` lines and
`test` blocks pass on the VM (six are `replays` tests answered from
recordings under `examples/fixtures/`). The cheat sheet measures
2999 of 3000 tokens. The Rust workspace has five crates:
`renyi_syntax`, `renyi_check`, `renyi_index`, `renyi_vm` and the `renyi`
binary with `check`, `format`, `tokens`, `parse [--json]
[--declarations]`, `index [--json | --budgets | --diff <map or
revision>]`, `run [--manifest] [options] <file> [arguments]`, `record
[--to file] [options] <file> [arguments]`, `reproduce <recording>
[<file>]`, `test [--strict] [--refresh name [--redact name]] [--explain]
<file>...`, `compile [--to file] <file.ry>`, `tools [path]`, `mcp
[path]` and `version`; 226 tests, clippy and fmt clean on Windows
with rustc 1.94.1. CI (`.github/workflows/ci.yml`) runs the same gates,
`renyi check compiler/*.ry` and the conformance suite
(`tests/conformance/`, 38 cases, every `run` case a second time from
its bytecode file;
runners `tools/conformance.py` and `crates/renyi/tests/conformance.rs`)
on a toolchain pinned to the owner's machine (rustc 1.94.1), so that CI
and the local gates agree on clippy's lints; `gh run list --limit 3`
shows the runs and `gh run view <id> --log-failed` a failure's log. The
VM depends on `ureq` (HTTP, with rustls), `rusqlite` (SQLite compiled
in), decision S1, and on `sha2`; the binary allocates through
`mimalloc` (decision X6).

Readability: four live rounds exist under `tests/readability/`; the last
(round 4, `2026-10-06-c696747/`, Sonnet 5.5 through the Claude Code CLI)
stands at Predict 90, Explain 100, Complete 79, Write 40. The rounds'
`notes.md` and "Done in session 6" below hold the detail; decisions R1 to
R8 and U1 to U9 came from them; they continue only if the owner asks
(decision V1).

## The self-hosted front end as it exists (`compiler/`)

- **The files** (decision W2). `ast.ry` declares the syntax tree, one
  type per construct, each `can ToJson` (records for nodes with one
  shape, sums for the alternatives: `Item`, `Type`, `TypeKind`,
  `Statement`, `Pattern`, `Expr`, `Outcome`, `QueryTerminal`, ...; every
  node's `span` is its last field). `lexer.ry` turns source text into
  tokens by the rules of reference section 1: `TokenKind` is `Word`
  (reserved words and phrases, one token each), `Identifier`,
  `TypeIdentifier`, `MemberName`, `IntegerToken`, `DecimalToken`,
  `TextToken(parts, is_block)` with holes lexed by a sub-lexer over the
  same character list, `RawToken`, `ClauseToken`, `CommentToken`,
  `LineBreak`, `Symbol(text)` and `EndOfFile`; it fails with `LexError`
  at the first bad character. `parser.ry` turns tokens into the tree
  rule for rule as `crates/renyi_syntax/src/parser.rs` does; the Rust
  parser's mutable cursor is a `Cursor` record (tokens, position, open
  brackets, whether functions have bodies) threaded through every
  function, each returning `Parsed of Node` (the node and the cursor
  after it); `peek` moves past the line breaks the layout rules make
  insignificant (inside brackets, after a comma, before a continuation
  word) exactly as the Rust `peek` does, including the spans that end at
  a line break after such a move, so that the two trees agree byte for
  byte; it fails with `ParseError` at the first error (a message and a
  character offset; no recovery, no fixes). `parse.ry` is the command
  line: `renyi run compiler/parse.ry [--declarations] <file>` lexes,
  parses and prints `json.render_indented` of the tree; `tokens.ry`
  prints the tokens as `renyi tokens` does (a development aid for
  comparing the lexers line by line).
- **The JSON** (decision W1) is the derived JSON of the `ast.ry` types:
  a record is an object keyed by its fields in declaration order, a
  variant an object with `kind` first, a `maybe` without a value `null`,
  a list an array, a span `{"start", "stop"}` in character offsets. The
  Rust encoder `crates/renyi_syntax/src/json.rs` prints the same
  document (its byte offsets converted through a table); the names in
  `ast.ry` keep it unambiguous (no two sums in scope share a variant
  name: `IsValue` beside the operator `Is`, the lexer's `MemberName`
  beside the expression `Member`; `exposed` for the reserved `exposing`).
- **The judge** (decision W3): `crates/renyi/tests/selfhost.rs` runs
  `renyi run compiler/parse.ry` over `examples/`,
  `tests/conformance/programs/`, `compiler/` and, with `--declarations`,
  `library/std/` on a few threads and compares with `renyi parse --json
  [--declarations]` byte for byte (83 programs: 80 equal, 3 rejected by
  both); a second test does the same for the checker (next section);
  a third checks `renyi format --check compiler/*.ry`. CI runs `renyi
  check compiler/*.ry` besides. Every compiler source is `renyi check`
  clean (no warnings) and in canonical layout.
- **Speed, the first measurement** (W4): on the VM the lexer and the
  parser take about 0.3 s on `examples/hello.ry`, 0.6 s on the prelude
  declarations, 1.0 s on the 750-line `lexer.ry` and 3.9 s on the
  2700-line `parser.ry` (`renyi check` of the compiler alone, which
  every run pays first, is 0.25 s). The lexer slices token text by
  characters in a loop, since `Text.drop` and `take` copy the whole text
  per call (79 s for `parser.ry` before that change). Growing a list
  with `set xs to xs.append(x)` or `xs.append_all(step.xs)` in a loop is
  linear (80 000 appends in 0.26 s, startup included: decision O1's
  `LoadMove` moves the receiver out of its slot and `take_list` in
  `value.rs` reuses the vector), so the list growth is not where the
  time goes; where it does go has not been profiled. That profile is
  the first performance item, now that the checker is written (W6).
- **Writing Renyi at this size, what bit**: field and binding names
  cannot be reserved words (`exposing`, `first`, `least`, `raw`,
  `module`, `within`, `end` ...); a text literal cannot appear inside a
  hole (bind it first); `"{"` is an unterminated hole (`"\{"`); `\r` is
  not an escape (the lexer tests `ch.trim() is ""` instead); a variant
  of a sum type cannot share a name with a variant of another sum in
  scope, and a field common to every variant cannot be read on the sum
  value (hence `expr_span`, `type_span`, `outcome_span` in the parser);
  a `failure(x)` pattern needs a type (`failure(error: Fault)`) before
  its fields can be read; a call that can fail must be handled on its
  own line (`otherwise fail`, or `match call()` directly); generic
  records (`Parsed of Node`) and function-typed parameters with `for
  any` work, so `bracketed` and `binary_chain` are shared across the
  grammar.

## The self-hosted checker as it exists (`compiler/`)

- **The files** (decision W5), each a transcription of one Rust file,
  every message and fix verbatim. `lists.ry` (`replace_at`).
  `report.ry` (`crates/renyi_syntax/src/diagnostics.rs`): `Diagnostic`
  (`is_error`, `code`, `message`, `span`, `fix: maybe Text`),
  `line_starts` and `position` over a character list, `json_string`,
  `render_json` and `render_text` as `renyi check` prints them,
  `sorted_by_start` (stable, as Rust's `sort_by_key`). `effects.ry`
  (`effects.rs`: `Grant`, capability parsing, scopes, `names_of`).
  `suggest.ry` (the closest name in scope, the Renyi spelling of a
  foreign name, `quoted`). `types.ry` (`types.rs`: `Ty`,
  `FunctionSignature`, substitution, `head_type`). `refine.ry`
  (refinements evaluated on literals, `Literal`, `Verdict`).
  `declare.ry` (3000 lines, `world.rs`): `World` with its modules,
  types, functions, abilities, implementations, params and `Builtins`;
  `add_module` registers names, `resolve_all` resolves imports, types,
  abilities, functions, implementations and constants in the Rust order,
  then the lookups, the suggestions, `resolve_type`, `declare_function`,
  `check_capabilities`, the method-signature check and `conforms`; the
  diagnostics are kept per module (`module_diagnostics`). `bodies.ry`
  (7550 lines, `check.rs`): the Rust `Checker` struct is a `Checker`
  record (world, module, diagnostics, references, variables, scopes,
  context, deferred checks, loop and task depth, literals and results
  noted for the VM) threaded through every function, each returning the
  record or `Done of Value` (a value and the record); `check_function`,
  `check_test`, `check_type_conditions`, `check_constant` and
  `check_module` are public, the rest mirrors the Rust methods one for
  one (inference, assignment and unification, abilities, scopes,
  statements, calls and overloads, constructions, patterns and
  exhaustiveness with witnesses, queries, the recorded references of
  W8). `checker.ry` is the command line: `renyi run compiler/checker.ry
  [--json] [--strict] [--library <dir>] <file>...` reads the twelve
  library declaration files from `library/std` under the working
  directory (`--library` names another), lexes and parses with the
  Renyi front end, reads the imports from the file's directory as
  `renyi_check::imported_files` does (a stack, the last import first;
  `.ry` then `.renyi`; `std` skipped), declares, resolves, checks the
  bodies, adds `module-name` (G3) and `import-errors`, the layout
  warnings of `layout.rs` (`line-width`, `trailing-whitespace`), sorts
  by start, turns `deprecated` into an error under `--strict`, prints
  the JSON or the text, and exits 1 on any error. A file that does not
  lex or parse gets one `syntax` diagnostic (the message and the
  position of the Renyi front end, not the Rust codes) and exit 1.
- **The judge** (decision W7): the test
  `the_renyi_checker_prints_what_the_rust_checker_prints` in
  `crates/renyi/tests/selfhost.rs` runs `renyi parse` to learn whether
  the Rust parser accepts a program, then `renyi run compiler/checker.ry
  --json [--strict]` against `renyi check --json [--strict]` on every
  program of `examples/`, `tests/conformance/programs/` and `compiler/`
  (71 programs, 142 cases: output byte for byte and exit status; a
  rejected program must make the Renyi checker fail). All equal at the
  first full comparison after the compiler's own sources were made
  clean; the run takes about 51 s on eight threads after X3 (64 s with
  X1 alone, 170 s before X1, when a small program took 1.5 s and
  `bodies.ry` 32 s). The lane
  `D:\Projects\.worktrees\Renyi\selfhost\compare.py` runs the same
  comparison outside `cargo test` and names the first differing line;
  `bench.py` there times the parser's and the checker's runs, best of
  five, for any binary (`RENYI_ROOT` and the binary path as arguments).
- **The references** (decision W8) are recorded as `check.rs` records
  them (`Reference(body, target, span)`, `Target`, `NumberKind`, the
  `literals` and `results` lists) and returned by `check_module` in
  `Checked`; nothing reads them yet, and nothing judges them until the
  emitter exists.
- **Where the two checkers could differ, by construction** (none shows
  on a program in the repository): the Rust checker iterates `HashMap`s
  in `suggest_*`, the imports and the method index, so a tie between
  two equally close names may be broken differently; `json_string`
  escapes `\n`, `\t` and `\r` and cannot write `\u00xx` for another
  control character; the `Path` literal check tests only `"\n"` (a
  NUL cannot be written in Renyi); `debug_quoted` escapes `"`, `\`,
  `\n` and `\t` where Rust's `{:?}` escapes every control character;
  the `import-errors` message joins the import's path with `/` where
  Rust uses the platform separator (no program in the repository
  imports a module with errors); an import that does not parse
  contributes none of its own imports (Rust parses with recovery and
  follows them); the Renyi front end stops at the first syntax error
  where the Rust one reports several with codes and fixes (the judge
  only requires failure there). Positions are characters on the Renyi
  side and bytes on the Rust side, printed as line and column in both,
  so they agree on any text.
- **Writing the checker in Renyi, what bit** (besides the parser's
  list above): a qualified type name (`ast.Branch`) cannot stand in a
  type position, so every type used is in an `exposing` list (a type
  brings its variants; the first name must follow `exposing` on the
  same line); a `with` must stay on one line, cannot nest, and swallows
  any `name: value` pairs that follow it in a call (compute the updated
  record into a local first); a `match` expression's arm holds one
  expression or an outcome (`return`, `crash with`), never a statement,
  and a `let x: maybe T be match ... end` needs the annotation when an
  arm is `nothing`; a one-line `if c then set a to x otherwise set a to
  y end` statement does not parse (write it on several lines); a call
  with one argument must not name it; an unused loop or pattern binding
  is an error, so `for each x in xs collect Constant` needs a helper; a
  local cannot share a function's name; `count`, `needs`, `all`, `any`,
  `first`, `from`, `to`, `by`, `within`, `at`, `test`, `example`,
  `failure`, `success`, `ability`, `only`, `run`, `import` and the other
  reserved words cannot name a binding or a parameter; a `failure(x)`
  binding's fields are unreadable (destructure: `when
  failure(UsageError(message))`); the formatter puts every argument of
  a wrapped call on its own line, so a function near the 60-line limit
  before formatting must be split; `renyi format` must run before
  anchors for a patch script are taken from the file.

## The profile and the loop of the VM (W6; decisions X1 to X4)

- **Build modes.** Every number reported before this profile came from
  the development build (`target/debug`, which `cargo test`, CI and
  `cargo run` use). The release build ran the parser on `parser.ry` in
  1.0 s instead of 5 s and the checker on `bodies.ry` in 5.5 s instead
  of 32 s. Decision X1: the root `Cargo.toml` sets `opt-level = 3` for
  `renyi_vm` and for every dependency in the development profile; a
  development build now runs the interpreter at release speed (the
  parser 1.07 s, the checker on `bodies.ry` 4.2 s with the sources of
  `f770f79`) while `renyi_syntax`, `renyi_check` and the binary keep
  their fast rebuilds. The first build after the change recompiles
  every dependency (a few minutes).
- **What the profile found** (release build, the checker on
  `bodies.ry`, 148 million ops, 5.5 million calls, 3.9 million primitive
  calls, 6.7 s before any fix): a tenth of the time in `line_of` of
  `bodies.ry` (a linear scan of the line starts, called twice per
  function body); a tenth in the lexer's `slice` (`chars.at(index)` and
  a concatenation per character); `line_starts` and `check_layout` of
  the driver 6 percent together (per-character loops, the characters
  computed three times per file); `find_function` 2 percent (a linear
  scan of the world's functions per item); the primitives `at`,
  `contains`, `length` and `append` 15 percent together (half of all
  primitive calls were `at`, from `char_at`); the rest spread over the
  VM's plain ops at about 35 ns each: `Load` 14 percent, `Field` 10 (a
  linear search of the record's field names on every access), `Binary`
  10 (`equal` looked a declared `equals` up by `(TypeId, String)` with
  an allocation per comparison of a record or variant), `Call` and
  `Return` 16 (arguments popped into a `Vec` and pushed back; a `Vec` of
  handlers per frame), `Construct` and `ConstructVariant` 8. Sizes
  then: `Value` 40 bytes, `Interrupt` 48, `Result<Option<Value>,
  Interrupt>` 48 (returned by every `step`), `Op` 16.
- **What `f770f79` fixed** (sources and the library, no VM change):
  `line_of` is a binary search; the lexer's predicates test `ch is not
  ""` instead of calling `length`; `check_module` maps each item to its
  function once (`body_index`); the driver converts a file to
  characters and line starts once (`Source.chars`, `Source.starts`;
  `set_source_lines` takes the starts); the lexer's `slice` is one
  `List.slice` and one `join` (X2). Release build on `bodies.ry`: 5.5 s
  to 3.3 s (93 million ops); the parser on `parser.ry` 1.0 s to 0.76 s.
- **The loop** (decision X3, done in the same session; `vm.rs`,
  `run_frames`). The running frame's code, pc and base live in locals of
  one loop that returns nothing per op; a call to a declared function
  writes the caller's pc back, pushes the callee's frame over the
  arguments where they lie on the stack (`push_frame_in_place`) and
  reloads the locals; a return, or a handler taking a failure, reloads
  them too (`Flow::Reload`); every interrupt writes the pc back first
  (`try_op!`, `crash!`), which is what locates a crash. One handler
  stack serves every frame (`Vm::handlers`, `Frame::handler_base`,
  truncated on return and when `execute` abandons the entry frame).
  `Op::Field { name, site }` remembers per site the type, tag and index
  it found last (`Vm::field_cache`, `Program::field_sites`). A
  primitive takes its arguments as `&mut [Value]`, a slice of a buffer
  the VM reuses (`Vm::scratch`; `natives::take` moves one out for the
  seven primitives that build on a collection in place), so a call
  allocates nothing. `Value` is 24 bytes (`Decimal` boxed, a compile-time
  assertion in `value.rs`). The declared `equals`, `compare`, `to_text`
  and `to_list` of every type are found once at compile time
  (`Program::specials`; `Program::method` is gone) and every call goes
  through `Program::function_codes`, a table. Two small Integers, two
  texts or two Booleans under an operator take a fast path
  (`small_binary`, `text_binary`, `boolean_binary`), the jumps pop their
  Boolean inline, a loop over a list walks the list itself (no copy),
  and a type without refinements constructs without the loop over its
  fields. `Interrupt` is unchanged (48 bytes; only the error path moves
  it).
- **The profiler** (decision X4, `profile.rs`): `renyi run --profile`
  (also `record`; `test` refuses it). A timer thread raises an atomic
  flag every half millisecond (the sleep is coarser on Windows: about
  one sample per 1.1 ms); the loop takes the flag down before an op and
  gives the sample to the op that ran last; a primitive takes it down
  when it returns, so its time lands on the primitive; every op is
  counted by `Op::kind` (a small number and a name, `Op::KINDS` of
  them), every call by code object, every primitive by name. The report
  goes to stderr when the run ends (`Vm::report_profile`, called by the
  runner): the totals, then samples by function and op, by function, by
  op kind with the primitives by name, then the counts (the top twenty
  rows of each). `tests/semantics.rs` has a case. The scratch worktree
  `D:\Projects\.worktrees\Renyi\profile` (detached at `246fca4`) holds
  the prototype and the micro-benchmarks `loop_count.ry`,
  `loop_calls.ry`, `loop_natives.ry` with `micro.py` (best of five, ops
  from `--profile`, ns per op); `D:\Projects\.worktrees\Renyi\base`
  is a worktree at `f770f79` with a release build, the baseline the
  numbers below were measured against.
- **Measured** (release build, best of five, the two binaries run back
  to back on the same machine; timings vary by 10 to 40 percent between
  runs, so only such pairs compare): the parser on `parser.ry` 963 to
  619 ms, the checker on `lexer.ry` 686 to 433 ms, on `bodies.ry` 3650
  to 2339 ms (1.56 times each; the op counts are unchanged, 17.3 and
  93.0 million); the counting loop 1475 to 794 ms (8.8 ns per op), the
  calling loop 629 to 282 ms, the loop of primitive calls 868 to 422 ms.
  The optimized development build (what `cargo test` runs) does the
  three runs in 718, 661 and 2717 ms; the gap to the release build is
  the unoptimized Rust front end that checks the program first, not the
  VM. `cargo test` takes 1 m 45 s, the two judges 51 s of it (64 s
  before X3). The target of X3 (1.5 to 2 times) is met at its low end
  on the real programs and exceeded on the micro-benchmarks.
- **What the profile says now** (the checker on `bodies.ry`, 2.2 s
  under the profiler): `Call` 13 percent, `Load` 11, `Field` 10,
  `Return` 10, `Binary` 6, the primitive `at` 6, `Construct` 5, `Store`
  4, `Const` 3, `ConstructVariant` 3, `IterNext` 3, `contains` 3; the
  parser on `parser.ry`: `Call` 13, `Load` 12, `Construct` 9, `Field`
  9, `ConstructVariant` 6, `Binary` 6, `Return` 6, `at` 6, `contains`
  6. The time is spread over the plain ops at about 20 ns each; a
  call and its return cost about 70 ns (the frame, the locals filled
  and dropped, the grant), a record two allocations (the `Rc` and its
  field vector), and every allocation goes to the system allocator,
  which is slow on Windows. Decision X6, asked and measured in the same
  session: `mimalloc` is the global allocator of the `renyi` binary
  (`crates/renyi/src/main.rs`, `#[global_allocator]`; `cargo add
  mimalloc -p renyi`); release build, best of five, back to back
  against the same commit without it: the parser on `parser.ry` 811 to
  496 ms, the checker on `lexer.ry` 510 to 389 ms, on `bodies.ry` 2712
  to 2138 ms; the micro-benchmarks, which allocate little, within
  noise. Against `f770f79` the three runs are now about 1.9, 1.8 and
  1.7 times faster. After this, speed comes from the emitter and what
  it can precompute, not from the loop.

## The residue of stage 1 (decisions Y1 to Y4)

The owner's order after X6: finish what stage 1 left open before the
emitter. Asked as one batch of four, answered with the recommended
options, done in one commit.

- **Y1, the boundary's denials.** `serve` outside the grant fails with
  `StartError.PermissionDenied(port)`, past a budget with the new
  `StartError.OverBudget(port)` (`library/std/server.ry`, the sketch;
  `vm.rs`, `denied` and `over_budget` take the call's arguments to name
  the port; `tests/network.rs`,
  `the_server_reports_a_denial_and_a_budget_through_start_error`). A
  primitive that cannot fail (`filesystem.exists`, `environment.get`)
  crashes by rule, naming the function whose `needs` narrowed the grant;
  containment of a path scope is lexical by rule (nothing resolved on
  disk); both in the reference, section 11, and in
  `06-runtime-guarantees.md`.
- **Y2, the recording's grant header.** `Vm::replay_with` checks every
  capability of the header against the effective grant before it checks
  the calls, refusing with "the recording's grant names X, which the
  grant G does not cover" (`recording.rs`,
  `a_run_is_recorded_and_replayed`: the header first, a trimmed header
  then the call, a widened header refused although no call uses it). The
  six fixtures fit their tests' `needs`.
- **Y3, documentation only.** Collections compare and hash by structure
  and consult no declared `equals` or `hash` (reference, section 5);
  `repeat`, `pad_left`, `pad_right` and `rounded` crash on an `Integer`
  past a machine word (section 7).
- **Y4 (i), the loop's mark.** `Op::MarkStack(slot)` at the entry of
  every statement loop stores the operand stack's height in a temp;
  `Op::UnwindStack(slot)` before the jump of every `break` and
  `continue` truncates to it (`bytecode.rs`, kinds 49 and 50, `Check`
  now 51, `KINDS` 52; `compile/mod.rs`, `LoopContext.mark`,
  `enter_loop(span)` emits the mark and is called before the position a
  `continue` returns to, `leave_regions_of_loop` emits the unwind;
  `compile/stmt.rs`, the three loops; `vm.rs`, the two arms). Tested by
  `tests/semantics.rs`,
  `break_and_continue_inside_an_expression_drop_its_operands`, which
  counts the ops of `main` and checks the output.
- **Y4 (ii), `refinement-field`.** A field's condition binds that field
  alone and the checker carries `ConditionScope { type_name, field,
  others }` while it checks it; a bare name that is another field of the
  type is `refinement-field` ("the condition of `high` reads `low`,
  another field of `Interval`"; fix "a refinement sees only its own
  field; check both where the value is built"), in `check.rs`
  (`check_type_conditions`, `check_condition_body`, `infer_name`) and
  mirrored in `compiler/bodies.ry` (`check_field_conditions`,
  `check_condition_body`, `refinement_field_error`); the reference's
  section 4 and appendix A; `rules.rs`,
  `refinement_conditions_are_checked_as_bodies`; conformance case 38,
  `refinement_field.ry`. The VM's condition code still takes every field.
- **R3-2** (constraints with type arguments) waits until after the
  emitter (Y4).

## The bytecode file (decisions Z1 to Z4)

The owner's batch after the residue, answered with the recommended
options: JSON, a hand-written loader, byte equality plus reruns as the
judge, `renyi compile` with loading by extension. The file side is done;
the emitter written in Renyi is next.

- **The format** is `compiler/bytecode.ry`: the VM's `Program` as Renyi
  types, each `can ToJson` (`Program`, `ModuleSource`, `BuiltinIds`,
  `TypeMeta`, `TypeShape`, `FieldMeta`, `VariantMeta`, `ImplEntry`,
  `MethodBinding`, `AbilityMeta`, `MethodEntry`, `FunctionMeta`,
  `ConstantMeta`, `TestMeta`, `ExampleMeta`, `Expected`, `Code`,
  `CodeKind`, `CodeConstant`, `Specials`, `GroupFold`, `Op` with its 52
  variants in the order of `bytecode.rs`, `OpConst` to `OpCheck`);
  `Ty`, `FunctionSignature` (`compiler/types.ry`) and `Grant`,
  `BudgetSpelling`, `SinkPath` (`compiler/effects.ry`) derive `ToJson`
  since this entry so that the format can hold them. A `.ryc` file is
  the derived JSON, `format` first (1). Names that are reserved words
  in Renyi are renamed in the file: `function_id`, `owner` (a module
  id) or `module_name` (its text), `result`, `failures`,
  `capabilities`, `fixture`, `summary`, `set_type`, `ability_id`,
  `is_descending`, `size`. Number constants are their digits in a
  string: `Int` and `Decimal` through `Display`, a Float through
  `render::float_text`, so that `Float.to_text` in Renyi gives the same
  digits.
- **The Rust side** is `crates/renyi_vm/src/file.rs`: `render(&Program)
  -> String` builds a `renyi_syntax::json::Json` tree (the writer of
  `renyi parse --json`, held equal to `json.render_indented` by the W3
  judge) with `impls` sorted by (ability, type) and methods by name and
  `method_index` sorted by (type, name); `load(&str) -> Result<Program,
  String>` reads with `natives::json::read_json`, names the place of a
  misfit (`the file.codes[3].ops[7].target: expected a number, found a
  string`), refuses another `format`, and ends with a consistency pass
  over every index (codes, functions, constants, jumps against the
  code's length, slots against `locals`, constants against the code's
  table, types, field sites, result types). `is_bytecode(path)` tests
  the extension `ryc`.
- **Spans count characters** everywhere now (`compile/mod.rs`:
  `Context.characters` holds a byte-to-character table per module with
  a source, built as `json.rs`'s encoder builds it; `Compiler::emit`
  converts every span it stores, `TestMeta.span` and `ExampleMeta.span`
  are converted where they are built; `text_of` and `source_text` keep
  taking the tree's byte spans). `Program.sources` is `Vec<Option<
  SourceLines { name, line_starts }>>` by module id, the line starts in
  characters, `None` for a library module; `Program::source(module)`
  and `location(module, span)` (`SourceLines::line_of`, a binary
  search) serve crash locations and test reports, and `runner.rs`'s
  `fixture_path` finds a `replays` file beside `source.name`. The
  repository's sources are ASCII, so nothing moved; a program with a
  wide character before a crash now reports the right line
  (`tests/file.rs`).
- **The commands** (`crates/renyi/src/main.rs`): `renyi compile [--to
  <file.ryc>] <file.ry>` (`compile_command`: the diagnostics of
  `check`, nothing written on an error, the default target
  `<stem>.ryc` in the working directory, "renyi: compiled X to Y" on
  stderr; a `.ryc` given to it is refused). `compile_with_sources`
  returns `(Program, Hashed)` where `Hashed::Sources(files)` hashes
  `main` through the project map as before and `Hashed::File(text)`
  hashes the file's bytes (`recording::sha256_of`); `run`, `record`,
  `test` and `reproduce` go through it, so a `.ryc` path works in each
  (`load_bytecode` prefixes the path to the loader's message).
- **The judges so far**: `crates/renyi/tests/conformance.rs` runs every
  `run` case without a `diagnostics` key a second time from
  `target/conformance/case<N>.ryc` and compares the same things (the
  thirteenth `run` case is the guarded print, whose failure comes from
  the boundary at run time and so from the file too);
  `crates/renyi/tests/compile.rs` compiles `hello.ry`, runs it, checks
  the manifest's hash is the file's, records and reproduces from the
  file, checks the default target, runs `weather_client.ry`'s tests
  from a file (the fixture found beside the source), and the refusals;
  `crates/renyi_vm/tests/file.rs` renders a program with every kind of
  code object and constant, loads it, renders again (equal), runs both
  and their tests (equal), locates a crash after wide characters, and
  tries the misfits (format, broken JSON, a wrong type, an index out
  of range).
- **Size**: `hello.ryc` is about 170 KB, because every library module's
  functions, types and signatures are in it (the VM needs them for
  calls, rendering and JSON decoding); fine for now, and the first
  thing to cut if files ever matter.

## The VM as it exists (`crates/renyi_vm`)

- **The loop** is described in the section above (decision X3): one
  loop over the ops with the running frame's state in locals, the
  arguments of a declared function left in place, one handler stack,
  the field cache per site, the reusable argument buffer of the
  primitives. A new op that can fail must leave through `try_op!` or
  `crash!`, never `?`, or a crash is located at the last synchronised
  pc of the frame.
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
  `Op::GroupFold` folds `sum`, `count`, `first`, `any` and `all` per group
  after `group by` (decision M1).
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
  `regex` crate; `std.http` (`natives/http.rs`) on `ureq` 3 with rustls:
  a 30-second limit per request, redirects followed one hop at a time with
  every host checked against the grant (`HostNotAllowed` for a hop outside
  it, at most ten hops), 303 and a redirected POST become GET, a non-2xx
  answer is `Status(url, status, body)`, duplicate headers joined with a
  comma; `std.sqlite` (`natives/sqlite.rs`) on `rusqlite` with SQLite
  compiled in: a `Connection` is a native value (`<connection PATH>` in
  text and in recordings; its path is the scope of every call on it),
  parameters bind by type (`Decimal` and an `Integer` past `i64` as text,
  `Boolean` as 0 or 1), `query` decodes each row into the record of the
  context type by column name (`as` names honoured) and fails with
  `Mismatch(column, expected, found)` when a cell does not fit the field
  or a row does not satisfy the record's refinement; `std.server`
  (`natives/server.rs`) hand-written over `std::net::TcpListener`:
  HTTP/1.1, one request at a time, `PortInUse` and `PermissionDenied`
  from `bind`, `PermissionDenied` and `OverBudget` from the boundary
  (decision Y1), the handler called through `Vm::call_function` with a
  `Request` record (lower-cased header names, percent-decoded query), the
  answer sent with `content-length` and `connection: close`.
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
  `FileError.PermissionDenied`, `HttpError.HostNotAllowed`,
  `DbError.PermissionDenied` or `StartError.PermissionDenied`, past a
  budget with `OverBudget`, and crashes when the primitive cannot fail
  (decision Y1). A replay first checks the recording's `grant` header
  against the grant, then each call (decision Y2). Live, the
  call runs and is appended to the recording (`Call { capability,
  primitive, arguments by name, outcome success/failure, duration_ms,
  at_ms }`, JSON by the `ToJson` rules); replaying, the entry with the
  same primitive and arguments answers it (identical entries in sequence
  order), its outcome decoded by the declared result and error types
  (`FunctionMeta.returns`/`fails`; a type-parameter result uses the
  context type the checker records for effectful primitives), nothing is
  written or sent, and a call the recording lacks crashes naming the call
  and the nearest recorded one. `--explain` narrates on stderr: `Purpose.
  (module.name, param: value)` on entry, `-> value` on exit, and each
  effect as `console "text"` or `capability name(args) -> result[, N
  ms]`, indented by call depth.
- **The grant stack** (decision Q1; `grant::within`, `Vm::frame_grant`).
  Every frame carries its effective grant: the enclosing frame's (the
  run's for `main` and for a test body), narrowed by the `needs` of the
  function the frame runs. On a line the needs mention, a granted
  capability takes the scope both allow (`filesystem.read("work")` inside
  a function declared `filesystem.read("work/data")` becomes the latter;
  a parent such as `filesystem` is split into its children first); a line
  the needs do not mention passes through, since the function can reach
  it only through a function value whose effects its caller covered (the
  sketch: effects of a function-typed parameter flow to the call site).
  `call_native` and the redirect check of `std.http` test the innermost
  frame's grant; a denial that crashes names the function whose `needs`
  narrowed the grant. The narrowed grant is remembered per function while
  the enclosing grant is the same `Rc`, so a function called in a loop
  intersects once (open item R7-4). Budgets stay the run's.
- **The run manifest and `renyi reproduce`** (decisions Q2 and S5;
  `recording::Manifest`, `runner::reproduce`). `renyi record` writes into
  the recording's header the toolchain (`renyi 0.0.1`), the source path,
  `code` (the content hash of `main` from the project map, which covers
  everything `main` reaches), the arguments, the environment variables
  read (`environment.get`, each with the hash of its value, `<redacted>`
  kept, `null` when unset), the outcome (`finished`, `failed with ...`,
  `exited with N`, `crashed: ...`) and `output` (the SHA-256 and length
  of the standard output, hashed by a writer wrapped around stdout while
  the run writes). `renyi run --manifest` prints the same header on
  stderr and writes no file. `renyi reproduce FILE [program.ry]` loads
  the recording (refused without a `code`), compiles the program named by
  the argument or by `source`, refuses when `main` hashes differently,
  warns when the toolchain differs, then replays under the recorded
  arguments with the console output written (`Options::replay_output`)
  and reports every difference: the outcome, the output hash and length,
  and recorded calls never reached; the replay itself names the first
  call that differs. Hand-written fixtures have no manifest and need
  none.
- **Not yet.** `run concurrently` and `concurrently` queries run their
  tasks one after the other by decision S2 (`within` sets a deadline that
  is checked between statements or items and fails with `TimedOut`);
  green threads would be an improvement within that decision, not a
  reversal.

## The MCP server as it exists (`crates/renyi/src/mcp.rs`)

- `renyi mcp [path]` enters the directory (the current one by default),
  reads one JSON-RPC message per line from standard input, writes one per
  line to standard output, logs only to standard error, and exits when the
  input closes. Messages are read and written with the VM's JSON reader
  and writer (`renyi_vm::natives::json`, decision T2).
- Both eras of the protocol (decision T1): a request whose `_meta` names
  `io.modelcontextprotocol/protocolVersion` is answered as revision
  2026-07-28 says (`server/discover`; `resultType: "complete"` and the
  server's identity in every result; `ttlMs` and `cacheScope` on the tool
  list and the discovery; `-32022` with the supported list for another
  version; `-32602` for a request without the client's capabilities), and
  `initialize` opens the handshake of 2024-11-05 to 2025-11-25 (the
  requested version is echoed when it is one of those, else 2025-11-25).
  `ping` answers in both. A legacy `tools/call` is served without an
  `initialize` first.
- Ten tools, in `tools/list` order: `cheat_sheet` (the cheat sheet is
  compiled into the binary), `library_lookup` (every top-level declaration
  of `library/std`, scanned by line into module, name, receiver type, head
  with its clauses, purpose; every word of the query must occur, entries
  named by a word rank first, 25 at most), `project_map` (`module`,
  `json`), `definition` (the record as `renyi index --json` prints it, then
  the canonical source lines), `effects` (the call tree under a
  definition: each project definition with declared and transitive effects
  and failures, library primitives with their declarations, a definition
  shown before marked and not expanded), `check` (`source`, `path`),
  `format`, `run` (arguments, `deny`, `allow_host`, `allow_read`,
  `allow_write`, `at_most`, `replay`, `explain`: the program's output, its
  standard error under a heading, then `--- finished ---` or the outcome;
  a run that did not finish or exited non-zero is a tool error),
  `run_tests` (`strict`, `explain`), `diff` (`base`: a saved map file
  under the served directory or a git revision; `json`; the same text or
  JSON as `renyi index --diff`). A missing or ill-typed argument and a
  failing tool are tool execution errors (`isError: true`); an unknown
  tool or method is a protocol error.
- The map (`Map::refresh`) is rebuilt when any file of the served
  directory differs from the one the last map was built from (decision
  T4); the compiled program of `run` and `run_tests` is built per call.
  `main.rs` holds the non-printing `compile_sources`, `diagnose`,
  `read_source` and `toolchain` that the commands and the server share;
  `maps.rs` loads the base of a diff.
- `crates/renyi/tests/mcp.rs` drives the binary over pipes: the legacy
  handshake, the tool list, the cheat sheet, a run of `hello.ry`, a
  `check` of source text, an unknown tool, `ping`; the modern discovery, a
  `definition` with its version, an unsupported version, missing
  capabilities, an unknown method, a malformed line, the tool list's cache
  hints; and the lookup, map, effects, format, tests, a denied run, the
  JSON map, an unknown definition and a `diff` against `HEAD`.

## The semantic diff as it exists (`crates/renyi_index/src/diff.rs`)

- `renyi index --diff <base> [--json] [path]` (and the `diff` tool)
  compares the project with a base: a map file `renyi index --json`
  wrote, read with the VM's JSON reader (`text_hash` optional, so maps
  from before this session load), or a git revision, whose `.ry` files
  (`git ls-tree`, `git show`; for a single file, its import closure at
  the revision) are indexed afresh with the same toolchain
  (`crates/renyi/src/maps.rs`). An argument that is neither is an error.
- Definitions are matched by qualified name; a name gone whose content
  hash (`id`, decision D5) is back under another name is one `renamed`
  entry and its callers are untouched. Per matched definition the changes
  are `signature` (the head and its clauses, so `needs` and `or fails
  with` count), `visibility`, `effects` widened or narrowed (on every
  definition whose transitive effects differ, callers included),
  `failures` added or removed, and `body` when the definition's own text
  hash or its `calls`/`uses` edges (old names translated through the
  renames) changed with the signature unchanged; `added` and `removed`
  otherwise. Each entry lists what it reaches: every transitive caller
  through `calls` and `uses` (in the old map for a removed definition).
  The bump of decision G1: a public definition removed, its signature
  changed, made private or renamed is `major`; one added or made public
  is `minor`; else `none`.
- `text_hash` (decision T5, `hash::own_text_hash`) is the SHA-256 of the
  definition's canonical text with its own name and every reference to a
  project definition blanked, and nothing appended, so it changes only
  when the definition's own text does; `id` also changes when a
  dependency does. Both are in every record (`renyi index --json`,
  `definition`, section 4 of `05-agent-tooling.md`). A map without
  `text_hash` falls back to `id` for the body test.
- Tests: `crates/renyi_index/tests/diff.rs` (a body change reported once
  and reaching its callers, a public signature change forcing a major
  bump, a rename by hash, added and removed definitions, an effect gained
  below widening the callers), `crates/renyi/tests/index.rs` (a saved
  map as the base, with `--json` and against itself; `HEAD` as the base
  for the corpus and for one file; a bad base), the `diff` call in
  `tests/mcp.rs`, and a unit test of `own_text_hash`.

## Done in session 8 (stage 2: the grammar, the reference, the front end in Renyi)

Three commits on `main`, each gated as in session 7:

1. `fa40315` the formal grammar `docs/grammar.ebnf` (W3C EBNF over the
   lexer's tokens, two start symbols, 76 rules) and
   `crates/renyi_syntax/tests/grammar.rs`, an interpreter of the file
   that runs it over the corpus, the library declarations, the
   conformance programs, every text hole and two lists of corner
   programs, and fails when it and the parser disagree. Decision V12: a
   line break after a comma carries no meaning (the formatter folds a
   wide `needs` clause), `is` and `fails` continue a line everywhere,
   one spelling per construct (a trailing comma, a variant's empty
   parentheses, a space after a dot, an `otherwise` arm before the last
   of a `match` expression, a head and `end` on one line, a hole in a
   plain text and `public` on a method are errors with fixes;
   `public-method` is the new code), a line break inside brackets
   before `.` or `(` is accepted. Conformance case 37, `one_spelling.ry`.
2. `17509df` the reference: `docs/reference.md` (sections 0 to 17 after
   the sketch's, appendix A the diagnostic codes with their severities
   and sections, appendix B the commands and exit statuses) and
   `crates/renyi_syntax/tests/reference.rs` (every quoted rule equals
   the grammar file's and every rule is quoted once; appendix A equals
   the codes the crates emit, found at every call named `error` or
   `warning`, with their severities); `CLAUDE.md`, `README.md`,
   `docs/GAPS.md`, the sketch's status line and its section 17 follow.
   Every claim of the reference was read back from the lexer, the
   parser, the checker, the VM and the library before it was written.
3. `63aa5ba` the front end in Renyi (decisions W1 to W4 from the
   owner's batch: the Renyi sources in `compiler/`, a Rust integration
   test as the judge, the lexer and the parser first, the JSON defined
   by the Renyi types with the Rust encoder changed to match).
   `compiler/ast.ry`, `lexer.ry`, `parser.ry`, `parse.ry`, `tokens.ry`
   as described above; `crates/renyi_syntax/src/json.rs` rewritten to
   the derived shape (character offsets through a byte-to-character
   table; its unit tests and `tests/corpus.rs` follow); `renyi parse
   --declarations`; `crates/renyi/tests/selfhost.rs`; the formatter's
   rule for a wide `exposing` list (`format.rs`, a unit test, reference
   section 16); CI's `renyi check compiler/*.ry` step; `CLAUDE.md`,
   `README.md`, `docs/GAPS.md` (status at the end of session 8, with
   the first measurement), appendix B of the reference. The parser was
   written in one pass as a transcription of `parser.rs` and agreed
   with it on the first comparison; the lexer needed one round of
   fixes from `renyi check` (reserved words as names, `"{"` as a hole,
   `\r` as an escape) and one performance fix (slicing by characters).
   Part of this commit (the JSON encoder, `tokens.ry`, the CI step and
   appendix B) was written by a second Claude Code session the owner
   ran in parallel; the two sessions split the files by message and
   this session ran the gates and committed.
4. The checker in Renyi (the last commit; decisions W5 to W8 from the
   owner's batch: the checker next, transcribed from
   `crates/renyi_check`; the profile after it; full equality with
   `renyi check --json` as the judge; the references produced and not
   yet judged). `compiler/lists.ry`, `report.ry`, `effects.ry`,
   `suggest.ry`, `types.ry`, `refine.ry`, `declare.ry`, `bodies.ry`
   and `checker.ry` as described above; `regex.problem(pattern)
   returns maybe Text` added to the library (`library/std/regex.ry`,
   the sketch, `natives/regex.rs`) so that `bodies.ry` can report
   `regex-invalid` with the engine's own message; the checker judge in
   `crates/renyi/tests/selfhost.rs` (`judge_all` generic over the case
   type); `CLAUDE.md`, `README.md`, `docs/GAPS.md` (status at the end
   of session 8, continued), the reference's appendix B, section W of
   the decisions. `declare.ry` and `bodies.ry` were each written in one
   pass from a reading of `world.rs` and `check.rs`, then taken through
   `renyi check` (reserved words, qualified type names, `with`, the
   match-arm rules, unused bindings, the body-length and line-width
   limits: the formatter's wrapping pushed six functions over sixty
   lines, each split into named helpers) until clean; the first full
   comparison with the Rust checker found no difference on any of the
   142 cases.
5. `f770f79` the profile of the VM (W6 and decisions X1 to X4
   from the owner's batch: the development profile optimizes the VM,
   `List.slice`, the interpreter loop rewritten next instead of the
   three cheap changes, `renyi run --profile`). `Cargo.toml` (X1),
   `slice` in the sketch, `library/std/prelude.ry`,
   `natives/prelude.rs` and the cheat sheet (X2; a case in
   `tests/semantics.rs`; two sentences of the sheet shortened to pay
   for it), the lexer's `slice`, the checker's own hot spots as
   described above, `docs/GAPS.md` (the profile's numbers), this file.
6. The interpreter loop and the profiler (the last commit; X3 and X4 as
   described in the profile section): `crates/renyi_vm/src/vm.rs`
   (`run_frames`, `settle`, `leave`, `push_frame_in_place`, `field_at`,
   `ability_target`, `derived_ability`, the fast paths), `profile.rs`
   (new), `bytecode.rs` (`Op::Field { name, site }`, `Op::kind`),
   `compile/mod.rs` (`field_sites`, `specials`, `date`,
   `function_codes`), `compile/expr.rs` and `pattern.rs` (the sites),
   `value.rs` (`Decimal` boxed, `plain_in_place`, the iterator over the
   list), every file under `natives/` (the slice signature,
   `natives::take`), `render.rs`, `runner.rs` (the report), `lib.rs`,
   `crates/renyi/src/main.rs` (`--profile`), `tests/semantics.rs` (the
   report's case), the reference's appendix B, `05-agent-tooling.md`
   (section 8), `README.md`, `CLAUDE.md`, `docs/GAPS.md`, this file.
7. The owner's batch after X3 (the last commit): the emitter writes a
   bytecode file (X5), `mimalloc` (X6, measured and kept), the
   development profile stays as X1 set it, the residue of stage 1
   before the emitter; `crates/renyi/Cargo.toml`, `main.rs`,
   `Cargo.lock`, section X of the decisions, `docs/GAPS.md`,
   `README.md`, this file.
8. The residue of stage 1 (decisions Y1 to Y4, the section "The residue
   of stage 1" above): `library/std/server.ry` and the sketch
   (`StartError.OverBudget`), `vm.rs` (`denied`, `over_budget`,
   `replay_with`, the two stack ops), `bytecode.rs`, `compile/mod.rs`,
   `compile/stmt.rs`, `check.rs` and `compiler/bodies.ry`
   (`refinement-field`), `tests/{semantics,recording,network}.rs`,
   `crates/renyi_check/tests/rules.rs`, conformance case 38, the
   reference (sections 4, 5, 7, 11, 14, appendix A), the decisions
   (section Y), `06-runtime-guarantees.md`, the sketch's R3-2,
   `docs/GAPS.md`, this file.
9. The bytecode file (decisions Z1 to Z4, the section "The bytecode
   file" above): `compiler/bytecode.ry` (new), `compiler/types.ry` and
   `effects.ry` (`can ToJson`), `crates/renyi_vm/src/file.rs` (new),
   `lib.rs`, `compile/mod.rs` (spans in characters, `SourceLines`),
   `runner.rs`, `crates/renyi/src/main.rs` (`compile`, `Hashed`, the
   `.ryc` loading), `tests/file.rs` and `tests/compile.rs` (new),
   `tests/conformance.rs` (the rerun), the reference's appendix B, the
   decisions (section Z), `README.md`, `CLAUDE.md`, `docs/GAPS.md`,
   this file.

## Done in session 7 (stage 1 of the gap audit)

Fourteen commits on `main`, each gated by fmt, clippy, the tests, the
canonical corpus, the lint, the token gate and the conformance suite:

1. `8c775cd` decisions V1 to V8 (the road to self-hosting; indentation
   carries no meaning, continuation lines included, so the parser and
   the lexer lost their indentation rules).
2. `8bc1534` the checker enforces V5 (`nesting-depth`, `body-length`),
   V6 (`deprecated:` tiers 1 and 2: the `deprecated` warning with the
   replacement as its fix, an error under `renyi check --strict`, the
   index drops the definition) and V7 (`task-independence`).
3. `7ade49a` `renyi tools [path]` prints the tool manifest of decision
   D6 (`crates/renyi_index/src/tools.rs`); `tool-type` rejects a
   parameter or result JSON cannot carry.
4. `71ad9d3` the `only to` guards of P3 run: origins as a bit set on
   values (`Value::Guarded`), refusals at the boundary with the
   prelude's `Guarded(origin, sink)`, the `guard-no-sink` warning
   (`crates/renyi_vm/tests/guards.rs`).
5. `d141e57` CI on every push and the conformance suite of V8.
6. `c952ed9` the runtime keeps its promises: an expired `within`
   deadline is the function's failure, `with` runs the refinements
   again, `filesystem.copy` and `move` check the target against the
   write scope, SQLite fails with `PermissionDenied` and `OverBudget`, a
   declared `equals` decides `is`, a `Float` past its range is a crash,
   `Text` and `List` edge cases, `module.constant`
   (`crates/renyi_vm/tests/semantics.rs`).
7. `e0023c4` the checker keeps the module rules: privacy for functions,
   methods, constants and abilities (`private-name`), constants across
   modules, an implementation carries the ability's signature
   (`method-signature`), methods at home (`method-module`, K1), a
   higher-order function declares only its own effects (B1, Q1), type
   parameters in scope for body annotations, a let-bound literal takes
   its own type.
8. `56f13da` exhaustiveness is the usefulness check over the shapes of
   the arms (`check.rs`, `missing`).
9. `b997b93` one spelling of the range loop (`range-loop`), the module
   name equals the path (`module-name`, G3), `.renyi` read everywhere.
10. `7127dd8` the literal rules: a literal pattern checked by the engine
    (`regex-invalid`, K3), `Url` and `Path` literals (`invalid-literal`,
    N2), a `Date` from literals and from run-time values, a variant
    field named `kind` under the JSON abilities (`kind-field`, K10),
    `see also:` existence (`unknown-reference`, C8b), `ability X where
    self can Y` (`missing-ability`), the `count` fix of M2; `Iterable`
    struck from the core abilities (open item R3-1).
11. `ccab15e` every diagnostic carries a fix (D3): the fix-less helpers
    are gone from the lexer, the parser and the checker; the parser's
    `expected` names the Renyi spelling of a foreign token (C4); the
    lexer keeps `userName` as one name (C5); an unknown name suggests
    the closest one in scope or the Renyi name of a foreign one
    (`crates/renyi_check/src/suggest.rs`); both conformance runners
    require a `fix:` line after every diagnostic.
12. `fb798a6` the documents of `docs/GAPS.md` section 5 corrected, the
    four clippy lints of GitHub's toolchain fixed.
13. `dc58bb3` `Iterable` is one method, `to_list` (V10: the prelude
    ability, the item type from the implementation's type argument, the
    VM's `IterInit` calling `to_list`, no `needs` on an ability's
    method); V9 records the runtime grant of a passed function; CI
    pinned to rustc 1.94.1.
14. The last commit: the freeze entry V11; the sketch's status line,
    `CLAUDE.md`, `README.md`, `docs/GAPS.md` and this handoff follow.

## Done in session 6

1. **`crates/renyi_vm`** (about 8300 lines with tests): `bytecode.rs`,
   `value.rs`, `integer.rs`, `decimal.rs`, `types.rs`, `compile/{mod, expr,
   stmt, pattern, query}.rs`, `vm.rs`, `render.rs`, `natives/{mod, prelude,
   system, time, filesystem, json, csv, regex}.rs`, `runner.rs`;
   `tests/corpus.rs` runs the ten Predict programs of the readability
   manifest against `tests/readability/reference/*.out` (all ten match: the
   hand-derived references are confirmed), every `example:` and `test` of
   the corpus, and a failing and a crashing `main`.
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
7. **The HTTP client, the HTTP server and SQLite** (third commit). The
   owner chose the dependencies (decision S1: `ureq` 3, `rusqlite` with
   SQLite compiled in, a hand-written server over `std::net`) and kept
   the tasks sequential (decision S2; sketch section 12 says so).
   `natives/{http, sqlite, server}.rs` as described above;
   `tests/network.rs` runs a server on a loopback port and a client
   against it in one process (`Options::serve_limit` stops the server
   after N requests), records and replays a SQLite program with the
   database deleted between the two runs, and checks a row outside a
   refinement. The checker records `Target::Otherwise { fallible }` at
   every `otherwise` and the compiler emits the new `Op::JumpIfFailure`
   for a fallible subject, so `otherwise` after a void fallible call
   (`server.serve(...) otherwise fail`) no longer mistakes the `Nothing`
   of success for an absent value (a bug the server test found).
   `renyi test --refresh` recorded live fixtures for `weather` (Berlin,
   14.0 C, wind 5.5 km/h), `currency_tool` (USD to EUR at 0.89254; the
   service moved from `frankfurter.app` to `api.frankfurter.dev` and the
   example follows) and `concurrent_fetch` (example.com, 577
   characters); the three `replays` tests pin those values and pass
   offline with `--strict`. `--at-most` now rejects a budget on a
   capability that takes none. `examples/README.md`, `README.md`,
   `CLAUDE.md`, the status line of `06-runtime-guarantees.md` and the
   crate doc of `renyi_vm` say so.
8. **The grant stack** (fourth commit): `grant::within` and
   `Vm::frame_grant` as described above; `tests/recording.rs` shows a
   helper declared `filesystem.read(".../data")` denied a file that
   `main` reads directly before and after the call, and the crash of a
   primitive that cannot fail naming the helper.
9. **Recordings for the rest of the corpus and `--redact`** (fifth
   commit): `pagination` and `assistant` carry hand-written fixtures (two
   pages; one chat answer with the `Authorization` header redacted),
   `inventory_db` a `replays` test that seeds its own table and was
   recorded live with `--refresh` from a scratch directory (the database
   file is not committed); every program that reaches the network or
   SQLite now runs offline. `renyi record --redact NAME` (decision S3,
   open item R6-3) replaces the argument, map entry or environment
   variable of that name with `<redacted>` in the recording, and a replay
   matches the placeholder against anything (`recording::redact`,
   `arguments_match`); open item R6-1 is settled by decision S4 (bodies
   stay inline as base64).
10. **The run manifest and `renyi reproduce`** (sixth commit; the owner
    chose the recording's header over a separate file or the source file
    itself, the standard output's hash and length as the comparison,
    hashed environment values, and a warning on a toolchain mismatch;
    decision S5). `tests/recording.rs` records a run with an argument and
    a variable, checks the manifest, reproduces it with the same output,
    and sees an edited recording reported on the outcome, the output and
    the unused call. `renyi reproduce` was also run by hand on `hello`:
    reproduced; refused after the program was edited; refused on a
    fixture without a manifest.
11. **Decision L2 carried out, and a VM bug it found** (seventh commit).
    `tests/readability/run.py` lets the VM judge Complete and Write
    (`renyi test` on the spliced or whole program after the lint and the
    checker; `score` keeps the summary and the failing items; a program
    with nothing to run still waits for `judgement.json`), and `compare`
    sets the VM's verdict against every judged sample. Live round: 179 of
    182 agree, the other 3 (gpt, `stacks`) are rejected today by
    `ignore-pure` (R6) before any judge, as the subagents had rejected
    them. Pre-test: 27 of 40 agree, the other 13 rejected today by the
    checker, the parser or the lint's foreign-keyword rule, four of them
    `while` loops judged correct under the old grammar. Before the fix
    the VM disagreed on every `sales_report` sample that wrote `for each
    sale in sales group by sale.region sum sale.amount` (ten): the
    compiler summed the whole list where decision M1 says per group.
    `Op::GroupFold` now folds `sum`, `count`, `first`, `any` and `all`
    per group (`tests/queries.rs`), and `totals_by_region` in the corpus
    is written in that form with a second example. Both rounds were
    re-scored with the cached Explain grades (no API call): the live
    round's rates are unchanged; agent-sonnet in the pre-test loses the
    four `while` samples. Each round's `notes.md` has a section on it.
12. **`renyi mcp`** (eighth commit; decisions T1 to T4, asked as a batch
    of four): `crates/renyi/src/mcp.rs` as described above, its tests,
    `renyi_index::definition_json` made public, the shared helpers in
    `main.rs`; `05-agent-tooling.md` (status, section 7 as implemented,
    section 8, open item R5-5), `README.md`, `CLAUDE.md`.
13. **The semantic diff** (ninth commit, `c1fb558`; decision T5,
    derived): `crates/renyi_index/src/diff.rs`, `hash::own_text_hash` and
    the `text_hash` field, `crates/renyi/src/maps.rs`, `renyi index
    --diff`, the `diff` tool, the tests listed above;
    `05-agent-tooling.md` (status, sections 1, 4, 6, 7, 8), `README.md`,
    `CLAUDE.md` (a `crates/renyi_index/` row).
14. **Readability round 2** (tenth commit). The owner chose all four
    models (Sonnet 5.5 and gpt-5.5 at their default temperature, the
    gating models of R1 and R8; Haiku 4.5 and gpt-5.4-mini at 0), the
    graders of round 1 and in-session adjudication; `run.py prepare`
    wrote `tests/readability/2026-10-05-ed37120/` (69 prompts; the cheat
    sheet of `ed37120`). Collection ran through the vendor APIs for
    everything but 45 Write samples of gpt-5.5, at which point the owner
    ruled that the pay-per-token keys are not spent by default
    (subscription quota instead: Claude Code subagents and the Codex
    CLI); the 45 came through `codex exec` on the ChatGPT subscription and
    `run --provider file` (provenance and session ids in the output
    records), and Explain was graded by subagents (Sonnet 5.5 first, Opus
    5.5 second, batches of fifty, reasons kept in `grades.json`), merged
    into the grade cache under `agent:` buckets so `score` ran offline.
    Eleven adjudications (`judgement.json`). A stale reference was found
    and fixed (`currency_tool` named the old host; twenty samples
    re-graded). The subagent graders are about one point more lenient
    than the API graders on the 204 samples both graded, flipping 30
    floor-model samples from fail to pass and none the other way; the
    notes say so and ask the owner which grader kind the protocol keeps.
    Results in the table above and in `notes.md` (the five-point rule:
    Sonnet's Write fell 40 to 30 on one item, two samples missing an
    import, noise at the model's default temperature; everything else
    held or rose). Status lines of `03-readability-test.md` and
    `tests/readability/README.md`.
15. **Decisions U1 to U4 applied; round 1 re-graded** (twelfth commit,
    after the handoff commit). The owner answered the four questions: M4
    stands and the cheat sheet says it in a sentence (U1);
    `rounded(places: 2)` in the cheat sheet, the library unchanged (U2);
    every library method listed with its parentheses, the `length of`
    form declined as a second spelling (U3); the subagent graders from
    here on and round 1 re-graded with them (U4). `docs/cheatsheet.md`
    at 2994 tokens after trimming elsewhere (the `deprecated` and
    `expose as tool` sentences, "No `async`/`await`", the `trimmed` call
    example, "Short signatures stay on the head line"). `run.py prepare`
    snapshots `reference/` into the run and `score` reads it from there
    (`reference_file`); both runs got their snapshot (round 1's
    `currency_tool.explain.txt` with the old host). Round 1's 450 Explain
    samples were graded by 18 subagents (nine Sonnet 5.5, nine Opus 5.5,
    batches of fifty, 2.07 million tokens of quota) and merged into its
    `grades.json` by `tests/readability/merge_agent_grades.py` (now in
    the repository); its API-graded tally moved to `scores-api.json` and
    `scores-strict-api.json`; `score` and `report` re-ran offline. The
    two subagents never disagreed by more than one point, so the hand
    verdicts of session 5 stand and nothing was adjudicated anew.
    Results and the caveats in round 1's `notes.md`, section 8; round
    2's notes, `README.md` and `03-readability-test.md` updated.
16. **Decisions U5 to U7** (thirteenth commit): the owner's answers to
    the re-grade's three questions. U5 applied to both rounds: fourteen
    verdicts in `judgement.json`, each naming the originating verdict and
    the sentence (round 1: Haiku's `inventory_db.1` to `.4`,
    `shipping_rules.0`, `invoice.2` and `.4`, the last two withdrawing
    session 5 verdicts of 4; Sonnet's `shipping_rules.1` and `retry.3`;
    round 2: Haiku's `inventory_db.0` to `.4`), both rounds re-scored
    offline (Haiku's Explain 27 of 30 in round 1, 26 in round 2; nothing
    else moved). `03-readability-test.md` amended (the Explain row, the
    sampling channel, the five-point rule), `README.md`, the notes of
    both rounds, section U of `01-decisions.md`.
17. **Round 3 through the CLIs** (fourteenth commit). `run.py` gained
    `--provider claude` (the Claude Code CLI on its subscription login:
    `claude -p` with the cheat sheet as `--system-prompt-file`, the
    dynamic system-prompt sections excluded, no tools, JSON output, no
    session persistence, `CLAUDE_CONFIG_DIR` pointed at a directory
    holding only the login, the keys removed from the environment),
    `--provider codex` (round 2's `codex exec` recipe moved into the
    harness), `--parallel`, and a `source` record per output file with
    the channel, the session ids and, for the Claude CLI, the thinking
    and output tokens of every session (thinking cannot be switched off;
    every switch was tried). `tests/readability/grade_batches.py` writes
    the subagent graders' batch files. The owner chose the two gating
    models on all four tasks; `prepare` wrote
    `tests/readability/2026-10-06-3e7c45a/`. Sonnet's 345 samples came
    with no failure (335 in the main run, 2333 seconds of session time
    four prompts at a time); gpt-5.5 stopped at 72 (Predict complete, 22
    Explain) when the Codex CLI reported the ChatGPT login's usage limit
    ("try again at Nov 4th, 2026"), and the owner chose to score Sonnet
    now. Explain graded by six subagents (three batches, two graders),
    merged, five adjudications (`file_tree.1` to `.4` graded 4 as in
    rounds 1 and 2, `config.0` graded 5), the U5 search found no
    recurrence. Results: Sonnet Predict 90, Explain 100, Complete 68,
    Write 40; gpt-5.5 Predict 100. The notes name the defect (item 18).
    A probe after the round found a second defect, in the channel: run
    from the repository, the CLI loads the project's `CLAUDE.md`, and
    the account's claude.ai connectors (Gmail, Claude Docs) bring 38
    tool definitions that `--tools ""` does not remove; round 3's samples
    had both in context. Fixed before round 4: the CLI runs in an empty
    `--work-dir` with `--strict-mcp-config` and no servers, and a probe
    through the harness's own command shows only the CLI's fixed frame
    (identity line, environment block, date, the email line) around the
    system text.
18. **Decision U8 and the corrected cheat sheet** (same commit). The
    round's Complete fell 89 to 68 because the sheet listed
    `rounded(places: 2)` (a named single argument, against M4) and
    `split()` (the separator hidden), both Claude's rendering of U2 and
    U3 and neither run through the checker before the round; Sonnet
    copied them in every sample of `invoice`, `shapes` and
    `compound_interest` and three of `word_count`. Asked as a batch of
    three; the owner chose the list with parameter names
    (`split(separator)`, `rounded(places)`, `replace(old, new)`; every
    entry checked against `library/std/prelude.ry`) plus the sentence
    "One parameter is positional, more are named" with three calls,
    round 3 as a defect round with round 4 on the corrected sheet
    through the same channel, and `count` kept reserved. Paid for by
    shortening the phrase-token list, dropping the nesting and body
    limits, the reserved-word count and "(explicit)" after `Float`:
    2998 tokens. `01-decisions.md` (U8), `03-readability-test.md`,
    `README.md`.
19. **Round 4** (fifteenth commit). The channel fix of item 17 in
    `run.py` (`call_claude_cli` takes `--work-dir`, runs there, passes
    `--strict-mcp-config --mcp-config '{"mcpServers":{}}'`; the
    `source` record names both), probed through the harness's own
    function before the run (0 tools, no instruction file, no MCP
    instructions). `prepare` wrote `tests/readability/2026-10-06-c696747/`;
    345 Sonnet samples in 2350 seconds of session time, no failure;
    Explain graded by six subagents, merged, two adjudications
    (`expression_tree.0` and `.4`, the program's own purpose line, graded
    5 as round 3's graders did), the U5 search found no recurrence.
    Results in the table above and in the round's `notes.md` (sections 1
    and 6 carry the analysis and the three questions). Round 3's notes,
    the README and this file corrected for the channel (the project
    `CLAUDE.md` and the claude.ai connectors were in round 3's context).
    A memory note on the channel recipe was saved outside the repository.
20. **Decision U9** (sixteenth commit): the owner's answers to round 4's
    questions. The sheet says when a refined construction takes
    `otherwise` ("A literal the checker can evaluate needs no
    `otherwise` (`Port(8080)`); a variable needs one even after a
    check"), paid for by the phrase-token sentence, the `# flatten`
    comment and "`renyi run` enforces it", 2991 tokens; the five-point reading stays in the notes;
    gpt-5.5 deferred. `01-decisions.md`, `03-readability-test.md`, round
    4's notes.

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

- **The residue of stage 1** is done (decisions Y1 to Y4) except open
  item R3-2 (constraints with type arguments), deferred until after the
  emitter; `docs/GAPS.md` section 7 keeps 1.11, 3.4 and 3.6 open.
- **The rename of `set` to `change`** (the owner's decision of
  2026-10-06, asked in this session after the owner raised it): done
  before the emitter, as a surface change under decision V11 (a decision
  entry first, then the reference, the grammar, the cheat sheet, the
  formatter, both front ends, the corpus, the conformance suite and the
  readability harness's inputs in one commit). The grammar has no `=`
  and binds a name once per function (`shadowing`), so `set` on a `let
  mutable` is already the only way to change a value; the rename removes
  the three meanings of `set` (the statement, the `Set` type, the
  `Map.set` method).
- **Stage 2, the next piece**: the emitter written in Renyi
  (`compiler/emit.ry`, the transcription of `crates/renyi_vm/src/
  compile/`, and the driver `compiler/compile.ry`), judged by byte
  equality with `renyi compile` over the corpus, the conformance
  programs and the compiler itself (decision Z3); the file, the
  loader and `renyi compile` exist (Z1 to Z4).
- **Readability**: the scores are no longer the gate (decision V1).
  Round 5 (U9's sentence) and gpt-5.5 run only if the owner asks.
- Session 5 printed the values of `ANTHROPIC_API_KEY` and
  `OPENAI_API_KEY` into a tool result once (not committed); the owner
  said in session 6 that they handle the transcript and the keys
  themselves, so no session needs to raise it again.

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

1. **Stage 2 of `docs/GAPS.md`, section 7, continued** (the grammar is
   frozen, V11; the formal grammar is `docs/grammar.ebnf`, V12; the
   language reference is `docs/reference.md`; the lexer and the parser
   in Renyi are `compiler/`, W1 to W4; the checker in Renyi is
   `compiler/declare.ry`, `bodies.ry` and `checker.ry`, W5 to W8; the
   profile and the loop are done, X1 to X6; the residue of stage 1 is
   done, Y1 to Y4, R3-2 deferred; the bytecode file, `renyi compile`
   and the loader are done, Z1 to Z4): first the rename of `set` to
   `change` (the owner's decision of 2026-10-06, a surface change under
   V11: a decision entry, the reference, the grammar, the cheat sheet,
   the formatter, both front ends, the corpus, the conformance suite and
   the readability harness's inputs in one commit), then the emitter
   written in Renyi (`compiler/emit.ry`: `compile/{mod,expr,stmt,pattern,query}.rs`
   and `types.rs` transcribed over `declare.World`, the tree and the
   references of `bodies.ry`; a driver `compiler/compile.ry` that runs
   the front end, the checker and the emitter and writes
   `json.render_indented` of the `Program` of `compiler/bytecode.ry`),
   judged in `selfhost.rs` by byte equality with `renyi compile --to`
   over the corpus, the conformance programs and `compiler/`, with
   both refusing what the Rust checker refuses (Z3),
   so that the Rust VM runs what the Renyi compiler emits, with the
   Rust toolchain as stage 0, which is where the references of W8 get
   their judge; the remaining performance items (string building, the
   pattern cache of `Text.matches`) as a later profile calls for them;
   any
   change to the loop is measured with `bench.py` and `micro.py`
   against the `base` worktree, the two binaries run back to back. The
   front end in
   Renyi reports one syntax error with a position and no fix; parity
   with the Rust parser's diagnostics (codes, fixes, recovery) is a
   later step.
2. **Readability, only on request**: round 5 on Sonnet measures U9
   (`run.py prepare`, the Sonnet command at the end of this item after
   its probe, the graders, `score`, the U5 search, `report`, the
   notes; `config` and `todo_cli` are the items to watch); gpt-5.5 on
   the corrected cheat sheet when the owner lifts the deferral and it
   has a channel: `run
   --provider codex --model gpt-5.5 --work-dir <empty dir> --parallel 3
   --run 2026-10-06-c696747` (restartable; the Codex quota returns Nov
   4th, 2026), or the API path with the owner's authorization; then
   `grade_batches.py`, six grader subagents, `merge_agent_grades.py`,
   `score` with `--grader agent:claude-sonnet-5-5 --second-grader
   agent:claude-opus-5-5 --temperature none` (and `--no-format --scores
   scores-strict.json`), the U5 search, adjudication, `report`, the
   notes. Round 3's 72 gpt-5.5 samples can be finished the same way in
   `2026-10-06-3e7c45a` for the record. A Sonnet round through the
   Claude CLI is `run --provider claude --model claude-sonnet-5-5
   --config-dir <login only dir> --work-dir <empty dir outside the
   repository> --parallel 4` after a probe through `call_claude_cli`
   (expect 0 tools, no instruction file, no MCP instructions); the
   login-only directory holds a hard link to the CLI's
   `.credentials.json` and a `.claude.json` of
   `{"hasCompletedOnboarding": true}`.
3. **Readability observations not asked**: `otherwise` binds loosest
   (Sonnet wrote `check f(x) otherwise "" is "y"`); Sonnet reasons before
   the Predict answer on `traffic_light` (0 of 5 with right lines); the
   rubric's reading of "only X and Y" when Z is also needed (round 1 and
   2 read it as a need missing).
4. **M4** (the package manager, budgets in the manifest, `std.process`,
   FFI) and **M5** (embedding API, `serve --watch`, LSP, a resident
   `World`, open item R5-5) as before: stage 3 of `docs/GAPS.md`.

## Known gaps and risks

- **The front end in Renyi.** The parser stops at the first error with
  a message and a character offset: no diagnostic codes, no fixes, no
  recovery, so it is not yet a replacement for `renyi check`'s
  front-end diagnostics; the judge covers acceptance and the tree only
  (the checker's judge likewise requires only failure on a program the
  Rust parser rejects).
  Its equality with the Rust parser is exact on every program in the
  repository, by construction of its `peek` (the Rust parser's
  side-effecting peek is simulated, line-break spans included); a new
  layout rule in `parser.rs` must be mirrored in `parser.ry` or the judge
  fails. The lexer's error messages are its own, not the Rust lexer's
  codes. The run time grows with the file (0.6 s for `parser.ry` in the
  release build after X3; the profile above says where it goes); the
  `"{text}{ch}"` concatenation in the lexer's `scan_segment` is
  quadratic in a token's length (negligible for tokens, visible on a
  long block text; `slice` is one `List.slice` since X2). `json.rs` and
  `ast.ry` must change together (W1); nothing checks that the Rust
  encoder's keys match `ast.ry` except the judge's byte comparison.
- **The checker in Renyi.** It is a transcription: a change to a
  message, a fix, a rule or the order of checks in `crates/renyi_check`
  is a change to `declare.ry` or `bodies.ry` in the same commit, or the
  judge fails (CLAUDE.md says so). The by-construction differences
  listed above (hash-map ties, control characters in `json_string`,
  the NUL in a `Path`, `debug_quoted`, the import path separator, the
  imports of a broken import) show on no program in the repository and
  have no test. The checker's run time after X3: 2.3 s on `bodies.ry`
  in the release build, 2.7 s in the optimized development build; the
  two judges add about 51 s to `cargo test` on eight threads and more
  on CI's runners. The Rust
  checker's quirk that a `failure(x)` binding's fields are unknown is
  reproduced on purpose (W7: equality first); fixing it is a change to
  both checkers and a conformance case.
- **The bytecode file.** JSON, about 170 KB for `hello.ry` since the
  whole library's metadata travels with every program; the loader's
  consistency pass checks indices, not stack discipline, so a file
  written by hand can still underflow the operand stack (a panic, not
  an exploit: the VM holds no unsafe code). The name of a test's code
  object is Rust's `{:?}` of the test's name, which the Renyi emitter
  must reproduce (quotes and backslashes escaped; other escapes do not
  occur in the corpus). A `.ryc` is tied to the toolchain that wrote
  it by nothing but `format`; a change to `Op`, `Program` or the
  format types bumps `FORMAT` and `compiler/bytecode.ry` together.
- **VM.** A declared `equals` decides `is` and `is not` (session 7);
  `contains`, `index_of`, sets and maps keep the derived form, and a
  user `Hash` implementation is never called, by decision Y3 and the
  reference's section 5. A
  value's `IsType` test for a builtin (`when failure(error: Text)`) is by
  kind. `random` is seeded from the clock; a run is reproducible through
  its recording only. `Float.to_text` is Rust's shortest round-trip form.
  `Text.matches` compiles its pattern at every call. Durations print as
  `1.5s` / `250ms` (no decision covers the format). A refinement condition
  on a library type (`Date`, `Port`) runs on construction but its
  references are not recorded (the checker does not walk library bodies);
  only literals and local names occur there today. An untyped `failure(x)`
  pattern on a call declared `or fails with Fault` gives `x` a type on
  which the record's fields are unknown (`unknown-field` on `x.message`);
  a typed pattern `failure(x: Fault)` or a destructuring one
  (`failure(Fault(message))`) works. Not yet reported as a checker
  item; found while writing the compiler, and met again in the checker
  (`checker.ry` destructures).
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
  the program's directory. The grant stack narrows scopes and nothing
  else: a capability kind a function does not declare passes through its
  frame, where only a function value it calls can use it; the static rule
  keeps everything else out. `--redact` works by name: a secret that
  reaches another argument (a printed line, a URL) is recorded as it is.
  The manifest hashes the standard output only: `console.print_error`
  lines and the narration go to stderr and are not compared. `reproduce`
  checks the local program's hash; fetching code by hash needs the
  registry (M4). The toolchain is named by its version only, not by a
  build hash. A `--manifest` run collects its calls in memory as `record`
  does.
- **MCP and the diff.** The map is rebuilt whole when any file changed
  (T4), and every tool call that needs it re-reads the served directory.
  The server does not implement `subscriptions/listen`, pagination,
  progress or cancellation (a `notifications/cancelled` is ignored; a
  running program runs to its end), answers a legacy `tools/call` without
  an `initialize` first, and validates `_meta` only when it carries a
  version. `run` executes effects for real unless `replay` is given; a
  program's relative paths resolve against the served directory, which
  the server enters at start. The library lookup scans the declaration
  files by line, so a declaration inside an `ability` block is not
  listed. The diff matches a rename by content hash only, so a definition
  renamed and edited in one step reads as removed and added; a
  definition moved between modules likewise (its qualified name changes);
  an `effects` change is listed on every definition it reaches, which is
  by design but makes a widened leaf verbose in a deep call graph; a git
  base indexes the revision's files afresh on every call (no cache); the
  revision's import closure for a single-file base follows `import`
  lines only (no `std` resolution needed). Open item R5-2 (local names in
  the content hash) stands.
- **Network, server, SQLite.** The server is single-threaded and answers
  one request at a time, binds `0.0.0.0`, speaks plain HTTP/1.1 without
  TLS, reads a head of at most 64 KiB and a body of at most 16 MiB, and
  has `Options::serve_limit` only so that a test can stop it. The client
  reads at most 10 MB of body (ureq's default; past it the call fails as
  `Unreachable` with ureq's message), follows at most ten redirects and
  sends ureq's own `User-Agent` (`ureq/<version>`). SQLite binds a
  `Decimal` parameter as text, so SQL that compares it with a REAL column
  compares text; the type mapping is exercised by `tests/network.rs`
  against a real database and by the `inventory_db` recording (its test
  seeds its own table). `renyi test --refresh` prints the fixture path with mixed
  separators on Windows (`examples\fixtures/weather.json`): the relative
  name is joined onto the source directory as given.
- **Checker.** First-generation (local inference, covariant type arguments,
  structural ability checks, capabilities checked at call sites only; no
  `example:` literal check, no unreachable-pattern detection). Since
  session 7 exhaustiveness is the usefulness check, `see also:` names are
  checked, `deprecated:` warns, literal patterns, URLs, paths and dates
  are checked, and every diagnostic carries a fix. The `ignore-pure` rule counts a call
  with effects anywhere inside the ignored expression. A field's
  condition binds that field alone since decision Y4 (`refinement-field`
  for another field of the type); the VM's condition code still takes
  every field.
- **Readability.** Explain is graded by subagents in both rounds (round
  1 re-graded, U4). They are about a point more lenient than the API
  graders, let three wrong statements pass in round 1 that the hand
  verdicts call wrong (round 1's `notes.md`, section 8), and read a
  boundary word strictly in one batch and loosely in another. Since U5 a
  hand verdict extends to every sample repeating the sentence, but a
  wrong statement neither grader catches and no hand verdict names still
  passes (a hand pass over every agreed grade was declined). The floor
  model's Explain number carries that; the gating models' failing samples are wrong statements
  both grader kinds mark 2. Sonnet 5.5 and gpt-5.5 cannot be sampled at temperature 0, so
  their rates carry sampling noise of about one item. gpt-5.5's Write
  samples came through the Codex CLI (its own system prompt around ours,
  reasoning effort `medium`), the rest through the API; round 3's Sonnet
  samples came through the Claude Code CLI with adaptive thinking on
  (tokens recorded per session; Predict `permissions` failed only in the
  sessions without thinking), the CLI's fixed frame (identity line,
  environment block, date, a one-line note of the account's email
  address) and, found by a probe afterwards, the project's `CLAUDE.md`
  and the claude.ai connectors' tool definitions (kept out since round
  4 by an empty `--work-dir` and `--strict-mcp-config`). Round 4's
  Complete (79) is ten points under round 2's with the targeted errors
  gone: four items fell on slips the notes attribute to the default
  temperature and to a sheet gap on refined construction and
  `otherwise`; the five-point rule's reading of that is with the owner. A call form written into the cheat sheet must
  be run through `renyi check` before a round: round 3 was lost to
  `rounded(places: 2)` and `split()`. The grant clauses
  (`at most`, `only to`, `replays` in a prompt) have not been through a
  round. `reference/*.explain.txt` must be re-read against the corpus
  before a round (one went stale this session; `prepare` now snapshots
  them into the run, so an old run keeps the text its samples were
  graded against).
- **Library.** The standard library sketch is a first draft from the
  corpus; the JSON derivation rules are exercised by the VM's decoder on
  the three recorded responses; the SQLite type mapping is exercised by one
  test and the HTTP defaults (the 30-second limit, redirect handling) by
  the three recordings and the loopback test only.
- 88 reserved words include common identifiers; the rounds measured their
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
  reviews. `main` is the only branch. In session 8 the owner ran two
  Claude Code sessions on the repository at once; they split the files
  by cross-session message, and one ran the gates and committed. A
  session that finds the working tree changing under it should list the
  peer sessions and ask before editing a shared file.
- The pay-per-token API keys are not spent by default (ruled 2026-10-06):
  subscription quota first (Claude Code subagents, the Codex CLI), the
  keys only when the owner says so in the same request, with the volume
  named. The readability harness drives the two CLIs itself (`run
  --provider claude`, `--provider codex`) and takes grades from the
  subagents' files (`grades.json` buckets) for that reason.
