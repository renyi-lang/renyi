# RenyiCore: Theorems 1, 2 and 3 of the core calculus in Lean 4

A mechanization of Theorem 1 (the authority bound) and Proposition 1a,
Theorem 2 (guards admit only their sinks) and Theorem 3 (replay) of
`docs/research/core-calculus.md` (sections 2, 3 and 4), over small models
of the calculus. Lean core only: no Mathlib, no Batteries, no dependencies.

## Build

```
lake build
```

Checked with `Lean (version 4.35.0, x86_64-w64-windows-gnu, commit
eb6cafb7290e8bd5f2937522d8d804f58f02bafc, Release)`, Lake 5.0.0
(`lean-toolchain`: `leanprover/lean4:v4.35.0`). The build has no warnings
and the sources contain no `sorry`. `#print axioms` gives `propext` and
`Quot.sound` for both forms of Theorem 1, and those with
`Classical.choice` for Proposition 1a (one case split on whether a stack
member covers the effect); `propext` alone for Theorem 2, for Theorem 3
and for its corollary, and no axiom at all for `origins_iff_prov`.

## Files

- `RenyiCore/Coverage.lean`: kinds, scopes, capabilities, coverage, grants,
  the grant stack's coverage, and a concrete scope instance (path scopes).
- `RenyiCore/Machine.lean`: programs, the static rule `WellChecked`, the
  machine (`Step`, `Run`), and two grant models.
- `RenyiCore/Authority.lean`: the invariant, Theorem 1 in both models,
  Proposition 1a.
- `RenyiCore/Guards.lean`: guards, origin sets, values as the record of
  their computation, the explicit provenance, a stack machine with the
  outgoing check, Theorem 2.
- `RenyiCore/Replay.lean`: a machine of pure steps and primitive calls,
  the live run and its recording, the replay with the first-unused match,
  Theorem 3 and its corollary.

Theorem 2 and Theorem 3 each have a machine of their own, which leaves the
files of Theorem 1 untouched; `Guards.lean` reuses `Coverage.lean`'s
capabilities, coverage and grants, and `Replay.lean` imports nothing.

## What is modelled

- **Kinds**: a kind is a path (`List String`); `Kind.Below k k'` iff `k'` is
  a prefix of `k`. The finite tree of reference section 11 is not fixed: any
  path is a kind.
- **Scopes**: an abstract type with a decidable preorder `le` (`le s t`:
  `t` contains `s`) and a top `⊤` (class `ScopeOrder`). One scope type for
  every kind, not a set `S_k` per kind. Antisymmetry is not needed. An
  instance for path scopes (`Option (List String)`, `none` the top, prefix
  containment) shows the class is inhabited by the intended model.
- **Coverage**: `covers g c` iff `c.kind` is at or below `g.kind` and
  `c.scope ⊑ g.scope` (`effects.rs`, `Capability::covers`, scope against
  scope). A grant is a list; it covers `c` iff a member does.
  `kindCovers G k` is the kind-only coverage the checker uses for a library
  primitive (`covers` with `runtime_scoped = true`).
- **Programs**: functions of an arbitrary type `F`, each with closed needs
  and a body of instructions `call g` or `prim k σ` (a primitive call of
  kind `k` whose actual scope is `σ`).
- **Static rule** (`WellChecked`): a `call g` in `f` needs `N(f)` to cover
  every capability of `N(g)`; a `prim k σ` in `f` needs `N(f)` to cover the
  kind `k` (scopes not compared, as the checker does for primitives).
- **Machine**: a list of frames, each holding the function, the rest of
  its body and its own grant (as each VM frame holds a grant index). A call
  pushes the callee's frame with `push E N(g)`; a primitive call is
  admitted iff the frame's grant admits `(k, σ)`, producing the event
  `effect (k, σ)`, else the event `refused (k, σ)` and no effect; a body
  that is done returns. `Run` is the reflexive-transitive closure with the
  trace of events. A `Run` is finite, but every finite prefix of an
  infinite run is one, so the statements cover every effect of every run.
- **Two grant models** (`GrantModel`: `push`, `admits`, and the law that
  `push` only narrows):
  - `stackModel`, the grant stack of decision Q1 and section 2: a grant is
    a list of grants, admitting `c` iff every member covers it (the
    intersection `∧`, by the property section 1 states). A function with
    no needs pushes nothing, as `vm.rs` does (`narrows` false,
    `frame_grant` returns the enclosing index).
  - `withinModel`, the implementation's `grant::within`: a frame's grant is
    one flat grant `within E N`. `within` is not the intersection: a
    granted capability on a line the needs do not mention passes through.
    The model takes `within` as an abstract function with one hypothesis,
    soundness (what `within E N` covers, `E` covers). That `grant.rs`'s
    `within` satisfies it is an assumption, not proved here.

## What is proved

- `effects_admitted_by_initial`: in any grant model, every effect of every
  run from `main`'s frame under `E₀` is admitted by `E₀`. No static rule.
- `authority_bound` (Theorem 1): in the stack model, with initial stack
  `[N(main), G₀]` for any host grant `G₀`, every effect in the trace is
  covered by `G₀` and by `N(main)` (scope included, which implies the
  section's kind-with-`⊤` claim).
- `authority_bound_within` (Theorem 1 for `within`): with `main`'s frame
  under `within G₀ N(main)`, every effect is covered by `G₀`; and if `G₀`
  is covered by `N(main)` (as `grant::effective` produces it), by `N(main)`.
- `refused_only_on_scope` (Proposition 1a): in the stack model and a
  well-checked program, when the innermost frame's next instruction is
  `prim k σ` and its stack `E` refuses it, then every grant of `E` other
  than `G₀` covers the kind `k`, and either `G₀` does not cover `(k, σ)` or
  some grant of `E` covers `k` but not `(k, σ)`. The refusal is on the
  scope unless the host's `G₀` itself lacks the kind (a `--deny`), which the
  paper statement leaves implicit.
- `origins_iff_prov` (section 3's "by construction `O(v) = P(v)`"): the
  origin set the guards' machine computes for a value is the explicit
  provenance, for every value.
- `guards_admit_only_sinks` (Theorem 2): in every run of the guards'
  machine, for every primitive call that happens (an `effect e a` event)
  with argument `a`, every guard in the explicit provenance of `a` has a
  sink covering `e`. No hypothesis on the starting configuration.
- `replay_tracks_live` (the induction of section 4): after `k ≤ n` steps,
  the replay under the recording of a live run of `n` steps is in the
  live run's state, has made its calls with their answers, and has used
  exactly the first entries of the recording.
- `replay_run` (Theorem 3): the replay of `n` steps under the recording of
  the live run of `n` steps from the same initial state succeeds, reaches
  the same state, makes the same sequence of primitive calls with the same
  answers (its calls are the recording), and uses every entry.
- `replay_output` (the corollary): for any `output` that is a function of
  the calls made (the console's among them), the replay's output is the
  live run's.

Neither form of Theorem 1 uses `WellChecked`, as section 2 says.

## Theorem 2: guards

What is modelled (`Guards.lean`, section 3):

- **Guards**: an abstract type `Γ` (the implementation has at most 64, one
  bit each), with each guard's capability `cap g` and its sinks `sinks g`,
  a grant (`grant.rs`, `Guard`). An origin set is a predicate `Γ → Prop`,
  the `u64` of `Value::origins`; the empty set is a literal's and `union`
  is the `|` of the implementation.
- **Values**: the record of their computation. `lit` is a literal; `op a b`
  the result of any operation other than a primitive call on two operands
  (arithmetic, construction, a field read, a function's return: every
  operation of section 3's second rule, an operation of more operands
  being a tree of these); `prim e a` the result of an admitted primitive
  call of effect `e` on the argument `a`. A call's arguments are one value,
  the tuple of them built by `op`, whose origins are the union of theirs,
  so the statement over `⋃ O(aᵢ)` is the statement over `O(a)`.
- **Origins and provenance**: `Val.origins` computes `O(v)` from the record
  by the three rules (`lit`: none; `op`: the union; `prim e a`: the union
  of `a`'s and `through e`, the guards whose capability covers `e`, which
  is `Vm::origins_through`). `Prov` is the least relation closed under the
  same rules. `origins_iff_prov` proves them equal.
- **The outgoing check**: `admitsAll gs O e` iff every guard in `O` has a
  sink covering `e` (`Guard::admits`; `Vm::blocking_guard` finds none).
- **The machine**: a stack machine; a configuration is the code left and
  the stack of values. `lit` pushes a literal, `dup` uses a value twice,
  `op` combines the two top values, `prim e` calls a primitive on the top
  value: when the check passes the event is `effect e a` and `prim e a` is
  pushed; when it fails the event is `refused e a`, nothing leaves, and a
  plain value is pushed (`Vm::refused` builds the failure from texts, with
  no origin bits; a primitive that cannot fail crashes, a run that stops
  there). `test t f` pops the value it tests and continues with either
  branch, chosen nondeterministically: the tested value is dropped, so its
  origins reach neither branch, as in the implementation
  (`Value::with_origins`: a Boolean carries no origins).
- **Runs**: as in `Machine.lean`, finite, prefix-closed.

What Theorem 2 does not say, as section 3 says: nothing about implicit
flow. A branch on a guarded value gives the branch's result the origins
of what the branch computes, not of the value tested; the model's `test`
reproduces that (the branch chosen carries nothing of the value), and the
theorem is only about the explicit provenance `Prov`. A program that
prints "yes" when a guarded value starts with "a" leaks one bit per run in
the model as in the implementation. Nothing about timing or the length of
what is sent.

What the model leaves out: the grant's own admission check (Theorem 1's
machine has it; here a call is checked against the guards only, and the
two checks are independent in `Vm::call_primitive`, the grant's first);
the composition of the two machines; the implementation's `covers(effect,
true)` for a sink or a guard's capability, which lets a scoped sink cover
an effect that has no scope (`effects.rs`, `runtime_scoped`), is `covers`
of `Coverage.lean` here, scope against scope with `⊤` for an unscoped
effect, which is stricter for that one case; the 64-guard limit; that the
guards come from the declaration, not the narrowed grant; a `Failure`
that carries origins (the model's refused call leaves a plain value, as
the implementation's does); the sandbox's guards; and the text that leaves
through the console outside a call (a crash message), which the
implementation checks the same way.

## Theorem 3: replay

What is modelled (`Replay.lean`, section 4):

- **The machine**: abstract types `State`, `Prim`, `Args`, `Answer`, and
  three functions. `request : State → Option (Prim × Args)` says whether
  the next step is a primitive call and which, with which arguments (the
  state determines both); `pure : State → State` is a pure step;
  `apply : State → Answer → State` is the step of a primitive call given
  its answer, a value or a failure. The lemma of section 4 (pure steps are
  deterministic) is a hypothesis of the model, embodied by `pure` being a
  function: that is where A1 to A6 enter. The grant, the budgets and the
  values' origins are folded into the state (A7): the admission check, the
  budgets and the outgoing guard check are inside `request`, and a call
  they refuse is a pure step, since nothing leaves and nothing is recorded
  (`Vm::denied`, `Vm::refused`). A halted state is a fixed point of
  `pure`. The one restriction on the state is that primitives and
  arguments have decidable equality, which the matching needs.
- **The live run**: `live M w s n`, the configuration after `n` steps from
  `s` under the world `w : Nat → Answer`, the answer to the `j`-th call,
  and the recording so far, the `(p, a, r)` entries in order. Any sequence
  of answers is such a function, so every live run of the machine is a
  live run under some world; the world's answer may depend on anything
  (its own state, what the program sent), since `w` is arbitrary.
- **The replay**: `replay M s R n`, the configuration after `n` steps from
  `s` under the recording `R`, with the calls it made, or `none` when it
  failed. The book is the recording with its used entries crossed out
  (`List (Option Entry)`); `takeFirst p a` finds the first unused entry
  with the primitive `p` and the arguments `a`, by arguments first and by
  sequence when several match, and crosses it out; the replay fails when
  there is none.
- **The induction**: `replay_tracks_live`, on the number of steps `k ≤ n`,
  as the paper proof. The invariant adds to the paper's what the book
  looks like: the first `j` entries used and the rest unused, `j` the
  number of calls completed, so the first unused entry with the `j`-th
  call's primitive and arguments is the recording's own `j`-th entry
  (`live_split`, `takeFirst_at`): the paper's "under A5 the calls complete
  in the order they are made".

What Theorem 3 does not say, as section 4 says: a replay shows that the
program still computes the same output from the same answers; it says
nothing about what the world would answer today.

What the model leaves out, and where it deviates from the statement:

- The replay starts from the same initial state as the live run, which
  includes the grant. The document's replay runs "under a grant covering
  `R`'s header"; the model does not prove that a different grant covering
  the header decides every recorded call alike, since the state is
  abstract. The same initial state is the stronger hypothesis.
- Redaction (a redacted argument matches any value) is not modelled: the
  match is by equality of primitive and arguments.
- The output is any function of the list of calls made (`replay_output`);
  the console's calls are among them, and nothing else of the output is
  modelled.
- `renyi reproduce`'s corollary (the hash of `main`, the dependencies, the
  comparison of outcomes) is not modelled.
- The number of steps is given (`n`); a run of the machine that halts is a
  fixed point of `pure`, and the statement is for every `n`, so it covers
  every finite run and every finite prefix of an infinite one.
- A5 (tasks one after the other) is embodied by the machine having one
  sequence of steps; the document's remark that the matching by arguments
  keeps Theorem 3 under an overlapping scheduler is not a statement here.

## The assumptions A1 to A8

- **A1** (no ambient effect): embodied. The only effects are `prim`
  instructions; nothing else in the machine produces an `effect` event.
  `foreign` and `python` are not modelled. In the replay model, every
  effect is a primitive call step, and every other step is a function of
  the state.
- **A2** (`time`, `random` as capabilities): embodied trivially, as
  primitive calls of some kind; in the replay model, as calls whose
  answers the world gives and the recording holds.
- **A3** to **A6** (iteration order, arithmetic, sequential tasks, no
  identity): hypotheses of the replay model, not proved. They are what
  makes a pure step a function of the state, and the model takes `pure`
  as a function. Theorem 1's machine has no values, no concurrency and no
  pure steps beyond control. A5 is also embodied by the replay model's
  single sequence of steps, which is why the recording's `j`-th entry is
  the `j`-th call's.
- **A7** (budgets and guards decided by the state): in the guards' machine
  the outgoing check reads the origins the values carry, and the origins
  are computed from the values' records, so the check is decided by the
  configuration; in the replay model the grant, the budgets and the
  origins are folded into the state by construction, and a refusal is a
  pure step. Budgets are not modelled beyond that.
- **A8** (`scope_p` reads the actual scope; paths by normalized text): the
  actual scope is a field of the `prim` instruction, fixed rather than
  computed from arguments, and scopes are compared by `ScopeOrder.le`, a
  relation on values with nothing resolved on disk. The guards' machine
  likewise takes the effect as a field of the instruction.

## Simplifications relative to core-calculus.md

- Needs are closed: no scope that names a parameter, no substitution at
  the call.
- A primitive's actual scope is static in the instruction, not a function
  of run-time arguments; the static rule ignores it, so the difference does
  not weaken the theorems.
- Theorem 1's machine has no values, arguments, results,
  failures-as-values, function values, tests, packages (the cross-package
  kind-only rule) or the run's arguments. A refused call continues the
  frame; a crash is a run that stops there, which the prefix-closed `Run`
  covers.
- `G₀ = G ∧ H` is not computed: in the stack model `G₀` is any grant and
  `N(main)` sits on the stack beside it; in the `within` model the relation
  of `G₀` to `N(main)` is a hypothesis.
- The intersection `∧` of section 1 is represented by the stack's
  conjunction of coverage, not by the computed set of greatest common
  capabilities (`grant::narrow`); the section's property of `∧` is thus a
  definition here, not a lemma.
- `within`'s soundness and `effective`'s output being covered by the
  declaration are hypotheses, not proofs about the Rust code.
- One scope order for all kinds instead of one per kind.
- The three theorems have three machines: Theorem 1's has frames and
  grants and no values; Theorem 2's has values with origins and no grant;
  Theorem 3's has an abstract state. Nothing here composes them.
