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

## Collection

Session 3 collected every Haiku sample and the Sonnet Predict and Explain
samples before the account's usage limit stopped the Sonnet subagents.
Session 4 collected the missing 15 Sonnet Complete and 10 Sonnet Write
samples the same way (one fresh `sonnet` subagent per prompt, reading only
`system.txt` and the prompt file, writing its answer to `answers/`), imported
everything with `run --provider file`, judged the Complete and Write samples,
graded the explanations and ran `score` and `report`.

Explain grading deviates from the harness in one more way: instead of one
grader call per explanation, six Sonnet subagents each graded a batch of ten
items, every item being exactly the text `EXPLAIN_GRADER` would send, with
the instruction to grade items independently. The grades are in
`outputs/<label>/judgement.json` next to the Complete and Write verdicts;
every entry carries a one-sentence reason (decision L2).

Two lint bugs surfaced while scoring and were fixed in `tools/lint_examples.py`
before the final tally: an indented comment line or a trailing comment was
reported as trailing whitespace (the check ran on the line with the comment
cut off), and a word such as `case` inside `purpose:` text was reported as a
foreign keyword. Neither affects the corpus, which has no indented comments.
Six Sonnet samples had been failing on the first bug alone.

## Results

The tally below follows the protocol as amended by decision M5 (`renyi
format` runs before the lint; the formatter leaves a program that does not
parse alone) and decision M1 (`group by ... sum` is a valid query, which
turned the two `totals_by_region` samples from failing to passing). It is in
`outputs/<label>/scores.json`; the strict tally, in which layout problems
count, is kept in `scores-strict.json` (`score --no-format`).

| Label | Predict | Explain | Complete | Write (protocol / strict) |
|-------|---------|---------|----------|------|
| agent-sonnet | 10/10 (100%) | 30/30 (100%) | 15/19 (79%) | 7/10 (70%) / 5/10 (50%) |
| agent-haiku | 8/10 (80%) | 29/30 (97%) | 8/19 (42%) | 0/10 (0%) / 0/10 (0%) |

Against the acceptance thresholds of `03-readability-test.md` (Predict and
Explain 90%, Complete 80%, Write 70% with no lint rule above a quarter of the
violations): Sonnet passes Predict, Explain and Write and misses Complete by
one sample (16 of 19 would pass); Haiku passes Explain only. One sample per
item makes every rate coarse: the protocol's five samples and four-of-five
rule are still owed. With two Write violations in total (both invented
library names) the quarter rule has no meaning at this sample size.

The Explain grades are suspiciously uniform (Sonnet: thirty 5s; Haiku:
sixteen 5s, thirteen 4s, one 3), which is the known generosity of a model
grader; the live round should use the second grader the protocol provides
for disagreements, and the owner may want to spot-check a few grades.

## Why samples failed

Every failing Complete or Write sample is explained by one of these causes;
the judgement reasons carry the tag in brackets, lint failures carry the
rule name from `report`.

| Cause | Sonnet | Haiku | Examples |
|-------|--------|-------|----------|
| invented library names | 4 | 4 | `split(separator:, limit:)`, `List.get`, `lowercase`, `words`, `first_character`, `uppercase`, `between`, `std.files`; Haiku `args`, `to_upper_case`, `find`, `substring` |
| layout only (line width, fixed by `renyi format`; no longer counted since decision M5) | 3 | 1 | example lines of 102 to 283 columns |
| `group by key sum expression` (not in the sketch; allowed since decision M1, so these two samples now pass) | 1 | 1 | both models, same program (`totals_by_region`) |
| unused query variable in `for each line in order.lines count` | 1 | 1 | both models, same program (`statement_line`) |
| `otherwise` on a value that cannot fail | 1 | 1 | `Port(8080) otherwise crash with ...`, `line.split(" ") otherwise fail with ...` |
| fallible call or refined construction without `otherwise` | 0 | 2 | `number_at(...)` without `otherwise fail`, `Done(position: position)` from a runtime value |
| unused loop variable in a `collect` query | 0 | 1 | `for each index in from 1 to width collect "-"` |
| reserved word as a name | 0 | 2 | `count` |
| single-letter name | 0 | 3 | `n`, `w`, `s`, `_` |
| `if` without `then`, block/`end` imbalance, top-level `let` without a type | 0 | 6 | Haiku Write and `shipping_rules` |
| missing `purpose:` on a public type | 0 | 1 | `type Grade` |
| infix `quotient`, wrong rendering | 0 | 2 | `length quotient 2`; `{shape.area()}` prints 6 where 6.00 is expected |

Judging rules applied, beyond the lint: a sample passes when it parses
(`renyi check`), would type-check under the sketch (named arguments for two
or more, `otherwise` exactly on `maybe` values and fallible calls, every
binding read, no library name the sketch does not declare), and its
`example:` lines and `test` blocks hold by hand evaluation. The loop variable
of a `first` query counts as read, because `first` returns it. The VM re-runs
every verdict at M3 (decision L2).

Other observations:

- Both models wrote `for each x in from 1 to 100`, which the parser accepts
  and the formatter normalizes to `for each x from 1 to 100`; no cost.
- Both models sometimes name a single argument (`column_widths(table: table)`,
  `score_to_grade(score: 95)`); the sketch says one argument is positional,
  the parser accepts the name. Not counted against any sample here, but the
  checker must decide (an error with the fix "drop the name" keeps one
  spelling).
- A Sonnet participant noted that the cheat sheet does not say what a derived
  `ToText` prints for a variant without fields; the reference outputs assume
  the bare variant name.
- Haiku's errors are mostly the rules the cheat sheet states outright (`then`,
  `end`, reserved words, single-letter names); Sonnet's are mostly library
  names the cheat sheet does not list. A short library section in the cheat
  sheet (prelude text, list and map methods; the module names) would address
  the larger Sonnet cause; the budget has about 550 tokens left.

The owner's answers to the questions these results raised are decisions M1
to M10 in `docs/design/01-decisions.md`: per-group terminals after `group
by`, the `count` loop variable stays unused, a superfluous `otherwise` and a
named single argument are errors, scoring formats first, the cheat sheet
gains a library section, Haiku stays the floor model, an MCP server for the
toolchain follows M2, and `repeat until` replaces `while` (the loops of the
pre-test samples were written under the old grammar and are judged as such).

## The VM re-judges Complete and Write (decision L2, session 6)

`run.py compare` on this round, with the VM of M3 (see the live round's
notes, section 7): agent-sonnet agrees on 18 of its 25 judged Complete and
Write samples and agent-haiku on 9 of 15; the other 13 are rejected today
before any judge sees them, by the checker, which did not exist at the
pre-test (`superfluous-otherwise`, `argument-name`, `argument-count`,
`missing-otherwise`, `unknown-method`, `unknown-module`,
`purpose-missing`), by the parser (an invented operator, `length quotient
2`), and by the lint's foreign-keyword rule, which rejects the `while`
loops that decision M10 replaced. Four of those thirteen had been judged
correct (agent-sonnet's `deadlines`, `dependency_order`, `markdown_table`
and `roman_numerals`, all `while` loops under the old grammar), so after
the re-score agent-sonnet stands at 12 of 19 Complete and 6 of 10 Write in
`scores.json` instead of 15 and 7; nothing else moved. The `sales_report`
sample of each agent was the VM bug the live round's notes describe, fixed
before the re-score.
