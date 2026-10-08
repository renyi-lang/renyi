# Renyi Agent Tooling: the Project Map, Budgets, Diffs and the MCP Server

Status: design accepted (decisions O2 to O5). `renyi index` (sections 1 to
4) is implemented in `crates/renyi_index` and measured on the corpus
(section 5); `renyi index --budgets` reports the values over the thresholds
of decision R7; `renyi mcp` (section 7) is implemented in
`crates/renyi/src/mcp.rs` with all ten tools; the diff (section 6) in
`crates/renyi_index/src/diff.rs` behind `renyi index --diff`. Date:
2026-10-06. Companion to `01-decisions.md` (D5, D6, M8, O1 to O5, R7, T1
to T5).

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
error count. The map is computed on the canonical text of every file (the
formatter's output), so that lines and hashes do not depend on layout; a
module record says whether its file was already canonical.

A definition record:

| Field | Content |
|-------|---------|
| `id` | content hash of the definition (section 3) |
| `text_hash` | hash of the definition's own canonical text with its name and its references blanked; unlike `id`, unchanged when a dependency changes (section 6) |
| `module`, `name`, `kind` | dotted module name; the definition's name; `function`, `method`, `type`, `ability`, `implementation`, `constant`, `test` |
| `package` | `<name> <version>` of the dependency the definition was read from (decision AC1); `null` for the project's own |
| `public` | whether the definition is part of the module's API |
| `signature` | the head and signature clauses in canonical form, one line |
| `purpose`, `tags`, `see_also`, `deprecated`, `exposed_as_tool` | the documentation clauses |
| `effects.declared`, `effects.transitive` | the `needs` clause, and the union over everything the body reaches, library primitives included; a call of an ability method reaches every implementation of it |
| `fails.declared`, `fails.transitive` | the `or fails with` types, and the union of the declared failure types over everything the body reaches |
| `calls`, `uses`, `implements`, `tested_by` | edges: the functions and ability methods it calls or passes by name (own project and library, qualified: `module.name`, `module.Type.method`, `module.Ability.method`), the types, constants and abilities it mentions, the ability and target of an implementation (`{"ability", "target"}`), the tests whose bodies refer to it |
| `metrics` | section 2 |
| `coverage.examples`, `coverage.tests` | the number of `example:` lines, and of `test` blocks whose bodies refer to the definition |
| `location` | file, first and last line |

The kinds are `function`, `method` (a function with a `self` parameter,
named `Type.method`), `type`, `ability`, `implementation` (named `Ability
for Target`; it refers to everything its methods refer to), `constant` and
`test` (named `test:` and its name).

A module record carries the module name and file, its purpose, its imports,
the number of definitions and of public ones, total lines, the union of its
definitions' transitive effects, the list of definition ids, the number of
errors the checker reported and whether the file was in canonical layout.
The map is the list of module records and the list of definition records;
its header names the project, the revision and the toolchain version.

## 2. Metrics

Six numbers per definition, each with a fixed definition so that two runs
on the same canonical text agree exactly.

| Metric | Definition |
|--------|------------|
| `lines` | lines from the head line to `end` inclusive, in canonical layout (documentation clauses included; a type's lines are its whole block) |
| `depth` | the deepest block nesting inside the body; the body itself is 0, every `if`, `match`, loop, `run concurrently` body adds 1 (the language rejects more than 4) |
| `branches` | decision points: one per `if` and `otherwise if` condition, per `when` and `otherwise` arm, per loop header and query, per `otherwise` fallback on a `maybe` or fallible value, and per `and` or `or` inside a condition |
| `effects` | the number of distinct capability paths in the transitive effect set (scopes do not count; a parent and a child declared at different levels count as two paths) |
| `fan_in`, `fan_out` | distinct project definitions that reference this one (tests included); distinct project definitions this one references by a call or a use, itself excluded, with `library_calls` (distinct library functions called) counted separately |
| `coverage` | the number of `example:` lines plus the number of tests that refer to the definition (reported as `coverage.examples` and `coverage.tests`) |

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

As implemented (`crates/renyi_index/src/hash.rs`): the name tokens that
refer to other project definitions are those of calls, functions passed by
name, constants, type names in signatures, constructions, patterns and
annotations, and ability names in `for any`, `can` and implementation
heads. A variant's name token is left as it is (its type is a dependency,
so a change to the type still changes the hash). References to the library
stay spelled as written, and the qualified library names used are appended
with the toolchain version. The members of a component are hashed in
definition order, so reordering two mutually recursive definitions changes
their hashes; reordering independent definitions does not.

The map is incremental through the resident world of decision AN1
(`renyi_workspace`): the check of an item is kept across refreshes while
its text and the project's declarations are unchanged, and the records
are computed again from the kept checks, which is cheap next to checking.

## 4. The JSON shape

```json
{
  "project": "examples",
  "revision": "3c7ee41",
  "toolchain": "renyi 0.0.1",
  "modules": [
    {
      "name": "invoice",
      "file": "examples/invoice.ry",
      "purpose": "Compute invoice totals with a percentage discount, exact to the cent.",
      "imports": [],
      "definitions": 7,
      "public": 5,
      "lines": 53,
      "effects": [],
      "ids": ["sha256:3f9c...", "sha256:7b31..."],
      "errors": 0,
      "canonical": true
    }
  ],
  "definitions": [
    {
      "id": "sha256:7b31...",
      "text_hash": "sha256:a4e2...",
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
      "calls": ["invoice.subtotal", "std.prelude.Decimal.rounded"],
      "uses": ["invoice.Invoice", "std.prelude.Decimal"],
      "implements": null,
      "tested_by": [
        "invoice.test:a ten percent discount is taken from the subtotal",
        "invoice.test:an invoice without lines totals zero"
      ],
      "metrics": {"lines": 8, "depth": 0, "branches": 0, "effects": 0, "fan_in": 4, "fan_out": 2, "library_calls": 1},
      "coverage": {"examples": 0, "tests": 2},
      "location": {"file": "examples/invoice.ry", "line": 34, "end_line": 41}
    }
  ]
}
```

Hashes are abbreviated here; the real ones are full. The record is the one
`renyi index --json examples` prints for `invoice.total` (its `fan_in` of 4
counts the two tests of `invoice` and the two functions of `invoice_report`,
which imports it). The text form (without `--json`) prints one line per
definition: kind, qualified name, signature, transitive effects and the
metrics, sorted by module and line.

## 5. Budgets

The language's own limits stay compile errors (decision D4: nesting depth,
body length, no shadowing, ASCII names). The map adds module-level budgets
that are reported, not enforced by the compiler (decision O3):

| Budget | Measures |
|--------|----------|
| public definitions per module | the size of a module's API, which is also what the version number (G1) tracks |
| transitive effect set per module | how much of the world a module can touch |
| fan-out per definition | how much a reader must know to understand one definition |

`renyi index --budgets` prints every value over its threshold, one line
each (`module m: 11 public definitions (budget 10)`), or "nothing over
budget", and exits 0 either way: CI treats it as a warning until the
thresholds have been measured on real projects. The thresholds are the
defaults of decision R7 (public definitions per module 10, transitive
effect paths per module 5, fan-out per definition 7, `renyi_index::Budgets`);
since decision AC1 the `budgets` object of `renyi.json`
(`public_per_module`, `effect_paths_per_module`, `fan_out_per_definition`)
overrides each one it names. A module or a definition read from a
dependency carries `package: <name> <version>` in the map (the text line
and the JSON field), so that a budget report or a diff can be read per
package.

First measurements, the corpus at revision 3c7ee41 (30 modules, 166
definitions: 95 functions, 4 methods, 52 types, 2 abilities, 3
implementations, 4 constants, 6 tests), from `renyi index --json examples`:

| Measure | median | 90th percentile | maximum |
|---------|--------|-----------------|---------|
| public definitions per module | 4 | 7 | 9 (`log_parser`) |
| transitive effect paths per module | 2 | 3 | 5 (`sales_report`, `todo_cli`) |
| fan-out per definition | 1 | 3 | 7 (`shipping_rules.quote`) |
| lines per function, method or test | 9 | 17 | 30 (`dependency_order.order`) |
| depth per body | 0 | 1 | 3 (`stacks.balanced`) |
| branches per body | 2 | 5 | 8 (`todo_cli.parse_command`) |
| library calls per body | 2 | 5 | 9 |

Of the 82 public functions and methods, 43 have neither an `example:` line
nor a test that refers to them. The corpus is thirty small programs, not a
project, so the thresholds taken from it (public definitions per module 10,
transitive effect paths per module 5, fan-out per definition 7: each one
just above the corpus maximum) are a first setting (decision R7,
2026-10-05), not a measurement on real projects; the corpus itself has
nothing over budget, which a test of `renyi_index` keeps true.

## 6. Diffs

`renyi index --diff <base> [--json] [path]` compares the project with a
base (decision O4): a map file that `renyi index --json` wrote, or a git
revision, whose files are read with `git show` and indexed afresh. It
reports, per definition: added, removed, renamed (the same content hash
under another name, decision D5), signature changed (the head and its
clauses, so a changed `needs` or `or fails with` is a signature change),
visibility changed, effects changed (widened or narrowed, with the
capabilities named, on every definition whose transitive effects differ),
failure types changed, and body changed with the signature unchanged; and
for each entry the definitions the change reaches, found by following the
`calls` and `uses` edges backwards to every transitive caller. The content
hash alone cannot tell a changed body from a changed dependency, since both
change it, so a record also carries `text_hash` (section 1): a body change
is reported once, where the text changed, and the callers appear under what
it reaches; a map without `text_hash` (an older toolchain's) falls back to
the content hash (decision T5). The version-bump rule of G1 reads this
diff: a public definition removed, its signature changed, made private or
renamed is a major bump; a public definition added or made public a minor
bump. An agent that changed a program checks the diff for "no effect
widened" before it runs anything.

The text form prints one line per changed definition (`name (public kind):
changes; reaches ...`) and the bump; `--json` prints `{"old", "new"}` (the
two headers), `"changes"` (one object per entry: `name`, `kind`, `public`,
`changes` as `{"change": "added" | "removed" | "renamed" | "signature" |
"visibility" | "effects" | "failures" | "body", ...}`, `reaches`) and
`"bump"`. Implemented in `crates/renyi_index/src/diff.rs`; the base is
loaded by `crates/renyi/src/maps.rs`.

## 7. The MCP server

`renyi mcp` (decision O5) serves the toolchain to any agent host over
JSON-RPC on standard input and output, following the Model Context
Protocol. It holds a resident world of the served directory
(`renyi_workspace`, decision AN1): before a call that needs the map, the
files are read again where their stamps changed, the world is declared
again when any text did, and only the items whose text changed are checked
again (open item R5-5, closed).

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
| `diff` | a base map or revision, optional `json` | the semantic diff of section 6 |

The first two tools answer the two causes of failure the readability
pre-test found (library names invented, cheat-sheet rules missed) before
any code is written; `check` and `format` close the loop; the map tools are
what an agent reads instead of files.

As implemented (`crates/renyi/src/mcp.rs`, decisions T1 to T4): `renyi mcp
[path]` serves the directory given (the current one by default) and exits
when its input closes. It speaks both eras of the protocol: a request
whose `_meta` carries `io.modelcontextprotocol/protocolVersion` is
answered as revision 2026-07-28 says (`server/discover`, a `resultType`
and the server's identity in every result, error `-32022` for a version
the server does not speak, `-32602` for a request without the client's
capabilities), and an `initialize` request opens the handshake of the
revisions 2024-11-05 to 2025-11-25; the tools are the same in both. The
messages are read and written with the VM's own JSON reader and writer.
`project_map` takes `module` and a `json` switch (the text form of `renyi
index` by default); `definition` and `effects` take a qualified name, or a
bare one when it is unique in the project; `check` takes `source` or
`path`, or both (the text and the file it would be, so that its imports
resolve); `run` takes the arguments, the narrowing options of the command
line (`deny`, `allow_host`, `allow_read`, `allow_write`, `at_most`),
`replay` and `explain`, answers with what the program printed, its
standard error and how it ended, and marks a run that did not finish as a
tool error; `run_tests` takes `strict` and `explain`. The map is refreshed
before the call through the resident world of decision AN1: a file is read
again when its stamp changed, parsed again when its text changed, and its
items are checked again where their text changed (every item when a
declaration changed anywhere). A missing argument or a failing tool is a
tool execution error (`isError`); an unknown tool or method is a protocol
error. `diff` takes `base` (a saved map file under the served directory,
or a git revision) and a `json` switch and answers as `renyi index --diff`
prints.

### The tool manifest (`renyi tools`)

Decision D6 publishes a function with `expose as tool` to agents; `renyi
tools [path]` prints the manifest (`crates/renyi_index/src/tools.rs`,
decision V6): one JSON object per tool with `name` (`module.function`),
`description` (the `purpose:`), `input_schema` (a JSON Schema object with
one property per parameter, `required` listing the ones that are not
`maybe`), `output_schema` (the result's schema, `null` without `returns`),
`fails` (the declared failure types), `permissions` (the `needs` clause),
`module`, `function`, `file` and `line`. The schemas follow the `ToJson`
and `FromJson` rules of the library sketch: a record is an object keyed by
its field names or their `as` names, a sum type is a `oneOf` with the
variant's name under `kind`, `maybe T` admits `null`, a list or a set is
an array, a map keyed by `Text` is an object, `Decimal` is a number with
`"format": "decimal"`, `Bytes` is a base64 string, an `Instant` or a
`Date` is a string; a type that refers to itself is written once under
`$defs`. The checker rejects a tool whose parameter or result is not JSON
(`tool-type`) or that has no purpose. `renyi serve --mcp` (M5) will serve
the same entries.

## 8. Order of work

1. `renyi index` with the record, the six metrics and the hashes, measured
   on the corpus; the text form and `--json`. Done 2026-10-05
   (`crates/renyi_index`; the checker records every reference it resolves
   and `renyi_check::check_project` checks a project as a whole).
2. M3 (the VM) under decision O1.
3. `renyi mcp` with the first seven tools, then `run`, `run_tests` and
   `diff` as M3 and `--diff` land. Done 2026-10-06
   (`crates/renyi/src/mcp.rs`, decisions T1 to T5).
4. Budgets with corpus-derived thresholds (done 2026-10-05, decision R7:
   `renyi index --budgets`); the diff (done 2026-10-06, decision T5); the
   budgets of `renyi.json` (done 2026-10-07, decision AC1).
5. `renyi run --profile` (done 2026-10-06, decision X4): a timer raises a
   flag every half millisecond, the operation running when it was raised
   gets the sample (a primitive's time lands on the primitive), and every
   operation, call and primitive call is counted; the report on the
   standard error has the samples by function and operation, by function
   and by operation kind with the primitives by name, then the counts.
   A development aid like `renyi tokens`: it is how the VM's own hot
   spots were found (`crates/renyi_vm/src/profile.rs`).

## 9. Open items

- R5-1: budget thresholds. Settled by decision R7 (10 / 5 / 7) as the first
  defaults; to be revisited on real projects.
- R5-2: whether local names are normalized away in the content hash.
- R5-3: whether `branches` should count `and`/`or` (cyclomatic style, as
  specified) or only statements; decided by what correlates with the
  readability test's failures.
- R5-4: atomic reference counts when the scheduler runs tasks on several OS
  threads (decision O1).
- R5-5: the per-definition refresh of the map in `renyi mcp`. Closed by
  decision AN1: the resident world of `renyi_workspace` checks again only
  the items whose text changed (decision T4 had rebuilt the whole map when
  any file of the served directory changed).
