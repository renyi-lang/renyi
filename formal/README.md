# RenyiCore: Theorem 1 of the core calculus in Lean 4

A mechanization of Theorem 1 (the authority bound) and Proposition 1a of
`docs/research/core-calculus.md` (section 2), over a small model of the
calculus. Lean core only: no Mathlib, no Batteries, no dependencies.

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
member covers the effect).

## Files

- `RenyiCore/Coverage.lean`: kinds, scopes, capabilities, coverage, grants,
  the grant stack's coverage, and a concrete scope instance (path scopes).
- `RenyiCore/Machine.lean`: programs, the static rule `WellChecked`, the
  machine (`Step`, `Run`), and two grant models.
- `RenyiCore/Authority.lean`: the invariant, Theorem 1 in both models,
  Proposition 1a.

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

Neither form of Theorem 1 uses `WellChecked`, as section 2 says.

## The assumptions A1 to A8

- **A1** (no ambient effect): embodied. The only effects are `prim`
  instructions; nothing else in the machine produces an `effect` event.
  `foreign` and `python` are not modelled.
- **A2** (`time`, `random` as capabilities): embodied trivially, as
  primitive calls of some kind.
- **A3** to **A6** (iteration order, arithmetic, sequential tasks, no
  identity): not modelled. They concern determinism (Theorem 3), and the
  machine has no values, no concurrency and no pure steps beyond control.
- **A7** (budgets and guards decided by the state): not modelled. There are
  no budgets and no guards (Theorem 2).
- **A8** (`scope_p` reads the actual scope; paths by normalized text): the
  actual scope is a field of the `prim` instruction, fixed rather than
  computed from arguments, and scopes are compared by `ScopeOrder.le`, a
  relation on values with nothing resolved on disk.

## Simplifications relative to core-calculus.md

- Needs are closed: no scope that names a parameter, no substitution at
  the call.
- A primitive's actual scope is static in the instruction, not a function
  of run-time arguments; the static rule ignores it, so the difference does
  not weaken the theorems.
- No values, arguments, results, failures-as-values, function values,
  tests, packages (the cross-package kind-only rule) or the run's
  arguments. A refused call continues the frame; a crash is a run that stops
  there, which the prefix-closed `Run` covers.
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
