# The starter pack for agents

Renyi is the scripting language for AI agents: a program's effects are
declared in its signatures, checked before it runs, limited while it runs
and recorded as it runs, so that a person can review what an agent wrote
in one pass. This directory is what an agent needs in its first hour
(decision AI3 of `docs/design/01-decisions.md`):

| Piece | Path | What it is |
|-------|------|------------|
| the skill | `skill/renyi/` | a `SKILL.md` an agent host loads when a task touches Renyi, with the cheat sheet beside it |
| the MCP configuration | `mcp.json` | one entry that starts `renyi mcp` for the current directory |
| the workflows | `workflows/` | five programs that do real work, each with its tests and, where it has effects, a recording |

## 1. Install `renyi`

One line on Linux and macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/renyi-lang/renyi/main/install.sh | sh
```

On Windows, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/renyi-lang/renyi/main/install.ps1 | iex
```

With a Rust toolchain, `cargo install renyi` builds the same binary.
`renyi --version` prints the version; `README.md` at the root of the
repository has the details.

## 2. Give the agent the skill

Copy `skill/renyi/` into the directory the host reads skills from. For
Claude Code that is `.claude/skills/renyi/` in the project, or
`~/.claude/skills/renyi/` for every project; other hosts that read the
`SKILL.md` format take the same directory. The skill tells the agent the
loop (write, `renyi check`, `renyi format`, `renyi test`, `renyi run`),
the rules the checker enforces that other languages do not, and where
the cheat sheet is. `skill/renyi/cheatsheet.md` is a copy of
`docs/cheatsheet.md`; CI keeps the two equal.

## 3. Connect the MCP server

`renyi mcp <directory>` serves a directory over the Model Context
Protocol on standard input and output. `mcp.json` holds the entry:

```json
{
  "mcpServers": {
    "renyi": {
      "command": "renyi",
      "args": ["mcp", "."]
    }
  }
}
```

For Claude Code, copy it to `.mcp.json` at the root of the project, or
run `claude mcp add --transport stdio renyi -- renyi mcp .`; other hosts
take the same command and arguments. The server's tools:

| Tool | What it answers |
|------|-----------------|
| `cheat_sheet` | the whole language on one page |
| `library_lookup` | standard library declarations by name or by a few words, with signature, effects and purpose |
| `project_map` | one record per module and definition of the served directory: signature, purpose, effects, failures, edges, metrics, content hash |
| `definition` | one definition's record and source text, by qualified name |
| `effects` | the transitive effect and failure graph under a definition, down to the library primitives |
| `check` | the diagnostics of a file or of a source text, as `renyi check --json` prints them |
| `format` | the canonical layout of a source text |
| `run` | run a program's `main` under its grant, narrowed by `deny`, `allow_host`, `allow_read`, `allow_write` and `at_most`; `replay` answers every effect from a recording |
| `run_tests` | every `example:` line and `test` block of a program, `replays` tests from their recordings |
| `diff` | what changed since a saved map or a git revision, what each change reaches, the version bump it forces |

The server is the same code as the commands: a host without MCP runs the
commands of section 5 instead.

## 4. The five workflows

Each is one file under `workflows/`, run from that directory. The `needs`
line of its `main` is the whole of what it may do.

| Program | What it does | `main` needs |
|---------|--------------|--------------|
| `repo_digest.ry` | who made the last fifty commits of a git repository, most first, with each author's latest subject | `console, environment, process("git") at most 1 per run` |
| `api_digest.ry` | one line of public numbers of a GitHub repository, from the API | `console, environment, network.http("api.github.com") at most 5 per run` |
| `expense_report.ry` | the totals of the expense CSV files of `data/expenses` by category and by month, printed and written to `out/report.md` | `console, filesystem.read("data"), filesystem.read("out"), filesystem.write("out")` |
| `log_triage.ry` | the five error codes that fired most in `data/app.log`, with when each was first and last seen | `console, environment, filesystem.read("data")` |
| `link_check.ry` | every link of `data/links.md` fetched, with its status, and the count of broken ones | `console, environment, filesystem.read("data"), network.http at most 20 per run` |

```sh
cd starter/workflows
renyi check *.ry                     # clean: no output, exit 0
renyi test *.ry                      # every example and test; the three replays tests answer from fixtures/
renyi run repo_digest.ry ../..       # the repository to digest (the current directory by default)
renyi run api_digest.ry rust-lang/rust
renyi run expense_report.ry
renyi run log_triage.ry
renyi run link_check.ry
```

A program with effects has a `replays` test, whose recording under
`fixtures/` was made from a live run with `renyi test --refresh "<test
name>" <file>`; `renyi test` answers the test's calls from the recording
and touches neither git nor the network. Re-record one after changing
what the program asks for:

```sh
renyi test --refresh "the log of this repository is read" repo_digest.ry
renyi test --refresh "a record is decoded" api_digest.ry
renyi test --refresh "three links are checked" link_check.ry
```

## 5. Reviewing what an agent wrote, in one pass

1. Read the `needs` line of `main`: the capabilities, their scopes (a
   host, a directory, a program) and their budgets (`at most 20 per run`)
   are the whole of what the program may do; the checker refused
   anything else (`capability-missing`).
2. Read the `purpose:` lines: every module, public function and type has
   one, in prose.
3. Read the `otherwise` and `fail with` clauses: every call that can fail
   says what happens when it does; there are no exceptions.
4. `renyi index workflows/` prints the project map, one line per
   definition with its effects and failures; `renyi index --budgets`
   every value over a budget.
5. `renyi run --manifest <file>` prints, after the run, the toolchain,
   the code hash, the grant, the arguments, the environment variables
   read, the outcome and the output hash; `renyi run --explain <file>`
   narrates the run, purpose by purpose.

## 6. Recording and reproducing a run

```sh
renyi record --to digest.json api_digest.ry rust-lang/rust   # the run, with every effect's arguments and result
renyi reproduce digest.json                                  # the same run from the recording: no network, same output
```

`renyi reproduce` replays the recording under its manifest and compares
the outcome and the output byte for byte; it exits 1 when they differ.
`renyi run --replay digest.json --explain api_digest.ry` runs the
program again with every effect, the arguments and the prints included,
answered from the recording, so nothing is sent or shown, and narrates
the run on the standard error: purpose by purpose, each call with its
arguments and its result. That is how a changed program is read against
an old run.
