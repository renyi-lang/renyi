/-!
# Capabilities, scopes and coverage

Section 1 of `docs/research/core-calculus.md`.

* A *kind* is a path in the capability tree, a list of names
  (`["filesystem", "read"]`). `Kind.Below k k'` says `k` is `k'` or below
  it: `k'` is a prefix of `k`.
* *Scopes* are an abstract type with a preorder `ScopeOrder.le`
  (`le s t`: the scope `t` contains the scope `s`) and a top `⊤` (unscoped).
  Antisymmetry is never used, so a preorder suffices. A concrete instance,
  path scopes, is given at the end of this file.
* A capability is a kind with a scope; `covers g c` iff `c`'s kind is at or
  below `g`'s and `c`'s scope is contained in `g`'s
  (`crates/renyi_check/src/effects.rs`, `Capability::covers`, with
  `runtime_scoped = false`).
* A grant is a finite list of capabilities; it covers `c` iff a member does.
-/

namespace RenyiCore

/-- A capability kind: a path in the capability tree. -/
abbrev Kind := List String

/-- `Kind.Below k k'`: the kind `k` is `k'` or below it in the tree. -/
def Kind.Below (k k' : Kind) : Prop := k' <+: k

theorem Kind.Below.refl (k : Kind) : Kind.Below k k := List.prefix_refl k

theorem Kind.Below.trans {a b c : Kind} (h₁ : Kind.Below a b) (h₂ : Kind.Below b c) :
    Kind.Below a c :=
  List.IsPrefix.trans h₂ h₁

instance (k k' : Kind) : Decidable (Kind.Below k k') :=
  inferInstanceAs (Decidable (k' <+: k))

/-- Scopes: a decidable preorder with a top element (the unscoped `⊤`).
`le s t` reads "`t` contains `s`". -/
class ScopeOrder (S : Type) where
  le : S → S → Prop
  le_refl : ∀ s, le s s
  le_trans : ∀ {a b c : S}, le a b → le b c → le a c
  top : S
  le_top : ∀ s, le s top
  decLe : ∀ s t, Decidable (le s t)

variable {S : Type} [ScopeOrder S]

/-- A capability: a kind with a scope. -/
structure Cap (S : Type) where
  kind : Kind
  scope : S

/-- `covers g c`: the capability `g` covers the capability `c`. -/
def covers (g c : Cap S) : Prop :=
  Kind.Below c.kind g.kind ∧ ScopeOrder.le c.scope g.scope

instance (g c : Cap S) : Decidable (covers g c) :=
  have := ScopeOrder.decLe c.scope g.scope
  inferInstanceAs (Decidable (_ ∧ _))

theorem covers_refl (c : Cap S) : covers c c :=
  ⟨Kind.Below.refl _, ScopeOrder.le_refl _⟩

theorem covers_trans {a b c : Cap S} (h₁ : covers a b) (h₂ : covers b c) : covers a c :=
  ⟨Kind.Below.trans h₂.1 h₁.1, ScopeOrder.le_trans h₂.2 h₁.2⟩

/-- A grant: a finite list of capabilities. -/
abbrev Grant (S : Type) := List (Cap S)

/-- A grant covers `c` iff one of its members does. -/
def grantCovers (G : Grant S) (c : Cap S) : Prop := ∃ g ∈ G, covers g c

/-- A grant covers a grant iff it covers each of its members. -/
def grantCoversGrant (G G' : Grant S) : Prop := ∀ c ∈ G', grantCovers G c

/-- The kind-only coverage the checker uses for a library primitive: a
scoped grant covers a primitive's unscoped kind because the primitive checks
its actual scope at run time (`effects.rs`, `covers` with
`runtime_scoped = true`). -/
def kindCovers (G : Grant S) (k : Kind) : Prop := ∃ g ∈ G, Kind.Below k g.kind

theorem grantCovers_trans {G G' : Grant S} {c : Cap S}
    (h : grantCoversGrant G G') (hc : grantCovers G' c) : grantCovers G c := by
  obtain ⟨g', hg', hcov'⟩ := hc
  obtain ⟨g, hg, hcov⟩ := h g' hg'
  exact ⟨g, hg, covers_trans hcov hcov'⟩

theorem kindCovers_trans {G G' : Grant S} {k : Kind}
    (h : grantCoversGrant G G') (hk : kindCovers G' k) : kindCovers G k := by
  obtain ⟨g', hg', hb'⟩ := hk
  obtain ⟨g, hg, hcov⟩ := h g' hg'
  exact ⟨g, hg, Kind.Below.trans hb' hcov.1⟩

theorem kindCovers_of_grantCovers {G : Grant S} {c : Cap S}
    (h : grantCovers G c) : kindCovers G c.kind := by
  obtain ⟨g, hg, hcov⟩ := h
  exact ⟨g, hg, hcov.1⟩

omit [ScopeOrder S] in
theorem not_kindCovers_nil (k : Kind) : ¬ kindCovers ([] : Grant S) k := by
  intro ⟨_, h, _⟩; cases h

/-- A stack of grants (the grant stack of decision Q1) covers `c` iff every
grant in it does. This is coverage by the intersection `C ∧ C'` of
section 1, by the property stated there: `C ∧ C'` covers `c` iff both do. -/
def stackCovers (st : List (Grant S)) (c : Cap S) : Prop := ∀ G ∈ st, grantCovers G c

theorem stackCovers_cons {N : Grant S} {st : List (Grant S)} {c : Cap S} :
    stackCovers (N :: st) c ↔ grantCovers N c ∧ stackCovers st c := by
  simp [stackCovers]

/-! ## A concrete scope model: path scopes

`none` is `⊤`; `some p` contains `some q` iff `p` is a prefix of `q`
("a path prefix contains its sub-paths", A8: by normalized text, nothing
resolved on disk). It only shows the class is inhabited by the intended
model; nothing below depends on it. -/

/-- Path scopes: `none` is the top. -/
def PathScope.le : Option (List String) → Option (List String) → Prop
  | _, none => True
  | none, some _ => False
  | some q, some p => p <+: q

instance : ScopeOrder (Option (List String)) where
  le := PathScope.le
  le_refl := by
    intro s; cases s <;> simp [PathScope.le]
  le_trans := by
    intro a b c h₁ h₂
    cases a <;> cases b <;> cases c <;> simp_all [PathScope.le]
    exact List.IsPrefix.trans h₂ h₁
  top := none
  le_top := by intro s; cases s <;> simp [PathScope.le]
  decLe := by
    intro s t
    cases s <;> cases t <;> simp only [PathScope.le] <;> infer_instance

end RenyiCore
