# Renyi Agent Tooling: the Project Map, Budgets, Diffs and the MCP Server

Status: design accepted (decisions O2 to O5), not yet implemented. Date:
2026-10-05. Companion to `01-decisions.md` (D5, D6, M8, O1 to O5).

Renyi programs are written mostly by LLM agents (decision A3). An agent works
inside a token budget and cannot hold a project in its context, so it needs
a view of the whole project that is smaller than the source, exact, and
cheap to refresh. This document specifies that view, the project map, the
budgets and diffs built on it, and the MCP server that hands all of it to
any agent host.

Three properties of the language make the map exact rather than estimated:
every definition is self-describing (purpose, signature, effects, failures
are syntax), there is no macro expansion, overloading or dynamic dispatch
outside abilities (so every call resolves statically), and the formatter
fixes the layout (so line counts and hashes are canonical).

---

## 1. The index record

`renyi index [path] [--json]` walks a project (a directory, or one file with
its imports) and emits one record per definition and one per module. The
checker (`renyi_check::World`) already resolves every call, type reference
and effect, so the index is a projection of the checked program; a program
with errors is indexed as far as it checks and the record carries the
error count.

A definition record:

| Field | Content |
|-------|---------|
| `id` | content hash of the definition (section 3) |
| `module`, `name`, `kind` | dotted module name; the definition's name; `function`, `method`, `type`, `ability`, `implementation`, `constant`, `test` |
| `public` | whether the definition is part of the module's API |
| `signature` | the head and signature clauses in canonical form, one line |
| `purpose`, `tags`, `see_also`, `deprecated`, `exposed_as_tool` | the documentation clauses |
| `effects.declared`, `effects.transitive` | the `needs` clause, and the union over everything the body reaches, library primitives included |
| `fails.declared`, `fails.transitive` | the `or fails with` types, and the union of what the body can propagate |
| `calls`, `uses`, `implements`, `tested_by` | edges: the definitions it calls (own project and library, qualified), the types it mentions, the ability and target of an implementation, the tests that call it |
| `metrics` | section 2 |
| `coverage.examples`, `coverage.tests` | the number of `example:` lines, and of `test` blocks that call the definition |
| `location` | file, first and last line |

A module record carries the module name and file, its purpose, its imports,
the number of public definitions, total lines, the union of its definitions'
transitive effects, and the list of definition ids. The map is the list of
module records; its header names the project, the revision and the toolchain
version.

## 2. Metrics

Six numbers per definition, each with a fixed definition so that two runs
on the same canonical text agree exactly.

| Metric | Definition |
|--------|------------|
| `lines` | lines from the head line to `end` inclusive, in canonical layout (documentation clauses included; a type's lines are its whole block) |
| `depth` | the deepest block nesting inside the body; the body itself is 0, every `if`, `match`, loop, `run concurrently` body adds 1 (the language rejects more than 4) |
| `branches` | decision points: one per `if` and `otherwise if` condition, per `when` and `otherwise` arm, per loop header and query, per `otherwise` fallback on a `maybe` or fallible value, and per `and` or `or` inside a condition |
| `effects` | the size of the transitive effect set (scoped capabilities count once per path) |
| `fan_in`, `fan_out` | distinct definitions that reference this one; distinct definitions this one references, with `library_calls` counted separately |
| `coverage` | the number of `example:` lines plus the number of tests that call the definition |

Module-level aggregates: definitions, public definitions, lines, the union
of transitive effects, and the maximum of each metric.

## 3. Content hash

Decision D5 fixes identity by content, in the Unison model. The hash of a
definition is a 256-bit hash of its canonical text (the formatter's output)
with the definition's own name removed and every reference to another
project definition replaced by that definition's hash. References to the
standard library are replaced by the library's qualified name and version.
Definitions that reference each other (mutual recursion) form one strongly
connected component and are hashed together, each member's hash being the
component hash plus its index. Renaming a definition therefore keeps its
identity and the identity of everything that calls it; changing a body
changes its hash and the hashes of its callers.

Local names (parameters, bindings) are part of the text in v1; replacing
them by positions so that renaming a local keeps the hash is open item
R5-2.

The hash makes the map incremental: a file whose definitions' hashes are
unchanged needs no new record, and `renyi mcp` refreshes only the changed
components.

## 4. The JSON shape

```json
{
  "project": "examples",
  "revision": "f520dc0",
  "toolchain": "renyi 0.0.1",
  "modules": [
    {
      "name": "invoice",
      "file": "examples/invoice.ry",
      "purpose": "Compute invoice totals with a percentage discount, exact to the cent.",
      "imports": [],
      "definitions": 7,
      "public": 4,
      "lines": 53,
      "effects": [],
      "ids": ["sha256:3f9c...", "sha256:a71e..."]
    }
  ],
  "definitions": [
    {
      "id": "sha256:a71e...",
      "module": "invoice",
      "name": "total",
      "kind": "function",
      "public": true,
      "signature": "total(invoice: Invoice) returns Decimal",
      "purpose": "Subtotal after the percentage discount, rounded to cents.",
      "tags": [],
      "see_also": ["subtotal"],
      "deprecated": null,
      "exposed_as_tool": false,
      "effects": {"declared": [], "transitive": []},
      "fails": {"declared": [], "transitive": []},
      "calls": ["invoice.subtotal", "std.prelude.rounded"],
      "uses": ["invoice.Invoice", "std.prelude.Decimal"],
      "implements": null,
      "tested_by": ["invoice.test:a ten percent discount is taken from the subtotal"],
      "metrics": {"lines": 8, "depth": 0, "branches": 0, "effects": 0, "fan_in": 2, "fan_out": 1, "library_calls": 1},
      "coverage": {"examples": 0, "tests": 2},
      "location": {"file": "examples/invoice.ry", "line": 34, "end_line": 41}
    }
  ]
}
```

Hashes are abbreviated here; the real ones are full. The text form (without
`--json`) prints one line per definition: kind, qualified name, signature,
transitive effects and the metrics, sorted by module and line.

## 5. Budgets

The language's own limits stay compile errors (decision D4: nesting depth,
body length, no shadowing, ASCII names). The map adds module-level budgets
that are reported, not enforced by the compiler (decision O3):

| Budget | Measures |
|--------|----------|
| public definitions per module | the size of a module's API, which is also what the version number (G1) tracks |
| transitive effect set per module | how much of the world a module can touch |
| fan-out per definition | how much a reader must know to understand one definition |

`renyi index --budgets` prints every value over its threshold; CI treats it
as a warning until the thresholds have been measured on real projects (open
item R5-1). Thresholds live in the project manifest once the package
manager exists (M4); until then they are command-line flags with the
corpus-derived defaults.

## 6. Diffs

`renyi index --diff <map or revision>` compares two maps (decision O4) and
reports, per definition: added, removed, signature changed, effects changed
(widened or narrowed, with the capabilities named), failure types changed,
body changed (hash) with the public API unchanged, and for each change the
callers it reaches, found by following `fan_in` edges. The version-bump rule
of G1 reads this diff: a removed or changed public signature is a major
bump, an added one a minor bump. An agent that changed a program checks the
diff for "no effect widened" before it runs anything.

## 7. The MCP server

`renyi mcp` (decision O5) serves the toolchain to any agent host over
JSON-RPC on standard input and output, following the Model Context
Protocol. It keeps the checker's `World` for the project directory resident
and refreshes definitions whose content hash changed.

| Tool | Input | Output |
|------|-------|--------|
| `cheat_sheet` | none | `docs/cheatsheet.md`, the whole language within its token budget |
| `library_lookup` | a name or a few words | matching library declarations with their purposes, from `library/std` |
| `project_map` | optional module name, optional detail level | the map of section 4, or one module's records |
| `definition` | a qualified name | the definition's source text and its record |
| `effects` | a qualified name | the transitive effect and failure graph under that definition |
| `check` | source text or a path | diagnostics as `renyi check --json` prints them |
| `format` | source text | the canonical text, or the parse diagnostics |
| `run`, `run_tests` | a path and arguments | the program's output or the test results (after M3) |
| `diff` | a base map or revision | the semantic diff of section 6 (after M3) |

The first two tools answer the two causes of failure the readability
pre-test found (library names invented, cheat-sheet rules missed) before
any code is written; `check` and `format` close the loop; the map tools are
what an agent reads instead of files.

## 8. Order of work

1. `renyi index` with the record, the six metrics and the hashes, measured
   on the corpus; the text form and `--json`.
2. M3 (the VM) under decision O1.
3. `renyi mcp` with the first seven tools, then `run`, `run_tests` and
   `diff` as M3 and `--diff` land.
4. Budgets with corpus-derived thresholds; the diff; M4 reads budgets from
   the manifest.

## 9. Open items

- R5-1: budget thresholds, after measuring the corpus.
- R5-2: whether local names are normalized away in the content hash.
- R5-3: whether `branches` should count `and`/`or` (cyclomatic style, as
  specified) or only statements; decided by what correlates with the
  readability test's failures.
- R5-4: atomic reference counts when the scheduler runs tasks on several OS
  threads (decision O1).
