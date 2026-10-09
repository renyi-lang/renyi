# Handoff

Last updated: 2026-10-09, session 9, the first in the cloud environment
(claude.ai/code), which finished the profile-guided round on strings and
JSON that session 8 had paused: decision AQ, the first of the
interspersed performance items the owner set after M5 (the section "The
profile-guided round on strings and JSON as it exists" below), and did
the baseline JIT, decision AR1, in its three stages: the fused op
`LoadField` in the bytecode (AR2), direct calls between generated
functions with register arguments (AR3) and the layout of `Value`
pinned so that the value operations, the frame protocol and the field
read run in place (AR4), then the tiering measured and the hotness
factor raised (AR5), and the round closed by the owner with the
self-check 15% below where AQ left it (AR6; the section "The baseline
JIT" below); then `renyi build`, the image of a program that runs
without compiling, decisions AS1 to AS4 (the section "`renyi build` as
it exists" below), `build --exe`, the self-contained executable,
included; then the round on the size of the generated code, decisions
AT1 to AT7 (the section "The size of the generated code" below): the
cuts at no cost, the image's program in binary, the static stack
height, the image's code section mapped from the file and the reference
counts as calls, every stage of the owner's plan measured and in, the
round closed by the owner on 2026-10-09 (AT7); then the typed round,
decision AU1, opened by the profile of `bench/strings.ry` on the JIT
tier and the owner's choice of a cure over another cut: the checker's
static types brought to the code generator (the section "The typed
round" below), in progress.
Session 8
(stage 2 of the gap audit of
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
decisions Z1 to Z4, with `renyi compile` and the loader; then the rename
of `set` to `change`, decision AA1; then the emitter written in Renyi,
`compiler/emit.ry` and `compiler/compile.ry`, held equal to `renyi
compile` byte for byte by the same test, decision Z3; then constraints
with type arguments, open item R3-2, decision AB1; then the judges run
the front end from its bytecode; then packages, decision AC1, the first
slice of stage 3 in two commits; then the diagnostics of the front end
in Renyi, decision AD1: the Rust lexer's and parser's codes, fixes and
recovery, the three judges byte-equal on rejected programs too; then
`std.process`, decision AE1, the second slice of stage 3; then the
foreign function interface, decision AF1, the third; then machine code
for the bytecode, decisions AG1 to AG5, and the bounded VM round, AG6;
then the niche, decisions AH1
to AH4 with `docs/design/08-positioning.md` and the README's opening;
then release 0.1 decided, AI1 to AI4, and its engineering: the
release workflow, the installers, the VS Code extension and the
procedure `docs/RELEASE.md` (621fecc); the starter pack for agents
under `starter/`, AI3 (ee8b74b); the documentation site, `tools/site.py`
with `.github/workflows/pages.yml` and `docs/index.md`, and the
crates.io metadata; then decision AI5, the copies that make the two
crates self-contained, so that `cargo package --workspace` verifies
all seven; then foreign packages decided, AJ1 to AJ4: Rust natives
through a registration API, Python through a typed bridge, after
0.1; then release 0.1.0 itself, the evening of 2026-10-07: the
repository public under `renyi-lang/renyi`, the seven crates on
crates.io, the tag `v0.1.0` with its release and the site live; then
the registration API for Rust natives, decisions AK1 to AK4: the
standard library as the first extension, `renyi` a library too, the
guide `docs/extensions.md`; the domain renyi-lang.org in front of
the site; and the Python bridge, decisions AL1 to AL4: a Python module
bound by the manifest as a foreign one is, one worker per run,
`PythonError`, the guide `docs/python.md`; and the Python binder,
decisions AM1 and AM2: `renyi bind --python` writes the declaration
file from the package; and the resident world, decision AN1:
`renyi_workspace`, which `renyi mcp` holds between calls, the first of
M5's four steps in the order the owner set; and the language server,
decisions AN2 and AN3: `renyi lsp` on that world, the client in the
VS Code extension; the watch of `serve`, decision AO1: `renyi serve
--watch` runs `main` again between two requests on a clean new
version, the socket kept open, the third step; and the embedding API,
decisions AP1 and AP2: `renyi::Sandbox` with the grant as a `needs`
clause and the memory budget, `renyi run --sandbox`, the fourth step,
which completes M5).
Branch: `main` is the only branch (owner's decision, 2026-10-05); commit
and push there directly.

## Where the project stands

Milestones M0 (design), M1 (front end), M2 (type and effect checker) and
M3 (the VM: `renyi run`, `record`, `run --replay`, `reproduce`,
`--explain`, `test` with `replays`, budgets, the grant stack of Q1, the
run manifest of Q2, every library module, the `only to` guards of P3,
tasks one after the other by S2) are done, with the project map (`renyi
index`, `--budgets`, `--diff`), `renyi tools` and `renyi mcp` on top. M4
is done but for its residue (packages AC1, `std.process` AE1, the FFI
AF1; `docs/GAPS.md`, section 4); M5 is done: the registration API of
decisions AJ1 and AK1 to AK4 (the section "The registration API as it
exists" below), the Python bridge of decisions AJ2, AJ3 and AL1 to AL4
(the section "The Python bridge as it exists" below) with its binder
of AM1 and AM2 (the section "The Python binder as it exists" below),
and the four steps in the owner's order: the resident world of
decision AN1 (the section "The resident world as it exists" below),
the language server of decisions AN2 and AN3 (the section "The
language server as it exists" below), the watch of `serve` of decision
AO1 (the section "The watch of `serve` as it exists" below) and the
embedding API of decisions AP1 and AP2 (the section "The embedding API
as it exists" below); of
M6 the
machine code exists (decisions AG1 to AG5), the interpreter had its
bounded round (AG6) and the strings and the JSON path had the
profile-guided round of decision AQ (the section "The profile-guided
round on strings and JSON as it exists" below), the baseline JIT of
decision AR1 is in (the section "The baseline JIT" below: the fused op
`LoadField`, the direct calls with register arguments and the pinned
layout, decisions AR2 to AR4, the tiering AR5) and `renyi build`
writes the image a run loads in place of compiling and, with `--exe`,
the self-contained executable (decisions AS1 to AS4).
Release 0.1.0 is out
(2026-10-07, the section "Release 0.1 engineering" below): the
repository is public at `github.com/renyi-lang/renyi`, the release
page carries the three archives, their checksums and the `.vsix`,
the seven crates of 0.1.0 are on crates.io (`cargo install renyi` builds
0.1.0), the installers were run against the release, and the site is
at `renyi-lang.org` (the GitHub Pages address redirects there).
Design decisions are
in sections 0 to AN of `01-decisions.md`; the positioning in
`08-positioning.md`; the agent tooling in
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
statement `set name to value` is `change name to value` since decision
AA1 (the owner's question of 2026-10-06; the section after the bytecode
file's); the emitter written in Renyi, which writes that file, exists
and is held equal to `renyi compile` byte for byte (decision Z3, the
section after the rename's): the toolchain in Renyi is the lexer, the
parser, the checker and the emitter, with the Rust toolchain as stage 0.
Stage 3 began on 2026-10-07 with packages (decision AC1, the section
"Packages" below).

The corpus has 30 programs, passes the lint, is in canonical layout,
checks cleanly, has nothing over budget, and its `example:` lines and
`test` blocks pass on the VM (six are `replays` tests answered from
recordings under `examples/fixtures/`). The cheat sheet measures
2999 of 3000 tokens. The Rust workspace has eight crates:
`renyi_json`, `renyi_syntax`, `renyi_package`, `renyi_check`,
`renyi_index`, `renyi_workspace`, `renyi_vm` and the `renyi`
binary with `check`, `format`, `tokens`, `parse [--json]
[--declarations]`, `index [--json | --budgets | --diff <map or
revision>]`, `run [--manifest] [options] <file> [arguments]`, `record
[--to file] [options] <file> [arguments]`, `reproduce <recording>
[<file>]`, `test [--strict] [--refresh name [--redact name]] [--explain]
<file>...`, `compile [--to file] <file.ry>`, `add <name> [<version>]`,
`update [--accept-effects]`, `audit`, `fetch`, `publish [--to
<directory>]`, `bind <header.h> --module <name> --library <names>
[--to <directory>]`, `tools [path]`, `mcp [path]`, `lsp`, `serve [--watch]`, `run --sandbox` and `version`; 317 tests,
clippy and fmt clean with rustc 1.94.1 on Windows (the owner's machine)
and on Linux (the cloud environment, where 1.94.1 is installed beside
its 1.97.0 for the gates). CI (`.github/workflows/ci.yml`) runs the same gates,
`renyi check compiler/*.ry`, the starter pack's workflows (check,
format, test, the cheat sheet's copy) and the conformance suite
(`tests/conformance/`, 54 cases, every `run` case a second time from
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
(decision V1). The rounds' records (`system.txt`, the outputs) keep the
grammar they measured, with `set`; a new round reads the current cheat
sheet, with `change` (decision AA1).

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
  `LineBreak`, `Symbol(text)`, `ErrorToken` and `EndOfFile`; it reports
  every bad character with the Rust lexer's code, message and fix and
  reads on (`Lexed`: the tokens and the diagnostics; decision AD1).
  `parser.ry` turns tokens into the tree
  rule for rule as `crates/renyi_syntax/src/parser.rs` does; the Rust
  parser's mutable cursor is a `Cursor` record (tokens, position, open
  brackets, whether functions have bodies) threaded through every
  function, each returning `Parsed of Node` (the node and the cursor
  after it); `peek` moves past the line breaks the layout rules make
  insignificant (inside brackets, after a comma, before a continuation
  word) exactly as the Rust `peek` does, including the spans that end at
  a line break after such a move, so that the two trees agree byte for
  byte; since decision AD1 it reports every syntax error with the Rust
  parser's code, message, span and fix, recovers where the Rust parser
  recovers and returns the partial tree with the diagnostics
  (`ParsedModule`; the section "The diagnostics of the front end in
  Renyi" below). `parse.ry` is the command line: `renyi run
  compiler/parse.ry [--declarations] <file>` lexes, parses and prints
  `json.render_indented` of the tree, then the diagnostics as `renyi
  parse --json` prints them, and exits 1 on an error; `tokens.ry`
  prints the tokens as `renyi tokens` does, then the lexer's
  diagnostics (a development aid for comparing the lexers line by
  line).
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
  [--declarations]` byte for byte, the exit status and the diagnostics
  of a rejected program included since decision AD1; a second test does
  the same for the checker (next section);
  a third checks `renyi format --check compiler/*.ry`. CI runs `renyi
  check compiler/*.ry` besides. Every judge runs its driver from the
  bytecode `renyi compile` writes of it when the test starts (the
  section "The judges run from bytecode" below). Every compiler source
  is `renyi check` clean (no warnings) and in canonical layout.
- **Speed, the first measurement** (W4): on the VM the lexer and the
  parser take about 0.3 s on `examples/hello.ry`, 0.6 s on the prelude
  declarations, 1.0 s on the 750-line `lexer.ry` and 3.9 s on the
  2700-line `parser.ry` (`renyi check` of the compiler alone, which
  every run pays first, is 0.25 s). The lexer slices token text by
  characters in a loop, since `Text.drop` and `take` copy the whole text
  per call (79 s for `parser.ry` before that change). Growing a list
  with `change xs to xs.append(x)` or `xs.append_all(step.xs)` in a loop is
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
  [--json] [--strict] [--library <dir>] <file>...` reads the fifteen
  library declaration files from `library/std` under the working
  directory (`--library` names another), lexes and parses with the
  Renyi front end, reads the imports from the file's directory as
  `renyi_check::imported_files` does (a stack, the last import first;
  `.ry` then `.renyi`; `std` skipped), declares, resolves, checks the
  bodies, adds `module-name` (G3) and `import-errors`, the layout
  warnings of `layout.rs` (`line-width`, `trailing-whitespace`), sorts
  by start, turns `deprecated` into an error under `--strict`, prints
  the JSON or the text, and exits 1 on any error. A file that does not
  lex or parse gets the front end's diagnostics, the Rust codes and
  fixes (decision AD1), the layout warnings and exit 1; the imports of
  its partial tree are followed as the Rust resolver follows them.
- **The judge** (decision W7): the test
  `the_renyi_checker_prints_what_the_rust_checker_prints` in
  `crates/renyi/tests/selfhost.rs` runs `renyi parse` to learn whether
  the Rust parser accepts a program, then `renyi run compiler/checker.ry
  --json [--strict]` against `renyi check --json [--strict]` on every
  program of `examples/`, `tests/conformance/programs/` and `compiler/`
  (output byte for byte and exit status, on a program with syntax
  errors too since decision AD1). All equal at the
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
  two equally close names may be broken differently; the `Path`
  literal check tests only `"\n"` (a
  NUL cannot be written in Renyi); `debug_quoted` escapes `"`, `\`,
  `\n` and `\t` where Rust's `{:?}` escapes every control character;
  the `import-errors` message joins the import's path with `/` where
  Rust uses the platform separator (no program in the repository
  imports a module with errors). Positions are characters on the Renyi
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
  arm is `nothing`; a one-line `if c then change a to x otherwise change a to
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
  anchors for a patch script are taken from the file; `ignore` of a
  call that has no effect is an error even when the call can fail (bind
  the result, or make the function return nothing and call it as a
  statement with `otherwise fail`); a call with one argument must not
  name it.

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
  op kind with the primitives by name, then the counts: by kind, by pair
  of kinds run one after the other in one code object (session 9, for the
  fused ops of decision AR1), by function, the primitives (the top twenty
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
- **R3-2** (constraints with type arguments) is done: decision AB1, the
  section "Constraints with type arguments" below.

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

## The rename of `set` to `change` (decision AA1)

The owner asked, while the bytecode file was being built, whether to
rename `set` to `change` and forbid every other way of changing a
binding. The second half was already the rule (no `=`; a name is bound
once per function, `shadowing`; `set` on a `let mutable` binding the
only mutation), so the question was the rename alone, asked back as one
structured question and answered "before the emitter". A surface change
under V11, one commit:

- **The grammar and the documents**: `docs/grammar.ebnf` (`'change'
  Identifier 'to'`), the reference (sections 1, 6, 8 and 17; 88 reserved
  words still, `set` out, `change` in), the cheat sheet (the same list,
  the examples; 2998 tokens), section AA of the decisions,
  `docs/GAPS.md`.
- **The Rust front end**: `Word::Change` in `token.rs`,
  `StmtKind::Change` in `ast.rs`, `parser.rs` (and its statement
  message), `json.rs` (the kind `Change`), `format.rs` (`change x to`);
  `check.rs` (the arm and the fixes `write `change x to
  x.method(...)``), `metrics.rs`, the VM's `compile/stmt.rs`.
- **The front end in Renyi**: `compiler/lexer.ry` (the reserved list),
  `ast.ry` (`Change(name, value, span)`), `parser.ry`
  (`change_statement`), `bodies.ry` (the arms, the purpose, the fixes),
  and every statement of the compiler's own sources (about 700).
- **The corpus and the tests**: the examples, the conformance programs
  (`body_length.ry` is sixty of them), the crates' tests, the lint's
  reserved words and `=` message. The sweep was one script
  (`sweep_set.py`, five patterns: a line start, after `\n` in a Rust
  string, after a backtick or quote, after `then` or `otherwise`, after a
  quote and spaces), then the keyword plumbing by hand; nothing else in
  the repository spells the statement.
- **Unchanged**: the `Set` type, `Map.set(key:, value:)`, `to_set`, the
  readability rounds' records, the sketch (`02-syntax-sketch.md`, the
  design record) and the earlier decisions, which keep `set` as history.

## The compiler written in Renyi (decision Z3)

The last piece of stage 2's toolchain: `compiler/emit.ry` turns the
checked tree into the `Program` of `compiler/bytecode.ry`, and
`compiler/compile.ry` writes it as `renyi compile` does.

- **`project.ry`** (new): what the command lines share, moved out of
  `checker.ry` unchanged: `Source` and `Front`, `source_of`, the front
  end (`front_end`), the library (`library_trees`), the imports
  (`imported_files`, `directory_of`, `join_path`), `read_file`,
  `print_block`, `has_error`, and `check_project`, which now returns a
  `Project` (the world and one `CheckedModule` per file: its module id,
  its path, its diagnostics and the references of its bodies).
  `checker.ry` keeps the options, the layout checks and `diagnose`.
- **`emit.ry`** (new, about 3500 lines): the transcription of
  `crates/renyi_vm/src/compile/{mod,expr,stmt,pattern,query}.rs` and
  `types.rs`, function for function. The Rust `Compiler` is the record
  `Emitter`, threaded through every function as `Checker` is in
  `bodies.ry` (`Step of Value` carries a value and the emitter); the
  shared `Context` (the world, the globals, the references by body, the
  result types, the field-site counter, and the tables being built:
  codes, function codes, constants, tests, examples, types) travels
  inside it and comes back out between bodies. The references are keyed
  by module and body (`"{owner}/{body key}"`), then by span
  (`"{start}:{stop}"`), in the order the checker recorded them, so
  "the first `Function` target at this span" means the same in both
  compilers. A code's constants are pooled as typed values (`Pooled`:
  an Integer, a Decimal, a Float, a text, a Boolean, nothing or a
  function id), compared with `is` and rendered to digits at the end,
  which reproduces the Rust pool's "an equal constant of the same kind
  is reused" exactly (`1.5` and `1.50` are one Decimal constant). Spans
  need no conversion: the Renyi parser's spans already count
  characters. A `match` of either kind goes through one `emit_match`
  over `GenericArm`s (`BlockBody` or `OutcomeBody`); `run concurrently`
  runs its tasks one after the other (S2), as the Rust compiler does.
  `bodies.ry` made `debug_quoted`, `statement_span`, `expr_span` and
  `pattern_span` public for it.
- **`compile.ry`** (new): `renyi run compiler/compile.ry [--to
  <file.ryc>] [--library <dir>] <file>` reads the file and its imports,
  checks the project, prints every module's diagnostics as `check`
  prints them, exits 1 when any is an error, and otherwise writes
  `json.render_indented(program)` with a trailing line break to the
  target (`<stem>.ryc` in the working directory by default, as `renyi
  compile` names it). A `.ryc` as input is refused with `renyi
  compile`'s message.
- **The judge** (`selfhost.rs`, `the_renyi_compiler_writes_what_renyi_
  compile_writes`): for every program of the corpus, the conformance
  programs and `compiler/`, named relative to the root with `/`, `renyi
  compile --to target/selfhost/rust/<name>.ryc` and `renyi run
  compiler/compile.ry --to target/selfhost/self/<name>.ryc`; when the
  Rust one succeeds the Renyi one must succeed and the two files must be
  equal byte for byte (the first differing line is reported); when the
  Rust one refuses, the Renyi one must refuse. The first full run agreed
  on every program: the 30 examples, the 29 conformance programs (9
  compiled, 20 refused by both) and the compiler's own 18 files, the
  7600-line `bodies.ry` among them.
- **One change on the Rust side.** `renyi_check::imported_files` now
  names an imported file as the importing file's directory and the
  import's segments joined with `/` on every platform (it joined with
  the platform's separator before, `\` on Windows), so that a bytecode
  file, which remembers the paths of its modules, is the same file
  wherever it is written; the Renyi `imported_files` always joined with
  `/`.
- **Time.** The Renyi compiler, run on the VM, compiles a corpus program
  in about a second and `bodies.ry` in about three (checking and
  loading the whole front end included); the judge adds about a minute
  to `cargo test` on eight threads. The judges run the front end from
  its bytecode since the end of the session (the section "The judges
  run from bytecode" below), which takes the checking out of every run.

## Constraints with type arguments (decision AB1)

Open item R3-2, chosen by the owner after the emitter; the four design
questions and their answers are the entry AB1.

- **The rule.** `for any Bag, Item where Bag can Iterable of Item`: a
  constraint, and an ability's `where self can` requirement, names an
  ability with as many type arguments as it declares, any types in
  scope; `type-arity` otherwise ("`Iterable` takes 1 type argument,
  found 0", fix "write `Bag can Iterable of Item`"). In the body the
  parameter is walked with items of `Item`, and the ability's methods
  are called on it with the ability's parameters substituted. At a call
  site `require_constraint` matches the arguments the argument's type
  has the ability with (`World::implemented_args`: the implementation's
  arguments with its parameters read as the type's, a parameter's
  constraint or what that constraint's ability requires, a subtype's
  base, an empty list for a derived ability) against the constraint's,
  which binds what the constraint leaves open; a mismatch is
  `missing-ability` naming the arguments, fix "implement `Iterable of
  Integer` for `Deck`". One implementation per ability and type:
  `duplicate-implementation`.
- **The collections are `Iterable`.** `library/std/prelude.ry` holds
  implementations for `List of Item`, `Set of Item`, `Map of Key to
  Value` (pairs), `Range` and `Text` (characters), with the methods'
  heads alone (both parsers read an implementation without bodies in a
  declaration file; `ImplDeclaration` in the grammar); the free
  `to_list` of `Set` and `Range` moved into them, the library sketch and
  `crates/renyi_syntax/tests/library.rs` follow. The checker's own loop
  rules for the collections are gone: `loop_items` asks
  `implemented_args` for `Iterable`, for a type and a parameter alike.
  An implementation head takes full types after `of` (`Implementation`
  in the grammar; `ability()` of both parsers parses types after `of`
  and demands names when no `for` follows).
- **Rust.** `parser.rs` (`ability`), `world.rs` (`AbilityRef`, carried
  by `ParamInfo.constraints` and `AbilityInfo.requirements`; `resolved`
  on `AbilityInfo`, so that the parameters of every ability of a module
  are known before a requirement is counted; `for_any_params`,
  `resolve_abilities`, `check_requirements`, `implemented_args`,
  `show_ability`, `check_ability_arity`, the duplicate check in the
  implementation branch of `resolve_functions`; an implementation's
  methods take the ability's visibility), `check.rs` (`Deferred` with
  `args`, `require_constraint`, `match_ability_args`,
  `report_missing_ability`, `show_ability`, `is_own_type`,
  `loop_items`, the ability arm of `infer_method_call`, the constraint
  loop of `call_known`), `vm.rs` (`builtin_type_id`: an ability call on
  a base value dispatches by its declared type; `has_type` uses it),
  `natives/prelude.rs` (`iterable_to_list` for `List`, `Map` and
  `Text`).
- **Renyi.** `parser.ry` (`type_list`, `parameter_names`,
  `implementation` with `ability_args`, bodies by
  `cursor.declarations`), `declare.ry` (`AbilityRef`, `PendingAbility`
  and the two-phase `resolve_ability_items`, `add_resolved_constraint`,
  `resolved_requirement`, `check_ability_arity`, `padded`,
  `implemented_args`, `param_args`, `implementation_for`,
  `bind_target_params` moved in from `bodies.ry`, `show_ability`,
  `has_constraint`, `note_duplicate_implementation`,
  `implemented_elsewhere`, `pair_params` public, `adopt_method` sets
  `is_public`), `bodies.ry` (`require_constraint`,
  `match_ability_args`, `missing_ability` with `args`, `show_ability`,
  `is_own_type`, `settle_deferred`, `iterable_item` over
  `implemented_args`, `source_without_iterable`,
  `call_ability_method` substitutes, `check_instance_constraints`),
  `types.ry` (`substitute_maybe` public). Judged by the three judges of
  `selfhost.rs` as before.
- **Tests and documents.** `rules.rs`
  (`a_constraint_names_an_ability_with_its_type_arguments`); the
  conformance cases 39 to 42 (`constraint_arguments.ry`, a run case;
  `constraint_arity.ry`, `duplicate_implementation.ry`,
  `constraint_mismatch.ry`); the reference (sections 2, 3, 4, 5 and 8,
  appendix A), the grammar, the cheat sheet (3000 of 3000 tokens: the
  `at_least`/`at_most` sentence went, the library line keeps them), the
  sketch (section 5, R3-2 in section 18), the library sketch, the
  decisions (section AB), `CLAUDE.md`, `docs/GAPS.md`, this file.

## The judges run from bytecode (session 8, after AB1)

The owner's choice after R3-2 among the three pieces "Next steps"
named. Each judge of `crates/renyi/tests/selfhost.rs` compiles its
driver with `renyi compile` when the test starts (`front_end`:
`compiler/parse.ry`, `checker.ry` or `compile.ry` to
`target/selfhost/front/<driver>.ryc`, written anew on every run, so
that the file is never older than the sources) and passes that file to
`renyi run` in place of the source, which loads the front end (decision
Z4) instead of checking its 25 000 lines first. The emitter's judge
adds the fixed point: the file the Renyi compiler writes of
`compiler/compile.ry` while running from its bytecode must equal the
file it ran from (the first differing line is reported). The lane
script `judge_emit.sh` runs the same way.

- **What it saves**, best of five on the development build, a corpus
  program as the input: a run of the parser 225 to 185 ms, of the
  checker 440 to 303 ms, of the compiler 521 to 345 ms (2432 to 2222 ms
  with `bodies.ry` as the input). `renyi check` of `compile.ry` alone
  takes 419 ms where a one-page program takes 163 ms, and loading the
  9.8 MB `compile.ryc` costs about 100 ms of a run, so a run saves the
  checking of the front end less the loading of its file. The selfhost
  test binary on eight threads: 56.3 s before, 50.3 s after.
  The rest of the judges' time is the VM running the front end on the
  inputs, the compiler's own sources above all (`bodies.ry`: about a
  second to parse, two to compile).
- **Not changed.** The files are build products under `target/`, not in
  the repository; `renyi run compiler/parse.ry` and the other two
  source forms work as before, and `README.md` names them.

## Packages (decision AC1; stage 3, the first slice: two commits)

The owner's choice after the judges ran from bytecode: stage 3 (M4) and,
beside it, the parity of the Renyi front end's diagnostics with the Rust
parser's ("1+2"); the four design questions of packages and their
answers are the entry AC1 (a static registry, JSON manifests, the
package name as the first segment of an import, packages before
`std.process` and the FFI).

- **The rule.** A program's project root is the directory of the nearest
  `renyi.json` in its file's directory or above it (a relative path
  stops at the working directory, an absolute one at its root), else
  the file's directory (J17); its own imports resolve from the root.
  The manifest names the dependencies (name to the version required,
  `major.minor.patch`; a requirement means the same major and at least
  that version) and the registry, a directory (absolute, or relative to
  the root) or an `http://` or `https://` base whose packages are
  fetched into `.renyi/packages/<name>/<version>/` under the root; the
  lockfile `renyi.lock.json` names every package's version and the
  `sha256:` hash of its `package.json`. `import <name>` is the package's
  root module `<name>.ry` and `import <name>.<path>` its file
  `<path>.ry`, both under `<registry>/<name>/<version>/`; the checker
  names the modules `<name>` and `<name>.<module>`; inside a package an
  import reaches the package's own files first, then one of its own
  dependencies; an import reaches a package only when the manifest (or
  the package's `package.json`) lists it as a dependency. A dependency's
  capabilities are covered as a library's are (the kind statically, the
  scope by the grant stack at run time) and the diagnostic names the
  package: "`announce` (package `greeting` 1.0.0) needs `console`, which
  `main` does not declare". `package-missing` (not in the lockfile, no
  registry, not at the registry), `package-mismatch` (the hash differs;
  the package is read all the same) and `manifest-invalid` (a manifest,
  a lockfile or a package file refused: not JSON, not an object, a field
  the format lacks, a wrong type, a bad name or version) are reported on
  the import; the manifest's and the lockfile's problems at the start of
  the main file.
- **Rust.** `crates/renyi_json` (the VM's `Json`, `read_json` and
  `write_json` moved out; `natives/json.rs` re-exports them).
  `crates/renyi_package`: `version.rs`; `manifest.rs` (`Manifest`,
  `Lock`, `PackageFile`, `Versions`, `Budgets`, `Effect`; strict readers
  with one message per refusal, `render` with two-space indentation and
  a fixed key order, so that a package's hash is the hash of its
  rendered `package.json`); `registry.rs` (`Registry`, `STORE`,
  `hash_of`, `is_absolute`, `join`: a textual join with `/` and `.` and
  `..` folded, so that every toolchain names a file the same way);
  `resolve.rs` (`Project::of`, `resolve`: a stack of pending imports,
  the last import of a file first as before, `seen` by qualified name,
  `locate_package` reporting once per package). `renyi_syntax`:
  `SourceFile.package` and `Package`. `renyi_check`:
  `ModuleInfo.package`, `World::set_package` (the rename),
  `imported_module`, `check_project_with_problems`, `charge_of` in
  `check.rs`, `check_file` and `imported_files` through `resolve`; the
  binary's `compile_sources` the same; `renyi_index::load_project` skips
  `.renyi/` and indexes the dependency files it reaches;
  `Bytes.sha256()` in the prelude (`sha2`).
- **Renyi.** `compiler/project.ry` (`resolve`, `root_of` and
  `root_above`, `root_at`, `read_manifest`, `read_lock`,
  `read_package_file` with the Rust reader's messages in the Rust
  reader's order, through `std.json`'s `JsonValue` and a `Refused`
  failure type; `join_path(base, relative)` is the textual join;
  `check_project_with_problems`), `declare.ry` (`PackageTag`,
  `ModuleInfo.package`, `set_package`, `imported_module`), `bodies.ry`
  (`Charge`, `charge_of`, `same_package`), `checker.ry` and
  `compile.ry` (`resolve` in place of `imported_files`). The three
  judges run over the fixture too (`PROGRAM_DIRECTORIES` in
  `selfhost.rs`).
- **Where the two resolvers differ, by construction**: a key given
  twice in a manifest is refused by the Rust reader alone (a Renyi map
  keeps one value per key); a budget spelled `1.0` or `1e2` is a whole
  number to the Renyi side alone; a version part beyond 64 bits is
  refused by Rust alone.
- **The fixture** `tests/conformance/packages/`:
  `registry/greeting/1.0.0/` (`package.json` as `renyi publish` will
  render it, `greeting.ry` importing `words`, `words.ry`,
  `versions.json` beside the version), `project/` (`renyi.json`, the
  lockfile with the right hash, `report.ry` run as case 43,
  `uncovered.ry` case 44), `stale/` (a wrong hash, case 45),
  `unlocked/` (no lockfile, case 46), `broken/` (a field the format
  lacks, case 47). `crates/renyi_package/tests/fixture.rs` holds
  `package.json` canonical and the lock hash right and checks the
  resolver's file order and tags; `rules.rs`,
  `a_dependency_is_charged_with_its_package_named`. The fixture was
  written by `make_fixture.py` in the session's scratchpad; `renyi
  publish` will regenerate `package.json` once it exists.
- **The second commit: the commands** (`crates/renyi/src/packages.rs`,
  `crates/renyi_package/src/select.rs`). `select` chooses one version
  per package: a requirement means the same major and at least that
  version, the choice is the highest the registry has, the chosen
  packages' own requirements join the manifest's, and the rounds repeat
  until the choice is stable; requirements that disagree on the major
  are refused naming both parties. `renyi add <name> [<version>]`
  (refuses `std`, a name that is not a package name, a name that is a
  directory of the project, and needs a `renyi.json` with a `registry`)
  writes the requirement (the version given, else the registry's
  highest), chooses, fetches and verifies every package, prints the
  added package's public functions with their effects and failures, and
  writes both files. `update [--accept-effects]` takes every dependency
  to the highest version its requirement allows; the capability kinds
  of the new version's effect manifest are compared with the old
  version's, a widening is refused without the flag, and with it each
  `main` of the project that reaches the package (found through the
  index: a public or private function `main` of an own module; reach
  through `resolve_in`) must declare the new kind. `audit` prints every
  locked dependency's kinds, whether each `main` that reaches it covers
  them (exit 1 when one does not), and the capabilities of a `main` no
  dependency uses. `fetch` takes the locked packages (the lock's hash
  must be the registry's). `publish [--to <directory>]` requires the
  project to check clean, publishes every `.ry` and `.renyi` under the
  root (`.renyi/` left out), writes `package.json` with the files'
  hashes and the effect manifest, updates `versions.json`, never
  overwrites a version, and checks the version against the semantic
  diff of the own definitions since the highest published version (G1:
  a new major, a new minor, or greater). A URL registry is read through
  `ureq` (404 is "not there"), its packages are written into the store
  and verified there; a directory registry is read in place. The effect
  manifest is recomputed from the sources on every fetch through the
  index (every public function or method of the package's own modules,
  its transitive effects and failures, sorted by name), with the package
  checked as a project of its own rooted where its files are and its
  dependencies read from the registry or, for a URL, from the store.
  The run manifest names every package the program reaches with the
  lockfile's version and hash (`Dependency` in `recording.rs`, after
  `code`; none for a program loaded from a `.ryc` file, whose hash
  covers them), and `reproduce` refuses a run whose dependencies differ.
  `renyi index` labels a dependency's modules and definitions with
  `package: <name> <version>` (the text line and the JSON field, which
  `maps.rs` reads back; the canonical copies the index checks keep the
  tag, which they lost before), and `--budgets` reads the thresholds of
  `renyi.json`. The commands act on the working directory, so they are
  not conformance cases: `crates/renyi/tests/packages.rs` runs the
  binary in scratch projects under `target/packages/` (seven tests:
  `publish` writes the fixture's `package.json` byte for byte and
  refuses an overwrite, an unclean project and a version too small;
  `add`, the run manifest, `reproduce` on a changed dependency; `audit`;
  `fetch` on a changed file and a stale lock; `update` refused without
  the flag and until `main` declares the new kind; a URL registry served
  by a thread; the map's labels and the budgets).
- **Next**: nothing of M4. The parity of the Renyi front end's
  diagnostics with the Rust parser's (the "2" of the owner's answer)
  is done: the next section; `std.process` (decision AE1) and the FFI
  (decision AF1) are done: the sections after it.

## The diagnostics of the front end in Renyi (decision AD1)

The "2" of the owner's answer "1+2" (2026-10-07): the front end written
in Renyi reports what the Rust front end reports, so that the three
judges compare output and exit status on every program, rejected ones
included, and `checker.ry` and `compile.ry` answer as `renyi check` and
`renyi compile` do on a file with syntax errors too.

- **The lexer** (`compiler/lexer.ry`): `lex(source) returns Lexed`, the
  tokens and the diagnostics in source order; every error site of
  `lexer.rs` is mirrored with its code, message, span and fix (`tab`,
  `crlf`, `equals-sign`, `semicolon`, `symbolic-operator`,
  `unknown-character`, `identifier-shape`, `single-letter-identifier`,
  `type-name-shape`, `number-shape`, `unterminated-text`,
  `unknown-escape`, `empty-hole`, `text-in-hole`, `unterminated-hole`);
  a bad character becomes an `ErrorToken`, which the parser's `expected`
  is silent about, as the Rust parser is. A carriage return is found
  through the base64 of its byte (`\r` is not an escape in Renyi);
  `tokens.ry` prints it as `\r`.
- **The parser** (`compiler/parser.ry`, 3400 lines): the `Cursor`
  carries the diagnostics so far and the source characters; a construct
  that cannot be parsed stops with `Halt`, a failure type that carries
  the cursor (its position, nesting and diagnostics); every error site
  of `parser.rs` is mirrored with its code, message, span and fix
  through the helpers `expected` (silent on an error token, the foreign
  spelling of decision C4 as the fix), `describe`, `token_for` (the
  Rust parser's `token_at`), `foreign_spelling`, `halt` and `diagnose`;
  `identifier` takes a reserved word with `reserved-word` and goes on,
  as the Rust one does; the recovery points are the Rust parser's
  (`recover` to the end of the line after a bad import, a bad statement
  in a block, a bad clause or example, and `skip_to_next_item` to the
  next item at the margin after a bad item; `lenient_end_of_line` and
  `lenient_symbol` where the Rust parser ignores the result);
  `parse_hole` parses a hole on its own cursor and reports
  `hole-syntax`; `parse(lexed, chars, declarations) returns
  ParsedModule`, the partial tree and the diagnostics, the lexer's
  first. The transcription left the line-break handling untouched.
- **The drivers**: `parse.ry` prints the tree, then `report.render_json`
  of the diagnostics when there are any, and exits 1 on an error, as
  `renyi parse --json` does; `project.ry`'s `Front` is `Parsed(tree)` or
  `Rejected(tree, diagnostics)`, the partial tree kept because the Rust
  resolver follows the imports of a file that does not parse;
  `check_project` gives a rejected file its parse diagnostics and no
  module, `checker.ry`'s `diagnose` appends the layout warnings and
  sorts, `compile.ry` prints them as `renyi compile` does. `report.ry`'s
  `json_string` writes `\u00xx` for a control character other than
  `\n`, `\t` and `\r` (the code read off the base64 of the byte), as
  the Rust one does.
- **The judges** (`selfhost.rs`): the parser judge compares stdout and
  exit status on every program; the checker judge no longer asks `renyi
  parse` whether a program parses; the emitter judge compares stdout
  and exit status, then the files when both wrote one. The lane scripts
  `judge_parse.sh`, `judge_check.sh` and `judge_compile.sh` under
  `D:\Projects\.worktrees\Renyi\selfhost\` run the same comparisons
  over the repository's programs and the probes under `probes/` (21
  lexer probes from `make_lex_probes.py`, 24 parser probes from
  `make_parse_probes.py`, both in the session's scratchpad), which
  exercise the error sites no program in the repository reaches: 135
  files, all equal, at the end of the session. Conformance cases 48
  and 49 (`lexer_errors.ry`, `syntax_recovery.ry`) hold a sample of
  both in the suite.
- **What bit**: a pattern binds a record's field by name, so
  `failure(Halt(cursor: halted))` is the spelling (`Halt(halted)` is
  `unknown-field`); `x with a: 1,` followed by another field of the
  enclosing constructor swallows that field (parenthesize the `with`);
  `return` followed by a line break ends the statement; `/` on two
  Integers is `integer-division` (`quotient`); a message over 100
  columns is built from two halves in a function, since a line of a
  constant cannot be broken.

## The process module (decision AE1; stage 3, the second slice)

The owner's four answers of 2026-10-07, all the recommended ones: one
call per run (no handle), a status other than 0 a failure, the scope
matched text for text, the parent's environment inherited.

- **The module.** `library/std/process.ry` and section 13 of the
  library sketch: `Completion` (`status`, `output`, `errors`, `bytes`),
  `Options` (`directory: maybe Path`, `environment: Map of Text to
  Text`, `input: Text`, `limit: maybe Duration`), `ProcessError`
  (`NotFound`, `CannotStart`, `Exited(program, status, output,
  errors)`, `Timeout`, `ProgramNotAllowed`, `OverBudget`), the functions
  `execute`, `execute_with`, `attempt`, `attempt_with` (all `needs
  process`) and `defaults()` (pure: no directory, no variables, no
  input, no limit; a program writes `process.defaults() with input:
  text`). `run` is a reserved word, hence `execute`.
- **The natives.** `crates/renyi_vm/src/natives/process.rs`:
  `std::process::Command` with the arguments one by one (no shell), the
  three streams piped, the output and the errors read on two threads
  while the parent waits (a child that fills a pipe does not stall), the
  input written on a third and the pipe closed after it; a limit polls
  `try_wait` every 5 ms and kills the child past it (`Timeout`); the
  status is the code, or 128 plus the signal on Unix; `output` and
  `errors` are decoded with replacement characters, `bytes` is the raw
  standard output. `NotFound` is the spawn error of that kind,
  `CannotStart(program, detail)` any other.
- **The boundary.** Nothing new in `call_primitive`: `grant::effect_of`
  already took the first `Text` argument of a `process` primitive as
  the scope, `scope_contains` compares a program name text for text,
  the counters count it; `vm.rs` maps a denial to
  `ProcessError.ProgramNotAllowed(program)` and an exhausted budget to
  `ProcessError.OverBudget(program)`, as the other modules do; a denied
  call is not counted, a `NotFound` one is. A guarded value (`only to`)
  as the program or an argument fails with `Guarded(origin, sink:
  "process(\"...\")")` before anything starts. A recording holds the
  call with its `Completion`; `run --replay` answers it and starts
  nothing (and prints nothing, as a replay does); `reproduce` shows the
  output.
- **The checker.** `process.ry` is in `LIBRARY` of
  `crates/renyi_check/src/lib.rs` and in `library_modules` of
  `compiler/project.ry` (the order the judges depend on);
  `effects::unavailable` kept `foreign` alone (`capability-unavailable`)
  in `effects.rs` and `compiler/effects.ry` until decision AF1 removed
  it (the next section).
- **The tests.** `crates/renyi/tests/process.rs` (six, every child the
  `renyi` binary itself running a small program, so nothing else needs
  to be installed): `renyi version` through `execute` (status, output,
  the empty errors, the byte count); `renyi nonsense` is `Exited` for
  `execute` and a completion with status 1 for `attempt`; the options
  (a child prints its directory, a variable, the line it read; a
  sleeper is killed at 300 ms); the scope (`ProgramNotAllowed`), a
  missing program and the budget (`OverBudget`); a guard; a recording
  replayed and reproduced with no marker file written. Conformance case
  50 (`process_errors.ry`) prints `NotFound`, `OverBudget` and
  `ProgramNotAllowed` without starting anything (case 20 held
  `foreign` unavailable until decision AF1).
- **What bit.** `run` as a function name (reserved); a call with one
  argument may not name it (`report("x")`); an `otherwise` on a
  continuation line is `otherwise-line` (break inside the parentheses
  instead); a replay prints no console output, by design.

## The foreign function interface (decision AF1; stage 3, the third slice)

The last slice of M4 in the order the owner chose (packages, then
`std.process`, then the FFI); the four design questions and their
answers are the entry AF1.

- **The rule.** A foreign module is a declaration file of the project
  (bodiless `public function`s, each `needs foreign` and nothing else)
  that the manifest's `foreign` section binds to shared libraries:
  `"foreign": {"libc": {"library": ["ucrtbase", "libc.so.6",
  "libSystem.B.dylib"], "symbols": {"renyi_name": "c_symbol"}}}`; the
  libraries are tried in order, `symbols` is optional. The resolver
  tags the file (`Project::foreign_of_file` for the main file by its
  path from the root, `Project::foreign_module` for an own import by
  its qualified name; `renyi_package::tagged` for a command's file,
  which `diagnose` of `main.rs` and `load_project` of the index use),
  `SourceFile.foreign` carries the `ForeignModule`, `check_project`
  parses a tagged file with `parse_declarations` and declares it with
  `is_library = true` (its functions are primitives: no body, recorded
  at the boundary), `ModuleInfo.foreign` holds the binding.
- **The types.** `library/std/foreign.ry`: `Int8` to `UInt64` and
  `Size` as refinements of `Integer`; `crates/renyi_check/src/foreign.rs`:
  `CType` (the spellings `i8` to `u64`, `size`, `f64`, `bool`, `text`,
  `bytes`), `CResult` (`void`, a type, `text_or_null`), `World::c_type_of`
  and `c_result_of`. `declare_function` of `world.rs` runs
  `check_foreign_signature` after the needs loop: `foreign-signature`
  (needs other than `foreign` alone, `or fails with`, `for any`),
  `foreign-type` (a parameter or a result the boundary does not carry),
  `foreign-arity` (over six words, `Bytes` counting two). A declared
  function's parameters are not `unused-binding` (`check_function` of
  `check.rs` marks them used when there is no body).
  `capability-unavailable` is gone from both checkers, the reference
  and the suite.
- **The bytecode.** `FunctionMeta.foreign: Option<Foreign>` (libraries,
  symbol, parameter spellings, result spelling) filled in
  `compile/mod.rs` from the module's binding; `file.rs` writes it as
  `"foreign"` after `"code"` and `FORMAT` is 2; `compiler/bytecode.ry`
  (`ForeignBinding`) and `compiler/emit.ry` (`foreign_binding`) mirror
  it; `compiler/declare.ry` holds the port of `foreign.rs` beside
  `world.rs`'s (`c_type_of`, `c_result_of`, the checks), `project.ry`
  the manifest section, the tagging and the declarations parse.
- **The VM.** `natives/foreign.rs`: at the first call `bind` loads the
  first library of the list that loads (`libloading`, kept in
  `Vm.libraries` by its name list) and looks the symbol up; `invoke`
  marshals the arguments into the words of `foreign_abi.rs` (generated
  by `tools/gen_foreign_abi.py`: `call(address, slots, returns)` matches
  the arity, the class mask and the result class to a `transmute`d
  `extern "C"` signature; at most six words), calls, and reads the
  result back by its C type (a narrower integer by its width, `Text`
  copied from the `char *`, a null `Text` a crash, a null `maybe Text`
  nothing). `Vm::run_primitive` dispatches a function without a native
  to it; the boundary (`call_native`) does the rest: the grant check
  (`foreign`, no scope, no budget), the recording, the replay, the
  `only to` guard. A library or a symbol that is missing, or a `Text`
  with a NUL, is a crash at the call. This is the VM's only unsafe
  code; what the C function does is outside every guarantee (Q3).
- **The commands.** `renyi run` prints "renyi: this program can call
  native code through `libc`" on the standard error when `main` grants
  `foreign` (07-system-design.md, section 2.2); `renyi bind
  <header.h> --module <name> --library <name>[,<name>...] [--to
  <directory>]` (`bind.rs`) writes `<name>.ry` from the prototypes of a
  header (comments and preprocessor lines removed, `extern "C"`
  dropped, the C types mapped; `long`, `float`, pointers other than
  `char *` and variadic functions left as `# skipped:` comments with
  the reason; names snake-cased and never reserved, a single letter
  becoming `argument_N`) and the entry into `renyi.json` when there is
  one (printed otherwise); `renyi publish` refuses a project with
  foreign modules; `renyi format` does not read a foreign module (a
  declaration file, like the library's).
- **The tests.** `crates/renyi/tests/foreign.rs` (seven, every library
  the C library of the platform: `ucrtbase` on Windows, `libc.so.6`
  and `libm.so.6` on Linux, `libSystem.B.dylib` on macOS): `strlen`,
  `abs`, `atoi`, `sqrt` through a renamed symbol and `getenv` as
  `maybe Text`, from the source and from the bytecode file; a
  recording replayed and reproduced with the libraries renamed to
  nothing that loads; `--deny foreign`; a missing symbol and a missing
  library; `bind` on a header, the module written, the manifest
  updated, the module checked; `publish` refused. The fixtures
  `tests/conformance/foreign/` (`libc.ry`, `length.ry` printing 5 and
  7; case 51) and `tests/conformance/foreign_bad/` (`bad.ry`, the
  three diagnostics; case 20) are under the three judges, the Renyi
  checker and compiler byte-equal on them; `tests/fixture.rs` holds
  their manifests canonical; `tests/library.rs` counted fourteen files
  (fifteen since the bridge).
- **What bit.** A heredoc un-escapes backslashes (a patch script
  written through it got a raw carriage return); a patch applied twice
  duplicated the additions whose anchor survived the first run; the
  `check` command parsed its file before the resolver tagged it;
  `json` is the namespace of an import in `project.ry` (no local of
  that name); a name is bound once per function (two `match` arms may
  not bind the same name); a single-letter parameter name is an error
  (`single-letter-identifier`), so the generator avoids it.

## Machine code for the bytecode (decisions AG1 to AG5)

The owner's direction of 2026-10-07: performance first, at the root,
ahead-of-time compilation included; then the language's niche; then the
release. The performance half is done to the point the decisions
record; the niche and the release are decided and not yet written (the
next sections of the plan, below).

- **What exists.** `crates/renyi_vm/src/native/`: `infer.rs` (an
  abstract interpretation over a code object's stack bytecode: the
  entry state of every pc, the kind of every slot, the handler regions
  and the marks; `Abs` is `Unset`, `Int`, `Bool`, `Float`, `Range` or
  `Boxed`, `SlotKind` the same plus `Mark`, `RangeIter`, `Iter` and
  `Deadline`; a code object the analysis cannot settle stays with the
  interpreter), `codegen.rs` (every op to Cranelift IR: typed operands
  and slots are Cranelift variables, boxed operands stay on the VM's
  stack in order; an op that may fail branches to one shared block per
  handler region and operand stack below it, which settles the failure
  on the region's floor and jumps to the handler; a status other than
  `CONTINUE` and `FAILURE` leaves the function; `deopt_here` writes the
  registers into a stack slot and calls `rt_deopt`), `runtime.rs` (the
  helpers, `extern "C"`, each the interpreter's arm on the VM's stack
  through the shared `Vm::op_*` methods; `rt_call` goes through
  `call_from_stack` and `run_top_frame`, so a callee runs natively when
  it has code; `rt_deopt` rebuilds the interpreter's frame), `mod.rs`
  (`Jit`: the Cranelift module, the helpers declared as imports, the
  state of every code object, the hotness rule and the report).
  `vm.rs`: `run_top_frame` tries the generated code first; the `call!`
  macro of the loop does the same for a callee; `hotness` counts the
  ops the interpreter runs per code object (`ran` in the loop, flushed
  at `reload!` and `leave!`); `Options.interpret`. The binary runs its
  command on a thread with a 64 MB stack (`main` spawns `dispatch`),
  and `DEPTH_LIMIT` (200) bounds the generated frames nested on it.
- **The tiering.** A code object is compiled once `hotness >=
  HOT_FACTOR * ops` (8000 since decision AR5, 2000 before it): Cranelift
  spends about a quarter of a million instructions per op compiled and
  a compiled op saves a few tens each time it runs, so a code object
  pays for its compilation after some ten thousand runs of each of its
  ops. The machine code takes over at the
  next call, or at the next turn of a loop the interpreter is in: the
  `Op::Jump` arm of the loop, on a jump backwards of a hot code object,
  asks `native_resume_of_top` for the entry at the target and runs it
  (`RETURNED` leaves the frame as `leave!` would, `DEOPT` reloads,
  `STAY` marks the code's resumes refused). The generated function's
  prologue dispatches on its `pc` parameter: a loop header's entry takes
  every typed slot from the frame through `rt_resume_*` (a `Nothing`
  slot, not yet assigned, gives a zero; a slot that does not fit returns
  `STAY`), then jumps to the header's block. Only headers whose entry
  state has nothing on the operand stack in registers get an entry.
  `RENYI_NATIVE_HOT=0` compiles everything at its first call and enters
  every loop at its first turn (the conformance suite's third pass and
  `tests/native.rs` use it). The interpreter iterates a range of small
  Integers through `Native::RangeIterator` (the next value, the last and
  the step), the shape the generated code keeps, so `rt_deopt` and
  `rt_resume_range` convert without listing the items.
- **The numbers** (development build unless said): `bench/primes.ry`
  84 ms against CPython's 529 ms; `hello` 46 ms either way; the front
  end forced native on `compiler/lexer.ry`: 676 code objects, 39,502
  ops, 240,713 Cranelift instructions in 22,202 blocks, 0.42 s, of
  which Cranelift 0.31 s (register allocation 0.23 s; `single_pass`
  was slower, `opt_level=speed` 25% slower for no run-time gain, the
  verifier another 25%); the compiler on `compiler/bodies.ry` (release,
  Linux): interpreter 1.6 s CPU, native 1.5 s plus 0.36 s compiling, so
  the machine code brings nothing to such programs (decision AG5 has
  the profile).
- **The tests.** `crates/renyi_vm/tests/native.rs` (six: overflow to a
  big Integer, a guarded Integer in a typed parameter, recursion past
  `DEPTH_LIMIT`, failures handled and unhandled, a crash's line, Floats
  and Booleans in registers and a stepped range; each run both ways
  and compared); `crates/renyi/tests/conformance.rs` runs every `run`
  case from its bytecode file on the interpreter and every case that
  runs a program a third time with `RENYI_NATIVE_HOT=1`; the three
  selfhost judges run the front end from its bytecode with the default
  tiering.
- **The development aids.** `RENYI_NATIVE_REPORT`, `RENYI_NATIVE_HOT`,
  `RENYI_NATIVE_OPT`, `RENYI_NATIVE_VERIFY` (decision AG5 describes
  them). Under `D:\Projects\.worktrees\Renyi\`: `selfhost/bench_hot.py`
  (the front end per hotness factor), `selfhost/bench_primes/`,
  `profile/micro.py` (the loop micro-benchmarks), `perf/` (the Linux
  build under WSL with `perf`: `build_linux.sh`, `profile_linux.sh`,
  `callers_linux.sh`, `native_vs_interp.sh` and their reports).
- **What bit.** Cranelift's verifier rejected a block never switched
  to (a fallthrough block made by `edge_to`); `stack_load` and
  `stack_store` take the pointer type first; `icmp_imm` and `bxor_imm`
  are deprecated for the `_s` forms; a heredoc un-escapes backslashes
  (again); the reserved words `sum`, `count`, `by`, `within` and `tags`
  and the named arguments of calls with two or more arguments in the
  test programs; `/` on two Integers is `integer-division`
  (`quotient`).

## The VM round (decision AG6)

- **What changed**, all in `crates/renyi_vm/src/vm.rs`, none of it
  visible to a program, the bytecode file or the compiler written in
  Renyi (the judges and the conformance suite pass unchanged): the
  `Op::Load` arm reads the field in the slot when the slot holds a
  record or a variant, the next op is `Op::Field` and the site's cache
  names the type (otherwise the load clones as before and `Op::Field`
  does the rest, including the guard wrapper and the cache miss);
  `Frame::grant` is an index into `Vm::grants` (the run's grant first),
  `frame_grant` returns an index, `Vm::narrows` says per code object
  whether its function narrows the grant at all, and
  `push_frame_in_place` fills the locals with one `resize`;
  `construct_variant` keeps a variant without fields in
  `Vm::unit_variants` (`unit_base` gives each type its slots).
- **Tried and dropped** (decision AG6 has the numbers): a store of a
  binding fused with the load that follows it; the fields of a record
  or a variant inline in its allocation (`smallvec`), which cost more
  instructions than the second allocation had.
- **How it was measured** (the owner's game loaded the CPU; wall and
  CPU time swung by a third between two runs, the instruction count did
  not): `perf stat -e instructions:u,cycles:u` on the release build
  under WSL, through the lane scripts of `D:\Projects\.worktrees\Renyi\perf\`:
  `build_linux.sh` (the release build with symbols into the lane),
  `perf_stat.sh <binary> <args>` (one command's counts), `measure_all.sh
  <binary>` (the self-check both ways and the five micro-benchmarks of
  `..\profile\`), `ab_stat.sh <a> <b> <rounds> <args>` (two binaries
  interleaved, the minimum of each count), `ab_linux.sh` (the same with
  user time), `chain_build5.sh`/`chain_build6.sh` (a build queued after
  the running one); the binaries of each step are kept there as
  `renyi_base`, `renyi_f1f4`, `renyi_f2`, `renyi_f5`, `renyi_f6` and
  `renyi_f7` (the final one); the last pushed commit is checked out in `..\base\` for a
  development-build twin. `..\profile\micro.py` reports user time
  (`GetProcessTimes`) and forces the interpreter unless
  `MICRO_MODE=native`; `loop_field.ry` and `loop_construct.ry` joined
  the micro-benchmarks there.
- **The numbers** are in decision AG6: 5.4% fewer instructions and
  4.1% fewer cycles on the compiler's self-check on the interpreter,
  from the fused field read (about 4%), the frame (1%) and the unit
  variants.

## The plan after the machine code (the owner's answers of 2026-10-07)

1. **A bounded round of VM work** (decision AG5; done, decision AG6,
   the section above): the fused field read, the grant by index, the
   unit variants; 5.4% fewer instructions and 4.1% fewer cycles on
   the compiler's self-check. The clone and drop of values,
   the stack traffic and the allocator remain, for the baseline JIT or
   `renyi build` after 0.1.
2. **Positioning** (decisions AH1 to AH4, recorded;
   `docs/design/08-positioning.md` written, the README's opening and
   its "what Renyi leads with" list derive from it): the niche is
   "the scripting language of AI agents": the language an agent writes
   and a person reviews at a glance, where the program declares what it
   may do, the runtime admits only that, and every run can be recorded,
   replayed and narrated; the target users run automation with Claude
   Code or Codex and will not run an agent's Python blind. Four selling
   points: effects as capabilities (declared, scoped, budgeted,
   enforced at the boundary, dependencies included); recorded,
   replayed and narrated runs; the tooling for agents (the project map,
   the MCP server, purpose as syntax, a fix on every error); package
   effect manifests computed by the tool, never widened silently. The
   English-like syntax is second: lead with the guarantees, the syntax
   is the means of review. No non-goals are written; the posture stays
   that of a general language. Still to do from it: the documentation
   site's front page (AI2) and the measurements of its section 5 on the
   starter pack (AI3).
3. **Release 0.1** (decisions AI1 to AI5): done on 2026-10-07. The
   engineering is the section "Release 0.1 engineering" below; the
   release itself is its last bullet and `docs/RELEASE.md` section 5.
   What remains of it: the announcement (the owner, with the texts of
   `08-positioning.md` section 6), the VS Code Marketplace publisher
   (optional: the `.vsix` is on the release page), the domain
   renyi-lang.org (AI4, when the owner wants it), and the three
   measurements of the positioning's section 5.
4. **Deferred** (decision AG5): both done in session 9: the baseline
   JIT that inlines the boxed operations (decisions AR1 to AR6: the
   self-check 15% fewer instructions on machine code than before the
   round, `bench/records.ry` 21% fewer) and `renyi build`, the image of
   bytecode and machine code and, with `--exe`, the self-contained
   executable of A1 (decisions AS1 to AS4).
5. **Foreign packages** (decisions AJ1 to AJ4, the owner's answers of
   2026-10-07, evening): after 0.1 and before the rest of M5, the
   registration API for Rust natives (the standard library's
   mechanism opened to extensions: a declaration file with the
   signature, the capability and the failures, the native built into
   the binary; half of M5's embedding API), then the typed Python
   bridge as the first extension package (one worker process per
   run, JSON messages, each call one recorded primitive) under the
   capability `python("<package>")`. The registration API exists
   (decisions AK1 to AK4, the section "The registration API as it
   exists" below), and so do the bridge and the binder (decisions AL1
   to AL4, AM1 and AM2, their sections below).

## The registration API as it exists (decisions AJ1, AK1 to AK4; session 8, 2026-10-07, late)

One commit, after the owner's four answers (AK1 to AK4, every
recommended option). The pieces:

- `crates/renyi_vm/src/extension.rs`: `Native` (module, name, the
  receiver as the checker spells it or its head, or none;
  `Native::function` and `Native::method` are `const fn`, so a table is
  a constant), `Extension` (name, version, the declaration files, the
  table), `Registry` (`standard()`, `add`, `with`, `extensions`,
  `extras` for the manifest, `library()` for the checker, `lookup` with
  the rule full spelling, then head, then bare, and `verify`, which
  reports every mismatch of the tables at once and the first problem of
  the files).
- The standard library is the first extension: `natives::standard()`
  joins the eleven per-module tables (`pub(crate) const NATIVES` in each
  file under `natives/`, written by a script from the old `lookup`
  functions; 200 entries) into one `Extension` behind a `OnceLock`;
  `natives::lookup` is gone and `Vm::new` asks `options.registry`. The
  two-way check passed on the standard library at its first real run.
- `renyi_check::Library` (`standard()`, `empty()`, `add`, `modules`,
  `world()`), `check_project_in(library, files, problems)` and
  `check_file_in`; the old names check against the standard library.
  `World::resolve_all` is public: a world of declarations alone needs
  it before its functions exist (without it the check passed
  vacuously, which the test caught).
- `renyi_index::index_files_in` and `tools_of_in`.
- `renyi_vm::Options::registry` (default `Registry::standard()`). The
  boundary's refusals are found by the shape of the declared failure
  types (`Vm::boundary_failure`: the variant named `PermissionDenied`,
  `HostNotAllowed`, `ProgramNotAllowed` or `OverBudget` with one field,
  the scope or, without one, the first argument), which replaced the
  match on the five module names and reports an extension's own types
  the same way.
- `Manifest::extensions` (`"extensions": ["demo 0.1.0"]`, left out when
  empty), rendered and parsed; `renyi reproduce` warns when the
  recording's list differs from the binary's.
- The crate `renyi` is a library and a binary: `src/lib.rs` is the old
  `main.rs` with `pub fn main_with(extensions: Vec<Extension>) ->
  ExitCode` (verifies when there are extensions, stores the registry in
  a `OnceLock`, then the 64 MB thread as before), `registry()` and
  `library()` for the commands, and `src/main.rs` is
  `renyi::main_with(Vec::new())`; every check, index, tools, run, test,
  reproduce and MCP path goes through them; `renyi version` prints one
  `extension name version` line per extension; the bin target has
  `doc = false` against the name collision.
- The test `crates/renyi_vm/tests/extension.rs`: the standard library
  verifies; the lookup rule; a three-native extension (`twice`, `shout`
  under `console`, `peek` under `filesystem.read` with its own
  `PeekError`) checks, runs, is recorded and is refused through
  `PeekError.PermissionDenied`; its native fails with `Missing`; the
  check names a declaration without a native, a native without a
  declaration, a capability outside the reference, a module declared
  twice and a file registered under another name.
- The documents: decisions AK1 to AK4; `docs/extensions.md`, the guide
  (the declaration file, the natives, the `Extension` value, the binary
  of three lines, what the boundary does, what an extension is trusted
  with); the reference (section 2: extension modules import by name;
  section 11: the refusal rule by shape; appendix B: the manifest,
  `reproduce`, `version`); `06-runtime-guarantees.md` and
  `07-system-design.md` on the manifest's `extensions` and the
  host-facing half of the embedding API; the front page and the site's
  navigation; the README's status; CLAUDE.md.

Gates: `cargo fmt`, `cargo clippy --all-targets` clean, `cargo test`
278 passed (270 before, the eight of the extension test new); the
corpus and `compiler/` untouched.

## The Python binder as it exists (decisions AM1 and AM2; session 8, 2026-10-07)

One commit after the bridge, from the owner's four answers (AM1, every
recommended option): `renyi bind --python <package> [--module <name>]
[--to <directory>]` writes the declaration file from the package.

- **The command.** `crates/renyi/src/bind.rs` parses `--python` next to
  the header form and dispatches to `bind/python.rs`; the writing of
  the file and of the manifest entry is shared (`write_module`,
  `ManifestEntry::{Foreign, Python}`: the `foreign` or the
  `python.modules` section, the entry printed as `"python":
  {"modules": {...}}` when there is no manifest), as are `renyi_name`
  (now with the prefix of a reserved word, `c_` or `py_`) and
  `declaration` (now with the trailing clauses given: `needs foreign`,
  or `or fails with PythonError` and `needs python("<package>")`). The
  module name defaults to the package name and must be a module name
  (`scipy.stats` is one; `Geometry` is not: name one with `--module`).
- **The inspection.** `bind/python_inspect.py`, embedded with
  `include_str!` and passed to the interpreter with `-c`, the package
  and the directory as its arguments: the directory first on
  `sys.path`, `sys.stdout` swapped for `sys.stderr` so that what the
  package prints on import cannot mix with the report, one JSON object
  on the standard output: the version, the file, the functions (name,
  parameters with kind, default and annotation tree, the return tree,
  the docstring's first line, the signature as `inspect` prints it),
  a function `skipped` with the reason, or `error`. The functions:
  the names of `__all__` that are routines when the module has it,
  else the public routines whose `__module__` is the module, in
  `vars()` order. An annotation is a tree: `{"class": "builtins.int"}`,
  `{"generic": "builtins.list", "args": [...]}`, `{"union": [...]}`,
  `{"none": true}`, `{"other": "<text>"}`; `inspect.signature(...,
  eval_str=True)` first, the unevaluated signature when that raises.
  The interpreter is the first of
  `renyi_vm::natives::python::interpreters(configured)` (the former
  `candidates`, now public: the manifest's, `RENYI_PYTHON`, the PATH)
  that gives a report line; an import that fails in it is the answer
  ("`python` cannot import `x`: ModuleNotFoundError: ...").
- **The mapping** (AM2, `Mapper` in `bind/python.rs`): the scalars,
  the list-like, set-like and map-like origins (`LISTS`, `SETS`,
  `MAPS`), `Optional[T]` and `T | None`, `Annotated`, a bare `list`,
  `set` or `dict` as the same of `JsonValue`; everything else
  `JsonValue` at that position with a note, the notes written as
  `# as JsonValue: \`x\` (Foo), the result (no annotation)` above the
  function; `import std.json exposing JsonValue` only when used. The
  parameters: the positional ones declared and required, `*args`,
  `**kwargs` and a keyword-only one with a default left out, a
  keyword-only one without a default skips the function (`# skipped:
  <signature> (a keyword-only parameter without a default: \`key\`)`),
  as does a function without a signature. A single-letter parameter is
  `argument_<n>`; the purpose is the docstring's first line as a
  sentence, else the signature in backticks, cut to the 100-column
  line.
- **The tests.** The unit tests of `bind/python.rs` (a hand-written
  report to the file and the entry, the mapping table, the purposes);
  `bind_writes_a_module_from_a_python_package_and_the_manifest_entry`
  in `tests/python.rs` through the binary: `geometry.py` without a
  manifest (the entry printed) and with one, `listed.py` with
  `__all__` and a re-export from `support.py` as `--module exports`,
  the manifest after both, a program `shapes.ry` run through the two
  generated modules, and the refusals (a package that does not import,
  a name that is not an import name, a package name that is not a
  module name, a header with `--python`).
- **The documents.** Decisions AM1 and AM2; the reference (section 11's
  Python bullet, appendix B's row); `docs/python.md` section 3 "The
  file written for you" (the sections after it renumbered); the
  library sketch's section 15; `README.md`; `CLAUDE.md`; the usage
  text of the binary.
- **What bit.** `renyi` had no dependency on `renyi_json` (added to its
  `Cargo.toml`); a Renyi call with two or more arguments names every
  argument and a construction names every field, so the test program
  is written that way; `otherwise fail` cannot start a line
  (`otherwise-line`), and `renyi check`'s diagnostics go to the
  standard output, so a test that shows the standard error on failure
  showed nothing; the scratch directory of a test was once locked on
  Windows right after a run that wrote `__pycache__` (the rerun
  passed; no worker lingered, `tasklist` showed none).

## The Python bridge as it exists (decisions AJ2, AJ3, AL1 to AL4; session 8, 2026-10-07)

One commit, after the owner's four answers (AL1 to AL4, every
recommended option); the pieces mirror the foreign function interface
(decision AF1) layer for layer.

- **The rule.** A Python module is a declaration file of the project
  (bodiless `public function`s) that the manifest's `python` section
  binds: `"python": {"interpreter": "python3", "modules": {"analysis":
  {"package": "analysis", "symbols": {"mean": "average"}}}}`;
  `interpreter`, `package` (the module's own name when left out) and
  `symbols` are optional, `Manifest::python` is a `PythonSection`
  (`manifest.rs`, with `symbols_of` and `symbols_json` now shared with
  the foreign section). The resolver tags the file
  (`Project::python_of_file` by its path, `Project::python_module` by
  its qualified name, `Project::tag` and `tagged` for a command's file;
  the `foreign` section wins when both name a module),
  `SourceFile.python` carries a `PythonBinding` (the `PythonModule` of
  `renyi_syntax`, the interpreter and the project root),
  `check_project_in` parses a tagged file with `parse_declarations` and
  declares it with `is_library = true`, `ModuleInfo.python` holds the
  binding.
- **The checker.** `crates/renyi_check/src/python.rs`: each function
  needs `python("<package>")` with the package of its binding and
  nothing else, fails with `PythonError` of `std.python` and nothing
  else, takes no type parameters (`python-signature`); its parameters
  are types that can `ToJson` and its result a type that can
  `FromJson`, or nothing (`python-type`): `World::carried` is the
  structural rule of `check.rs::has_ability` on declared types
  (derives, implementations, a subtype's base, lists and sets of
  carried items, maps with `Text` keys, `maybe`, the scalars),
  `World::json_ability` finds the prelude's abilities. `effects::TREE`
  has `python`, `takes_scope` too; `python` takes no budget.
  `library/std/python.ry` (and the crate's copy) declares `PythonError`:
  `Raised(exception, message)`, `NotCarried(detail)`,
  `Unavailable(detail)`, `PermissionDenied(package)`; the first field
  was `kind` at first, which the JSON encoding of a variant uses for
  its name (decision K10), so a replay decoded the recorded failure
  wrongly until it was renamed.
- **The bytecode.** `FunctionMeta.python: Option<Python>` (package,
  symbol, interpreter, root) filled in `compile/mod.rs` from the
  module's binding; `file.rs` writes it as `"python"` after
  `"foreign"` and `FORMAT` is 3; `compiler/bytecode.ry`
  (`PythonBinding`) and `compiler/emit.ry` (`python_binding`;
  `symbol_of` takes the list of renames, for both bindings) mirror
  it; `compiler/declare.ry` holds the port of `python.rs`
  (`check_bound_signature` dispatches to the foreign or the Python
  rules, so that `declare_function` stays under 60 lines; `carried`,
  `carried_shape`, `json_ability`), `project.ry` the manifest section
  (`python_section`, `python_modules`, `python_entry`), the tagging
  (`python_of_module`, `python_of_file`, `module_of_file` shared with
  the foreign tagging, `step_python`, `python_tagged`) and the
  fifteenth library module in `library_modules`.
- **The VM.** `natives/python.rs` with `python_worker.py` (embedded
  through `include_str!`, passed to the interpreter with `-c` and the
  project root as its argument): `Worker::start` tries the candidates
  of decision AL3 (`candidates`: the manifest's interpreter, else
  `RENYI_PYTHON`, else `python` then `python3` on Windows and `python3`
  then `python` elsewhere) and keeps the first that spawns and answers
  the greeting `{"ready": true, "version": ...}`; `Worker::call` writes
  one request line and reads one answer line, checking the call
  number; `run` encodes the arguments with `json::encode` by position,
  starts the worker at the first call (`Vm.python`), decodes the answer
  by the declared type with `json::decode` (a result the declaration
  does not mention is dropped) and maps the worker's `error` by its
  `where`: `call` to `Raised`, `result` to `NotCarried`, else
  `Unavailable`; a worker that stops answering is dropped (killed on
  `Drop`) and the next call starts another. The worker imports a
  module once and caches it, swaps `sys.stdout` for `sys.stderr` so
  that a package's prints cannot corrupt the protocol, and answers
  with `ensure_ascii=False, allow_nan=False`. `grant::effect_of` gives
  a `python` call the scope of its declared need; `Vm::run_primitive`
  dispatches a function with a `python` binding to the bridge.
- **The commands.** `renyi run` prints "renyi: this program can run
  Python through `analysis`" on the standard error when `main` grants
  `python` (`bound_modules` and `grants` of `lib.rs`, shared with the
  foreign notice); `renyi publish` refuses a project with Python
  modules; `diagnose` parses a tagged file as declarations.
- **The tests.** `crates/renyi/tests/python.rs` (six, a `helpers.py`
  beside the manifest): the run from the source and from the bytecode
  file, the notice and a Python print on the standard error; a
  recording replayed and reproduced with the interpreter renamed to
  nothing that starts; `--deny python`; an interpreter that is not
  there, through the manifest and through the variable; a worker ended
  by `sys.exit` and started again; `publish` refused.
  `crates/renyi_check/tests/rules.rs` has the capability and the
  signature rules, `manifest.rs` the section's round trip. The fixtures
  `tests/conformance/python/` (`analysis.py`, `analysis.ry`, `stats.ry`
  printing five lines; case 53) and `tests/conformance/python_bad/`
  (`bad.ry`, the two diagnostics; case 21) are under the three judges,
  the Renyi checker and compiler byte-equal on them; `tests/fixture.rs`
  holds their manifests canonical; `tests/library.rs` counts fifteen
  files; `tests/file.rs` and `tests/compile.rs` name the format numbers
  (3 read, 4 refused). The suite and the tests need a Python 3 on the
  PATH: CI's `setup-python` step moved before `cargo test`, and the
  release workflow has one before its suite.
- **The documents.** Decisions AL1 to AL4; the guide `docs/python.md`
  (the manifest, the declaration file and the types that cross, the
  program and the worker, what fails) in the site's navigation and on
  the front page; the reference (the module list, the manifest, the
  capability table, the scopes, the static rules, the run time,
  appendix A); the library sketch's section 15 (`std.python`; the two
  after it renumbered); the syntax sketch's capability list; the
  runtime and system documents; `extensions.md`; the cheat sheet
  (`python`, `std.python`; 2999 tokens after four words were trimmed);
  the starter skill; `README.md`; `CLAUDE.md`; the conformance
  `README.md`.
- **What bit.** `tags` and `count` are reserved words, as field names
  too, and a text literal cannot stand inside a hole (the first fixture
  tripped on all three); a purpose line is held to 100 columns like any
  line; `ability` is reserved as a parameter name; the `kind` field of
  `Raised` (above); the Renyi resolver's fixed list of library modules
  had to grow (without it the judges saw `std.python` as unknown and
  every bytecode file differed by the missing type);
  `declare_function` passed 60 lines with the second dispatch, hence
  `check_bound_signature`. After the gates, `cargo test` found what the
  cheat sheet's trim and the format number had left behind: the two
  copies of the cheat sheet (`crates/renyi/cheatsheet.md` under
  `tests/cheat_sheet.rs`, `starter/skill/renyi/cheatsheet.md` under
  CI's `cmp`) are copied again after every trim, `tests/library.rs`
  counts the library's files, and `tests/file.rs` refuses the next
  format by number; CI installed Python after `cargo test`, which the
  bridge's tests now need before it.

## Release 0.1 engineering (decisions AI1 to AI4; session 8, 2026-10-07)

Three commits after the positioning: 621fecc (the workflow, the
installers, the extension, the procedure), ee8b74b (the starter pack)
and the commit of the site and the crates.io metadata.

- **The release workflow** `.github/workflows/release.yml` runs on a
  tag `v<version>`: three native builds (`cargo build --release
  --locked` on ubuntu, macos and windows runners), each refusing a tag
  whose version is not the workspace version, the conformance suite
  run against each release binary, archives with `README.md` and
  `LICENSE` and a `.sha256` each, the VS Code extension packaged with
  `vsce` (node 22), then a GitHub Release with generated notes
  (`softprops/action-gh-release@v2`). `install.sh` and `install.ps1`
  download the archive for the machine, verify the checksum and put
  the binary in place (`RENYI_VERSION`, `RENYI_INSTALL_DIR`,
  `RENYI_REPO`). `editors/vscode/` is the extension: a TextMate
  grammar with dot-guarded keyword patterns, the language
  configuration, packaged locally once to check it. `docs/RELEASE.md`
  is the procedure: what a release is, release 0.1 step by step with
  the owner's steps marked, every release after it, where the pieces
  are.
- **The starter pack** `starter/` (decision AI3): `README.md` (install,
  the skill, the MCP server with its ten tools, the five workflows
  with the `needs` line of each `main`, how to review a program in one
  pass, record and reproduce); `skill/renyi/SKILL.md` (the loop, the
  rules the checker enforces with their codes, a module that checks
  clean, the commands, the library modules) with `cheatsheet.md` a
  copy of `docs/cheatsheet.md` that CI holds equal with `cmp`;
  `mcp.json`; `workflows/`: `repo_digest.ry` (git log by author,
  `std.process`), `api_digest.ry` (the GitHub API), `expense_report.ry`
  (CSV files to `out/report.md`), `log_triage.ry` (error codes
  ranked), `link_check.ry` (every link of a file fetched); their data
  under `data/`, the three recordings under `fixtures/` made with
  `renyi test --refresh` from live runs on 2026-10-07 (the recordings
  hold public data only: this repository's log, the GitHub record of
  rust-lang/rust, two pages of example.com and one of iana.org). CI
  checks, format-checks and tests them. Pitfalls met while writing
  them, for the next workflow: `count` is reserved (a parameter named
  `limit`), an `otherwise fail with` on its own line is `otherwise-line`
  (break inside the call's parentheses), `otherwise fail` on a `maybe`
  is `otherwise-fail-maybe` (`otherwise crash with` in a test), a test's
  head is one line that the formatter never breaks (shorten the test
  name), `purpose:` lines over 100 columns are shortened by hand.
- **The site** (decision AI2): `tools/site.py` renders every document
  under `docs/` but `HANDOFF.md`, the grammar, `examples/README.md`
  with each example's source and `starter/README.md` as pages with one
  header and relative links (46 pages; the `markdown` package;
  `python tools/site.py [_site]`); `docs/index.md` is the front page
  (the positioning's paragraph, install, the links);
  `.github/workflows/pages.yml` builds the site on every push to
  `main` and deploys it from a public repository or by hand
  (`workflow_dispatch`); on the private repository the deploy job is
  skipped, the build job still checks the script.
- **crates.io** (decisions AI2 and AI5): every path dependency carries
  its version through `[workspace.dependencies]` of `Cargo.toml` (cargo
  refuses to package without one). `renyi_check` embeds the standard
  library and `renyi` the cheat sheet; a crates.io tarball carries only
  the files under the crate, so each embeds a copy under its own
  directory (`crates/renyi_check/library/std/`,
  `crates/renyi/cheatsheet.md`), and a test in each crate holds the copy
  equal to the canonical file (`tests/library_copy.rs`,
  `tests/cheat_sheet.rs`); after editing a library declaration or the
  cheat sheet, copy the file again (the starter pack's cheat sheet
  too). `cargo package --workspace --allow-dirty` packages all seven
  crates and verifies the six libraries from their tarballs; on the
  binary it stops with a cargo internal error ("no hash listed for
  renyi_index v0.0.1", cargo 1.94.1), which comes from the temporary
  registry cargo builds for a workspace package and not from the
  crate: the extracted tarball `target/package/renyi-0.0.1/`, copied to
  the lane with its five dependencies pointed at the workspace crates,
  builds and runs (`renyi 0.0.1`). `cargo publish -p renyi` verifies
  against crates.io, where the libraries are by then. `cargo package` prints
  `--no-verify` in its usage, which the cc-enforcer hook refuses on the
  command line (it reads it as git's flag); use `--list` or the full
  verification instead, and never while another cargo command runs on
  the same target directory (a `cargo test` beside it failed to build).
- **Release 0.1.0, as it went** (2026-10-07, 20:00 to 23:30 UTC;
  `docs/RELEASE.md` section 5 has the lessons). The owner created the
  organisation and transferred the repository; the session renamed it
  to the lowercase `renyi` of AI4 (`gh repo rename`), registered the
  Pages site (`gh api -X POST repos/renyi-lang/renyi/pages -f
  build_type=workflow`; the organisation is on the Team plan), scanned
  the tracked files and the history for secrets and private paths
  (none), made the repository public (`gh repo edit --visibility
  public`), bumped the version to 0.1.0 (the fixture's `package.json`,
  the lockfile hash and two test strings followed; commit 868db78
  "Release 0.1.0"), published the crates with the login already on
  the machine (five at once, then crates.io's limit on new crates:
  one more every ten minutes, so `renyi_vm` at 22:46 and `renyi` at
  22:56 UTC), tagged `v0.1.0` on 868db78, and the release workflow
  built the three binaries, ran the conformance suite on each,
  packaged the extension and published the release page (about
  fifteen minutes; the Windows build the slowest). Checked after:
  the Site workflow deployed on the push of 868db78 and the pages
  answer; `install.sh` under WSL installed and ran 0.1.0 from the
  release; the Windows archive's checksum and binary were verified
  by hand; `cargo install renyi` built the binary from crates.io.

## The typed round (decision AU1; session 9, 2026-10-09), in progress

The owner asked, after the size round closed, for a cure of the
performance rather than another cut, and chose the route on
2026-10-09 (decision AU1, with the reasons and the profile's numbers):
the checker's static types brought to the code generator, as a typed
bytecode (format 5) that both emitters write. The root cause the
profile showed: the bytecode and the VM are dynamically typed under a
statically typed language, the baseline JIT recovers only the three
scalars, and the generated code is the interpreter unrolled (14% fewer
instructions than the interpreter on the self-check, 7% faster in
wall-clock; `primes` 9.7 times faster than CPython because it is all
Integers, `strings` slower than CPython because it is all boxed).

The profile that opened it (`valgrind --tool=callgrind` on the release
binary, `renyi run bench/strings.ry`, the JIT tier; the inclusive and
exclusive listings are in the decision): 1,149 million instructions,
966 per glyph over 1,188,895 glyphs; 43% the primitive boundary
(`rt_call`, `call_from_stack`, `call_primitive`, `run_primitive`,
`status`, `guarded`, `effect_of`, the drops of the arguments, the
scratch copy), 12% the search itself, 8% the iteration, 6% the retains
and clones of the constant and the glyph, 7% the generated code, the
rest the first half of the program (`rt_to_text` through `core::fmt`,
`rt_concat`, `append` through the boundary). Wall-clock on this
machine, best of five: the whole program 130 ms, its first half alone
33 (of which about 10 ms is the process and the compile), CPython 84.
The glyphs allocate nothing (the ASCII table of `Value::character`,
AQ), so inline small texts would save only the reference counts here.

The plan (AU1), each stage measured by AT1's rule with the benchmarks
against CPython beside it, a stage kept when the self-check's estimate
falls or a benchmark gains more than 5% without the self-check losing
more than 1%:

1. **The call to a primitive without the boundary.** A `Native` may
   carry a typed entry from a fixed family (`fn(&[Value]) ->
   Option<T>`, `T` one of `bool`, `i64`, `f64`, `Value`; `None` sends
   the call down the general path), called by the generated code with
   the arguments borrowed on the stack and the result in a register; a
   pure primitive without one runs on a flat path (`memory_check`,
   `plain_in_place`, the native, `guarded`, nothing else); an operand a
   typed call borrows is neither retained nor released when a `Load` or
   a `Const` pushed it. The interpreter takes the flat path too. The
   registration API gains the entry (an addition to AK2; the guide
   `docs/extensions.md`).
2. **The loop over a list in the generated code**: the list iterator
   in a fixed layout (`#[repr(C)]`: the items' address and count, the
   position, the list that keeps them alive), read and advanced by the
   generated code with a tag check and the helper as the fallback.
3. **The typed bytecode, format 5.** Both checkers record the type of
   every expression at its span (`Target::Typed`, the zonked type,
   skipped when a variable remains), as `Target::Result` and
   `Target::Number` already travel from `check.rs` and
   `compiler/bodies.ry` to `compile/` and `compiler/emit.ry`; both
   emitters write a type table per program (first use first) and per
   code object the type of what each op pushes (`emit` looks the span
   up, so no emission site changes); `file.rs`, `binary.rs`,
   `compiler/bytecode.ry`; the judges hold the two emitters equal; the
   VM checks the annotations against its inference of the three scalars
   on the corpus, the conformance suite and `compiler/` before it uses
   them.
4. **What the types buy**: the field read at a static index, the
   ability method chosen statically, the loop variable typed, the
   retains and releases elided wherever the consumer borrows.
5. **Later rounds**: the representation (small texts inline, `Int`
   flattened, lists of unboxed Integers, values in registers across
   ops).

Safety does not rest on the types: the generated code reads a tag
before every fast path and hands the frame to the interpreter when it
does not match (AR1), so a type is the choice of the fast path, never a
promise. The recordings, the replay, the narration and the `.ryc` as
JSON stay as they are.

**Stage 1 is in (decision AU2, 2026-10-09).** `Typed` and
`Native::with_typed` in `extension.rs` (`Registry::lookup_entry`,
`verify` holds an entry to the declared result's kind and to a function
without `needs`); the `plain_*` helpers in `natives/mod.rs`; forty-two
typed entries in `natives/prelude.rs` (the `_typed` functions beside the
natives); `Vm::call_scratch`, `call_pure_from_stack`,
`call_typed_in_place`, the tables `typed` and `pure`, `CallKind` and
`call_kinds` in `vm.rs`; `rt_call_pure`, `rt_call_typed` and the status
`BOXED` in `native/runtime.rs`; `borrowed_operands`, `kind_agrees`,
`Gen::call_typed`, the `calls`, `borrowed` and `masks` of `Gen` in
`native/codegen.rs`; `Jit::new` and `image::build` take the call kinds
(the latter the registry); `Pinned::forget_from`. Tests:
`tests/extension.rs` (a typed entry asked on both tiers, answering and
declining; `verify` on a wrong kind and on a function with `needs`),
`tests/native.rs` (`typed_entries_of_the_prelude_answer_or_decline_on_both_tiers`,
the decline of `absolute` on the smallest Integer and the hand-back of
its big answer). The guide `docs/extensions.md` has the entry. Measured:
the self-check's estimate on the JIT run 14.83 to 13.86 billion cycles
(-6.5%), the interpreter 17.92 to 17.23 (-3.9%; 12.85 to 12.21 billion instructions), the image 11.18 to
10.27 (-8.2%; 7.97 to 7.16 billion instructions, -10.2%; the machine code 5.62 to 5.63 MB, 122 bytes of body per op as before); `strings` 134 to 94 ms (CPython 81); the profile after the
stage in AU2: `rt_call_typed` itself 111 instructions a call, the
iteration 82 a glyph, the loop variable's retain and release 54, the
first half of the program a fifth of the whole (`core::fmt` for every
Integer written into a text, `append` freeing and allocating its `Rc`
each call).

**Stage 1b is in (decision AU3, 2026-10-09)**: the small cuts the
profile named. `rt_call_typed` syncs the pc only on the general path
and the error paths and drops nothing when every argument was borrowed
(`consume_arguments`); `integer::push_digits` and `digits` write a
machine-word Integer without `core::fmt` (used by `render` and
`op_concat`); `append`, `append_all`, `set`, `add` and `without` on
lists, maps and sets go through `Rc::make_mut` and keep the allocation
(`take_list` and `take_set` are gone from the prelude). Measured:
strings 94 to 84 ms (87 by `tools/bench.py`, CPython 79; 77 as an
image, past CPython), the self-check's estimate within half a percent
on every tier (13.86 to 13.83 billion on the JIT run); the profile 733
to 669 million instructions, 563 a glyph: the iteration 82, the search
72, the typed call's helper 80, the generated code 56, the entry's match
20, the loop variable's retain and release about 40, the rest the first
half of the program.

**Stage 2 is in (decision AU4, 2026-10-09)**: the loop over a list in
the generated code. `value.rs`: `Native` is `repr(C, u8)`,
`Native::Iterator(ListIter)` with `ListIter { items, len, position,
list }` and `ListIter::next`; `layout::TAG_NATIVE`, `NATIVE_TAG`,
`NATIVE_PAYLOAD`, `NATIVE_ITERATOR`, `ITER_ITEMS`, `ITER_LEN`,
`ITER_POSITION`, held to the types by
`a_list_iterator_lies_where_the_generated_code_reads_it`;
`native/codegen.rs`: `Op::IterNext` on an `Iter` slot reads the slot's
tag and the native's, walks a list iterator in place and falls back to
`rt_iter_next` for anything else; `runtime.rs`: `iterator_of` and
`iterator_next` through `ListIter`. Test:
`loops_over_lists_texts_sets_and_ranges_agree_on_both_tiers` in
`tests/native.rs`. Measured: the self-check's estimate on the JIT run
13.83 to 13.66 billion (-1.3%), the interpreter +0.4%, the image 10.22
to 10.08 (-1.4%; the machine code 5.63 to 5.70 MB, 123 bytes of body per op, the inline walk's price); strings 87 to 78 ms by `tools/bench.py` against CPython's
79 (at par; 76 as an image), 84 to 83 best of seven; the profile 669 to
625 million instructions, 525 a glyph.

**Stage 2b is in (decision AU5, 2026-10-09)**: `rt_call_typed` is the
fast path alone (`typed_call_general`, `typed_value_answer`,
`typed_over_memory` beside it; `Vm::memory_is_over`), `text_has` in the
prelude (a one-byte part in a text of at most 32 bytes by a plain loop,
for `contains` and its typed entry), `push_digits` pushes characters.
Measured: strings 83 to 78 ms (77 by `tools/bench.py`, CPython 79 to
82), the profile 625 to 589 million instructions; the self-check's
estimate on the JIT run +0.6% (the code's layout: instruction misses up,
instructions level), kept by the rule's second clause.

**Stage 3 is in (decision AU6, 2026-10-09)**: the typed bytecode,
format 5 (binary encoding 2, image format 4). `check.rs`: `infer` wraps
`infer_inner` and notes every expression's type (`typed`), `finish_body`
records `Target::Typed`; `compiler/bodies.ry`: `infer`/`infer_inner`,
`record_typed`, `TypedTarget`. `compile/mod.rs`: `emit` writes
`Code::types[at]` from `type_index_at` (the table `result_types`,
`type_index`), `emit_untyped` for the literal pattern's comparison
(`pattern.rs`); `compiler/emit.ry`: the same (`type_index_at`,
`type_index`, `typed_target`, `emit_untyped`, `types` on `Emitter` and
`Code`). `bytecode.rs`: `Code::types`, `Op::pushes_its_expression`;
`file.rs`, `binary.rs`: the field written, read and checked;
`native/infer.rs`: `abs_of_type` sees through a refined subtype;
`renyi_index/src/edges.rs` and `lsp/describe.rs` ignore the new target.
Tests: `crates/renyi_vm/tests/typed.rs` (the annotations against the
inference over the corpus, the clean conformance programs and the
compiler; more than half of the compiler's ops typed), the judges of
`selfhost.rs` hold both emitters equal on the new field (in 117 s),
`tests/compile.rs` and `tests/file.rs` on format 5. Measured: the
compiler's checker as a `.ryc` 8.99 to 9.67 MB; 65% of its 46,272 ops
typed, 425 types in the table; the self-check's estimate rose with the Renyi checker's own new work, see the decision. The VM
reads the types nowhere yet: stage 4 is what they buy (the field read at
a static index when the holder's type is known, the ability method
chosen statically, the loop variable typed from the list's item type,
the retains and releases elided where the consumer borrows, `Binary`
comparisons first).

**Stage 3b is in (decision AU7, 2026-10-09)**: the notes recorded
cheaply. `compiler/bodies.ry`: `infer` keeps a body's typed notes in
chunks of 64 (`typed`, `typed_chunks`, the binding `rotated`),
`record_typed` appends to `typed_references`, one list per body, which
`all_references` joins into the module's references once;
`compile/mod.rs`: `type_index` through `Context::type_indices`, a map.
Measured: the self-check's estimate on the JIT run 16.67 to 14.58
billion (6.0% above AU5's 13.75: the Renyi checker's own remaining cost
of the notes, see the decision), the interpreter 20.20 to 18.04, the
image 12.75 to 10.66; the judges in 102 s (117 at AU6). Noted for a
later round: `record with field: record.field.append(x)` copies the
list on every call throughout the compiler; an emitter that recognised
the update of a field of a uniquely held record would make the compiler
faster by a large factor on its hottest paths.

## The size of the generated code (decisions AT1 to AT7; session 9, 2026-10-08 and 09)

The owner's four answers of 2026-10-08 after `renyi build` closed
(decision AT1): the round's rule is KCachegrind's estimate of the
cycles over cachegrind's counts on the self-check (`Ir + 10 (I1m + D1m)
+ 10 Bm + 20 Bi + 100 LLm`), deterministic, with the wall-clock at the
end of each stage; the cuts that cost no instruction first; the image's
bytecode in a binary encoding (the `.ryc` stays JSON); the record gains
a tag word so that a record and a variant share the prefix the field
read reads.

- **The tools.** `tools/measure_size.sh <binary>` runs cachegrind with
  `--cache-sim=yes --branch-sim=yes` on the self-check three times (the
  JIT run, `--interpret`, the image built under `valgrind --tool=none`
  since valgrind hides CPU features) and prints the counts and the
  estimate, then the census; about ten minutes a binary.
  `tools/image_census.py <file.ryi> [--summary]` reads an image and
  prints the bytes of machine code per code object and per op, the op
  kinds, and (with numpy) a least-squares attribution of the bytes to the
  op kinds over the code objects. The census that opened the round, on
  the compiler's image of AS4: 17.5 MB, of which 9.0 MB the bytecode as
  JSON and 8.3 MB the machine code (955 code objects, 46,193 ops, 179
  bytes of body per op, 8.7 KB per code object; the trampolines 151 KB);
  the attribution (r² 0.98): `LoadField` 462 bytes each (22% of the
  bodies), `Load` 173 (21%), `Call` 440 (21%), `Store` 171 (11%),
  `Const` 148 (7%), `Return` 223 and `ReturnNothing` 363 (10%), a deopt
  point 384 (4%). Loading the compiler's image costs 152 ms against
  172 ms for compiling it from source (`renyi run <file> /nonexistent`,
  which fails before `main`), almost all of it the parse of the 9 MB of
  JSON (`renyi run selfcheck.ryc` costs 160 ms), which is why the binary
  encoding of stage 2 belongs to the round; since AT3 and AT5 the image
  of the compiler loads in 16 ms, its code section mapped from the file
  (most of the 69 ms measured after AT3 was the checker loading the
  standard library before failing on the missing file, the program's
  work).
- **The baseline** (the binary of AS4, 71965aa):

  | run | instructions | I1 misses | D1 misses | LL misses | mispredicts (cond, ind) | estimated cycles |
  |-----|-------------:|----------:|----------:|----------:|------------------------:|-----------------:|
  | JIT run | 11,181,306,636 | 116,596,885 | 75,149,511 | 2,173,057 | 35,445,492, 85,031,817 | 15,371,167,556 |
  | `--interpret` | 12,956,694,773 | 11,959,382 | 64,644,504 | 1,981,283 | 38,216,065, 185,330,643 | 18,009,635,443 |
  | image (`speed`) | 9,070,866,423 | 160,217,080 | 53,100,744 | 3,388,570 | 26,140,638, 41,528,543 | 12,634,878,903 |

  The compiler's image: 17,455,597 bytes, 8,261,937 of machine code in
  the bodies (178.9 per op), 825 deopt points.
- **Stage 1, the cuts at no instruction cost (decision AT2).**
  `codegen.rs`: the prologue writes `Nothing` to every local that is not
  a boxed parameter (once per callee; `push_frame_inline` wrote them at
  every call site), the `LoadField` fast path reads a record and a
  variant on one path (`RECORD_TAG`, the shared prefix, the holder test
  `tag - TAG_RECORD < 2`, no range check), `deopt_block` and
  `emit_deopts` share one block per pc and operand state (`deopt_unless`
  takes it; `deopt_blocks` on `Gen`), `call_direct` goes to the slow
  path when the table has no body (the ask path and `rt_direct_entry`
  are gone, with `Vm::direct_entry_of` and `Jit::direct`), and the call
  counter is gone (`NativeState` has `depth`, `direct_table`, `helpers`,
  `constants`; `Jit::report` takes the hotness alone and says how many
  calls went to the interpreter). `value.rs`: `Record { ty, tag, fields
  }` with `tag` always `usize::MAX`, `RECORD_TAG`, and the three
  compile-time assertions of the shared prefix. `CODE_FORMAT` 2.
  Measured:

  | run | instructions | I1 misses | D1 misses | LL misses | mispredicts (cond, ind) | estimated cycles |
  |-----|-------------:|----------:|----------:|----------:|------------------------:|-----------------:|
  | JIT run | 11,019,336,948 | 106,417,090 | 74,824,738 | 2,281,534 | 35,221,233, 85,881,206 | 15,129,745,078 (-1.6%) |
  | `--interpret` | 12,847,784,410 | 13,096,267 | 65,085,528 | 2,139,424 | 39,819,316, 185,723,809 | 17,956,214,100 (-0.3%) |
  | image (`speed`) | 8,876,981,034 | 155,552,936 | 53,227,555 | 3,348,443 | 28,440,844, 41,641,494 | 12,416,868,564 (-1.7%) |

  The compiler's image: 17,019,001 bytes, 7,829,707 of machine code in
  the bodies (-5.2%; 169.5 per op), 709 deopt points. Kept by the rule.
- **Stage 2, the image's program in binary (decision AT3).**
  `crates/renyi_vm/src/binary.rs` (new): `encode(&Program) -> Vec<u8>`
  and `decode(&[u8]) -> Result<Program, String>`, the content and order
  of the bytecode file in bytes (LEB128 integers, length-prefixed
  texts, one byte for an option or a variant, the maps in key order;
  `FORMAT` 1 first; a count past the end is refused before anything is
  allocated; `file::check` runs on the result as on a loaded file);
  `image.rs`: `Image { header, code_hash, program, codes }`,
  `IMAGE_FORMAT` 2, `build` stores `sha256_of(file::render(program))`
  and `binary::encode(program)`; `lib.rs` (the binary): `Hashed::Given`
  for the image's hash (`code_hash`, `compile_with_sources`,
  `run_embedded`), `load_image_bytes` decodes the program;
  `tools/image_census.py` takes the `.ryc` beside the image for the
  ops (`tools/measure_size.sh` compiles it); `tests/build.rs`: the
  compiler's program round-trips through the encoding and renders as
  its bytecode file, an image's hash is the bytecode file's, the
  decoded program renders the same, and a recording made from the
  `.ryc` reproduces against the image and the other way round.
  Measured: the compiler's image 17,019,001 to
  8,705,877 bytes (the program 8,989,252 of JSON to 676,056), its load
  152 to 69 ms against 166 ms for compiling the compiler from source
  (`renyi run <file> /nonexistent`); the rule on the self-check as an
  image: 8,126,378,865 instructions, 151,181,468 I1 misses, 50,981,874
  D1 misses, 2,204,948 LL misses, 21,727,334 and 39,288,677
  mispredicts, 11,371,553,965 estimated cycles (-8.4% against stage 1);
  the JIT run 15,147,420,418 (+0.1%, the binary's own layout: the
  generated code is untouched) and the interpreter 17,897,196,056
  (-0.3%); `hello` as an image 42 KB and 7 ms (235 KB and 11 before,
  8 ms from the source); the self-check in wall-clock 1448 ms as an
  image against 1723 on the JIT run (best of five). Kept: the stage
  targets the image, whose estimate fell.
- **Stage 3a, the static stack height and the frame pointer (decision
  AT4).** `codegen.rs`: `height()` is the frame's part of the stack by
  the state (`locals + boxed_depth(state)`), `length_at(height)` and
  `store_height(height)` compute and store the VM's length from `base`,
  `frame_var` holds the stack's pointer plus `base24` (defined by
  `reload_frame` once after the prologue's room check and after every
  helper call in `Gen::call` and after the direct call's `call_indirect`),
  and `address_at(height)`, `top_address()` and `last_address()` address
  the slots and the operands from it; `stack_len`, `set_stack_len`,
  `stack_ptr` and `slot_address` are gone, `item_address` remains for a
  record's fields; `push_typed_value` and `push_nothing_at` take the
  height (the state's for a push, `0` for a result left where the frame
  was); the frame push computes the callee's base as `height - boxed`.
  The one subtlety: after `box_top(1); state.pop()` the value lies at
  `height()`, not `height() - 1`, which the boxed return, the typed
  return of a boxed value and `IsNothing`/`IsFailure` say explicitly.
  Measured: the rule on the self-check, JIT run:
  10,867,362,765 instructions, 102,196,693 I1 misses, 74,887,436 D1
  misses, 2,355,761 LL misses, 34,816,885 and 85,118,789 mispredicts,
  14,924,324,785 estimated cycles (-1.5% against stage 2); the image:
  7,994,955,436 instructions, 142,581,697 I1 misses, 11,145,328,756
  estimated cycles (-2.0%); the interpreter 17,902,234,378 (unchanged);
  the machine code 7,829,707 to 7,909,162 bytes (+1.0%: the frame
  pointer's reloads and block parameters, against fewer instructions and
  misses); the self-check 1678 against 1780 ms in wall-clock, back to
  back on a busy machine. Kept by the rule. A step not taken: a flag per
  helper for whether it can grow the stack (most can, through the user
  code they run), so that `Gen::call` skips the reload after `rt_drop_at`
  and the few others that cannot.
- **The load (decision AT5).** The profile (`strace -T`, the JIT's own
  `placing` clock, a Python experiment on fresh against touched buffers):
  the system calls of a load take 4 ms; 44 ms went to reading the 8.7 MB
  file into a fresh buffer and 49 ms to copying the 7.8 MB of code into
  the arena's fresh pages, first-touch page faults that this Firecracker
  VM makes heavy (about 6 µs a page) and every machine pays in
  proportion. The fix: `image.rs` lays the bodies and trampolines
  sixteen-aligned in one code section at an offset that is a multiple of
  `SECTION_ALIGN` (16 KB), the table before it holding each
  `Placement { offset, len }`; `Image::open` and `Image::open_at` map
  the file read-only (`memmap2`), parse the table and the program from
  the mapping, and leave the section in the file (`MappedSection { file,
  offset }`) when its offset is a multiple of the page size, else copy
  it; `CodeArena::map_section` maps it executable (`Pages::Mapped`
  beside `Pages::Owned`), and `Jit::load_image` falls back to reading
  and placing the section when the system refuses (a `noexec` mount);
  `Image::read` (bytes in memory) copies the section as before, for the
  tests and for a file that cannot be mapped. `exe.rs` pads the
  executable to the alignment before the image, so that the embedded
  image's section maps from the executable's own file; the report says
  "the section mapped from the file" or "copied", which `tests/build.rs`
  asserts for a `.ryi` run and for the executable. `IMAGE_FORMAT` 3;
  `tools/image_census.py` reads the table and the section's offset.
  Measured: a run that exits at once (`renyi run <image>`
  without arguments) 16 ms with the image mapped, 27 with it copied and
  the check as it was, 105 from the source, 86 from the `.ryc`; with the
  library loaded 46 against 167 ms; `hello` 6 against 8 ms; the load's
  instructions 62 to 32 million (`file::check` 32 to 1 million: it
  formatted a message per op before knowing the check's outcome, built
  on failure now; the decode about 10 million; `Vm::new` and the
  registry under 2); the compiler's image 8.8 MB, its executable 26.6
  MB. The profile's tool: `valgrind --tool=callgrind` on `renyi run
  <image>` with no arguments, then `callgrind_annotate --inclusive=yes`;
  with a file argument the checker loads the standard library first,
  which is the program's work, not the load's. A note on the gates run
  in a worktree under the scratchpad's long path: `a_run_is_narrated`
  (`crates/renyi_vm/tests/recording.rs`) fails there because the
  narration truncates long arguments, and passes from the main tree;
  it is the path, not the code.
- **Stage 3b, the reference counts as calls (decision AT6).** Three
  variants on the binary of AT4, by the rule: (1) the frame pointer not
  reloaded after a helper that cannot move the stack (`rt_drop_at`, the
  grant, the frame growth, the typed takes and resumes, the tag tests):
  the pointer then lives across those calls and Cranelift saves and
  restores it around each, so the code grew 0.7% and the JIT run's
  estimate 1.0%, rejected; (2) on top of it `retain` and `release` as
  calls to `rt_retain_at` (a clone's count kept) and `rt_drop_at` (the
  value dropped whatever its count) in place of the inline sequences of
  AR4: the machine code 7.91 to 5.62 MB (-29%), the JIT run's estimate
  -0.7% (instructions -1.7%, instruction misses -15%, indirect
  mispredicts +16%), the image's +0.3% (misses -32%, indirect
  mispredicts +75%, the model's charge for the calls); (3) the calls
  without (1): the JIT run 14.85 billion and the
  image 11.21, a fifth of a percent behind (2), which stays by the rule
  (the code equal, 5.62 MB). In wall-clock against the binary of AT3,
  best of five: the self-check 1886 to 1775 ms on the JIT run, 1369 to
  1351 as an image, the interpreter, records and primes level. `codegen.rs`: `retain` and `release` are
  the two calls, `rc_of` and the inline bodies are gone, the helpers
  `rt_retain_at` and `rt_drop_at` (`runtime.rs`), `CODE_FORMAT` 3.
- **The round's close (decision AT7).** The owner closed it on
  2026-10-09 with every stage of the plan measured and in: the cuts at
  no cost (AT2), the image's program in binary (AT3), the static stack
  height (AT4), the image's load (AT5) and the reference counts as
  calls (AT6). Against the binary the round began from (71965aa): the compiler's machine code 8.26 to 5.62 MB
  (-32%), its image 17.5 to 6.5 MB, the self-check's estimate 15.37 to
  14.83 billion cycles on the JIT run (-3.5%) and 12.63 to 11.18 as an
  image (-11.5%), the image's load 152 to 16 ms, the self-check in
  wall-clock 1841 to 1775 ms on the JIT run and 1511 to 1351 as an
  image. What is left for a later round: `RELEASES_INLINE` (four calls
  against one `rt_truncate` on a return) is unmeasured since the calls
  replaced the sequences; the indirect call through the helper table is
  what the model now charges most (99 million mispredicts on the JIT
  run, a tenth of the estimate), which a direct `call` to a helper
  would remove at the price of a relocation per site, against AS1's
  address-free code; and `Int` flattened into `Value` would make a
  clone or a drop one compare, which the owner did not pick.

## `renyi build` as it exists (decisions AS1 to AS4; session 9, 2026-10-08)

The owner's four answers of 2026-10-08 after the baseline JIT's round
closed (decision AS1): an image file `.ryi` (the bytecode with the
machine code of every code object), later `build --exe`; the generated
code without addresses, so that the image is a plain block of bytes;
every code object compiled, the optimisation level measured; a
mismatched image refused with the fix.

- **Stage A, the address-free code (decision AS2).** `vm.rs` has
  `NativeState` (`repr(C)`: `depth`, `direct_table`, `helpers`,
  `constants`; `calls` went in decision AT2), a field of the VM the
  generated code reaches
  through the VM pointer (`codegen.rs`: `native_state_offset`, the
  `STATE_*` offsets, `Gen::native_state`, `Gen::call` through the
  helper table with `helper_index` into `SIGNATURES` and a `SigRef`
  per helper, the constants of a code object through the table, the
  trampoline's call of the body through the table of bodies);
  `codegen::compile` returns `Compiled { body, trampoline, deopts,
  headers, stats }` as bytes from Cranelift's `Context::compile`
  (`machine_code`: a relocation refuses the function; none occurs), and
  `mod.rs` places them (`CodeArena::place`, `place_many`: `region` pages
  written, flushed with the icache crate `cranelift-jit` used, made
  executable once). `cranelift-jit` and `cranelift-module` are gone;
  `Jit::new(program, opt_level)`, `Jit::isa`, `Jit::host_target`,
  `compile_code`, `compile_everything`, `load_image`, `state_pointers`.
  Measured against AR5's binary: the self-check 11.13 to 11.18 billion
  instructions on machine code (+0.46%), records 451 to 452 million,
  the typed call per turn 130 to 135: the price of the image.
- **Stage B, the image (decision AS3).** `native/image.rs`: `MAGIC`,
  `IMAGE_FORMAT` (the file), `CODE_FORMAT` (what the code assumes of the
  VM: bump it with any change to the layout, the helpers, the statuses),
  `Header { renyi, code_format, target, opt_level }` with
  `Header::mismatch` (the message with the fix), `Image { header,
  code_hash, program, codes }` with `write` and `read` (little-endian,
  lengths first; a unit test round-trips one; the program in the binary
  encoding of `binary.rs` with its bytecode file's hash since decision
  AT3, the `.ryc` text before), `ImageCode { body, trampoline,
  headers, deopts }`, `build(program, opt_level)`, `target_of(isa)` (the
  triple and every ISA flag), `is_image`, `EXTENSION`. `Options.image`
  (`vm.rs`): `Vm::new` loads it into the JIT after creating it (a load
  that fails says so on stderr and compiles as before). The binary
  (`lib.rs`): `build_command` (`--to`, `--opt none|speed`; a `.ry` or a
  `.ryc`; the message counts the code objects and the bytes),
  `load_image_file` (read, `Header::mismatch` against
  `Jit::host_target`, the program from the embedded bytecode),
  `compile_with_sources` returns the image beside the program and the
  hash (`Hashed::Given`, the bytecode file's, stored in the image since
  AT3), and `run`, `record`, `test`
  and `reproduce` pass it to the options; `tests/build.rs` (an image
  runs, records and reproduces as the source does and compiles nothing;
  tests run from one and find their fixtures; a mismatched target, a
  truncated file and a text are refused naming the fix; `build` of an
  image and a bad `--opt` are refused; both levels run). Measured on the
  self-check built as an image: the self-check 11.18 billion
  instructions on the JIT run to 9.23 as an image at `none` (-17.5%) and
  9.07 at `speed` (-18.9%; no compilation at run time, no cold code on
  the interpreter), `bench/records.ry` 452 million to 423 and 418; the
  compiler's image is 17.6 MB (955 code objects, the checker with every
  module it imports) and builds in 3.7 s at `none` and 4.4 s at `speed`;
  `speed` is the default by AR1's rule, compile time being the build's
  (decision AS3). The cachegrind runs use images built under valgrind,
  which hides AVX-512 from the CPU-feature detection, so a native image
  is refused there by the header check, as designed.
- **Stage C, the executable (decision AS4).** `crates/renyi/src/exe.rs`:
  `TRAILER_MAGIC` (`RENYIEXE`), `TRAILER_LEN` (24 bytes: the image's
  offset, its length, the magic; little-endian), `embedded_image` (the
  binary's own file opened, its last 24 bytes read, the image read when
  the trailer is there and consistent with the file's length, `None`
  otherwise: every command of `renyi` pays one open and one small read)
  and `write` (this binary's bytes, the image, the trailer; executable
  on Unix; on macOS `codesign --force -s -` is run and the command is
  the message when it is not at hand). `main_with` (`lib.rs`) asks
  `embedded_image` before dispatching and runs an embedded image through
  `run_embedded`: the program from `load_image_bytes` (the mismatch
  check as for a file, the executable's name in the messages), the
  whole command line as the arguments, the default flags, through
  `run_loaded`, the tail of `run_command` split out (the visibility
  notices, the sandbox refusal, the replay, the manifest, the run, the
  recording, the exit status). `build --exe` (`build_command`): the
  default name is the program's stem with the platform's suffix
  (`<name>.exe` on Windows); the message says "a self-contained
  executable" with the counts. `tests/build.rs`: the executable runs the
  program with its command line (`greet build` prints `Hello, build!`:
  no command of `renyi` is read from it), compiles nothing (the report
  under `RENYI_NATIVE_REPORT`), starts with this binary's bytes and ends
  with the magic; the default name. The compiler's executable is 35 MB:
  the binary (17.7 MB) and its image (17.5 MB).
- **Measured in wall-clock** (the release build on the quiet machine,
  the best of five runs in milliseconds, the median in brackets; the
  images built by the same binary at both levels):

  | program | JIT run | `--interpret` | image `none` | image `speed` |
  |---------|---------|---------------|--------------|---------------|
  | the self-check (`compiler/checker.ry` on `bodies.ry`) | 1841 (1974) | 1950 (2032) | 1691 (1736) | 1511 (1583) |
  | `bench/records.ry` | 62 (76) | 84 (94) | 54 (56) | 54 (57) |
  | `bench/primes.ry` | 42 (52) | 1088 (1156) | 36 (44) | 35 (37) |
  | `examples/hello.ry` | 10 (10) | 8 (9) | 11 (12) | 11 (12) |

  The image at `speed` runs the self-check 18% faster than the JIT run
  (the compile time, the warm-up and the cold code gone), records 13%,
  primes 17%; the two levels tie on the small programs, and a program
  as short as `hello` pays a millisecond for loading its 235 KB of code
  (thirteen code objects of 18 KB each) and gains nothing: an image is
  for a program that runs long enough to compile. The executable of
  `--exe` runs as its image does. The cache simulation (cachegrind with
  `--cache-sim=yes --branch-sim=yes` on images built under valgrind):
  `none` 9.23 billion instructions and 167.6 million first-level
  instruction misses, `speed` 9.07 and 158.0, `speed_and_size` 9.07 and
  158.0 with an image nine bytes larger, so the level does not shrink
  the code. To measure again: `renyi build --opt <level> --to x.ryi
  compiler/checker.ry`, then time `renyi run x.ryi compiler/bodies.ry`
  against `renyi run compiler/checker.ry compiler/bodies.ry`; `tools/bench.py`
  prints the image column for the benchmark programs.
- **What an image and an executable are not yet.** Not portable: the
  target string holds every CPU feature Cranelift detected, so an image
  moves only between machines with the same features (a build on an
  older CPU would run on a newer one, and is refused anyway: equality is
  the rule for now); the executable refuses a mismatch at startup with
  the same message. Not signed or hashed: the run manifest's code hash
  is the bytecode's, the machine code is trusted as the file is. The
  executable is not smaller than the toolchain (the whole `renyi`
  binary comes first, its commands unreachable rather than removed) and
  not compressed; a `.ryi` already built is not an input of `build
  --exe` (it builds from the sources or the bytecode, as `build` does);
  on macOS the copy needs the ad hoc signature the build applies when
  `codesign` is at hand. The image of the compiler is large (17.6 MB for
  955 code objects, 18 KB a code object; its size is what the
  instruction-cache misses of the section below point at), the inline
  sequences of AR4 being what they are; Cranelift's `speed_and_size`
  level does not shrink it (the same code as `speed` within nine bytes
  and the same cache misses on the self-check), so the size is a
  question for the sequences `codegen.rs` emits.

## The baseline JIT (decisions AR1 to AR6; session 9)

The owner's four answers of 2026-10-08 (decision AR1): the baseline JIT
of AG5 (iii) in three stages, each measured on the compiler's
self-check by its instruction count (the rule of AG6: a step stays when
it cuts the count, the round ends when a step brings under 2% or the
self-check is twice as fast as AG6 left it), with `bench/records.ry`
and four micro-benchmarks beside it; fused operations in the bytecode
itself; direct calls between generated functions with register
arguments; the layout of `Value` pinned and the value operations inline.

- **Why.** Micro-benchmarks of the two tiers (instructions per turn of a
  loop, `cachegrind`, the release build of commit 2fcab99; machine code
  against the interpreter): a call with an Integer parameter 668 against
  1133; a field of a local 1043 against 818; a call with a record
  parameter that reads a field of it 1535 against 1183; a record built
  and a field of it read 1357 against 1480. The generated code calls a
  helper per boxed op on the VM's stack, so on the operations ordinary
  programs are made of it is the interpreter less the dispatch plus the
  helper calls, and it lost the field read in the slot that AG6 gave the
  interpreter. The four programs are twenty lines each: a loop of
  `rounds` turns (the first argument, `1000000` by default) around one
  statement (`change total to total + add_one(index)`; `change total to
  total + point.east + index` on a `Point` in a local; `change total to
  total + first_of(point) + index` with `first_of(point: Point) returns
  Integer` reading `point.east`; `let point be Point(east: index,
  north: index)` then `change total to total + point.east`), run at two
  sizes and the counts' difference divided by the size's; they are
  `bench/micro/call_integer.ry`, `field_read.ry`, `call_record.ry` and
  `record_build.ry`, with `call_integer_1000000.ry` and
  `call_integer_200000.ry` the typed call with a literal bound (the
  owner's answer of 2026-10-08: into the repository, held like
  `bench/*.ry` by CI), and `tools/measure_native.sh <binary>` runs the
  whole set.
- **Stage 1, done: `LoadField`.** `Op::LoadField { slot, name, site }`
  is `Load(slot)` followed by `Field { name, site }`: the field read in
  the slot, the holder neither cloned nor dropped. Both emitters emit it
  for `x.field` and `self.field` on a local that is not a move receiver
  (`compile/expr.rs` `member`, `compiler/emit.ry` `emit_member` with
  `local_holder`) and for a field of a variant pattern
  (`compile/pattern.rs`, `field_pattern`); the interpreter's arm reads
  through the site cache (`Vm::op_load_field` in `vm.rs`) and the look
  at the next op that AG6 had put in `Op::Load` is gone; the generated
  code calls `rt_load_field` (one helper where it called two) and
  unboxes a scalar field as it did after `rt_field`; the analysis
  (`infer.rs`) treats it as a load of a boxed slot that pushes
  `abs_of_field`; the file (`file.rs`) writes and reads `OpLoadField`
  (`slot`, `name`, `site`), checks its three indices, and is format 4
  (`compiler/bytecode.ry` has the variant; `emit.ry` writes 4). The
  static count in `checker.ry`'s bytecode: 3,859 of 4,480 `Field` ops
  followed a `Load`; dynamically on the self-check `Load` was 29% and
  `Field` 6% of 101 million ops. Measured against the binary of
  commit 2fcab99 (`cachegrind`, the release build of each): the
  self-check 13.14 to 12.51 billion instructions on machine code
  (-4.7%) and 12.84 to 12.63 billion on the interpreter (-1.6%);
  `bench/records.ry` 571 to 516 million (-9.7%) and 655 to 653 million;
  per turn of the micro-benchmarks, machine code then the interpreter:
  the field of a local 1043 to 902 and 818 to 811, the call with a
  record parameter 1535 to 1394 and 1183 to 1161, the record built and
  read 1357 to 1216 and 1480 to 1464, the Integer call 668 unchanged
  and 1133 to 1104. Kept by the rule of AG6. The profiler's new table
  of operation pairs on the self-check after it (`Load Load` 6.1
  million, `Load Call` 6.0, `Load Const` and `Const Binary` 4.7, `Store
  Load` 3.8, `JumpIfFalse Load` 3.0, `Binary JumpIfFalse` 2.4, `Store
  LoadField` 2.1, `Binary Store` 1.6, the `or` short circuit `Binary
  Dup JumpIfTrue` 1.6, the `match` dispatch `Load IsVariant
  JumpIfFalse` 1.1, of 96 million) says each further fusion saves a
  dispatch and a push or a pop, about a percent of the self-check
  apiece: under the 2% a step must bring, so the fusions stop here
  (decision AR2).
- **Stage 2, done: direct calls (decision AR3).** Every code object
  compiles to two Cranelift functions (`codegen.rs`): the body
  (`renyi_direct_*`; `direct_signature`: the VM, the base, the pc, a
  flag for a direct entry, then the typed parameters in slot order;
  back come a status of `runtime::D_*` and the bits of a typed result)
  and the trampoline (`renyi_code_*`, the `Entry` signature the VM
  calls; `trampoline()`: at the start the typed parameters from the
  slots through `rt_param_*` with `DEOPT` when one does not fit, at a
  loop header zeros for the body to overwrite, the statuses mapped; the
  body's `Return` leaves a typed result in a register only when the
  flag says a direct caller waits, else on the stack through
  `rt_return_*` as before, so that a call from the interpreter costs
  what it cost). A call site (`call_direct`)
  loads the callee's body from the JIT's table (`Jit::direct_table`,
  by code object, filled at compilation; its address is a constant of
  the generated code, the box the JIT lives in keeps it fixed), asked
  `rt_direct_entry` when the table had none yet (which compiled a hot
  callee; since decision AT2 the call goes through the interpreter
  instead, which compiles a hot callee on the way), reads the count of
  generated frames (`Jit::depth`, by its
  address) and goes through `call_through_helper`, the `rt_call` path
  as before, when there is no body or the count is at `DEPTH_LIMIT`;
  else it counts the frame on, boxes the operands of the boxed
  parameters in place, has `rt_direct_frame` (`Vm::push_frame_direct`)
  push the frame with the boxed arguments moved into their slots by a
  mask, calls the body with the typed operands from registers, counts
  the frame off, takes a typed result from the payload or a boxed one
  from the stack, and sends a failure, a handed-back frame (run by the
  interpreter to its end) or an interrupt (its frame abandoned)
  through `rt_direct_after`. The body's returns: `rt_leave_typed` (no
  push), `rt_leave_unbox` (a boxed value where the declared result is
  typed: the bits when it fits, else boxed), `rt_leave_boxed`,
  `rt_left_status` after a helper left the frame. A site is direct
  only when every typed parameter has a typed operand in the caller;
  `infer.rs` holds a parameter's slot to its declared kind
  (`abs_of_params`; a function that stores another kind there stays
  with the interpreter, none in the compiler), so the caller derives
  the callee's signature from the declared types alone. `run_generated`
  counts the trampoline's frame on and off as before, on `Jit::depth`
  now (`Vm::native_depth` is gone). On the self-check with every code
  object compiled: 555 compiled, none rejected, 5.23 million calls from
  generated code, none to the interpreter, no deopt; the default
  tiering compiles 112 and sends 126 thousand of 4.1 million calls to
  cold callees. A first form of the stage asked `rt_direct_entry` on
  every call and carried the depth in the VM, moved by the helpers: it
  measured 12.51 to 12.53 billion instructions on the self-check (no
  gain; `Jit::ready` and the helper cost what the old path had cost),
  which is why the table and the count moved into the generated code.
  Measured against the binary of stage 1
  (`cachegrind`, the release build of each): the self-check 12.51 to
  12.32 billion instructions on machine code (-1.5%) and 12.63 to 12.91
  billion on the interpreter (+2.2%: the code layout, the loop is
  untouched); `bench/records.ry` 516 to 518 million; per turn on machine
  code: the typed call with its argument in a register 542 to 305
  (-44%), the same call with a boxed argument through `rt_call` 668 to
  719 (+8%, the trampoline), the record call 1394 to 1236 (-11%), the
  field and the record built unchanged. Kept by the rule of AG6; under
  the 2% that ends the round by AR1, which decision AR3 flags for the
  owner, the session going on to stage 3 as the round's purpose.
- **Stage 3, done: the pinned layout (decision AR4).** `Pinned<T>`
  (`crates/renyi_vm/src/pinned.rs`): a vector as three words in a fixed
  order, the pointer, the length and the capacity, with `push`, `pop`,
  `truncate`, `insert`, `extend`, `split_off`, `drain_into`, `resize`,
  `room` and `grow` on the words themselves and `with_vec` (a `Vec` made
  of the parts, used, taken apart) for what remains; `Deref` to a slice,
  `From<Vec<T>>` and into one. `Vm.stack`, `Vm.frames`, `Vm.handlers`
  and `Vm.field_cache` are pinned, `Record.fields` and `Variant.fields`
  too, `Vm`, `Frame`, `FieldSite`, `Record` and `Variant` are `repr(C)`,
  `Value` and `Int` are `repr(C, u8)` (24 and 16 bytes as before, X3).
  `value::layout` holds the constants (the tag at 0 in declaration
  order, the payload at 8, `RC_TAGS` the thirteen tags with an `Rc`
  payload, the `Int` tag at 8 and its payload at 16, `RC_VALUE` 16 into
  an allocation, the record's and the variant's offsets) and
  `layout_tests` holds them to the types (the strong count is the first
  word of an `Rc` allocation; an `Rc<str>` is a fat pointer whose
  address is the allocation). In `codegen.rs`: the stack section
  (`stack_vector`, `stack_len`, `stack_ptr`, `item_address`,
  `slot_address`, `copy_value`, `tag_at`, `rc_of` (one branch: the tag's
  bit in the mask or a big `Int`, both payload words read and one
  selected), `retain`, `release` (`rt_drop_at` when the count was one),
  `push_copy`, `write_typed`, `push_typed_value`); `Load`, `LoadMove`,
  `Store`, `Pop`, `Dup`, `Const`, `Nothing`, `IsNothing`, `IsFailure`,
  `JumpIfAbsent` and `JumpIfFailure` in place; `box_position` pushes a
  typed top in place (`rt_insert_*` below the top as before),
  `unbox_top` and `take_bool_top` read the tags (a value that does not
  fit deopts as before, a non-Boolean condition crashes through
  `rt_take_bool`); the prologue checks the stack's capacity once for the
  frame's locals and its deepest operand stack (`rt_room`), so no push
  checks; the frames section (`frames_vector`, `handlers_vector`,
  `frame_address`, `push_nothing_inline`, `push_frame_inline`: the boxed
  arguments moved to their slots by the static mask (every other local's
  tag `Nothing` was written here until decision AT2 moved it to the
  callee's prologue, once per callee), the record written with the
  caller's grant or `rt_frame_grant`'s when the callee narrows,
  `rt_room` and `rt_grow_frames` for the capacities (the count of calls
  at `Addresses::calls` went in AT2); `leave_frame_inline`: the holders
  among the locals
  and the boxed operands under the result released, up to
  `RELEASES_INLINE` (4) of them, else `rt_truncate`, the handlers cut,
  the frame popped; `leave_boxed_inline`: the result moved to the
  frame's first slot, `D_FAILURE` by its tag); every `Return` arm and
  `ReturnNothing` through them (`rt_direct_frame`, `rt_leave_*`,
  `rt_return_*`, `kind_code` and `Vm::push_frame_direct` are gone;
  `Addresses { depth, direct_table, calls }` replaces the two address
  arguments of `compile`); `LoadField` reads the site's cache entry and
  the holder in one path for a record and a variant (the type first in
  both, the tag and the fields selected by the holder's tag,
  `usize::MAX` the record's tag as the cache has it) and copies the
  field with one more reference, else `rt_load_field` as before;
  `RENYI_NATIVE_REGALLOC` selects Cranelift's register allocator (a
  development aid, `mod.rs`). Measured as AR1 says, each sub-stage
  against the one before (the stage-2 binary first): 3a+3b the
  self-check 12.32 to 12.10 billion instructions on machine code
  (-1.8%), the interpreter 12.91 to 12.90, `bench/records.ry` 518 to 459
  million (-11.5%), per turn the typed call 305 to 311, the boxed call
  719 to 633, the field 901 to 731, the record call 1236 to 1055, the
  record built 1215 to 1109; 3c the self-check 12.10 to 11.69 (-3.4%),
  the interpreter 12.94 (+0.3%), records 459 to 464 (+1.1%, which is
  Cranelift's time on the larger IR, the run-time code equal), the typed
  call 311 to 119 (-62%), the boxed call 633 to 587, the record call
  1055 to 873; 3d as first written the self-check 11.69 to 11.70 (+0.1%:
  the IR had grown from 131 to 236 thousand Cranelift instructions in 19
  to 34 thousand blocks since 3b, and Cranelift's own work from 5.6% to
  12.5% of the self-check, which ate the run-time gain), the interpreter
  12.94 to 12.90, records 464 to 429 (-7.5%), the field 730 to 648, the
  record call 873 to 790, the record built 1109 to 1005; made compact
  (the reference-count test one branch with both payload words read and
  one selected, the stack's capacity checked once in the prologue for
  the frame's locals and its deepest operand stack instead of at every
  push, one path for a record and a variant in `LoadField` with the type
  at one offset in both and the rest selected, `RELEASES_INLINE` 4): the
  self-check 11.69 to 11.66 (-0.25%), the interpreter 12.90, records 464
  to 437 (-5.8%), per turn the typed call 119 to 131, the boxed call 587
  to 605, the field 730 to 677, the record call 873 to 849, the record
  built 1109 to 1041; the IR 200 thousand Cranelift instructions in 15
  thousand blocks, Cranelift 10.3% and the generated code 16.6% of the
  self-check (the first form was faster per turn and slower on the
  self-check: the compact test and the selected path cost at run time
  what the compile time pays back, so a branchy test for a value without
  a reference, cheap at run time and small in IR, is a form still to
  try). The register allocator: `single_pass` halves Cranelift's time
  (658 to 339 ms on the self-check) and loses more in the code (the
  self-check 11.70 to 12.19 billion, +4.2%; records +11%, the record
  call per turn 789 to 1025), so `backtracking` stays.
- **The tiering (decision AR5), the owner's choice for the round's next
  step.** The hotness factor swept on the self-check with the stage-3
  binary (`RENYI_NATIVE_HOT`, the same binary for every value): 500
  13.15 billion instructions (205 code objects compiled, Cranelift 1.1
  s), 1000 12.09, 2000 11.66 (112 compiled), 4000 11.24, 6000 11.16,
  8000 11.13 (51 compiled, 1,477 ops, Cranelift 146 ms), 12000 11.23 (40
  compiled), 16000 11.39, 32000 11.78; `bench/records.ry` the other way,
  433 million at 500 to 437 at 2000, 451 at 8000 and 469 at 16000 (one
  hot loop in `main`: every doubling is more turns interpreted before
  the loop-header entry). `HOT_FACTOR` is 8000: the self-check -4.5%,
  records +3.2%, by the rule. A second factor for the loop-header entry
  (`LOOP_FACTOR` 500 beside the call factor, so that a running loop
  compiles sooner) was built and measured and lost: with the call factor
  2000 the self-check 11.66 to 12.05 billion (137 compiled, the loops of
  twenty-five more code objects whose turns did not pay), with 4000
  11.24 to 11.77; the code was reverted, the numbers are in AR5. The
  shape to try next, if the round goes on: a cheaper first tier (the IR
  without the inline sequences of AR4 for a code object that just became
  warm, the full one when it stays hot), so that the cost of being wrong
  about a function falls instead of the threshold rising; the count of
  runs so far predicts the runs to come poorly for the self-check's
  medium functions.
- **The round's end (decision AR6).** The owner closed the round on
  2026-10-08 after the tiering: against the binary AQ left, the
  self-check 13.14 to 11.13 billion instructions on machine code
  (-15.3%), `bench/records.ry` 571 to 451 million (-21%), the typed
  call per turn 668 to 130, the field 1043 to 677, the record call
  1535 to 849, the record built 1357 to 1041 (`tools/measure_native.sh`
  on the release build of d26dfac; the interpreter within 3% of where
  it was). A later round starts from the profile below and the cheaper
  first tier of AR5.
- **After stage 3.** The profile of the self-check on machine code
  (`cg_annotate`, the scratch directory): the generated code 16.6%,
  `run_frames` 8.7% (`main` and the 443 code objects called but cold),
  Cranelift and regalloc2 10.3%, `drop_glue::<Value>` 5.1%,
  `call_from_stack` 3.0%, `status` 2.9%, `call_primitive` 2.4%,
  `Value::clone` 2.2%, `binary_values` 1.8%, `rt_binary` 1.8%,
  `push_frame_in_place` 1.7%, `Value::eq` 1.7%, mimalloc about 3%,
  `refined_record` 0.8%, `rt_construct_variant` 0.8%, `op_load_field`
  0.7% (the cache misses), `Jit::ready` 0.7% (the IR built), `op_with`
  0.7%. The IR's size is now a cost the round must count: every inline
  sequence is Cranelift time at each of the 112 compilations, so a
  change is measured on the self-check with both effects, and the next
  steps that remain are the primitive boundary (`status`,
  `call_from_stack`, `call_primitive`, `run_primitive`, about 9%: the
  `Result` moved, the arguments drained into the scratch vector, the
  grant and budget checks of a pure primitive), the interpreter's own
  share (`run_frames` 8.7%: `main` and the 443 code objects called but
  cold under `HOT_FACTOR`, against the compile time of each), the clone
  and the drop that remain in the helpers and the primitives
  (`drop_glue` 4.9%, `clone` 2.1%), and the boxed binary operations
  (`rt_binary` and `binary_values`, 3.6%).
- **Measuring here.** `tools/measure_native.sh <binary>`: `valgrind
  --tool=cachegrind --cache-sim=no <binary> run [--interpret]
  compiler/checker.ry compiler/bodies.ry` for the self-check (about
  four minutes a run), the same on `bench/records.ry`, and the micro
  programs of `bench/micro/` at two sizes; the binary before a step
  kept beside the binary after it (the release build of each, 1.97.0
  here). `RENYI_NATIVE_REPORT=1` prints the compile statistics (the
  IR's size in Cranelift instructions and blocks, the time by phase,
  the cold callees), `RENYI_NATIVE_HOT=<n>` sets the hotness factor and
  `RENYI_NATIVE_REGALLOC=<algorithm>` the register allocator for one
  run, and `cg_annotate` on the self-check shows Cranelift's own
  share, which the IR's size moves.

## The profile-guided round on strings and JSON as it exists (decision AQ; session 9, 2026-10-08)

The first of the interspersed performance items the owner set on
2026-10-07 (next steps, item 5), measured on the two benchmarks the
positioning compares with CPython. Session 8 profiled the two programs
on the owner's machine and paused with the change written as a script
(commit 44763c4); session 9, the first in the cloud environment,
applied it, reviewed the diff, ran the gates and measured it. The entry
AQ1 of the decisions has the profiles and the change in full.

- **Where the code is.** `crates/renyi_vm/src/natives/json.rs`: `Path`
  (the position as a chain of borrowed segments, rendered only for a
  `Mismatch` or `Constraint`), `field_type` (a `Cow`: the declared type
  borrowed unless the type has parameters), `Naming::key` (a `Cow` too),
  `decode_at` over `vm.program` borrowed, the `Sink` trait with
  `TreeSink` (the tree of `encode`, kept by the recording,
  `http.post_json`, `server.ok_json` and the Python bridge) and
  `TextSink` (the text of `render`, `render_indented` and `render_with`,
  no tree in between), `walk` (the one encoder over a value) and the
  unit test that holds the text sink equal to `write_json` on both
  layouts. `crates/renyi_json/src/lib.rs`: the reader keeps the text
  beside its bytes and `string` copies a run of plain bytes at once;
  `write_string` copies the runs between escapes; both it and `newline`
  are `pub` for the sink. `crates/renyi_vm/src/value.rs`:
  `Value::character` and the per-thread `ASCII` table;
  `natives/prelude.rs`: `characters(text)`, used by `characters()`,
  `split("")` and `Vm::iterate` over a text (`vm.rs`);
  `natives/regex.rs`: `CACHE` and `CACHE_LIMIT` (256, then emptied);
  `native/runtime.rs`: `op_concat` joins the pieces on the stack, the
  text sized first, an Integer written straight into it.
- **Conformance case 54**, `tests/conformance/programs/json_paths.ry`
  with `expected/json_paths.out`: the differential program of the
  session, printed by the binary before the change and held to by the
  binary after it on both tiers and by the development build. It
  covers the path of every `Mismatch` and `Constraint` error (indices
  under keys, the variant's `kind`, a map's key, a missing field, a
  refined subtype), a document with control characters, escapes and
  non-ASCII text on both layouts, the three namings there and back,
  `characters()`, `split("")`, a loop over a text, every kind of value
  in a hole (a big Integer, a Float, a Decimal, a Boolean, a record, a
  variant, a text) and the pattern cache (a thousand matches of one
  pattern, three hundred of different ones, past the cache's limit).
- **The numbers** (2026-10-08, the cloud environment: Linux, four
  cores, the release build with rustc 1.97.0, the two binaries built
  from the same tree with and without the change; `cachegrind` counts
  user-space instructions, `tools/bench.py --runs 5` the wall-clock
  time, the two binaries back to back):

  | program | instructions, machine code | instructions, interpreter | wall-clock, machine code | wall-clock, interpreter | CPython |
  |---|---|---|---|---|---|
  | `strings` | 1.74 to 1.39 billion (-20%) | 1.97 to 1.62 billion (-18%) | 282 to 226 ms | 284 to 262 ms | 106 ms |
  | `json_round_trip` | 2.34 to 1.00 billion (-57%) | 2.36 to 1.01 billion (-57%) | 314 to 126 ms | 303 to 125 ms | 230 ms |
  | `checker.ry` on `bodies.ry` | 13.40 to 13.14 billion (-2.0%) | 13.12 to 12.84 billion (-2.1%) | 2895 to 2679 ms | 2572 to 2496 ms | |

  `primes`, `records` and `hello` are unchanged within the machine's
  noise, which moved a repeated measurement by a tenth (CPython's
  `primes` twin by a fifth between the two rounds): a wall-clock
  difference under a tenth says nothing here, and the instruction
  counts are the measure, as in AG6. Against the owner's targets: JSON
  is past CPython (1.8 times its speed, from 0.7); strings stands at
  half of CPython's speed (from four tenths); the self-check moved by
  two percent. The numbers session 8 wrote down to beat (CI's
  development build of a2b0702, one run: `strings` 179 ms and
  `json_round_trip` 386 ms against CPython's 43 and 89 ms) are of
  another machine and another build, and CI prints the new ones on the
  next push.
- **Where the rest goes** (`cachegrind` on the interpreter after the
  change, `cg_annotate`): on `strings`, 40% in the interpreter loop, 30%
  in the primitive boundary once per character (`call_primitive`, the
  move of the arguments into the scratch buffer, `run_primitive`,
  `text_contains`, the drops, `guarded`, `effect_of`), 7% dropping and
  4% cloning values, 5% the `contains` search itself, 2% `characters`,
  2% `op_concat`, the allocator under 2%; the page faults of the 29 MB
  list of one-character texts (now one allocation at its exact length)
  are kernel time outside the count, which is why the interpreter's
  wall-clock gain is smaller than its instruction gain. On
  `json_round_trip`, 21% in the reader, 8.5% in `write_string`, 8% in
  `decode_at`, 4% in `walk`, 7% growing vectors and texts, 9% in the
  allocator, 5% in the interpreter loop, 3% in the keys, 2% parsing
  Integers, 1% finding a field by name: the plain cost of reading and
  writing the format. The profiles are `cg_annotate` output of the
  session; nothing of them is in the repository.
- **Not done, on purpose**: `Text` stays `Rc<str>` (decision X3), so
  `change out to "{out}{piece}"` in `compiler/` still copies the
  accumulated text per piece (the lexer's `scan_segment` stays
  quadratic in a token's length); the pure primitive boundary stays as
  AG6 left it, and it is now the largest item after the loop on
  `strings`; `Text.matches` is not on the compiler's path (three calls
  in `project.ry` and `refine.ry`), so the cache changes nothing for
  the self-check; `http.post_json` and `server.ok_json` still build the
  tree and write it, which `TextSink` could replace in a line each
  (not on any benchmark).
- **How it was gated**: `cargo fmt`, `cargo clippy --all-targets -D
  warnings` and `cargo test` (315 tests) with rustc 1.94.1 (the CI
  toolchain, installed beside the environment's 1.97.0 so that the
  lints agree), the conformance suite by both runners (54 cases), the
  corpus canonical, `compiler/*.ry`, `bench/*.ry` and the starter pack
  checked, formatted and tested, the lint. The release binaries and
  the measurement used 1.97.0 (both binaries alike).
- **How to measure here**: build the previous commit's release binary
  first and keep a copy (`cargo build --release`, then copy
  `target/release/renyi` out of `target/`), apply the change, build
  again; `valgrind --tool=cachegrind --cache-sim=no <binary> run
  [--interpret] <program>` prints `I refs`, deterministic under load;
  `python tools/bench.py <binary> --runs 5` needs a quiet machine (no
  build or judge running). On the owner's machine the lane's
  `ab_stat.sh` (`perf stat`) does the same with cycles beside the
  instructions.

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
  `json.parse`). Locals are slots on the value stack; `change x to
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

## The embedding API as it exists (decisions AP1 and AP2; session 8, 2026-10-08)

The fourth and last step of M5 in the owner's order: `renyi::Sandbox`,
the grant as a `needs` clause, the memory budget, `renyi run --sandbox`.
The guide is `docs/embedding.md`.

- **The API.** `crates/renyi/src/sandbox.rs`, exported by the crate:
  `Grant` (`capabilities`, `memory` in bytes; `parse` reads the `needs`
  spelling through `renyi_syntax::parse_grant` and checks each
  capability is known, takes its scope and names known sinks;
  `from_json` and `from_file` read `{"grant": ..., "memory": "256
  megabytes"}` strictly; `parse_memory`, `bytes_text`), `Sandbox`
  (`load(path, grant)` through `compile_sources`, a `.ryc` refused;
  `load_source(name, text, grant)` through the new `compile_file`; the
  public functions of the main module from `renyi_index::index_files_in`
  with the tool schemas from `tools_of_in`; `module`, `grant`,
  `functions`, `warnings`; `call(name, args)`: the name must be a
  public function, the arity must match, every `need` must be covered
  by the grant, then a `Vm` of its own with `Narrowing { sandbox }` and
  `Options.memory`, `begin_run(function.needs)`, the budgets of the
  previous call resumed (`Vm::budgets`, `resume_budgets`), the result
  mapped to `CallError::{Refused, Failed, Crashed, Exited, OverMemory}`;
  `guarded(value, capability)` tags a value with the bit of the grant's
  guard), `Function` (name, signature, purpose, parameters, needs,
  `exposed_as_tool`, `input_schema` as a JSON text).
- **The VM.** `memory.rs`: `Counting<A>`, a `GlobalAlloc` wrapper with
  the statics `INSTALLED`, `ACTIVE`, `NET`, `PEAK`, `LIMIT`, `OVER`
  (one load per allocation while inactive; `begin(limit)`, `end()`,
  `over()`, `peak()`, `installed()`). `Options.memory`, `Vm.memory` and
  `Vm.counting`; `begin_run` starts the count (refused without the
  allocator, or while another budget is in force) and `Drop for Vm`
  ends it; `memory_check` at the `call!` macro, at a backward `Jump`
  and at `call_primitive`; a budgeted run is the interpreter's.
  `Interrupt::OverMemory { limit, used }` and `RunOutcome::OverMemory`.
  `Narrowing.sandbox`: `effective` intersects the declared capabilities
  with it (`within`, then only what the sandbox covers) and adds its
  budgets; `begin_run` adds its guards after the declared ones.
- **The binary.** `renyi::Allocator` (mimalloc under `Counting`)
  declared in `src/main.rs`, no longer in the library, so that a host
  links its own; `Counting` and `Value` re-exported. `--sandbox
  <grant.json>` among the flags of `run`, `record` and `serve`
  (`sandbox::apply_flag` reads the file into `Narrowing.sandbox` and
  returns the memory; `sandbox::refusal` refuses when `main` needs a
  capability the grant does not cover; `test` refuses the flag);
  `exit_of` exits 2 on `OverMemory` with the limit and the peak in
  words. `renyi_syntax::parse_grant` parses a `needs` clause alone.
- **Held equal.** `crates/renyi/tests/sandbox.rs` (its own counting
  allocator): a module of eight public functions loaded from a text,
  their list with purpose, parameters, needs and signature; `add`,
  a private function refused, the arity refused, `shout` under
  `console`, a failure rendered, a crash with its location; `shout`
  refused under an empty grant and `add` still answered; a scope
  (`filesystem.read` of one directory, another refused at the boundary
  with `PermissionDenied`); `at most 2 per run` counting across three
  calls; a guarded value printed but refused into a file with
  `Guarded`, the file not written, a plain value written; a memory
  budget of 8 megabytes stopping a list of four million items with
  `OverMemory` and the next call answered; and the binary with
  `--sandbox`: `hello` under `console, environment` with a memory
  budget, refused under `console` alone naming `environment`, a grant
  with an unknown capability refused, `test --sandbox` refused. The
  unit tests of `sandbox.rs` cover the grant's spellings and the
  memory's.
- **Not covered.** The console output of a sandboxed call goes to the
  process's streams, not to the host; the count is the process's, so a
  multi-threaded host's other threads count during a budgeted call and
  one budget is in force at a time; a bytecode file cannot be loaded
  (no visibility in the format); each call builds a VM, which costs
  the setup of the natives table and the field cache per call; the
  cost of the counting allocator on the benchmarks is not measured
  (one relaxed load per allocation and deallocation by design); no C
  API yet (decision A5).

## The watch of `serve` as it exists (decision AO1; session 8, 2026-10-08)

The third step of M5 in the owner's order: `renyi serve --watch`, a
reload between two requests on the resident world of AN1.

- **The command.** `crates/renyi/src/serve.rs`: `renyi serve` parses
  the flags of `run` (`--watch` is one of them now; `run` and `test`
  refuse it); without `--watch` it is `run_command`. With it, the first
  version is compiled as `run` compiles (its errors end the command), a
  `Workspace` in canonical mode is opened on the file's project (the
  nearest `renyi.json` above it, else its directory) and refreshed once
  for the map, and `main` runs in a loop: every run gets
  `Options.watch`, a closure over the watch state, and
  `Options.listener`, the socket the previous run left; a run that
  ends with `RunOutcome::Reload` takes the version the watch prepared
  and starts again, any other outcome is `run`'s exit status
  (`exit_of`, shared with `run`). The watch's `poll`:
  `Workspace::refresh`, nothing to do unless the world was declared
  again; then `compile_sources` (errors: "the new version has errors;
  still serving the last good one:" with the diagnostics); the
  program's sources compared text for text with the serving version (a
  change elsewhere in the directory is no new version); `--deny`
  checked as at start; the map built and diffed with
  `renyi_index::diff`; the message "reloading <file>: 2 definitions
  changed: app.handle (body changed); app.version (added)", a signature
  change as "signature `old` -> `new`". `--watch` takes `--deny`,
  `--allow-*`, `--at-most`, `--explain` and `--interpret`; `--replay`,
  `--manifest`, `--profile`, `--to`, `--redact` and a `.ryc` file are
  refused.
- **The VM.** `Options.watch: Option<Box<dyn FnMut() -> bool>>` and
  `Options.listener: Option<TcpListener>`, copied into the `Vm`;
  `Interrupt::Reload` and `RunOutcome::Reload` ("stopped for a new
  version" in a manifest); `Run.listener` carries the socket out.
  `natives/server.rs`: `serve` continues on the handed socket when its
  port is the one asked for, else binds; under a watch the listener is
  non-blocking, `accept` is tried every 20 ms and the watch asked every
  500 ms (`WATCH_INTERVAL`, `WATCH_SLEEP`); when it answers true the
  listener goes back into `vm.listener` and the native returns
  `Err(Interrupt::Reload)`; an accepted stream is set blocking again.
  A request that arrives while a new version compiles waits in the
  backlog.
- **Held equal.** `crates/renyi/tests/serve.rs` runs the binary on a
  one-file service under `target/serve/`: "v1" answered; the body
  edited and the next request answered "v2", the message naming
  `app.handle (body changed)`; a version with an unknown name reported
  with its diagnostic while "v2" is still served; a definition added
  and the handler changed, "v3" with both named; the error reported
  once, no crash; and the refusals: a first version with errors (exit
  1, the diagnostics on the standard output), `run --watch` ("`--watch`
  is an option of `renyi serve`"), and `serve` without the flag as
  `run` on `examples/hello.ry`. The sequence of three reloads takes
  3.6 s in the test, the poll interval included.
- **Not covered.** `main` is run again whole, so what it did before
  `server.serve` (a print, a database opened) is done again; a file
  imported from outside the project's directory is compiled but not
  stat-ed, so a change to it alone is not noticed; the watch does not
  look while a request is being handled, so a reload waits for the
  request in flight; `serve_limit` stays the tests' business.

## The language server as it exists (decisions AN2 and AN3; session 8, 2026-10-07)

The second step of M5 in the owner's order: `renyi lsp`, a standard
server over standard input and output on the resident world of AN1,
and the client in the VS Code extension.

- **The command.** `crates/renyi/src/lsp.rs`: the base protocol's
  framing (`Content-Length`, then the JSON body), one `Workspace` in
  as-written mode over the folder `initialize` names
  (`workspaceFolders[0]`, else `rootUri`, else `rootPath`, else the
  working directory), the documents the editor opens as overlays
  (`didOpen`, `didChange` with whole texts, `didClose`), the
  diagnostics of every file of the folder published after each change
  for the files whose diagnostics changed (the workspace's plus
  `check_layout`, sorted, each with its code, `renyi` as the source
  and the fix after the message), `shutdown` and `exit` (exit status 0
  after a shutdown, 1 otherwise), `-32002` before `initialize`,
  `-32601` for a method it does not serve. `lsp/describe.rs`: what is
  under a position (the innermost reference there, else the
  declaration whose name is there), described by its signature
  (`renyi_index::function_signature` and `type_signature`, now public)
  and its purpose, located at its declared name; the outline (functions,
  types with their fields or variants, abilities with their methods,
  implementations with theirs, constants, tests); positions as
  zero-based lines and UTF-16 code units. Hover on a library definition
  reads the library's declaration files; definition answers null for
  it. URIs: `file:` with percent-decoding, a drive letter's leading
  slash dropped, an open document's URI reused as the editor spelled
  it.
- **The workspace** gained `as_written()` (no formatting; the tree,
  the diagnostics and the map over the text on disk or in the overlay,
  the mode of `renyi check`), the lazy map (`index(header)` builds it
  when the last refresh changed anything, `index_built()` reads it;
  `refresh()` no longer takes the header), `file`, `files`, `module_of`
  and `file_of`. `tests/refresh.rs` holds the as-written diagnostics of
  the corpus and the compiler equal to `check_project_in` on the files
  as loaded.
- **The client.** `editors/vscode/src/extension.js` starts `renyi lsp`
  through `vscode-languageclient` for the `renyi` language; the setting
  `renyi.path` names the binary (`renyi` on the PATH by default); a
  server that does not start is one warning. `npm run build` bundles
  it with its library into `out/extension.js` (esbuild); `package.json`
  carries `main`, the activation on the language, the setting and the
  scripts; `.vscodeignore` leaves `src/`, `node_modules/` and the
  lockfile out of the `.vsix`; `package-lock.json` is committed for
  `npm ci`. CI builds the bundle on every push and the release
  workflow before packaging. The repository ignores `node_modules/`
  and `out/` under the extension.
- **Held equal.** `crates/renyi/tests/lsp.rs` drives the binary over
  pipes on a two-file project under `target/lsp/`: the handshake, the
  empty diagnostics of both files, a buffer opened with an unknown name
  in a text hole after an emoji (the error at its UTF-16 column, the
  fix in the message), the buffer restored (empty again), hover on a
  call (the signature and the purpose), definition into the other file
  at the function's name, hover on `console.print` (the library's
  declaration and purpose), the outline of a file (a record with its
  field, a function), hover on nothing, an unknown method, the clean
  exit after `shutdown`; and a request before `initialize`, refused,
  with the input closing an unclean exit. Unit tests cover the framing,
  the URIs and the positions.
- **Measured** (`time_lsp.py`-style driver: initialize, the first
  diagnostics, then a keystroke inside a body followed by a hover; a
  development build on Windows): `compiler/` 1200 ms to
  the first diagnostics, 92 to 135 ms per keystroke;
  `examples/` 80 ms, then 15 to 22 ms.
- **Not covered.** No completion, references, rename or formatting
  through the server (the editor's `renyi format` is the command line);
  the client has not been run in a VS Code window here, only bundled;
  incremental synchronization is not offered (whole texts); a
  workspace with several folders gets a server per folder from the
  client library's default.

## The resident world as it exists (decision AN1; session 8, 2026-10-07)

The first step of M5 in the order the owner set on 2026-10-07 (the
resident `World`, then the LSP, then `serve --watch`, then the
embedding API; decision AN1 records the four answers, AN2 the LSP's
shape): a crate of its own, `crates/renyi_workspace/`, that `renyi mcp`
holds between calls.

- **The crate.** `Workspace::new(root, &library)` parses the library's
  declaration files once and reads nothing; `as_written()` switches
  it to the editor's mode (the section above); `refresh()` does the
  work and reports it (`Refresh`: files read, files parsed, whether
  the world was declared again, items checked, items reused);
  `index(header)` builds the map when the last refresh changed
  anything and `index_built()` reads it; `checked()`, `file(name)`,
  `files()`, `canonical_text(file)`, `diagnostics(file)`,
  `module_of(name)` and `file_of(module)` answer from the last
  refresh; `set_overlay(name, text)` and `clear_overlay(name)` make a
  text stand in for a file on disk (an editor's unsaved buffer). A
  file is named relative to the root or as the map names it.
- **A refresh.** The `.ry` and `.renyi` files under the root (the
  store `.renyi/` left out) are listed and stat-ed. A file whose size
  and modification time are those kept is skipped, unless it was
  stamped within two seconds of its last write (file systems round the
  time; such a file is read and its text compared). A file read with
  the same text keeps its tree and its checks. A file with a new text
  is tagged (`renyi_check::tagged`), formatted and parsed as `renyi
  index` does (`canonical_files` now carries the `foreign` and
  `python` tags through, not only `package`), its declarations
  fingerprinted, its items' texts kept; when the fingerprint is the
  old one, the old items' checks are inherited where an item's text is
  unchanged, their spans moved to where the item now sits. Otherwise,
  and when a file appeared, disappeared or stopped parsing, or
  `renyi.json` or `renyi.lock.json` at the root changed (the files are
  tagged again), every item is checked again, and only then are the
  dependencies resolved again (`imported_files`). The world is
  declared again from the kept trees (the library's modules cloned,
  then the own files in path order and the dependencies in the order
  brought, as `load_project` orders them); each module's items are
  checked with `check::check_item` or reused; the module's diagnostics
  are assembled as `check_project_in` assembles them (parse,
  `module-name`, the world's, the items', the module's
  `purpose-missing`, sorted by position); the map is built with
  `renyi_index::index_checked` from the canonical files and the
  checked project, when `index` is asked for.
- **The fingerprint** is the text of everything a body elsewhere can
  see: the module's head (the text before the first item: the name,
  the docs, the imports), each function's signature and docs (the text
  before its body; the docs because `deprecated` warns at call sites),
  an implementation's head and its methods' heads, a test's name,
  needs and recording, and a type, an ability or a constant whole. A
  change to a body costs one check; a change to a signature costs them
  all.
- **The checker** gained `check::check_item(world, module, index)` and
  `check::module_purpose_diagnostic`, over which
  `check_module_with_references` is now written (`Checker::new` is a
  pure constructor, so an item checked alone is checked as in the
  loop); `module_name_mismatch` is public. The index gained
  `index_checked` (the second half of `index_files_in`) and
  `canonical_files` public.
- **Held equal.** `tests/refresh.rs`: the corpus, `compiler/` and
  `starter/workflows` give the map and every module's diagnostics of a
  fresh `index_files_in` and `check_project_in`, and a second refresh
  parses, declares and checks nothing; a two-file project is taken
  through a body edit (one item checked, three reused), a signature
  edit (all four, the caller now wrong), the caller mended, a file
  added and removed, a file broken and mended, the same text written
  again (read, not parsed), an overlay set (its text is what is
  checked) and cleared, each state equal to a fresh build. Unit tests
  cover the fingerprint, the names and the stamp's margin.
- **Measured** (a script driving `renyi mcp` over pipes: the legacy
  handshake, then `project_map` four times; a development build on
  Windows): `compiler/` 1453 ms for the first call, 12 to 13 ms for
  each later one (the map of 287 thousand characters rendered each
  time); `examples/` 152 ms, then 5 to 6 ms.
- **`renyi mcp`** holds a `Workspace` over `.` (it enters the served
  directory first) and refreshes it before `project_map`, `definition`,
  `effects` and `diff`; `Map` and its whole-directory rebuild are
  gone; `definition` reads the canonical text from the workspace.
- **Not covered.** The manifest is watched at the root only (a
  `renyi.json` above the served directory is read by the resolver, but
  a change to it is not noticed until a file changes); the resolver's
  problems (a missing package, a manifest that does not read) are not
  in the map, as they were not before; a dependency's files are read
  again only when a declaration or the manifest changed.

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
- The map comes from the resident world (`Server::refresh` on a
  `renyi_workspace::Workspace` over the served directory, decision AN1;
  the section above): the files read again where their stamps changed,
  the world declared again when any text did, the items checked again
  where their text changed; the compiled program of `run` and
  `run_tests` is built per call.
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

## Done in session 9 (the profile-guided round on strings and JSON and the baseline JIT, in the cloud environment)

Eleven commits on `main` (2fcab99, cf4c912, 02380ea, 5eeb28e, 176cfc2,
d26dfac and 5e4f1d0, the commit of AR6; then 4af6311, the handoff,
7094e1d and efa63a2, `renyi build` with AS1 to AS3, and 71965aa,
`build --exe` with AS4), each gated as in session 8 (rustc 1.94.1, the CI toolchain,
installed beside the environment's 1.97.0 for `cargo fmt`, `cargo
clippy --all-targets -- -D warnings` and `cargo test`; the conformance
suite by both runners; the corpus canonical; `compiler/*.ry`,
`bench/*.ry` and the starter pack checked, formatted and tested; the
lint). For most of the session the pushes were refused (403) until
the owner installed the Claude GitHub App on the `renyi-lang`
organisation (https://github.com/apps/claude/installations/select_target;
a cloud session needs it, and the patches were handed to the owner as
a bundle meanwhile); the seven then went up in one push and CI passed
on the head (run 51 on 5e4f1d0), and the four after them went up one
by one, CI green on each (runs 52 to 55; 55 on 71965aa).

1. the round (decision AQ; the section "The profile-guided round on
   strings and JSON as it exists"): `crates/renyi_vm/src/natives/json.rs`
   (`Path`, `field_type`, `Naming::key` as a `Cow`, the `Sink` trait,
   `TreeSink`, `TextSink`, `walk`, the unit tests),
   `crates/renyi_json/src/lib.rs` (the reader by runs, `write_string`
   by runs, `newline` and `write_string` public), `value.rs`
   (`Value::character`, the `ASCII` table), `natives/prelude.rs`
   (`characters`), `vm.rs` (the loop over a text), `natives/regex.rs`
   (the cache), `native/runtime.rs` (`op_concat`); conformance case 54
   (`tests/conformance/programs/json_paths.ry`,
   `expected/json_paths.out`, the manifest); `tools/apply_perf_round.py`
   deleted; the decisions (section AQ), `docs/GAPS.md` (the performance
   entry of section 4, the status at the end of section 7), `CLAUDE.md`
   (the decisions row), this file.
2. the baseline JIT, stage 1 (decisions AR1 and AR2; the section "The
   baseline JIT"): `Op::LoadField` in `crates/renyi_vm/src/bytecode.rs`
   (kind 52), emitted by `compile/expr.rs` (`member`) and
   `compile/pattern.rs` and by `compiler/emit.ry` (`emit_member`,
   `local_holder`, `field_pattern`); the interpreter's arm and
   `Vm::op_load_field` in `vm.rs`, the look at the next op gone from
   `Op::Load`; `infer.rs`, `codegen.rs` and `runtime.rs`
   (`rt_load_field`); `file.rs` (format 4, the op written, read and
   checked) with `tests/file.rs` and `crates/renyi/tests/compile.rs`;
   `compiler/bytecode.ry` (the variant, format 4 in `emit.ry`); the
   profiler's table of operation pairs (`profile.rs`,
   `tests/semantics.rs`, `05-agent-tooling.md`); the decisions (section
   AR), `docs/GAPS.md`, `CLAUDE.md`, this file.
3. the baseline JIT, stage 2 (decision AR3; the section "The baseline
   JIT"): `codegen.rs` (`direct_signature`, the body as
   `renyi_direct_*`, `trampoline`, `call_direct`, `call_through_helper`,
   `return_direct`, `return_with`, `payload_bits`, `push_payload`, the
   `Return` arms, the exit), `runtime.rs` (the `D_*` statuses,
   `kind_code`, `rt_direct_entry`, `rt_direct_frame`, `rt_direct_after`,
   `rt_leave_typed`, `rt_leave_unbox`, `rt_leave_boxed`,
   `rt_left_status`), `mod.rs` (`State::Ready.direct`, `Jit::direct`,
   `Jit::depth`, `Jit::direct_table`), `vm.rs` (`direct_entry_of`,
   `push_frame_direct`, `run_generated` on the JIT's count), `infer.rs`
   (`abs_of_params`, the rule on parameter slots); the decisions
   (AR3), `docs/GAPS.md`, this file.
4. the baseline JIT, stage 3 (decision AR4; the section "The baseline
   JIT"): `crates/renyi_vm/src/pinned.rs` (new: `Pinned<T>`),
   `value.rs` (`repr(C, u8)` on `Value`, `repr(C)` on `Record` and
   `Variant` with pinned fields, the module `layout`, `layout_tests`),
   `integer.rs` (`repr(C, u8)` on `Int`), `vm.rs` (`repr(C)` on `Vm`,
   `Frame` and `FieldSite`; the stack, the frames, the handlers and the
   field cache pinned; `frame_grant` crate-visible; `push_frame_direct`
   gone), `native/codegen.rs` (the stack and the frames sections,
   `Addresses`, the arms in place, the prologue's room check, the
   merged `LoadField`), `native/runtime.rs` (`rt_room`,
   `rt_grow_frames`, `rt_frame_grant`, `rt_drop_at`; `rt_direct_frame`,
   `rt_leave_*`, `rt_return_*` and `kind_code` gone), `native/mod.rs`
   (`Addresses`, `RENYI_NATIVE_REGALLOC`), `lib.rs` (`pub mod pinned`);
   the decisions (AR4), `docs/GAPS.md`, `CLAUDE.md` (the `renyi_vm`
   row), this file.
5. the micro-benchmarks into the repository (the owner's answer):
   `bench/micro/` (six programs), `tools/measure_native.sh`, the CI
   step, `CLAUDE.md`, this file; then the tiering (decision AR5):
   `HOT_FACTOR` 8000 in `crates/renyi_vm/src/native/mod.rs`, the
   decisions (AR5), `docs/GAPS.md`, this file; then the round closed
   (decision AR6: the totals), the plan's item 4, this file.
6. `renyi build` (decisions AS1 to AS3; the section "`renyi build` as
   it exists"): `crates/renyi_vm/src/vm.rs` (`NativeState`,
   `Options.image`, the load in `Vm::new`), `native/codegen.rs` (the
   address-free code: the helper table, the state's offsets, the
   constants and the bodies through tables, `Compiled` as bytes,
   `machine_code`), `native/mod.rs` (`CodeArena`, `Jit::new` with the
   level, `isa`, `host_target`, `compile_code`, `compile_everything`,
   `load_image`, `state_pointers`; `cranelift-jit` and
   `cranelift-module` gone, `region` and the icache crate in),
   `native/image.rs` (new), `native/infer.rs` (the kinds' bytes),
   `native/runtime.rs` and `runner.rs` (the counters), `Cargo.toml`;
   `crates/renyi/src/lib.rs` (`build_command`, `load_image_file`, the
   image through `compile_with_sources` to `run`, `record`, `test` and
   `reproduce`, the usage), `crates/renyi/tests/build.rs` (new); the
   decisions (section AS), `docs/reference.md` (appendix B),
   `docs/GAPS.md`, `CLAUDE.md`, this file.
7. `renyi build --exe` (decision AS4; the section "`renyi build` as it
   exists", stage C): `crates/renyi/src/exe.rs` (new), `lib.rs`
   (`run_embedded`, `run_loaded`, `load_image_bytes`, `--exe` in
   `build_command`, the usage), `tests/build.rs` (the executable's
   test); the decisions (AS4), `docs/reference.md` (appendix B),
   `docs/GAPS.md`, `CLAUDE.md`, this file; the cache simulation's
   finding on `speed_and_size` and the wall-clock picture of the image
   against the JIT, in the section.
8. the size round, stage 1 (decisions AT1 and AT2; the section "The
   size of the generated code"): `crates/renyi_vm/src/native/codegen.rs`
   (the prologue's locals, the field read's one path, `deopt_block` and
   `emit_deopts`, the direct call without the ask path, no call
   counter), `value.rs` (`Record::tag`, `RECORD_TAG`, the assertions),
   `vm.rs` (`NativeState` without `calls`, `direct_entry_of` gone),
   `native/runtime.rs` (`rt_direct_entry` gone, no count), `native/mod.rs`
   (`report`, `Jit::direct` gone), `native/image.rs` (`CODE_FORMAT` 2),
   `runner.rs`; `tools/measure_size.sh` and `tools/image_census.py`
   (new); the decisions (section AT), `docs/GAPS.md`, `CLAUDE.md`, this
   file.
9. the size round, stage 2 (decision AT3; the section "The size of the
   generated code"): `crates/renyi_vm/src/binary.rs` (new), `lib.rs`
   (the module), `file.rs` (`check` shared), `native/image.rs` (the
   image's program and hash, format 2); `crates/renyi/src/lib.rs`
   (`Hashed::Given`, the decode), `tests/build.rs` (the round trip and
   the cross-reproduction); `tools/image_census.py` and
   `tools/measure_size.sh` (the `.ryc` beside the image); the decisions
   (AT3), `docs/reference.md` (appendix B), `docs/GAPS.md`,
   `CLAUDE.md`, this file.
10. the size round, stage 3a (decision AT4; the section "The size of the
    generated code"): `crates/renyi_vm/src/native/codegen.rs` (the
    static height, the frame pointer); the decisions (AT4),
    `docs/GAPS.md`, this file.
11. the image's load (decision AT5; the same section): `native/image.rs`
    (the code section, `Placement`, `MappedSection`, `Image::open`,
    `Image::open_at`, format 3), `native/mod.rs` (`Pages`,
    `CodeArena::map_section`, `Jit::load_image` with the fall-back,
    `Jit::mapped`, the report), `file.rs` (the check's messages on
    failure only), `Cargo.toml` (`memmap2`); `crates/renyi/src/lib.rs`
    (`check_image`, `Image::open`), `exe.rs` (the padding, the file
    handed over), `tests/build.rs`; `tools/image_census.py`; the
    decisions (AT5), `CLAUDE.md`, this file.
12. the size round, stage 3b (decision AT6; the same section):
    `crates/renyi_vm/src/native/codegen.rs` (`retain` and `release` as
    calls, `rc_of` and the inline sequences gone, `STACK_SAFE`),
    `native/runtime.rs` (`rt_retain_at`, `rt_drop_at`'s role),
    `native/image.rs` (`CODE_FORMAT` 3); the decisions (AT6),
    `docs/GAPS.md`, this file.
13. the round's close (decision AT7, the owner's answer of 2026-10-09,
    with the benchmark table against CPython of that day): the
    decisions, `docs/GAPS.md`, this file.

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
10. The rename of `set` to `change` (decision AA1, the section "The
    rename of `set` to `change`" above): the grammar, the reference, the
    cheat sheet, the decisions (section AA), both front ends, the
    corpus, the conformance programs, the crates' tests, the lint,
    `docs/GAPS.md`, this file.
11. The compiler written in Renyi (decision Z3, the section "The
    compiler written in Renyi" above): `compiler/project.ry`, `emit.ry`
    and `compile.ry` (new), `checker.ry` (the shared pieces moved out),
    `bodies.ry` (four helpers made public), `crates/renyi_check/src/
    lib.rs` (`imported_files` joins with `/`), the judge in
    `crates/renyi/tests/selfhost.rs`, `README.md`, `CLAUDE.md`,
    `docs/GAPS.md`, this file.
12. Constraints with type arguments (decision AB1, the section
    "Constraints with type arguments" above): `parser.rs`, `world.rs`,
    `check.rs`, `vm.rs`, `natives/prelude.rs`, `library/std/prelude.ry`,
    `compiler/parser.ry`, `declare.ry`, `bodies.ry` and `types.ry`,
    `rules.rs`, `tests/library.rs`, conformance cases 39 to 42, the
    grammar, the reference, the cheat sheet, the sketch, the library
    sketch, the decisions (section AB), `CLAUDE.md`, `docs/GAPS.md`,
    this file.
13. The judges run from bytecode (the section "The judges run from
    bytecode" above): `crates/renyi/tests/selfhost.rs` (`front_end`,
    the driver's file passed to every judge, the fixed point of
    `compile.ry`), `CLAUDE.md`, `docs/GAPS.md`, this file.
14. Packages, the first commit (decision AC1, the section "Packages"
    above): `crates/renyi_json` and `crates/renyi_package` (new),
    `renyi_syntax` (`Package`, `SourceFile.package`), `renyi_check`
    (`world.rs`, `check.rs`, `lib.rs`, `rules.rs`), `renyi_index`
    (`load_project`), the binary (`compile_sources`, the judges'
    `PROGRAM_DIRECTORIES`), `renyi_vm` (`natives/json.rs` re-exports,
    `Bytes.sha256`), `library/std/prelude.ry` and the library sketch,
    `compiler/project.ry`, `declare.ry`, `bodies.ry`, `checker.ry` and
    `compile.ry`, the fixture `tests/conformance/packages/` with
    conformance cases 43 to 47 and `renyi_package/tests/fixture.rs`,
    the reference (sections 2 and 11, appendix A), the decisions
    (section AC), `CLAUDE.md`, `docs/GAPS.md`,
    `tests/conformance/README.md`, `.gitignore`, this file.
15. Packages, the second commit (decision AC1, the section "Packages"
    above): `crates/renyi_package` (`select.rs`, `Project::of_directory`,
    `resolve_in`), `crates/renyi/src/packages.rs` (the five commands) and
    `main.rs` (the dispatch, the usage, the run manifest's dependencies,
    `reproduce`, the budgets of the manifest), `renyi_vm/recording.rs`
    (`Dependency`), `renyi_index` (`package` on modules and definitions,
    the canonical copies keep the tag), `maps.rs`, `budgets.rs`,
    `crates/renyi/tests/packages.rs`, the reference (appendix B),
    `README.md`, `07-system-design.md` (2.3, 3, 6),
    `05-agent-tooling.md`, `CLAUDE.md`, `docs/GAPS.md`, this file.
16. The diagnostics of the front end in Renyi (decision AD1, the
    section "The diagnostics of the front end in Renyi" above):
    `compiler/lexer.ry` (rewritten around `Lexed`), `parser.ry`
    (rewritten around `Cursor.diagnostics` and `Halt`), `parse.ry`,
    `tokens.ry`, `project.ry` (`Front`), `checker.ry`, `report.ry`
    (`json_string`), the three judges of `selfhost.rs`, conformance
    cases 48 and 49, the decisions (section AD), `CLAUDE.md`,
    `README.md`, `docs/GAPS.md`, this file.
17. `std.process` (decision AE1, the section "The process module"
    above): `library/std/process.ry`, section 13 of the library
    sketch, `natives/process.rs`, the two failure arms of `vm.rs`,
    `effects.rs` and `effects.ry` (`foreign` alone unavailable), the
    library lists of `lib.rs` and `project.ry`, `tests/library.rs`,
    `tests/rules.rs`, `crates/renyi/tests/process.rs`, conformance cases
    20 and 50, the
    reference (the module list, the capability table, the static rule,
    the run-time errors), the cheat sheet (`process` and the module
    back on the sheet, within the budget), the decisions (section AE),
    `CLAUDE.md`, `README.md`, `docs/GAPS.md`, this file.
18. The foreign function interface (decision AF1, the section "The
    foreign function interface" above): `library/std/foreign.ry`,
    section 14 of the library sketch, `crates/renyi_check/src/foreign.rs`
    and the three diagnostics of `world.rs`, the `foreign` section of
    `manifest.rs` and the tagging of `resolve.rs`, `SourceFile.foreign`,
    `FunctionMeta.foreign` and `FORMAT` 2 of the bytecode file,
    `natives/foreign.rs` with the generated `foreign_abi.rs` and
    `tools/gen_foreign_abi.py`, `Vm::run_primitive`, `bind.rs`, the run
    notice, the `publish` refusal, `capability-unavailable` removed; the
    mirrors in `project.ry`, `declare.ry`, `effects.ry`, `bodies.ry`,
    `bytecode.ry` and `emit.ry`; `crates/renyi/tests/foreign.rs`, the
    fixtures `tests/conformance/foreign/` and `foreign_bad/` with cases
    20 and 51, `tests/fixture.rs`, `tests/library.rs`, `tests/rules.rs`,
    the judges' directories; the reference (the module list, the
    manifest, the capability table, the static rules, the run time,
    appendices A and B), the library sketch, the cheat sheet, the
    decisions (section AF), `CLAUDE.md`, `README.md`, `docs/GAPS.md`,
    the conformance `README.md`, this file.

19. the Python bridge (decisions AJ2, AJ3 and AL1 to AL4; the section
    "The Python bridge as it exists"): `library/std/python.ry`,
    `crates/renyi_check/src/python.rs` and the two diagnostics, the
    `python` section of `manifest.rs` and the tagging of `resolve.rs`,
    `SourceFile.python`, `FunctionMeta.python` and `FORMAT` 3 of the
    bytecode file, `natives/python.rs` with `python_worker.py`,
    `Vm::run_primitive`, the run notice, the `publish` refusal; the
    mirrors in `project.ry`, `declare.ry`, `effects.ry`, `bytecode.ry`
    and `emit.ry`; `crates/renyi/tests/python.rs`, the fixtures
    `tests/conformance/python/` and `python_bad/` with cases 21 and 53,
    `tests/fixture.rs`, `tests/library.rs`, `tests/rules.rs`,
    `tests/file.rs` and `tests/compile.rs` (the format numbers), the
    judges' directories, `ci.yml` (Python before `cargo test`); the
    guide `docs/python.md`, the reference, the library sketch, the
    syntax sketch, the runtime and system documents, the cheat sheet
    and its two copies, the decisions (section AL), `CLAUDE.md`,
    `README.md`, the conformance `README.md`, this file.

20. the Python binder (decisions AM1 and AM2; the section "The Python
    binder as it exists"): `--python` in `crates/renyi/src/bind.rs` with
    the shared `write_module`, `bind/python.rs` and
    `bind/python_inspect.py`, `interpreters` public in
    `natives/python.rs`, `renyi_json` a dependency of `renyi`, the usage
    text; the unit tests and the binary test in `tests/python.rs`; the
    decisions (section AM), the reference, `docs/python.md`, the library
    sketch, `README.md`, `CLAUDE.md`, this file.
21. the resident world (decision AN1; the section "The resident world
    as it exists"): `crates/renyi_workspace/` (`src/lib.rs`,
    `tests/refresh.rs`), `check::check_item` and
    `check::module_purpose_diagnostic` with
    `check_module_with_references` written over them,
    `module_name_mismatch` public, `index_checked` and
    `canonical_files` public with every tag carried through, `renyi
    mcp` on a `Workspace`; the workspace manifest and `Cargo.lock`; the
    decisions (section AN, with AN2 for the LSP),
    `05-agent-tooling.md` (R5-5 closed), `docs/GAPS.md`,
    `docs/RELEASE.md` (the publish order), `CLAUDE.md`, this file.
22. the language server (decisions AN2 and AN3; the section "The
    language server as it exists"): `crates/renyi/src/lsp.rs` and
    `lsp/describe.rs`, the `lsp` command, `tests/lsp.rs`; the
    workspace's as-written mode, lazy map and accessors with their
    test; `function_signature` and `type_signature` public in
    `renyi_index`, the MCP server's JSON helpers shared; the VS Code
    client (`editors/vscode/src/extension.js`, `package.json`,
    `package-lock.json`, `.vscodeignore`, the README and the
    changelog), the client's build in CI and in the release workflow,
    `.gitignore`; the decisions (AN3), the reference (appendix B),
    `README.md`, `docs/RELEASE.md`, `docs/GAPS.md`, `CLAUDE.md`, this
    file.
23. the watch of `serve` (decision AO1; the section "The watch of
    `serve` as it exists"): `crates/renyi/src/serve.rs`, the `serve`
    command with `--watch` among the flags of `run` (`exit_of` shared),
    `tests/serve.rs`; `Options.watch` and `Options.listener`,
    `Interrupt::Reload`, `RunOutcome::Reload`, `Run.listener` and the
    polling `serve` native in `crates/renyi_vm`; the decisions (section
    AO), the reference (appendix B), `07-system-design.md` (section 5
    as implemented, R7-3 closed), `docs/GAPS.md`, `README.md`,
    `CLAUDE.md`, this file.
24. the embedding API (decisions AP1 and AP2; the section "The
    embedding API as it exists"): `crates/renyi/src/sandbox.rs`
    (`Grant`, `Sandbox`, `CallError`, `Function`, `--sandbox` for
    `run`, `record` and `serve`), `Allocator` in the binary,
    `compile_file`, `tests/sandbox.rs`; `crates/renyi_vm/src/memory.rs`,
    `Options.memory`, `Interrupt::OverMemory`, `RunOutcome::OverMemory`,
    `Narrowing.sandbox`, `Vm::budgets` and `resume_budgets`;
    `renyi_syntax::parse_grant`; the guide `docs/embedding.md`, the
    decisions (section AP), the reference (appendix B),
    `07-system-design.md` (section 4 as implemented, R7-1 closed),
    `docs/GAPS.md`, `README.md`, `docs/extensions.md` (the allocator
    line), `docs/index.md` and `tools/site.py` (the guide in the
    navigation), `CLAUDE.md`, this file.
25. `44763c4` the profile-guided round on strings and JSON, paused by
    the owner to finish it elsewhere: the change as the script
    `tools/apply_perf_round.py` (applied, judged and deleted in session
    9, decision AQ), the two profiles and the numbers to beat in this
    file.

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

- **Release 0.1.0 is out** (2026-10-07). Left to the owner: the
  announcement with the texts of `08-positioning.md` section 6; the
  VS Code Marketplace publisher `renyi-lang` and `vsce publish` from
  `editors/vscode/` if the extension is to be found by search (the
  `.vsix` on the release page installs by hand). The domain
  renyi-lang.org is in front of the site (AI4, 2026-10-07, late):
  the zone is on Cloudflare, its records name GitHub Pages, the
  Pages setting names the domain and HTTPS is enforced; the session
  did it with a one-hour token minted from a credential of the
  owner's that stays on the owner's machine, outside the repository
  (`docs/RELEASE.md` section 5, item 8).
- **Updates and installers** (the owner's question of 2026-10-07,
  evening): the one-line installers overwrite the binary in place and
  `RENYI_VERSION` pins a version; `cargo install renyi` upgrades in
  place too; there is no package-manager manifest (winget, scoop,
  Homebrew) and no update check in the binary. The session's advice,
  not decided: never an automatic check (the toolchain of a language
  whose point is no undeclared effect should not phone home); an
  explicit `renyi upgrade` command and the three manifests are
  candidates after 0.1, for the owner to order.
- **Decision AI5** (the owner's answer at the end of session 8): the
  two crates that embedded files from outside their directories now
  embed copies held equal by tests; nothing remains for the owner on
  it.
- **The residue of stage 1** is done (decisions Y1 to Y4), and open item
  R3-2 (constraints with type arguments) is decision AB1; `docs/GAPS.md`
  section 7 keeps 1.11, 3.4 and 3.6 open.
- **The rename of `set` to `change`** (the owner's decision of
  2026-10-06, asked back as a structured question after the owner raised
  it) is done: decision AA1 and the section "The rename of `set` to
  `change`" above.
- **Stage 2's toolchain in Renyi is complete** (the lexer, the parser,
  the checker and the emitter, W1 to W8 and Z3), R3-2 is decided (AB1)
  and the judges run the front end from its bytecode (the section "The
  judges run from bytecode"): the next piece is the owner's to choose
  between the remaining performance items and stage 3 (M4), which is
  done since: packages, `std.process`, the FFI.
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

1. **After the bridge and the binder** (decisions AJ2, AJ3, AL1 to
   AL4, AM1 and AM2 are in; the sections above): pending from the
   release, the announcement (owner), the three measurements of the
   positioning's section 5 on the starter pack, the Marketplace if
   wanted, and the update candidates above (`renyi upgrade`, the
   package-manager manifests), in the order the owner sets; then M5
   (item 5). Candidates around the bridge, none decided: record types
   from dataclasses and `TypedDict`s in the binder; a deadline per
   call; one worker across the tests of a `renyi test` run instead of
   one per VM; keyword arguments by parameter name instead of
   position; how the checker written in Renyi (`compiler/project.ry`,
   a fixed list of the fifteen standard modules under `--library`)
   sees extension modules, which the judges need once the corpus has a
   program that imports one. The behaviour of `renyi tools` on a
   directory of declaration files (it fails on them as programs;
   `renyi index` counts their errors) is settled: the owner struck it
   from this list on 2026-10-07, the files are not programs.
2. **Stage 2 of `docs/GAPS.md`, section 7, continued** (the grammar is
   frozen, V11; the formal grammar is `docs/grammar.ebnf`, V12; the
   language reference is `docs/reference.md`; the lexer and the parser
   in Renyi are `compiler/`, W1 to W4; the checker in Renyi is
   `compiler/declare.ry`, `bodies.ry` and `checker.ry`, W5 to W8; the
   profile and the loop are done, X1 to X6; the residue of stage 1 is
   done, Y1 to Y4; the bytecode file, `renyi compile` and the loader
   are done, Z1 to Z4; the rename of `set` to `change` is done, AA1;
   the emitter written in Renyi is done and judged, Z3; constraints
   with type arguments are done, AB1, which closes R3-2): the toolchain
   in Renyi is complete with the Rust toolchain as stage 0, and the
   judges run the front end from its bytecode. Next, in the order the
   owner chooses: the remaining performance item (the string building
   of `change out to "{out}{piece}"`, which copies the accumulated text
   per piece; the pattern cache of `Text.matches` is done, decision AQ)
   as a later profile calls for it; any change to the loop is measured with `bench.py`
   and `micro.py` against the `base` worktree, the two binaries run
   back to back. The front end in Renyi reports the Rust parser's
   diagnostics (codes, fixes, recovery; decision AD1) and the judges
   are byte-equal on rejected programs too, the step the owner chose
   beside stage 3 ("1+2", 2026-10-07).
3. **Readability, only on request**: round 5 on Sonnet measures U9
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
4. **Readability observations not asked**: `otherwise` binds loosest
   (Sonnet wrote `check f(x) otherwise "" is "y"`); Sonnet reasons before
   the Predict answer on `traffic_light` (0 of 5 with right lines); the
   rubric's reading of "only X and Y" when Z is also needed (round 1 and
   2 read it as a need missing).
5. **M4**: its three slices (packages, decision AC1; `std.process`,
   decision AE1; the FFI, decision AF1; the sections above) are done.
   **M5**, in the order the owner set on 2026-10-07: the resident
   `World` is done (decision AN1; open item R5-5 closed) and the
   language server is done (decisions AN2 and AN3: `renyi lsp` with
   diagnostics, hover, definition and the outline, the client in the
   VS Code extension) and the watch of `serve` is done (decision AO1:
   `renyi serve --watch` runs `main` again between two requests on a
   clean new version, the socket kept open, on the same resident
   world) and the embedding API is done (decisions AP1 and AP2:
   `renyi::Sandbox`, the grant as a `needs` clause with the memory
   budget, `renyi run --sandbox`): M5 is complete. Next, as the owner
   set on 2026-10-07, the interspersed items below, and later the C
   API of decision A5 over the Rust one. Candidates for the server,
   none decided: completion (names in
   scope, the library's), references, rename, formatting through the
   editor, a `renyi.path` prompt when the binary is missing. Interspersed, as the owner asked the same day: the performance
   items by the profile of AG5 (the pattern cache of `Text.matches`,
   the string building, the JSON path; strings and JSON to CPython's
   speed, the compiler's self-check twice as fast): the first round is
   done, decision AQ (the section "The profile-guided round on strings
   and JSON as it exists" below, with the numbers against the targets
   and what it left); then the baseline JIT (AG5, item iii), decided as
   AR1 and under way in three stages (the section "The baseline JIT"
   below: stage 1, `LoadField`, and stage 2, direct calls with
   register arguments, are in; stage 3, the pinned layout of `Value`
   with the value operations inline, is next); and a showcase, a bookmark and reading-list web app
   in a repository of its own under `renyi-lang/` (HTTP for the titles,
   SQLite, an HTML page and a JSON API, tags and search, CSV export,
   one use of the Python bridge, `replays` tests).

## Known gaps and risks

- **The embedding API.** The console output of a sandboxed call goes
  to the process's streams; the memory count is the process's (a
  multi-threaded host's other threads count during a budgeted call, and
  one budget is in force at a time); a bytecode file cannot be loaded
  into a sandbox; each call sets up a VM; the counting allocator's
  cost on the benchmarks is not measured; the `Value` type a host
  builds arguments with is `renyi_vm`'s, re-exported, with no
  conversion from JSON yet (the C API will need one).
- **The watch of `serve`.** A reload runs `main` again from the top,
  so a service that does work before `server.serve` repeats it on
  every reload; a file imported from outside the project's directory
  is not stat-ed, so a change to it alone is not noticed until a file
  of the directory changes; a change to an unrelated file of the
  directory costs a compile; a reload waits for the request in flight;
  the listener is polled every 20 ms under the watch, a cost a service
  without `--watch` does not pay.
- **The language server.** The VS Code client has only been bundled
  here, not run in a window: the first real session will show whether
  `vscode-languageclient` 9 and the server agree on every detail (the
  settings reload, the warning when the binary is missing); the server
  checks after every keystroke with no debounce (fine at the measured
  speed, to be watched on a large folder); the positions assume the
  UTF-16 default and do not negotiate `positionEncoding`.
- **The resident world.** The manifest is watched at the served
  directory's root only; the resolver's problems are not in the map
  (as before); a dependency's files are read again only when a
  declaration or the manifest changed; the stamp of a file written
  within two seconds of a refresh is not trusted, so an editor that
  saves every second makes every refresh read that file (the text is
  compared, not parsed).
- **The binder.** `renyi bind --python` imports the package, so the
  package's top-level code runs on the binder's machine, as it does
  for any Python tool that imports it; the signature in a purpose is
  the interpreter's rendering (Python 3.14 prints `Optional[int]` as
  `int | None`), so a generated file differs in its purposes across
  versions; a class, a dataclass or a `TypedDict` is `JsonValue`, no
  record type is generated; classes, methods and constants are not
  declared; a default is not carried, the Renyi caller passes every
  positional parameter.
- **The bridge.** The worker's standard error is inherited from the
  VM's process, not routed through `Options::stderr`, so a harness
  that captures the VM's standard error does not see what Python
  prints; a `Decimal` crosses as a JSON number, which Python reads as
  a float; a Python function that blocks (reads its standard input,
  waits on a socket) blocks the call with no deadline; the candidates
  of decision AL3 are tried in order, and a stub that prints and exits
  (the Windows Store's `python3`) costs a message on the standard
  error before the next candidate answers; the `tests/python.rs` tests
  and the two conformance cases need a Python 3 on the PATH.
- **Extensions and the checker in Renyi.** `compiler/project.ry`
  reads the fifteen standard modules by name from `--library`; an
  extension's modules are invisible to it, so the self-hosted judges
  cannot compare a program that imports one (none does today). A
  bytecode file compiled with an extension runs on a binary without
  it until the first call, which crashes as a library function this
  build does not implement, rather than being refused at load.
- **Three copies to keep.** `crates/renyi_check/library/std/` and
  `crates/renyi/cheatsheet.md` are copies (decision AI5) that a test
  in each crate holds equal to `library/std/` and `docs/cheatsheet.md`,
  and `starter/skill/renyi/cheatsheet.md` (decision AI3) is one that
  CI's `cmp` holds equal to the document; an edit to the canonical
  file without the copies fails `cargo test` or CI, which is the
  point, but the failure names the file to copy rather than copying
  it.
- **The site is checked as text, not in a browser**: the live pages
  were fetched over HTTP (status, size, titles, the example links,
  the build stamp) and the generator's output inspected as text; no
  one has looked at the rendering. The `markdown` package renders the
  documents' tables and fenced blocks; nothing highlights Renyi.
- **The FFI.** `natives/foreign.rs` and `foreign_abi.rs` are the VM's
  only unsafe code: a declaration that does not match the C side (a
  width, a missing length, a pointer the function keeps) is undefined
  behaviour in C's sense, not a Renyi crash; the signature family stops
  at six words and at integer-class and double-class scalars (no
  `float`, no struct by value, no callback); the libraries are named by
  the manifest per platform, so a project that binds `libc` names three
  names. `renyi format` does not read a foreign module. Nothing in the
  tests calls a library other than the platform's C library.
- **The front end in Renyi.** Its diagnostics are a transcription of
  the Rust lexer's and parser's (decision AD1): a new error site, a
  changed message or fix in `lexer.rs` or `parser.rs` is the same
  change in `lexer.ry` or `parser.ry` in the same commit, or the judges
  fail; the probes that exercise the error sites live in the lane, not
  in the repository (two conformance cases hold a sample).
  Its equality with the Rust parser is exact on every program in the
  repository, by construction of its `peek` (the Rust parser's
  side-effecting peek is simulated, line-break spans included); a new
  layout rule in `parser.rs` must be mirrored in `parser.ry` or the judge
  fails. The run time grows with the file (0.6 s for `parser.ry` in the
  release build after X3; the profile above says where it goes); the
  `"{text}{ch}"` concatenation in the lexer's `scan_segment` is
  quadratic in a token's length (negligible for tokens, visible on a
  long block text; `slice` is one `List.slice` since X2). `json.rs` and
  `ast.ry` must change together (W1); nothing checks that the Rust
  encoder's keys match `ast.ry` except the judge's byte comparison.
- **`std.process`.** One call per run, by decision AE1: no handle, no
  interactive reading or writing, no background process, and the
  child's whole output is held in memory. A limited run polls every
  5 ms. The program is found as `std::process::Command` finds it: on
  Windows a `.exe` on the `PATH`, not a `.cmd` or `.bat`. The scope is
  the program text for text, so `git` and `/usr/bin/git` are two
  grants. A program a signal ended reports 128 plus the signal.
- **The checker in Renyi.** It is a transcription: a change to a
  message, a fix, a rule or the order of checks in `crates/renyi_check`
  is a change to `declare.ry` or `bodies.ry` in the same commit, or the
  judge fails (CLAUDE.md says so). The by-construction differences
  listed above (hash-map ties, the NUL in a `Path`, `debug_quoted`, the
  import path separator) show on no program in the repository and
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
  reproduces through `bodies.debug_quoted` (quotes, backslashes, `\n`
  and `\t` escaped; other escapes do not occur in the corpus). The
  Renyi emitter's whitespace, when it puts an example's or a condition's
  text on one line, is what `Text.trim` strips (Rust's
  `char::is_whitespace`, as `split_whitespace` uses). A `.ryc` is tied to the toolchain that wrote
  it by nothing but `format`; a change to `Op`, `Program` or the
  format types bumps `FORMAT` and `compiler/bytecode.ry` together.
- **VM.** A declared `equals` decides `is` and `is not` (session 7);
  `contains`, `index_of`, sets and maps keep the derived form, and a
  user `Hash` implementation is never called, by decision Y3 and the
  reference's section 5. A
  value's `IsType` test for a builtin (`when failure(error: Text)`) is by
  kind. `random` is seeded from the clock; a run is reproducible through
  its recording only. `Float.to_text` is Rust's shortest round-trip form.
  `Text.matches` and `std.regex` keep a compiled pattern per thread (256
  at most, then the cache is emptied; decision AQ). Durations print as
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
  peer sessions and ask before editing a shared file. In session 9 the
  owner moved from the local machine to the cloud environment
  (claude.ai/code), whose session names a branch of its own
  (`claude/...`); asked, the owner kept the rule: the commit goes to
  `main`, and the session's branch is deleted. The cloud environment
  has rustc 1.97.0; the CI toolchain 1.94.1 is installed beside it
  (`rustup toolchain install 1.94.1 -c clippy -c rustfmt`) for the
  gates, with a target directory of its own.
- The pay-per-token API keys are not spent by default (ruled 2026-10-06):
  subscription quota first (Claude Code subagents, the Codex CLI), the
  keys only when the owner says so in the same request, with the volume
  named. The readability harness drives the two CLIs itself (`run
  --provider claude`, `--provider codex`) and takes grades from the
  subagents' files (`grades.json` buckets) for that reason.
