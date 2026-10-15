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
| M6 | ahead-of-time code: machine code for the bytecode, the baseline JIT, the template tier, `renyi build` images and self-contained executables | the machine code exists (stages 16, 21, 22, 24, 26); the WASM target is stage 33 (its study AW28 and AW29; the toolchain built for wasm32 at 33a, AW30, the suspension and `std.screen` at 33b, AW31, the console host and the game on the library's types at 33c, AW32, the pages at 33d, AW33: stage 33 done on 2026-10-13); the C API comes after the language server's completion (AW28) |
| Self-hosting | the compiler written in Renyi (lexer, parser, checker, emitter) held equal to the Rust toolchain by the judges; the binary switches to it when it is within 3 times the Rust front end's time (decision AU35 ii) | the compiler exists and is judged (stages 11, 12, 14, 15); the speed round (stages 25 to 32) closed at the gate on 2026-10-13, the ratio 2.96 against 3 (AW26); the switch deferred by the owner, the Rust front end kept (AW27) |

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
    4.99. Stage 3b done (session 14, 2026-10-12, decision AW12):
    `status` and `into_plain` off the path, a plain record or variant
    built through a helper without a status on both tiers, a plain
    item pushed onto a plain list where it lies, the guard tests in
    place with the wrapping out of line, a plain list iterated as it
    is; the check row 2,203.9 to 2,099.1 million (-4.8%); the
    ratio by instructions 4.99 to 4.75. Stage 3 is done. Stage 4a
    done (session 14, 2026-10-12, decision AW13): `is` and `is not`
    answered in place as structural equality unless the left
    operand's type declares its own `equals`, and two values of one
    variant type compared in the generated code of both tiers by the
    type and the tag of their blocks, the same tag of a variant with
    fields left to the helper; the check row 2,099.1 to 1,995.8
    million (-4.9%); the ratio by instructions 4.75 to
    4.52. Stage 4b
    done (session 14, 2026-10-12, decision AW14): a loop's iterator in
    two slots of the frame, the source listed in the first and the
    position as a small Integer in the second, which the Cranelift tier
    keeps in a register and reads the list through in place, the
    native iterator and its block gone; the check row 1,995.8
    to 1,930.3 million (-3.3%); the ratio by instructions
    4.52 to 4.37. Stage 4c
    done (session 14, 2026-10-12, decision AW15): a `match` with at
    least three arms naming a variant entered through a jump table by
    the subject's tag, past the arm's own tag test, on both tiers and
    in the interpreter, the arms' own tests kept for what falls
    through; the check row 1,930.3 to 1,896.4 million
    (-1.8%); the ratio by instructions 4.37 to
    4.29. Stage 4 is done. The checker's own cuts between the
    stages done (session 14, 2026-10-12, decision AW16): the variants
    of a module's scope indexed by name once the types are final, the
    parser's cursor carrying its looked-at token and updated in its
    slot as it moves, the lexer's previous token as two Integers and
    the text of a run as the primitive `slice_text`, the resolver's
    loop, the checker's lists updated in the slot; the check row
    1,896.4 to 1,586.1 million (-16.4%); the ratio by
    instructions 4.29 to 3.59. Stage 5a done (session 14,
    2026-10-12, decision AW17): the moves of AU17 written in the image
    and in the cache keyed by the sources, the pass run once where a
    program enters the VM; the check row 1,586.1 to 1,547.9
    million (-2.4%); the ratio by instructions 3.59 to
    3.51. Stage 5b done (session 14, 2026-10-12, decision AW18): the
    code objects decoded on demand, a body decoded at its first read
    from the image's or the cache entry's bytes; the check row
    1,547.9 to 1,528.5 million (-1.3%); the ratio by
    instructions 3.51 to 3.46, which completes stage
    5. Stage 6 done (session 14, 2026-10-12, decision AW19): AU50's
    expansion of small callees measured again under the new convention
    and left off (the check row 1,528.5 to 1,585.3 million, +3.7%,
    with it on: an expanded callee loses the borrowed parameters of
    AW5 and the pair return of AW9), which completes AW1's order; the
    ratio by instructions stands at 3.46 on the check row
    against the gate's 3, and the road from here is the owner's
    decision.

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

31. **The new code generator, road (B)** (session 14, 2026-10-12, in
    progress; decision AW20, the study; AW21, the owner's answers). The
    owner's answer after AW1's order: with the check row at 1,528.5
    million instructions and the ratio at 3.46 against the gate's 3,
    road (B) of AW1 over a second round of the checker's cuts, the
    emitter's round and the switch at 3.46. The study (the handoff's
    section "Stage 31, the new code generator (road (B)): the design
    study"): the generated code 800.8 million of the 1,528.5 (52.4%), a
    direct call about 100 instructions beside its body, a typed field
    read about 58, a boxed local's load or store about 18 (the lane's
    micro programs per turn), the loads, moves, stores, field reads,
    constants and dups 26.0 million of the run's 49.5 million ops; (B)
    buildable on the Cranelift tier as it stands in four measured
    stages: B2 the operands in registers (`Abs::Held`, the VM's stack
    written only where a helper reads it, about 180 million), B3 the
    locals in registers (`SlotKind::Held`, the pair slots folded in,
    about 90), B1 the frame record lazy (a chain on the machine stack
    materialised by the helpers that read `Vm::frames`, about 40), B4
    the counts by dataflow (about 30): about 340 million in all, the
    check row to about 1,190 million, 2.7 times the reference, under
    the gate. The owner's answers (AW21, every recommended option): the
    four stages on the Cranelift tier in the order B2, B3, B1, B4, a
    held value as its three words, each stage measured as AW1's were
    with the lane's micro programs per turn, the expansion measured
    again after B3, the pair slots folded into B3, the template tier
    unchanged, the checker's cuts as the reserve. B2 done (session 14,
    2026-10-12, decision AW22): the operands in registers, `Abs::Held`
    on the Cranelift tier, the VM's stack written only where a helper,
    a landing, an exit or a hand-back reads it; the check row
    1,528.5 to 1,482.3 million (-3.0%, 46.2
    million against the study's estimate of about 180); the ratio by
    instructions 3.46 to 3.36. B3 (the locals in registers) built,
    measured and left off (decision AW23, 2026-10-12: the check row
    +6.0% against the base with the leak fixed, AT1's estimate +9.0%,
    the whole rise in the generated code, three SSA words per slot
    spilled and moved); the leak its measure found, since stage 2b,
    the owned analysis stopping at an `otherwise` region a pair's
    call skips, fixed (AW23: the check row 1,482.3 to 1,493.5 for the
    blocks now freed, the ratio 3.38). B1 (the frame record lazy) done
    (decision AW24, 2026-10-13: a direct call pushes no record, the
    callee's node in its machine frame and the records made when a
    helper reads them; the check row 1,493.5 to 1,482.7, -0.7%; the
    ratio 3.36). B4 (the counts by dataflow) done (decision AW25,
    2026-10-13: a field read and a module constant pushed borrowed for
    their consumer, a store releasing nothing where the slot holds no
    reference, and the walk of the borrowed operands stopped where
    another path enters, a leak since AU13 fixed on both tiers; the
    check row 1,482.7 to 1,437.8, -3.0%; the ratio 3.26). The four
    stages of road (B) are measured; the owner's call after them is
    stage 32.

32. **The second round of the checker's own cuts** (session 14,
    2026-10-13, done; decision AW26). The owner's answer after B4: the
    reserve of AW21 (8), cuts in the compiler's own sources on stage
    B4's binary, over the emitters' round and the switch at 3.26. Four
    cuts, each judged byte-equal and kept by the check row: the parser's
    result records read after their cursor bind the other field first
    (138 sites); a record updated into a fresh local written as a move
    and an update in the slot (12 sites, the checker copied 17,036 times
    in `infer` alone); `identifier`, `primary` and `word_primary` moving
    the cursor only in the arms that take it; the one-line helpers
    `is_word`, `is_symbol`, `token_at`, `moved` and `var_at` inlined. An
    experiment on the emitters' liveness rule (a write kills the name)
    measured and not kept: the VM's pass of AU17 already moves every
    slot's last read. The check row 1,437.8 to 1,307.5 million (-9.1%),
    the parse row 629.5 (-6.0%), the compile row 2,468.1 (-5.0%); the
    ratio 2.96, within the gate of 3; the record copies 109,613 to
    19,266. The owner's decision after it (AW27, 2026-10-13): the switch
    of the front end deferred although its gate is met, the Rust front
    end kept, the speed round closed; the next stage is one of the
    candidates of section 4, the owner's choice.

33. **The WASM target** (session 14, 2026-10-13, done; decision
    AW28, the owner's order). The VM for the browser: the interpreter
    compiled to WebAssembly with the natives that cannot exist there
    left out or stubbed, a page on the site that runs a program, and the
    suspension of a program inside a native call (`screen.present`,
    decision AW28 (3)) so that a frame loop hands control back to the
    page. The design study done on 2026-10-13 (the handoff's section
    "Stage 33, the WASM target: the design study (session 14)"): the
    front end's crates build for wasm32 as they are and the VM stops in
    `getrandom` through `ureq`, `region` and `libloading` have no wasm
    backend, the record block's offsets are 64-bit (a bug for any 32-bit
    target), the clock panics on the target, the JIT and the natives of
    what the browser cannot have go behind cargo features; the
    suspension as a fifth interrupt that keeps the frames, resumed by
    the host with the input, on the interpreter in this stage; a web
    crate with a C ABI by hand and the page's own script, the VM alone
    on bytecode; `std.screen` as a library module with a new capability. The owner's answers (AW29, 2026-10-13): the VM's own
    suspension, the whole toolchain in the browser (the owner's choice
    over the VM alone: a page takes sources, checks and runs them, the
    game compiled in the browser and a playground beside it), a C ABI by
    hand with no tool beyond cargo, cargo features for what the browser
    cannot have. The stages: 33a the toolchain for wasm32, 33b the suspension, 33c `std.screen`, 33d the pages. 33a done on
    2026-10-13 (decision AW30: the features `jit`, `foreign`, `http` and
    `sqlite` of `renyi_vm`, the record block's offsets in words, the
    clock of the VM's own, the files provided to the resolver, the crate
    `renyi_web` with its C ABI, `tools/wasm_check.js`, CI's `wasm` job;
    the module 3,027,356 bytes (2.89 MiB), 978,737 gzipped (0.93 MiB);
    hello and the game's demo run under node, the interpreter at about
    two thirds of its native speed on integer loops); 33b done on
    2026-10-13 (decision AW31: `Interrupt::Suspended` passed through
    `execute` with the frames kept, the host `Options::screen`,
    `Vm::resume` noting the call in the recording, a suspension only
    from the run's own loop and on the interpreter, `Sandbox::set_screen`,
    the module `std.screen` and the capability `screen`; seven tests of
    the VM, one of the sandbox, a conformance case; a frame of 320
    columns suspended and resumed costs 470 to 497 microseconds against 409 to 497
    answered at once on the interpreter, coarse wall-clock); 33c done on
    2026-10-13 (decision AW32: the console host `ConsoleScreen`, the
    frame as text at the terminal's size and a line read per frame,
    given by `renyi run`, `record` and `test` to a program that needs
    `screen`; the game on the library's types with its `play` mode;
    `screen` in the page's grant); 33d done on 2026-10-13 (decision
    AW33: the page as the host of `std.screen`, the web crate's session
    holding a suspended run, `renyi_resume`, `renyi_frame` and the
    status `SUSPENDED` in the C ABI, the play page of the site with the
    game on a canvas and a playground, `tools/site.py --module`, the
    Pages workflow building the module, `tools/wasm_check.js --script`
    and CI's run of the game's `play` mode; in a headless Chrome 60
    frames a second at 3.98 to 5.51 ms a frame in the VM, coarse
    wall-clock), which closes the stage.

34. **The showcase game** (session 14, 2026-10-13, done, in
    parallel with stage 33; decision AW28). A Doom-style raycasting maze
    written in Renyi under `showcase/raycaster/` by a subagent (Opus
    5.5, medium effort): the map, the player, the rays, the enemies, the
    score, every frame a pure function of the state and the input,
    rendered as a list of shapes (`Column`, `Rectangle`, `Label`) on a
    320 by 200 screen and, for local runs, as text; the subagent's part
    done on 2026-10-13 (commit b8f700e: eleven modules, 94 examples and
    tests, the demo reaching the exit, CI holding it); the library
    module `std.screen` (the frame handed to the host, the input
    returned, `present` suspending) built at 33b (decision AW31) and
    the game on its types with its `play` mode at 33c (decision AW32:
    playable at the terminal through `renyi run`); and the play page
    of the site, where the game is played in the browser, at 33d
    (decision AW33), which closes the stage.

35. **The language server's completion, references and rename** (session
    14, 2026-10-13, done; decision AW28, the owner's order). The
    server of decisions AN2 and AN3 gains `textDocument/references`,
    `textDocument/rename` with `prepareRename`, `textDocument/completion`
    and, if the owner takes them, `signatureHelp` and `workspace/symbol`;
    the checker records the uses of locals as references
    (`Target::Local`, mirrored in `compiler/bodies.ry`) so that a
    variable has references and a rename too. The design study done on
    2026-10-13 (the handoff's section "Stage 35, the language server's
    completion, references and rename: the design study (session 14)":
    what the server and the checker's record have, the shadowing rule,
    what each feature needs, the cost, the measure, the stages 35a
    references and rename of declarations, 35b the locals, 35c
    completion); its four questions answered (decision AW34). 35a
    done (decision AW38, 2026-10-13): `textDocument/references`,
    `prepareRename` and `rename` over the checker's recorded
    references, the signatures' (`renyi_index::signature_references`)
    and the exposing imports; a method renamed with its ability's
    and every implementation's; a rename refused when a member is the
    library's or a dependency's, when the new name is not a name of
    its class or is reserved, and when the project checked with the
    edits applied gains an error. 35b done (decision AW39,
    2026-10-13): both checkers record every use of a binding as a
    reference to its declaration (`Target::Local`, `LocalTarget`),
    the server answers references, definition and rename for a
    binding within its body, `self` refused; the record's cost on the
    check row -2.9% by instructions once the Renyi checker's
    references were kept per body (the first build +12.2%: an append
    through a checker held twice copies the module's list, found by a
    probe). 35c done (decision AW44, 2026-10-13): completion with its
    contexts (an import's path, a `needs` clause, an import's members,
    a type's members after a dot, the names in scope with the
    statement words), signature help by the innermost open
    parenthesis, workspace symbols from the outlines; both checkers
    record a binding's declaration as a reference to itself; a line
    the parser refuses is repaired for the request. Stage closed.

36. **The DOOM look** (session 14, 2026-10-13, done; decisions AW35,
    the owner's call after the play page, and AW37). The showcase game made
    to look like the original: `std.screen` gains `Strip` (a column of
    a named image between two rows, shaded), `Picture` (a named image
    stretched to a size at a place, shaded) and `play(sound)`, the
    console host drawing both as glyphs of their shade and the page
    drawing them from the game's images and playing its sounds; the
    art from Freedoom (3-clause BSD) taken by a script with its notice
    and its sizes as constants; the game rewritten on the contract by a
    subagent (textured walls, an animated demon, the shotgun, the
    status bar, the sounds); a page of its own on the site with a title
    screen, full width; the frame rate measured again. The parts: 36a
    the contract and the hosts, 36b the art, 36c the game (the
    subagent's: sixteen modules, 3,045 lines, 192 examples and tests),
    36d the page and the measure. Measured: the demo escapes at frame
    279 with the score 1,120; under node 3.85 to 4.46 ms a frame in the
    module; in a headless Chrome 155 on the owner's machine 55 to 60
    frames a second at 3.0 to 3.8 ms a frame in the VM, no console
    error. Stage 35 follows it.

37. **The map for retrieval by embedding** (session 14, opened
    2026-10-13 at the owner's request after 35b: "find a way to fit the
    RAG side"). The map's records and the sources as chunks for an
    embedding pipeline on the host's side: one per definition (and per
    module and per test if the measure says so) with an id that changes
    with the embedded text, the metadata to filter by and to walk from
    a hit, as JSON Lines from `renyi index --chunks` and the MCP tool
    `chunks`; the chunk's text settled by a prototype measured on a
    query set (hit@1 and hit@5 with one embedding model); no model in
    the toolchain. The design study is the handoff's section "Stage 37,
    the map for retrieval by embedding: the design study (session 14)";
    its four questions answered the same day, every one the recommended
    option (decision AW40: the prototype measures the three chunk texts
    first; `index --chunks` and the MCP tool `chunks`; the id the hash
    of the embedded text; module and test chunks in 37b, the documents
    and the example in 37c). 37a done (decision AW41, 2026-10-13): the
    prototype and the measure on forty-five questions over the corpus
    and the compiler, 1,747 chunks, with bge-m3, all-MiniLM-L6-v2 and
    BM25; the header text settled (bge-m3 hit@1 68.9%, hit@5 91.1%;
    BM25 on the same text hit@1 71.1%). 37b done (decision AW42,
    2026-10-13): `renyi_index::chunks`, `renyi index --chunks` (JSON
    Lines, one chunk per module and per definition, the id the hash of
    the text) and the MCP tool `chunks`, held by tests and checked
    against the prototype byte for byte. 37c, the extras (the
    language's documents as chunks, the example pipeline in
    `starter/`) done (decision AW43, 2026-10-13): `renyi index --chunks
    --language` (the cheat sheet's and the reference's sections, the
    library's modules and public declarations, from the binary's own
    copies), the tool's `language`, `starter/retrieval/retrieve.py`
    with its README. Stage 37 closed; 35c follows.
38. **Release 0.2.0** (session 14, 2026-10-13, done; decision AW36,
    `docs/RELEASE.md` sections 3 and 7). The version bumped with the
    fixture's hash (commit 6c999d1, the gates and CI green), the eight
    crates on crates.io in two minutes, the tag `v0.2.0` and its
    release page (the three archives, their checksums, the `.vsix`),
    the extension on Open VSX and, through the owner's browser the
    same evening, on the Visual Studio Marketplace as
    `renyi-lang.renyi` (the publisher `renyi-lang` created, the
    `.vsix` uploaded by hand, no token), `cargo install renyi` and
    `install.sh` checked against them, the social preview uploaded.
    The owner's step after it: the announcements from the texts
    written in the lane.
39. **After release 0.2.0: the measures, the games, the article**
    (session 14, 2026-10-14, decision AW45). The measures of the
    positioning's section 5 on release 0.2.0: reproduction five of
    five (the starter workflows recorded on Windows, reproduced on
    Linux), the first-run rate one, one, one, three and two attempts
    on five tasks with the skill file alone (the gaps named are one
    bullet more in the skill file), the review time the owner's and
    pending. The repository `renyi-lang/renyi-games`: Snake, Blocks,
    Sokoban and Mines over `std.screen`, written by subagents from the
    skill, the cheat sheet and the showcase game, each with examples
    and tests (68, 61, 89 and 53 pass) and a demo line CI compares,
    the page at renyi-lang.github.io/renyi-games driven by the main
    site's module. The article `paper/renyi.tex` (eleven pages, builds
    with `latexmk`; the citations checked; for arXiv cs.PL with cs.AI).
    The repository `renyi-lang/renyi-tools` the same day: a static
    site generator with a Markdown converter, a to-do list in a JSON
    file, shared bills from a CSV, each with pure modules under
    examples, its effects tested through recordings, sample data with
    the expected outputs CI compares (61, 81 and 41 tests pass). Two of
    its three subagents found that a scope of the current directory,
    `filesystem.read(".")`, refused every relative path at 0.2.0 (the
    scope normalises to the empty text and the prefix test then asked
    for a leading separator): fixed on `main` in the checker's
    `scope_contains` and its mirror `compiler/effects.ry`, the
    reference's sentence and a semantics test. Open: the
    Marketplace's verified domain (the TXT record through `cf.py txt`,
    which waits for the record's name and text from the publisher's
    page in the owner's browser), the Open VSX namespace claim (the
    text in the measure lane, to be opened from the owner's account).
40. **The C API** (session 15, opened 2026-10-15; the design study in
    the handoff, its four questions answered, decision AW46). The
    embedding API of AP1 and AP2 for C hosts and the languages that
    call C. 40a done (decision AW47, 2026-10-15): the crate `renyi_c`,
    a shared and a static library over `renyi::Sandbox` with the
    hand-written header `include/renyi.h` held equal to the exports
    by a test, the values as JSON texts both ways through
    `Sandbox::call_json` (each argument decoded against its parameter's
    declared type before anything runs), the statuses 0 to 5, the
    module described (`renyi_module`, `renyi_grant`, `renyi_warnings`,
    `renyi_functions`), the counting allocator declared by the library;
    seven Rust tests over the exports, the C example `examples/host.c`
    compiled and run in CI against `host.expected`. 40b done (decision
    AW48, 2026-10-15): `bindings/python/renyi.py` over `ctypes` with
    `example.py` run in CI, the archive `renyi_c-<version>-<target>`
    per target in the release workflow, `renyi_c` the ninth crate of
    the release procedure, a Windows path with backslashes fixed in
    `Sandbox::load`; the measure: a call 36 to 44 us from Rust and
    through the C API, 51 to 58 us from Python, the load 3 ms and 10
    ms. 40c when a host asks: the screen callback, the value handles
    beside JSON.
41. **The Python bridge's items** (session 15, opened 2026-10-15 by
    the owner's order after stage 40, "按顺序推1到3"; the design study
    in the handoff, its four questions answered, decision AW49). The
    three candidates of track E since session 8. 41a done (decision
    AW50, 2026-10-15): the deadline per call from the manifest's
    `python` section (`"deadline": "5 seconds"`, a count and a unit,
    an entry's own winning for its module), carried in the binding
    through the checker into the bytecode file (`"deadline"` in
    milliseconds) and the binary encoding, read the same way by the
    resolver written in Renyi; the worker's output read by a thread of
    its own so a call waits on a channel with the deadline, the worker
    killed past it and started again at the next call; `TimedOut(after:
    Duration)` the fifth variant of `PythonError`; the conformance
    fixture's `stall` past a deadline of one second and a binary test
    with an entry's deadline of 250 milliseconds. 41b next: the record
    types from dataclasses, `NamedTuple`s and `TypedDict`s in the
    binder and the worker. 41c: the item "one worker across a test run"
    closed as done (the tests of a file share the `Vm` and its worker,
    measured in the study; each file of a run keeps its own).

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
  stages 3 and 4, the checker's own cuts between them and stage 5a
  (AW2 to AW19), every stage of AW1's order done: the ratio by
  instructions 3.46 on the check row against the gate's 3; stage 31
  (AW20, AW21), road (B), the new code generator on the Cranelift tier
  in four measured stages, in progress: B2 done (AW22, the check row to
  1,482.3, -3.0%, the ratio to 3.36), B3 measured and left off and the
  leak of stages 2b to B2 fixed (AW23, the check row 1,493.5, the
  ratio 3.38), B1 done (AW24, the check row to 1,482.7, -0.7%,
  the ratio to 3.36), B4 done (AW25, the check row to 1,437.8, -3.0%,
  the ratio to 3.26); the four stages measured; stage 32 (AW26),
  the second round of the checker's own cuts on the same binary,
  done (the check row to 1,307.5, -9.1%, the ratio to
  2.96, within the gate of 3); the round closed by the owner's decision
  (AW27, 2026-10-13: the switch of the front end deferred, the Rust
  front end kept).
- **E. Packages, the FFI and Python**: stages 15, 18, 41. The deadline
  per Python call done (41a); the record types from Python classes in
  the binder next (41b); one worker across a test run closed as done
  (41c).
- **F. The agent tooling and the integrations**: stages 5, 7 (the MCP
  server), 19 (the resident world, the language server, the watch, the
  embedding API), 35 (completion, references and rename in the
  language server; done), 37 (the map for retrieval by
  embedding, opened 2026-10-13, its study done and its questions
  answered, AW40; 37a the measure done, AW41; 37b the export done,
  AW42; 37c the extras done, AW43; the stage closed), 40 (the C API
  of decision A5, opened 2026-10-15, AW46; 40a the crate, the header
  and the C example done, AW47; 40b the Python binding, the archive
  and the measure done, AW48; 40c when a host asks).
- **G. Release and the ecosystem**: stages 17, 30, 38, 39. Releases
  0.1.0 (2026-10-07), 0.2.0 (2026-10-13), 0.2.1 (2026-10-14) and 0.3.0
  (2026-10-15, the C API) are out, the extension on
  the Marketplace and Open VSX since 0.2.0; every release follows
  `docs/RELEASE.md`; the icon and the wordmark (AV1) are in the site,
  the README, the extension and the repository's social preview; two
  of the three measures of the positioning's section 5 are taken
  (AW45), the review time is the owner's; the games repository and the
  article exist (stage 39); the announcements (the texts written), the
  tools repository, the verified domain and the namespace claim are
  open.
- **H. Readability**: stages 2, 6, 8. Four rounds exist under
  `tests/readability/`; they run only on request (V1).

## 4. What is next

Stage 29 (decisions AU58 and AW1): the new backend's stages in the order
the study set and the owner confirmed: stage 0 is done (AW2, 2026-10-11:
the allocator entered by mimalloc's plain functions, the module
constants an image builds before any run; the check row 3,085.3 to
2,889.9 million instructions, -6.3%, the ratio by instructions 6.99 to
6.54), and stage 1 is done (AW3 to AW6, 2026-10-11 and 2026-10-12: a
typed parameter's boxed operand taken from the stack at the call site,
the check row 2,889.9 to 2,826.6, -2.2%, the ratio 6.54 to 6.40; the
frame's leave by the static ownership state of the slots, the check row
to 2,729.4, -3.4%, the ratio to 6.18; a read-only parameter passed
borrowed between compiled bodies, the check row to 2,719.3, -0.4%, the
ratio to 6.16; the frame record of a direct call made cheaper, the check
row to 2,681.0, -1.4%, the ratio to 6.07), and the character tables with
the hasher of the VM's own are done (AW7, 2026-10-12: the check row to
2,527.2, -5.7%, the ratio to 5.72), and stage 2a is done (AW8,
2026-10-12: `with` by static index, the check row to 2,370.7, -6.2%, the
ratio to 5.37), and stage 2b is done (AW9, 2026-10-12: a record of two
fields returned as a pair, the check row to 2,301.9, -2.9%, the ratio to
5.21), and stage 2c is done (AW10, 2026-10-12: the frees in place, the
check row to 2,296.2, -0.2%, AT1's estimate -1.3%, the ratio to 5.20),
and stage 3a is done (AW11, 2026-10-12: the natives' in-place entry, the
check row to 2,203.9, -4.0%, the ratio to 4.99), and stage 3b is done
(AW12, 2026-10-12: `status` and `into_plain` off the path, the check row
to 2,099.1, -4.8%, the ratio to 4.75), and stage 4a is done (AW13,
2026-10-12: `is` and `is not` in place, variants compared in the
generated code by the type and the tag, the check row to 1,995.8, -4.9%,
the ratio to 4.52), and stage 4b is done (AW14, 2026-10-12: the iterator
in two slots, the check row to 1,930.3, -3.3%, the ratio to 4.37), and
stage 4c is done (AW15, 2026-10-12: `match` by a table, the check row to
1,896.4, -1.8%, the ratio to 4.29), which completes stage 4, and the
checker's own cuts between the stages are done (AW16, 2026-10-12: the
variants indexed, the parser's cursor with its token, the lexer's
previous token as two Integers and `slice_text`, the resolver's loop,
the lists updated in the slot; the check row to 1,586.1, -16.4%, the
ratio to 3.59), and stage 5a is done (AW17, 2026-10-12: the moves of
AU17 written in the image and the cache, the check row to 1,547.9,
-2.4%, the ratio to 3.51), and stage 5b is done (AW18, 2026-10-12: the
code objects decoded on demand, the check row to 1,528.5, -1.3%, the
ratio to 3.46), which completes stage 5, and stage 6 is done (AW19,
2026-10-12: the expansion measured again under the new convention, +3.7%
on the check row with it on, left off), which completes AW1's order;
each measured by the checker's run under callgrind with the table by
category, the ratio of AU36 (iii), AT1's rows where the tiers change,
the four judges and the conformance suite on both tiers, kept by AT1's
rule, toward the gate of AU35 (ii) at 3 times on the check row (3.46 by
instructions after stage 6, every stage of AW1's order done); then road
(B), the owner's choice of 2026-10-12 (AW20): the new code generator on
the Cranelift tier in four measured stages (the operands in registers,
the locals in registers, the frame record lazy, the counts by dataflow;
the study's estimate 340 million, the check row to about 1,190 million,
2.7 times the reference), of which B2 is done (AW22, 2026-10-12: the
operands in registers, the check row to 1,482.3, -3.0%, the ratio to
3.36) and B3 measured and left off with the leak of stages 2b to B2
fixed (AW23, 2026-10-12: the check row 1,493.5, the ratio 3.38) and B1
done (AW24, 2026-10-13: the frame record lazy, the check row to 1,482.7,
-0.7%, the ratio to 3.36) and B4 done (AW25, 2026-10-13: the counts by
dataflow, the check row to 1,437.8, -3.0%, the ratio to 3.26; the four
stages measured), and after them stage 32 (AW26, 2026-10-13: the second
round of the checker's own cuts, the check row to 1,307.5, -9.1%, the
ratio to 2.96, within the gate of 3); the owner's decision after it
(AW27, 2026-10-13): the switch of the binary's front end to the compiler
written in Renyi is deferred although its gate is met, the Rust front
end stays, and the speed round closes. The owner's choice after it
(AW28, 2026-10-13): the WASM target of M6 (stage 33, its design study
first), then the language server's completion, references and rename,
then the C API of decision A5, in that order, with the showcase game in
parallel (stage 34: a Doom-style raycaster in Renyi by a subagent, the
library module `std.screen`, `present` suspending the program for the
page, the game played in the browser after stage 33). Stages 33 and 34
are done (decision AW33, 2026-10-13: the play page of the site); the
language server's completion, references and rename are stage 35,
its design study done on 2026-10-13 and its four questions answered
(decision AW34); stage 36, the DOOM look, was opened the same day in
parallel with it (decision AW35: the owner's call after the play
page) and is done (decision AW37: the game's page of the site at 55
to 60 frames a second in the browser); 35a and 35b are done
(decisions AW38 and AW39: references and rename of declarations,
then of the locals), so the next stage is 37, the map for retrieval
by embedding (the owner's request of 2026-10-13; its design study is
in the handoff and its four questions answered, decision AW40; 37a
measured the chunk texts and settled the header, decision AW41; 37b
built the export, `renyi index --chunks` and the tool `chunks`, decision
AW42; 37c the language's documents and the example pipeline, decision
AW43; the stage closed); 35c, completion with signature help and
workspace symbols, is done (decision AW44), which closes stage 35.
Release 0.2.0 is out (stage 38, 2026-10-13: the crates, the tag, the
release page, the extension on Open VSX and the Marketplace, the social
preview; the announcements are the owner's step, the texts written), and
stage 39 followed it (AW45, 2026-10-14: the measures, the games
repository, the article; `renyi-tools` the same day). Release 0.2.1
followed (2026-10-14: the scope of the current directory fixed, the
toolchain on the official MCP registry as `org.renyi-lang/renyi`, Open
VSX verified; `docs/RELEASE.md` section 8), the article went through
the de-AI pass, and the namespace claim is done. Release 0.3.0 followed
on 2026-10-15 (the C API and its Python binding, the archive per
target, `renyi_c` the ninth crate, the registry and Open VSX at 0.3.0;
`docs/RELEASE.md` section 9). Open from AW45: the Marketplace's
verified domain (not before 2027-04-13), the article's submission by
the owner (after 0.3.0, so now open), the awesome list's Glama step
(done: the three pull requests carry the badges).
The C API of decision A5 is stage 40, opened on 2026-10-15 by AW28's
order (AW46: JSON texts across the boundary, the crate `renyi_c` with
a hand-written header, C and Python first, a library archive per
target); 40a (AW47: the crate, the header, the C example in CI) and
40b (AW48: the Python binding, the archive per target, the ninth
crate, the measure) are done, and the crate goes to crates.io with
the next release; 40c, the screen callback and the value handles,
waits for a host that asks. Not scheduled: the
Python binder's items of track E, the Rust front end's name sets as
bits, the emitter's own round for the compile row.
