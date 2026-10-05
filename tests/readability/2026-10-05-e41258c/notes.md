# Pre-test round, 2026-10-05

This round was produced inside a Claude Code session, not through vendor
APIs: no API keys were available in the environment. Each sample comes from
a fresh Claude Code subagent (model aliases `haiku` and `sonnet`, labelled
`agent-haiku` and `agent-sonnet`) that was instructed to read exactly two
files, `system.txt` (the cheat sheet) and one prompt file, and to write its
answer to a file. Temperature was not controllable; one sample per prompt.

Deviations from `docs/design/03-readability-test.md`: one vendor instead of
two, one sample instead of five, agents instead of raw API calls. Explain
was graded by fresh subagents given only the program's documentation clauses
and the explanation; Complete and Write were judged in the session
(decision L2) with reasons in `outputs/<label>/judgement.json`.

The round is a smoke test of the cheat sheet, not the acceptance run.

## Answer-key leak and redo

The first version of `prepare` stored the reference output inside each
Predict prompt file and the original program inside each Complete prompt
file. API runs never see those fields, but the subagents of this round read
the whole file, and two of them reported seeing the key. All Predict and
Complete samples produced before the fix were discarded and re-collected
from prompt files that hold no key; Explain and Write prompts never held one.

## Collection status at the pause (2026-10-05, end of session 3)

The session paused on the owner's request when the account reached its usage
limit. The Sonnet subagents had already failed with HTTP 429 ("weekly limit,
resets Oct 8, 3pm UTC"). Every raw answer collected so far is saved under
`answers/` (`agent-haiku/`, `agent-sonnet/`, and `tainted/` for the samples
quarantined after the answer-key leak). `outputs/` was rebuilt from `answers/`
with `run --provider file` and scored with `score`; no judgement exists yet.

| Label | Predict | Explain | Complete | Write |
|-------|---------|---------|----------|-------|
| agent-haiku | 10/10 collected, 8 pass | 30/30 collected, ungraded | 19/19 collected | 10/10 collected |
| agent-sonnet | 10/10 collected, 10 pass | 30/30 collected, ungraded | 4/19 collected | 0/10 collected |

Sonnet Complete answers present: config, deadlines, dependency_order,
expression_tree. Those four agents wrote their file and then failed on their
final reply with the 429, so the files are complete and usable.

Missing, to collect after the limit resets (same prompt template as above,
one fresh subagent per item, model `sonnet`, answer file under
`answers/agent-sonnet/<task>/<name>.0.txt`):

- Complete (15): invoice, invoice_report, log_parser, markdown_table,
  permissions, sales_report, semver, shapes, shipping_rules, stacks,
  statistics, temperature_table, todo_cli, traffic_light, word_count.
- Write (10): compound_interest, fizz_words, initials, letter_grades,
  low_stock_report, merge_sorted, password_strength, request_window,
  roman_numerals, title_length_tool.

Observations so far, before judging:

- Both Haiku Predict misses are column-padding arithmetic (`pad_left` width
  off by one or more), not misreadings of the language: `invoice_report`
  shifted one column, `temperature_table` printed `104` one column left.
- Haiku Write: 9 of 10 samples fail the lint. Causes: lines over 100 columns
  (4, all fixable by `renyi format`), calls to undeclared functions
  (`to_upper_case`, `find`, `substring`, `args`), `if` without `then` (2),
  block/`end` imbalance (3), single-letter names. Haiku Complete: 6 lint
  failures, including the reserved word `count` used as a name twice.
- A Sonnet participant noted a cheat-sheet gap: it does not say what a
  derived `ToText` prints for a variant without fields (presumably the bare
  variant name).

Remaining steps: collect the 25 missing Sonnet samples; re-import with
`run --provider file --answers tests/readability/2026-10-05-e41258c/answers/<label> --model <label> --samples 1`;
run `score`; check every lint-clean Complete and Write sample with
`renyi check` as well as by reading, and record booleans with reasons in
`outputs/<label>/judgement.json` (decision L2); grade the 60 explanations
(integer grades in the same file, or `score --grader`); `report`; write the
findings here and turn any failing threshold into questions for the owner.
