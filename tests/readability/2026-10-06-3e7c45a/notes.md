# Live round 3, 2026-10-06

The round that was to measure decisions U1 to U3 (the cheat sheet after
round 2: "One argument is never named", every library method listed with
its parentheses, `rounded(places: 2)`), on the grammar at revision
`3e7c45a`, and that found the rendering of U2 and U3 wrong instead
(section 6; decision U8): a defect round, whose rates measure the
defect; round 4 on the corrected sheet is the measurement. The corpus is
unchanged since round 2; the cheat sheet differs from round 2's in those
three places and in the sentences trimmed to pay for them (round 2's
`notes.md`, section 6, and decisions U1 to U3). The two gating models
only (decisions R1 and R8; the owner's choice for this round), five
samples per prompt, 69 prompts (10 Predict, 30 Explain, 19 Complete, 10
Write), 345 samples per model, collected through the subscription
channels of decision U7:

| Label | Channel | Settings |
|-------|---------|----------|
| `claude-sonnet-5-5` | `run --provider claude`: the Claude Code CLI 2.1.289 on its subscription login, one fresh session per sample | the cheat sheet as the whole system prompt, no tools, the model's default temperature; adaptive thinking cannot be switched off and its tokens are recorded per session |
| `gpt-5.5` | `run --provider codex`: the Codex CLI 0.153.3 on the ChatGPT login, one fresh session per sample | the cheat sheet and the task in one user message, read-only sandbox in an empty directory, reasoning effort medium, as for round 2's Write samples |

Sonnet's 345 samples are complete. gpt-5.5 stopped at 72 (Predict
complete, 22 Explain samples) when the Codex CLI reported the ChatGPT
login's usage limit (section 5, item 6); the owner chose to score Sonnet
now and to finish gpt-5.5 when it has a channel. Explain was graded by
Claude Code subagents (Sonnet 5.5 first, Opus 5.5 second; decision U4),
Complete and Write by the VM (decision L2).

## 1. Results

Pass rates per task by the four-of-five rule, against the thresholds of
`docs/design/03-readability-test.md` (Predict and Explain at least 90,
Complete at least 80, Write at least 70, on every gating model; Write
also needs no single lint rule over a quarter of the violations). The
protocol tally runs `renyi format` before the lint (decision M5); the
strict tally does not. Round 2 in parentheses:

| Label | Predict | Explain | Complete (protocol / strict) | Write (protocol / strict) |
|-------|---------|---------|----------|------|
| claude-sonnet-5-5 | 9/10 (90%; r2 90) | 30/30 (100%; r2 100) | 13/19 (68%; r2 89) / 12/19 (63%; r2 84) | 4/10 (40%; r2 30) / 2/10 (20%; r2 10) |
| gpt-5.5 (72 of 345 samples) | 10/10 (100%; r2 100) | 22 samples, ungraded | no samples | no samples |

Sonnet passes Predict and Explain, misses Complete by three items and
Write by three. The tally is in `outputs/<label>/scores.json`, the strict
tally in `scores-strict.json`, the grades and the graders' reasons in
`grades.json`, the adjudications in `judgement.json`.

Against the five-point rule of the protocol (decision U6: a grammar
change that lowers a gating model's passing rate by more than five
points is reverted or explained): Sonnet's Complete fell 21 points, 17
items to 13. `shapes` (5 of 5 in round 2, 0 now) and `word_count` (4 to
0) are lost to the cheat sheet's call forms (section 4 and section 6),
`deadlines` (4 to 3) and `invoice_report` (4 to 0) to the reserved word
`count`; `invoice` (3 to 0) and `statistics` (3 to 1) failed in both
rounds. Nothing in the grammar changed; the first two are the defect
this round is recorded for, the other two a reserved word the owner
keeps (decision R2, reaffirmed after this round). Predict and Explain
held at the item level (one Predict item lost, one gained, section 2),
Write rose by one item on moves of one or two samples (noise at the
default temperature, as round 2 said).

## 2. Predict

- Sonnet fails `permissions`, 2 of 5: three samples print "the editor
  may publish" where the program prints "missing: posts:publish"
  (`allowed` asks whether the viewer and the editor together grant
  `posts:publish`, which neither does). The two right samples thought
  first (104 and 73 thinking tokens); the three wrong ones did not (0).
  `traffic_light`, 0 of 5 in rounds 1 and 2 for reasoning before the
  answer, passes 5 of 5 here: every sample is the five lines and nothing
  else. `invoice_report` 5 of 5 (round 2: 4).
- gpt-5.5 passes all ten, as in round 2.

## 3. Explain

Graded as in round 2: two Claude Code subagents per batch of fifty
samples with the rubric of `run.py` verbatim (5 when nothing is wrong
and the purpose, the effects and the main behaviours are stated; 4 when
one of those is missing; 3 vague or several missing; 2 one wrong
statement; 1 a different program), a one-sentence reason per grade,
`grade_batches.py` writing the batches and `merge_agent_grades.py`
putting the grades into `grades.json`. The first grader's grade stands
when the second is within one point.

Sonnet's 150 samples: 138 fives, 11 fours, 1 two. The two subagents
agree exactly on 144 of the 145 samples they decided and within one
point on the other; five samples disagreed by more than one and were
adjudicated by hand (`judgement.json`):

- `file_tree.1` to `.4`: "It only reads the file system and writes to
  the console" (and three phrasings of it), the environment need unnamed
  while the command-line argument is described. Opus 2, Sonnet 5;
  graded 4, as rounds 1 and 2 graded the same phrasing in Sonnet's
  `file_tree` samples (the need missing), so the rounds compare. The
  item passes either way with 4 and would fail with 2; the rubric still
  does not say which reading is meant.
- `config.0`: "the fallback loader, which returns defaults (port 8080,
  4 workers, normal logging) or a configuration you supply". Opus 2
  (falling back to the defaults is the deprecated function), Sonnet 5;
  graded 5: the loader returns whatever configuration the caller
  supplies, the defaults included, the next sentence names the older
  "load or default" function as the deprecated one, and round 1's
  `config.1` ("uses either your own configuration or the defaults when
  loading fails") was graded 5 by both graders on the same reading.
- The one two is `stacks.1`, both graders: "the checker is covered by
  examples and tests", where `balanced` has five `example:` lines and
  the program's two tests cover `pop` on an empty stack and the last
  item pushed.

The search of decision U5: the quoted sentences of the 35 hand verdicts
of 2 across the four runs were searched for in this round's samples.
Three `shipping_rules` samples contain "at or above 50.00", which the
verdict on round 1's gpt-5.4-mini `shipping_rules.1` quotes as the
correct part of that sample; no wrong sentence recurs, and no verdict
was added by U5.

gpt-5.5's 22 Explain samples (`active_users`, `config`, `currency_tool`,
`deadlines` and two of `dependency_order`) wait for grading with the
rest of its round.

## 4. Complete and Write

Where each sample was decided (Sonnet; Complete 95 samples, Write 50):

| Task | lint | checker | VM pass | VM fail |
|------|------|---------|---------|---------|
| Complete | 11 | 15 | 69 | 0 |
| Write | 4 | 16 | 28 | 2 |

- **The cheat sheet's call forms** (section 6). The library list of this
  revision shows `rounded(places: 2)`. Every sample of `invoice` and
  `shapes` (Complete) and `compound_interest` (Write) writes it that way
  and the checker rejects every one (`check:argument-name`, "a call with
  one argument does not name it": 27 error lines on those three items
  and `fizz_words`, 9 of them in `compound_interest`); in round 2 the
  same items wrote `rounded()` or `rounded(2)`, and `shapes` passed 5 of
  5. The list also shows `split()`, like every other method: three of
  five `word_count` samples call `line.split()` (`check:argument-count`,
  "`split` takes 1 argument (separator), found 0"), and the two that
  write `split(" ")` fail on other things (an unknown escape `\r`; a
  two-argument call with its arguments unnamed); round 2's five all
  wrote `split(" ")` and four passed. `log_parser`, `semver` and
  `initials` write `split` with its separator in every sample that uses
  it, as in round 2. Four items at 0 of 5 on the two forms.
- **Reserved words as names.** `count` in every sample of
  `invoice_report` (0 of 5; round 2 4 of 5), four of `statistics` (1 of
  5; round 2 3) and two of `deadlines` (3 of 5; round 2 4): eleven
  samples against two in round 2, with the reserved list unchanged
  (decision R2). `title_length_tool` also names `remainder` and `count`.
- The rest, by item. Write: `low_stock_report` 1 of 5 (`std.filesystem`
  imported twice in three samples, `otherwise` after `Path(...)` in
  two); `title_length_tool` 0 of 5 (`otherwise` missing on its own
  `PageError`, `otherwise` after `Url(...)`, a `Response` where `Text`
  is needed, an unknown type, long lines, the reserved words);
  `request_window` 3 of 5 (two VM failures: the `rejected_count`
  example expects 1 and gets 0, as one gpt-5.5 sample did in round 2);
  `roman_numerals` 3 of 5 (`+` on `Text`, a query without its
  terminal); `password_strength` 3 of 5 (a foreign keyword in two);
  `merge_sorted` 4 of 5 (a call to a function named `merging` that is
  not defined); `fizz_words` 4 of 5 (named single arguments in one).
  `initials` and `letter_grades` pass 5 of 5 (`letter_grades` 1 of 5 in
  round 2) in the protocol tally and 0 of 5 in the strict one: lines the
  formatter wraps. Complete: the thirteen other items pass 5 of 5, and
  `shipping_rules` passes 5 of 5 formatted and 0 of 5 strict (one long
  line in every sample, as in round 2).

The quarter rule for Write: Sonnet's largest rule is
`check:argument-name` at 24 percent of 29 violations (7; then
`superfluous-otherwise` 4, `duplicate-name` and `type-mismatch` 3 each),
under the line by one; in the strict tally `line-width` is 83 percent
(35 of 42).

Thinking, from the session records (section 5, item 2): none in the 150
Explain sessions; Predict 36 of 50 sessions, median 262 thinking tokens,
at most 609; Complete 68 of 95, median 334, at most 1102; Write 45 of
50, median 799, at most 1395.

## 5. Method, and what changed from round 2

1. **The channels.** Rounds 1 and 2 sampled through the vendor APIs (round
   2's gpt-5.5 Write samples excepted); this round samples through the two
   CLIs, which the harness now drives itself (`--provider claude`,
   `--provider codex`, `--parallel`). For the Claude CLI the harness
   points `CLAUDE_CONFIG_DIR` at a directory that holds only the login (a
   hard link to the credentials file and a `.claude.json` of
   `{"hasCompletedOnboarding": true}`), gives the cheat sheet with
   `--system-prompt-file`, excludes the dynamic system-prompt sections and
   removes every tool, and removes the pay-per-token key from the
   environment so that the CLI uses the login. A probe before the round
   (the model asked to list everything in its context) showed the default
   configuration adding the owner's hooks, instruction files, output
   style and MCP servers, and the login-only configuration adding one
   line, the account's email address. A second probe after the round, run
   as the harness ran (from the repository), showed two more things in
   the context: the project's `CLAUDE.md`, which the CLI loads from the
   working directory, and the account's claude.ai connectors (Gmail,
   Claude Docs: 38 tool definitions and their instructions, which
   `--tools ""` does not remove); the configuration directory's state
   file lists both connectors as connected through it, and the first
   probe, which asked about the context rather than the tools, did not
   report them. This round's samples therefore had the project notes and
   those tool definitions in their context, in a measure not recorded per
   sample; neither teaches the language, but the context was not what
   this item first claimed, and it is the second reason the round is a
   defect round. Round 4's channel runs the CLI in an empty directory
   (`--work-dir`) with `--strict-mcp-config` and no servers, which a
   probe through the harness's own command shows to leave the system
   text, the task and the CLI's fixed frame: its identity line, the
   environment block, the date and the email line. `--bare` would drop
   the frame but reads no login (it takes an API key only). The Codex
   side is round 2's `sample.sh` moved into the harness.
2. **Thinking.** The API calls of rounds 1 and 2 had no thinking. The
   Claude CLI's adaptive thinking cannot be switched off (`MAX_THINKING_TOKENS`,
   `CLAUDE_CODE_DISABLE_THINKING`, `--settings {"alwaysThinkingEnabled":
   false}`, `--effort low` were each tried on a prompt that triggers it;
   the thinking tokens stayed between 600 and 900), so Sonnet's samples
   here come after thinking and its thinking tokens are recorded with every
   session. gpt-5.5 through the API reasons by default at the same effort
   as the Codex setting, so its side is unchanged. Sonnet's rates are
   therefore not on round 2's footing where thinking helps (Predict and
   Complete most); the owner may authorize the keys for a round to measure
   the difference.
3. **Temperature.** Neither CLI takes one; both gating models already ran at
   their default temperature in round 2 (Sonnet rejects 0, gpt-5.5 rejects
   0), so nothing changed.
4. **Provenance.** Every record carries `source`: the tool and its version,
   the login kind, the settings, and one entry per session (the session id;
   for the Claude CLI also the models billed and the thinking and output
   tokens).
5. **Adjudication** by the rule of U5: after the graders, the sentences of
   every hand verdict of 2 in the earlier runs were searched for in this
   round's samples (section 3); none recurs.
6. **gpt-5.5 is partial.** 72 samples (Predict 50, Explain 22, in the
   harness's order) and then `codex exec` failed three times with "You've
   hit your usage limit. Upgrade to Plus to continue using Codex, or try
   again at Nov 4th, 2026 9:49 PM"; the run was stopped. The Predict
   samples are scored (every item passes, as in round 2); the Explain
   samples wait for grading; Complete and Write have none. The owner
   chose to score Sonnet now and to finish gpt-5.5 when it has a channel
   (the Codex quota, or the OpenAI key with the owner's authorization in
   the same request).
7. **Time.** The 335 samples of Sonnet's main run (ten `hello` Predict
   samples came from the channel probe before it) took 2333 seconds of
   session time, four prompts at a time: a median of 5 seconds for an
   Explain sample, 6 for Predict, 7 for Complete, 13 for Write; no call
   failed.

## 6. The defect, and the owner's decisions

The cheat sheet of this revision renders decisions U2 and U3 as
`rounded(places: 2)` in the library list and `()` after every method
name. The first names a single argument, which decision M4 forbids in a
call and the checker rejects; the second hides every parameter, so
`split()` reads as a call without a separator. Both were Claude's
rendering of what the decisions ask (the argument seen before it is
needed; every method a call), written without running the forms through
`renyi check`, and Sonnet copied both as shown (section 4). The three
questions put to the owner after the round, recommended option first:

- How the list shows a method's arguments: every method with its
  parameter names (`split(separator)`, `rounded(places)`, `replace(old,
  new)`), and one sentence after the list giving the call form of M4
  (`line.split(",")`, `price.rounded(2)`, `text.replace(old: "a", new:
  "b")`); the alternatives were the call forms in the list, where
  `rounded(2)` reads as a value, and the M4 sentence alone. Chosen:
  decision U8; the list is checked against `library/std/prelude.ry`
  entry by entry, and the sheet is at 2998 tokens after shortening the
  phrase-token list to three examples and dropping the nesting and body
  limits, the reserved-word count and "(explicit)" after `Float`.
- What this round is: a defect round, with the sheet corrected and round
  4 run on Sonnet through the same channel; the alternatives were to
  count it as the measurement of U1 to U3 or to wait for gpt-5.5's
  channel. Chosen.
- The reserved word `count` (eleven samples this round): keep R2 and
  revisit with gpt-5.5's data; the alternative was to free it now.
  Chosen.

Observations not asked: `permissions` is right only when the model
thinks (section 2); the `count` cost grew from two samples to eleven with
nothing changed in the reserved list; `low_stock_report` imports
`std.filesystem` twice (once plain, once `exposing Path`) in three of
five samples, which the cheat sheet's import block does not warn
against; the rubric's reading of "only X and Y" with Z also needed is
still open (section 3).
