import RenyiCore.Coverage

/-!
# Programs, the static rule and the abstract machine

Section 2 of `docs/research/core-calculus.md`.

A program is a set of functions (an arbitrary type `F`), each with its
declared needs and a body. A body is a list of instructions: a call of a
function of the program, or a call of a library primitive whose effect is
the capability `(k, σ)`; `σ` stands for `scope_p(a)`, the actual scope the
primitive reads from its arguments at run time (A8).

The machine is parameterised by a `GrantModel`: how a frame's grant is
formed from the enclosing frame's grant and the pushed function's needs,
and when a grant admits an effect. Each frame carries its own grant, as each
frame of the VM carries a grant index (`crates/renyi_vm/src/vm.rs`,
`frame_grant`).
-/

namespace RenyiCore

variable {S : Type} [ScopeOrder S]

/-- An instruction of a body. -/
inductive Instr (F S : Type) where
  /-- a call of a function of the program -/
  | call (g : F)
  /-- a primitive call whose effect is `(k, σ)` -/
  | prim (k : Kind) (σ : S)

/-- A program: the functions' needs and bodies, and `main`. -/
structure Program (F S : Type) where
  needs : F → Grant S
  body : F → List (Instr F S)
  main : F

/-- The checker's static rule (coverage): a call of `g` in `f` needs
`N(f)` to cover `N(g)`; a primitive call of kind `k` needs `N(f)` to cover
`k` (kinds only: the actual scope is checked at run time). -/
structure WellChecked {F : Type} (P : Program F S) : Prop where
  calls : ∀ f g, Instr.call g ∈ P.body f → grantCoversGrant (P.needs f) (P.needs g)
  prims : ∀ f k σ, Instr.prim k σ ∈ P.body f → kindCovers (P.needs f) k

/-- How grants are formed and checked. The one law: pushing a function's
needs can only narrow what is admitted. -/
structure GrantModel (S : Type) [ScopeOrder S] where
  Eff : Type
  push : Eff → Grant S → Eff
  admits : Eff → Cap S → Prop
  push_narrows : ∀ E N c, admits (push E N) c → admits E c

/-- A frame: the running function, the rest of its body, its grant. -/
structure Frame (F S Eff : Type) where
  fn : F
  code : List (Instr F S)
  grant : Eff

/-- What a step shows the world: an effect that happened, or a primitive
call refused before anything happened. -/
inductive Event (S : Type) where
  | effect (c : Cap S)
  | refused (c : Cap S)

/-- One step of the machine; the head of the list is the innermost frame. -/
inductive Step {F : Type} (P : Program F S) (M : GrantModel S) :
    List (Frame F S M.Eff) → Option (Event S) → List (Frame F S M.Eff) → Prop where
  /-- a call pushes the callee's frame with the grant narrowed by its needs -/
  | call {f g rest E fs} :
      Step P M (⟨f, .call g :: rest, E⟩ :: fs) none
        (⟨g, P.body g, M.push E (P.needs g)⟩ :: ⟨f, rest, E⟩ :: fs)
  /-- a primitive call the frame's grant admits: the effect happens -/
  | admit {f k σ rest E fs} (h : M.admits E ⟨k, σ⟩) :
      Step P M (⟨f, .prim k σ :: rest, E⟩ :: fs) (some (.effect ⟨k, σ⟩))
        (⟨f, rest, E⟩ :: fs)
  /-- a primitive call the frame's grant refuses: no effect; the program
  continues (a failure it handles; a crash is a run that stops here) -/
  | refuse {f k σ rest E fs} (h : ¬ M.admits E ⟨k, σ⟩) :
      Step P M (⟨f, .prim k σ :: rest, E⟩ :: fs) (some (.refused ⟨k, σ⟩))
        (⟨f, rest, E⟩ :: fs)
  /-- a body that is done returns -/
  | ret {f E fs} : Step P M (⟨f, [], E⟩ :: fs) none fs

/-- A run: finitely many steps, with the trace of events they showed. Every
finite prefix of an infinite run is a `Run`, so statements about every
event of every `Run` cover infinite runs too. -/
inductive Run {F : Type} (P : Program F S) (M : GrantModel S) :
    List (Frame F S M.Eff) → List (Event S) → List (Frame F S M.Eff) → Prop where
  | refl {s} : Run P M s [] s
  | step {s tr s' e s''} : Run P M s tr s' → Step P M s' e s'' →
      Run P M s (tr ++ e.toList) s''

/-- The initial state: `main`'s frame under the initial grant. -/
def initial {F : Type} (P : Program F S) (M : GrantModel S) (E₀ : M.Eff) :
    List (Frame F S M.Eff) :=
  [⟨P.main, P.body P.main, E₀⟩]

/-! ## Two grant models -/

/-- The grant stack of decision Q1 and section 2: a frame's grant is the
list of grants it is the intersection of; a function with needs pushes
them, a function without needs leaves the grant as it is (`vm.rs`:
`narrows` is false for empty needs, and `frame_grant` returns the
enclosing grant). -/
def pushStack (E : List (Grant S)) : Grant S → List (Grant S)
  | [] => E
  | N@(_ :: _) => N :: E

def stackModel (S : Type) [ScopeOrder S] : GrantModel S where
  Eff := List (Grant S)
  push := pushStack
  admits := stackCovers
  push_narrows := by
    intro E N c h
    cases N with
    | nil => exact h
    | cons n ns => exact (stackCovers_cons.mp h).2

/-- The implementation's model: a frame's grant is one flat grant,
`within E N` (`grant.rs`, `within`), which is not the intersection: a
granted capability on a line the needs do not mention passes through. All
that is assumed of it is soundness: what `within E N` covers, `E` covers
(each capability `within` returns is the enclosing one, a child of it with
its scope, or it with a contained scope). This property of `grant.rs` is
an assumption here, not proved. -/
def withinModel (S : Type) [ScopeOrder S] (within : Grant S → Grant S → Grant S)
    (sound : ∀ E N c, grantCovers (within E N) c → grantCovers E c) : GrantModel S where
  Eff := Grant S
  push := within
  admits := grantCovers
  push_narrows := sound

end RenyiCore
