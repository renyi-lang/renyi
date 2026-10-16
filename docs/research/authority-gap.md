# The authority gap: what a declared grant stops before the run

Status: measured 2026-10-16 (decision AX2). The first result of the
research track (decision AX1). The scripts are `tools/authority_gap/`,
the data and the pinned output `tests/authority/`, which CI holds.

## 1. The question

A prompt injection reaches an agent's program through the data a tool
returns and asks it to do something the user did not. In a language
where the program's authority is declared before the run and enforced
at every effect (`docs/reference.md` section 11), an injected goal can
only be carried out if it fits inside that authority. The question is
how often it does not: for a benchmark of real injection attacks, what
fraction of the attacks need a capability, a scope or a count that the
benign task's own program would never declare?

That fraction is the **authority gap**. It is a property of the task
and the attack, not of any model: for every pair in the gap, no program
that passed the review for the user's task can carry out the attack,
whatever the model does once the program runs. It is the ceiling of
every defense that fixes authority before the run, and the number that
says what is left for the mechanisms that act during it (guards,
budgets, the recording) and after it (the reviewer).

## 2. The benchmark

AgentDojo (Debenedetti et al., 2024; arXiv:2406.13352; MIT license),
its suites `v1` at commit `089ed468cf3ed0322acc66b0211f26d9d90dbf60`:
four suites (banking, slack, travel, workspace), 86 user tasks and 27
injection tasks, every user task of a suite paired with every injection
task of the suite, 567 pairs. Each task declares a `ground_truth`: the
list of tool calls, with their arguments, that carry it out. Those
lists are the data: the user task's is the smallest set of calls a
correct program makes, the injection task's the calls the attack must
make. The benchmark's own paper counts 97 tasks and 629 cases over a
later version; the measure is of `v1` and names it.

`tools/authority_gap/extract_tasks.py` reads the task files as syntax
trees and never runs them (an argument that resolves through class and
module constants, f-strings included, is `literal`; one computed from
the environment is `derived`). `tests/authority/tasks_v1.json` is its
output.

## 3. The model

A tool is a capability kind. The argument that names a recipient, a
URL, a channel, a hotel, a file or a participant list is the kind's
scope (`SCOPE` in `tools/authority_gap/authority_gap.py`; one tool,
`update_scheduled_transaction`, is two kinds, since changing a
recipient is another authority than changing an amount). A user task's
**grant** is what its program can declare in the `needs` of `main`:

- *kind*: every kind the ground truth calls, without scopes;
- *prompt*: the same, with a scope declared for a kind when every call
  of it has a literal scope that the user's prompt spells out, since
  that is what the program's author can write as a text literal (a
  recipient read from a file is not declarable, and the kind is granted
  without a scope);
- *prompt+budget*: the same, with `at most n per run` on every kind, `n`
  the number of calls of the kind in the ground truth;
- *literal*: a scope declared for every literal argument, whether or
  not the prompt spells it out: an upper bound, not a grant anyone can
  write.

A pair is **blocked** when the injection's calls need a kind the grant
lacks, a scope outside a declared one, or more calls of a kind than
the budget; it **fits** otherwise. An injection with no tool call at
all (an attack that only wants the agent to say something) is
`no-tool`: no grant can touch it. An injection whose scope is derived
against a declared scope would be `undetermined`; none is.

## 4. The result

```
all: 567 pairs
  kind          blocked  475 ( 83.8%): kind 475, scope   0, budget   0; undetermined   0, fits  72, no-tool  20
  prompt        blocked  513 ( 90.5%): kind 475, scope  38, budget   0; undetermined   0, fits  34, no-tool  20
  prompt+budget blocked  515 ( 90.8%): kind 475, scope  38, budget   2; undetermined   0, fits  32, no-tool  20
  literal       blocked  539 ( 95.1%): kind 475, scope  64, budget   0; undetermined   0, fits   8, no-tool  20
```

Per suite (the *prompt+budget* grant): banking 131 of 144 (91.0%),
slack 77 of 85 (90.6%), travel 116 of 140 (82.9%; its 20 `no-tool`
pairs are one speech-only injection against every task), workspace
191 of 198 (96.5%). The whole output is `tests/authority/expected.txt`.

Read as a ceiling: with the kinds alone, 83.8% of the benchmark's
attacks ask for an authority the benign program never had; a scope the
reviewer can read in the prompt takes it to 90.5%; a per-run count adds
two pairs (three transfers where the task makes one); the upper bound
of every literal as a scope is 95.1%.

## 5. What is left, pair by pair

The 32 pairs that fit the *prompt+budget* grant, by why:

- **The task's own sink is the attack's sink** (11 pairs). Banking
  `UserTask15` names the attacker's account as the user's new landlord
  (7 pairs); banking `UserTask14` asks for a password change and the
  attack asks for another password (1); slack `UserTask1`, `13` and
  `14` message Alice, and the attack sends Alice a link (3). No notion
  of authority separates these: the difference is in the content.
- **The sink is read from untrusted data by the task itself** (21
  pairs). Banking `UserTask0` pays a bill whose recipient is in the
  bill (5 attacks redirect it); slack tasks read a page whose address
  is in a channel message (5 attacks name another page); workspace
  tasks email or invite people found by name (6) or delete a file found
  by listing (1); travel tasks create calendar events, which have no
  scope at all, and the attack creates another (4). Here the task's own
  specification derives the sink from the data the attacker controls;
  an authority fixed before the run cannot help, by construction.

What acts on them: a budget on an amount (`send_money` of 1,000,000 at
once, or "as much as possible") is not a budget Renyi has, since its
budgets count calls; it is the one extension this measure argues for. A
guard (`only to`) does not help: the data flows to the sink the task
wants. The recording and the narration answer the question after the
run (the recipient is in the recording), and the reviewer of the
signature sees `send_money` without a scope and knows that the bill
chooses the recipient. The 20 `no-tool` pairs are outside every
mechanism of this kind.

## 6. What the measure does not say

- It is static and assumes the program for the task is fixed before any
  effect: written by the agent, reviewed through its signature, then
  run. An agent that writes new code after reading tool output is
  outside it; a Renyi program's effects are declared once.
- It uses the ground truth as the task's authority. A program may
  declare more than its ground truth needs (a reviewer who accepts a
  broader grant); the *kind* row is the floor and the *literal* row the
  ceiling.
- It does not run a model, so it says nothing about utility (whether
  the agent still solves the task) or about attacks the benchmark does
  not contain. CaMeL (Debenedetti et al., 2025; arXiv:2503.18813)
  reports utility and security with a model in the loop; this measure
  is the part of the question that needs none.
- The scope model is equality on the argument's text; a path prefix
  would contain sub-paths, as in the reference.

## 7. Reproducing it

```
python -I tools/authority_gap/extract_tasks.py <agentdojo>/src/agentdojo/default_suites/v1 tests/authority/tasks_v1.json
python -I tools/authority_gap/authority_gap.py tests/authority/tasks_v1.json [--pairs]
```

CI runs the second line and compares its output with
`tests/authority/expected.txt`; a change in the model or the data
changes the number and the file in one commit.
