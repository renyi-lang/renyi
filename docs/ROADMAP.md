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
| Self-hosting | the compiler written in Renyi (lexer, parser, checker, emitter) held equal to the Rust toolchain by the judges; the binary switches to it when it is within 3 times the Rust front end's time (decision AU35 ii) | the compiler exists and is judged (stages 11, 12, 14, 15); the speed round is under way (stages 25 to 28; stage 29, a new backend for the VM, has its study, its stages 0 and 1, the character tables with the hasher, stage 2 and stage 3a, AW1 to AW11) |

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
    (`tools/count_tokens.py`), the corpus lint
    (`tools/lint_examples.py`). Done. 2. **The design, round 2, the
    library and the readability protocol** (session 2). Decisions J (the
    second round), K (the standard library: the prelude, the core
    modules, the extension packages, as declaration files under
    `library/std/`), L (the readability test: Predict, Explain,
    Complete, Write, graded by models). Done. 3. **The front end in
    Rust, M1, and the pre-test round** (sessions 3 and 4).
    `crates/renyi_syntax`: spans, diagnostics, the lexer, the AST, the
    parser, the JSON encoder, the formatter; `renyi check`, `format`,
    `tokens`, `parse`; the corpus tests; decisions M from the subagent
    pre-test. Done. 4. **The type and effect checker, M2** (session 4).
    `crates/renyi_check`: the world of declarations, the bodies, the
    effects, the refinements on literals, the fixes (the closest name,
    the Renyi spelling of a foreign name, the prelude's conversions);
    decisions N. Done. 5. **The runtime design, the agent tooling and
    the system-level commitments** (session 4). Decisions O (the project
    map `renyi index`, metrics, content hashes, budgets, the semantic
    diff; the narrated run), P (signature capabilities: `needs`, `at
    most` budgets, `only to` provenance guards), Q (the grant stack, the
    run manifest); `crates/renyi_index`. Done. 6. **The first live
    readability round** (session 5, 2026-10-05). `tests/readability/`:
    the harness, the manifest, the Explain references, the Write tasks;
    decisions R1 to R8 from the round. Done; later rounds only on
    request (decision V1). 7. **The VM, M3** (session 6).
    `crates/renyi_vm`: the bytecode and its compiler from the checked
    tree, the interpreter, the primitive boundary (grants, budgets,
    recording, replay, narration), the library natives (HTTP through
    `ureq`, the server, SQLite, decision S1), tasks one after the other
    (S2); `renyi run`, `record`, `run --replay`, `reproduce`, `test`
    with `replays`; the MCP server `renyi mcp` (decisions T). Done. 8.
    **Readability round 2, rounds 3 and 4** (session 6, 2026-10-06).
    Decisions U1 to U9; round 4 on Sonnet through the Claude Code CLI:
    Predict 90, Explain 100, Complete 79, Write 40. Done; the scores are
    no longer the gate (V1). 9. **The gap audit and the road to
    self-hosting** (session 6, 2026-10-06). `docs/GAPS.md`: what the
    design promised and the implementation did not deliver, with the
    order of work; decisions V1 (the aim: a self-hosted language people
    use), V5 to V7 (the limits the checker enforces), V8 (CI and the
    conformance suite), V11 (the surface frozen by decision at commit
    `dc58bb3`), V12 (the formal grammar to come). Done. 10. **Stage 1 of
    the gap audit** (session 7, 2026-10-06). Fourteen commits: the
    checker enforces V5 to V7, `renyi tools` (D6), the `only to` guards
    run, CI on every push with `tests/conformance/`, the runtime's
    promises (deadlines, refined updates, scopes, `equals`, the Float
    range), the module rules, the readability references as conformance
    cases. Done. 11. **Stage 2: the grammar, the reference and the front
    end written in Renyi** (session 8). `docs/grammar.ebnf` (W3C EBNF,
    interpreted by a test against the parser), `docs/reference.md`
    (normative, held to the grammar and the crates by a test, the sketch
    retired to the design record); `compiler/ast.ry`, `lexer.ry`,
    `parser.ry`, `parse.ry`, `tokens.ry` (decisions W1 to W4), held
    byte-equal to `renyi parse --json` by
    `crates/renyi/tests/selfhost.rs`. Done. 12. **The checker written in
    Renyi** (session 8). `compiler/declare.ry`, `bodies.ry`,
    `checker.ry` and their helpers (decisions W5 to W8), held byte-equal
    to `renyi check --json`, with and without `--strict`, on every
    program and exiting as it exits. Done. 13. **The profile and the
    loop of the VM** (session 8). Decisions X1 to X6: the development
    profile optimizes the VM, `List.slice`, the interpreter loop
    rewritten, `renyi run --profile`, the emitter writes a bytecode
    file, mimalloc. Done. 14. **The residue of stage 1, the bytecode
    file, the rename and the emitter in Renyi** (session 8). Decisions
    Y1 to Y4; the bytecode file `.ryc` with `renyi compile` and the
    loader (Z1, Z2, Z4); `set` renamed `change` (AA1); the emitter
    `compiler/emit.ry` and `compile.ry`, held byte-equal to `renyi
    compile` (Z3); constraints with type arguments (AB1, open item R3-2
    closed); the judges run the front end from its own bytecode. Done:
    the toolchain in Renyi is complete, with the Rust toolchain as stage
    0. 15. **Stage 3: packages, the diagnostics in Renyi, the process
    module, the FFI** (session 8, 2026-10-07; M4).
    `crates/renyi_package` and `renyi add`, `update`, `audit`, `fetch`,
    `publish` with the effect manifests (AC1, in both front ends); the
    Rust lexer's and parser's codes, fixes and recovery in the Renyi
    front end, the judges equal on rejected programs (AD1);
    `std.process` (AE1); `renyi bind` for a C header and the boundary's
    rules (AF1). Done. 16. **Machine code for the bytecode** (session
    8). `crates/renyi_vm/src/ native/`: the abstract interpretation,
    every op translated to Cranelift IR, the hand-back to the
    interpreter, the hotness rule (AG1 to AG5); the bounded VM round
    (AG6); the benchmarks `bench/` with CPython twins and
    `tools/bench.py` (AG4). Done. 17. **The niche and release 0.1**
    (session 8, 2026-10-07). The positioning (AH1 to AH4,
    `docs/design/08-positioning.md`: the scripting language of AI
    agents); the release workflow, the installers, the VS Code
    extension, the procedure `docs/RELEASE.md` (AI1, AI2), the starter
    pack for agents `starter/` (AI3), the documentation site
    (`tools/site.py`, GitHub Pages, renyi-lang.org), the self-contained
    crates (AI5); release 0.1.0: the repository public under
    `renyi-lang/renyi`, the seven crates on crates.io, the tag `v0.1.0`.
    Done. 18. **Foreign packages: Rust natives and Python** (session 8).
    The registration API (`Extension`, `Native`, `Registry`; the
    standard library as the first extension; `renyi` a library too,
    `main_with`; `docs/extensions.md`; AJ1, AK1 to AK4); the Python
    bridge (a module bound by the manifest, one worker per run,
    `PythonError`, `docs/python.md`; AL1 to AL4) and the binder `renyi
    bind --python` (AM1, AM2). Done. 19. **M5: the resident world, the
    language server, the watch of `serve`, the embedding API** (session
    8, 2026-10-07 and 08). `crates/renyi_workspace` held by `renyi mcp`
    between calls (AN1); `renyi lsp` with the client in the VS Code
    extension (AN2, AN3); `renyi serve --watch`, `main` run again
    between two requests on a clean new version with the socket kept
    (AO1); `renyi::Sandbox`, the grant as a `needs` clause with the
    memory budget, `renyi run --sandbox`, `docs/embedding.md` (AP1,
    AP2). Done: M5 complete. 20. **The profile-guided round on strings
    and JSON** (session 9, 2026-10-08, the first session in the cloud
    environment). Decision AQ: the pattern cache of `Text.matches`, the
    string building, the JSON path, measured against the targets set
    after M5. Done. 21. **The baseline JIT** (session 9). Decisions AR1
    to AR6: the fused op `LoadField`, direct calls between generated
    functions with register arguments, the layout of `Value` pinned so
    that the value operations run in place, the tiering measured and the
    hotness factor raised; the self-check 15% below where AQ left it.
    Done. 22. **`renyi build`** (session 9). Decisions AS1 to AS4: the
    image of a program (its bytecode with the machine code of every
    function), which `run`, `record`, `test` and `reproduce` load
    without compiling; `--exe`, the self-contained executable. Done. 23.
    **The size of the generated code** (session 9, 2026-10-08 and 09).
    Decisions AT1 to AT7: the rule (KCachegrind's estimate of the cycles
    on the compiler's self-check, `tools/measure_size.sh`), the cuts at
    no cost, the image's program in a binary encoding, the static stack
    height, the image's code section mapped from the file, the reference
    counts as calls. Done. 24. **The typed round: the checker's static
    types brought to the code generator** (sessions 9 to 12, 2026-10-09
    and 10). Decision AU1 and the stages AU2 to AU34: typed calls to
    primitives without the boundary, the loop over a list in the
    generated code, the typed bytecode, the types of parameters, the
    image cache of `renyi run` (AU10), the update of a uniquely held
    record in place, comparisons on borrowed operands, the compile
    thread, the liveness pass, the template tier (a small x86-64
    assembler, every code object on machine code from its first call;
    AU18, AU19, a Windows job in CI), its direct calls, the
    representation items (small texts in the value, records and variants
    in one block), the counts in place on both tiers, `List.at` in
    place. The self-check's JIT run fell from 1,593 ms to about 1,000 on
    one hardware thread over the round. Closed by the owner with its
    three goals met (AU35). 25. **The front end's round, the Rust side**
    (sessions 12 and 13, 2026-10-10). The study (why the compiler
    written in Renyi takes eleven times the Rust front end's time, per
    operation) and the owner's order (AU36); A0, every file parsed once
    (AU37); A1, token comparisons by variant (AU38); A4, the module
    trees shared and a program declaring only the library modules it
    needs, in both front ends (AU39, AU42); the self-check frozen under
    `bench/selfcheck/` (AU41); A2, the cache keyed by the sources: a
    second run of the same program loads the compiled program and runs
    no front end (AU40, AU43, AU44). `renyi check compiler/bodies.ry`
    from 437 million instructions and 72 ms to 153 million and 34.5 ms;
    a hit of `hello` -33% in instructions. Done. 26. **The front end's
    round, the VM side: B5** (session 13, 2026-10-11). Two Integers of
    which at least one is boxed and the checker typed so compared in
    place in both tiers when both are small, the helper kept for the
    rest (AU45): the case of an Integer read from a list, a field or a
    `maybe` against another such or a literal, which a lexer over code
    points compares at every character; `bench/micro/scan_codes.ry`,
    -48% in instructions a turn; the Renyi checker 806 to 756 ms on
    `compiler/bodies.ry`. Done. 27. **The front end's round, the Renyi
    side** (session 13, 2026-10-11; the list done, the gate not met).
    B1, the Renyi lexer rewritten for speed with the library as it is:
    one loop, scans without a call per character, the word lists as
    sets, a step's product a sum; the lexer alone on
    `compiler/bodies.ry` 983 to 514 million instructions (-48%),
    byte-equal by the judges (AU46); B2, the parser: its keyword and
    symbol tests and token reads cut first (AU47, the parser alone
    -13%), then a look as an index with no record per look and the eight
    expression levels as one loop (AU48, 1,039 to 810 million
    instructions per parse, -22%; the keywords as variants without
    fields left for the measure after the VM's cuts); the variant test
    in place on both tiers (AU49, every `match` arm's test without its
    helper: the self-check's synchronous run -0.9%, the parser alone
    -1.4%); B3, small callees expanded into their callers on the
    Cranelift tier (AU50: a rewrite of the bytecode the tier sees, the
    frames rebuilt at a hand-back, built and tested; measured, the
    expanded code is level with the calls it replaces in the steady
    state and the compile costs 6% of a parse, so it is off unless
    `RENYI_NATIVE_INLINE=1`). The gate: the compiler written in Renyi
    within 3 times the Rust front end's time on `compiler/bodies.ry`
    (10.0 times at AU45, 8.6 at AU47 against AU34's binary; 7.5 at AU49
    and 7.4 at AU50 with the expansion off, 8.4 with it on, the Renyi
    checker 583 ms), then the binary switches to it (AU35 ii). The list
    of AU36 is finished short of the gate; the owner's evaluation (AU51)
    keeps the gate and opens stage 28.

28. **The compiler's own code** (session 13, 2026-10-11, closed;
    decisions AU51 to AU57, B4). The lever the owner chose after B3, cut
    by cut, each judged byte-equal, measured by its driver under
    callgrind and recorded: the checker's line starts and layout check
    over lines (the checker -2.0%), the parser's look computed once per
    move of the cursor (the parser alone -0.8%), the lexer's scans as
    two list primitives, `run_of` and `run_until` (the lexer alone
    -1.4%; the checker -3.0% for the three), the reserved words, phrases
    and symbols as Integer codes compared in place, with the VM reading
    a built module constant in place on both tiers (the parser alone
    -3.5%, the lexer alone -2.4%, the checker -1.2% against the stage's
    start), two lookups by index, the lexer's last append in place and a
    shared list appended with one copy (the checker -3.1% more), the
    last read of a local moved by both emitters with a taken field read
    in place on both tiers and the checker's code reshaped where it
    defeated the rule (the checker -1.6% more), the frame leave dropping
    only the slots that hold a counted block (the checker -0.8%
    more), `List.with_at`, a replacement by index in place, under the
    compiler's `replace_at` (the checker -0.9% more; its other half, a
    small text hashed as two words, measured slower under SipHash and
    left out). The measure: the checker's run 4,898.3 to 4,391.6
    million instructions (-10.3%), the ratio against AU34's binary 7.4
    at the stage's start and 6.9 after AU55. Closed by the owner's
    answers after it (AU58), which open stage 29.

29. **A new backend for the VM, its design study first** (sessions 13
    and 14, 2026-10-11, in progress; decisions AU58 and AW1). The
    owner's answers after B4: the gate of AU35 (ii) stays at 3 times,
    2.3 times away by its own measure (the checker's run from its image
    3,085.3 million instructions on session 14's binary against three
    times AU34's `renyi check` of the same file, 441.6 million), which
    cuts of one to three percent will not cover; the way is a new
    backend planned by a design study before any code. The study
    (session 14, the handoff's section "Stage 29, the new backend: the
    design study"): the instructions attributed by category (the
    generated code 32.8%, the counts and drops 14.5%, the natives with
    their glue 11.6%, mimalloc 8.4%, the hashing 7.1%, the records
    6.7%), eleven findings (a call costs about 130 instructions besides
    its body; a third of the frames leave through `rt_truncate` at 329
    each; one call in nine enters through the VM's path; a record
    update costs 1,070 and a free 232; the lexer's character classes
    are three quarters of the hashing), and the road the owner chose
    (AW1): the Cranelift tier evolved in stages, each measured and
    judged, stage 0 (the allocator's plain entry, the literal constants
    in images), stage 1 (the calling convention: a boxed operand unboxed
    at the site, the leave by the static ownership state, a parameter
    only read passed borrowed under a flag in the bytecode, the frame
    record cheaper), stage 2 (the records: `with` by static index,
    `Done` returned in the caller's slots, the frees in place), stage 3
    (the natives answering in place, the character tables, the VM's
    hasher, the glue), stage 4 (field-less variants compared in place,
    `match` by a table, the iterator in two slots), stages 5 and 6 (the
    load, the expansion measured again), the checker's own cuts between
    them; the estimates sum to 1,540 million, 3.5 times the reference
    with every stage landed, the last part the checker's shape. The
    gate is measured on the check row alone (AW1 (2)); the compile row
    (13.25 by instructions) waits on the emitter's own round. Stage 0
    done (session 14, 2026-10-11, decision AW2): the allocator entered
    by mimalloc's plain functions for an alignment of at most 8, and
    the module constants a pure computation gives built when an image
    is built, so that a small Integer among them is a literal in the
    image's code (the checker's constants are computed, `symbol_code(",")`,
    not literal, which the study had assumed); the check row 3,085.3
    to 2,889.9 million instructions (-6.3%, the study's estimate 5%),
    the ratio by instructions 6.99 to 6.54, AT1's synchronous row -3.6%
    and -2.5% by the estimate, `records` -11.5%. Found on the way: on
    Windows every run of an image copied its code section, the mapping
    refused for want of execute access on the file's handle (a2c4685).
    Stage 1a done (session 14, 2026-10-11, decision AW3): a typed
    parameter's boxed operand, which the checker typed (a variant's
    field bound by a pattern, a `maybe` after its `otherwise`), taken
    from the stack at the call site, its tags tested in place, so that
    the call enters the callee's body directly; the calls through the
    VM's path 141,577 to 9,380 (the parser's function values remain),
    the check row 2,889.9 to 2,826.6 million instructions (-2.2%, the
    study's estimate 2.1%), the ratio by instructions 6.54 to 6.40.
    Stage 1c done (session 14, 2026-10-11, decision AW4): a frame's
    leave releases only the slots that may hold a counted block at the
    return, by a fixed point over the ops (a boxed slot moved out by
    its last read or updated in place holds `Nothing` and is skipped),
    inline up to eight; the leaves through `rt_truncate` 917,264 to
    32,514, the check row 2,826.6 to 2,729.4 million
    instructions (-3.4%, the study's estimate 6.5%), the ratio by
    instructions 6.40 to 6.18. Stage 1b done (session 14, 2026-10-12,
    decision AW5): a read-only flag per parameter in the bytecode
    (format 7 to 8, both emitters, the judges; 2,133 of the checker's
    2,328 parameters), and a direct call passes such a parameter,
    when its callee never moves it into an owned sink, borrowed,
    without a reference of its own, which the callee's leave skips and
    its other ways out take first; the check row 2,729.4 to
    2,719.3 million instructions (-0.4%, the study's estimate
    2.6%), the ratio by instructions 6.18 to 6.16. Stage 1d done
    (session 14, 2026-10-12, decision AW6): the frame record of a
    direct call made cheaper, the grant in the body's signature, the
    stack's length written and the room checks made in the callee's
    prologue; the check row 2,719.3 to 2,681.0 million instructions
    (-1.4%, the study's estimate 1.8%), the ratio by
    instructions 6.16 to 6.07. Stage 1 is complete (the check
    row 2,889.9 to 2,681.0 since stage 0, -7.2%; the study's
    estimate 13%). The character tables and the hasher done
    (session 14, 2026-10-12, decision AW7): `run_of` and `run_until`
    scan by a table of 128 bits of the set's one-character members,
    the tables of the last eight sets kept, and the VM's maps and sets
    hash by a folded multiplication keyed per process in place of
    SipHash; the check row 2,681.0 to 2,527.2 million instructions
    (-5.7%; the tables alone -2.2%), the ratio by
    instructions 6.07 to 5.72. Stage 2a done (session 14, 2026-10-12,
    decision AW8): a `with` whose base the checker typed updated by
    static index, through a helper that takes the type and the
    indices from the site on both tiers and in the generated code on
    the Cranelift tier; the check row 2,527.2 to 2,370.7 million
    instructions (-6.2%; the helper alone -4.4%, the
    study's estimate 4.9%), the ratio by instructions 5.72 to
    5.37. Stage 2b done (session 14, 2026-10-12, decision AW9): a
    record of two fields returned as a pair of cells to a direct
    caller, kept in a pair slot and taken apart in place, past the
    test of an `otherwise` that only a failure reaches; the check row
    2,370.7 to 2,301.9 million instructions (-2.9%; the pair at
    the call alone -1.2%, the study's estimate 4.9%), the
    ratio by instructions 5.37 to 5.21. Stage 2c done (session 14,
    2026-10-12, decision AW10): the frees in place, a value released
    by its tag in the VM's Rust, the last holder of a record or a list
    releasing its fields or its elements the same way; by
    instructions the walk costs nearly what Rust's drop glue cost (the
    check row 2,301.9 to 2,296.2 million, -0.2%, against the
    study's estimate of 3.2%: the frees' cost is the inspection and
    the deallocation, not the glue's dispatch), but it spares the
    glue's indirect branch per field (AT1's synchronous estimate
    -1.3%), and the stage stays by AT1's rule; the ratio by
    instructions 5.21 to 5.20. Stage 2 is done. Stage 3a done
    (session 14, 2026-10-12, decision AW11): the natives' in-place
    entry, a second entry per native through AK1's registration API
    that the generated code of both tiers and the interpreter call
    directly through a table the VM keeps per function, answering in
    the receiver's place with the arguments consumed, no scratch
    vector, no `Result`, no status and no push on the way, the
    standard library's 26 hot natives first; the check row 2,296.2 to
    2,203.9 million (-4.0%); the ratio by instructions 5.20 to
    4.99. Stage 3b, `status` and `into_plain` off the path,
    is next.

30. **The icon and the wordmark** (session 13, 2026-10-11, done;
    decision AV1). The owner asked for an icon that does not read as the
    R language's, as the R on the accent tile did, and chose over five
    rounds of rendered options (the character 人, six divergent
    directions, the name as a signature in forty-five script fonts, R and
    Ry built from objects, Ry on grids and five objects): a war hammer on
    the accent tile and the name signed in Ms Madi. `tools/brand.py`
    holds both and writes the README's logo, the VS Code extension's
    icons and the social preview; the site's header shows the mark and
    the signature. The social preview waits on the owner's upload in the
    repository's settings.

## 3. The tracks

The same stages grouped by the line of work they belong to, with the
state of each line.

- **A. The design and the normative texts**: stages 1, 2, 9 (the
  freeze), 11 (the grammar and the reference), 17 (the positioning).
  The surface is frozen (V11): a change is a new decision entry first,
  then the reference, the grammar, the cheat sheet, the formatter and
  the conformance suite in one commit. Open: R3-1 (`Iterable`), section
  6 of `docs/GAPS.md`.
- **B. The toolchain in Rust**: stages 3, 4, 5, 7, 10, 13, 14, 22, 25.
  Complete for the language as designed; the Rust front end is the
  binary's until the switch of track C.
- **C. The compiler written in Renyi (self-hosting)**: stages 11, 12,
  14, 15 (AD1), 27, 28. The lexer, the parser, the checker and the
  emitter exist and are judged byte-equal; the speed round's list
  (AU36) is done: the lexer's rewrite (AU46), the parser's (AU47,
  AU48), the variant test in place (AU49) and the expansion of small
  callees (AU50, off by its measure); stage 28, the compiler's own
  code, closed at the ratio 6.9 against the gate of 3, which the owner
  kept (AU51, AU58); the gate now waits on stage 29 of track D.
- **D. The VM and its speed**: stages 7, 13, 16, 20, 21, 23, 24, 26,
  27 (B3), 29. Measured by AT1's rule on the frozen self-check and by
  the four benchmarks; every stage's numbers are in its decision entry.
  Stage 29, a new backend, has its design study (AU58, AW1) and its
  stages 0 and 1, the character tables with the hasher, stage 2 and
  stage 3a (AW2 to AW11); stage 3b, the glue off the path, is next.
- **E. Packages, the FFI and Python**: stages 15, 18. Done; candidates
  not decided: record types from dataclasses in the binder, a deadline
  per Python call, one worker across a test run.
- **F. The agent tooling and the integrations**: stages 5, 7 (the MCP
  server), 19 (the resident world, the language server, the watch, the
  embedding API). Done; candidates not decided: completion, references
  and rename in the language server, the C API of decision A5.
- **G. Release and the ecosystem**: stages 17, 30. Release 0.1.0 is
  out; every release after it follows `docs/RELEASE.md`; the icon and
  the wordmark (AV1) are in the site, the README and the extension; the
  announcement, the three measurements of the positioning's section 5
  and the upload of the social preview are the owner's pending actions.
- **H. Readability**: stages 2, 6, 8. Four rounds exist under
  `tests/readability/`; they run only on request (V1).

## 4. What is next

Stage 29 (decisions AU58 and AW1): the new backend's stages in the
order the study set and the owner confirmed: stage 0 is done (AW2,
2026-10-11: the allocator entered by mimalloc's plain functions, the
module constants an image builds before any run; the check row 3,085.3
to 2,889.9 million instructions, -6.3%, the ratio by instructions 6.99
to 6.54), and stage 1 is done (AW3 to AW6, 2026-10-11 and
2026-10-12: a typed parameter's boxed operand taken from the stack at
the call site, the check row 2,889.9 to 2,826.6, -2.2%, the ratio
6.54 to 6.40; the frame's leave by the static ownership state of the
slots, the check row to 2,729.4, -3.4%, the ratio to 6.18; a
read-only parameter passed borrowed between compiled bodies, the
check row to 2,719.3, -0.4%, the ratio to 6.16; the frame record of a
direct call made cheaper, the check row to 2,681.0, -1.4%, the
ratio to 6.07), and the character tables with the hasher of the VM's
own are done (AW7, 2026-10-12: the check row to 2,527.2, -5.7%,
the ratio to 5.72), and stage 2a is done (AW8, 2026-10-12: `with` by
static index, the check row to 2,370.7, -6.2%, the ratio to
5.37), and stage 2b is done (AW9, 2026-10-12: a record of two fields
returned as a pair, the check row to 2,301.9, -2.9%, the ratio
to 5.21), and stage 2c is done (AW10, 2026-10-12: the frees in place,
the check row to 2,296.2, -0.2%, AT1's estimate -1.3%,
the ratio to 5.20), and stage 3a is done (AW11, 2026-10-12: the
natives' in-place entry, the check row to 2,203.9, -4.0%, the
ratio to 4.99), then the glue off the path (3b), stage 4, the
checker's own cuts between the stages, stages 5 and
6 last; each measured by the checker's run under callgrind with the
table by category, the ratio of AU36 (iii), AT1's rows where the tiers
change, the four judges and the conformance suite on both tiers, kept
by AT1's rule, toward the gate of AU35 (ii) at 3 times on the check
row (4.99 by instructions after stage 3a); then, when the ratio is
within 3, the switch of the binary's front end to the compiler written
in Renyi. Candidates beside it, none decided: the emitter's own round
for the compile row, the Rust front end's name sets as bits, the C
API, the language server's completion, the showcase application, the
WASM target of M6.
