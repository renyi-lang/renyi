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
| `weather.ry` | agent / API glue | HTTP and JSON, generic decoding through a `let` annotation, `success` / `failure` matching with typed error patterns |
| `invoice.ry` | library | exact money with Decimal, refinements with `and`, `see also`, `test` blocks with `check` |
| `retry.ry` | library | function-typed parameter, `for any` with a constraint, mutable locals, `time` and `random` capabilities, wrapped parameter list |
| `concurrent_fetch.ry` | agent / API glue | `concurrently` query, `run concurrently` block, loop header with `sorted by` |
| `currency_tool.ry` | agent tool | `expose as tool`, refined `Text` subtype used as a map key, three-way error union |
| `sales_report.ry` | data | CSV rows with `maybe` results, error translation with `otherwise fail with`, mutable `Map`, writing a file |
| `config.ry` | service | refined subtypes, decoder-enforced constraints, `deprecated: ... replaced by`, fallback with `otherwise` |
| `http_service.ry` | backend service | pure request handler passed by name, `match` on text, `otherwise return` early exit |
| `todo_cli.ry` | CLI | commands as a sum type, `example:` on parsing, `with_index`, fallible construction with `otherwise` default |

Still to write before the readability test reaches thirty programs: a state
machine, a recursive tree walk, Set operations, Text processing with
`matches`, date arithmetic with `time`, an SQLite query, a generic container
type with its own ability, a module that imports another corpus module, an
embedded-host example, and an `example: ... fails with ...` clause.
