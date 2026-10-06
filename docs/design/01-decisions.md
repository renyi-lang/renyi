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

---

## J. Round 2 decisions (session 2)

Round 2 settled the open items R2-1 to R2-17 of the syntax sketch. Each entry
names the item it closes. The sketch was updated in the same commit.

**J1. Named types are upward-only subtypes (R2-11).** `type X is Base` makes
`X` a subtype of `Base`: an `X` is accepted wherever a `Base` is expected and
keeps its methods; a `Base` becomes an `X` only through construction, which is
fallible when `X` is refined. This is the one subtyping relation in the type
system. Rejected: fully distinct newtypes (every arithmetic step unwraps and
rewraps), plain aliases (no safety). (user)

**J2. Map literals use JSON braces (R2-1).** `{"a": 1, "b": 2}` and `{}`. The
notation is universal and matches the data the programs consume; a word form
would be long and error-prone. (user)

**J3. Documentation clause words are reserved (R2-2).** `purpose`, `tags`,
`example`, `see`, `also`, `deprecated` stay in the reserved list. One word,
one role; the compiler suggests renames (`tags` to `labels`). (user)

**J4. No contracts on parameters (R2-13).** Constraints live on named types
(`type Attempts is Integer where value is at least 1`) and are checked at
construction. Parameter signatures stay plain and call sites do not sprout
`otherwise`. (user)

**J5. Interpolation everywhere, plus `raw` (R2-14).** Every `"..."` and
`"""..."""` literal interpolates `{expression}`; a literal brace is `\{`; a
hole may not contain a string literal. `raw "..."` is a literal with no holes
and no escapes, for regular expressions and for JSON or SQL samples. One new
reserved word. Rejected: `\{` alone (regular expressions become unreadable),
opt-in interpolation (the common case gets longer and the prefix gets
forgotten). (user)

**J6. `if` and `match` are expressions (R2-12).** Allowed where a value is
expected when every branch is a single expression or a way out (`fail`,
`fail with`, `return`, `crash with`); an `if` expression needs `otherwise`.
Six of the thirty corpus programs use the form where other languages use a
ternary operator. (user)

**J7. No list patterns in v1 (R2-3).** `is_empty()`, `first()` and `rest()`
cover the corpus. Revisit after the readability test if recursion over lists
turns out to be common. (user)

**J8. Bare variant patterns; unused bindings are errors (R2-16).**
`when Circle then` matches a variant without binding its fields. Any binding
that is never read, whether from `let`, a loop header, a parameter or a
pattern, is a compile error with the fix "remove it". Patterns therefore name
exactly what the branch uses, and no wildcard is needed. (user)

**J9. `/` needs Decimal or Float operands (R2-4).** Dividing two Integers is a
compile error that points to `a.quotient(b)` (integer division) or
`a.to_decimal() / b`. No operation changes the type of its operands. The
corpus never divided two Integers, so the rule costs nothing. Supersedes the
sketch's earlier "Integer / Integer yields Decimal". (user)

**J10. No default method bodies in v1 (R2-5).** Every implementation block
spells out every method, so a retrieved implementation is complete on its
own. Revisit if the corpus shows repeated identical implementations. (user)

**J11. Scoped capabilities are in v1 (R2-6).** A capability may carry one
literal argument naming its scope: a path prefix for `filesystem` and its
children, a host for `network` and its children, a variable name for
`environment`, a program name for `process`. A declaration without an
argument covers every scope. The checker requires each caller to cover its
callees: the same or an ancestor capability, with no argument or with one
that contains the callee's. At run time a primitive compares the actual path
or host with the calling function's declared scope and reports a mismatch
through its ordinary error type (`PermissionDenied(path)`,
`HostNotAllowed(host)`). `renyi run` can narrow scopes further from the
command line. Claude recommended deferring this to keep the checker simple;
the owner chose precision in the source. (user)

**J12. Deadlines on concurrency with `within` (R2-7).** `run concurrently
within duration` and `for each ... concurrently within duration ...` cancel
the remaining tasks when the duration expires and fail with the built-in
`TimedOut`, which the enclosing function lists in `or fails with`. Durations
come from `std.time` (`time.seconds(5)`). One new reserved word. (user)

**J13. No positional exception for commutative functions (R2-8).** Two or
more arguments are always named. The standard library expresses the common
cases as single-argument methods: `width.at_least(minimum)`,
`width.at_most(limit)`, `items.largest()`, `items.smallest()`. (user)

**J14. External field names with `as` (R2-9).** `has kind: Text as "type"`
gives a field the name that `ToJson`, `FromJson` and `FromRow` use; it is the
only way to map keys that are reserved words or contain punctuation. A
convention-wide mapping is a decoder option:
`json.parse(text: text, naming: CamelCase)`. `as` is already reserved. (user)

**J15. Unused results are errors; `ignore` discards (R2-15).** A call whose
non-empty result is not used is a compile error whose message proposes the
likely fix (`set items to items.append(item)`). `ignore expression` discards
a result on purpose. One new reserved word. (user)

**J16. Decimal is IEEE 754 decimal128 (R2-17).** 34 significant digits;
literals, sums and products of everyday values are exact; division rounds
half-even at the 34th digit; overflow crashes like any arithmetic domain
error. `Float` is IEEE 754 binary64. Rejected: exact rationals (unbounded
denominators, and printing needs a rounding rule anyway), fixed scale
(truncates rates and scientific values). (user)

**J17. Single-file module names (R2-10).** A `.ry` file run outside a project
is the module named after its file stem, and its header must say so. The
rule "module name equals path" holds with the file's directory as the project
root. (derived)

**J18. Reserved words after round 2: 84.** `ignore`, `raw` and `within` join
the 81 words of C4a. The phrase table is unchanged. (derived)

---

## K. Standard library round (session 2)

Asked while writing `04-stdlib-sketch.md`; answers shape every module.

**K1. Methods are functions with `self: Type` (S1).** A function whose first
parameter is `self: T` is a method of `T`, called `value.name(...)`, and must
be declared in the module that defines `T`. Base-type methods (`text.trim()`)
and standard-library methods (`connection.query(...)`) follow this one rule;
abilities exist for polymorphism, not for attaching methods. Rejected: one
single-implementation ability per type (ceremony, longer retrieval results),
module functions only (no chaining). (user)

**K2. Non-2xx HTTP statuses are failures (S2).** `http.get(url) otherwise fail`
leaves a 2xx response on the success path; other statuses fail with
`HttpError.Status(url, status, body)`. The classic forgotten status check
cannot happen; a program that expects a 404 matches the failure. (user)

**K3. `matches` in the prelude, the rest in `std.regex` (S3).**
`text.matches(pattern)` needs no import, so refined text types stay short;
capture groups, find-all, replace and split live in `std.regex` on a `Pattern`
type. The engine is in the runtime; literal patterns are checked at compile
time. (user)

**K4. Minimal `Bytes` in v1 (S4).** `Bytes` is a prelude type with `length`,
`is_empty`, `to_text` (fallible UTF-8 decode) and `to_base64`;
`text.to_bytes()`, `filesystem.read_bytes`, `filesystem.write_bytes`, and
`Response.bytes` use it. (user)

**K5. `json.parse_with` clarifies J14.** There is no overloading and there are
no default arguments, so the convention-mapping decoder is a separate
function: `json.parse_with(text:, naming:)`, `json.render_with(value:,
naming:)`. (derived)

**K6. Record patterns.** A record type is matched like a single variant:
`when InvalidDate(input) then`. Error types such as `InvalidDate` and `GaveUp`
are records, so `match` and `example: ... fails with ...` need this. (derived)

**K7. `Self` in ability declarations.** Inside an `ability` declaration `Self`
names the implementing type, for binary methods such as `compare(self, other:
Self)`. It is a predefined type name like `Integer`, not a reserved word.
(derived)

**K8. Pairs.** `Pair of Left, Right` is a prelude record with fields `left` and
`right`. A two-variable loop or query header destructures a pair;
`map.entries()` and `items.with_index()` return lists of pairs, and iterating
a map yields its entries. (derived)

**K9. Maps and sets keep insertion order (R3-1).** Iteration, `keys()`,
`values()` and `entries()` follow insertion order, so output is deterministic
and examples can compare whole maps. Python and JavaScript behave the same
way, which is what generated code assumes. (user)

**K10. Variants encode as a flat object with a `kind` key (R3-2).**
`Circle(radius: 2.5)` is `{"kind": "Circle", "radius": 2.5}` and `Point` is
`{"kind": "Point"}`; a variant with a field named `kind` is a compile error
when the type derives `ToJson` or `FromJson`. This is the discriminated-union
shape most APIs use. Supersedes the externally tagged draft in the first
version of the library sketch. (user)

**K11. `console.print` takes only `Text` (R3-3).** Other values are printed
through interpolation, `console.print("{total}")`. No generic entry point
converts to text implicitly. (user)

**K12. Float renders with a decimal point.** `Float.to_text()` is the shortest
text that reads back as the same value and always contains a decimal point
(`5.0`), so a `Float` is never mistaken for an `Integer` in output. `Decimal`
keeps the digits of its exponent instead (`6.00` after `rounded(2)`).
(derived)

---

## L. Readability test round (session 2)

**L1. Models and credentials.** The first live round uses Anthropic and OpenAI
models, at least three in total including the smallest model the project
intends to support (`claude-haiku-4-5`). Credentials are environment variables
of the cloud environment (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`), never part of
the repository or the conversation. (user)

**L2. Who judges Complete and Write before M3.** Claude judges each lint-clean
sample in the session and records the verdict with a one-sentence reason in
`judgement.json`; the owner spot-checks. The judge shares its origin with some
of the tested models, so every verdict is re-run by the VM at M3 and the two
sets of results are compared. (user)

---

## M. Pre-test round decisions (session 4)

The readability pre-test (`tests/readability/2026-10-05-e41258c/notes.md`)
raised these questions; the owner answered them in one batch.

**M1. `group by` combines with every terminal clause.** `for each sale in
sales group by sale.region sum sale.amount` is a `Map of Text to Decimal`:
the terminal after `group by` is applied to each group, so `collect`, `sum`,
`count`, `first`, `any` and `all` all work per group, and a bare `group by`
keeps whole items as before. Both models of the pre-test wrote this form for
the same program and the parser already accepted it. (user)

**M2. The loop variable of a bare `count` query is unused.** In `for each
line in order.lines count` the variable `line` is never read, so decision J8
applies and the fix is `order.lines.length()`; one concept keeps one spelling.
The variable of a `first` query counts as read because `first` returns it.
(user)

**M3. `otherwise` on a value that cannot fail is an error.** `otherwise` is
allowed only after a `maybe` value or a fallible call. `Port(8080) otherwise
crash with ...` (a literal checked at compile time) and `line.split(" ")
otherwise fail` are compile errors whose fix is "remove `otherwise`: this
cannot fail". (user)

**M4. A named single argument is an error.** `column_widths(table: table)`
is rejected with the fix "drop the name": one argument is positional, two or
more are named, and the formatter never changes tokens. The same holds for a
construction with one field (`UserId(id: 7)`). (user)

**M5. The readability scoring formats before it lints.** `renyi format` runs
on every Complete and Write program before the lint, since an agent runs the
formatter anyway; layout violations are still counted and reported per rule
but no longer fail a sample. `03-readability-test.md` is updated. (user)

**M6. The cheat sheet gains a library section.** Names only (prelude text,
list, map and set methods; the standard modules), within the 3000-token
budget. Invented library names were the largest cause of Sonnet failures.
(user)

**M7. Haiku 4.5 stays the floor model (L1).** Its pre-test rates (Complete
37%, Write 0%) are from one sample per item; the cheat sheet is iterated on
the failure causes first, and the live round with five samples decides.
(user)

**M8. An MCP server for the toolchain, after M2.** A server that offers the
cheat sheet, a library-name lookup, `check` and `format` (later `run` and
`test`) to any agent host is scheduled after the type checker, when `check`
has substance; whether it reuses the `renyi serve --mcp` command of decision
D6 or gets its own subcommand, and whether a JSON dependency is taken, is
decided then. (user)

**M9. `repeat until` replaces `while`.** The only condition loop is
`repeat until condition` ... `end`: the condition is tested before each pass
and names the state that holds when the loop is over, so the reader gets the
postcondition without negating anything (`repeat until remaining.is_empty()`
instead of `while not remaining.is_empty()`). `repeat until` is one phrase
token; `repeat` and `until` are reserved and `while` leaves the list, which
has 85 words (supersedes the count in J18). `while` becomes a foreign
keyword: the compiler rejects it and suggests `repeat until` with the
opposite condition. The owner asked for a simpler, fixed loop shape;
`for each` is unchanged. (user)

**M10. No bottom-tested loop.** The "do, then test" case (pagination) keeps
its flag variable or uses `break`; the corpus has one such loop. Revisit after
the live readability round. (user)

---

## N. Decisions forced by the type checker (session 4, M2)

Derived while turning the library sketch into declaration files the compiler
reads (`library/std/*.ry`) and writing the checker; each follows from an
earlier decision and is recorded so that it can be questioned.

**N1. A method may be named with a reserved word.** The library declares
`first`, `at`, `set`, `sum`, `sorted` and `repeat` as methods, and the
corpus calls them after a dot, where any word is allowed (C4a). A function
whose first parameter is `self` is only ever called after a dot, so the
parser accepts any word as its name; an ordinary function keeps the rule.
Parameters are bound as plain names and keep the rule too, so the library's
`count` parameters became `length`, `days` and `amount`. (derived)

**N2. `Path`, `Url` and `Pattern` are plain subtypes of `Text`.** Their
validity is checked by the operations that use them (a missing or malformed
path fails with `FileError`, a malformed URL with `Unreachable`) and, for
literals, at compile time by the toolchain. Constructing them from runtime
text therefore needs no `otherwise`, which is how the corpus already writes
`Url("{base}?{query}")` and `Path(path_text)`; the sketch's `where value is
not ""` on `Path` is dropped. (derived)

**N3. Library error types derive `ToText`.** The corpus logs errors with
`error.to_text()` in `failure` arms, and the cheat sheet shows the same, so
every error type the library declares carries `can ToText`; a program's own
error type adds the clause when it is rendered. `ToText` stays opt-in for
other types. (derived)

---

## O. Runtime and agent tooling (session 4)

Asked by the owner as three questions (how memory is managed, what sets the
language apart, what the language can offer LLM agents); the owner took the
recommendations as given. The design is in `05-agent-tooling.md`.

**O1. Memory is reference counting with in-place reuse; supersedes E2.**
Renyi's values are immutable, there are no references, no closures (a
function is passed by name and captures nothing), and no shared mutable
state, so the value graph is acyclic: a value cannot contain itself.
Acyclic data needs no tracing collector. The VM counts references, frees a
value when its count reaches zero (so a file or a connection closes as soon
as it is dropped, deterministically), and updates a value in place when its
count is one, which makes `set items to items.append(item)` an O(1) append
when nothing else holds the list (the Perceus/Koka and Roc model; the
compiler marks last uses so that arguments are moved, not copied). The v1
scheduler runs green threads on one OS thread, so counts are not atomic;
when tasks move to several threads, values that cross a task boundary get
atomic counts. The user-facing promise of E2 stands unchanged: ownership
and lifetimes never appear in the language or in a diagnostic. A future
`lazy` must keep the graph acyclic (a thunk may not refer to its own
binding). (user, on Claude's recommendation)

**O2. The project map is the agent's view of a project.** `renyi index`
(decision D5) emits one record per definition and one per module, as JSON:
identity (content hash), kind, signature, purpose, tags, declared and
transitive effects, declared and transitive failure types, the edges
(calls, type uses, implementations, tests), six metrics (lines, nesting
depth, branch points, effects, fan-in and fan-out, example and test
coverage) and the source location. Every number is exact because the
language has no macros, no overloading and no dynamic dispatch outside
abilities. The map is incremental by hash and is what an agent reads
instead of files; granularity is module to definition, with tests as nodes
of their own. (user)

**O3. Complexity budgets.** The per-definition limits of D4 (nesting depth
4, about 60 lines) stay compile errors. Module-level budgets (public
definitions per module, size of the transitive effect set, fan-out) are
reported by the map and gated in CI as warnings first; thresholds are set
once the map has been measured on the corpus (open item R5-1). (user)

**O4. Effect and semantic diffs.** `renyi index --diff` compares two maps
and reports changed signatures, effects and failure types with the callers
each change reaches; the version-bump rule of G1 reads the same diff, and
an agent checks that its change did not widen any grant. Scheduled after
M3. (user)

**O5. The toolchain MCP server is `renyi mcp`.** Decision M8's server gets
its own subcommand, distinct from `renyi serve --mcp` (D6), which serves a
program's own `expose as tool` functions. `renyi mcp` speaks JSON-RPC over
stdio and offers `project_map`, `definition`, `effects`, `check`,
`format`, `cheat_sheet`, `library_lookup`, and after M3 `run`, `run_tests`
and `diff`. The checker's `World` stays resident and is refreshed by
content hash. `serde_json` is taken as a dependency: the zero-dependency
rule ended when `regex` entered the checker, and a hand-written parser
would buy nothing. (derived)

---

## P. Signature capabilities (session 4)

The owner asked for at least one capability no other language has had, new,
practical and elegant, and chose three of the four candidates. Each rests on
the effect system: every effect goes through a declared capability and a
small set of library primitives, so the runtime sees all of them. Design:
`06-runtime-guarantees.md`.

**P1. Recorded runs.** `renyi record program.ry` runs a program under its
grant and writes every effect call (capability, primitive, arguments, result
or failure, clock and random values) to a recording. A test declared with
`replays "path"` answers its effects from the recording, deterministically
and offline; a call the recording lacks fails the test. `renyi run --replay`
re-executes a recorded run for debugging, and `--explain` narrates any run
with the `purpose:` clauses of the definitions it passes through, the data
they produced and the effects they used (the self-narrating layer). The
effect system guarantees the recording is complete; the ToJson rules give
it a readable format that the replay validates against the declared types.
One new reserved word, `replays`. Nearest prior art: VCR-style HTTP
recording libraries and Darklang's trace-driven development, neither a
language feature with a completeness guarantee. Scheduled in M3. (user)

**P2. Budgets in grants.** A capability in the grant of `main` or of a
test may carry `at most COUNT per UNIT` (units `second`, `minute`, `hour`,
`day`, `run`); the runtime counts primitive calls under it and fails the
call that exceeds the budget with the module's error type. Budgets apply to
`network`, `process` and `filesystem`. One new reserved word, `per`, and
the phrase `at most`. Scheduled in M3. (user)

**P3. Provenance guards.** A capability in the grant may carry `only to
SINK or SINK`: values that enter through it, and values computed from them,
carry that origin and may leave the program only through the listed sinks;
any other outgoing primitive fails with `Guarded(origin, sink)`. This is the
language-level answer to exfiltration through prompt injection, the central
risk of agent sandboxes. The runtime tracks origins as a small set per heap
value, which Renyi's immutable values and explicit effects make cheap.
One new reserved word, `only`, and the phrase `only to`. The syntax is in
the grammar now so that the cheat sheet, the parser and the live
readability round see it; the runtime part is designed first and scheduled
in M4. Prior art: information-flow languages (Jif, FlowCaml, LIO) and
runtime taint modes (Perl, Ruby); none ties the policy to the capability
grant. (user)

**P4. Reserved words after P1 to P3: 88, phrases 17.** `only`, `per`,
`replays` join the list (supersedes the count in M9); `at most` and `only
to` join the phrase table. Budget units are plain words after `per`, not
reserved. (derived)

---

## Q. System-level commitments (session 4)

The owner asked for system-level innovations that solve pain points of
mainstream languages outright and resolve their "cannot have both"
trade-offs; all four candidates were taken. Design: `07-system-design.md`,
whose section 1 states the trade-offs the language claims to resolve.

**Q1. Capability-safe packages.** A package's effect manifest is computed
by the toolchain at publish and recomputed by the registry; `main`'s grant
must cover the transitive effects of every dependency the program reaches,
with the package and capability named in the error; an update whose
effects widen is refused unless the grant covers them and the user accepts;
scopes narrow dynamically across package boundaries through the grant
stack (the effective grant is the intersection along the call chain, which
every primitive checks); native code is visible as `foreign`. Supersedes
the wording of J11 for cross-package calls; within a program the static
coverage rule stands. M3 (grant stack), M4 (packages). (user)

**Q2. Reproducibility by construction.** A run manifest names everything a
run depends on (toolchain, code and dependency hashes, grant, arguments,
environment variables read, recording); `renyi reproduce` replays it and
compares byte for byte; builds are functions of the same hashes. Rests on
P1, K9 and D5. M3 and M4. (user)

**Q3. In-process sandboxing.** The effect grant is the isolation boundary:
a host loads a module with a grant (capabilities, scopes, budgets, guards,
a memory budget) and the module can reach nothing else, with no process or
container; several modules with different grants share one process through
the grant stack on the task tree. After Q1; embedding API at M5. (user)

**Q4. Checked live update.** A running service swaps the changed
definitions of a type-checked new version between requests, by content
hash, after the semantic diff shows no public signature a running
definition calls has changed; there is no state to migrate because the
language has no hidden mutable state. M5. (user)

---

## R. Decisions from the first live readability round (session 5)

The round (`tests/readability/2026-10-05-1623155/`, its `notes.md`) ran
Sonnet 5.5, Haiku 4.5 and gpt-5.4-mini with five samples per prompt. By the
four-of-five rule: Sonnet Predict 90, Explain 97, Complete 79, Write 40;
Haiku 30, 90, 37, 0; gpt 80, 97, 32, 0. Each measured cost below was put to
the owner as a question with the recommendation first.

**R1. The freeze is gated on the large models.** The acceptance thresholds
of `03-readability-test.md` apply to the current large model of each vendor
(Sonnet 5.5 and its successors, gpt-5.5 from the next round); the floor
model of M7 (Haiku 4.5) is run and reported as a trend and does not gate
the freeze. Haiku failed Predict at 30 percent through arithmetic and
attention errors (mis-added means, dropped parentheses, wrong column
widths), none of them a grammar misreading, so "every model" would never
freeze the grammar. Supersedes the wording "on every model" of the
protocol; M7 stands. (user)

**R2. J3 stands: reserved words are reserved everywhere.** The query words
(`count`, `sorted`, `first`, `sum`, ...) stay reserved outside queries too.
Measured cost: Sonnet loses two Complete items to `count` as a local name
(79 percent against 89 without the rule); Haiku and gpt lose more to
`sorted` and `first`. One word, one role, is worth it. (user)

**R3. `purpose:` stays required on public types and on the module.** It is
the largest single Write rule (28 percent of Sonnet's violations, over the
protocol's quarter; 25 percent of gpt's). The rule is kept because the
project map (O2) reads those purposes; the cheat sheet states it in one
explicit sentence and every public type in its examples carries one, since
the readers copy the examples. (user)

**R4. M4 stands: a named single argument is an error.** It is the largest
checker rule for Haiku (92 samples) and gpt (77). The uniformity (one
argument positional, two or more named) is kept over the pass rate. (user)

**R5. `is` on numbers.** Both operands must have one type: an `Integer` and
a `Decimal` do not compare (no implicit conversion, as the cheat sheet
says). A numeric literal takes the numeric type its context expects (an
Integer literal fits `Decimal` and `Float`, a Decimal literal fits `Float`),
so `total is 0` with `total: Decimal` compares two Decimals. Two Decimals
compare by numeric value; the scale is not significant: `32.0 is 32.00` is
true, and so is `0.00 is 0` through the literal rule. This is what the
judges of the round, the checker's refinement evaluation and IEEE
decimal128 comparison already do. (user; the value reading derived from
J16)

**R6. `ignore` discards only the result of a call with effects.** Supersedes
the wording of J15: `ignore call` is allowed when the call needs a
capability (the callee's `needs`, or those of a function passed to it);
discarding the result of a pure call is a compile error, `ignore-pure`,
whose fix proposes `set x to x.method(...)` for a method on a mutable
binding and otherwise "bind the result with `let`, or remove the call".
Three judged samples of the round wrote `ignore pop(stack)` on an immutable
value and so never shrank the stack; under this rule the checker catches
it. The error for a call returning nothing (`ignore-nothing`) stays. (user)

**R7. Budget thresholds (R5-1).** The first defaults of `renyi index
--budgets` are 10 public definitions per module, 5 transitive effect paths
per module and fan-out 7 per definition, each just above the corpus
maximum; a value over budget is reported, never a compile error (O3). The
thresholds move to the project manifest with the package manager (M4).
(user)

**R8. A fourth model from the next round.** `gpt-5.5` rejects temperature 0
and was not run; it joins the next round at its default temperature, as
Sonnet 5.5 ran in this one, so that each vendor has a large model gating
the freeze (R1). (user)

## S. Runtime dependencies and concurrency (session 6, M3)

Each question was put to the owner with the recommendation first, after a
probe crate had built the candidates on the owner's machine (GNU toolchain,
31 seconds, all three working).

**S1. Three dependencies for the runtime.** The HTTP client of `std.http`
is the `ureq` crate (version 3, rustls with the `ring` provider: no system
TLS, no C code for the client); `std.sqlite` is `rusqlite` with its
`bundled` feature (the SQLite sources compiled into the binary, which needs
a C compiler on the build machine); `std.server` is written over
`std::net::TcpListener` (HTTP/1.1, one request at a time, no TLS, which a
local service or one behind a proxy does not need). Redirects are followed
by the runtime one hop at a time so that every host a request reaches is
checked against the grant. Alternatives declined: a hand-written client
(no TLS, so the corpus could only run from recordings), `reqwest` (tokio),
`tiny_http`, and leaving any of the three unbuilt. (user)

**S2. Tasks run one after the other.** `run concurrently` and
`concurrently` queries execute their tasks in source order on one thread,
with the `within` deadline checked between statements and items. The
meaning of a program is unchanged: tasks are independent, so their order is
unobservable except through time, and values stay single-threaded
reference counts (O1). Overlapping I/O waits (green threads on one OS
thread) is a later runtime improvement, not a language change; a thread
per task was declined because values would have to cross threads. (user)

**S3. Redaction by name.** `renyi record --redact NAME` (also `renyi test
--refresh`) replaces, in the recording, the value of an argument named
NAME, of an entry named NAME (ignoring case) inside a map argument such as
a header list, and of the environment variable NAME as `environment.get`
returns it, with the placeholder `<redacted>`; a replay matches the
placeholder against any value, so a recording made with a real key replays
with any key. Redaction is by name, not by data flow: a secret that
reaches another argument is recorded as it is (provenance guards, P3, are
the data-flow mechanism). Settles open item R6-3. (user)

**S4. Response bodies stay inline.** A recorded HTTP response keeps both
its text (`body`) and its bytes (`bytes`, base64) in the recording, with
no size limit and no side file; the corpus fixtures are small, and a
side-file format can be added when a recording needs one. Settles open
item R6-1. (user)

**S5. The run manifest lives in the recording.** `renyi record` writes
the manifest of decision Q2 into the header of the recording it makes:
the toolchain, the source path, the content hash of `main` (which covers
everything `main` reaches), the arguments, the environment variables read
with the hash of each value or the redaction placeholder, the outcome, and
the SHA-256 and length of the standard output. `renyi run --manifest`
prints the same header and writes no file. `renyi reproduce FILE
[program]` refuses when `main` hashes differently, warns when the
toolchain differs, replays with the console output written, and reports a
different outcome or output and any recorded call not reached. The owner
asked whether the manifest could live in the `.ry` file itself; declined
because a manifest describes one run of a program rather than the program,
every `record` would rewrite the source, and the grammar under measurement
would grow. A separate manifest file was declined to keep one file per
run. Until the registry (M4) the code is read from the path the manifest
names and checked by hash. (user)

**S6. No narration of query clauses.** `--explain` narrates a function's
entry with its arguments, its result, and each effect; a query inside it
(`for each ... where ... collect`) is not narrated as a sentence of its
own, since the function's result shows what the query produced and a line
per query would lengthen the trace. Settles open item R6-2. (user)

**S7. A budget is a number.** `at most COUNT per UNIT` takes a literal
count, not a constant: the budget sits in the signature of `main` or of a
test so that the reviewer reads the number there. Settles open item R6-4.
(user)

---

## T. The MCP server (session 6)

`renyi mcp` (decision O5) was built after M3; each question was put to the
owner with the recommendation first.

**T1. Both eras of the protocol.** The current revision of the Model
Context Protocol (2026-07-28) has no handshake: every request names its
version and the client's capabilities in `_meta`, and the server must
answer `server/discover`; the hosts in use today still open with the
`initialize` handshake of revision 2025-11-25 and earlier. `renyi mcp`
speaks both, as the specification's compatibility section allows a
"dual-era" server to: a request with a version in its `_meta` is answered
statelessly as the current revision says, and an `initialize` request
selects the handshake. Supporting only the handshake would leave the
server behind the specification; supporting only the current revision
would leave every host on this machine unable to connect. (user)

**T2. The VM's JSON reader serves the protocol.** Decision O5 took
`serde_json` as a dependency for the server; since then the VM has its own
JSON reader and writer (`std.json`, the recordings of decision S4), which
the server uses instead, so the toolchain keeps one JSON implementation
and no new dependency. Supersedes that sentence of O5; the rest of O5
stands. (user)

**T3. `run` and `run_tests` take what the command line takes.** Besides
the path and the arguments of section 7's table, the `run` tool accepts
the narrowing options of `renyi run` (`deny`, `allow_host`, `allow_read`,
`allow_write`, `at_most`), a `replay` recording and `explain`, and
`run_tests` accepts `strict` and `explain`, so that an agent can sandbox
or replay what it runs exactly as a person can. (user)

**T4. The map is refreshed by file, not by definition.** Section 7 of
`05-agent-tooling.md` said the server refreshes only the definitions whose
content hash changed; the server rebuilds the whole map when any file of
the served directory differs from the one the last map was built from.
The result is the same, and the corpus (30 modules) rebuilds in about a
second in a debug build; the per-definition refresh is open item R5-5,
for projects large enough to need it. (user)

**T5. The diff reads a definition's own text hash beside its content
hash.** Section 6 of `05-agent-tooling.md` reported a body change from the
content hash; the content hash of decision D5 also changes when anything
the definition depends on changes, so a one-line change in a helper would
read as a body change in every caller, and the origin of a change would be
lost. Each record now also carries `text_hash`, the hash of the
definition's own canonical text with its own name and every reference to a
project definition blanked (the first two edits of D5, the references
blanked rather than replaced by hashes, and without the dependency and
library material). The diff reports a body change where `text_hash` or the
edges changed and lists the callers under what the change reaches; a
definition is matched across the maps by its qualified name, and a name
gone whose content hash is back under another name is one rename entry,
its callers untouched, as D5 intended. A map written before this field
falls back to the content hash. The base of a diff is a map file or a git
revision; a revision's files are read with `git show` and indexed afresh
with the same toolchain, so that the diff never needs a map to have been
saved. The version bump of G1 is derived from the entries: a public
definition removed, its signature changed (the `needs` and `or fails with`
clauses included), made private or renamed is a major bump; one added or
made public a minor bump. (derived)

## U. Readability round 2 (session 6)

Round 2 (`tests/readability/2026-10-05-ed37120/notes.md`) raised four
questions; each was put to the owner with the recommendation first.

**U1. M4 stands; the cheat sheet says it in a sentence.** "A call with one
argument does not name it" (decision M4) was again the largest checker rule
for all four models of the round (43, 50, 90 and 68 error lines for Sonnet
5.5, gpt-5.5, Haiku 4.5 and gpt-5.4-mini) and cost gpt-5.5 the Write item
`fizz_words` outright. The recommendation was to let a single argument be
named or not; the owner keeps the rule, as after round 1, and the cheat
sheet states it in a sentence beside the call examples ("One argument is
never named: `web.get(url)`, not `web.get(url: url)`") instead of a comment.
Measured again in the next round. (user)

**U2. `rounded` keeps its `places`; the cheat sheet shows the call.** Both
gating models wrote `value.rounded()` and `decimal.to_decimal()` and lost
`invoice` and `shapes` (Complete) and `compound_interest` (Write) to it;
gpt-5.5 would pass Complete without those two items. The recommendation
was a library change (`places` defaulting to 0, `to_decimal` an identity on
`Decimal`); the owner keeps the library as it is and has the cheat sheet
list the method as `rounded(places: 2)`, so that the argument is seen
before it is needed. (user)

**U3. Every method call takes parentheses; the cheat sheet lists methods
so.** The models wrote `text.length`, `items.is_empty` and `items.last` as
fields (21, 100 and 20 error lines for gpt-5.5, Haiku and gpt-5.4-mini)
because the library section of the cheat sheet listed the method names
bare. The owner asked whether a form without parentheses should be spelled
`length of items`; that would be a second spelling of one call, against
the one-spelling rule of the cheat sheet's first line, and no model had
written it, so the grammar is unchanged and the cheat sheet lists every
library method with its parentheses (`length()`, `is_empty()`, ...). (user)

**U4. Explain is graded by Claude Code subagents; round 1 is re-graded the
same way.** Round 2 graded Explain with subagents (Sonnet 5.5 first, Opus
5.5 second, fifty samples a batch, a reason kept with every grade) because
the owner ruled during the round that the pay-per-token API keys of
decision L1 are not spent by default and subscription quota is used
instead. Measured against the API grades collected before the ruling, the
subagents are about one point more lenient and flip 30 of 204 floor-model
samples from fail to pass, none the other way. The protocol needs one
grader kind across rounds, so the subagents grade Explain from here on,
round 1 is re-graded with them so that the series is on one scale (its
API-graded tally is kept beside the new one), and the API graders are used
only when the owner authorizes the keys for a round. The harness now
snapshots the references into each run directory at `prepare`, after a
description went stale between the rounds. L1's credentials sentence
stands for the API path; this entry adds the default. (user)

**U5. A hand verdict extends to every sample that repeats the sentence it
called wrong.** The re-grade of round 1 (U4) showed the subagent graders
agreeing on a passing grade for three statements the hand verdicts call
wrong (Sonnet's `retry.3`, Haiku's `inventory_db.2` and
`shipping_rules.0`), and the adjudication rule of the protocol reaches
only samples the two graders disagree on by more than one point. From
here a hand verdict of 2 on a sentence applies to every sample, of any
model in any round, that repeats the sentence or makes the same statement
about the same program; the adjudicator searches the program's samples
for it and records a verdict that names the originating one. Applied to
both rounds in session 6, fourteen verdicts: in round 1 Haiku's
`inventory_db.1` to `.4`, `shipping_rules.0`, `invoice.2` and `.4` (where
a session 5 verdict of 4 had read "quantities and prices are
non-negative" as correct and is withdrawn, the quantity being at least
1), Sonnet's `shipping_rules.1` and `retry.3`; in round 2 Haiku's
`inventory_db.0` to `.4` ("items that have fallen below their reorder
threshold" for at or below). Haiku's Explain is 27 of 30 in round 1 and
26 of 30 in round 2 after it; no gating model's item moved. The
alternatives were to keep the rule (the floor model's number then
carries grades the hand would not give) and a hand pass over every
agreed grade (1050 samples, declined as not worth it while the gating
models are unaffected). (user)

**U6. The five-point rule applies to the gating models.** The protocol's
rule that a grammar change lowering any passing rate by more than five
points is reverted or explained did not exempt the floor model, and
Haiku 4.5's Explain fell between the rounds on the one scale (97 to 90
before U5, 90 to 87 after it) on wrong statements about `todo_cli`,
`invoice` and `inventory_db`, not on any changed rule. The floor model is
reported as a trend and does not gate (R1), and its reading errors would
trigger the rule every round. The rule now names the gating models; the
floor model's moves are reported and explained in the round's notes.
`03-readability-test.md` amended. (user)

**U7. Round 3's samples come through the subscription channels.** The
default of U4 (no pay-per-token keys unless the owner says so for the
round) applies to collecting samples too: the Claude models are sampled
by Claude Code subagents and the OpenAI models by the Codex CLI on the
ChatGPT subscription, each sample fed to the harness with `run --provider
file` (round 2's gpt-5.5 Write samples came that way). The cost is
comparability: a subagent cannot be sampled at temperature 0 and sees the
host's system prompt around the cheat sheet, so the floor model's rates
carry more noise than in rounds 1 and 2; the gating models already ran at
their default temperature. The owner may still authorize the keys for a
round, and then the API path of `run` is used as in rounds 1 and 2.
`03-readability-test.md` amended. (user)

**U8. The library list carries parameter names; a sentence beside it gives
the call form.** Round 3 (`tests/readability/2026-10-06-3e7c45a/notes.md`,
Sonnet 5.5 through the Claude Code CLI) measured the cheat sheet after U1
to U3 and found the rendering of U2 and U3 wrong: the list showed
`rounded(places: 2)`, a single argument named, which decision M4 forbids
in a call, and `split()` beside `length()`, which hides the separator.
Sonnet copied both as shown: `rounded(places: 2)` in every sample of
`invoice` and `shapes` (Complete) and `compound_interest` (Write), all
rejected by the checker ("a call with one argument does not name it"),
and `split()` in three of five `word_count` samples ("`split` takes 1
argument (separator), found 0"); its Complete fell from 89 to 68, with
`shapes` and `word_count` lost to the rendering and `deadlines` and
`invoice_report` to the reserved word `count`. The rendering was
Claude's reading of U2 and U3, not what they decide (the argument seen
before it is needed; every method a call): both stand, in this form. The
list now gives every method with its parameter names (`split(separator)`,
`rounded(places)`, `replace(old, new)`, `set(key, value)`), checked
against `library/std/prelude.ry`, and one sentence after it gives the
call form of M4: `line.split(",")`, `price.rounded(2)`,
`text.replace(old: "a", new: "b")`. Paid for by shortening the
phrase-token list to three examples, dropping the nesting and body
limits, the count of reserved words and "(explicit)" after `Float`; 2998
tokens. Round 3 is a defect round: its rates measure the defect, and
round 4 on the corrected sheet through the same channel (Sonnet 5.5
first; gpt-5.5 when the Codex quota or the owner's key authorization
gives it a channel) is the measurement of U1 to U3. `count` stays
reserved (R2); the owner waits for gpt-5.5's data before reopening it.
(user)

**U9. The cheat sheet says when a refined construction takes
`otherwise`.** Round 4 (`tests/readability/2026-10-06-c696747/notes.md`,
Sonnet 5.5 on the sheet after U8, through the CLI channel run in an empty
directory with no MCP server) shows the errors U1 to U3 and U8 addressed
gone: no named single argument, `rounded()` or field-form error in 345
samples, against 43 named-argument lines in round 2. Complete is 79
against round 2's 89 on four items whose slips no changed sentence
touches, and two of them are one gap: `Port(8080) otherwise crash with
...` on a literal the checker proves valid (`superfluous-otherwise`,
`config`) and `Done(position: number)` without `otherwise` after `if
number is less than 1 then return Help end` (`missing-otherwise`,
`todo_cli`); the sheet said only that refined construction can fail. It
now says "A literal the checker can evaluate needs no `otherwise`
(`Port(8080)`); a variable needs one even after a check", paid for by the
phrase-token sentence, a query comment and "`renyi run` enforces it";
2991 tokens. The checker is
unchanged (a flow-sensitive refinement, which would read the check, was
the alternative and is declined for now). The owner also ruled that the
notes' item-by-item explanation satisfies the five-point rule for the
fall of Complete without a decision entry, since no change caused it,
and that gpt-5.5 does not run on this sheet for now: the rounds continue
on Sonnet, and the freeze still needs the second gating model (R8).
(user)

## V. The gap audit and the road to self-hosting (session 6)

On 2026-10-06 the owner set the goal beyond the readability scores and
answered the questions of the gap audit (`docs/GAPS.md`) in two batches of
four, the recommendation first in each.

**V1. The goal is a self-hosted language with users; the grammar freezes
by decision.** The readability rounds did their work (the sheet and the
corpus are what the gating model reads at 90, 100, 79, 40); the aim from
here is a language that stands on its own: a front end written in Renyi
with the Rust VM as the runtime, a formal grammar and a reference, first
users among people who build agent workflows and among learners, and a
corpus that future models meet in their training data. The grammar is
frozen by a decision entry rather than by the thresholds of
`03-readability-test.md`, and that entry is written only after the
implementation does what the design documents say: stage 1 of
`docs/GAPS.md` section 7 (the defects, the promised checks, the four
parsed-but-empty features, the document corrections, CI and the
conformance suite) comes before the freeze, then the formal grammar and
the reference, then the self-hosted front end, then the milestones M4 to
M6. The repository stays private until the owner says otherwise.
Supersedes the freeze-by-measurement sentences of H2, R1, R8 and U9; the
readability harness remains a measurement the owner can ask for. (user)

**V2. Indentation carries no meaning, continuation lines included.** The
parser contradicted C2: a statement continued on the next line only when
that line was indented deeper than the line that started it, and a
documentation clause continued on deeper lines (sketch section 1 described
both). The owner chose to make the language indentation-insensitive
outright rather than record the exception. A statement or clause now ends
at the newline unless a bracket is open or the next non-blank line starts
with a continuation word (`where`, `sorted by`, `group by`, `collect`,
`sum`, `count`, `first`, `any`, `all`, `returns`, `or fails with`,
`needs`, `for any`, `with`, `and`, `or`, and inside an `example:` `is`
and `fails with`); the value after `be` or `to` may start on the next
line. `otherwise` is not a continuation word: at the start of a line it
opens the branch of an `if` or `match`, so the `otherwise` that guards a
value stays on the line of that value and the formatter breaks inside the
parentheses of the longer argument list, the call's or the fallback's,
instead (`otherwise-line` is the error for an `otherwise` that starts a
line where no branch can). `return` keeps its value on its line, since `return`
alone is a statement and the next line could start one. The free text of
`purpose:`, `tags:`, `see also:` and `deprecated:` runs to the end of its
line. The corpus was rewritten by the formatter (eleven `otherwise` lines
and three `return` lines joined); no documentation clause of the corpus
wrapped. Sketch sections 1, 13 and 16 amended. (user)

**V3. `Ordering` is `Less`, `Same`, `Greater`.** The library has said
`Same` since the declaration files were written, because `Equal` is the
name of the equality ability and a variant of that name would read as a
type test in `when Equal then`. C3b's table names the result of
`compare`; it is read with `Same` in place of `Equal`. Supersedes the
variant names of C3b. (user)

**V4. The phrase table lives in the syntax sketch.** C4a said the fixed
phrase table is printed in full in the cheat sheet; U8 and U9 shortened it
to three examples to pay for the library parameters and the
refined-construction sentence. The table's normative place is section 17
of `02-syntax-sketch.md`, and the language reference once it exists; the
cheat sheet shows examples. Amends C4a. (user)

**V5. The nesting and body limits are compile errors: depth 4, 60 lines.**
D4 and O3 promised them and the checker never enforced them. A statement
nested deeper than four blocks (an `if`, `match`, loop or `run
concurrently` body adds one; the function body is depth 0) is
`nesting-depth`, and a function body whose statements span more than 60
source lines is `body-length`, each with the fix "move the inner part
into a function". The corpus's deepest body is 3 and its longest 30
lines, so nothing in it changes. (user)

**V6. The four parsed-but-empty features.** `only to` gets its runtime
now, as section 3 of `06-runtime-guarantees.md` designs it (P3 had
scheduled it in M4). `deprecated:` gets tiers 1 and 2 of C8c now: a call
to a deprecated definition is a warning that `renyi check --strict` turns
into an error, and `renyi index` drops the definition; `renyi migrate`
(tier 3) waits for the package manager. `expose as tool` gets `renyi
tools`, which prints the manifest of D6 (JSON Schema from the
parameters, the description from `purpose:`, the permissions from
`needs`) and the checker rejects a parameter or return type that JSON
cannot represent; `renyi serve --mcp` waits for M5. `process` and
`foreign` leave the cheat sheet, and the checker rejects them in a
`needs` clause (`capability-unavailable`) until the package manager
brings `std.process` and the FFI (M4). Nothing in the frozen surface
parses and does nothing. (user)

**V7. Task independence is a checker rule.** E1 promised that a race
cannot be written, and S2 delivers it only because tasks run one after
the other. The checker now enforces independence: a task of `run
concurrently` may not `set` a mutable binding declared outside the block
and may not read a binding declared by another task of the same block;
the violation is `task-independence` with the fix "bind the result with
`let` inside the task, or run the statements in sequence". The body of a
`concurrently` query is one expression per item and is independent by
construction. Real concurrency (green threads) remains a runtime
improvement that the rule makes safe. (user)

**V8. CI and the conformance suite.** D1 and H6 named them; neither
existed. A GitHub Actions workflow runs `cargo fmt --check`, `cargo
clippy --all-targets`, `cargo test`, the cheat-sheet token gate and the
corpus lint on every push; the private repository's minutes are the
owner's. The conformance suite is `tests/conformance/`: Renyi programs
with their expected standard output, exit code and diagnostics, in files
a compiler written in Renyi can be tested against without the Rust
crates; the corpus's reference outputs are its first members. (user)
