# The authority gap

The data and the pinned result of the measure of decision AX2
(`docs/research/authority-gap.md`): for every pair of a user task and
an injection task of AgentDojo's `v1` suites, whether the attack fits
inside the smallest grant the user task's program can declare.

- `tasks_v1.json`: every user and injection task of the four suites
  (banking, slack, travel, workspace) as data: its prompt or goal and
  its ground-truth tool calls with their arguments, each argument
  `literal` or `derived`. Extracted by
  `tools/authority_gap/extract_tasks.py` from AgentDojo at commit
  `089ed468cf3ed0322acc66b0211f26d9d90dbf60`
  (https://github.com/ethz-spylab/agentdojo, MIT license, copyright
  2024 Edoardo Debenedetti, Jie Zhang, Mislav Balunović, Luca
  Beurer-Kellner, Marc Fischer and Florian Tramèr); the task texts are
  theirs.
- `expected.txt`: the output of `tools/authority_gap/authority_gap.py`
  on it, which CI compares with a fresh run.

To refresh after a change to the model (`SCOPE`, `kind_of`, the grant
rules) or to the data:

```
python -I tools/authority_gap/authority_gap.py tests/authority/tasks_v1.json > tests/authority/expected.txt
```

and change the numbers in `docs/research/authority-gap.md` and the
article in the same commit.
