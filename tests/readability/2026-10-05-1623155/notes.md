# Live round 1, 2026-10-05

The first round run through the vendor APIs, on the grammar at revision
`1623155` (the cheat sheet after decisions M1 to M10 and P1 to P4: `repeat
until`, the library section, the grant clauses). Three models from two
vendors, five samples per prompt, 69 prompts (10 Predict, 30 Explain, 19
Complete, 10 Write), 345 samples per model:

| Label | Vendor | Settings |
|-------|--------|----------|
| `claude-sonnet-5-5` | Anthropic | no `temperature` field: the API rejects it for this model ("deprecated"), so the samples are at the model's default |
| `claude-haiku-4-5-20251001` | Anthropic | temperature 0 (the floor model, decision M7) |
| `gpt-5.4-mini` | OpenAI | temperature 0; `gpt-5.5` rejects temperature 0, so the smaller model that accepts it was used |

Explain was graded by `claude-sonnet-5-5` with `gpt-5.4-mini` as the second
grader (section 3). Complete and Write were judged by Claude Code subagents
(section 4, decision L2).

## 1. Results

Pass rates per task by the four-of-five rule, against the acceptance
thresholds of `docs/design/03-readability-test.md` (Predict and Explain at
least 90, Complete at least 80, Write at least 70, every model). The
protocol tally runs `renyi format` before the lint (decision M5); the strict
tally does not:

| Label | Predict | Explain | Complete (protocol / strict) | Write (protocol / strict) |
|-------|---------|---------|----------|------|
| claude-sonnet-5-5 | 9/10 (90%) | 29/30 (97%) | 15/19 (79%) / 12/19 (63%) | 4/10 (40%) / 1/10 (10%) |
| claude-haiku-4-5-20251001 | 3/10 (30%) | 27/30 (90%) | 7/19 (37%) / 6/19 (32%) | 0/10 (0%) / 0/10 (0%) |
| gpt-5.4-mini | 8/10 (80%) | 29/30 (97%) | 6/19 (32%) / 6/19 (32%) | 0/10 (0%) / 0/10 (0%) |

No model meets every threshold. Sonnet passes Predict and Explain, misses
Complete by one item (as in the pre-test) and misses Write; Haiku and gpt
pass Explain only. The tally of this scoring is kept in
`outputs/<label>/scores-api.json`, the strict tally in
`scores-strict-api.json`, the per-sample reasons in `judgement.json`.
Explain was re-graded by the subagent graders in session 6 (section 8,
decisions U4 and U5): 29, 27 and 29 of 30 on that scale, which is round
2's; `scores.json` and `scores-strict.json` hold that tally.

## 2. Predict

Every reference output of a failing item was re-derived by hand from the
program before the tally; all ten references stand.

- Sonnet fails only `traffic_light`: all five samples give the right five
  lines but preface them with their reasoning and the remark that `to_text`
  of a variant "presumably" prints its name, against the prompt's "no
  explanation". The cheat sheet did not state the derived `ToText` rule
  (the library sketch does); it does now (commit `c994d06`).
- gpt fails `statistics` (two of five samples mis-add the mean: 5.25 for
  5.0) and `temperature_table` (four of five drop the two literal spaces
  between the padded columns).
- Haiku fails seven of ten: wrong column widths (`invoice_report`,
  `temperature_table`), the alphabetical tie-break ignored
  (`dependency_order`), the outer parentheses dropped (`expression_tree`),
  the smaller shape named largest (`shapes`), a mis-added mean
  (`statistics`), and an extra `Red` before the emergency
  (`traffic_light`). None of these is a grammar misreading; they are
  arithmetic and attention errors of the small model.

## 3. Explain

Each explanation was graded by both graders against the author's
description of the program (`tests/readability/reference/<name>.explain.txt`:
purpose, inputs, outputs, effects, failures) with the rubric in `run.py`:
5 when nothing is wrong and the purpose, the effects and the main behaviours
are stated, 4 when one of those is missing, 3 when the explanation is vague
or several are missing, 2 when one statement is wrong, 1 for a different
program. The first grader's grade stands when the second is within one point
of it. The 49 samples where the graders disagreed by more (15 Sonnet, 20
Haiku, 14 gpt) were adjudicated by hand against the program and the
description, with a reason each in `judgement.json` (36 pass). Sonnet
refused to grade four of Haiku's `concurrent_fetch` explanations; a
refusal decides nothing, so gpt's grade stood there.

Grades over the 150 samples per model: Sonnet 103 fives, 42 fours, 5 twos;
gpt 30 fives, 114 fours, 1 three, 5 twos; Haiku 6 fives, 130 fours, 8
threes, 6 twos. The common grade of 4 means the explanation states the
purpose and the behaviour but not what the program needs from outside
(console, files, network, clock): the readers say "prints" and "reads" but
rarely name the needs the signatures declare.

Failing items:

- `shipping_rules` fails on every model. Sonnet (two samples) and gpt (all
  five) write "2.50 per full kilogram beyond the first"; `weight_steps` is
  `(weight_grams - 1).quotient(1000)`, every started kilogram beyond the
  first, and the program's own example prices 2500 grams to Europe at 9.50
  plus 5.00. The readers repeated the function's purpose line, "how many
  whole kilograms beyond the first the parcel weighs", which was itself
  imprecise; after the round it reads "started kilograms" in the corpus
  (the stored prompts keep the text the models saw). Haiku's five samples
  say "orders above 50.00" for `at least 50.00` and describe the pricing as
  "tiered rates for international destinations" (every non-free parcel,
  Domestic included, pays the destination base price plus the weight steps).
- Haiku also fails `file_tree` (three samples graded 3: the sorting, the
  directory sums and the error behaviour are all missing) and
  `inventory_db` ("items below their reorder level" for at or below, and
  the unknown-sku failure, the table creation and the "nothing to reorder"
  line missing).

The boundary wording decided the adjudications more than anything else:
"above 50.00" for `at least 50.00` and "below the level" for `<=` were
counted as wrong statements (grade 2) wherever the program states the
boundary explicitly; a verb that implies an effect ("prints", "downloads")
without naming the need was counted as the effects missing (grade 4), and
an explicit "console-only" or "to the console" as stated.

## 4. Complete and Write

A sample passes when it is formatted by `renyi format`, passes the lint and
`renyi check`, and a judge finds every `example:` line and `test` block
holds and the function or program does what its purpose or task says. The
lint and the checker (new since the pre-test) decide most samples: of the
145 Complete and Write samples per model, the lint failed 19 of Sonnet's, 56
of Haiku's and 49 of gpt's, and the checker 28, 50 and 51 more; the rest
went to the judges.

The 182 lint- and check-clean samples (98 Sonnet, 39 Haiku, 45 gpt) were
judged by ten Claude Code subagents, each given about nine packets (one
packet per distinct program text, 91 in all) and the cheat sheet, the
prelude sketch and the rendering rules, with the instruction to evaluate
every example and test by hand and to write a one-sentence reason; the
verdicts are in `outputs/<label>/judgement.json` (decision L2). 83 packets
pass and 8 fail (12 samples: 4 of Haiku's, 8 of gpt's; all 98 of Sonnet's
pass): two bodies of `balanced` write `ignore pop(stack)` and so never
shrink the stack; four bodies of `quote` charge 2.00 instead of the 2.50 per
weight step the example implies; two bodies of `statement_line` pad to the
wrong widths; one `request_window` program resets a fixed window instead of
sliding. Three judgement calls are recorded in the reasons: Decimal `is`
compares values and ignores scale (`0.00 is 0`, `32.0 is 32`, the IEEE
decimal128 reading, which the checker's refinement evaluation also uses but
no document states); a request exactly one window length earlier is outside
the window; `EmergencyCleared` outside an emergency gives `Red`.

Why items fail, per model (the first rule of each failing sample; a sample
may break several):

- Sonnet, Complete (4 items): the reserved word `count` used as a local
  name (`deadlines`: two of five samples, `invoice_report`: three) and
  `rounded()` called without its `places` argument (`invoice`: all five;
  `shapes`: two). Forgiving the reserved-word rule alone would lift Sonnet
  to 17 of 19 (89 percent).
- Sonnet, Write (6 items): a public type or the module without `purpose:`
  (`letter_grades`, `roman_numerals`, `title_length_tool`, `compound_interest`),
  a fallible call without `otherwise` (`roman_numerals`, `title_length_tool`,
  `letter_grades`), `rounded()` without `places`, a namespace imported twice,
  lines over 100 columns the formatter cannot wrap (long string literals
  and `example:` lines), and one `password_strength` program judged wrong.
- Haiku: reserved words as names (`count`, `sorted`, `first`: `deadlines`,
  `statistics`, `todo_cli`, `word_count`), unused bindings
  (`dependency_order`, `shapes`), block imbalance (`traffic_light`), a named
  single argument (decision M4: 92 samples) and unnamed arguments in
  multi-argument calls (30 samples), `.length` as a field, `otherwise`
  missing on a fallible call (`semver`), `purpose:` missing on types or the
  module, invented library names, and the judged `quote` bodies.
- gpt: a named single argument (77 samples) and `purpose:` missing on the
  module (every Write program), a local named like a function of the module
  (`subtotal` in `invoice`: all five), `maybe` values used as plain values
  (`log_parser`, `stacks`), reserved words (`count`, `sorted`), `remainder`
  and `all` written as functions, lines the formatter cannot wrap, and the
  judged `statement_line`, `quote` and `balanced` bodies.

The protocol's quarter rule for Write (no single lint rule over a quarter
of the violations) is broken by Sonnet: `check:purpose-missing` is 28
percent of its 58 violations (then `missing-otherwise` 14, `line-width`
12). For gpt it is exactly a quarter (28 of 112, then `argument-name` 19);
Haiku's largest is `argument-name` at 16 percent of 134. In the strict
tally `line-width` alone is 83 percent of Sonnet's violations: the models
write lines the formatter wraps, which is what decision M5 expects.

## 5. Harness fixes made during the round

Each of these was found by the round and fixed in `run.py` before the final
tally; the README describes the behaviour as it now is.

1. `run` picked the run directory by name and wrote the first Haiku and gpt
   samples into the pre-test directory; it now takes the newest `meta.json`,
   and `--run` names one explicitly. The stray outputs were removed and the
   samples re-collected.
2. `--temperature none` omits the field for models that reject it (Sonnet).
3. A program that imports another corpus module is shown together with
   that module (`with_imports`): the first `invoice_report` prompts showed
   `invoice.total` without its definition. The three `invoice_report`
   prompts were regenerated and their 45 samples re-collected; the Predict
   answers changed for Haiku only (from prose to a wrong table).
4. On Windows the scratch file was written with CRLF line endings, which
   the formatter and the checker reject, so every Complete and Write sample
   failed with `check:crlf`; the scratch file is now written with LF.
5. Complete and Write samples run through `renyi check` after the lint
   (rules `check:<code>`), and the grades of Explain are cached.
6. The Explain grader's answer was parsed by its first digit; Sonnet 5.5
   reasons before answering, so "users at least 18" or "a 10% discount"
   returned a grade of 1 for correct explanations. The grade is now the
   last standalone digit, and the grader's answer is cached with the grade.
7. The grader read only the `purpose:` lines, so a correct detail the
   purposes do not mention ("then prints a count line") was graded as
   wrong. The grader now compares against an author's description of every
   program (`reference/<program>.explain.txt`) with a rubric; a second
   grader grades every explanation, and a disagreement of more than one
   point is adjudicated by hand in `judgement.json`.
8. Sonnet 5.5 refuses to grade some explanations of `concurrent_fetch`
   (`stop_reason: refusal`, empty content); a refusal decides nothing and
   the other grader's grade stands.
9. Every `score` process wrote its sample to the same scratch file
   `examples/.scratch.ry`, so three processes run at once read, formatted
   or deleted each other's sample (two of them crashed in the final
   re-score). The scratch name now carries the process id, and the lint's
   scan of declared names skips other scratch files. All three tallies were
   re-made after the fix; the per-task counts of lint, checker and judged
   samples equal the earlier ones.

## 6. Open questions for the owner

Raised as decision questions after this round and answered the same day
(decisions R1 to R8 in `docs/design/01-decisions.md`: the freeze is gated on
the large models, J3 and M4 stand, `purpose:` stays and the cheat sheet says
it louder, `is` compares values of one type, `ignore` only on calls with
effects, budgets 10 / 5 / 7, gpt-5.5 joins the next round):

- Haiku 4.5 as the floor model (M7) fails Predict at 30 percent before any
  grammar question arises; the thresholds say "every model".
- The reserved word `count` costs Sonnet two Complete items (89 percent
  without it); `sorted` and `first` cost Haiku and gpt more.
- `purpose:` on public types and on the module is the largest single Write
  rule for Sonnet and gpt, and breaks the quarter rule for Sonnet.
- A named single argument (M4) is the largest checker rule for Haiku and gpt.
- Decimal `is` and scale (`0.00 is 0`) is undefined in the documents; the
  judges and the checker assume value equality.
- `ignore pop(stack)` on an immutable value silently does nothing; a
  checker warning for a discarded result whose receiver is a local of the
  same type would catch it.

Corpus findings that are not grammar questions: the cheat sheet now states
the derived `ToText` rule for variants, and the purpose line of
`weight_steps` in `shipping_rules.ry` now says "started kilograms" (the
imprecise "whole kilograms" misled every model's explanation).

## 7. The VM re-judges Complete and Write (decision L2, session 6)

With the VM built (M3), `run.py compare` set its verdict on every judged
Complete and Write sample against the subagents' (`judgement.json`): the
sample is spliced or taken whole as `score` does, formatted, linted,
checked, and then `renyi test` runs the program's own `example:` lines and
`test` blocks.

| Label | Judged | Agree | Disagree | Rejected today before any judge |
|-------|--------|-------|----------|-------------------------------|
| claude-sonnet-5-5 | 98 | 98 | 0 | 0 |
| claude-haiku-4-5-20251001 | 39 | 39 | 0 | 0 |
| gpt-5.4-mini | 45 | 42 | 0 | 3 (`stacks`: `ignore` of a pure call, decision R6, which came after the judging; the subagent had rejected them too) |

Before that table could be written the comparison found one disagreement,
on every `sales_report` sample that wrote

```
return for each sale in sales group by sale.region sum sale.amount
```

(five of Sonnet's, five of Haiku's; gpt wrote a loop): the judges accepted
it, citing decision M1 (a terminal after `group by` applies per group), and
the VM printed `0` for the empty list where the example wants `{}`. The VM
was wrong: its compiler summed over the whole list whatever the `group by`.
It now folds `sum`, `count`, `first`, `any` and `all` per group
(`Op::GroupFold`, `crates/renyi_vm/tests/queries.rs`), and
`totals_by_region` in the corpus is written in the M1 form with a second
example. The ten samples agree after the fix.

Both tallies were then re-scored with the VM deciding (`score` with the
cached Explain grades, no API call): every verdict is the one the subagents
gave, so the table of section 1 stands. From here the VM judges Complete
and Write, and `judgement.json` is consulted only for a program with
nothing to run.

## 8. Explain re-graded by the subagent graders (decision U4, session 6)

Round 2 graded Explain with Claude Code subagents instead of the API
graders of section 3 and found them about a point more lenient; the owner
then made the subagents the graders from here on and had this round
re-graded with them, so that the two rounds are on one scale. The method
is round 2's: each model's 150 explanations in three batches of fifty,
with the references as this run saw them (now snapshotted in
`reference/`; `currency_tool.explain.txt` still names `api.frankfurter.app`,
the host of this round's corpus), one Sonnet 5.5 subagent per batch as
the first grader and one Opus 5.5 subagent as the second, the rubric
verbatim, a one-sentence reason per grade. The answers were merged into
`grades.json` under `agent:claude-sonnet-5-5` and `agent:claude-opus-5-5`
with the harness's sample digests (`merge_agent_grades.py`), and `score`
ran without an API call. Eighteen subagents, 2.07 million tokens of
subscription quota; one of them (Haiku's second batch, first grader)
wrote its file twice, the second time after the session's quota was
interrupted and restored, and the final file is the one merged. The hand verdicts of section 3 stand: they are
verdicts on the samples, not on the graders, and the adjudication rule
reaches only samples the graders disagree on by more than one point. The
two subagents never disagreed by more than one point on any of the 450
samples, so nothing was adjudicated anew.

On the subagents' grades and the hand verdicts of section 3, Explain
passed 29 of 30 items for every model (Sonnet 29, Haiku 29, gpt-5.4-mini
29), `shipping_rules` the one failing item everywhere; under the API
graders Haiku had 27, and `file_tree` and `inventory_db` passed on the
subagents' grades. The first grader's grades over the 150 samples per
model: Sonnet 141 fives, 5 fours, 4 twos; Haiku 118 fives, 32 fours;
gpt-5.4-mini 142 fives, 3 fours, 5 twos. The two subagents give the same
grade on 145, 121 and 133 of the 150 samples; where they differ, Opus is
the stricter on 5, 19 and 17 samples and Sonnet on 0, 10 and 0. Sample
by sample against the API first grader, the subagent never grades lower:
it gives the same grade on 108, 31 and 36 samples (Sonnet, Haiku, gpt),
one point more on 38, 102 and 111, two more on 0, 15 and 2, three more
on 4, 2 and 1. Most of the difference is the API grader's fours for
needs left unnamed becoming fives: the subagent takes "prints" and
"reads a file" as stating the console and the filesystem.

With the hand verdicts of section 3 applied, 1, 9 and 1 samples flipped
from fail to pass and none the other way. Eight of the eleven were the
API grader's threes ("several things missing": the needs, an error case,
a detail of the output) that both subagents read as complete or as one
thing missing: Haiku's `file_tree.0`, `.2` and `.3`, `inventory_db.1`,
`todo_cli.2`, `traffic_light.1` and `word_count.0`, and gpt-5.4-mini's
`word_count.3`. The other three were wrong statements that both
subagents let pass, which the adjudication rule did not reach:

- Sonnet `retry.3`: "the last error, which includes its attempt number"
  contradicts the description (GaveUp carries the last error text only;
  the attempt prefix is in the per-attempt records). The API graders
  gave 2 and 2, the subagents 5 and 5 ("complete"). Round 2 adjudicated
  the same statement in Sonnet's `retry.4` as wrong.
- Haiku `inventory_db.2`: "items that have fallen below their reorder
  thresholds" for at or below, the sentence the hand verdict on `.0`
  calls wrong (all five samples share it; the API grader gave `.2` a 3,
  calling the sentence "technically contradicted at equality"). The
  subagents gave 5 and 5.
- Haiku `shipping_rules.0`: "orders above 50.00" for at least 50.00; the
  API graders gave 2 and 2, the subagents 4 and 4, the first grader
  noting the threshold "given loosely as 'above' rather than 'at least'"
  without counting it wrong, while in round 2 the same subagent grader
  counted the same sentence of Haiku's as wrong.

So the subagent graders miss a contradiction carried in a subordinate
clause and read a boundary word loosely in one batch and strictly in
another. The owner then decided (U5) that a hand verdict on a sentence
extends to every sample that repeats it, in any round, and nine verdicts
were added here, each naming the originating one: Haiku's
`inventory_db.1` to `.4` (the sentence of `.0`), `shipping_rules.0` (the
sentence of `.1` to `.4`), `invoice.2` and `.4` ("quantities and prices
are non-negative", the sentence round 2's `invoice.0` verdict calls
wrong; the session 5 verdicts of 4 on these two had read it as correct
and are withdrawn, the quantity being at least 1 at this revision too);
Sonnet's `shipping_rules.1` (the statement of `.3`) and `retry.3` (the
statement of round 2's `retry.4`). Round 2 got five, its Haiku
`inventory_db`. After U5 Explain passes 29, 27 and 29 of 30 (Sonnet,
Haiku, gpt-5.4-mini): Haiku fails `shipping_rules`, `inventory_db` and
`invoice`, the count it had under the API graders, with `file_tree`
passing and `invoice` failing instead. Against the API tally sample by
sample the tallies now differ on 0, 10 and 1 samples: Haiku's six API
threes above pass and `inventory_db.3`, `.4`, `invoice.2` and `.4` fail
by the new verdicts; gpt-5.4-mini's `word_count.3` passes. A wrong
statement that neither grader catches and no hand verdict names still
passes; a hand pass over every agreed grade was declined with U5 as not
worth it while the gating models are unaffected.
