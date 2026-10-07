---
name: renyi
description: Write, check, format, test and run Renyi programs (files ending in .ry). Renyi is the scripting language for AI agents; a program's effects are declared in its signatures, checked before it runs, limited while it runs and recorded as it runs. Use whenever a task writes or changes a .ry file or automates something in Renyi.
---

# Renyi

Read `cheatsheet.md` beside this file before writing a line of Renyi:
it is the whole language on one page and the authority on its spelling.
When the `renyi` MCP server is connected, `cheat_sheet` is the same
page and `library_lookup` finds a library declaration by name or by a
few words; look a function up before calling it, so that its name and
its arguments are not invented.

## The loop

1. Write one module per file: `module <name>` with a `purpose:` line,
   the imports, the types, the functions, and `main` last with the
   `needs` line that declares every effect the program may have.
2. `renyi check <file>`: every diagnostic carries a `fix:` line; apply
   it. The exit code is 1 while an error remains.
3. `renyi format <file>`: the canonical layout, the only one.
4. `renyi test <file>`: every `example:` line and every `test` block.
5. `renyi run <file> [arguments]`: `main`, under the grant its `needs`
   line declares; exit 1 when it fails, 2 on a crash.

A function with effects is tested through a recording: write a `test`
whose head carries `needs` and `replays "fixtures/<name>.json"`, run it
once live with `renyi test --refresh "<test name>" <file>`, and keep the
file it writes beside the program; from then on `renyi test` answers the
test's calls from it and touches nothing outside the process.

## What the checker refuses that other languages allow

Each rule is an error with a fix in the diagnostic; the code is in
parentheses.

- A module and every public function carry a `purpose:` line, in prose
  (`purpose-missing`). Write one on every type too.
- A value name is lower snake case and at least two characters
  (`single-letter-identifier`), and is not one of the 88 reserved words
  (`reserved-word`; reference section 17). The ones that bite as names:
  `sum`, `count`, `first`, `at`, `to`, `in`, `by`, `with`, `within`,
  `tags`, `all`, `any`, `one`, `most`, `least`, `run`, `self`, `some`,
  `type`, `group` and `power`; after a dot they are fine
  (`items.first()`).
- A call with one argument names nothing; a call with two or more names
  every argument, in declaration order (`argument-name`,
  `argument-order`): `recent_commits(directory: here, limit: 50)`.
- Every binding is read (`unused-binding`): a `let`, a parameter, a loop
  variable or a pattern binding that is never used is an error. A name
  is bound once per function (`shadowing`): no reuse of a name.
- A call whose result is not used is an error (`unused-result`); the
  usual fix is `change items to items.append(item)`. `ignore` discards
  the result of a call that has effects.
- Both operands of an arithmetic operator have one numeric type
  (`type-mismatch`), and `/` does not divide two `Integer`s
  (`integer-division`): `dividend.quotient(divisor)` and
  `dividend.remainder(divisor)`.
- A call that can fail or answer `nothing` is read with `otherwise` or
  matched with `success` and `failure` (`missing-otherwise`); there are
  no exceptions. `otherwise fail` on a `maybe` value has no error to pass
  on (`otherwise-fail-maybe`): give a default, `otherwise fail with
  <Error>(...)`, `otherwise return`, `otherwise continue` or `otherwise
  crash with "<reason>"`.
- The `otherwise` that guards a value stays on the line of that value
  (`otherwise-line`); a long statement breaks inside parentheses
  instead, and `renyi format` does the breaking.
- `example:` lines belong to pure functions only (`example-effects`);
  `check` belongs in a `test` block (`check-outside-test`).
- Every capability a body uses, directly or through a call, is in the
  function's `needs` (`capability-missing`); a test's `needs` covers its
  body likewise. A budget is `at most <n> per second|minute|hour|day|run`
  after the capability: `network.http("api.example.com") at most 60 per
  minute`.
- A line is at most 100 columns (`line-width`, a warning); no tabs, no
  carriage returns (`tab`, `crlf`); `renyi format` wraps inside
  parentheses, and a `purpose:` line is shortened by hand.
- An empty list literal names its type: `let mutable items: List of
  Item be []`. A record decodes from JSON with `can FromJson` and
  encodes with `can ToJson`. A library type is imported by name:
  `import std.filesystem exposing Path, FileError`; the module's
  functions are called qualified: `filesystem.read_text(path)`.
- There is no `=`, no `==`, no `null` and no anonymous function
  (`equals-sign`, `symbolic-operator`): `let total be 0`, `is`,
  `nothing`, a named function passed by name.

## A module

```
module log_triage
  purpose: Rank the error codes of a log file by how often they fired.

import std.console
import std.environment
import std.filesystem exposing Path, FileError

public type Entry
  purpose: One ERROR line: its timestamp, its code and its message.
  has timestamp: Text
  has code: Text
  has message: Text
end

public function parse_error(line: Text) returns maybe Entry
  purpose: The entry of a line "timestamp ERROR code message"; nothing for any other line.
  example: parse_error("2026-10-07T10:00:00 INFO started") is nothing

  let words be line.split(" ")
  if words.length() is less than 4 then return nothing end
  let level be words.at(1) otherwise return nothing
  if level is not "ERROR" then return nothing end
  let timestamp be words.at(0) otherwise return nothing
  let code be words.at(2) otherwise return nothing
  return Entry(timestamp: timestamp, code: code, message: words.drop(3).join(" "))
end

public function main() or fails with FileError needs console, environment, filesystem.read("data")
  purpose: Print every error entry of the log named by the argument (data/app.log).

  let arguments be environment.arguments()
  let name be arguments.at(0) otherwise "data/app.log"
  let text be filesystem.read_text(Path(name)) otherwise fail
  for each line in text.lines()
    let entry be parse_error(line) otherwise continue
    console.print("{entry.code}: {entry.message}")
  end
end
```

## The commands

| Command | What it does |
|---------|--------------|
| `renyi check [--json] [--strict] <file>...` | the diagnostics, each with its fix; exit 1 on an error |
| `renyi format [--check] <file>...` | the canonical layout; `--check` only reports |
| `renyi test [--strict] [--refresh <name>] [--explain] <file>...` | every example and test; `--refresh` records one test live |
| `renyi run [options] <file> [arguments]` | `main` under its grant; `--explain` narrates, `--manifest` prints the run manifest, `--replay <recording>` answers every effect from a recording |
| `renyi record [--to <file>] <file> [arguments]` | `main`, with every effect's arguments and result written to a recording |
| `renyi reproduce <recording>` | the same run from the recording, the outcome and the output compared byte for byte |
| `renyi index [--json] [--budgets] [path]` | the project map: every definition with its effects, failures and metrics |
| `renyi tools <file>` | the manifest of every function marked `expose as tool`, as JSON Schema |
| `renyi mcp [path]` | the same answers over the Model Context Protocol |

The standard library modules are `console`, `csv`, `environment`,
`filesystem`, `foreign`, `http`, `json`, `process`, `random`, `regex`,
`server`, `sqlite` and `time`, each under `std.`; the prelude's types,
`Text`, `Integer`, `Decimal`, `Float`, `Boolean`, `Bytes`, `List of`,
`Map of ... to`, `Set of`, `Pair of`, `Range`, `Duration` and `maybe`,
need no import.
