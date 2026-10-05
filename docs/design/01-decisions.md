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
