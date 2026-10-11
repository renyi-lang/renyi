# Roadmap

The stages of the project from its first design document to today, in
the order the work happened, each with what it built, the decisions
that settled it, how it was measured and where it stands; then the same
stages grouped by track, and what comes next. `docs/HANDOFF.md` is the
record of each session and `docs/design/01-decisions.md` the reasons;
this file is the map, rewritten when a stage closes (CLAUDE.md asks the
session that closes one to update it). Every stage was written by
Claude in sessions and reviewed by the owner, who takes the decisions;
`main` is the only branch, so the tracks below are lines of work, not
branches of the repository.

## 1. The milestones

| Milestone | Content | Status |
|-----------|---------|--------|
| M0 | the design: positioning, types, syntax, the constraints for LLM readers and writers, the runtime, interoperability, modules, the implementation language (decisions 0 to J); the library (K); the readability protocol (L) | done (stages 1 and 2) |
| M1 | the front end in Rust: lexer, parser, formatter, `renyi check`, `format`, `tokens`, `parse` | done (stage 3) |
| M2 | the type and effect checker, every diagnostic with a fix | done (stage 4) |
| M3 | the VM: `run`, `record`, `run --replay`, `reproduce`, `--explain`, `--profile`, `test` with `replays`, budgets, the grant stack, the run manifest, every library module | done (stage 7) |
| M4 | the library and the package manager: packages, `std.process`, the foreign function interface | done (stage 15) |
| M5 | the integration: the registration API, the Python bridge and binder, the resident world, the language server, `serve --watch`, the embedding API | done (stages 18 and 19) |
| M6 | ahead-of-time code: machine code for the bytecode, the baseline JIT, the template tier, `renyi build` images and self-contained executables | the machine code exists (stages 16, 21, 22, 24, 26); no WASM or C API target |
| Self-hosting | the compiler written in Renyi (lexer, parser, checker, emitter) held equal to the Rust toolchain by the judges; the binary switches to it when it is within 3 times the Rust front end's time (decision AU35 ii) | the compiler exists and is judged (stages 11, 12, 14, 15); the speed round is under way (stages 25 to 27) |

## 2. The stages, in order

Each entry: the sessions, the decisions, what was built, the measure
where one decided, the status. The dates are those of the decisions.

1. **The design, round 1** (session 1, M0). The eight design documents
   under `docs/design/`: the tensions and the trade-offs, the
   positioning, types and strictness, the syntax, the constraints that
   serve LLM readers and writers, the runtime, interoperability,
   modules and the library, the implementation (decisions 0 to I: Rust,
   one binary named `renyi`, every document in English); the example
   corpus `examples/`, the cheat sheet under a budget of 3000 tokens
   (`tools/count_tokens.py`), the corpus lint (`tools/lint_examples.py`).
   Done.
2. **The design, round 2, the library and the readability protocol**
   (session 2). Decisions J (the second round), K (the standard library:
   the prelude, the core modules, the extension packages, as
   declaration files under `library/std/`), L (the readability test:
   Predict, Explain, Complete, Write, graded by models). Done.
3. **The front end in Rust, M1, and the pre-test round** (sessions 3
   and 4). `crates/renyi_syntax`: spans, diagnostics, the lexer, the
   AST, the parser, the JSON encoder, the formatter; `renyi check`,
   `format`, `tokens`, `parse`; the corpus tests; decisions M from the
   subagent pre-test. Done.
4. **The type and effect checker, M2** (session 4). `crates/renyi_check`:
   the world of declarations, the bodies, the effects, the refinements
   on literals, the fixes (the closest name, the Renyi spelling of a
   foreign name, the prelude's conversions); decisions N. Done.
5. **The runtime design, the agent tooling and the system-level
   commitments** (session 4). Decisions O (the project map `renyi
   index`, metrics, content hashes, budgets, the semantic diff; the
   narrated run), P (signature capabilities: `needs`, `at most`
   budgets, `only to` provenance guards), Q (the grant stack, the run
   manifest); `crates/renyi_index`. Done.
6. **The first live readability round** (session 5, 2026-10-05).
   `tests/readability/`: the harness, the manifest, the Explain
   references, the Write tasks; decisions R1 to R8 from the round. Done;
   later rounds only on request (decision V1).
7. **The VM, M3** (session 6). `crates/renyi_vm`: the bytecode and its
   compiler from the checked tree, the interpreter, the primitive
   boundary (grants, budgets, recording, replay, narration), the library
   natives (HTTP through `ureq`, the server, SQLite, decision S1), tasks
   one after the other (S2); `renyi run`, `record`, `run --replay`,
   `reproduce`, `test` with `replays`; the MCP server `renyi mcp`
   (decisions T). Done.
8. **Readability round 2, rounds 3 and 4** (session 6, 2026-10-06).
   Decisions U1 to U9; round 4 on Sonnet through the Claude Code CLI:
   Predict 90, Explain 100, Complete 79, Write 40. Done; the scores are
   no longer the gate (V1).
9. **The gap audit and the road to self-hosting** (session 6,
   2026-10-06). `docs/GAPS.md`: what the design promised and the
   implementation did not deliver, with the order of work; decisions V1
   (the aim: a self-hosted language people use), V5 to V7 (the limits
   the checker enforces), V8 (CI and the conformance suite), V11 (the
   surface frozen by decision at commit `dc58bb3`), V12 (the formal
   grammar to come). Done.
10. **Stage 1 of the gap audit** (session 7, 2026-10-06). Fourteen
    commits: the checker enforces V5 to V7, `renyi tools` (D6), the
    `only to` guards run, CI on every push with `tests/conformance/`,
    the runtime's promises (deadlines, refined updates, scopes,
    `equals`, the Float range), the module rules, the readability
    references as conformance cases. Done.
11. **Stage 2: the grammar, the reference and the front end written in
    Renyi** (session 8). `docs/grammar.ebnf` (W3C EBNF, interpreted by
    a test against the parser), `docs/reference.md` (normative, held to
    the grammar and the crates by a test, the sketch retired to the
    design record); `compiler/ast.ry`, `lexer.ry`, `parser.ry`,
    `parse.ry`, `tokens.ry` (decisions W1 to W4), held byte-equal to
    `renyi parse --json` by `crates/renyi/tests/selfhost.rs`. Done.
12. **The checker written in Renyi** (session 8). `compiler/declare.ry`,
    `bodies.ry`, `checker.ry` and their helpers (decisions W5 to W8),
    held byte-equal to `renyi check --json`, with and without
    `--strict`, on every program and exiting as it exits. Done.
13. **The profile and the loop of the VM** (session 8). Decisions X1 to
    X6: the development profile optimizes the VM, `List.slice`, the
    interpreter loop rewritten, `renyi run --profile`, the emitter
    writes a bytecode file, mimalloc. Done.
14. **The residue of stage 1, the bytecode file, the rename and the
    emitter in Renyi** (session 8). Decisions Y1 to Y4; the bytecode
    file `.ryc` with `renyi compile` and the loader (Z1, Z2, Z4);
    `set` renamed `change` (AA1); the emitter `compiler/emit.ry` and
    `compile.ry`, held byte-equal to `renyi compile` (Z3); constraints
    with type arguments (AB1, open item R3-2 closed); the judges run
    the front end from its own bytecode. Done: the toolchain in Renyi
    is complete, with the Rust toolchain as stage 0.
15. **Stage 3: packages, the diagnostics in Renyi, the process module,
    the FFI** (session 8, 2026-10-07; M4). `crates/renyi_package` and
    `renyi add`, `update`, `audit`, `fetch`, `publish` with the effect
    manifests (AC1, in both front ends); the Rust lexer's and parser's
    codes, fixes and recovery in the Renyi front end, the judges equal
    on rejected programs (AD1); `std.process` (AE1); `renyi bind` for a
    C header and the boundary's rules (AF1). Done.
16. **Machine code for the bytecode** (session 8). `crates/renyi_vm/src/
    native/`: the abstract interpretation, every op translated to
    Cranelift IR, the hand-back to the interpreter, the hotness rule
    (AG1 to AG5); the bounded VM round (AG6); the benchmarks `bench/`
    with CPython twins and `tools/bench.py` (AG4). Done.
17. **The niche and release 0.1** (session 8, 2026-10-07). The
    positioning (AH1 to AH4, `docs/design/08-positioning.md`: the
    scripting language of AI agents); the release workflow, the
    installers, the VS Code extension, the procedure `docs/RELEASE.md`
    (AI1, AI2), the starter pack for agents `starter/` (AI3), the
    documentation site (`tools/site.py`, GitHub Pages, renyi-lang.org),
    the self-contained crates (AI5); release 0.1.0: the repository
    public under `renyi-lang/renyi`, the seven crates on crates.io, the
    tag `v0.1.0`. Done.
18. **Foreign packages: Rust natives and Python** (session 8). The
    registration API (`Extension`, `Native`, `Registry`; the standard
    library as the first extension; `renyi` a library too, `main_with`;
    `docs/extensions.md`; AJ1, AK1 to AK4); the Python bridge (a module
    bound by the manifest, one worker per run, `PythonError`,
    `docs/python.md`; AL1 to AL4) and the binder `renyi bind --python`
    (AM1, AM2). Done.
19. **M5: the resident world, the language server, the watch of
    `serve`, the embedding API** (session 8, 2026-10-07 and 08).
    `crates/renyi_workspace` held by `renyi mcp` between calls (AN1);
    `renyi lsp` with the client in the VS Code extension (AN2, AN3);
    `renyi serve --watch`, `main` run again between two requests on a
    clean new version with the socket kept (AO1); `renyi::Sandbox`, the
    grant as a `needs` clause with the memory budget, `renyi run
    --sandbox`, `docs/embedding.md` (AP1, AP2). Done: M5 complete.
20. **The profile-guided round on strings and JSON** (session 9,
    2026-10-08, the first session in the cloud environment). Decision
    AQ: the pattern cache of `Text.matches`, the string building, the
    JSON path, measured against the targets set after M5. Done.
21. **The baseline JIT** (session 9). Decisions AR1 to AR6: the fused op
    `LoadField`, direct calls between generated functions with register
    arguments, the layout of `Value` pinned so that the value operations
    run in place, the tiering measured and the hotness factor raised;
    the self-check 15% below where AQ left it. Done.
22. **`renyi build`** (session 9). Decisions AS1 to AS4: the image of a
    program (its bytecode with the machine code of every function),
    which `run`, `record`, `test` and `reproduce` load without
    compiling; `--exe`, the self-contained executable. Done.
23. **The size of the generated code** (session 9, 2026-10-08 and 09).
    Decisions AT1 to AT7: the rule (KCachegrind's estimate of the cycles
    on the compiler's self-check, `tools/measure_size.sh`), the cuts at
    no cost, the image's program in a binary encoding, the static stack
    height, the image's code section mapped from the file, the reference
    counts as calls. Done.
24. **The typed round: the checker's static types brought to the code
    generator** (sessions 9 to 12, 2026-10-09 and 10). Decision AU1 and
    the stages AU2 to AU34: typed calls to primitives without the
    boundary, the loop over a list in the generated code, the typed
    bytecode, the types of parameters, the image cache of `renyi run`
    (AU10), the update of a uniquely held record in place, comparisons
    on borrowed operands, the compile thread, the liveness pass, the
    template tier (a small x86-64 assembler, every code object on
    machine code from its first call; AU18, AU19, a Windows job in CI),
    its direct calls, the representation items (small texts in the
    value, records and variants in one block), the counts in place on
    both tiers, `List.at` in place. The self-check's JIT run fell from
    1,593 ms to about 1,000 on one hardware thread over the round.
    Closed by the owner with its three goals met (AU35).
25. **The front end's round, the Rust side** (sessions 12 and 13,
    2026-10-10). The study (why the compiler written in Renyi takes
    eleven times the Rust front end's time, per operation) and the
    owner's order (AU36); A0, every file parsed once (AU37); A1, token
    comparisons by variant (AU38); A4, the module trees shared and a
    program declaring only the library modules it needs, in both front
    ends (AU39, AU42); the self-check frozen under `bench/selfcheck/`
    (AU41); A2, the cache keyed by the sources: a second run of the
    same program loads the compiled program and runs no front end
    (AU40, AU43, AU44). `renyi check compiler/bodies.ry` from 437
    million instructions and 72 ms to 153 million and 34.5 ms; a hit
    of `hello` -33% in instructions. Done.
26. **The front end's round, the VM side: B5** (session 13, 2026-10-11).
    Two Integers of which at least one is boxed and the checker typed
    so compared in place in both tiers when both are small, the helper
    kept for the rest (AU45): the case of an Integer read from a list,
    a field or a `maybe` against another such or a literal, which a
    lexer over code points compares at every character;
    `bench/micro/scan_codes.ry`, -48% in instructions a turn; the
    Renyi checker 806 to 756 ms on `compiler/bodies.ry`. Done.
27. **The front end's round, the Renyi side** (session 13, 2026-10-11,
    under way). B1, the Renyi lexer rewritten for speed with the
    library as it is: one loop, scans without a call per character,
    the word lists as sets, a step's product a sum; the lexer alone on
    `compiler/bodies.ry` 983 to 514 million instructions (-48%),
    byte-equal by the judges (AU46). Next: B2, the parser's keywords
    as variants without fields and a cursor that builds no record per
    look; B3, small functions inlined on the Cranelift tier (AU36,
    stage 6). The gate:
    the compiler written in Renyi within 3 times the Rust front end's
    time on `compiler/bodies.ry` (10.0 times at AU45 against AU34's
    binary), then the binary switches to it (AU35 ii).

## 3. The tracks

The same stages grouped by the line of work they belong to, with the
state of each line.

- **A. The design and the normative texts**: stages 1, 2, 9 (the
  freeze), 11 (the grammar and the reference), 17 (the positioning).
  The surface is frozen (V11): a change is a new decision entry first,
  then the reference, the grammar, the cheat sheet, the formatter and
  the conformance suite in one commit. Open: R3-1 (`Iterable`),
  section 6 of `docs/GAPS.md`.
- **B. The toolchain in Rust**: stages 3, 4, 5, 7, 10, 13, 14, 22, 25.
  Complete for the language as designed; the Rust front end is the
  binary's until the switch of track C.
- **C. The compiler written in Renyi (self-hosting)**: stages 11, 12,
  14, 15 (AD1), 27. The lexer, the parser, the checker and the emitter
  exist and are judged byte-equal; the speed round is what remains
  before the switch (10 times the Rust front end's time at AU45; 3 is
  the gate); the lexer's rewrite is in (AU46), the parser's is next.
- **D. The VM and its speed**: stages 7, 13, 16, 20, 21, 23, 24, 26,
  27 (B3). Measured by AT1's rule on the frozen self-check and by the
  four benchmarks; every stage's numbers are in its decision entry.
- **E. Packages, the FFI and Python**: stages 15, 18. Done; candidates
  not decided: record types from dataclasses in the binder, a deadline
  per Python call, one worker across a test run.
- **F. The agent tooling and the integrations**: stages 5, 7 (the MCP
  server), 19 (the resident world, the language server, the watch, the
  embedding API). Done; candidates not decided: completion, references
  and rename in the language server, the C API of decision A5.
- **G. Release and the ecosystem**: stage 17. Release 0.1.0 is out;
  every release after it follows `docs/RELEASE.md`; the announcement
  and the three measurements of the positioning's section 5 are the
  owner's pending actions.
- **H. Readability**: stages 2, 6, 8. Four rounds exist under
  `tests/readability/`; they run only on request (V1).

## 4. What is next

In the order the owner set (AU36): the rest of stage 27 (B2, B3), each
measured by AT1's rule and recording the ratio of the compiler written
in Renyi to the Rust front end (AU36 iii); then, when the ratio is
within 3, the switch of the binary's front end to the compiler written
in Renyi (AU35 ii). Candidates after it, none decided: the levers the
study named beyond B3 (values unboxed across calls, an optimizing
tier, interned names and a faster hash in the Renyi checker), the C
API, the language server's completion, the showcase application, the
WASM target of M6.
