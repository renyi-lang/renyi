# LLM Readability Test Protocol

Status: protocol defined, harness not yet written. Date: 2026-10-05.

Decision H2 freezes the grammar by measurement, not by implementation: before
the parser is written, several models must read the cheat sheet and work with
the corpus at a correctness rate that justifies the design. This document fixes
the protocol so that the result is comparable across grammar revisions and
across models.

## What is measured

Four tasks, each scored per program per model. Every prompt contains only the
cheat sheet (`docs/cheatsheet.md`) and the task; no other Renyi material.

| Task | Prompt | Scored by |
|------|--------|-----------|
| Predict | "Here is a Renyi program and its input. What does it print?" | exact match of the predicted output against the reference output |
| Explain | "Explain what this program does in three sentences." | a second model grades the explanation against the purpose clauses, blind to which grammar revision produced the program |
| Complete | a program with one function body removed, the signature and purpose kept | the completed body passes the program's `example:` and `test` blocks, judged by hand until M3 and by the VM afterwards |
| Write | a one-paragraph task description | the written program passes the lint, then the same acceptance tests as Complete |

The Write task also records the number of lint violations per program, broken
down by rule, so that each grammar rule's cost in model errors is visible.

## Models and settings

At least three models from at least two vendors, including the smallest model
the project intends to support. Temperature 0 where the API allows it. Five
samples per task per program; a task passes for a program when at least four
of five samples are correct. Prompts, raw outputs and scores are committed
under `tests/readability/<date>-<grammar-revision>/`.

## Acceptance for freezing the grammar

- Predict and Explain: at least 90 percent of programs pass on every model.
- Complete: at least 80 percent on every model.
- Write: at least 70 percent on every model, and no single lint rule accounts
  for more than a quarter of the violations.
- The cheat sheet stays within its 3000-token budget (`tools/count_tokens.py`).

A grammar change that lowers any passing rate by more than five points is
reverted or accompanied by a decision entry explaining why the loss is worth
it.

## Regression

The same protocol reruns in CI on every change to `docs/cheatsheet.md`, to the
grammar, or to the corpus, with one sample per task to keep cost low, and with
the full five samples before each release.

## Known limits

Models trained after the corpus is public may have seen it; the Write task with
fresh task descriptions is the control. The Explain grader shares the biases of
the grading model; two graders are used when the two disagree by more than one
point on a five-point scale.
