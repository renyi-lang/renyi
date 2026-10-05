# Renyi System Design: Packages, Reproducibility, Sandboxing and Live Update

Status: design accepted (decisions Q1 to Q4), scheduled M3 to M5. Date:
2026-10-05. Companion to `05-agent-tooling.md` and `06-runtime-guarantees.md`.

This document records the system-level commitments that rest on Renyi's
four foundations: effects in the type system with scopes (B1, J11), immutable
values without shared mutable state (B3, E1), content-addressed definitions
(D5) and documentation as syntax (C8a). Section 1 states the trade-offs the
language claims to resolve, so that they can be measured and argued with;
sections 2 to 5 design the four capabilities the owner chose.

---

## 1. Trade-offs Renyi claims to resolve

Each row is a pair that mainstream languages treat as a choice; Renyi's
answer and the decision it rests on are in the last column. They are claims
to be tested, not slogans: the readability test measures the first two, the
corpus and M3 the rest.

| Trade-off | The two sides | Renyi's answer |
|-----------|---------------|----------------|
| Memory | Rust's predictability without pauses; a collector's ease | reference counting with in-place reuse of uniquely held values: no lifetimes in the language, no pauses (O1) |
| Expressiveness | Haskell's type power; Go's readability | algebraic types, generics, abilities, refinements and effects, with one spelling per concept, no overloading, no macros and size limits (C1, C4, D4) |
| Speed of writing | Python's brevity; Java's maintainability | full inference inside bodies; signatures, `purpose:` and `example:` mandatory at the boundary (B5b, C8a) |
| Concurrency | performance; freedom from data races | structured concurrency over values with no shared mutable state, so a race cannot be written (E1) |
| Debugging | a live system (Smalltalk, Erlang); static safety | recorded, replayable, narrated runs (P1) and checked live update (section 5) |
| Reuse | depending on others' code; knowing what it does | a dependency's effects are computed and must be granted (section 2) |
| Isolation | a sandbox; one process | the effect grant is the isolation boundary, in process (section 4) |
| Reproducibility | fast iteration; builds and runs that reproduce | hashes for code and dependencies, recordings for effects (section 3) |

## 2. Capability-safe packages (decision Q1)

### 2.1 The problem

A dependency in a mainstream ecosystem can do anything the process can do,
and nobody can say from its manifest what that is; supply-chain attacks
exploit exactly this. Runtime permission flags (Deno) are process-wide and
say nothing per dependency; audits (cargo-vet, crev) are social.

### 2.2 The design

- **The effect manifest is computed.** When a package is published,
  `renyi index` computes, for every public function, its transitive effects
  and failure types; the registry stores them with the package's content
  hashes and recomputes them on receipt, refusing a package whose claimed
  manifest differs. A package cannot understate what it does.
- **The grant covers the whole program.** The checker already requires a
  caller to cover its callees (J11). With packages, `main`'s grant must
  cover the transitive effects of every package function the program
  reaches. The error names the package, the function and the capability:
  "`http_client.fetch` (package `http_client` 1.2.0) needs `network.http`;
  add it to the `needs` of `main`, or choose another package". `renyi add`
  prints the package's effect summary before anything is written.
- **No silent widening.** `renyi update` diffs the effect manifests of the
  old and new versions (O4). A version whose effects widen is refused
  unless the program's grant already covers the new effects and the user
  passes `--accept-effects`; the lockfile pins content hashes (G1), so a
  republished version with new effects is a different hash and a refused
  update, never a quiet change.
- **Scopes narrow dynamically: the grant stack.** A general HTTP client
  package declares `network.http` without a scope; a program that grants
  `network.http("api.example.com")` may still use it. Across a package
  boundary the static rule checks the capability kind; the scope is
  enforced at run time by the grant stack: the effective grant of a running
  function is the intersection of the program's grant with the declared
  scopes along the call chain, and every primitive checks the actual path
  or host against the effective grant. The same mechanism serves the
  sandbox of section 4. Inside one program the static coverage rule of J11
  stays as it is.
- **Native code is visible.** A package that calls native code needs
  `foreign`; a program that grants `foreign` is told so by `renyi run` the
  first time ("this program can call native code through package X").
- **`renyi audit`** lists every dependency's transitive effects against the
  program's grant, and flags grants no dependency uses.

### 2.3 Where it sits

M4 (package manager and registry). The checker's coverage rule and the
index are in place; the grant stack is part of the M3 runtime because the
sandbox needs it too.

## 3. Reproducibility by construction (decision Q2)

A Renyi run is a deterministic function of its code, its dependencies and
its effects: time and randomness are effects and therefore recorded (P1),
maps and sets iterate in insertion order (K9), Decimal and Float arithmetic
are IEEE standard, and concurrent tasks' calls are matched to a recording
by arguments. What remains is to name the inputs.

- **The run manifest.** `renyi run --manifest run.json` (and every
  `renyi record`) writes: toolchain version, the content hash of `main`'s
  closure, the dependency hashes from the lockfile, the grant, the
  arguments, the names of environment variables read with hashes of their
  values, the recording's id when one was made, and the outcome.
- **`renyi reproduce run.json`** fetches the code by hash, replays the
  recording and compares the output byte for byte; a difference is a bug
  in the toolchain or a dependency on something outside the manifest, and
  the report says which call differed.
- **Reproducible builds.** `renyi build` output is a function of the same
  hashes; two builds of the same manifest are identical files (M6).

Scheduled with M3 (manifest, reproduce) and M4 (dependency hashes).

## 4. In-process sandboxing (decision Q3)

### 4.1 The use

An agent writes a Renyi module and a host program (Rust, Python, JS or C
through the embedding API of A5 and F1, or a Renyi program) runs it with a
grant it chooses: compute over data the host hands in, call one API, write
under one directory. Today this takes a process, a container or a Wasm
runtime; Renyi's effect system is the boundary itself.

### 4.2 The design

- **Loading with a grant.** The embedding API has one entry: load a module
  (source or compiled) with a grant, which is a list of capabilities with
  scopes, budgets (P2) and guards (P3), and call its exposed functions. The
  module runs on its own task with that grant on the grant stack; it can
  reach nothing the grant does not name, because there is no ambient I/O,
  no reflection and no native call without `foreign`.
- **Resources.** `within` deadlines (J12) bound time, budgets bound calls,
  and the VM takes a memory budget per loaded module (`at most 256
  megabytes memory` in the grant, open item R7-1 for the syntax); a module
  that exceeds it fails as a whole and the host is told.
- **Data.** Values the host hands in may carry origins (P3), so a sandboxed
  module can be given a secret it may use only toward one host.
- **Several modules, one process.** Each loaded module has its own grant;
  the structured concurrency tree carries grants, so a task inherits the
  effective grant of its parent and a module's tasks never widen it.
- **The same grant from the command line.** `renyi run --sandbox grant.json
  program.ry` runs a program under a grant narrower than its `main`
  declares (E3's `--deny` generalized).

### 4.3 What it does and does not promise

It isolates effects, time and resource use. It does not hide timing or
scheduling from the module, and `foreign` code, once granted, is outside
every guarantee. The document says so.

Scheduled after section 2 (the grant stack) with the embedding API at M5.

## 5. Checked live update (decision Q4)

### 5.1 The use

A backend service (`std.server`) gets a new version of its code without a
restart and without losing a request, and the new code is known to fit
before it runs: Erlang's hot code loading with static types.

### 5.2 The design

- **What changes is known.** A new version is type-checked and diffed
  against the running one (O4) by content hash: the set of changed
  definitions and whether any public signature changed.
- **No hidden state.** A Renyi module has no mutable state at the top level
  (top-level `let` is a constant) and no mutable value escapes a function,
  so there is nothing to migrate; the hard part of Erlang's hot swap does
  not exist. State that must survive a swap lives where it already lives:
  the database, files, the request itself.
- **Swapping.** `renyi serve` handles each request on a task; a swap
  replaces the code of the changed hashes atomically between requests. A
  task that started before the swap finishes on the code it started with
  (code objects are reference-counted like values; the old code is freed
  when the last such task ends). A definition on the stack of a
  long-running task (a `repeat until` loop in `main`) is swapped when that
  frame returns; the swap reports what it is waiting for and for how long.
- **Compatibility.** A swap whose diff changes a public signature that a
  running definition calls is refused with the signature named; adding
  definitions and changing bodies always swap. `renyi serve --watch`
  reloads on save during development; production deployment tooling
  follows at M5 or later.

## 6. Order of work

1. M3: the grant stack in the runtime (sections 2 and 4), the run manifest
   and `renyi reproduce` (section 3).
2. M4: the package manager with computed effect manifests, `renyi add`,
   `update --accept-effects`, `audit` (section 2); dependency hashes in
   the manifest (section 3).
3. M5: the embedding API with grants and memory budgets (section 4);
   `renyi serve --watch` and checked swaps (section 5).

## 7. Open items

- R7-1: the syntax of a memory budget in a grant (`at most 256 megabytes
  memory`, or a command-line and embedding-only setting).
- R7-2: who runs the registry and how its recomputation of manifests is
  trusted (signatures, reproducible builds of section 3).
- R7-3: whether a hot swap may replace a definition on the stack of a
  running task by applying at its next call (needed for `main` loops that
  never return).
- R7-4: the cost of the grant stack when effective grants are intersected
  on every cross-package call; a cached effective grant per call site is
  the expected answer.
