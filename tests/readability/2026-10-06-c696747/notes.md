# Live round 4, 2026-10-06

The measurement of decisions U1 to U3 that round 3 was meant to be, on
the grammar at revision `c696747`: the cheat sheet after decision U8 (the
library list with every method's parameter names, the sentence "One
parameter is positional, more are named" with three calls, 2998 of 3000
tokens), the corpus unchanged since round 2. Sonnet 5.5 only (the
gating model of decision R1; gpt-5.5, the other, has no channel until the
Codex quota returns or the owner authorizes the key, round 3's notes,
section 5), five samples per prompt, 69 prompts (10 Predict, 30 Explain,
19 Complete, 10 Write), 345 samples, through the Claude Code CLI 2.1.289
on its subscription login (`run --provider claude`, decision U7) with the
channel corrected after round 3: one fresh session per sample in an empty
directory (`--work-dir`, so no `CLAUDE.md` is loaded) with
`--strict-mcp-config` and no servers (so the account's claude.ai
connectors stay out), the cheat sheet as the whole system prompt, no
tools, the model's default temperature, adaptive thinking on because it
cannot be switched off (its tokens recorded per session). A probe
through the harness's own command before the run showed the context to
be the system text, the task and the CLI's fixed frame (its identity
line, the environment block, the date and a one-line note of the
account's email address), with no tools, no instruction file and no MCP
instructions. The 345 samples took 2350 seconds of session time, four
prompts at a time (medians: Predict 6 s, Explain 5, Complete 7, Write
13); no call failed.

Explain was graded by Claude Code subagents (Sonnet 5.5 first, Opus 5.5
second; decision U4), Complete and Write by the VM (decision L2).

## 1. Results

Pass rates per task by the four-of-five rule, against the thresholds of
`docs/design/03-readability-test.md` (Predict and Explain at least 90,
Complete at least 80, Write at least 70, on every gating model; Write
also needs no single lint rule over a quarter of the violations). The
protocol tally runs `renyi format` before the lint (decision M5); the
strict tally does not. Round 2 (the last measurement before the defect
round) and round 3 (the defect round) in parentheses:

| Label | Predict | Explain | Complete (protocol / strict) | Write (protocol / strict) |
|-------|---------|---------|----------|------|
| claude-sonnet-5-5 | 9/10 (90%; r2 90, r3 90) | 30/30 (100%; r2 100, r3 100) | 15/19 (79%; r2 89, r3 68) / 13/19 (68%; r2 84, r3 63) | 4/10 (40%; r2 30, r3 40) / 1/10 (10%; r2 10, r3 20) |

Sonnet passes Predict and Explain, misses Complete by one item and
Write by three; the quarter rule for Write holds for the first time
(section 4). The grammar is not frozen; gpt-5.5 has not run on this
sheet. The tally is in `outputs/claude-sonnet-5-5/scores.json`, the
strict tally in `scores-strict.json`, the grades and the graders' reasons
in `grades.json`, the adjudications in `judgement.json`.

What the changes did, measured against round 2 (round 3 measured the
defect, not the changes):

- **A named single argument** (decision M4; U1's sentence "One argument
  is never named"): 43 checker lines in round 2, the largest rule for
  every model; 0 in this round. Nothing in Complete or Write names a
  single argument any more, and `argument-count` and `unknown-field`
  (the other two rules U2 and U3 addressed: `rounded()` without
  `places`, methods written as fields) are at 0 as well.
- **`rounded` and `split`** (decisions U2, U8): every sample of `invoice`,
  `shapes` and `compound_interest` writes `rounded(2)` and every sample
  that splits writes `split(" ")` or `split(",")`, as the sheet's
  sentence shows; the four items round 3 lost to `rounded(places: 2)`
  and `split()` are back (`invoice` 5 of 5, `shapes` 5, `word_count` 5;
  `compound_interest` 3 of 5 on long lines only).
- **Complete 89 to 79** is still a fall of ten points, past the
  five-point rule of the protocol (decision U6). The items: `invoice`
  (3 of 5 in round 2, 5 now) and `statistics` (3, now 5: no sample
  names `count`) came back; `config` (5, now 3), `todo_cli` (5, now 3),
  `traffic_light` (5, now 2) and `deadlines` (4, now 3) fell. The four
  fall on slips that no changed sentence touches: `Port(8080)
  otherwise crash with "unreachable: default port is valid"`, an
  `otherwise` after a refined construction from a literal the checker
  proves valid (`superfluous-otherwise`, two samples with identical
  bodies); `Done(position: number)` without `otherwise` after the guard
  `if number is less than 1 then return Help end`, where the refined
  field can still fail as far as the checker sees (`missing-otherwise`,
  two samples; the corpus writes `Done(position: position) otherwise
  Help`); one `end` too many after a `match` nested in the last arm of
  another (three samples, the lint's block heuristic; round 3's five
  samples nest the same way with the right count); and `count` as a
  name (two samples, as in rounds 2 and 3). The two refined-construction
  slips are the one pattern: the sheet says "Refined construction can
  fail: `Email(input) otherwise fail with BadInput`" and does not say
  that a literal the checker can evaluate takes no `otherwise` while a
  variable needs one even after a check (section 6). The rest is the
  sampling noise of the default temperature that round 2's notes
  describe: with five samples and a four-of-five rule an item that
  passes six times in ten passes about a third of the time.
- **Write 30 to 40**: `request_window` and `merge_sorted` back to 5 of
  5, `initials` and `password_strength` at 4; the failing items fail on
  the same things as in round 2 (`console` used without its import in
  `fizz_words`, `otherwise` missing on the program's own error types,
  type mismatches, long lines the formatter cannot wrap).

## 2. Predict

- Sonnet fails `traffic_light`, 1 of 5: four samples trace first
  ("Trace: start Red. 1. TimerExpired: Red → Green ...") and then give
  the five right lines, against the prompt's "no explanation", as in
  rounds 1 and 2; round 3's five samples gave the lines alone and
  passed. No session of the item in either round has thinking tokens,
  so whether the model traces in the answer is not a matter of adaptive
  thinking here; the item fails on the prose, not the reading, and the
  comparison stays exact. `permissions`, which round 3 lost on its
  three sessions without thinking, passes 5 of 5 here, every session
  with thinking (123 to 162 tokens).
- `invoice_report` 5 of 5 (round 2: 4).

## 3. Explain

Graded as in round 3 (its notes, section 3): two Claude Code subagents
per batch of fifty samples with the rubric of `run.py` verbatim, a
one-sentence reason per grade, `grade_batches.py` writing the batches
and `merge_agent_grades.py` putting the grades into `grades.json`; the
first grader's grade stands when the second is within one point.

Sonnet's 150 samples: 147 fives, 3 fours, no two; every sample passes
and every item with it (30 of 30, as in rounds 2 and 3). The two
subagents agree exactly on 142 of the 148 samples they decided and
within one point on the other six (Opus the stricter each time, as in
every round); two samples disagreed by more than one and were
adjudicated by hand (`judgement.json`):

- `expression_tree.0` and `.4`: "prints it in infix form with
  parentheses around every operation". Opus 2 (the description gives a
  negation a leading minus sign and parentheses only to additions and
  multiplications), Sonnet 5; graded 5: the sentence is the program's
  own `purpose:` line for `render` ("The expression in infix notation
  with parentheses around every operation"), which the description
  refines rather than contradicts, and round 3's `expression_tree.1` to
  `.4` carry the same phrase with both graders at 5, as do the "fully
  parenthesized" samples of every round. The reference could say
  "every addition and multiplication" is a refinement of the purpose
  line, or the purpose line could be made exact; neither is a reading
  error of the model.
- The three fours are `semver.1`, `.2` and `.4` (both graders 4): the
  explanations never say that the module is pure and needs nothing,
  which the rubric counts as the effects missing.

The search of decision U5 over the 35 hand verdicts of 2 in the earlier
runs: one `shipping_rules` sample contains "at or above 50.00", the
phrase a round 1 verdict quotes as the correct part of its sample; no
wrong sentence recurs, and no verdict was added by U5.

## 4. Complete and Write

Where each sample was decided (Complete 95 samples, Write 50):

| Task | lint | checker | VM pass | VM fail |
|------|------|---------|---------|---------|
| Complete | 6 | 4 | 85 | 0 |
| Write | 6 | 14 | 30 | 0 |

Every program that reaches the VM passes its own examples and tests.
Checker error lines over both tasks: `type-mismatch` 28,
`missing-otherwise` 14, `unknown-function` 9, `unknown-name` 7,
`superfluous-otherwise` 4, `duplicate-name` 2; `argument-name`,
`argument-count` and `unknown-field` 0 (round 2: 43, and the two rules
behind `rounded()` and the field form). Reserved words: `count` in two
`deadlines` samples and one `invoice_report` sample, `remainder` in one
`title_length_tool` sample.

Why items fail:

- Complete (4 items): `config` 3 of 5, `todo_cli` 3 of 5,
  `traffic_light` 2 of 5 and `deadlines` 3 of 5, each as section 1
  says. `invoice_report` passes at 4 of 5 (`count` in one sample).
  `markdown_table` passes 5 of 5 formatted and 2 of 5 strict,
  `shipping_rules` 5 and 0: long lines the formatter wraps.
- Write (6 items): `compound_interest` 3 of 5 (two samples with lines
  of 102 to 105 columns the formatter leaves as they are), `fizz_words`
  2 of 5 (`console` used without `import std.console` in three samples,
  the slip of round 2), `letter_grades` 3 of 5 (a type mismatch, an
  `otherwise` missing on its own `GradingError`, a call to a function
  that does not exist), `low_stock_report` 3 of 5 (a namespace imported
  twice, a long line), `roman_numerals` 1 of 5 (`otherwise` missing on
  its own error type in two samples, a type mismatch, an unknown name),
  `title_length_tool` 0 of 5 (type mismatches in three samples,
  `otherwise` after `Url(...)` in two, long lines, `otherwise` missing
  on its own `PageError`, a namespace imported twice, `remainder` as a
  name). `initials` and `password_strength` pass at 4 of 5 (a call to
  an undefined function; a long line), `merge_sorted` and
  `request_window` at 5 of 5.

The quarter rule for Write holds: the largest rule is `line-width` at 23
percent of 26 violations (6; then `type-mismatch` 5, `unknown-name` and
`missing-otherwise` 4 each). In the strict tally `line-width` is 84
percent (37 of 44), as in every round: the models write lines the
formatter wraps, which is what decision M5 expects.

Thinking, from the session records: Predict 40 of 50 sessions, median
318 thinking tokens, at most 601; Explain 11 of 150, median 283, at
most 433; Complete 82 of 95, median 370, at most 1245; Write 48 of 50,
median 843, at most 1432.

## 5. Method, and what changed from round 3

1. **The channel is corrected** (round 3's notes, section 5, item 1):
   the CLI runs in an empty directory and with no MCP server, which
   keeps the project's `CLAUDE.md` and the account's claude.ai
   connectors out of the context; the probe above is the evidence. The
   rest of the channel is round 3's: the login-only configuration
   directory, the cheat sheet as the system prompt, no tools, thinking
   recorded per session. `--exclude-dynamic-system-prompt-sections` is
   still passed, so that the command is round 3's; the CLI reference
   says it is ignored when a system prompt file is given.
2. **Grading and adjudication** as in round 3: three batches of fifty
   per grader, `grade_batches.py` and `merge_agent_grades.py`, the
   first grader's grade standing when the second is within one point,
   a hand verdict where they differ by more, the search of decision U5
   over the hand verdicts of every earlier run.
3. **One model.** gpt-5.5 is absent (no channel); the round therefore
   measures U1 to U3 on one of the two gating models, and the freeze
   needs the other.

## 6. Observations and open questions

Put to the owner after this round, with the recommendation first:

- **Refined construction and `otherwise`** cost `config` and `todo_cli`
  (two samples each, two Complete items) on one gap: the sheet says
  "Refined construction can fail: `Email(input) otherwise fail with
  BadInput`" and not that a literal the checker can evaluate takes no
  `otherwise` while a variable needs one even after a check. A sentence
  saying so costs about 30 tokens against the 2 left in the budget, so
  it needs a trim of the same size; the alternative is to leave the gap
  and count the two items as the noise they also resemble (round 3's
  samples of both items passed 5 of 5 on the same sheet text).
- **The five-point rule** (decision U6) on Complete, 89 in round 2 to 79
  here: the targeted errors are gone (section 1) and the four items
  that fell did so on slips no changed sentence touches; the notes
  explain the fall, as the rule asks, and no decision entry proposes
  the loss as worth anything because no change caused it. Whether that
  reading of the rule stands, or a second run on the same sheet is
  wanted to separate noise from signal, is the owner's call.
- **gpt-5.5** has not run on this sheet: the freeze needs both gating
  models, and the channel is the Codex quota (the CLI named Nov 4th,
  2026) or the OpenAI key with the owner's authorization for the round
  (345 samples).

Observations not asked: Predict's `traffic_light` traces before the
answer in four of five samples without thinking (rounds 1, 2 and 4) and
gives the lines alone in round 3, so the item is a coin toss on this
channel; `console` used without `import std.console` costs `fizz_words`
again (three samples, as in round 2); `count` as a name persists at
three samples (`deadlines`, `invoice_report`); the lint's block
heuristic caught one `end` too many in three `traffic_light` samples;
Explain stays at 100 with thinking in only 11 of 150 sessions.

Answered in session 6: the sentence is on the sheet (decision U9, paid
for by the phrase-token sentence, the `# flatten` comment and "`renyi
run` enforces it", 2991 tokens; not yet measured), the notes' explanation stands for the
five-point rule without a decision entry, and gpt-5.5 does not run on
this sheet for now (the rounds continue on Sonnet; the freeze still
needs the second gating model).
