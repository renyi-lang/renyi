# Handoff

Last updated: 2026-10-06, session 6 (the VM: `renyi run` and `renyi test`;
the primitive boundary: `renyi record`, `run --replay`, `--explain`,
`replays` tests, budgets and scope checks; the HTTP client, the HTTP
server and SQLite; the grant stack; recordings for every network and
database program, `--redact`; the run manifest and `renyi reproduce`;
decision L2 carried out, the VM judging Complete and Write; terminals
after `group by` per group; `renyi mcp`, the toolchain for agent hosts;
the semantic diff, `renyi index --diff` and the `diff` tool; readability
round 2 collected, graded and written up, its four questions answered as
decisions U1 to U4 and applied; round 1 re-graded on the subagent
scale; decisions U5 to U7 from the re-grade).
Branch: `main` is the only branch (owner's decision, 2026-10-05); commit
and push there directly.

## Where the project stands

Milestones M0 (design), M1 (front end) and M2 (type and effect checker) are
done; the project map (`renyi index`, decision O2) exists with its budget
report (`--budgets`, decisions O3 and R7) and its semantic diff (`--diff`,
decision O4, T5); M3 (the VM) runs programs with every module of the
standard library and implements decisions P1 and P2 (recorded runs,
budgets, the grant stack of Q1, the run manifest and `renyi reproduce` of
Q2) at its primitive boundary, which completes M3 (tasks run one after the
other by decision S2); `renyi mcp` (decision O5, T1 to T5) serves all of
it to an agent host. Design decisions are in sections 0 to U of
`01-decisions.md`; the agent tooling in `05-agent-tooling.md`, the
signature capabilities in `06-runtime-guarantees.md`, the system-level
commitments in `07-system-design.md`. The corpus has 30 programs, passes
the lint, is in canonical layout, checks cleanly, has nothing over
budget, and its 89 `example:` lines and `test` blocks pass on the VM (six
are `replays` tests answered from recordings under `examples/fixtures/`,
two of them hand-written). The cheat sheet measures 2994 of 3000 tokens
(decisions U1 to U3: the one-argument sentence, every library method
with its parentheses, `rounded(places: 2)`). The Rust workspace has five crates:
`renyi_syntax`, `renyi_check`, `renyi_index`, `renyi_vm` and the `renyi`
binary with `check`, `format`, `tokens`, `parse [--json]`, `index [--json
| --budgets | --diff <map or revision>]`, `run [--manifest] [options]
<file> [arguments]`, `record [--to file] [options] <file> [arguments]`,
`reproduce <recording> [<file>]`, `test [--strict] [--refresh name
[--redact name]] [--explain] <file>...`, `mcp [path]` and `version`; 142
tests, clippy and fmt clean on Windows. The VM depends on `ureq` (HTTP,
with rustls) and `rusqlite` (SQLite compiled in), decision S1, and on
`sha2` for the manifest and the hashes.

Two live readability rounds exist, both with Explain graded by Claude
Code subagents (decision U4; round 1's API-graded tally is kept beside in
`scores-api.json`). Round 1 (`tests/readability/2026-10-05-1623155/`,
session 5, three models through the vendor APIs, Explain re-graded in
session 6) gave decisions R1 to R8 and stands at Sonnet 5.5 Predict 90,
Explain 97, Complete 79, Write 40; Haiku 4.5 30, 90, 37, 0; gpt-5.4-mini
80, 97, 32, 0. Round 2 (`tests/readability/2026-10-05-ed37120/`, session
6, the four models of R8 on the cheat sheet after R1 to R8) gives, by the
four-of-five rule with the VM judging Complete and Write: Sonnet 5.5 90,
100, 89, 30; gpt-5.5 100, 100, 74, 10; Haiku 4.5 50, 87, 42, 0;
gpt-5.4-mini 80, 97, 37, 0 (thresholds 90, 90, 80, 70 on the two gating
models). Sonnet passes Complete; neither gating model passes Write, and
gpt-5.5 misses Complete by one item. The grammar is not frozen. The four
questions round 2 raised (decision M4 again, `rounded()` without
`places`, zero-argument methods written as fields, the Explain grader
kind) were answered by the owner in session 6 as decisions U1 to U4 and
applied: the cheat sheet changed, the grammar and the library did not.
The re-grade raised three questions, answered the same session as
decisions U5 (a hand verdict extends to every sample repeating the
sentence it called wrong; fourteen verdicts added across the rounds), U6
(the five-point rule names the gating models; Haiku's Explain moved 90
to 87 on wrong statements about the programs) and U7 (round 3's samples
through the subscription channels).

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
  from `bind`, the handler called through `Vm::call_function` with a
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
  `FileError.PermissionDenied` or `HttpError.HostNotAllowed`, past a budget
  with `OverBudget`, and crashes when the primitive cannot fail. Live, the
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

- **None open from the readability work**: the re-grade's three
  questions were answered in session 6 as decisions U5 to U7 and applied.
  Round 3 needs the owner's go-ahead on its volume before it starts
  (about 1,380 samples for four models, one subagent or Codex session
  each; see Next steps).
- Session 5 printed the values of `ANTHROPIC_API_KEY` and
  `OPENAI_API_KEY` into a tool result once (not committed); the owner said
  in session 6 that they handle the transcript and the keys themselves, so
  no session needs to raise it again.

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

1. **Round 3 on the cheat sheet after U1 to U3**: `run.py prepare`, then
   `run` through the subscription channels (decision U7): Claude Code
   subagents for the Claude models and `codex exec` for the OpenAI
   models, fed to the harness with `run --provider file` (round 2's
   gpt-5.5 Write samples came that way; Predict, Explain and Complete at
   five samples per prompt would be new ground for it: 69 prompts, 345
   samples per model, 1,380 for four models, one subagent or Codex
   session each; round 2's 45 Codex samples took about an hour). Ask the
   owner before starting that volume. Explain graded by subagents and
   merged with `tests/readability/merge_agent_grades.py`, Complete and
   Write by the VM. It measures U1 to U3 and the five-point rule on the
   gating models (U6).
2. **Readability observations not asked**: `otherwise` binds loosest
   (Sonnet wrote `check f(x) otherwise "" is "y"`); Sonnet reasons before
   the Predict answer on `traffic_light` (0 of 5 with right lines); the
   rubric's reading of "only X and Y" when Z is also needed (round 1 and
   2 read it as a need missing).
3. **M4** (provenance guards `only to`, the package manager, budgets in
   the manifest) and **M5** (embedding API, `serve --watch`, LSP) as
   before; open items R5-5 (per-definition refresh in `renyi mcp`) and
   R7-4.

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
  `example:` literal check, no `see also` check, no `deprecated` warnings,
  no unreachable-pattern detection). The `ignore-pure` rule counts a call
  with effects anywhere inside the ignored expression. The condition
  bodies bind every field of the record or variant, which the sketch
  (section 4: "over the field name") does not promise; a condition that
  reads another field type-checks and runs.
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
  reasoning effort `medium`), the rest through the API. The grant clauses
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
  reviews. `main` is the only branch.
- The pay-per-token API keys are not spent by default (ruled 2026-10-06):
  subscription quota first (Claude Code subagents, the Codex CLI), the
  keys only when the owner says so in the same request, with the volume
  named. The readability harness takes samples and grades from files
  (`run --provider file`, `grades.json` buckets) for that reason.
