/-!
# Determinism and replay

Section 4 of `docs/research/core-calculus.md`.

A run is a sequence of states. Each step is a pure step, a function of the
state alone (`pure`: the lemma of section 4 is embodied by making it a
function, which is where A1 to A7 enter), or a primitive call, a function
of the state and of an answer (`apply`), the state determining the
primitive and its arguments (`request`). The grant, the budgets and the
guards' origins are part of the state (A7): the admission check, the
budgets and the outgoing guard check are inside `request`, and a call
they refuse is a pure step, since nothing leaves and nothing is recorded
(`Vm::denied`, `Vm::refused`). A halted state is a fixed point of `pure`.

A live run takes its answers from the world, a function of the index of
the call; any sequence of answers is such a function, so every live run of
the machine is a live run under some world. Its recording is the list of
`(p, a, r)` entries in the order the calls completed. A replay takes each
answer from the first unused entry of the recording with the same
primitive and arguments (the matching of section 4: by arguments first,
by sequence when several match; redaction is not modelled) and stops when
there is none.
-/

namespace RenyiCore.Replay

/-- A machine partitioned into pure steps and primitive calls. -/
structure Machine (State Prim Args Answer : Type) where
  /-- the primitive call the state is about to make, if its next step is
  one; `none` when the next step is pure -/
  request : State → Option (Prim × Args)
  /-- a pure step: a function of the state alone -/
  pure : State → State
  /-- the step of a primitive call, given its answer -/
  apply : State → Answer → State

/-- An entry of a recording: a primitive call that completed, with its
answer. -/
structure Entry (Prim Args Answer : Type) where
  prim : Prim
  args : Args
  answer : Answer

variable {State Prim Args Answer : Type}

/-! ## The live run and its recording -/

/-- The configuration of a live run: the state and the number of calls
completed, which is the index of the next answer in the world. -/
structure LiveCfg (State : Type) where
  state : State
  calls : Nat

/-- One step of a live run under the world `w`: a pure step, or the call
the state requests answered by the world and recorded. -/
def liveStep (M : Machine State Prim Args Answer) (w : Nat → Answer) (c : LiveCfg State) :
    LiveCfg State × Option (Entry Prim Args Answer) :=
  match M.request c.state with
  | none => (⟨M.pure c.state, c.calls⟩, none)
  | some (p, a) => (⟨M.apply c.state (w c.calls), c.calls + 1⟩, some ⟨p, a, w c.calls⟩)

/-- The live run of `n` steps from `s` under the world `w`: its
configuration and the recording so far. -/
def live (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) :
    Nat → LiveCfg State × List (Entry Prim Args Answer)
  | 0 => (⟨s, 0⟩, [])
  | n + 1 =>
    ((liveStep M w (live M w s n).1).1,
      (live M w s n).2 ++ ((liveStep M w (live M w s n).1).2).toList)

/-- The state of the live run after `n` steps. -/
def liveState (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (n : Nat) :
    State :=
  (live M w s n).1.state

/-- The recording of the live run of `n` steps. -/
def recording (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (n : Nat) :
    List (Entry Prim Args Answer) :=
  (live M w s n).2

theorem liveStep_none {M : Machine State Prim Args Answer} {w : Nat → Answer} {c : LiveCfg State}
    (h : M.request c.state = none) : liveStep M w c = (⟨M.pure c.state, c.calls⟩, none) := by
  simp [liveStep, h]

theorem liveStep_some {M : Machine State Prim Args Answer} {w : Nat → Answer} {c : LiveCfg State}
    {p : Prim} {a : Args} (h : M.request c.state = some (p, a)) :
    liveStep M w c = (⟨M.apply c.state (w c.calls), c.calls + 1⟩, some ⟨p, a, w c.calls⟩) := by
  simp [liveStep, h]

/-- The recording has one entry per call completed. -/
theorem live_length (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) :
    ∀ n, (live M w s n).2.length = (live M w s n).1.calls
  | 0 => rfl
  | n + 1 => by
    have ih := live_length M w s n
    cases h : M.request (live M w s n).1.state with
    | none => simp [live, liveStep_none (c := (live M w s n).1) h, ih]
    | some pa =>
      obtain ⟨p, a⟩ := pa
      simp [live, liveStep_some (c := (live M w s n).1) h, ih]

/-- The recording of a longer run extends the recording of a shorter one. -/
theorem live_prefix (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (k : Nat) :
    ∀ d, ∃ t, (live M w s (k + d)).2 = (live M w s k).2 ++ t
  | 0 => ⟨[], by simp⟩
  | d + 1 => by
    obtain ⟨t, ht⟩ := live_prefix M w s k d
    refine ⟨t ++ ((liveStep M w (live M w s (k + d)).1).2).toList, ?_⟩
    rw [← Nat.add_assoc]
    simp [live, ht]

/-- When the `k`-th step of a run of at least `k + 1` steps is a call, the
recording of the whole run holds its entry right after the recording of
the first `k` steps. -/
theorem live_split (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State)
    {k n : Nat} (hk : k + 1 ≤ n) {p : Prim} {a : Args}
    (h : M.request (live M w s k).1.state = some (p, a)) :
    ∃ t, (live M w s n).2 = (live M w s k).2 ++ ⟨p, a, w (live M w s k).1.calls⟩ :: t := by
  obtain ⟨t, ht⟩ := live_prefix M w s (k + 1) (n - (k + 1))
  rw [Nat.add_sub_of_le hk] at ht
  refine ⟨t, ?_⟩
  rw [ht]
  simp [live, liveStep_some (c := (live M w s k).1) h]

/-! ## The replay -/

/-- The replay's book: the recording with its used entries crossed out. -/
abbrev Book (Prim Args Answer : Type) := List (Option (Entry Prim Args Answer))

/-- The first unused entry with the given primitive and arguments, and the
book with it crossed out; `none` when there is none. -/
def takeFirst [DecidableEq Prim] [DecidableEq Args] (p : Prim) (a : Args) :
    Book Prim Args Answer → Option (Answer × Book Prim Args Answer)
  | [] => none
  | none :: rest => (takeFirst p a rest).map fun x => (x.1, none :: x.2)
  | some en :: rest =>
    if en.prim = p ∧ en.args = a then some (en.answer, none :: rest)
    else (takeFirst p a rest).map fun x => (x.1, some en :: x.2)

/-- The configuration of a replay: the state and the book. -/
structure ReplayCfg (State Prim Args Answer : Type) where
  state : State
  book : Book Prim Args Answer

/-- One step of a replay: a pure step, or the call the state requests
answered from the book; `none` when the book has no entry for it (the
replay fails). -/
def replayStep [DecidableEq Prim] [DecidableEq Args] (M : Machine State Prim Args Answer)
    (c : ReplayCfg State Prim Args Answer) :
    Option (ReplayCfg State Prim Args Answer × Option (Entry Prim Args Answer)) :=
  match M.request c.state with
  | none => some (⟨M.pure c.state, c.book⟩, none)
  | some (p, a) =>
    (takeFirst p a c.book).map fun x => (⟨M.apply c.state x.1, x.2⟩, some ⟨p, a, x.1⟩)

/-- The replay of `n` steps from `s` under the recording `R`: its
configuration and the calls it made, or `none` when it failed. -/
def replay [DecidableEq Prim] [DecidableEq Args] (M : Machine State Prim Args Answer)
    (s : State) (R : List (Entry Prim Args Answer)) :
    Nat → Option (ReplayCfg State Prim Args Answer × List (Entry Prim Args Answer))
  | 0 => some (⟨s, R.map some⟩, [])
  | n + 1 =>
    (replay M s R n).bind fun r => (replayStep M r.1).map fun x => (x.1, r.2 ++ x.2.toList)

theorem replayStep_none [DecidableEq Prim] [DecidableEq Args] {M : Machine State Prim Args Answer}
    {st : State} {book : Book Prim Args Answer} (h : M.request st = none) :
    replayStep M ⟨st, book⟩ = some (⟨M.pure st, book⟩, none) := by
  simp [replayStep, h]

theorem replayStep_some [DecidableEq Prim] [DecidableEq Args] {M : Machine State Prim Args Answer}
    {st : State} {book : Book Prim Args Answer} {p : Prim} {a : Args}
    (h : M.request st = some (p, a)) :
    replayStep M ⟨st, book⟩ =
      (takeFirst p a book).map fun x => (⟨M.apply st x.1, x.2⟩, some ⟨p, a, x.1⟩) := by
  simp [replayStep, h]

/-- Past `j` used entries, the first unused entry with the primitive and
the arguments is matched and crossed out. -/
theorem takeFirst_at [DecidableEq Prim] [DecidableEq Args] (p : Prim) (a : Args) (r : Answer) :
    ∀ (j : Nat) (t : Book Prim Args Answer),
      takeFirst p a (List.replicate j none ++ some ⟨p, a, r⟩ :: t) =
        some (r, List.replicate (j + 1) none ++ t)
  | 0, t => by simp [takeFirst]
  | j + 1, t => by simp [takeFirst, List.replicate_succ, takeFirst_at p a r j t]

theorem drop_length_succ {α : Type} (x : α) (t : List α) :
    ∀ (l : List α) {j : Nat}, l.length = j → (l ++ x :: t).drop (j + 1) = t
  | [], _, h => by subst h; rfl
  | _ :: l, _, h => by subst h; simp [drop_length_succ x t l rfl]

/-! ## Theorem 3 -/

/-- The induction of section 4: after `k ≤ n` steps, the replay under the
recording of the live run of `n` steps is in the live run's state, has
made the live run's calls with their answers, and has used exactly the
first entries of the recording, as many as the calls completed. -/
theorem replay_tracks_live [DecidableEq Prim] [DecidableEq Args]
    (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (n : Nat) :
    ∀ k, k ≤ n →
      replay M s (live M w s n).2 k =
        some (⟨(live M w s k).1.state,
          List.replicate (live M w s k).1.calls none ++
            ((live M w s n).2.drop (live M w s k).1.calls).map some⟩,
          (live M w s k).2)
  | 0, _ => by simp [replay, live]
  | k + 1, hk => by
    have ih := replay_tracks_live M w s n k (Nat.le_of_succ_le hk)
    rw [replay, ih]
    cases h : M.request (live M w s k).1.state with
    | none =>
      simp [replayStep_none (book := _) h, live, liveStep_none (c := (live M w s k).1) h]
    | some pa =>
      obtain ⟨p, a⟩ := pa
      obtain ⟨t, ht⟩ := live_split M w s hk h
      have hdrop : (live M w s n).2.drop (live M w s k).1.calls =
          ⟨p, a, w (live M w s k).1.calls⟩ :: t := by
        rw [ht]; exact List.drop_left' (live_length M w s k)
      have hdrop' : (live M w s n).2.drop ((live M w s k).1.calls + 1) = t := by
        rw [ht]; exact drop_length_succ _ _ _ (live_length M w s k)
      simp [replayStep_some (book := _) h, hdrop, takeFirst_at, live,
        liveStep_some (c := (live M w s k).1) h, hdrop']

/-- **Theorem 3 (replay).** The replay of `n` steps from `s` under the
recording of the live run of `n` steps from `s` succeeds, reaches the live
run's state, makes the same sequence of primitive calls with the same
answers (its calls are the recording), and uses every entry of the
recording. -/
theorem replay_run [DecidableEq Prim] [DecidableEq Args]
    (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (n : Nat) :
    replay M s (recording M w s n) n =
      some (⟨liveState M w s n, List.replicate (recording M w s n).length none⟩,
        recording M w s n) := by
  unfold recording liveState
  rw [replay_tracks_live M w s n n (Nat.le_refl n), ← live_length]
  simp

/-- **Corollary.** The output of a run is a function of its primitive
calls, the console's among them: the replay produces the live run's
output. -/
theorem replay_output [DecidableEq Prim] [DecidableEq Args] {Output : Type}
    (output : List (Entry Prim Args Answer) → Output)
    (M : Machine State Prim Args Answer) (w : Nat → Answer) (s : State) (n : Nat) :
    (replay M s (recording M w s n) n).map (fun r => output r.2) =
      some (output (recording M w s n)) := by
  rw [replay_run]
  rfl

end RenyiCore.Replay
