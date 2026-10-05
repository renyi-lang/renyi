# Example corpus

Hand-written programs that fix the surface of Renyi before the parser exists.
They are the input to the LLM readability test (`docs/design/03-readability-test.md`)
and become the first conformance tests at M1. Every file follows
`docs/design/02-syntax-sketch.md`; `python3 tools/lint_examples.py` checks the
parts a regular expression can see.

| File | Use case | Demonstrates |
|------|----------|--------------|
| `hello.ry` | CLI | module header, `main`, capabilities, `otherwise` default, interpolation |
| `temperature_table.ry` | script | pure function with `example:` lines, ranges with `by`, Decimal literals |
| `shapes.ry` | library | sum type, refinement on variant fields, ability declaration and implementation, `match` as expression, `maybe` |
| `active_users.ry` | data | refined alias, record with derived `FromJson`, multi-line query, error union |
| `word_count.ry` | data | nested query sources, `group by`, `sorted by ... descending`, `take` |
| `weather.ry` | agent / API glue | HTTP and JSON, generic decoding through a `let` annotation, `success` / `failure` matching with typed error patterns, capability scoped to one host |
| `invoice.ry` | library | exact money with Decimal, refinements with `and`, `see also`, `test` blocks with `check` |
| `retry.ry` | library | function-typed parameter, `for any` with a constraint, mutable locals, `time` and `random` capabilities, wrapped parameter list |
| `concurrent_fetch.ry` | agent / API glue | `concurrently` query, `run concurrently` block with a `within` deadline, loop header with `sorted by` |
| `currency_tool.ry` | agent tool | `expose as tool`, refined `Text` subtype used as a map key, three-way error union |
| `sales_report.ry` | data | CSV rows with `maybe` results, error translation with `otherwise fail with`, mutable `Map`, writing a file, capabilities scoped to one directory |
| `config.ry` | service | refined subtypes, decoder-enforced constraints, `deprecated: ... replaced by`, fallback with `otherwise` |
| `http_service.ry` | backend service | pure request handler passed by name, `match` on text, `otherwise return` early exit |
| `todo_cli.ry` | CLI | commands as a sum type, `example:` on parsing, `with_index`, fallible construction with `otherwise` default |
| `traffic_light.ry` | library | state machine as nested `match` over two sum types, `can ToText` on a sum type, accumulating loop |
| `expression_tree.ry` | library | recursive sum type and recursive functions over it, nested constructors in `example:` lines |
| `permissions.ry` | library | `Set` union, intersection, difference and subset, `to_set()`, refined `Text` subtype checked with `matches` |
| `log_parser.ry` | data | `raw` regular expression in a refinement, `match` on text literals, `example: ... fails with ...`, `group by` with a computed key, one-line `if ... end` |
| `deadlines.ry` | script | `Date` arithmetic and ordering from `std.time`, `Weekday` variants through `exposing`, `repeat until`, multi-line `example:` clauses |
| `inventory_db.ry` | data | SQLite through `std.sqlite`, `can FromRow`, typed `let` to decode rows, multi-line string constant, query parameters, `ignore` for an unneeded result |
| `stacks.ry` | library | generic record `Stack of Item`, own ability `Sized` with a generic implementation (`for any Item`), generic functions, `test` blocks with typed `let` |
| `invoice_report.ry` | script | imports the corpus module `invoice` and calls its functions qualified, `sum` over a call, nested record literals |
| `shipping_rules.ry` | embedded library | no `main` and no capabilities: the host application loads the module and calls `quote` with a JSON parcel; `public let` constant; `if` as an expression |
| `semver.ry` | library | hand-written `ToText` implementation, `can Compare by` three fields, refined `Part` subtype, `example: ... fails with ...`, `otherwise fail` inside tests |
| `pagination.ry` | agent / API glue | cursor loop with `repeat until`, `maybe Text` state, `match` as an expression, external field name with `as`, `append_all`, `group by` |
| `statistics.ry` | script | `Float` arithmetic with `power`, `remainder` and `square_root`, `maybe Float` results, `success` / `failure` matching on a conversion |
| `dependency_order.ry` | library | topological sort over a `Map` with `Set` subset tests, nested loops with `where`, `repeat until` over a shrinking map |
| `assistant.ry` | agent / API glue | HTTP POST with headers and a JSON body, `environment.get`, re-raising a matched error with `fail with error`, top-level typed constant |
| `file_tree.ry` | CLI | recursion with effects, `filesystem.inspect` and `filesystem.list`, `Entry` variants through `exposing`, early `return` from a one-line `if` |
| `markdown_table.ry` | library | multi-line `"""` literal in an `example:`, `with_index` queries, `pad_right` and `repeat`, building a list with `set ... to ... append` |

The corpus is at its target size of thirty programs. It grows from here only
when the readability test or the conformance suite needs a construct that no
program covers. Every method and module function the programs call is
declared in `docs/design/04-stdlib-sketch.md`; the lint rejects a call that is
not.
