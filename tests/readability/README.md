# Readability test harness

Implements `docs/design/03-readability-test.md`. Everything here is a
development aid in Python; it needs only the standard library plus `tiktoken`
for the cheat-sheet gate.

## Files

| Path | Content |
|------|---------|
| `run.py` | the harness: `prepare`, `run`, `score`, `report`, `compare` |
| `manifest.json` | per program: arguments for Predict, the function removed for Complete |
| `reference/<program>.out` | the exact output the deterministic programs print (Predict references, and the first conformance expectations for M3) |
| `reference/<program>.explain.txt` | the author's description of every program (purpose, inputs, outputs, effects, failures), which the Explain grader compares explanations against |
| `write_tasks.json` | ten task descriptions for Write that are not in the corpus |
| `<date>-<revision>/` | one run: `system.txt` (the cheat sheet as system prompt), `prompts/`, `outputs/<model>/`, `scores.json` |

## Running a round

```
python3 tests/readability/run.py prepare
ANTHROPIC_API_KEY=... python3 tests/readability/run.py run --provider anthropic --model claude-sonnet-5-5
OPENAI_API_KEY=...    python3 tests/readability/run.py run --provider openai --model <id>
OPENAI_API_KEY=...    python3 tests/readability/run.py run --provider openai --base-url https://<vendor>/v1 --model <id>
python3 tests/readability/run.py score --grader anthropic:claude-sonnet-5-5
python3 tests/readability/run.py report
python3 tests/readability/run.py score --no-format --scores scores-strict.json   # layout counts too
python3 tests/readability/run.py report --scores scores-strict.json
python3 tests/readability/run.py compare                                      # VM verdicts against judgement.json
```

`prepare` shows a program together with every corpus module it imports
(`invoice_report` is shown with `invoice`), since the program's behaviour
depends on them; the library is described by the cheat sheet alone.
`run` stores five samples per prompt at temperature 0 and is restartable: it
only fetches the samples that are missing. `--temperature none` omits the
field for a model that rejects it (Claude Sonnet 5.5 does); the temperature
used is recorded with the samples. `--provider file --answers <dir>` reads
answers from `<dir>/<task>/<name>.<index>.txt`, for a model that has to be
driven by hand. Every command works on the run `prepare` wrote most recently
unless `--run <directory name>` names another one.

## Scoring

- **Predict**: the sample must equal `reference/<program>.out` exactly after
  trailing whitespace is removed.
- **Complete** and **Write**: the code is spliced into the program (Complete)
  or taken whole (Write), formatted with `renyi format` when it parses, must
  pass `tools/lint_examples.py`, and must then pass `renyi check` (the type
  and effect checker; its errors are reported as rules `check:<code>`). The
  VM then runs the program's own `example:` lines and `test` blocks (`renyi
  test`): the sample passes when every item passes, and the summary and the
  failing items are kept in the score (decision L2, since M3). A program
  with nothing to run is `pending` until a human records a verdict in
  `outputs/<model>/judgement.json` as `{"complete/shapes.0": true, ...}`; an
  entry may also be an object, `{"verdict": true, "reason": "..."}`, so that the
  reason travels with the verdict. `compare` sets the VM's verdict against
  every judged sample and lists the disagreements, the samples the VM cannot
  decide, and those the lint or the checker now reject.
- **Explain**: graded 1 to 5 by the model named in `--grader` against the
  author's description in `reference/<program>.explain.txt` (the `purpose:`
  and `example:` lines stand in when there is none), with the rubric in
  `EXPLAIN_GRADER`: a statement the description contradicts is wrong, one it
  does not cover is ignored, as are style and added cautions; 5 is complete
  (purpose, effects, main behaviours), 4 misses one of those, 3 is vague or
  misses several, 2 has one wrong statement, 1 is wrong; 4 or more passes.
  The description is language-neutral, so the grader never reads Renyi.
  The grade is the last standalone digit of the grader's answer (a grader
  that reasons first ends with the digit). The grades are cached in
  `outputs/<model>/grades.json` by grader and rubric wording, with the
  grader's answer and a hash of the explanation and the description graded,
  so that a re-score does not call the grader again unless one of them
  changed. The model named in `--second-grader` grades every
  explanation too: the first grader's grade stands when the second is within
  one of it, and a wider disagreement leaves the sample `pending` with both
  grades, for adjudication in `judgement.json` (an integer, or
  `{"verdict": 4, "reason": "..."}`). A grader that answers without a digit
  (Claude Sonnet 5.5 refuses a few explanations of programs that fetch web
  pages) decides nothing, and the other grader's grade stands.
- `report` applies the four-of-five rule per item and prints pass rates and,
  for Write, which lint rules the models violated.
- `score` runs `renyi format` on every Complete and Write program before the
  lint (decision M5), so that layout problems the formatter fixes do not
  count; `--no-format` gives the strict tally and `--scores <name>` keeps it
  in a second file.

## Status

Prompts, references and scoring are in place. One pre-test round,
`2026-10-05-e41258c/`, was collected from Claude Code subagents (one sample
per item, two Claude models). The first live round, `2026-10-05-1623155/`,
ran Sonnet 5.5, Haiku 4.5 and gpt-5.4-mini through the vendor APIs with
five samples per prompt; its `notes.md` has the method, every deviation,
the results against the thresholds and the analysis of the failures. The
credentials are the environment variables `ANTHROPIC_API_KEY` and
`OPENAI_API_KEY` (decision L1).
