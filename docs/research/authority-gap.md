# The authority gap: what a declared grant stops before the run

Status: measured 2026-10-16 (decision AX2) and with a model in the
loop the same day (decision AX4); the eleven combined tasks the first
extraction missed added, the model loop completed on them and its
programs run for utility on 2026-10-17 (decisions AX5 and AX6, sections
2, 7 and 8). The first result of the research track (decision AX1). The
scripts are `tools/authority_gap/`, the data and the pinned outputs
`tests/authority/`, which CI holds.

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
four suites (banking, slack, travel, workspace), 97 user tasks and 27
injection tasks, every user task of a suite paired with every injection
task of the suite, 629 pairs, the counts of the benchmark's own paper.
Eleven of the user tasks (slack 17 to 20, workspace 4, 19, 23 and 36 to
39) are combined tasks, built by the benchmark's `TaskCombinator` from
two others: the prompt the one given or "Perform the following tasks in
the order they are given: 1. … 2. …", the ground truth the two in
order. The first extraction (decision AX2) read the task classes alone
and missed them, 86 tasks and 567 pairs; decision AX5 added them, and
the numbers below are of the full set. Each task declares a
`ground_truth`: the list of tool calls, with their arguments, that carry
it out. Those lists are the data: the user task's is the smallest set
of calls a correct program makes, the injection task's the calls the
attack must make.

`tools/authority_gap/extract_tasks.py` reads the task files as syntax
trees and never runs them (an argument that resolves through class and
module constants, f-strings included, is `literal`; one computed from
the environment is `derived`; a combined task is read from its
`create_combined_task` call). `tests/authority/tasks_v1.json` is its
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
all: 629 pairs
  kind          blocked  527 ( 83.8%): kind 527, scope   0, budget   0; undetermined   0, fits  82, no-tool  20
  prompt        blocked  571 ( 90.8%): kind 527, scope  44, budget   0; undetermined   0, fits  38, no-tool  20
  prompt+budget blocked  573 ( 91.1%): kind 527, scope  44, budget   2; undetermined   0, fits  36, no-tool  20
  literal       blocked  600 ( 95.4%): kind 527, scope  73, budget   0; undetermined   0, fits   9, no-tool  20
```

Per suite (the *prompt+budget* grant): banking 131 of 144 (91.0%),
slack 94 of 105 (89.5%), travel 116 of 140 (82.9%; its 20 `no-tool`
pairs are one speech-only injection against every task), workspace
232 of 240 (96.7%). The whole output is `tests/authority/expected.txt`.

Read as a ceiling: with the kinds alone, 83.8% of the benchmark's
attacks ask for an authority the benign program never had; a scope the
reviewer can read in the prompt takes it to 90.8%; a per-run count adds
two pairs (three transfers where the task makes one); the upper bound
of every literal as a scope is 95.4%. On the 86 tasks of the first
extraction the rows were 83.8%, 90.5%, 90.8% and 95.1%: the eleven
combined tasks, whose grants are the union of two, moved the number by
a third of a point.

## 5. What is left, pair by pair

The 36 pairs that fit the *prompt+budget* grant, by why:

- **The task's own sink is the attack's sink** (12 pairs). Banking
  `UserTask15` names the attacker's account as the user's new landlord
  (7 pairs); banking `UserTask14` asks for a password change and the
  attack asks for another password (1); slack `UserTask1`, `13`, `14`
  and the combined `18` message Alice, and the attack sends Alice a
  link (4). No notion of authority separates these: the difference is
  in the content.
- **The sink is read from untrusted data by the task itself** (24
  pairs). Banking `UserTask0` pays a bill whose recipient is in the
  bill (5 attacks redirect it); slack tasks read a page whose address
  is in a channel message, the combined `18` and `20` among them (7
  attacks name another page); workspace tasks email or invite people
  found by name (6) or delete a file found by listing (2); travel tasks
  create calendar events, which have no scope at all, and the attack
  creates another (4). Here the task's own specification derives the
  sink from the data the attacker controls; an authority fixed before
  the run cannot help, by construction.

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
- It runs no model; section 7 puts one in the loop for the programs
  and section 8 runs them for utility (whether the agent still solves
  the task); attacks the benchmark does not contain are outside all
  three. CaMeL (Debenedetti et al., 2025;
  arXiv:2503.18813) reports utility and security with a model in the
  loop; the static measure is the part of the question that needs none.
- The scope model is equality on the argument's text; a path prefix
  would contain sub-paths, as in the reference.

## 7. With a model in the loop

The static measure takes the ground truth as the task's program. The
question it leaves is whether a model, asked to write that program in
Renyi, declares the same authority: more, and the gap narrows; less,
and the task is not done. Measured on 2026-10-16 (decision AX4, the 86
tasks of the first extraction) and completed on the eleven combined
tasks the next day (decision AX6), with Claude Sonnet 5.5, sampled
through the toolchain's own channel (the readability harness's:
`claude -p` on a subscription login, no tools, no MCP servers, an empty
working directory; what no switch removes is the CLI's frame and the
user-level `CLAUDE.md` of the machine's profile, unrelated to the tasks
and noted in `model_loop.py`).

**The setup.** Each suite's tools are a Renyi project
(`tests/authority/tools/<suite>/`, written by
`tools/authority_gap/gen_tool_modules.py` from the tools' signatures):
one Python module per tool, bound in `renyi.json` to the package
`tools.<tool>`, so that each tool is a capability kind of its own,
`python("tools.<tool>")`, and the record types a tool returns are
declared inside its module as `renyi bind --python` writes them. The
system text is the starter pack's skill sheet, the cheat sheet and a
note of twelve lines on the tools (in `model_loop.py`); the user text
is the task's prompt, the suite's declarations and the instruction to
write the module `task`, whose `main` declares in its `needs` exactly
the effects the program has. The program is checked with `renyi check`;
on an error, the program and the diagnostics go back and the model
writes it again, at most three times. The clean program's imports among
the suite's tools are what it declares: in Renyi the grant of `main`
covers every call, and an unused import or need is an error, so the
declared kinds and the called kinds are one set. The travel project, as
sampled, declared three tools of the calendar client that the travel
suite does not register (`get_current_day`,
`reschedule_calendar_event`, `add_calendar_event_participants`), from
the generator's hand-copied list; one program, travel `UserTask8`,
called one of them (section 8); the project in the repository is the
suite's 28.

**The result** (`tests/authority/model_loop_expected.txt`, from
`tests/authority/model_loop.json`, which holds every attempt and every
final program):

| suite | tasks | clean | exact | over | under | pairs | blocked, model's grant | blocked, ground truth |
|---|---|---|---|---|---|---|---|---|
| banking | 16 | 16 | 11 | 4 | 2 | 144 | 98 (68.1%) | 102 (70.8%) |
| slack | 21 | 21 | 14 | 4 | 5 | 105 | 88 (83.8%) | 86 (81.9%) |
| travel | 20 | 19 | 16 | 2 | 1 | 133 | 108 (81.2%) | 108 (81.2%) |
| workspace | 40 | 40 | 34 | 3 | 6 | 240 | 228 (95.0%) | 222 (92.5%) |
| all | 97 | 96 | 75 | 13 | 14 | 622 | 522 (83.9%) | 518 (83.3%) |

96 of the 97 tasks got a program the checker accepts: 55 at the first
attempt, 38 at the second, 3 at the third; travel's `UserTask2` (the
restaurants of a city by cuisine, price and rating) ended its third
attempt with two errors. Of the 96, 75 (78.1%) call exactly the
ground truth's tools; 13 call a tool the ground truth does not (one
each, two in one case: `get_iban` to compare the sender,
`get_current_day` for a date, `list_files` in place of `search_files`);
14 miss a tool the ground truth calls, of two kinds: another route to
the same end (`list_files` for `search_files`, `search_calendar_events`
for `get_day_calendar_events`) and a program that stops short of the
task because the task's actions are in the data the program reads
(workspace `UserTask13`, "do the actions specified in the email": the
program finds and prints the email; slack `UserTask18` and `19`, "do
all the tasks i have on my TODO list at <page>": the program fetches
the page and prints it, five of the ground truth's tools undeclared;
banking `UserTask9` and `UserTask12`, which read the scheduled
transactions and do not update them). A program fixed before the run
does not carry out instructions it has not read, and these programs
declare nothing for them. Under the grants the 96 programs declare, 522
of their 622 pairs are blocked (83.9%); under the ground truth's kinds
on the same pairs, 518 (83.3%); the static kinds row over all 629 is
83.8%. The two grants differ on 21 tasks and agree on the number: what
a model declares, writing the program in a language where the grant is
the call set, is what the benchmark's ground truth declares, and the
floor of section 4 holds with a model in the loop.

**The checker as the teacher.** The first attempt's errors, over the 97
first calls: `ignore-nothing` 14 (a call that returns nothing under
`ignore`), `maybe-value` 9, `unknown-type` 8, `expected` 5,
`reserved-word` 3, `type-mismatch` 3, `missing-ability` 2,
`missing-otherwise` 2, `type-name-shape` 2, `unknown-method` 2,
`equals-sign` 1. Every diagnostic carries its fix, and 38 of the 42
programs refused once were clean after one round of them. The sampling
took 1,496 seconds over 143 calls; the model's thinking was 80,839
tokens, median 474 per call.

**What this half says.** The authority a model declares for these
tasks is the ground truth's, so the static number is not an artefact of
the ground truth. Whether the programs do the tasks is section 8. One
model, one sample per task, a prompt fixed by the script: another model
or another prompt is another run, and a run is a new decision entry.

## 8. Utility: the programs run

The programs of section 7 were run (decision AX6) against AgentDojo's
own environments through the Python bridge. The suite's tools are the
package `tools` (`tools/authority_gap/utility_tools/__init__.py`, one
shim per tool written by `utility_run.py`): a call goes through the
benchmark's `FunctionsRuntime` with the environment's dependencies
injected as the benchmark injects them, and the environment is pickled
after every call for the judge (pickled, since the models rebuild
derived fields when validated again and a JSON round trip would lose
what the run added). `utility_run.py`, under the lane's virtual
environment with AgentDojo installed, runs each program with `renyi
run` (`RENYI_PYTHON` naming that interpreter) and asks the task's
`utility` of the program's output and the environment after the run,
`utility_from_traces` for the one task whose `utility` is not
implemented (slack `UserTask11`). The judge was validated first
(`utility_truth.py`): the ground truth of every one of the 97 tasks
runs through the same runtime without an error, and the pickle round
trip gives back an equal environment.

**The result** (`tests/authority/utility_expected.txt`, from
`tests/authority/utility.json`, which holds every program's output and
verdict):

| suite | programs | exit 0 | utility |
|---|---|---|---|
| banking | 16 | 15 | 10 (62.5%) |
| slack | 21 | 21 | 14 (66.7%) |
| travel | 19 | 18 | 5 (26.3%) |
| workspace | 40 | 37 | 25 (62.5%) |
| all | 96 | 91 | 54 (56.2%) |

54 of the 96 programs do their task (56.2%). Of the 42 that do not, 37
ran to the end and either did the wrong thing or said it the wrong way
(travel's rules read the output strictly, a price or a rating spelled
as the task expects, which is where travel's 26.3% comes from); 3
stopped on their own `crash` when the bill, the email or the minutes
they parsed had a format the program had guessed wrong; 2 failed, one
on a tool's error (a calendar query that found nothing) and one, travel
`UserTask8`, on `get_current_day`, the tool the fixture declared and
the suite lacks (section 7). The attempt a program took says little
(30 of 55 first-attempt programs do the task, 22 of 38 second-attempt,
2 of 3 third); the call set says more: 48 of the 75 programs calling
exactly the ground truth's tools do the task (64.0%), 6 of the 21
others (28.6%). The runs took 620 seconds.

**What it says.** A program written once, from the prompt and the
declarations and without a look at the data, does the task in 56% of
the cases on this benchmark, with the authority of section 7 and under
the gap of section 4. What the other 44% need is a look at the data
before the program is fixed: the loop the starter pack describes
(write, check, run, read the recording, write again), which keeps the
grant reviewed at every turn, and whose utility is the measure after
this one. The benchmark's leaderboard is not a comparison point here:
its agents read every tool's output between calls.

## 9. Reproducing it

```
python -I tools/authority_gap/extract_tasks.py <agentdojo>/src/agentdojo/default_suites/v1 tests/authority/tasks_v1.json
python -I tools/authority_gap/authority_gap.py tests/authority/tasks_v1.json [--pairs]
python -I tools/authority_gap/gen_tool_modules.py <agentdojo>/src/agentdojo/default_suites/v1/tools tests/authority/tools
python tools/authority_gap/model_loop.py --tasks tests/authority/tasks_v1.json --tools tests/authority/tools --out <runs> --renyi <binary> --repo . --home <dir> --work-dir <empty dir>
python -I tools/authority_gap/model_loop_report.py --tasks tests/authority/tasks_v1.json --runs <runs> --save tests/authority/model_loop.json
<venv>/python tools/authority_gap/utility_truth.py
<venv>/python tools/authority_gap/utility_run.py --tasks tests/authority/tasks_v1.json --runs <runs> --tools tests/authority/tools --pkg tools/authority_gap/utility_tools --out <utility runs> --renyi <binary>
python -I tools/authority_gap/utility_report.py --tasks tests/authority/tasks_v1.json --runs <utility runs> --loop <runs> --save tests/authority/utility.json
```

`<venv>` is a virtual environment with AgentDojo installed from the
checkout (`pip install -e <agentdojo>`); the measures before it run no
code of the benchmark. CI runs the second line and compares its output
with `tests/authority/expected.txt`, checks the four smoke programs of
`tests/authority/tools/`, and compares the reports on `model_loop.json`
and `utility.json` with `model_loop_expected.txt` and
`utility_expected.txt`; a change in the model or the data changes the
number and the file in one commit, and a new run of the model is a new
decision entry.
