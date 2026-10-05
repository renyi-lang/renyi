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
