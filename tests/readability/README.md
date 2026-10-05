# Readability test harness

Implements `docs/design/03-readability-test.md`. Everything here is a
development aid in Python; it needs only the standard library plus `tiktoken`
for the cheat-sheet gate.

## Files

| Path | Content |
|------|---------|
| `run.py` | the harness: `prepare`, `run`, `score`, `report` |
| `manifest.json` | per program: arguments for Predict, the function removed for Complete |
| `reference/<program>.out` | the exact output the deterministic programs print (Predict references, and the first conformance expectations for M3) |
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
python3 tests/readability/run.py score --format --scores scores-formatted.json   # layout fixed first
python3 tests/readability/run.py report --scores scores-formatted.json
```

`run` stores five samples per prompt at temperature 0 and is restartable: it
only fetches the samples that are missing. `--provider file --answers <dir>`
reads answers from `<dir>/<task>/<name>.<index>.txt`, for a model that has to
be driven by hand.

## Scoring

- **Predict**: the sample must equal `reference/<program>.out` exactly after
  trailing whitespace is removed.
- **Complete** and **Write**: the code is spliced into the program (Complete)
  or taken whole (Write) and must pass `tools/lint_examples.py`. A lint-clean
  answer is then `pending` until a human records a verdict in
  `outputs/<model>/judgement.json` as `{"complete/shapes.0": true, ...}`; an
  entry may also be an object, `{"verdict": true, "reason": "..."}`, so that the
  reason travels with the verdict (decision L2). The VM takes over this step
  at M3.
- **Explain**: graded 1 to 5 by the model named in `--grader`, from the
  program's `purpose:` and `example:` lines; 4 or more passes. A grade can be
  overridden in `judgement.json` with an integer.
- `report` applies the four-of-five rule per item and prints pass rates and,
  for Write, which lint rules the models violated.
- `score --format` runs `renyi format` on every Complete and Write program
  before the lint, so that layout problems the formatter fixes do not count;
  `--scores <name>` keeps that tally in a second file next to the strict one.

## Status

Prompts, references and scoring are in place. One pre-test round exists,
`2026-10-05-e41258c/`, collected from Claude Code subagents (one sample per
item, two Claude models) with its results and deviations in its `notes.md`.
No live round has been run yet: it needs API credentials for at least three
models from two vendors, which the owner provides as environment variables
of the cloud environment.
