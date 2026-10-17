import RenyiCore.Coverage

/-!
# Guards, origins and the explicit provenance

Section 3 of `docs/research/core-calculus.md`.

A *guard* is a declared capability with a set of sinks, each a capability
(`crates/renyi_vm/src/grant.rs`, `Guard`). The guards of a run are an
abstract type `Γ`; the implementation has at most 64 of them and gives each
a bit. Every value carries an *origin set* `O(v) ⊆ Γ`, here a predicate on
`Γ`, the `u64` of `value.rs` (`Value::origins`); a literal has none.

The values of this machine are the record of how they were computed: a
literal; an operation on two operands (every operation of section 3 other
than a primitive call gives its result the union of its operands' origins,
and an operation of more operands is a tree of these); or the result of an
admitted primitive call of effect `e` on its argument. A call's arguments
are one value, the tuple of them built by `op`, whose origins are the
union of theirs, so a statement over `⋃ O(aᵢ)` is a statement over
`O(a)`. The origin set is computed from the record by the rules of
section 3 (`Val.origins`), and the explicit provenance `P(v)` is the least
relation closed under the same rules (`Prov`); `origins_iff_prov` is the
section's "by construction `O(v) = P(v)`".

The machine is a stack machine over such values: a configuration is the
code left and the stack. A primitive call is made only when every guard in
its argument's origins has a sink covering the effect (`Guard::admits`,
`Vm::blocking_guard`), and its result gains the guards whose capability
covers the effect (`Vm::origins_through`); refused, it leaves a plain
failure value (`Vm::refused` builds it from texts, with no origin bits)
and the program continues, as in `Machine.lean`. A branch (`test`) drops
the value it tests: the tested value's origins reach neither branch, as in
the implementation (`Value::with_origins`: a Boolean carries no origins).
The model reproduces that implicit flow is untracked; it does not
strengthen it.
-/

namespace RenyiCore.Guarded

variable {S : Type} [ScopeOrder S] {Γ : Type}

/-! ## Origin sets and guards -/

/-- An origin set: the guards a value came through (`Value::origins`, one
bit per guard). -/
abbrev Origins (Γ : Type) := Γ → Prop

/-- The empty origin set, a literal's. -/
def Origins.none : Origins Γ := fun _ => False

/-- The union of two origin sets. -/
def Origins.union (a b : Origins Γ) : Origins Γ := fun g => a g ∨ b g

/-- The guards of a run: each guard's capability and its sinks
(`grant.rs`, `Guard`: `capability` and `sinks`). -/
structure Guards (Γ S : Type) where
  cap : Γ → Cap S
  sinks : Γ → Grant S

/-- The guards whose capability covers the effect `e`: the origins the
result of a call of effect `e` gains (`Vm::origins_through`). -/
def Guards.through (gs : Guards Γ S) (e : Cap S) : Origins Γ :=
  fun g => covers (gs.cap g) e

/-- `Guard::admits`: a sink of the guard covers the effect. -/
def Guards.admits (gs : Guards Γ S) (g : Γ) (e : Cap S) : Prop :=
  grantCovers (gs.sinks g) e

/-- The outgoing check (`Vm::blocking_guard` finds no guard): every guard
in the origin set admits the effect. -/
def Guards.admitsAll (gs : Guards Γ S) (O : Origins Γ) (e : Cap S) : Prop :=
  ∀ g, O g → gs.admits g e

/-! ## Values, their origins and their provenance -/

/-- A value as the record of its computation. -/
inductive Val (S : Type) where
  /-- a literal -/
  | lit
  /-- any operation other than a primitive call, on two operands -/
  | op (a b : Val S)
  /-- the result of an admitted primitive call of effect `e` on the
  argument `a` -/
  | prim (e : Cap S) (a : Val S)

/-- The origin set the machine computes for a value, by the rules of
section 3: a literal has none; an operation's result has the union of its
operands'; a primitive call's result has the union of its argument's and
the guards that cover its effect. -/
def Val.origins (gs : Guards Γ S) : Val S → Origins Γ
  | .lit => Origins.none
  | .op a b => (a.origins gs).union (b.origins gs)
  | .prim e a => (a.origins gs).union (gs.through e)

/-- The explicit provenance `P(v)`: the least relation closed under the
rules of section 3, with `{g : g covers e}` at the primitive calls. -/
inductive Prov (gs : Guards Γ S) : Val S → Γ → Prop where
  | opLeft {a b : Val S} {g : Γ} : Prov gs a g → Prov gs (.op a b) g
  | opRight {a b : Val S} {g : Γ} : Prov gs b g → Prov gs (.op a b) g
  | primArg {e : Cap S} {a : Val S} {g : Γ} : Prov gs a g → Prov gs (.prim e a) g
  | primThrough {e : Cap S} {a : Val S} {g : Γ} :
      covers (gs.cap g) e → Prov gs (.prim e a) g

/-- The machine's origin set is the explicit provenance: `O(v) = P(v)` for
every value, by induction on the value's record. -/
theorem origins_iff_prov (gs : Guards Γ S) (v : Val S) (g : Γ) :
    v.origins gs g ↔ Prov gs v g := by
  induction v with
  | lit =>
    constructor
    · intro h; exact False.elim h
    · intro h; cases h
  | op a b iha ihb =>
    simp only [Val.origins, Origins.union]
    constructor
    · rintro (h | h)
      · exact .opLeft (iha.mp h)
      · exact .opRight (ihb.mp h)
    · intro h
      cases h with
      | opLeft h => exact Or.inl (iha.mpr h)
      | opRight h => exact Or.inr (ihb.mpr h)
  | prim e a ih =>
    simp only [Val.origins, Origins.union, Guards.through]
    constructor
    · rintro (h | h)
      · exact .primArg (ih.mp h)
      · exact .primThrough h
    · intro h
      cases h with
      | primArg h => exact Or.inl (ih.mpr h)
      | primThrough h => exact Or.inr h

/-! ## The machine -/

/-- An instruction: push a literal, use the top value twice, combine the
two top values by an operation, call a primitive of effect `e` on the top
value, or branch on the top value. -/
inductive Instr (S : Type) where
  | lit
  | dup
  | op
  | prim (e : Cap S)
  | test (t f : List (Instr S))

/-- A configuration: the code left and the stack of values. -/
structure Config (S : Type) where
  code : List (Instr S)
  stack : List (Val S)

/-- What a step shows the world: a primitive call of effect `e` made on
the argument `a`, or one refused by a guard before anything left. -/
inductive Event (S : Type) where
  | effect (e : Cap S) (a : Val S)
  | refused (e : Cap S) (a : Val S)

/-- One step of the machine. -/
inductive Step (gs : Guards Γ S) : Config S → Option (Event S) → Config S → Prop where
  /-- a literal: no origins -/
  | lit {rest st} : Step gs ⟨.lit :: rest, st⟩ none ⟨rest, .lit :: st⟩
  /-- a value used twice -/
  | dup {rest v st} : Step gs ⟨.dup :: rest, v :: st⟩ none ⟨rest, v :: v :: st⟩
  /-- an operation: the union of its operands' origins -/
  | op {rest a b st} : Step gs ⟨.op :: rest, a :: b :: st⟩ none ⟨rest, .op a b :: st⟩
  /-- a primitive call that passes the outgoing check: the effect happens
  and the result carries the argument's origins and the guards covering
  the effect -/
  | admit {e rest a st} (h : gs.admitsAll (a.origins gs) e) :
      Step gs ⟨.prim e :: rest, a :: st⟩ (some (.effect e a)) ⟨rest, .prim e a :: st⟩
  /-- a primitive call a guard stops: nothing leaves; a plain failure value
  takes the result's place and the program continues (a crash is a run that
  stops here) -/
  | refuse {e rest a st} (h : ¬ gs.admitsAll (a.origins gs) e) :
      Step gs ⟨.prim e :: rest, a :: st⟩ (some (.refused e a)) ⟨rest, .lit :: st⟩
  /-- a branch on a value: the value is dropped, its origins reach neither
  branch (implicit flow, untracked by design) -/
  | testLeft {t f rest v st} : Step gs ⟨.test t f :: rest, v :: st⟩ none ⟨t ++ rest, st⟩
  | testRight {t f rest v st} : Step gs ⟨.test t f :: rest, v :: st⟩ none ⟨f ++ rest, st⟩

/-- A run: finitely many steps, with the trace of events they showed; every
finite prefix of an infinite run is one, as in `Machine.lean`. -/
inductive Run (gs : Guards Γ S) : Config S → List (Event S) → Config S → Prop where
  | refl {c} : Run gs c [] c
  | step {c tr c' e c''} : Run gs c tr c' → Step gs c' e c'' →
      Run gs c (tr ++ e.toList) c''

/-! ## Theorem 2 -/

/-- The only step that shows an effect is an admitted call, and it passed
the outgoing check on its argument's origins. -/
theorem step_effect {gs : Guards Γ S} {c c' : Config S} {ev : Option (Event S)}
    {e : Cap S} {a : Val S} (st : Step gs c ev c') (h : Event.effect e a ∈ ev.toList) :
    gs.admitsAll (a.origins gs) e := by
  cases ev with
  | none => simp at h
  | some ev =>
    simp at h
    subst h
    cases st with
    | admit h => exact h

/-- **Theorem 2 (guards admit only their sinks).** In every run, for every
primitive call that happens with effect `e` on the argument `a`, every
guard in the explicit provenance of `a` has a sink covering `e`. No
hypothesis on the starting configuration is needed: the check is made at
every call. -/
theorem guards_admit_only_sinks (gs : Guards Γ S) {c c' : Config S} {tr : List (Event S)}
    (run : Run gs c tr c') :
    ∀ e a, Event.effect e a ∈ tr → ∀ g, Prov gs a g → gs.admits g e := by
  induction run with
  | refl => intro e a ha; simp at ha
  | step _ st ih =>
    intro e a ha g hg
    rcases List.mem_append.mp ha with h | h
    · exact ih e a h g hg
    · exact step_effect st h g ((origins_iff_prov gs a g).mpr hg)

end RenyiCore.Guarded
