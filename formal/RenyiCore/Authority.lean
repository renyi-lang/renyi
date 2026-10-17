import RenyiCore.Machine

/-!
# Theorem 1 (authority bound) and Proposition 1a

`docs/research/core-calculus.md` section 2.
-/

namespace RenyiCore

variable {S : Type} [ScopeOrder S] {F : Type}

/-! ## The generic invariant: every frame's grant admits only what the
initial grant admits -/

/-- Every frame's grant admits only what `E₀` admits. -/
def Bounded (M : GrantModel S) (E₀ : M.Eff) (fs : List (Frame F S M.Eff)) : Prop :=
  ∀ fr ∈ fs, ∀ c, M.admits fr.grant c → M.admits E₀ c

theorem bounded_step {P : Program F S} {M : GrantModel S} {E₀ : M.Eff}
    {s s' : List (Frame F S M.Eff)} {e : Option (Event S)}
    (hs : Bounded M E₀ s) (st : Step P M s e s') :
    Bounded M E₀ s' ∧ ∀ c, e = some (.effect c) → M.admits E₀ c := by
  cases st with
  | call =>
    refine ⟨?_, by simp⟩
    intro fr hfr c hc
    simp only [List.mem_cons] at hfr
    rcases hfr with rfl | rfl | hfr
    · exact hs _ (List.mem_cons.mpr (Or.inl rfl)) c (M.push_narrows _ _ _ hc)
    · exact hs _ (List.mem_cons.mpr (Or.inl rfl)) c hc
    · exact hs fr (by simp [hfr]) c hc
  | admit h =>
    refine ⟨?_, ?_⟩
    · intro fr hfr c hc
      simp only [List.mem_cons] at hfr
      rcases hfr with rfl | hfr
      · exact hs _ (List.mem_cons.mpr (Or.inl rfl)) c hc
      · exact hs fr (by simp [hfr]) c hc
    · intro c hc
      cases hc
      exact hs _ (List.mem_cons.mpr (Or.inl rfl)) _ h
  | refuse h =>
    refine ⟨?_, by simp⟩
    intro fr hfr c hc
    simp only [List.mem_cons] at hfr
    rcases hfr with rfl | hfr
    · exact hs _ (List.mem_cons.mpr (Or.inl rfl)) c hc
    · exact hs fr (by simp [hfr]) c hc
  | ret =>
    exact ⟨fun fr hfr => hs fr (by simp [hfr]), by simp⟩

/-- In every run from `main` under `E₀`, every effect that happened is
admitted by `E₀`. No static rule is needed. -/
theorem effects_admitted_by_initial (P : Program F S) (M : GrantModel S) (E₀ : M.Eff)
    {tr : List (Event S)} {s : List (Frame F S M.Eff)}
    (run : Run P M (initial P M E₀) tr s) :
    Bounded M E₀ s ∧ ∀ c, Event.effect c ∈ tr → M.admits E₀ c := by
  induction run with
  | refl =>
    refine ⟨?_, by simp⟩
    intro fr hfr c hc
    simp [initial] at hfr
    subst hfr
    exact hc
  | @step _ _ e _ _ st ih =>
    obtain ⟨hb, htr⟩ := ih
    obtain ⟨hb', he⟩ := bounded_step hb st
    refine ⟨hb', ?_⟩
    intro c hc
    rcases List.mem_append.mp hc with h | h
    · exact htr c h
    · cases e with
      | none => simp at h
      | some ev =>
        simp at h
        exact he c (by rw [h])

/-! ## Theorem 1 in the grant-stack model -/

/-- **Theorem 1 (authority bound).** Run from `main` with the initial grant
stack `[N(main), G₀]`, where `G₀` is the host's narrowing (any grant),
every effect that happens is covered by `G₀` and by `N(main)`. -/
theorem authority_bound (P : Program F S) (G₀ : Grant S)
    {tr : List (Event S)} {s : List (Frame F S (stackModel S).Eff)}
    (run : Run P (stackModel S) (initial P (stackModel S) [P.needs P.main, G₀]) tr s) :
    ∀ c, Event.effect c ∈ tr → grantCovers G₀ c ∧ grantCovers (P.needs P.main) c := by
  intro c hc
  have h : stackCovers [P.needs P.main, G₀] c :=
    (effects_admitted_by_initial P (stackModel S) _ run).2 c hc
  exact ⟨h G₀ (by simp), h (P.needs P.main) (by simp)⟩

/-! ## Theorem 1 in the implementation's model (`within`) -/

/-- **Theorem 1 for `grant.rs`'s `within`.** With a frame's grant formed by
any sound `within` (which passes through lines the needs do not mention, so
it is not the intersection), and `main`'s frame under `within G₀ N(main)`:
every effect is covered by `G₀`; and when `G₀` is a narrowing of `N(main)`
(as `grant::effective` makes it: every capability it returns is covered by a
declared one), every effect is covered by `N(main)`. -/
theorem authority_bound_within (P : Program F S) (G₀ : Grant S)
    (within : Grant S → Grant S → Grant S)
    (sound : ∀ E N c, grantCovers (within E N) c → grantCovers E c)
    (hG₀ : grantCoversGrant (P.needs P.main) G₀)
    {tr : List (Event S)} {s : List (Frame F S (withinModel S within sound).Eff)}
    (run : Run P (withinModel S within sound)
      (initial P (withinModel S within sound) (within G₀ (P.needs P.main))) tr s) :
    ∀ c, Event.effect c ∈ tr → grantCovers G₀ c ∧ grantCovers (P.needs P.main) c := by
  intro c hc
  have h : grantCovers (within G₀ (P.needs P.main)) c :=
    (effects_admitted_by_initial P (withinModel S within sound) _ run).2 c hc
  have h₀ : grantCovers G₀ c := sound _ _ _ h
  exact ⟨h₀, grantCovers_trans hG₀ h₀⟩

/-! ## Proposition 1a: a refusal is on the scope, never on the kind -/

/-- A frame of a well-checked run in the grant-stack model: its remaining
code is from its function's body, and its grant is a chain of needs above
`G₀`, each of which covers every kind its function's needs cover. -/
def GoodFrame (P : Program F S) (G₀ : Grant S) (fr : Frame F S (stackModel S).Eff) : Prop :=
  (∀ i ∈ fr.code, i ∈ P.body fr.fn) ∧
  ∃ chain : List (Grant S), fr.grant = chain ++ [G₀] ∧
    ∀ N ∈ chain, ∀ k, kindCovers (P.needs fr.fn) k → kindCovers N k

theorem good_tail {P : Program F S} {G₀ : Grant S} {f : F} {i : Instr F S}
    {rest : List (Instr F S)} {E : (stackModel S).Eff}
    {fs : List (Frame F S (stackModel S).Eff)}
    (h : ∀ fr ∈ (⟨f, i :: rest, E⟩ :: fs : List (Frame F S (stackModel S).Eff)),
      GoodFrame P G₀ fr) :
    ∀ fr ∈ (⟨f, rest, E⟩ :: fs : List (Frame F S (stackModel S).Eff)),
      GoodFrame P G₀ fr := by
  intro fr hfr
  simp only [List.mem_cons] at hfr
  rcases hfr with rfl | hfr
  · obtain ⟨hcode, hchain⟩ := h _ (List.mem_cons.mpr (Or.inl rfl))
    exact ⟨fun j hj => hcode j (by simp [hj]), hchain⟩
  · exact h fr (by simp [hfr])

theorem good_step {P : Program F S} {G₀ : Grant S} (hP : WellChecked P)
    {s s' : List (Frame F S (stackModel S).Eff)} {e : Option (Event S)}
    (hs : ∀ fr ∈ s, GoodFrame P G₀ fr) (st : Step P (stackModel S) s e s') :
    ∀ fr ∈ s', GoodFrame P G₀ fr := by
  cases st with
  | @call f g rest E fs =>
    intro fr hfr
    simp only [List.mem_cons] at hfr
    rcases hfr with rfl | hfr
    · obtain ⟨hcode, chain, hE, hchain⟩ := hs _ (List.mem_cons.mpr (Or.inl rfl))
      dsimp only at hcode hE hchain
      have hcall : grantCoversGrant (P.needs f) (P.needs g) :=
        hP.calls f g (hcode _ (List.mem_cons.mpr (Or.inl rfl)))
      refine ⟨fun j hj => hj, ?_⟩
      show ∃ chain', pushStack E (P.needs g) = chain' ++ [G₀] ∧
        ∀ N ∈ chain', ∀ k, kindCovers (P.needs g) k → kindCovers N k
      cases hg : P.needs g with
      | nil =>
        refine ⟨chain, hE, ?_⟩
        intro N _ k hk
        exact absurd hk (not_kindCovers_nil k)
      | cons n ns =>
        refine ⟨(n :: ns) :: chain, ?_, ?_⟩
        · simp [pushStack, hE]
        · intro N hN k hk
          simp only [List.mem_cons] at hN
          rcases hN with rfl | hN
          · exact hk
          · rw [← hg] at hk
            exact hchain N hN k (kindCovers_trans hcall hk)
    · exact good_tail hs fr (List.mem_cons.mpr hfr)
  | admit _ => exact good_tail hs
  | refuse _ => exact good_tail hs
  | ret => exact fun fr hfr => hs fr (by simp [hfr])

theorem good_run {P : Program F S} {G₀ : Grant S} (hP : WellChecked P)
    {tr : List (Event S)} {s : List (Frame F S (stackModel S).Eff)}
    (run : Run P (stackModel S) (initial P (stackModel S) [P.needs P.main, G₀]) tr s) :
    ∀ fr ∈ s, GoodFrame P G₀ fr := by
  induction run with
  | refl =>
    intro fr hfr
    simp [initial] at hfr
    subst hfr
    refine ⟨fun j hj => hj, [P.needs P.main], rfl, ?_⟩
    intro N hN k hk
    simp at hN
    subst hN
    exact hk
  | step _ st ih => exact good_step hP ih st

/-- **Proposition 1a.** In a well-checked program, when the innermost frame
is about to make a primitive call of effect `(k, σ)` and its grant stack `E`
refuses it, then either the host's `G₀` does not cover it, or some grant of
the stack covers the kind `k` but not the effect: the refusal is on the
scope. Every grant of the stack other than `G₀` covers `k` (the second
conjunct), so when `G₀` covers the kind too (no `--deny` of it), no grant
refuses on the kind alone. -/
theorem refused_only_on_scope (P : Program F S) (G₀ : Grant S) (hP : WellChecked P)
    {tr : List (Event S)} {f : F} {k : Kind} {σ : S}
    {rest : List (Instr F S)} {E : List (Grant S)} {fs : List (Frame F S (stackModel S).Eff)}
    (run : Run P (stackModel S) (initial P (stackModel S) [P.needs P.main, G₀]) tr
      (⟨f, .prim k σ :: rest, E⟩ :: fs))
    (refused : ¬ stackCovers E ⟨k, σ⟩) :
    (¬ grantCovers G₀ ⟨k, σ⟩ ∨ ∃ N ∈ E, kindCovers N k ∧ ¬ grantCovers N ⟨k, σ⟩) ∧
    (∀ N ∈ E, N = G₀ ∨ kindCovers N k) := by
  obtain ⟨hcode, chain, hE, hchain⟩ := good_run hP run _ (List.mem_cons.mpr (Or.inl rfl))
  dsimp only at hcode hE hchain
  have hkind : kindCovers (P.needs f) k := hP.prims f k σ (hcode _ (List.mem_cons.mpr (Or.inl rfl)))
  have hall : ∀ N ∈ E, N = G₀ ∨ kindCovers N k := by
    intro N hN
    rw [hE] at hN
    rcases List.mem_append.mp hN with h | h
    · exact Or.inr (hchain N h k hkind)
    · exact Or.inl (by simpa using h)
  refine ⟨?_, hall⟩
  have : ∃ N ∈ E, ¬ grantCovers N ⟨k, σ⟩ := by
    apply Classical.byContradiction
    intro hno
    exact refused (fun N hN => Classical.byContradiction fun hc => hno ⟨N, hN, hc⟩)
  obtain ⟨N, hN, hc⟩ := this
  rcases hall N hN with rfl | hk
  · exact Or.inl hc
  · exact Or.inr ⟨N, hN, hk, hc⟩

end RenyiCore
