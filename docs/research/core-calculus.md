# The core of the guarantees: three statements and their proofs

Status: written 2026-10-16 (decision AX3), on paper; nothing is
mechanized yet. The implementation the statements describe is
`crates/renyi_vm/src/grant.rs` (coverage, narrowing, guards),
`crates/renyi_vm/src/vm.rs` (`frame_grant`, `call_native`, the origins)
and `crates/renyi_check/src/effects.rs` (the static coverage rule);
`docs/reference.md` section 11 is the normative text they formalize.

The three statements are what the design documents promise and the
article claims: the declared authority bounds every effect; a guarded
value leaves only through its sinks; a run is a function of its code,
its dependencies and the answers its primitive calls received. Each is
"by construction" in the sense that the implementation was built to make
it true; the value of writing them down is that each names exactly the
assumptions it rests on, which section 5 lists, and that the authority
gap (`authority-gap.md`) is a measurement of the first.

## 1. Capabilities, scopes, coverage

A **kind** is a path in the finite tree of reference section 11
(`filesystem.read` under `filesystem`); `k ≤ k'` says `k` is `k'` or
below it. Each kind has a set of **scopes** `S_k` with a partial order
`⊑_k` (a path prefix contains its sub-paths; a host, a variable, a
program name or a package contains only itself) and a top `⊤`
(unscoped), above every scope. A **capability** is a pair `c = (k, σ)`
with `σ ∈ S_k ∪ {⊤}`.

Coverage: `(k, σ) ⊒ (k', σ')` (the first covers the second) iff
`k' ≤ k` and `σ' ⊑ σ`. Coverage is reflexive and transitive. A set of
capabilities `C` covers `c` iff some member of `C` covers `c`; it covers
a set `C'` iff it covers every member.

The **intersection** of two sets, `C ∧ C'`, is the set of the greatest
capabilities covered by both: for every `c ∈ C` and `c' ∈ C'` on one
line of the tree, the lower kind with the lower scope when the scopes
are comparable, nothing when they are not (`grant::within`,
`grant::narrow`). The property used below: `C ∧ C'` covers `c` iff both
`C` and `C'` cover `c`.

An **effect** is a capability with an actual scope: the host of a
request, the path of a file, `(k, σ)` with `σ` never `⊤`. Every library
primitive `p` has a declared kind `k_p`, and a call of it with arguments
`a` has the effect `e(p, a) = (k_p, scope_p(a))`, where `scope_p` reads
the host or the path from the arguments (`⊤`-less; a kind without scopes
has the single scope `•`).

## 2. Programs, grants, the grant stack

A program is a set of functions `f` with declared needs `N(f)`, a set
of capabilities whose scopes may name a parameter of `f` (then the
scope is the argument's value at the call, and `N(f)` below means the
set with the parameters substituted). `main`'s needs are the **grant**
`G = N(main)`, narrowed by the host's choice `H` (the command line's
`--deny` and `--allow-*`, a sandbox's grant): the run's grant is
`G₀ = G ∧ H`.

**The static rule (coverage).** The checker accepts a program only if,
for every function `f` and every call in its body to a function `g` of
the same program, `N(f)` covers `N(g)`, and for every call to a library
primitive `p`, `N(f)` covers `(k_p, ⊤)`; a test's head covers its body
likewise. For a call into another package only the kinds are compared
(`(k, ⊤)` on both sides).

**The dynamic rule (the grant stack).** A run is a sequence of steps of
an abstract machine whose state holds a stack of frames, each a function
`f` with a grant index. The grant of a frame for `f` pushed above a frame
whose grant is `E` is `E_f = E ∧ N(f)` (`frame_grant`; a function
without needs leaves `E` as it is). The grant of the first frame is
`G₀`. A primitive call made in the innermost frame with grant `E` and
arguments `a` is **admitted** iff `E` covers `e(p, a)`; otherwise it
fails with the module's error (or crashes, for a primitive that cannot
fail) and no effect happens (`call_native` before anything else).

**Theorem 1 (authority bound).** In every run of a program that the
checker accepts, every effect that happens is covered by `G₀`, and its
kind is covered by `N(main)` with `⊤` as scope.

*Proof.* Let the call be admitted in a frame of grant `E`. By induction
on the stack, `E = G₀ ∧ N(f₁) ∧ … ∧ N(fₙ)` for the chain
`main = f₀, f₁, …, fₙ` of the frames below it (each push intersects
with the pushed function's needs and nothing else changes a grant). By
the property of `∧`, `E` covers `e` only if `G₀` covers `e`. That is
the first claim. For the second, `G₀ = G ∧ H` covers `e` only if `G =
N(main)` covers `e`, and coverage of `(k, σ)` implies coverage of
`(k, ⊤)`'s kind. ∎

The static rule is not needed for Theorem 1; it is what makes the
dynamic check succeed within one program. **Proposition 1a.** In a
program the checker accepts, a primitive call made in `fₙ` through the
chain `f₁, …, fₙ` of one program is refused only on its scope, never on
its kind: each `N(fᵢ)` covers `(k_p, ⊤)` by the static rule applied
along the chain (coverage is transitive, and a call site's `N(f)`
covers its callee's `N(g)`), so every factor of `E` covers the kind,
and only the actual scope `scope_p(a)` can fall outside a declared one.
Across a package boundary the kind is still covered (the rule compares
kinds there), so the same holds; the scope is the run's.

What the theorem does not say: nothing about effects that are not
primitive calls, because there are none (section 5, assumption A1).

## 3. Guards

A **guard** is a declared capability `g` of `G` (or of the sandbox's
grant) with a set of sinks `Σ_g`, each a capability; a run has at most
64 guards, in declaration order, and the position of `g` is its bit.
Every value `v` of the machine carries an **origin set**
`O(v) ⊆ Guards`, a word; a value from a literal has `O = ∅`.

The machine's rules for origins:

- the result `r` of an admitted primitive call `p(a)` has
  `O(r) = ⋃ O(aᵢ) ∪ {g : g covers e(p, a)}`;
- the result of every other operation (arithmetic, interpolation,
  construction of a list, a map, a record or a variant, a field read,
  `with`, the items of a loop, the clauses of a query, a derived
  ability, an unpacking, a function call's return) has the union of its
  operands' origins; a container's origin is the union of its items';
- a `match` or an `if` on a guarded value gives the branch's result the
  origins of what the branch computes, not of the value tested.

**The outgoing check.** Before an admitted primitive call `p(a)` with
effect `e` is made, the boundary requires, for every `g ∈ ⋃ O(aᵢ)`,
that `Σ_g` covers `e` (`Guard::admits`); otherwise the call fails with
`Guarded(g, e)` before anything leaves (a primitive that cannot fail
crashes; a `crash with` message and an unhandled failure's text leave
through `console` and are checked against it; a server handler's
response is checked against `network.socket`).

Define the **explicit provenance** `P(v)` of a value as the set of
guards `g` such that `v` was computed, through the operations listed
above, from the result of a call that `g` covered: the least relation
closed under the same rules with `{g : g covers e}` at the primitive
calls. By construction `O(v) = P(v)` for every value of every run
(the rules are the definition, applied at each step).

**Theorem 2 (guards admit only their sinks).** In every run, for every
primitive call that happens with effect `e` and arguments `a`, every
guard `g ∈ P(a₁) ∪ … ∪ P(aₙ)` has a sink covering `e`. Hence a value
explicitly derived from what entered through `g` leaves the program only
through an effect a sink of `g` covers.

*Proof.* The call happened, so it passed the outgoing check, which
tested exactly the condition on `O(aᵢ) = P(aᵢ)`. ∎

What the theorem does not say: nothing about implicit flow. A program
that prints "yes" when a guarded value starts with "a" leaks one bit per
run, since the tested value's origins are not added to the branch's
result (the design chose explicit flow, `06-runtime-guarantees.md`
section 3.4); a budget on the sink bounds the rate of that channel.
Nothing about timing or the length of what is sent. And a replay
derives the same `O` from the same grant header (the recording stores
plain values), so Theorem 2 holds of a replay as of the live run.

## 4. Determinism and replay

A **run** of a program `P` with arguments `x` is the machine's sequence
of states from the initial one. Partition the machine's steps into
**pure** steps and **primitive calls**. A primitive call's step is a
function of the state and of an **answer** (the value the primitive
returns, or its failure), which the world supplies during a live run
and which the recording holds for a replay.

**Lemma (pure steps are deterministic).** Every pure step is a function
of the state alone. This is where the language's choices enter: there
is no ambient input (A1), `time` and `random` are primitive calls (A2),
maps and sets iterate in insertion order (A3), decimal and float
arithmetic follow their standards (A4), tasks of `run concurrently` run
one after the other in source order (A5), a value's identity is never
observable (A6), and a failure of a budget or a guard is decided by the
state (A7).

A **recording** `R` of a run is its sequence of primitive calls
`(pᵢ, aᵢ, rᵢ)`: the primitive, its arguments and its answer, in the
order they completed. A **replay** of `P` under `R` is the run in which
the step of the `j`-th primitive call `p(a)` takes as its answer the
`rᵢ` of the first unused entry with `pᵢ = p` and `aᵢ = a` (matched by
arguments first, by sequence when several match; a redacted argument
matches any value), and fails the replay when there is none.

**Theorem 3 (replay).** Let `R` be the recording of a run of `P` on `x`
under grant `G₀`. The replay of `P` on `x` under `R` and a grant
covering `R`'s header makes the same sequence of primitive calls
`(pᵢ, aᵢ)` with the same answers, and produces the same output.

*Proof.* By induction on the number of steps, the states of the two
runs are equal. A pure step preserves equality by the lemma. At the
`j`-th primitive call, the arguments are equal by the induction
hypothesis, so the entry matched is the `j`-th of `R` (under A5 the
calls complete in the order they are made, so the first unused entry
with the same primitive and arguments is `R`'s own `j`-th entry), and
the answer is `rⱼ`, the live run's; the admission check and the guard
check decide alike on equal states (A7). The output is a function of
the console calls, which are among the primitive calls. ∎

**Corollary (reproduce).** `renyi reproduce` checks that the program's
`main` hashes as the manifest says (so `P` is the same closure), that
the dependencies are the manifest's, replays, and compares the outcome
and the output's hash; by Theorem 3 a difference is a difference in
`P`, in a dependency, in the toolchain's implementation of a pure step,
or a violation of an assumption, and the first differing call names
where.

What the theorem does not say: a replay shows that `P` still computes
the same output from the same answers; it says nothing about what the
world would answer today.

## 5. The assumptions, named

- **A1 (no ambient effect).** Every effect of a program is a call of a
  library primitive made through `call_native`; there is no reflection,
  no FFI outside the `foreign` capability and no Python outside
  `python`, and what a `foreign` or `python` function does once called
  is outside every statement here (reference section 11).
- **A2.** `time` and `random` are capabilities whose primitives are
  calls like any other.
- **A3.** Maps and sets iterate in insertion order (decision K9); the
  hasher's per-process key never reaches a program.
- **A4.** `Decimal` and `Float` arithmetic are the standards' (decision
  Q2).
- **A5.** Tasks run one after the other (decision S2); a replay of a
  future scheduler that overlaps I/O waits keeps Theorem 3 only with
  the matching by arguments, which is why the matching is by arguments
  first.
- **A6.** No operation exposes an address or an allocation; `is` and
  `is not` compare values.
- **A7.** Budgets are decided by counts the state holds (a sliding
  window reads the recorded `at_ms` of a call under replay, not the
  clock), and guards by the origins the state holds.
- **A8 (the model of section 2).** The library's `scope_p` reads the
  actual host or path from the arguments as the primitives do, and a
  path scope contains a path by its normalized text with nothing
  resolved on disk, so a symbolic link is outside the model (decision
  Y1).

Each assumption is a sentence of the reference or a decision entry; a
change to one of them is a change to these statements.

## 6. What would make this a mechanized result

The three statements are short enough to state in Lean or Coq over a
core calculus (a stack machine with a primitive-call rule carrying a
grant and an origin word, pure steps as an abstract deterministic
relation). Theorem 1 and Theorem 3 are inductions on the trace;
Theorem 2 is an invariant of the origin rules. The work is the calculus,
not the proofs; it is a stage of the research track if the owner asks
for it.
