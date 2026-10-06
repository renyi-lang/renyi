# Live round 2, 2026-10-06

The second round, on the grammar at revision `ed37120`: the cheat sheet
after decisions R1 to R8 (the `purpose:` rule said louder, `is` on numbers
compares values of one type, `ignore` only on calls with effects, the
derived `ToText` rule stated), 2977 of 3000 tokens, and the corpus after
session 6 (six programs gained `replays` tests, `currency_tool` moved to
`api.frankfurter.dev`, `totals_by_region` in `sales_report` is written in
the form of decision M1 with a second example line, which the Complete
prompt now shows). Four models from two vendors, the same 69 prompts as
round 1 (10 Predict, 30 Explain, 19 Complete, 10 Write), five samples per
prompt, 345 samples per model:

| Label | Vendor | Settings |
|-------|--------|----------|
| `claude-sonnet-5-5` | Anthropic | no `temperature` field (the API rejects it for this model); a gating model (decision R1) |
| `gpt-5.5` | OpenAI | no `temperature` field (the API rejects 0); joins as the second gating model (decision R8). Predict, Explain, Complete and the Write samples of `compound_interest` through the API; the other 45 Write samples through the Codex CLI (section 5) |
| `claude-haiku-4-5-20251001` | Anthropic | temperature 0 (the floor model, decision M7; reported as a trend since R1) |
| `gpt-5.4-mini` | OpenAI | temperature 0 |

Explain was graded by two Claude Code subagents, Sonnet 5.5 first and Opus
5.5 second, instead of the API graders of round 1 (section 3 and section
5). Complete and Write were judged by the VM (decision L2): a sample passes
when it is formatted, passes the lint and `renyi check`, and every
`example:` line and `test` block of the program holds.

## 1. Results

Pass rates per task by the four-of-five rule, against the thresholds of
`docs/design/03-readability-test.md` (Predict and Explain at least 90,
Complete at least 80, Write at least 70, on every gating model; Write also
needs no single lint rule over a quarter of the violations). The protocol
tally runs `renyi format` before the lint (decision M5); the strict tally
does not. Round 1 is in parentheses where the model ran in it, its
Explain on the subagent graders' scale after the re-grade of session 6
(round 1's `notes.md`, section 8):

| Label | Predict | Explain | Complete (protocol / strict) | Write (protocol / strict) |
|-------|---------|---------|----------|------|
| claude-sonnet-5-5 | 9/10 (90%; r1 90) | 30/30 (100%; r1 97) | 17/19 (89%; r1 79) / 16/19 (84%; r1 63) | 3/10 (30%; r1 40) / 1/10 (10%; r1 10) |
| gpt-5.5 | 10/10 (100%) | 30/30 (100%) | 14/19 (74%) / 13/19 (68%) | 1/10 (10%) / 1/10 (10%) |
| claude-haiku-4-5-20251001 | 5/10 (50%; r1 30) | 26/30 (87%; r1 90) | 8/19 (42%; r1 37) / 6/19 (32%; r1 32) | 0/10 (0%; r1 0) / 0/10 |
| gpt-5.4-mini | 8/10 (80%; r1 80) | 29/30 (97%; r1 97) | 7/19 (37%; r1 32) / 6/19 (32%; r1 32) | 0/10 (0%; r1 0) / 0/10 |

Sonnet now passes Predict, Explain and Complete and misses Write by four
items; gpt-5.5 passes Predict and Explain, misses Complete by one item and
Write by six. The grammar is not frozen. The tally is in
`outputs/<label>/scores.json`, the strict tally in `scores-strict.json`,
the grades and the graders' reasons in `grades.json`, the adjudications in
`judgement.json`.

Against the five-point rule of the protocol (a grammar change that lowers
a passing rate by more than five points is reverted or explained): no rate
of the three models of round 1 fell by more than five points except
Sonnet's Write, 40 to 30, and Haiku's Explain, 90 to 87 on the subagent
scale after decision U5 (below). Sonnet's is one item, `fizz_words` (5 of 5 samples
in round 1, 3 of 5 now): two samples call `console.print` without
`import std.console`, a slip the checker catches (`unknown-name`), not a
reading of any changed rule. Sonnet cannot be sampled at temperature 0,
and with five samples and a four-of-five rule an item whose samples pass
six times in ten passes about a third of the time, so single-item moves
between rounds are noise for this model. Haiku's Explain loses `todo_cli`
(every sample says every command saves the list; only `add` and `done`
do) and fails `shipping_rules`, `invoice` and `inventory_db` in both
rounds once decision U5 is applied: wrong statements about the programs,
not readings of a changed rule, from the floor model, which does not
gate; by decision U6 the five-point rule names the gating models and the
floor model's moves are reported here. The other moves are up:
Sonnet's Complete from 15 to 17 items (`deadlines`, `invoice_report` and
`shapes` now pass, `statistics` now fails on the reserved word `count` in
two samples), Haiku's Predict from 3 to 5 items.

Nothing changed in the grammar between the rounds; the cheat sheet changes
were clarifications, and the measurable one is the `traffic_light` item
of Predict, where round 1's Sonnet samples said that `to_text` of a
variant "presumably" prints its name: the remark is gone, the samples are
right, and the item still fails (section 2).

## 2. Predict

- Sonnet fails only `traffic_light` again, 0 of 5: every sample gives the
  five right lines after six lines of reasoning ("1. TimerExpired: Red
  goes to Green. ..."), against the prompt's "no explanation". The
  comparison is exact, so the item fails on the prose, not on the
  reading. gpt-5.5 answers every item with the output only.
- gpt-5.4-mini fails `invoice_report` (every sample pads the customer
  column to 13 characters where `pad_right(12)` gives 12) and
  `temperature_table` (two of five drop the literal spaces between the
  columns, as in round 1); it no longer fails `statistics`.
- Haiku fails five of ten, two fewer than in round 1: a mis-added subtotal
  (`invoice_report`: 20.00 for 21.00), the smaller shape named largest
  (`shapes`), a mis-added mean (`statistics`, two of five: 5.125), column
  widths (`temperature_table`), an extra `Red` before the emergency
  (`traffic_light`). Arithmetic and attention, as before.

## 3. Explain

Each explanation was graded by two Claude Code subagents against the
author's description of the program (`reference/<name>.explain.txt`) with
the rubric of `run.py` (5 when nothing is wrong and the purpose, the
effects and the main behaviours are stated; 4 when one of those is
missing; 3 vague or several missing; 2 one wrong statement; 1 a different
program), one batch of fifty items per subagent, with the instruction to
grade every item on its own and to give a one-sentence reason. The first
grader's grade stands when the second is within one point; eleven samples
where they disagreed by more (4 Sonnet, 7 Haiku; none for gpt-5.5 and
gpt-5.4-mini) were adjudicated by hand with a reason each in
`judgement.json`. Round 1 adjudicated 49. Five more verdicts came in
session 6 by decision U5 (a hand verdict extends to every sample that
repeats the sentence it called wrong): Haiku's `inventory_db.0` to `.4`,
"items that have fallen below their reorder threshold" for at or below,
the sentence round 1's `inventory_db.0` verdict calls wrong; both
subagents had graded the five samples 5.

Grades over the 150 samples per model: Sonnet 142 fives, 7 fours, 1 two;
gpt-5.5 143 fives, 7 fours; Haiku 113 fives, 25 fours, 12 twos;
gpt-5.4-mini 146 fives, 2 fours, 2 twos. The two subagents agree exactly
on 128 to 142 of 150 samples per model and within one point on all but
the eleven adjudicated; where they differ by one, Opus is the stricter
(20 samples each for gpt-5.5 and gpt-5.4-mini, 4 for each Claude model;
section 5 compares both with round 1's API graders).

Failing items and the adjudications:

- Haiku fails `shipping_rules` on the same sentence as in round 1, "free
  domestic shipping on orders above 50.00" for `at least 50.00`, in all
  five samples (Sonnet 2, Opus 4; adjudicated 2 by the boundary rule of
  round 1); `todo_cli`, all five samples saying every command reads the
  list, saves it back and displays it, where `show` only prints and an
  unknown command prints the usage line (both graders 2); and `invoice`,
  two samples saying quantities are validated as "non-negative" where a
  line's quantity is at least 1 (Sonnet 4, Opus 2; adjudicated 2: one
  wrong statement); and `inventory_db` by decision U5, all five samples
  (both subagents 5). Haiku's fours mostly leave the needs unnamed, as in round 1.
- gpt-5.4-mini fails `todo_cli` on two of five samples: "no safeguards for
  invalid task numbers beyond normal failure" (a bad number prints the
  usage line) and "fails if it cannot read the task file" (a missing file
  counts as an empty list).
- Sonnet's one two is `retry.4`: "the last error message, which includes
  the attempt number", where `GaveUp` carries the last error text only
  (the `attempt <n>:` prefix is in the per-attempt records); round 1 graded
  the same claim in `retry.3` the same way. Three `file_tree` samples
  say the program "only reads the filesystem and writes to the console"
  while describing the command-line argument it reads; Opus counted the
  "only" as a wrong statement (2), round 1's graders had read the same
  sentence in Sonnet's round-1 samples as the environment need missing
  (4), and the adjudication keeps round 1's reading so the rounds
  compare. The item passes either way with 4, fails with 2; the rubric
  does not say which reading is meant (section 6).
- gpt-5.5's five `currency_tool` explanations were first graded 2 by both
  subagents for naming `api.frankfurter.dev`: the reference still said
  `api.frankfurter.app`, the host the program used in round 1 (session 6
  moved it; the reference was not updated). The reference was corrected
  and the twenty `currency_tool` samples re-graded (section 5); all
  twenty are fives.

## 4. Complete and Write

Where each sample was decided, per model (Complete 95 samples, Write 50):

| Label | Complete: lint / checker / VM pass / VM fail | Write: lint / checker / VM pass / VM fail |
|-------|------|------|
| claude-sonnet-5-5 | 4 / 4 / 87 / 0 | 3 / 22 / 25 / 0 |
| gpt-5.5 | 6 / 15 / 74 / 0 | 11 / 23 / 15 / 1 |
| claude-haiku-4-5-20251001 | 24 / 31 / 40 / 0 | 24 / 26 / 0 / 0 |
| gpt-5.4-mini | 25 / 25 / 41 / 4 | 23 / 27 / 0 / 0 |

Every program that reaches the VM passes its own examples and tests but
five: one `request_window` of gpt-5.5 whose `count_rejected_requests`
example expects 1 and gets 0, and four `invoice_report` bodies of
gpt-5.4-mini that pad `statement_line` to the wrong widths (as in round
1). The checker and the lint decide the rest, and the same few rules
decide most of it:

- **A named single argument** (decision M4, `check:argument-name`, "a call
  with one argument does not name it"): 43 error lines in Sonnet's samples
  (`letter_for(score: score)`, `grade_all(scores: scores)`), 50 in
  gpt-5.5's (`word_choice(number: number)`), 90 in Haiku's, 68 in
  gpt-5.4-mini's; the largest checker rule for every model, as in round
  1, and a rule of 11 of gpt-5.5's 50 Write samples. It alone costs
  gpt-5.5 `fizz_words` (four of five samples fail on nothing else) and
  shares `initials` with the field form below; for Sonnet it is one of
  several rules on four Write items.
- **`rounded()` without `places`, and `to_decimal()` on a Decimal**
  (`check:argument-count`, `check:unknown-method`): the models round to a
  whole number with `(discounted * 100.00).rounded()` and convert back
  with `cents.to_decimal() / 100.00`; `rounded` takes `places` and
  `to_decimal` exists on `Integer` and `Float` only. It costs gpt-5.5 the
  Complete items `invoice` and `shapes` (0 of 5 each) and the Write item
  `compound_interest` (0 of 5), Sonnet two `invoice` and two
  `compound_interest` samples, Haiku `invoice` (0 of 5). Forgiving it would
  lift gpt-5.5's Complete to 16 of 19 (84 percent, over the threshold) and
  Sonnet's to 18 of 19.
- **Zero-argument methods written as fields** (`check:unknown-field`:
  `text.length`, `items.is_empty`, `items.last`, `password.characters`):
  21 error lines for gpt-5.5 (`initials`, `password_strength`,
  `roman_numerals`, `dependency_order`, `word_count`), 100 for Haiku
  (`merge_sorted` alone 62), 20 for gpt-5.4-mini. The cheat sheet lists
  the methods as bare names (`Text: length is_empty trim ...`) and shows
  the call form once (`text.trim()`); the models read the list as fields.
- **Reserved words as names** (`count` in `statistics` and `deadlines`,
  `one` in `roman_numerals`, `sorted`): Sonnet's `statistics` (2 of 5),
  gpt-5.5's `statistics` (4 of 5) and `roman_numerals` (2), Haiku's and
  gpt-5.4-mini's `deadlines` and `statistics` (every sample). Forgiving
  the reserved words alone would lift Sonnet's Complete to 18 of 19 and
  gpt-5.5's to 15 of 19.
- **`otherwise` binds loosest**: three of Sonnet's `roman_numerals`
  programs test with `check to_roman(2024) otherwise "" is "MMXXIV"`,
  which parses as `to_roman(2024) otherwise ("" is "MMXXIV")` (nine
  `type-mismatch` pairs: "the condition is `Text`, but `Boolean` is
  needed" and "the default is `Boolean`, but `Text` is needed"). The
  sketch gives `otherwise` the lowest precedence; the cheat sheet does not
  say so.
- The rest is scattered: a fallible call without `otherwise` on the
  program's own error types (Sonnet `letter_grades`, `title_length_tool`;
  Haiku and gpt-5.4-mini `semver`), `otherwise` after `Path(...)` and
  `Url(...)` which cannot fail (`superfluous-otherwise`, Sonnet
  `low_stock_report`, `title_length_tool`), a namespace imported twice
  (Sonnet `low_stock_report`, three samples), `console` used without its
  import (Sonnet `fizz_words`, two samples), `import std.http as web`
  (gpt-5.5 `title_length_tool`, parse errors), `Text` keys for a map
  declared over the `Grade` variant (gpt-5.4-mini `letter_grades`, 40
  lines), unused pattern bindings (Haiku `shapes`, every sample), lines
  the formatter cannot wrap (`line-width`), block imbalance.

Why items fail, per gating model:

- Sonnet, Complete (2 items): `invoice` 3 of 5 (`rounded()` without
  `places`) and `statistics` 3 of 5 (`count` as a name). Write (7 items):
  `compound_interest` 2 of 5 (`rounded()`, a 149-column line, a block
  imbalance), `fizz_words` 3 of 5 (`console` not imported),
  `letter_grades` 1 of 5 (named single arguments, `otherwise` missing on
  its own `GradingError`), `low_stock_report` 0 of 5 (a namespace imported
  twice, `otherwise` after `Path(...)`, an argument of the wrong type),
  `password_strength` 3 of 5 (a long line, a named single argument),
  `roman_numerals` 1 of 5 (the `otherwise` precedence, `otherwise`
  missing), `title_length_tool` 0 of 5 (`otherwise` missing on its own
  `PageError`, `otherwise` after `Url(...)`, a `Response` passed where
  `Text` is needed, a long line). `initials`, `merge_sorted` and
  `request_window` pass.
- gpt-5.5, Complete (5 items): `invoice` and `shapes` 0 of 5 (`rounded()`,
  `to_decimal()` on a Decimal), `statistics` 1 of 5 (`count`),
  `dependency_order` 3 of 5 (`is_empty` as a field, a condition without
  `then`), `word_count` 3 of 5 (`ignore` of a pure call, decision R6;
  `is_empty` as a field). Write (9 items): `compound_interest` 0 of 5
  (`rounded()`), `fizz_words` 1 of 5 (named single arguments), `initials`
  2 of 5 (`is_empty` as a field, named single arguments), `letter_grades`
  2 of 5 (named single arguments, a long line), `low_stock_report` 0 of 5
  (arguments of the wrong type, long lines), `password_strength` 3 of 5
  (`length` and `characters` as fields), `request_window` 3 of 5 (the VM
  failure above, a block imbalance), `roman_numerals` 0 of 5 (`one` as a
  name, `is_empty` and `length` as fields, named single arguments),
  `title_length_tool` 0 of 5 (`import ... as`, long lines, an unused
  binding, a reserved word). `merge_sorted` passes.

The quarter rule for Write (no single lint rule over a quarter of the
violations): Sonnet's largest is `check:type-mismatch` at 23 percent of
39 (then `missing-otherwise` 21, `argument-name` and
`superfluous-otherwise` 13 each); gpt-5.5's is `check:argument-name` at
exactly a quarter (11 of 44, then `unknown-field` 18, `line-width` 16);
Haiku's `argument-name` at 20 percent of 106; gpt-5.4-mini's
`argument-name` at 18 percent of 98. In the strict tally `line-width` is
87 percent of Sonnet's violations and 48 percent of gpt-5.5's: the models
write lines the formatter wraps, which is what decision M5 expects.

## 5. Method, and what changed from round 1

Each of these is a deviation from round 1's method or a fix made during the
round; the README describes the harness as it now is.

1. **Where the samples came from.** Predict, Explain and Complete for all
   four models, Write for Sonnet, Haiku and gpt-5.4-mini, and the Write
   samples of `compound_interest` for gpt-5.5 came through the vendor
   APIs as in round 1 (`run.py run`, 300 of gpt-5.5's 345). The owner then
   ruled that the pay-per-token API keys are not spent by default and
   that subscription quota is used instead; the remaining 45 Write samples
   of gpt-5.5 (nine tasks, five each) were taken through the Codex CLI on
   the ChatGPT subscription (`codex exec`, Codex 0.153.3, model `gpt-5.5`,
   reasoning effort `medium`, read-only sandbox, an empty working
   directory, the system text and the task prompt in one user message, 10
   to 36 seconds each) and fed in with `run --provider file`. Each of the
   nine output records carries a `source` field with those settings and
   the Codex session id of every sample, and `temperature: null` (the CLI
   exposes none). The samples are the model's final message, one code
   fence each, as the API samples are; what differs is the harness around
   the model (Codex's own system prompt precedes ours) and the reasoning
   effort, which the API calls did not set. Of the 45, 15 pass the VM
   (section 4); the five API-sourced `compound_interest` samples all fail
   on `rounded()`, so the two sources are not distinguishable in the
   tally.
2. **The Explain graders.** Round 1 graded through the APIs (Sonnet 5.5
   first, gpt-5.4-mini second, one call per sample). This round grades
   with Claude Code subagents on the subscription: Sonnet 5.5 first and
   Opus 5.5 second, each given a batch of fifty samples with the rubric
   verbatim, writing a grade and a one-sentence reason per item; the
   answers were merged into `grades.json` under the buckets
   `agent:claude-sonnet-5-5` and `agent:claude-opus-5-5` with the
   harness's own sample digests, so `score` ran without any API call. 24
   batches and two re-grades (item 4) cost about 3.2 million tokens of
   subscription quota. Three things differ from round 1: the grader sees
   fifty items at once (and is told to grade each on its own), the second
   grader is Opus rather than gpt-5.4-mini, and every grade carries a
   reason.
3. **The subagent graders are more lenient than the API graders.** Before
   the owner's ruling the API graders had graded 100 of Haiku's and 104
   of gpt-5.4-mini's samples; those grades are kept in `grades.json`
   under their own buckets and compared with the subagent's on the same
   samples. The subagent first grader agrees with the API Sonnet grader
   on 22 of Haiku's 100 and 16 of gpt-5.4-mini's 104 samples, and is one
   point above it on 60 and 82; it never grades lower. The API grader
   gave mostly fours ("the needs are not named"), the subagent mostly
   fives. Where it matters, at the pass line of 4: the subagent passes 23
   of the 100 Haiku samples the API grader failed (`config`, `file_tree`,
   `http_service`, `inventory_db`, graded 2 or 3 by the API grader) and 7
   of gpt-5.4-mini's 104; nothing
   goes the other way. Under the API grader Haiku would fail about four
   more Explain items. Round 2's Explain rates are therefore not on the
   same scale as round 1's for the floor models; for the gating models
   no API grades exist to compare, and their failing samples are wrong
   statements, which both kinds of grader mark 2. The owner kept the
   subagents and had round 1 re-graded with them (decision U4; round 1's
   `notes.md`, section 8): on the one scale, after decision U5, round 1's Explain is 97, 90
   and 97 (Sonnet, Haiku, gpt-5.4-mini) and this round's Haiku is 87,
   and the parentheses of section 1 use it.
4. **A stale reference** (fix 1 of this round). `reference/currency_tool.explain.txt`
   named `api.frankfurter.app`, the host of round 1's corpus; session 6
   moved the program to `api.frankfurter.dev` and gpt-5.5, the one model
   that named the host, was graded 2 five times for being right. The
   reference now names the new host, the `v1` endpoint and the `replays`
   test; the twenty `currency_tool` samples were re-graded by both
   subagents (batch 4 of each label), the stale grades were dropped, and
   the Explain rates above are after the re-grade (gpt-5.5 from 29 to 30
   items; the other three models were unaffected). The other corpus
   changes since round 1 (the `replays` tests, the M1 form of
   `totals_by_region`, "started kilograms" in `weight_steps`) are
   consistent with their references.
5. **Adjudication** was done by Claude in the session, with the owner
   spot-checking, as the owner chose before the round; eleven samples,
   each with the sentence that decided it.

## 6. Open questions for the owner

Raised as decision questions after this round, in a batch of four, with
the numbers above, and answered in session 6 as decisions U1 to U4 (M4
stands and the cheat sheet says it in a sentence; `rounded(places: 2)`
in the cheat sheet; every method listed with its parentheses; the
subagent graders from here on, round 1 re-graded):

- A named single argument (decision M4) is again the largest checker rule
  for every model, both gating models included; it is a rule of 11 of
  gpt-5.5's 50 Write samples and costs gpt-5.5 `fizz_words` outright.
  Round 1 kept M4 against the measured cost.
- `rounded()` without `places` and `to_decimal()` on a Decimal cost both
  gating models the Complete items `invoice` and `shapes` and the Write
  item `compound_interest`; with it forgiven gpt-5.5 passes Complete.
  Either the library gives `places` a default and `to_decimal` an identity
  on Decimal, or the cheat sheet shows `rounded(places: 2)`.
- Zero-argument methods are written as fields (`.length`, `.is_empty`,
  `.last`) because the cheat sheet lists them as bare names: either the
  list shows `length()`, or the grammar accepts the field form for a
  method without parameters.
- The Explain grader: the subagent graders are a point more lenient than
  the API graders and flip a fifth of the floor models' samples from fail
  to pass; the protocol needs one grader kind across rounds, and
  re-grading round 1 with the subagents costs quota, grading with the
  APIs costs the keys.

Observations that are not asked as questions: `otherwise` binds loosest
and the cheat sheet could say so (Sonnet's `roman_numerals` tests, 23
tokens of budget left); Sonnet's `traffic_light` samples are right but
reason first, against the prompt (the comparison stays exact); Haiku as
the floor model fails Predict at 50 and Write at 0 as it did at 30 and 0
(settled by R1: the floor model does not gate); the reserved word
`count` costs both gating models `statistics` (settled by R2); the
rubric does not say whether "only X and Y" with Z also needed is a wrong
statement or a missing need (this round followed round 1: missing).
