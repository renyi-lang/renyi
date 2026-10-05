# Renyi Design Decisions, Round 1

Status: accepted. Date: 2026-10-05.

This record captures the first round of design decisions for Renyi. Each entry
lists the question, the options that were on the table, the decision, and the
reason. Decisions marked **(user)** were made explicitly by the language owner;
decisions marked **(derived)** follow from those answers and the design
reasoning recorded under them. The questions were asked in an interactive
session; the original question list lives in git history (commit 231567e).

Later rounds append new entries rather than editing these. A decision is
reversed by a new entry that names the one it supersedes.

---

## 0. Tensions and the trade-offs taken

The requirements pull against each other in four places. These trade-offs
were put to the owner and accepted.

| # | Tension | Decision |
|---|---------|----------|
| T1 | English-like syntax vs. token cost and LLM "plausible invention" | **Regular English, not natural English.** Fixed clause shapes, exactly one spelling per concept, no synonyms, one formatter output. (user) |
| T2 | Scripting convenience vs. Haskell-grade strictness | **No dynamic escape hatch.** Full type inference inside function bodies; mandatory signatures at the top level; errors carry fix suggestions so an agent self-corrects in one loop. (user) |
| T3 | Interop with other languages vs. strictness | **FFI is first-class but boundary-validated.** Foreign bindings read like ordinary functions and are generated from the host's own type information; every value crossing the boundary is validated at runtime and failures flow through the normal error channel. (user, revising the originally proposed "second-class but safe") |
| T4 | Runtime speed vs. compile speed vs. implementation effort | **Compile/check speed > implementation effort > runtime speed.** v1 is a bytecode VM; hot paths get AOT later. (user) |

---

## A. Positioning

**A1. Script language, application language, or both.** One toolchain, two
modes. `renyi run file.ry` type-checks and executes on the bytecode VM with
sub-30 ms startup. `renyi build` produces a self-contained executable (VM plus
bytecode). AOT compilation is a later milestone. (user)

**A2. First real use cases.** All four: agent workflows and API glue (including
a sandbox for agents to run computations safely), data processing and cleaning
scripts, backend services, and CLI / automation tools. The standard library and
the capability model are prioritized in that order. (user)

**A3. Primary author.** LLM agents write, humans review. Consequences: errors
are machine-parsable, the grammar forbids alternative spellings, documentation
clauses are mandatory on public definitions, and the acceptance test for every
syntax decision is "a human reviewer understands it at a glance". (user)

**A4. Performance anchor.** Lua-class interpreter for v1: bytecode VM, startup
under 30 ms, several times faster than CPython on typical scripts. No AOT or
JIT in v1. (user)

**A5. First-class platforms.** Linux, macOS and Windows command line; server-side
WASI; browser WASM; embedding into host programs (Rust, Python, JS, C) through a
stable C API. Mobile is out of scope. (user)

---

## B. Types and strictness

**B1. Effects in the type system.** Yes, fine-grained. A function declares the
capabilities it uses with a `needs` clause: `needs network.http,
filesystem.read`. Time and randomness are capabilities too, so a function with
no `needs` clause is pure and therefore replayable and trivially testable. The
checker requires every caller to declare a superset of its callees' effects.
Effects of function-typed parameters flow to the call site automatically, so
users never see effect variables. (user)

**B2. Error handling.** No exceptions. A function that can fail declares
`or fails with ErrorType` in its signature; the caller writes `otherwise fail`
to propagate, `otherwise fail with NewError` to translate, or `otherwise
<default>` to recover. `Result` and `Maybe` exist in the compiler but users
rarely spell them. The only ways a program terminates abnormally are an
explicit `crash with "message"` and arithmetic domain errors such as division
by zero. (user; the arithmetic exception is derived)

**B3. Mutability.** Immutable by default. `let mutable name be value` opens a
local mutable binding; `set name to value` updates it. Mutable state cannot be
shared across functions; the type system has no reference type. Imperative
`for each` loops stay in the language because LLMs write them more reliably
than folds. (user)

**B4. Evaluation.** Strict. A `lazy` keyword is reserved for opt-in laziness in
a later version. (user)

**B5. Type system features in v1.** Algebraic data types with exhaustive
pattern matching; generics spelled `List of Item`; abilities (type classes
without exposed higher-kinded types); refinement types and contracts spelled
`Integer where value is at least 0`. Higher-kinded types, GADTs and dependent
types are not implemented even internally. (user, who asked for the full set)

**B5a. One unified type construct.** The owner asked for "one universal,
powerful type construct that greatly improves readability". Type theory needs
only four constructors for data: product, sum, subset and constraint. Renyi
spells them with one keyword and four clause words:

```
type User
  has name: Text                                  # product
  has age: Integer where age is at least 0        # subset (refinement)
  can Compare by age                              # ability (constraint)
end

type Shape is one of                              # sum
  Circle(radius: Decimal)
  Rectangle(width: Decimal, height: Decimal)
end

type Email is Text where value matches email_pattern   # refined alias
```

The full design is in `02-syntax-sketch.md`. (derived)

**B5b. Type annotations.** Every top-level function carries a full signature
(parameters, return type, failure type, effects). Inside function bodies
everything is inferred; `let name: Type be ...` is available when the author
wants to pin a type or guide generic inference. (user)

**B6. Escape hatches.** None. No `any`, no unsafe cast, no dynamic typing mode.
Unsafety lives only on the far side of the FFI boundary. (user)

**B7. Null.** Forbidden. An optional value has type `maybe T`; the absent value
is `nothing`; reading it requires `otherwise` or a `match`. (user)

**B8. Object orientation.** No classes, no inheritance. Data is records and
algebraic types; polymorphism is abilities. Dot syntax is kept for field access
(`user.name`) and for calling ability functions (`shape.describe()`). (user)

**B9. Numbers.** `Integer` is arbitrary precision. `Decimal` is exact decimal
arithmetic and is the type of literals like `19.99`. `Float` is IEEE double and
must be chosen explicitly. There are no implicit numeric conversions. (user)

---

## C. Syntax

**C1. Overall style: clause grammar.** The owner asked for a merge of the
"sentence" style and the "keyword" style, with a principled basis. The basis:
syntax is a map from the abstract syntax tree to strings, and three properties
make it good. It is injective and invertible (one spelling per program; the
formatter is the inverse map). It has low conditional entropy (given the
prefix, few continuations are legal, which is what makes an LLM's output
reliable). It is homomorphic to the semantics (types are products, sums,
subsets; error propagation is a monadic bind; collection queries are set
comprehensions and monoid folds), so surface words map one-to-one onto those
structures.

The resulting design is the **clause grammar**: every construct is a head
followed by labelled clauses. Clauses behave like record fields: optional,
with a canonical order the formatter enforces. Mathematical notation that is
universal stays (`f(x)`, `x: Type`, `+ - * /`); everything else is an English
word. A short signature sits on one line; a long one puts each clause on its
own line. Both are the same token sequence. (user accepted the merged design)

**C1a. Three verbs replace `=`.** `let x be 5` defines, `set x to 6` mutates,
`x is 5` tests. The language has no `=` symbol. This removes the `=`/`==`
confusion class of bugs entirely; an LLM that writes `=` by reflex gets a
compile error with the fix. (user)

**C2. Blocks.** Explicit `end`. Indentation is never semantic. LLMs are not
robust to whitespace, RAG chunking and copy-paste destroy indentation, and
`end` makes every block self-delimiting. The formatter sets indentation to two
spaces. (user)

**C3a. Equality.** `is` and `is not`. Renyi has no object identity, so there is
no Python-style identity/equality confusion. (user)

**C3b. Ordering comparisons: six phrases, no symbols.** The owner suggested
deriving comparison words from `is`. One primitive, `compare(a, b)` returning
`Less`, `Equal` or `Greater`, is provided by the `Compare` ability; the six
phrases are the only spellings of relations:

| Spelling | Meaning |
|----------|---------|
| `a is b` | equal |
| `a is not b` | not equal |
| `a is less than b` | compare is Less |
| `a is at most b` | compare is not Greater |
| `a is greater than b` | compare is Greater |
| `a is at least b` | compare is not Less |

The phrase set is closed: `larger`, `bigger`, `more than` are not accepted.
Rule for symbols versus words: symbols are kept for operations that nest and
grow long (arithmetic), words are used for relations, which produce a Boolean
and do not nest. (user)

**C3c. Remaining symbol table.** Accepted as proposed: `and`, `or`, `not`;
`+ - * /` kept; `remainder` and `power` as infix words replacing `%` and `**`;
no pipeline operator (use `let` steps); `#` comments; `"Hello {name}"`
interpolation; `List of T` generics; `maybe T` optionals. (user)

**C4. One spelling per concept.** No synonyms. There is `otherwise` and no
`else`; `function` and no `fn` or `def`. Common misspellings from other
languages produce a compile error with the canonical form as the suggested
fix. (user)

**C4a. Multi-word keywords are single lexical units.** The owner asked whether
`at least` is one keyword or two, and whether to write `at_least`. Decision:
phrases such as `is at least`, `or fails with`, `for each` are single tokens
recognized by the lexer through a fixed phrase table with longest match (the
same technique SQL uses for `ORDER BY` and `IS NOT NULL`). No underscores: LLM
tokenizers split on whitespace anyway, embedding models align `at least` with
natural-language queries while `at_least` reads as an identifier, and the
source reads as English. Rules: the phrase table is fixed and printed in full
in the cheat sheet; only single spaces are allowed inside a phrase and no line
breaks; every word that appears in any code phrase is a reserved word; content
words used as keywords (`count`, `sum`, `first`, `type`, `end`) are reserved
too and the compiler suggests a descriptive rename, which is itself good for
retrieval; the one exception is a member name after a dot (`event.type`,
`list.count`), which may be any word because position disambiguates. Words
inside free-text documentation clauses are not reserved. (user)

**C5. Enforcement.** Naming rules (snake_case values and functions, PascalCase
types, abilities and variants) are compile errors with a rename suggestion.
Layout (one statement per line, no semicolons, line width, indentation) is
owned by `renyi format`, which has no configuration. (user)

**C6. Identifiers.** ASCII only: `a-z`, `0-9`, `_` for values, `A-Z a-z 0-9`
for types. Strings and comments accept any Unicode. (user)

**C8a. Documentation is part of the grammar.** Every public function, public
type and every module must carry a one-sentence `purpose:` clause; a missing
purpose is a compile error. Private helpers may omit it. (user)

**C8b. Documentation clauses.** `purpose:`, `example:` (an expression and its
expected value, executed by `renyi test`), `tags:` (free words, exported to the
index), `see also:` (references the compiler checks for existence),
`deprecated:` (see C8c). (user)

**C8c. Deprecation is three-tiered.** The owner asked whether deprecated
functions could be "skipped entirely". Skipping compilation would break every
existing caller at once, which defeats the purpose of a grace period, so the
decision is to skip *discovery* instead. A definition with a `deprecated:`
clause is (1) removed from `renyi index`, LSP completion, tool export and the
main documentation listing, so agents never pick it up for new code; (2) still
compiled for existing callers, with a warning that `renyi check --strict`
turns into an error, and an outright "delete me" hint once it has zero
callers; (3) automatically migratable: `deprecated: since 2.0, replaced by
new_name` lets `renyi migrate` rewrite call sites mechanically when the
replacement's signature is compatible. Removing a public definition bumps the
major version automatically (see G1). (derived, accepted)

**C9. Anonymous functions.** None. Every function has a name and, if public, a
purpose. The query form `for each ... where ... collect` covers map and filter;
`sorted by field`, `group by field`, `sum`, `count`, `first`, `any`, `all`
cover the remaining common higher-order uses; callbacks are passed as named
functions. (user)

---

## D. Constraints that serve LLM readers and writers

**D1. Cheat-sheet token budget.** The complete syntax cheat sheet must fit in
3000 tokens. It is measured in CI after every grammar change; exceeding the
budget blocks the change until a feature is removed or compressed. (user)

**D2. Cold start.** The spec and cheat sheet travel in the prompt, the grammar
skeleton stays close enough to mainstream languages for transfer, and the
compiler's fix suggestions close the loop. A fine-tuning corpus is a later
option, not a dependency. (user)

**D3. Error output.** Every diagnostic is emitted both as human-readable text
and as JSON (`renyi check --json`), with location, cause, and a suggested fix.
(user)

**D4. Banned in v1.** Operator overloading, macros and metaprogramming,
implicit type conversions, wildcard imports, variable shadowing, single-letter
identifiers, nesting deeper than four levels, and function bodies longer than
about 60 lines. Each is a compile error. The depth and length limits exist so
that every definition fits in one RAG chunk and one screen. (user)

**D5. Operational definition of "retrievable code", plus content addressing.**
Every definition carries a natural-language purpose; there are no wildcard
imports and no implicit context, so a chunk is understandable in isolation;
signatures and effect declarations are complete; there is no overloading and
no macro expansion, so the text is the semantics; `renyi index` emits one
record per definition with a stable identifier, signature, purpose, tags and
effects. In addition the owner chose **content addressing**: every definition
is identified by the hash of its canonical AST with dependency names replaced
by their hashes (the Unison model), so renames do not change identity, the
index deduplicates across copies, and the lockfile pins dependencies by hash.
v1 computes hashes in the toolchain; language semantics do not depend on them.
(user)

**D6. Agent tools are first-class.** An `expose as tool` clause on a function
publishes it as an agent-callable tool. Parameter types produce the JSON
Schema, `purpose:` produces the description, `needs` produces the permission
list. `renyi tools` lists them and `renyi serve --mcp` serves them. (user)

---

## E. Runtime

**E1. Concurrency.** Structured concurrency on green threads. `run concurrently
... end` runs the statements inside as parallel tasks and waits for all of
them; the first failure cancels the rest and propagates. `for each ... in ...
concurrently collect ...` is the parallel query. There is no `async`/`await`,
no bare thread, and shared mutable state is impossible by construction. (user)

**E2. Memory.** Tracing garbage collector. Ownership and lifetimes never
appear in the surface language or in error messages. The VM may mutate in
place when a value is uniquely referenced; this is invisible to users. (user)

**E3. Sandbox.** The runtime enforces declared capabilities. `renyi run` grants
exactly what the program's `main` declares and nothing more; `--deny network`
style flags narrow the grant further. (user)

---

## F. Interoperability

**F0. FFI posture.** First-class with boundary validation. Bindings are
generated from C headers, Python type hints and TypeScript declarations,
and read like Renyi functions. Return values are validated against the
declared types when they cross the boundary; a mismatch fails through the
ordinary error channel. (user, see T3)

**F1. Compatibility scope for v1.** Consuming JSON, HTTP, OpenAPI and databases
(types generated from OpenAPI and JSON Schema); semantics close enough to
Python and TypeScript that existing code translates mechanically; embedding
Renyi as a scripting engine. Calling foreign libraries through FFI follows the
posture in F0 but is scheduled after the v1 core (see H3). Transpiling Renyi
to other languages is not planned. (user)

**F2. "API friendly" priority.** Calling external APIs first: HTTP and JSON are
first-class, types are generated from OpenAPI. The language's own API
(compiler as a library, AST as JSON, LSP) follows immediately at M5. (user)

**F3. Host ecosystems.** Python, Rust, JavaScript/TypeScript and C, all four.
Implementation order: C ABI first because the other three build on it. (user)

---

## G. Modules, packages, standard library

**G1. Versioning.** Built-in package manager with a central registry and a
lockfile. Semantic versions are decided by the compiler, Elm-style: removing or
changing a public signature forces a major bump, adding one forces a minor
bump. Combined with content addressing, a version number cannot lie. (user)

**G2. Standard library.** Small core (base types, collections, text, time)
plus official extension packages (HTTP, JSON, filesystem, regex, CSV, SQLite)
maintained in the same repository and released in lockstep. Imported on
demand. (user)

**G3. Files and modules.** One file is one module and the module name equals
the path. The `module` header must match the path. `.renyi` and `.ry` are
fully equivalent. (user)

---

## H. Implementation

**H1. Implementation language: Rust, one binary.** The owner asked about
Haskell for the compiler and why the runtime needs a language choice at all.

Why the runtime needs a systems language: the runtime is the bytecode
interpreter, the garbage collector, the green-thread scheduler, the capability
sandbox, the native parts of the standard library, the FFI bridge and the
embedding C API. It runs *underneath* Renyi programs and cannot be written in
Renyi, which by design has no manual memory management and no escape hatch.
The platform decisions (30 ms startup, embedding, WASI and browser WASM as
first-class targets) require a language without its own GC runtime, with a C
ABI and mature WASM support: Rust, C or Zig. Rust is the only one with algebraic
data types and pattern matching.

Why not Haskell for the compiler: it is entirely feasible (Elm and PureScript
prove it) but costs two toolchains in one product, makes "compiler as a
library" and embedding require linking the GHC runtime into Python, JS or Rust
hosts, has an experimental WASM backend and painful Windows cross-compilation,
and is not installed in the development sandbox while Rust 1.97 is. The parts
of Haskell that matter (algebraic types, pattern matching, type classes,
Option/Result) exist in Rust; the parts that do not (GC-backed AST sharing,
laziness) are replaced by arenas and indices at roughly 1.3 to 1.5 times the
code. Gleam, Roc and rustc are written this way. Haskell's heritage goes into
Renyi's type system, not its implementation language. Self-hosting the compiler
in Renyi is the long-term path. (user confirmed Rust)

**H1b. Who writes it.** Primarily Claude, in sessions. Work is sliced into
units a session can finish; every unit ships with tests and a design note.
The owner makes design decisions and reviews. (user)

**H2. Order of work.** Example corpus and cheat sheet first, then an LLM
readability test (several models predict outputs and complete code from the
cheat sheet; correctness rates are recorded), then the parser. The grammar is
frozen by the readability test, not by the implementation. (user)

**H3. First demonstrable milestone: M3.** Parser, type checker and bytecode VM
running the full example corpus. Milestones: M0 design documents and corpus;
M1 lexer, parser, formatter; M2 type and effect checker with diagnostics; M3
bytecode VM; M4 standard library and package manager; M5 LSP, index, compiler
API; M6 AOT and WASM targets; FFI generators follow M4. (user)

**H4. License.** Apache-2.0. (user)

**H5. Documentation language.** All English, including design discussion in
this directory. (user)

**H6. Testing from M1 on.** A specification conformance suite (`.ry` inputs with
expected outputs, implementation-independent, reused for self-hosting); golden
tests for parse trees, formatter output and diagnostics; property tests
(formatter idempotence, parse/print round trip, type-checked programs never
crash the VM); and an LLM readability regression that reruns the model
prediction test and the cheat-sheet token count as a CI gate. (user)

---

## I. Naming and clarifications

**I1. The name.** "Renyi" is the owner's online handle. Spelling is fixed as
`Renyi`, no accent. The collision with "Rényi entropy" in search is handled by
documentation phrasing ("Renyi programming language", "Renyi lang") and does
not constrain the design. (user)

**I2. "Keyword handling" means all four.** Reserved words versus identifiers
(C4a), lexing of multi-word keywords (C4a), a cap on the total number of
keywords tied to the 3000-token cheat-sheet budget (D1), and searchable
keywords in code (C8b `tags:` and `purpose:`). (user)

**I3. "Format optimization" means two things.** Layout rules chosen for LLM
tokenization efficiency and attention locality, not only for human aesthetics;
and agent-friendly formats for everything the toolchain emits (diagnostics,
index, documentation). (user)

**I4. Influences.** Adopted: Elm (diagnostics, compiler-enforced versions, no
runtime exceptions), Gleam (small static type system without type classes'
complexity), Roc and Koka (effects in types, platform/application split),
Unison (content addressing, documentation as a first-class citizen). Explicitly
avoided: Haskell's obscure surface (custom operators, point-free style,
transformer stacks, extension maze) and Python's dynamism and significant
indentation. (user)
