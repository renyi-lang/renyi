# The authority gap

The data and the pinned results of the measures of decisions AX2, AX4,
AX5 and AX6 (`docs/research/authority-gap.md`): for every pair of a
user task and an injection task of AgentDojo's `v1` suites, whether the
attack fits inside the grant the user task's program declares, the
grant the ground truth implies (AX2, AX5) and the grant a model's own
program declared (AX4, AX6); and whether those programs do their task
when run (AX6).

- `tasks_v1.json`: every user and injection task of the four suites
  (banking, slack, travel, workspace; 97 user tasks, the eleven
  combined ones among them since decision AX5, and 27 injection
  tasks) as data: its prompt or goal and its ground-truth tool calls
  with their arguments, each argument `literal` or `derived`.
  Extracted by
  `tools/authority_gap/extract_tasks.py` from AgentDojo at commit
  `089ed468cf3ed0322acc66b0211f26d9d90dbf60`
  (https://github.com/ethz-spylab/agentdojo, MIT license, copyright
  2024 Edoardo Debenedetti, Jie Zhang, Mislav Balunović, Luca
  Beurer-Kellner, Marc Fischer and Florian Tramèr); the task texts are
  theirs.
- `expected.txt`: the output of `tools/authority_gap/authority_gap.py`
  on it, which CI compares with a fresh run.
- `tools/<suite>/`: the suite's tools as a Renyi project, one Python
  module per tool (`<tool>.ry`, its signature read from the tool's
  source by `tools/authority_gap/gen_tool_modules.py`, the record types
  it returns declared inside it), `renyi.json` binding each to the
  package `tools.<tool>`, so that each tool is its own capability kind,
  and `smoke_<suite>.ry`, a program that calls every tool once, which CI
  checks. The declarations are what the model reads in the measure of
  AX4; nothing here runs the tools.
- `model_loop.json`: the record of that measure (`tools/authority_gap/model_loop.py`,
  Claude Sonnet 5.5 through `claude -p`, 2026-10-16): for every user
  task, its attempts (the outcome, the checker's error codes, the
  seconds and the thinking tokens of each), the final program and the
  tools it calls, and the ground truth's tools.
- `model_loop_expected.txt`: the output of
  `tools/authority_gap/model_loop_report.py` on it, which CI compares
  with a fresh run.
- `utility.json`: the record of the programs run against the
  benchmark's environments (decision AX6, `tools/authority_gap/utility_run.py`
  under a virtual environment with AgentDojo installed, the package
  `tools/authority_gap/utility_tools/` as the tools' Python side): for
  every program, its exit status, its output, the calls it made and the
  verdict of the task's own `utility`.
- `utility_expected.txt`: the output of
  `tools/authority_gap/utility_report.py` on it, which CI compares with
  a fresh run.

To refresh after a change to the model (`SCOPE`, `kind_of`, the grant
rules) or to the data:

```
python -I tools/authority_gap/authority_gap.py tests/authority/tasks_v1.json > tests/authority/expected.txt
python -I tools/authority_gap/model_loop_report.py --tasks tests/authority/tasks_v1.json --results tests/authority/model_loop.json > tests/authority/model_loop_expected.txt
python -I tools/authority_gap/utility_report.py --tasks tests/authority/tasks_v1.json --results tests/authority/utility.json > tests/authority/utility_expected.txt
```

and change the numbers in `docs/research/authority-gap.md` and the
article in the same commit. A new run of the model (`model_loop.py`,
then `model_loop_report.py --runs <dir> --save tests/authority/model_loop.json`)
is a new decision entry: its numbers are the sample's.
