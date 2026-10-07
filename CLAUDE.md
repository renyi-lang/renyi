# Renyi: notes for Claude Code sessions

Read `docs/HANDOFF.md` first. It is the current state of the project and is
rewritten at the end of every session. Then read `docs/reference.md`, the
language reference, and the eight design documents under `docs/design/` in
order.

## What this repository is

Renyi is a programming language. Its design (M0) is complete; the Rust front
end (M1: lexer, parser, formatter), the type and effect checker (M2) and
the VM (M3: `renyi run`, `record`, `run --replay`, `reproduce`,
`--explain`, `--profile`, `test` with `replays`, budgets, scope checks, the grant stack
of decision Q1, the run manifest of Q2, every library module including
HTTP, the server and SQLite; tasks run one after the other by decision
S2) exist, and the compiler is written again in Renyi under
`compiler/` (the lexer, the parser, the checker and the bytecode
emitter, decisions W1 to W8 and Z1 to Z4), run by the Rust VM and held
equal to the Rust toolchain by tests. The owner
(GitHub `skymanbp`) makes design decisions and reviews; Claude writes the
documents, the example corpus, the Rust implementation and the Renyi
compiler.

## Conventions

- All documents, code comments and commit messages are in English (decision
  H5). Chat with the owner is in Chinese.
- Design decisions live in `docs/design/01-decisions.md` as append-only
  entries. A reversal is a new entry that names the entry it supersedes. Never
  edit an accepted entry.
- `docs/reference.md` is the language reference, normative for the
  surface syntax, the static rules and the run-time behaviour;
  `docs/grammar.ebnf` is the formal grammar of the syntax level, which the
  reference quotes rule by rule; `docs/design/02-syntax-sketch.md` is the
  design record both were derived from (its section 18 keeps the open
  items), and `docs/design/04-stdlib-sketch.md` is the source for library
  names. The surface is frozen by decision V11: a change to it is a new
  decision entry first, then the reference, the grammar, the cheat sheet,
  the formatter and the conformance suite in one commit; `cargo test -p
  renyi_syntax --test grammar` keeps the grammar equal to the parser, and
  `--test reference` keeps the reference's excerpts equal to the grammar
  file and its appendix A equal to the codes the crates emit. When an
  example needs something they do not define, extend the reference in the
  same commit (library additions are not surface changes).
- Every grammar change must keep `docs/cheatsheet.md` under 3000 tokens. Run
  `python3 tools/count_tokens.py` (requires `pip install tiktoken`). It exits
  non-zero over budget.
- Every `.ry` file under `examples/` must pass `python3 tools/lint_examples.py`
  with zero problems.
- When a design question needs the owner, ask with AskUserQuestion in batches
  of four, recommended option first and labelled "(Recommended)". The owner
  asked for this format explicitly and answers quickly in it.
- The implementation language is Rust, one binary named `renyi` (decision H1).
  The Python scripts in `tools/` are development aids only. Before a commit
  that touches `crates/`: `cargo fmt`, `cargo clippy --all-targets` and
  `cargo test` must be clean. The corpus tests also require every example to
  be in canonical layout: run `cargo run -- format examples/*.ry` after
  editing an example. The same checks, plus the conformance suite, run on
  GitHub Actions for every push (`.github/workflows/ci.yml`, decision V8).
- A new diagnostic or a new reference output gets a case in
  `tests/conformance/manifest.json`, which `cargo test` and
  `tools/conformance.py` both run.
- The Renyi sources under `compiler/` are programs like any other:
  `renyi check` clean, in canonical layout (run `cargo run -- format
  compiler/*.ry` after editing one; a test checks it) and within the
  size limits; `bench/*.ry` likewise (CI checks and formats them). `crates/renyi/tests/selfhost.rs` is the judge (decision
  W3): the Renyi parser must print, byte for byte, what `renyi parse
  --json` prints on every program of the corpus, the conformance suite,
  `compiler/` and (with `--declarations`) `library/std/`; and the Renyi
  checker must print, byte for byte, what `renyi check --json` prints,
  with and without `--strict`, on every program of the corpus, the
  conformance suite and `compiler/`, and exit as it exits (decision W7);
  and the Renyi compiler must write, byte for byte, the bytecode file
  `renyi compile` writes on every program of the corpus, the conformance
  suite and `compiler/`, and refuse what it refuses (decision Z3); each
  judge runs its driver from the bytecode `renyi compile` writes of it
  when the test starts (`target/selfhost/front/`), and the Renyi
  compiler must write that file of itself.
  A change to the tree's shape is a change to `compiler/ast.ry` and to
  `crates/renyi_syntax/src/json.rs` in one commit (decision W1); a
  change to the bytecode is a change to `compiler/bytecode.ry`,
  `crates/renyi_vm/src/file.rs`, `crates/renyi_vm/src/compile/` and
  `compiler/emit.ry` in one commit; a new
  diagnostic, message or fix in `crates/renyi_check` is the same change
  in `compiler/declare.ry` or `compiler/bodies.ry` in one commit, and
  one in the lexer or the parser of `crates/renyi_syntax` the same
  change in `compiler/lexer.ry` or `compiler/parser.ry` (decision AD1).
- Every diagnostic carries a fix (decision D3): the lexer's, the parser's
  and the checker's helpers take the fix as an argument, and both
  conformance runners require a `fix:` line after every diagnostic.
- `main` is the only branch (owner's decision, 2026-10-05). Commit and push
  there directly; do not create other branches, local or remote.
- Before ending a session, rewrite `docs/HANDOFF.md` so the next session can
  start without this conversation.

## Where things are

| Path | Content |
|------|---------|
| `docs/design/01-decisions.md` | design decisions with reasons: rounds 1 and 2 (sections 0 to J), library (K), readability (L), pre-test (M), checker (N), runtime and agent tooling (O), signature capabilities (P), system-level commitments (Q), the live round (R), runtime dependencies and concurrency (S), the MCP server (T), readability round 2 (U), the gap audit and the road to self-hosting (V), the self-hosted front end (W), the speed of the VM (X), the residue of stage 1 (Y), the bytecode file and the emitter (Z), the rename of `set` (AA), constraints with type arguments (AB), packages (AC), the diagnostics of the front end in Renyi (AD), the process module (AE), the foreign function interface (AF), machine code for the bytecode (AG), the niche (AH), release 0.1 (AI) |
| `docs/design/02-syntax-sketch.md` | concrete syntax; section 18 tracks open items (round 2 is settled, new items start at R3-1) |
| `docs/design/03-readability-test.md` | the readability protocol: it measured the grammar before the freeze; decision V1 made the freeze a decision (V11), so the rounds run only on request |
| `docs/design/04-stdlib-sketch.md` | prelude, core modules and extension packages; the lint checks corpus calls against its `function` lines |
| `docs/design/05-agent-tooling.md` | the project map (`renyi index`), metrics, content hashes, budgets, diffs and the `renyi mcp` server |
| `docs/design/06-runtime-guarantees.md` | recorded runs and `replays` tests, narrated runs, budgets in grants (`at most`), provenance guards (`only to`) |
| `docs/design/07-system-design.md` | the trade-offs the language claims to resolve; capability-safe packages, reproducibility, in-process sandboxing, checked live update |
| `docs/design/08-positioning.md` | the niche (decisions AH1 to AH4): the scripting language of AI agents, the four points it leads with and the mechanism behind each, the syntax second, the comparison with the alternatives, the success metrics and the wording the README and the site derive from |
| `docs/cheatsheet.md` | the whole language on one page; token-budgeted |
| `docs/grammar.ebnf` | the formal grammar of the syntax level: W3C EBNF over the lexer's tokens, the token classes and the line rules in its preamble; `crates/renyi_syntax/tests/grammar.rs` interprets it and checks that it accepts exactly what the parser accepts |
| `docs/reference.md` | the language reference, normative: one section per construct with the grammar excerpt, the meaning, the static rules with their diagnostic codes and the run-time behaviour; appendix A every diagnostic code with its severity, appendix B the commands and exit statuses; `crates/renyi_syntax/tests/reference.rs` holds it to the grammar file and the crates |
| `docs/GAPS.md` | the gap audit of 2026-10-06: what the design promises and the implementation does not deliver, with the proposed order of work |
| `examples/` | the corpus, one program per file, index in `examples/README.md` |
| `tools/count_tokens.py` | cheat-sheet budget gate |
| `tools/lint_examples.py` | regex-level checks for the corpus, including calls against the library sketch |
| `tools/gen_foreign_abi.py` | writes `crates/renyi_vm/src/natives/foreign_abi.rs`, the fixed signature family of the FFI (decision AF1) |
| `tools/conformance.py` | runs the conformance suite against any `renyi` binary, without the Rust crates |
| `bench/`, `tools/bench.py` | the benchmarks of decision AG4: four programs with CPython twins next to them, the compiler's self-check and `hello`; the script times each with the binary, with `--interpret` and with CPython; CI prints the numbers and does not gate on them; `bench/*.ry` check clean and are in canonical layout like `compiler/` |
| `tests/conformance/` | the conformance suite (decision V8): `manifest.json` (cases: program, command, expected output, exit code, diagnostic codes), `expected/` (the outputs, the ten Predict references among them), `programs/` (programs written for one diagnostic or guarantee), `packages/` (the package fixture of decision AC1: a registry with `greeting` 1.0.0 and four projects), `foreign/` and `foreign_bad/` (the foreign fixtures of decision AF1: a project whose manifest binds `libc`, and one whose declarations the boundary refuses), `README.md` |
| `tests/readability/` | harness for the readability protocol: `run.py`, manifest, the Explain references, Write tasks |
| `.github/workflows/ci.yml` | continuous integration: format, clippy, tests, canonical corpus, lint, token gate, conformance suite |
| `crates/renyi_syntax/` | spans, diagnostics, lexer, AST, parser, JSON encoder, formatter; `tests/corpus.rs` runs the corpus through all of them, `tests/library.rs` parses the library declarations, `tests/diagnostics.rs` checks that every diagnostic carries a fix and that a foreign spelling gets the Renyi one, `tests/grammar.rs` interprets `docs/grammar.ebnf` over the corpus, the conformance programs, the library and two lists of corner programs, `tests/reference.rs` checks the rules `docs/reference.md` quotes and the codes it lists |
| `crates/renyi_json/` | the JSON value, reader and writer shared by the VM (recordings, bytecode files, `std.json`) and the package crate |
| `crates/renyi_package/` | packages (decision AC1): `version.rs`, `manifest.rs` (`renyi.json` with its `foreign` section, `renyi.lock.json`, `package.json`, `versions.json`: strict readers, one rendering), `registry.rs` (a directory or a URL, the store `.renyi/packages/`, the content hash, the textual `join`), `resolve.rs` (`Project::of`, `resolve`: a program's imports to its own files and its dependencies' files, tagged with their package), `select.rs` (the choice of versions: the same major, at least the version required, the highest the registry has); `tests/fixture.rs` holds the conformance fixture canonical |
| `crates/renyi_check/` | the type and effect checker (M2): `world.rs` declares modules, `check.rs` checks bodies, `effects.rs` covers capabilities, `refine.rs` evaluates refinements on literals, `suggest.rs` proposes the fixes (the closest name in scope, the Renyi spelling of a foreign name, the prelude's conversions), `foreign.rs` the C types of the boundary (decision AF1); `lib.rs` resolves a file's imports through `renyi_package` (`check_file`, `check_project_with_problems`); `tests/corpus.rs` and `tests/rules.rs` |
| `library/std/` | the standard library as Renyi declaration files (one per module), compiled into the checker; kept in step with `04-stdlib-sketch.md` by a test |
| `compiler/` | the compiler written in Renyi (decision W2), run by the Rust VM: `ast.ry` (the syntax tree, one type per construct, `can ToJson`; its derived JSON is the format of `renyi parse --json`, decision W1), `lexer.ry` (source text to tokens, the rules of reference section 1, with the Rust lexer's diagnostics, decision AD1), `parser.ry` (tokens to the tree, rule for rule as the Rust parser, with a cursor record threaded through every function that reproduces the Rust parser's line-break handling exactly, and the Rust parser's diagnostics: every code, message, fix and recovery point, decision AD1), `parse.ry` (`renyi run compiler/parse.ry [--declarations] <file>` prints the tree as JSON, then the diagnostics, as `renyi parse --json` does), `tokens.ry` (`renyi run compiler/tokens.ry <file>` prints the tokens as `renyi tokens` does), `bytecode.ry` (the bytecode file's format as Renyi types, decision Z1: a `.ryc` file is their derived JSON); the checker (decisions W5 to W8): `lists.ry`, `report.ry` (diagnostics, their text and JSON rendering, line starts and positions), `effects.ry` (capabilities and grants), `suggest.ry` (the closest name, foreign spellings), `types.ry` (the type representation, substitution, showing), `refine.ry` (refinements on literals), `declare.ry` (the port of `world.rs` and of `foreign.rs`: modules, types, functions, abilities, implementations, resolution, signatures, conformance, the foreign modules of decision AF1), `bodies.ry` (the port of `check.rs`: bodies, tests, examples, constants, conditions, exhaustiveness, queries, the recorded references), `checker.ry` (`renyi run compiler/checker.ry [--json] [--strict] [--library <dir>] <file>...` prints what `renyi check` prints); `project.ry` (what the command lines share: the files, the resolver of decision AC1 mirrored from `crates/renyi_package` (the manifest, the lockfile, the registry, the package tags), the library, `check_project` with the references of every body); the emitter (decision Z3): `emit.ry` (the checked tree to the `Program` of `bytecode.ry`, the transcription of `crates/renyi_vm/src/compile/` and `types.rs`), `compile.ry` (`renyi run compiler/compile.ry [--to <file.ryc>] [--library <dir>] <file>` writes what `renyi compile` writes); `crates/renyi/tests/selfhost.rs` holds the parser, the checker and the compiler equal to the Rust ones, on rejected programs too |
| `crates/renyi_index/` | the project map (`renyi index`): `lib.rs` builds the records from the checked program, `metrics.rs`, `hash.rs` (content hashes and the own-text hash), `budgets.rs`, `render.rs` (text and JSON), `diff.rs` (the semantic diff: changes, reach, version bump); a module or a definition of a dependency carries `package` (decision AC1); `tests/corpus.rs`, `tests/diff.rs` |
| `crates/renyi_vm/` | the VM (M3): `compile/` lowers the checked tree to bytecode (`bytecode.rs`) through the checker's recorded references, `vm.rs` runs it and holds the primitive boundary (`call_native`: grant and budget checks, recording, replay, narration; the loop of decision X3), `profile.rs` the profiler of `renyi run --profile` (decision X4), `grant.rs` the effective grant and budget counters, `recording.rs` the recording format, the run manifest (its `dependencies` since decision AC1), redaction and the replay, `natives/` the library primitives (`natives/foreign.rs` and the generated `natives/foreign_abi.rs` the FFI of decision AF1, the VM's only unsafe code), `runner.rs` runs `main`, examples and tests and reproduces a recording, `file.rs` the bytecode file (the encoder and the loader, decisions Z1 and Z2; `tests/file.rs`); `tests/corpus.rs` checks the ten expected outputs and every `example:` and `test` block, `tests/recording.rs` the boundary, `tests/guards.rs` the `only to` guards, `tests/semantics.rs` the promises of the gap audit (deadlines, refined updates, the boundary's failures, `equals`, the Float range, edge cases), `tests/network.rs` the server, the client and SQLite, `tests/queries.rs` the terminals after `group by`; `native/` the machine code of decision AG1: `infer.rs` the abstract interpretation that settles which operands and slots stay in registers, `codegen.rs` the translation of every op to Cranelift IR, `runtime.rs` the helpers the generated code calls and the hand-back to the interpreter, `mod.rs` the JIT and the hotness rule (`HOT_FACTOR` times the size in ops run; the code takes over at the next call or, through an entry at the loop header, at the next turn of a loop); `tests/native.rs` the hand-backs against the interpreter |
| `crates/renyi/` | the `renyi` binary: `check` (parse, type and effect check), `format`, `tokens`, `parse [--json] [--declarations]`, `index [--json \| --budgets \| --diff <map or revision>]` (`maps.rs` loads the base; `tests/index.rs`), `run [options] <file> [arguments]`, `run --manifest ...`, `record [--to file] [options] <file> [arguments]`, `reproduce <recording> [<file>]`, `test [--strict] [--refresh name [--redact name]] [--explain] <file>...`, `compile [--to <file.ryc>] <file.ry>` (the bytecode file, decision Z4; `run`, `record`, `test` and `reproduce` take a `.ryc` file in place of a source; `tests/compile.rs`), `add <name> [<version>]`, `update [--accept-effects]`, `audit`, `fetch` and `publish [--to <directory>]` (the package commands of decision AC1, `packages.rs`: the registry read in place or over HTTP, every package verified, the effect manifest recomputed through the index; `tests/packages.rs`), `bind <header.h> --module <name> --library <name>[,<name>...] [--to <directory>]` (a C header as a foreign module, decision AF1, `bind.rs`; `tests/foreign.rs`), `mcp [path]` (the toolchain for an agent host over standard input and output, `mcp.rs`; `tests/mcp.rs` drives the binary through both protocol eras), `tools [path]` (the tool manifest of decision D6; `tests/tools.rs`); `tests/conformance.rs` runs the conformance suite against the binary, every `run` case a second time from its bytecode file, `tests/selfhost.rs` the Renyi parser, checker and compiler against the Rust ones |
