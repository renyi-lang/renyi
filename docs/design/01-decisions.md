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

**V9. A passed function runs under the grant in force where it is
called.** B1 charges the needs of the function actually passed to the
call site that passes it, whatever the parameter's type lists, and a
higher-order function declares only its own effects. At run time the
grant stack of Q1 applies as everywhere else: the passed function runs
inside the frame of the function that calls it and is narrowed by that
function's `needs` (sketch section 3). A function value stored in a
collection and called later is charged nowhere statically and is refused
by the grant stack at run time (`docs/GAPS.md`, 1.11). (user)

**V10. `Iterable` is one method, `to_list`.** `public ability Iterable of
Item` declares `function to_list(self) returns List of Item`, without
`needs`: an implementation's method never declares effects
(`method-signature` otherwise). `for each` and the queries walk any type
that implements it, through the list `to_list` returns, after the
built-in collections; the item type is the implementation's type
argument (`ability Iterable of Card for Deck`), with the implementation's
`for any` parameters substituted. A cursor protocol (`next`) and
constraints with type arguments (`where Bag can Iterable of Item`, open
item R3-2) are not in v1. Supersedes the `Iterable` of the early
core-ability list, which was declared nowhere. (user)

**V11. The grammar is frozen.** The surface syntax is what
`02-syntax-sketch.md` and `docs/cheatsheet.md` describe at commit
`dc58bb3` (2026-10-06), with the corpus under `examples/` and the
conformance programs under `tests/conformance/programs/` as its
witnesses: the reserved words and phrases of sketch section 17, the
clause grammar of sections 2 to 15, the canonical layout of section 16,
and the core abilities of section 5 with `Iterable` (V10). From here a
change to the surface is a new decision entry that names what it
supersedes; it lands in the sketch, the cheat sheet (within its budget),
the formatter and the conformance suite in one commit, and the old
spelling gets the new one as its fix (C4). The open items of sketch
section 18 (R3-2) and `docs/GAPS.md` section 6 are not surface changes
until decided; library additions (`04-stdlib-sketch.md`) are not surface
changes. The formal grammar (EBNF) and the language reference are
derived from the frozen sketch in stage 2; what they find unclear is
settled by an entry, not by the implementation. (user)

**V12. Line breaks after commas and before `is` and `fails`; one spelling
per construct.** Three rulings the formal grammar required, all on the
line rule of sketch section 1. (1) A line break after a comma carries no
meaning wherever the comma stands: a `needs` list, a `for any` list, an
`exposing` list, a `can Compare by` list, a `with` update or the sources
of a query may continue on the next line, as the example of section 11
always showed; the formatter breaks a `needs` clause wider than the line
one capability per line, indented once more than the clause. (2) `is`
and `fails` are continuation words everywhere, not only inside an
`example:`: the continuation words are one list read without context,
which the grammar's token filter and a self-hosted lexer can state; the
formatter breaks before them only in an example. (3) The parser accepts
one spelling of each construct, since the formatter never changes a
token (section 16): it rejects, with a fix, a trailing comma in a
bracketed list, a variant with empty parentheses in a declaration or a
pattern, a space between a dot and the member after it, an `otherwise`
arm that is not the last of a `match` expression, a head and `end` on
one line, a hole in a test name, a capability scope, a recording's path
or an external name, and `public` on an ability's method
(`public-method`); and it accepts, as section 1 says, a line break
inside brackets before a dot or a call's parenthesis. The grammar
`docs/grammar.ebnf` and its test `crates/renyi_syntax/tests/grammar.rs`
state the surface under this entry. (user)

## W. The self-hosted front end (session 8)

**W1. The syntax tree's JSON is the derived JSON of the Renyi AST.**
`compiler/ast.ry` declares the syntax tree as Renyi types, one per
construct, each `can ToJson`; the document `renyi parse --json` prints is
their derived JSON: a record is an object whose keys are its fields in
declaration order, a variant is an object with `kind` first and its
fields after it, a `maybe` without a value is `null`, a list is an array,
and a span is `{start, stop}` in character offsets, as Renyi's text
indices count (the Rust encoder converts its byte offsets). The Rust
encoder (`crates/renyi_syntax/src/json.rs`) produces the same document,
so the two front ends can be compared byte for byte; the previous
document (`node` keys, line and column positions) is gone, in one
change. The names of the Renyi types keep the derived document
unambiguous: no two sum types in the compiler's scope share a variant
name (`IsValue` beside the operator `Is`, the lexer's `MemberName` beside
the expression `Member`), no field is a reserved word (`exposed` for
`exposing`), and `span` is the last field of every node. (user)

**W2. The front end written in Renyi lives in `compiler/`.** The modules
are `ast` (the tree), `lexer`, `parser`, `parse` (the command line:
`renyi run compiler/parse.ry [--declarations] <file>` prints the tree)
and `tokens` (the token dump, as `renyi tokens` prints it); they are
named by their file stems and import one another by those names, since
the directory of the main file is the project root (J17). They are Renyi
programs like any other: `renyi check` clean, in canonical layout
(`renyi format compiler/*.ry`), within the size limits of V5, run by the
Rust VM. The parser's `import ast exposing ...` names every node type,
wider than any line, so the formatter breaks an `exposing` list wider
than the line after its commas, one name per line indented once, as V12
breaks a `needs` clause (reference section 16). (user)

**W3. The judge is a Rust integration test.**
`crates/renyi/tests/selfhost.rs` runs the Renyi parser on the VM over the
corpus, the conformance programs, the compiler's own sources and, with
`--declarations`, the library declarations, and compares its output with
`renyi parse --json` byte for byte; a program the Rust parser rejects
must make the Renyi parser fail. It runs in `cargo test` and in CI, so
the two front ends cannot drift apart. `renyi parse --declarations`
exists for it: a library declaration file's functions have no bodies.
(user)

**W4. The lexer and the parser come first.** The bytecode file format or
loader, and the performance items of `docs/GAPS.md` section 7 (in-place
collections, string building, a pattern cache), wait until the
self-hosted checker or emitter needs them or a measurement shows the VM
too slow; the parser's own run over the corpus is the first measurement.
(user)

**W5. The checker is the next piece, transcribed from `crates/renyi_check`.**
The checker written in Renyi lives in `compiler/` beside the parser, one
module per Rust file where the Rust has one: `lists`, `report` (the
diagnostics, their rendering and the layout checks of `renyi_syntax`),
`effects`, `suggest`, `types`, `refine`, `declare` (`world.rs`: the
modules, types, functions, abilities and implementations of a project
and the library, resolved after every module is in), `bodies`
(`check.rs`: every function body, test, example, constant and refinement
condition, with the Rust checker's mutable state as a `Checker` record
threaded through every function) and the command line `checker`
(`renyi run compiler/checker.ry [--json] [--strict] [--library <dir>]
<file>...`, which reads the library's declaration files, the file and
its imports, and prints what `renyi check` prints). It is written in one
pass as a transcription, as the parser was, and committed as one piece;
every message and fix is the Rust checker's, verbatim. (user)

**W6. The profile of the VM waits until the checker exists.** Where the
3.9 s of the parser's run on `parser.ry` go is measured after the
checker is written in Renyi, so that one profile covers the parser's
and the checker's runs; the checker's run over the compiler's own
sources is the second measurement. (user)

**W7. The judge of the checker is full equality with `renyi check --json`.**
`crates/renyi/tests/selfhost.rs` runs `renyi run compiler/checker.ry
--json` over every program of the repository (the corpus, the
conformance programs and the compiler's own sources; the library
declaration files are not checkable programs) with and without
`--strict`, and compares the output with `renyi check --json
[--strict]` byte for byte (codes, severities, positions, messages, fixes
and order) and the exit status with the Rust one; a program the Rust
parser rejects must make the Renyi checker fail. It runs in `cargo
test` and in CI with the parser's judge. (user)

**W8. The checker also produces the reference table the VM's compiler
needs.** `bodies.ry` records what `check.rs` records for the VM: the
target of every name (`Target`: a function, an ability method, a
constant, a type, a variant, an ability), the kind of every number
literal (`NumberKind`), the context type of every call whose result the
context alone decides, whether every `otherwise` guards a fallible
value, and the literals and results noted while a body is checked
(`Reference`, `SpanTy`), as Renyi values keyed by span. They are
produced now and judged when the emitter exists, which is the piece
that consumes them. (user)

## X. The speed of the VM (session 8)

**X1. The VM is optimized in development builds.** `cargo test` and CI
run the `renyi` binary of the development profile, where the parser
written in Renyi took 5 s on `parser.ry` and the checker 32 s on
`bodies.ry`; the release profile runs both four to five times faster
(1.0 s and 5.5 s). The workspace manifest sets `opt-level = 3` for
`renyi_vm` and for every dependency in the development profile, so that
the judges, CI and every `renyi run` from a development build get the
release speed of the interpreter while the front end, the checker and
the binary keep their fast rebuilds. (user)

**X2. `slice(start, stop)` on lists.** The lexer written in Renyi cut a
token's text one character at a time (`chars.at(index)` and a text
concatenation per character: 12 percent of a checker run), and no
composition of the existing methods takes a middle run of a list in
time proportional to its length (`take` and `drop` copy from the
start). The prelude gains `slice(self: List of Item, start: Integer,
stop: Integer) returns List of Item`: the items from `start` to `stop`,
the end exclusive, the bounds clamped to the list as `take` and `drop`
clamp theirs; the sketch, `library/std`, the VM and the cheat sheet
change in one commit (a library addition, not a surface change under
V11). The lexer's `slice` is one `slice` and one `join`. Measured on the
checker's own 7500-line `bodies.ry`: 18 percent off the checker's run
and 17 percent off the parser's. (user)

**X3. The interpreter loop is rewritten for speed before the emitter.**
The profile (`docs/GAPS.md`, status at the end of session 8) shows the
time spread over the VM's plain operations, about 35 ns each: `Load`,
`Field` (a linear search of the record's field names on every access),
`Binary`, `Call` and `Return`, with every `step` returning a 48-byte
`Result<Option<Value>, Interrupt>` and every `Value` 40 bytes wide. The
owner chose the deep change over the three cheap ones measured
(arguments left in place at a call and one handler stack for all
frames, 5 to 8 percent; a cache of the field index, about 8 percent; a
fast path for `is` on records, about 3 percent): the dispatch loop
keeps the running frame's state in locals and returns nothing per
operation, `Value` is slimmed by boxing its wide variants, and the cheap
changes come with it. The target is 1.5 to 2 times on the parser's and
the checker's runs, measured with X4; the bytecode emitter waits for
it. (user)

**X4. `renyi run --profile`.** The VM carries its own profiler: a timer
raises a flag every half millisecond, the operation running when it was
raised gets the sample, every operation, call and library primitive is
counted, and the report (samples by function and operation, by
function, by operation kind with the primitives by name; the counts)
goes to the standard error when the run ends. It is a development aid
like `renyi tokens`, named in appendix B of the reference and in
`05-agent-tooling.md`. (user)

**X5. The bytecode emitter writes a file.** The compiler written in
Renyi will emit the VM's own instruction set (`crates/renyi_vm/src/
bytecode.rs`) into a file that `renyi run` loads and runs, rather than
hand its checked tree and references to the Rust compiler in one
process (a loader). The file format is the contract between the two
halves and must be kept equal to `bytecode.rs`; the Rust front end can
then be replaced without touching the VM, which is the point of
self-hosting (decision W2). The format and the loader are designed when
the emitter is written, after the residue of stage 1 (the owner's
order, below). (user)

**X6. The binary allocates through mimalloc.** The profile after X3
shows the remaining time spread over the plain operations, with every
record, list, text and frame going through the system allocator, which
is slow on Windows. The owner chose to add `mimalloc` as the global
allocator of the `renyi` binary (one dependency of C code, built by
`cc` on CI's Ubuntu and on MSVC; a decision in the territory of S1),
on the condition that it buys at least a tenth. Measured in the release
build, best of five, the two binaries run back to back: the parser on
`parser.ry` 811 to 496 ms, the checker on `lexer.ry` 510 to 389 ms, on
`bodies.ry` 2712 to 2138 ms (1.3 to 1.6 times); it stays. Asked in the
same batch: the development profile keeps the front end and the
checker unoptimized (X1 stands), and the residue of stage 1 (`docs/
GAPS.md`, section 7, the status of session 7) is finished before the
emitter is written. (user)

## Y. The residue of stage 1 (session 8)

The owner's order after X6 was to finish what stage 1 of `docs/GAPS.md`
(section 7) left open before the bytecode emitter: entries 1.8, 1.14,
1.17 and 1.18 of the audit and the sketch's open item R3-2. Each was
read in full and the design choices asked as one batch of four.

**Y1. A denied primitive that cannot fail crashes; the server fails.**
Decision J11 reports a scope denial through the primitive's ordinary
error type, which a primitive without a failure type does not have:
`filesystem.exists`, `environment.get` and their kind crash when the
effective grant does not allow the call, and the crash names the
function whose `needs` narrowed the grant. This is now the rule (the
reference, section 11), not a gap: giving such primitives a failure
type would make every call to them take `otherwise`. The server joins
the modules that fail: `serve` outside the grant is
`StartError.PermissionDenied(port)`, past a budget the new
`StartError.OverBudget(port)` (`library/std/server.ry`). Containment of
a path scope stays lexical, written into the reference: a scope
contains a path by its text once `.`, `..` and the separators are
normalized, nothing is resolved on disk, so a symbolic link under the
scope reaches wherever it points. Canonical paths would cost a system
call per check and change the answer with the state of the disk, which
a replay cannot reproduce. (user)

**Y2. A replay compares the recording's grant with the `needs`.**
`06-runtime-guarantees.md` says a replay checks that the recording does
not exceed the grant it is replayed under, and the implementation
checked each recorded call's capability but never the `grant` header.
Now `replay_with` checks the header first, then the calls: every
capability the header names must be covered by the effective grant of
the test or program, else the replay is refused naming the capability
and the grant. The six fixtures of the corpus already fit their tests'
`needs`. (user)

**Y3. Collections keep the derived `Equal` and `Hash`.** A declared
`equals` decides `is` and `is not` (stage 1); `Set` items, `Map` keys,
`distinct`, `to_set`, `contains` and `index_of` compare and hash by
structure and consult no declared `equals` or `hash`, as the ability
table of the library sketch said and the reference now says (section
5). Consulting them would make a hash depend on user code and a
collection's behaviour on the implementation in scope, which the
derived form avoids; a type that wants its own notion of equality in a
collection keys it by a field. In the same entry: `repeat`, `pad_left`,
`pad_right` and `rounded` take a count that fits a machine word, and a
larger `Integer` is a crash, `this Integer is too large for the
operation`, written into the reference (section 7) rather than made a
failure. (user)

**Y4. The loop unwinds its operands; a refinement sees its own field;
R3-2 waits.** Entry 1.18 had two parts. (i) `break` or `continue` as
the outcome of an `if` or `match` expression nested in another
expression left the interrupted expression's operands on the stack: a
statement loop now marks the operand stack's height at its entry
(`Op::MarkStack`) and every `break` and `continue` drops the operands
above the mark (`Op::UnwindStack`) before it jumps, so the stack of a
long loop no longer grows and the emitter of X5 can assume a fixed
height at every loop boundary. (ii) The reference says a refinement is
a condition "over the field's name", and both checkers bound every
field of the record or variant: a field's condition now sees that field
alone (a subtype's `value`), and a condition that names another field
of the type is the error `refinement-field`, with the fix to check both
fields where the value is built; the condition code of the VM still
takes every field, unchanged. The sketch's open item R3-2 (constraints
with type arguments, `for any Bag where Bag can Iterable of Item`) is
deferred until after the emitter: it needs a design round of its own
and nothing in the compiler waits on it. (user)

## Z. The bytecode file and the emitter (session 8)

Decision X5 set the emitter's shape: it writes a file the VM loads.
What the file is, how the VM reads it, how the emitter written in Renyi
is judged and where the commands go were asked as one batch of four,
answered with the recommended options.

**Z1. The bytecode file is the derived JSON of Renyi types.** The
format is declared once, as the types of `compiler/bytecode.ry`
(`Program`, `Code`, `Op` and the rest, each `can ToJson`), and a
bytecode file is their derived JSON (library sketch, section 7), as the
syntax tree's JSON is the derived JSON of `compiler/ast.ry` (decision
W1): a record is an object whose keys are its fields in declaration
order, a variant an object with `kind` first, a `maybe` without a value
`null`, a list an array, a number a JSON number, and every span counts
characters. The first field is `format`, the version of the layout; the
extension is `.ryc`. The emitter written in Renyi builds the `Program`
value and prints it with `json.render_indented`; the Rust side
(`crates/renyi_vm/src/file.rs`) renders the same document from the VM's
own `Program` through the JSON writer of `renyi_syntax`, which the judge
of decision W3 already holds equal to Renyi's. A binary layout would be
smaller and faster to load, and would need a second definition of every
structure on each side; the JSON is read by any tool, diffed in a
review, and defined by the types the Renyi compiler needs anyway. The
maps of the VM's program (`impls`, `method_index`) are written sorted,
so that the document is a function of the program alone. Field names
that are reserved words of Renyi (`function`, `module`, `returns`,
`fails`, `needs`, `set`, `ability`, `count`, `purpose`) are renamed in
the file (`function_id`, `owner` or `module_name`, `result`,
`failures`, `capabilities`, `set_type`, `ability_id`, `size`,
`summary`), and a number constant is written as its digits in a string,
so that an `Integer` past a machine word and a `Decimal`'s scale
survive the trip. (user)

**Z2. The loader is hand-written over the VM's own JSON reader.** No
serialization crate: `file::load` reads the document with
`natives::json::read_json`, the reader behind `json.parse`, and walks it
field by field into the VM's `Program`, refusing anything that does not
fit with the path of the place (`the file.codes[3].ops[7].target: ...`)
and refusing a `format` that is not the one this VM reads. After the
walk a consistency pass checks every index, a function's code, a
constant's code, an example's codes, a jump's target, a slot against
the code's locals, a constant against the code's table, a type, a
field site and a result type against the program's tables, so that a
file written by hand cannot send the VM past a table. This keeps the
dependency list at decision S1's (the reader exists already) and the
format readable by the Renyi compiler itself one day. (user)

**Z3. The judge is byte equality, then behaviour.** The emitter written
in Renyi is right when `renyi run compiler/compile.ry <file.ry> --to
a.ryc` writes, byte for byte, what `renyi compile --to b.ryc <file.ry>`
writes, over the corpus, the conformance programs and the compiler's own
sources, and when both refuse every program the Rust checker refuses
(`crates/renyi/tests/selfhost.rs`, as decisions W3 and W7 judge the
parser and the checker). Behaviour is judged by running: every `run`
case of the conformance suite runs a second time from the `.ryc` file
`renyi compile` writes for its program and must print the same output
and exit the same way (`crates/renyi/tests/conformance.rs`), the VM's
own test renders a program, loads it back, renders it again and runs
both (`crates/renyi_vm/tests/file.rs`), and the binary's test runs,
records and reproduces a program from its file
(`crates/renyi/tests/compile.rs`). Byte equality is the stricter judge
and the one that locates a difference; the reruns catch what a file
loses on the way (a span, a constant's digits, a fixture's path). (user)

**Z4. `renyi compile`, and the file loads by its extension.** `renyi
compile [--to <file.ryc>] <file.ry>` checks the program with its
imports and writes its bytecode, by default `<name>.ryc` in the working
directory, as `renyi record` names its recording; its diagnostics are
those of `check`, and a program with errors writes nothing. `renyi
run`, `record`, `test` and `reproduce` take a `.ryc` file wherever they
take a `.ry` file and load it instead of checking and compiling, so a
program's `main`, tests and examples run from the file alone; the
`replays` fixtures are found beside the source paths the file remembers.
The run manifest's code hash of a program loaded from a file is the
SHA-256 of the file's bytes, since the definitions `main` reaches are
not there to hash: a recording made from a `.ryc` reproduces from the
same `.ryc`. No separate `load` command and no `--bytecode` option: the
extension says what the file is. In the same entry: the spans a program
keeps, in process or in a file, count characters, as the syntax tree's
JSON does, so that a program loaded from a file is the program compiled
in process; the compiler converts the parser's byte offsets when it
emits, and a module's source is remembered as its path and the
character offsets of its line starts, which is what a crash location
and a test report need. (user)

## AA. The rename of `set` (session 8)

**AA1. `set name to expression` becomes `change name to expression`.**
The owner asked whether to rename `set` to `change` and to forbid every
other way of changing a binding. The second half was already the rule:
the grammar has no `=`, a name is bound once per function (`shadowing`),
and `set` on a `let mutable` binding was the only way to change a value.
The rename stands on its own: `set` had three meanings, the statement,
the `Set` type and the `Map.set` method, and `change x to x + 1` names
what happens to an existing binding. A surface change under decision
V11: this entry, then the reference (sections 1, 6, 8 and 17), the
grammar, the cheat sheet, the formatter, both front ends (`Word::Change`
and the statement `Change` of the tree and of its JSON; the reserved
list of `compiler/lexer.ry`, `change_statement` in `compiler/parser.ry`),
the checker's fixes (`write `change x to x.method(...)``), the corpus,
the conformance programs, the compiler's own sources, the crates' tests
and the lint, in one commit. `set` leaves the reserved words and
`change` joins them (88 words still); `Set` and `Map.set` are unchanged.
The readability rounds under `tests/readability/` keep the grammar they
measured. (user)

## AB. Constraints with type arguments (session 8)

**AB1. A constraint takes the type arguments its ability declares, and
the collections are `Iterable`.** Open item R3-2 of the sketch, deferred
by Y4 until after the emitter, asked as a batch of four and answered
with the recommended options. (i) `for any Bag, Item where Bag can
Iterable of Item`: a constraint names an ability with as many type
arguments as the ability declares (`type-arity` otherwise, with the fix
`write `Bag can Iterable of Item``), and the arguments are any types in
scope: the clause's own parameters, concrete types and composites. An
ability's requirement follows the same rule with the ability's own
parameters in scope (`ability Countable of Item where self can Iterable
of Item`). Inside the body a parameter constrained to `Iterable of Item`
is walked by `for each` and the queries with items of type `Item`, and
a method of the ability called on it has the ability's parameters
substituted (`bag.to_list()` is a `List of Item`). At a call site the
argument's type must have the ability with matching arguments: those of
its implementation, with the implementation's parameters read as the
type's arguments (`Iterable of Item for Stack of Item` gives `Stack of
Text` the argument `Text`), or those a constraint of its own gives it;
matching binds what the constraint leaves open, so `Item` is inferred
from `Deck`'s `Iterable of Text`, and a mismatch is `missing-ability`
naming the arguments (``total` needs `Iterable of Integer`, which `Deck`
does not have`). (ii) The prelude implements `Iterable` for `List of
Item`, `Set of Item`, `Map of Key to Value` (its pairs), `Range`
(`Integer`) and `Text` (its characters), so that a generic walker
accepts a list and `to_list` exists on each; the loop's own rules for
the collections are these implementations now, an implementation head
takes full types as arguments (`ability Iterable of Pair of Key, Value
for Map of Key to Value`), a declaration file holds an implementation
with its methods' heads alone, and the VM dispatches an ability call on
a base value by its declared type. The alternative, a constraint that
only a declared implementation satisfies, would have refused a list to
every generic walker. (iii) An ability named without the arguments its
parameters call for, `Bag can Iterable`, is `type-arity`, the rule the
implementation head already followed; an item type left unknown would
have been an existential for little gain. (iv) One implementation of an
ability per type, whatever the arguments: a second is
`duplicate-implementation`, since dispatch is by the value's type alone
and a loop needs one item type; two implementations were accepted
silently before. In the same entry: an implementation's methods are as
visible as their ability, which the reference said and the checker did
not do (`private-name` on a public ability's method from another
module). A surface change under decision V11: this entry, then the
reference (sections 2, 3, 4, 5 and 8, appendix A), the grammar
(`Implementation` takes types after `of`; `ImplDeclaration` for a
declaration file), the cheat sheet, both front ends, the library and its
sketch, the conformance suite (four cases) and the checker's tests, in
one commit. (user)

## AC. Packages (session 8)

**AC1. Packages: a static registry, JSON manifests, the package name as
the first segment of an import, and the first slice of M4.** The owner's
batch after the judges ran from bytecode, four questions answered with
the recommended options; the rules that carry them out are this entry.
(i) *The registry is a directory or a URL*, not a service:
`<registry>/<name>/versions.json` lists the versions,
`<registry>/<name>/<version>/package.json` holds the package's name,
version, purpose, dependencies, the toolchain that published it, every
source file's path with its SHA-256 and the effect manifest of Q1 (for
every public function, its transitive capabilities and failure types),
and the source files lie beside it. A package's content hash is the
SHA-256 of its `package.json` text, spelled `sha256:` and the hex digest
as the index spells hashes. A directory registry (absolute, or relative
to the project root) is read in place; a `http://` or `https://`
registry is fetched into `.renyi/packages/<name>/<version>/` under the
project root by `renyi fetch` and by `renyi add`, which check every
file's hash and recompute the effect manifest from the sources, refusing
a package whose claimed manifest differs: Q1's recomputation by the
registry is the client's until a registry with signatures exists (R7-2).
`renyi publish [--to <directory>]` writes a project into a directory
registry (a URL registry is published through its local copy): the
project must check clean, the version must be at least what the
semantic diff against the highest published version demands (G1: a new
major when a public signature went or changed, a new minor when one
came, else greater than the previous), and a published version is never
overwritten. (ii) *The manifest is JSON.* `renyi.json` in the project
root holds `name`, `version`, `purpose`, `dependencies` (package name to
the version required), `registry` and `budgets` (the thresholds of R7,
`public_per_module`, `effect_paths_per_module` and
`fan_out_per_definition`, which `renyi index --budgets` reads when they
are there); the lockfile `renyi.lock.json` beside it holds `packages`,
every package the program reaches with its `name`, `version` and
`hash`, sorted by name, written by `renyi add` and `renyi update` and
read by every other command. A project is the directory holding
`renyi.json`; a program's project root is the nearest such directory in
its file's directory or above it (up to the working directory for a
relative path), else the file's directory as before (J17), and its own
imports resolve from the root. A version is `major.minor.patch`; a
requirement names a version and means the same major and at least that
version; a program holds one version of a package, the highest one every
requirement allows, and requirements that disagree on the major are an
error. One format for the recording, the run manifest, the bytecode file
and the manifests, read by one reader (`crates/renyi_json`, the VM's
reader moved out); the alternatives were a `package.ry` in Renyi syntax
(a surface change under V11) and TOML (a new dependency). (iii) *The
package name is the first segment.* `import <name>.<path>` reaches the
dependency's file `<path>.ry` from the package's root and `import
<name>` its root module `<name>.ry`; inside the package a module header
is the path from the package's root (`module fetch`), and the checker
names the module `<name>.fetch`, the root module `<name>`; an import
inside a package that names one of the package's own dependencies
reaches that dependency, any other its own files; at the program level
an import reaches a package only when the manifest lists it as a
dependency. A dependency wins over a directory of the project with its
name; `renyi add` refuses such a name and `std`. No new keyword, and no
package name in the headers. (iv) *The first slice is packages*: the
manifest, the lockfile, `renyi add <name> [<version>]` (resolve, fetch,
print every public function's effects and failures, write both files),
`renyi update [--accept-effects]` (every dependency to the highest
version its requirement allows; one whose effects widen is refused
unless the flag is given and `main`'s grant covers them), `renyi audit`
(every dependency's transitive effects against `main`'s grant, and the
capabilities of the grant no dependency uses), `renyi fetch`, `renyi
publish`, the dependencies with their hashes in the run manifest, which
`renyi reproduce` compares (Q2), and the thresholds of R7 in the
manifest; `std.process` with the capability `process` is the next slice
and the foreign function interface (`foreign`, F0) the one after. Across
a package boundary the static coverage rule checks the kind of a
capability and the grant stack its scope, as reference section 11
already said, and the error names the package ("`announce` (package
`greeting` 1.0.0) needs `console`, which `main` does not declare"). A
package whose `package.json` differs from the lockfile is
`package-mismatch` on the import, a dependency of the manifest that the
lockfile or the registry lacks `package-missing`, and a manifest, a
lockfile or a `package.json` that cannot be read `manifest-invalid`. The
resolver is `crates/renyi_package`, used by the checker and the binary;
`Bytes.sha256()` joins the prelude so that the front end written in
Renyi verifies a package as the Rust resolver does
(`compiler/project.ry`), and the judges hold the two equal over the
projects with a dependency in the conformance suite
(`tests/conformance/packages/`). The front end written in Renyi reads the
manifests through `std.json`, whose maps keep one value per key, so a key
given twice is refused by the Rust resolver alone, as is a budget spelled
`1.0` or `1e2` or a version part beyond 64 bits; the two are left to
differ there. (user)

## AD. The diagnostics of the front end in Renyi (session 8)

**AD1. The front end written in Renyi reports what the Rust front end
reports: every diagnostic with its code, message, span and fix, and the
same recovery.** The "2" of the owner's answer "1+2" of 2026-10-07
(stage 3 and, beside it, this). Until this entry `compiler/lexer.ry`
and `parser.ry` stopped at the first problem with a message and an
offset, and the judges of `crates/renyi/tests/selfhost.rs` asked only
that both front ends reject the same programs; a toolchain in Renyi
that answers for `renyi check` and `renyi compile` must say what they
say on a broken file too. The rule: (i) the lexer and the parser in
Renyi carry every error site of `crates/renyi_syntax/src/lexer.rs` and
`parser.rs` with the code, the message, the span and the fix verbatim,
and recover where the Rust ones recover (the lexer reads on after a bad
character, which becomes an `ErrorToken`; the parser skips to the end
of the line after a bad import, statement, clause or example and to the
next item at the margin after a bad item, and keeps the partial tree);
the parser's `Cursor` carries the diagnostics and its failure `Halt`
carries the cursor, so that a caller that recovers reads them off it.
(ii) `renyi run compiler/parse.ry` prints the tree, then the
diagnostics as `renyi parse --json` prints them, and exits as it exits;
`checker.ry` and `compile.ry` print a rejected file's diagnostics as
`renyi check` and `renyi compile` do, and follow the imports of its
partial tree as the Rust resolver does. (iii) The three judges compare
the standard output and the exit status on every program, rejected
ones included (the emitter judge compares the files besides when both
wrote one); lane probes exercise the error sites no program in the
repository reaches, and two conformance cases hold a sample. (iv) The
rule of W7 extends to the front end: a new diagnostic, message or fix
in the lexer or the parser of `crates/renyi_syntax` is the same change
in `compiler/lexer.ry` or `compiler/parser.ry` in the same commit. The
alternative, one `syntax` diagnostic per rejected file, was what
existed: enough for the trees, not for the toolchain. (user)

## AE. The process module (session 8)

**AE1. `std.process` starts a program and waits for it: one call per
run, a status other than 0 a failure, the scope the program's name as
spelled, the parent's environment inherited.** The second slice of
stage 3 (M4), after packages (AC1), in the order the owner chose. The
four questions of 2026-10-07 and their answers: (i) the shape of the
module: `execute(program, arguments)` and `attempt(program, arguments)`
run the program to its end and report a `Completion` (the status, the
standard output as text and as bytes, the standard error as text);
`execute_with` and `attempt_with` take an `Options` record (a working
directory, variables added to the environment, text for the standard
input, a time limit past which the program is killed and the call fails
with `Timeout`) and `defaults()` sets none; no handle, no interactive
reading or writing, no background process: one primitive call is one
record of a recording, and a handle would sit badly with tasks that run
one after the other (S2); a handle can be a later entry when a program
needs one. (ii) A status other than 0 is the failure `Exited(program,
status, output, errors)` of `execute`, so that a caller handles it like
any failure (as `http.get` fails on a status outside 2xx, K2);
`attempt` reports any status as a completion, for the programs whose
status is an answer (`grep`, `diff`). (iii) The scope `process("git")`
is matched by the program as the call spells it, text for text: the
rule J11 states ("a program name contains only itself"), the simplest
to explain, and the one under which a package's effect manifest says
exactly which programs the package may start; a full path needs a grant
of that path, and `git.exe` is not `git`. The alternative, the file
stem, would have let `process("git")` start any file so named anywhere.
(iv) The program inherits the parent's environment and working
directory; `Options` adds to or overrides them. A clean environment
would have made most programs (`git` wants `HOME`) need variables
passed by hand, for a capability the grant already limits by program.
The function is `execute`, not `run`: `run` is a reserved word (`run
concurrently`). The VM's boundary already took the first `Text`
argument of a `process` primitive as the scope of its effect and
counted its budget; the module adds the natives, the module's
`ProgramNotAllowed` and `OverBudget`, the tests, a conformance case,
and `foreign` alone stays unavailable (V6). (user)

## AF. The foreign function interface (session 8)

**AF1. A foreign module is a declaration file of the project that the
manifest binds to shared libraries; scalars, text and bytes cross the
boundary; the libraries load at run time through `libloading` and a call
goes through a fixed signature family; a foreign call is a primitive at
the boundary.** The third slice of stage 3 (M4), after packages (AC1)
and `std.process` (AE1), in the order the owner chose; decisions F0 and
F1 named the capability and left the mechanism open. The four questions
of 2026-10-07 and their answers: (i) Bindings are declared, not written
in a new syntax (the surface is frozen, V11): a foreign module is a file
of bodiless `public function` declarations, each `needs foreign` and
nothing else, that the manifest's `foreign` section names (`"foreign":
{"libc": {"library": ["ucrtbase", "libc.so.6", "libSystem.B.dylib"],
"symbols": {"renyi_name": "c_symbol"}}}`): the libraries are tried in
order and the first that loads is used; `symbols` renames a function
whose C symbol is spelled otherwise. The resolver tags the file (the
main file by its path from the project root, an imported one by its
qualified name), the checker parses it as declarations and declares it
as a library module (its functions are primitives, recorded like the
library's), and `renyi bind <header.h> --module <name> --library
<name>[,<name>...] [--to <directory>]` writes such a file from a C
header's prototypes, with the manifest entry. The alternative, an
`external` clause on a function, was a surface change for what a file
and a manifest entry say as well. (ii) Only scalars, text and bytes
cross: the width types of `std.foreign` (`Int8` to `UInt64` and `Size`,
refinements of `Integer`), `Float` (a C `double`), `Boolean`, `Text` (a
NUL-terminated `char *`, copied each way) and `Bytes` (a pointer and a
length, two words, parameters only); a result is one of those but
`Bytes`, `maybe Text` for a `char *` that may be null, or nothing.
Records, lists, callbacks and ownership wait for a later slice; what the
first slice carries is what `libc` and `libm` take. The checker holds a
foreign module to it: `foreign-signature`, `foreign-type` and
`foreign-arity` (at most six words). (iii) The libraries load at run
time through `libloading`, a runtime dependency like those of S1, and a
call goes through a fixed family of signatures generated into
`foreign_abi.rs` by `tools/gen_foreign_abi.py` (every arity to six,
every pattern of integer-class and double-class words, three result
classes): the one unsafe corner of the VM; no `libffi`, no C compiler at
run time, and a C function that does not fit the family is reached
through a C wrapper that does. (iv) A foreign call is a primitive at the
boundary: a recording holds it with its result, a replay answers it
without calling, `reproduce` likewise; a guarded value (`only to`)
refuses to cross; `foreign` takes no scope and no budget (a budget
meters the world; a C call is the program's own code); `renyi run`
prints "this program can call native code through `libc`" when `main`
grants `foreign` (07-system-design.md, section 2.2); and `renyi publish`
refuses a project with foreign modules, since a package carries no
native code. `capability-unavailable` goes: nothing waits for M4 any
more. (user)

## AG. Machine code for the bytecode (session 8)

**AG1. The VM generates machine code for the bytecode through Cranelift
inside the one `renyi` binary, per code object once it is hot; the
bytecode file stays the contract between the front ends and the VM, the
compiler written in Renyi is untouched, and the interpreter remains the
reference: a frame whose typed assumptions fail at run time is handed to
it at the op in question.** The owner's direction of 2026-10-07:
strengthen performance at the root, ahead-of-time compilation included.
The four options and the choice: (i) Cranelift in the process (chosen):
one binary, no toolchain at run time, and the same backend serves a later
`renyi build`; (ii) a C or LLVM backend: a compiler at run time, or a
second build pipeline; (iii) a faster interpreter alone: the loop of
decision X3 had reached 3.3 times slower than CPython on integer loops,
near the ceiling of a loop over boxed values; (iv) WASM: another target,
not a speed-up. How it works (`crates/renyi_vm/src/native/`): `infer.rs`
runs an abstract interpretation over a code object's stack bytecode and
settles which operands and slots stay unboxed (small Integers, Booleans,
Floats and the bounds of a range, in registers); `codegen.rs` translates
every op to Cranelift IR over that analysis, arithmetic, comparisons,
branches and range loops in registers and everything else as a call to a
runtime helper that does what the interpreter's arm does on the VM's
stack; `runtime.rs` holds those helpers and the hand-back (`rt_deopt`
boxes the registers into the interpreter's frame and operand stack at
the op). The generated code and the interpreter share every op's
implementation (`Vm::op_*`), so a change to an op is one change. The
conformance suite runs every program a third time with machine code for
every code object (`RENYI_NATIVE_HOT=1`), and `crates/renyi_vm/tests/native.rs`
covers the hand-backs: an Integer that leaves the machine word, a guarded
Integer in a typed parameter, a call past the depth of native frames,
failures, crashes and deadlines inside generated code. (user)

**AG2. Integer stays arbitrary precision (decision B9): the generated
code computes on `i64` with overflow checks and hands the op to the
interpreter, which has the big integers, when a result leaves the machine
word.** The alternatives were a fixed-width Integer, a change to the
language, or big integers in the generated code, every arithmetic op a
call. A program that overflows a word at every op runs at the
interpreter's speed; one that never does runs in registers; the result is
the same either way. (user)

**AG3. Machine code is on by default for `run`, `record`, `test` and
`reproduce` and so for the judges; `--explain` and `--profile` use the
interpreter, which narrates and counts; `--interpret` forces the
interpreter.** A code object is compiled once the interpreter has run
`HOT_FACTOR` (2000) times its size in ops of it, and the machine code
takes over at the code's next call or, when the interpreter is inside
one of its loops, at the loop's next turn: the generated function has an
entry at every loop header whose operand stack holds nothing in
registers, which takes the frame's slots into registers under the same
checks as the parameters (a slot that does not fit leaves the frame
with the interpreter for good). Compiling an op costs a few hundred
times running it on the interpreter, so code run once or twice (a
driver, a setup, a short loop) never pays for machine code it would not
use, and a small script compiles nothing; the first rule, "a code
object with a loop at its first call", compiled 130 code objects of the
checker written in Renyi on `examples/hello.ry` for 0.13 s a run, most
of them loops of a few turns. A range of small Integers is iterated by
both tiers in the same shape (`Native::RangeIterator`: the next value,
the last and the step, no longer a list of its items), so a frame can
change hands inside such a loop. Measured on the development build:
`examples/hello.ry` starts in 46 ms either way; the front end written in
Renyi, compiling `compiler/lexer.ry` with every code object forced
native, generates code for 676 code objects (39,502 ops, 240,713
Cranelift instructions) in 0.42 s, three quarters of it Cranelift's own
register allocation and lowering. (user)

**AG4. A `bench/` directory with six benchmarks and `tools/bench.py`; CI
runs them and prints the numbers, it does not gate on them.** The
programs: `primes.ry` (integer loops), `strings.ry` (text building and
character counting), `records.ry` (records in lists),
`json_round_trip.ry` (JSON rendering and parsing), the self-check of the
compiler written in Renyi (`compiler/checker.ry` on `compiler/bodies.ry`)
and the start of `examples/hello.ry`; the first four have CPython twins
next to them, and `bench/*.ry` are held to `renyi check` and the
canonical layout like `compiler/`. The targets the owner set: `primes`
at least five times faster than CPython (met: 84 ms against 529 ms on
the development build, 6.3 times), the compiler's self-check at least
twice as fast as before (not met by machine code; AG5), `hello` no
slower (met). (user)

**AG5. What the machine code does not do, measured, and what follows:
the VM's own operations are the next performance work, bounded; an
ahead-of-time image (`renyi build`) and a baseline JIT that inlines the
boxed operations are planned after release 0.1.** Measured 2026-10-07 on
the release build (Linux under WSL): the compiler written in Renyi
checking `compiler/bodies.ry` takes 1.6 s of CPU on the interpreter and
1.5 s on machine code for every code object, plus 0.36 s to generate it;
of its 5.4 million calls none hands a frame back. The time of such a
program is in the values, not in the dispatch: the profile (`perf`)
puts 32% in the interpreter's loop (of it 12% pushing values on the
stack), 14% in cloning values, 8% in dropping them, 10% in the allocator
(records and lists), 6% in the kernel zeroing fresh pages, 5% in pushing
frames and 4% in field access, and the generated code calls the same
runtime for all of it. So: (i) a bounded round of VM work on those paths
comes next (the stack push, the clone and drop of values, the allocation
of records, the call frame, the boundary of the pure primitives),
measured against `bench/`, after which positioning and release proceed
whatever the number (the owner's choice over "until the self-check is
twice as fast" and "no VM work now"); (ii) `renyi build`, an image of
the bytecode with the machine code of this machine that `run` loads in
place of generating it, is deferred until the machine code wins on
ordinary programs, since today it wins only on numeric loops, which
generate in milliseconds; (iii) inlining the boxed operations (loads,
stores, field access, calls) in the generated code, the road of a
baseline JIT, is deferred the same way. The development aids of the
backend: `RENYI_NATIVE_REPORT` prints what was compiled, the time per
phase and the hand-back and call counts on the standard error when the
run ends; `RENYI_NATIVE_HOT` sets the factor (`0` compiles everything
at its first call and enters every loop at its first turn);
`RENYI_NATIVE_OPT` sets Cranelift's optimisation level (`none` by
default: the optimiser found nothing in helper-call code for twice the
time); `RENYI_NATIVE_VERIFY` turns Cranelift's IR verifier on. (user)

**AG6. The bounded VM round of AG5, done and measured. Kept: a load of
a record or a variant that a read of one of its fields follows reads
the field in the slot; a frame names its grant by an index into a
table, a per-code flag says whether the function narrows the grant at
all, and the locals are filled in one resize; a variant without fields
is built once per type and tag. Tried and dropped: a store of a binding
fused with the load that follows it; the fields of a record or a
variant inline in its allocation. The interpreter's other operations
stay as they are, and the next performance work is the one AG5 defers
until after release 0.1.** The owner's answer of 2026-10-07, on
learning that the machine code brings nothing to the compiler written
in Renyi: one bounded round of work on the VM's own operations, then
on to the next item whatever the number. The measure is the count of
user-space instructions and cycles (`perf stat`, the release build
under WSL, two binaries interleaved, the minimum of three rounds): the
loaded machine of that day moved wall and CPU time by a third between
two runs, the instruction count by nothing and the cycles by a few
percent. The program is the compiler written in Renyi compiling
`compiler/bodies.ry` on the interpreter. The whole round: instructions
from 15.49 billion to 14.66 billion (-5.4%), cycles from
10.71 billion to 10.27 billion (-4.1%); with the machine
code on, -2.5% and -4.5%. Step by step: (i) the fused field
read in its first form, with the store fusion, 2.0% of the
instructions; (ii) the grant by index, the flag and the resize, another
1.0%; (iii) the fused field read restricted to a slot that holds a
record or a variant, and the store fusion dropped, another 2.0%: the
look at the next op had cost every load and store something, and the
store fusion, which saved a pop and a push, took the record off the
slot, so the field read after it was the slow one again; (iv) the
inline fields (`SmallVec<[Value; 3]>` in `Record` and `Variant`, one
allocation in place of two), tried with the unit variants: 3.4% more
instructions and 0.8% fewer cycles on this program, 8.2% fewer
instructions and 0.6% fewer cycles on `bench/records.ry`, 0.4% more
and 3.0% more on `bench/json_round_trip.ry`, 2.2% more and 8.9% more
on a loop that builds a three-field record and reads it; the branch on
every field access and the element-wise build and drop cost what the
second allocation had cost, so the fields went back to a `Vec` and the
unit variants stayed. On the micro-benchmarks of the interpreter loop
(instructions per turn of the loop, before and after the round): a
counting loop, 713 to 731; a call and return, 1097 to
1100; a primitive call, 1857 to 1906; a field read,
1266 to 1133; a record built and read, 2099 to
1970 (the first three, which the round does not touch, move
with the code layout of the interpreter loop, which a build changes by
up to 3%, so a micro-benchmark alone attributes nothing). Not done,
with the reason: the boundary of a pure
primitive is a move of the arguments into the scratch buffer, a check
that no argument is guarded, a look at the function's `needs` (none)
and the call, and nothing in it is worth a second path; `change x to x
with ...` could move the receiver as `change x to x.method(...)` does
(decision X3) and update the record in place, but that rule is a change
to both emitters and the judges for 34 sites in `compiler/`, and waits;
a fused `LoadField` op would take the look at the next op out of the
loop, but an op the bytecode file never carries is a format question
first (decision Z1), and waits too. The time of such a program stays
where AG5 found it: in the clone and drop of values on every load and
return, in the stack traffic and in the allocator, which only the
baseline JIT or `renyi build` of AG5 can take out. (user)

## AH. The niche (session 8)

**AH1. Renyi is the scripting language of AI agents: the language an
agent writes and a person reviews at a glance; the program declares what
it may do, the runtime admits only that, and every run can be recorded,
replayed and narrated. Its first users are people who run automation
with Claude Code, Codex and their like and will not run an agent's
Python blind.** The owner's second direction of 2026-10-07: the design
had its innovations but no fixed niche (decision V1 named "people
building agent workflows and learners" as the first users without saying
what the language is to them). The four candidates and the choice: (i)
the scripting language of agents (chosen): every guarantee the design
made (the effect system of sections C and P, the recorded runs of O and
P, the agent tooling of D and the project map, the packages of AC) is
what a person needs before letting an agent's program run; (ii) a
teaching language: the readability is real but the guarantees would be
a side show; (iii) a safe automation language for operations teams: the
same guarantees, but the author would be a person, for whom the English
syntax is slower to write than it is to read; (iv) a general-purpose
language with a safety story: nothing to lead with. The posture stays
general (AH4); the niche says who comes first. `docs/design/08-positioning.md`
holds the text. (user)

**AH2. Four selling points, in this order: effects as capabilities,
declared on every function, scoped, budgeted and enforced at the
boundary, dependencies included; recorded, replayed and narrated runs;
the tooling an agent needs (the project map, the MCP server, `purpose:`
as syntax, a fix on every error); package effect manifests computed by
the tool and never widened silently.** All four candidates offered were
taken; each is a promise the implementation keeps today (the conformance
suite, the judges, `renyi audit`), which is the condition for leading
with it. (user)

**AH3. The English-like syntax ranks second: the positioning leads with
the guarantees and shows the syntax as the means by which a person
reviews a program at a glance.** Leading with the syntax was the
project's original pitch (decision A2) and reads as a curiosity; the
guarantees are what the first users lack elsewhere. (user)

**AH4. No non-goals are written; the posture stays that of a general
language.** A list of non-goals would read as limits before the language
has users; the niche says who comes first, not who is excluded. (user)

## AI. Release 0.1 (session 8)

**AI1. Release 0.1 after the machine code, the positioning document and
the release engineering land.** The alternatives: release at once from
the private repository, with nothing to install and nothing to read; or
after the embedding API and the LSP of M5, months away. (user)

**AI2. The release engineering, all of it: CI builds Linux, macOS and
Windows binaries on a tag and publishes a GitHub Release; an install
script and `cargo install renyi` (the crate name is free on crates.io,
checked 2026-10-07); a documentation site on GitHub Pages with the
reference, the cheat sheet, the examples and the positioning; a VS Code
extension for syntax highlighting.** (user)

**AI3. The first acquisition is a starter pack for agents: a skill file,
the MCP server and five runnable workflow examples.** The alternatives,
a launch post or a benchmark table, say what the language is; the pack
lets an agent use it in its first hour, which is where the niche of AH1
is won or lost. (user)

**AI4. The name stays; a GitHub organisation `renyi-lang` takes the
repository as `renyi-lang/renyi`; the owner buys renyi-lang.org.** (user)

**AI5. The crates are self-contained: `renyi_check` embeds a copy of
`library/std/` under `crates/renyi_check/library/std/`, and `renyi` a
copy of `docs/cheatsheet.md` as `crates/renyi/cheatsheet.md`; a test in
each crate holds the copy equal to the canonical file, byte for
byte.** A crates.io tarball carries only the files under the crate's
directory, so an `include_str!` of a file outside it compiles in the
workspace and fails from the tarball, which `cargo package
--workspace` showed on 2026-10-07. The alternatives: the canonical
`library/std/` moved under the crate (every path in the documents,
the tests, the judges and the lane scripts changes, and the cheat
sheet still needs a copy); or no crates.io for 0.1. The copies cost
copying the file again after an edit, which the tests demand; the
canonical paths stay. (user)

## AJ. Foreign packages: Rust and Python (session 8)

The owner's question of 2026-10-07, evening: can a Renyi program use
Python or Rust packages directly, so that the language extends without
a library of its own for everything? Four answers.

**AJ1. Rust packages extend Renyi through the mechanism the standard
library uses: a native function registered with the VM under a
declaration file that states its signature, its capability and its
failure types, built into the `renyi` binary; the registration API is
the half of the embedding API of M5 (decision Q3) that faces the
host.** A registered native is checked, recorded, replayed and narrated
like a library primitive, so an extension costs no guarantee; the
standard library's own natives move to the same mechanism, which makes
the API tested by its first user. The alternatives: the C FFI alone
(AF1: a Rust crate as a C-ABI library through `renyi bind`, with
scalars, text and bytes only, and `foreign` outside every guarantee),
or nothing until a user asks. (user)

**AJ2. Python packages are reached through a typed bridge to a worker
process: a declaration file states each function's signature over the
types JSON carries (numbers, text, booleans, lists, maps, records with
`FromJson` and `ToJson`), the VM starts one `python` process per run
and sends each call as a JSON message, and each call is one primitive
at the boundary, recorded with its arguments and its result.** Not
CPython in the process: it opens a hole that can do anything, needs
`libpython` found at run time and the GIL handled, for a speed a script
does not need. Not `std.process` alone: one process per call, text
only, the types decoded by hand. The bridge keeps recording, replay,
`reproduce` and `--explain`; what the Python side does is outside
Renyi's guarantees, and the reference says so where it says it of
`foreign` (`07-system-design.md`, section 4.3). The bridge is an
extension package written against the API of AJ1, the first after the
standard library. (user)

**AJ3. A call through the bridge charges one capability, `python`,
scoped by the package the declaration file binds: `needs
python("pandas")`.** A reviewer reads in the signature that the program
runs Python and which package. The alternatives hide that: effects
declared per function by the binding's author are a promise the checker
cannot verify, and `foreign` does not tell a C library from Python. A
new capability kind is a change to the reference's section 11 and the
checker's table, not to the surface frozen by V11. (user)

**AJ4. The order: after release 0.1, the registration API of AJ1
first, then the bridge of AJ2 and AJ3, before the other items of M5.**
The alternatives, before 0.1 or after M5, either delay the release or
leave the ecosystem question open while the first users arrive. (user)

## AK. The registration API (session 8)

The owner's answers of 2026-10-07, late evening, to the four questions
decision AJ1 left open. The API is `crates/renyi_vm/src/extension.rs`,
the guide `docs/extensions.md`, the test
`crates/renyi_vm/tests/extension.rs`.

**AK1. An extension is built into a binary by a crate that depends on
`renyi` as a library: `renyi::main_with(vec![EXTENSION])` is the whole
program of such a binary, and the official `renyi` is
`main_with(Vec::new())`.** The crate `renyi` is a library and a binary;
an extension is an ordinary Rust crate that depends on `renyi_vm` for
the value types and holds an `Extension` value; a user builds a `renyi`
with it in a crate of three lines and `cargo install --path .`. The run
manifest names the extensions a run had beyond the standard library
(`extensions`, each `name version`), `reproduce` warns when they differ,
and `renyi version` lists them. The alternatives: Cargo features of the
`renyi` crate, which would make every extension a dependency of this
repository and leave a third party to fork it; or loading an extension
as a shared library at run time, which Rust's unstable ABI would reduce
to the C family of AF1, without lists, maps and records. (user)

**AK2. A declaration file and its natives meet by name: an `Extension`
names its modules with their declaration files and a table of `Native`
entries, each a module, a function name, the type of the function's
first parameter as the checker spells it, in full (`List of Integer`) or
by its head (`List`), or no receiver, and the Rust function;
`Registry::verify` holds the table and the declarations equal both ways,
and a binary with an extension that fails it does not start.** A lookup
takes the entry whose receiver is the full spelling, then the head, then
the entry without one. The standard library is the first extension
(`natives::standard()`): the eleven lookup functions under `natives/`
became the same keys as data, two hundred entries, and the two-way check
passed on them at once, which is the test of the API that decision AJ1
asked for. The alternative, a procedural macro reading the Rust
signature and writing the declaration, would move the place a reviewer
reads the capability and the failures into Rust source, for machinery
the table does not need. (user)

**AK3. A native of an extension sees what the standard library's natives
see: the `Vm`, the plain `Value`s of its arguments and the helpers of
`renyi_vm::natives` (`arg`, `text`, `int`, `small`, `list`, `map`,
`Vm::library_record`, `Vm::fail_variant` and the rest); `NativeFn` is
the one signature.** The standard library's two hundred natives are the
proof that it is enough. A typed layer (`FromValue`, `IntoValue`, a Rust
signature per Renyi signature) can be laid over it later without
changing it; it is not needed to write an extension. (user)

**AK4. An extension declares its capabilities from the kinds of
reference section 11 and adds none: `console`, `filesystem`, `network`,
`environment`, `time`, `random`, `process` and `foreign`, with `python`
to come by AJ3; a new kind is a decision and a change to the reference
and the checker's table.** The reviewer's vocabulary does not grow with
the extensions a binary carries, and a native declared under a kind
that takes a scope gets the scope check, the budgets and the guards of
the boundary from its `Path` or `Url` parameter without code of its own.
A denied call or one past a budget is reported through the variant of
the function's declared failure type named `PermissionDenied`,
`HostNotAllowed` or `ProgramNotAllowed`, or `OverBudget`, with the scope
as its one field (the first argument when the capability carries none,
as `serve` names its port): the rule the library's modules followed by
name, now followed by shape, so that an extension's own error type is
reported the same way; a function without such a variant crashes, as
before. The alternative, kinds registered by extensions, would make the
grant's names depend on the binary. (user)

## AL. The Python bridge (session 8)

The owner's answers of 2026-10-07, late evening, to the four questions
decision AJ2 left open. The bridge is `crates/renyi_vm/src/natives/python.rs`
with the worker `python_worker.py` beside it, the checker's rules
`crates/renyi_check/src/python.rs`, the guide `docs/python.md`, the
tests `crates/renyi/tests/python.rs` and the fixtures
`tests/conformance/python/` and `python_bad/`.

**AL1. A Python module is bound as a foreign module is (decision AF1): a
declaration file of the project, written by hand, that the manifest's
`python` section names: `"python": {"interpreter": "...", "modules":
{"analysis": {"package": "analysis", "symbols": {"mean": "average"}}}}`,
`interpreter`, `package` (the Python import name; the module's own name
when left out) and `symbols` optional.** Each function of such a file
needs `python("<package>")` and nothing else, fails with `PythonError`
and nothing else and takes no type parameters (`python-signature`); its
parameters are types that can `ToJson` and its result a type that can
`FromJson`, or nothing (`python-type`): what `std.json` renders and
parses is what crosses, records and sum types included, and the
checker's structural rule decides it. The bytecode file carries the
binding per function (`python` after `foreign`; the file is format 3).
The alternatives: declaration files shipped with the toolchain for
known packages, which would make the toolchain answer for the shape of
every Python package it names and go stale with each release of one; or
declaration files generated from Python's annotations, which most
packages do not carry and which say nothing about failures. (user)

**AL2. The protocol is one JSON object per line on the worker's
standard input and standard output: `{"call": N, "module": "...",
"function": "...", "arguments": [...]}` and `{"call": N, "result":
...}` or `{"call": N, "error": {"where": "import" | "call" | "result",
"kind": "ValueError", "message": "..."}}`, the arguments by position in
the order the declaration lists them; one worker per run, started at the
first call with the project root on its module path, ended with the
VM.** The worker's program is a text the VM passes with `-c`, so a
project installs nothing; what the Python functions print goes to the
standard error, so it cannot mix with the protocol; a replay never
starts the worker; a result the declaration does not mention is
dropped. The alternatives: a socket or a pipe pair with a framing of its
own, more machinery for the same messages; or one process per call
(`std.process`), which pays an interpreter start per call and loses the
module-level state between calls. (user)

**AL3. The interpreter is the manifest's `interpreter` when given, else
the environment variable `RENYI_PYTHON` when set, else `python3` and
then `python` on the PATH (`python` first on Windows, where `python3`
is usually the Store's stub); the first that starts and answers the
bridge's greeting is kept for the run.** A project that needs a
particular environment (a virtual environment, a version) names it in
the manifest, next to the modules that need it; a machine-wide choice
goes in the variable; a plain setup needs nothing. The alternative, the
PATH alone, would make a project's virtual environment a matter of the
shell the program was started from. (user)

**AL4. One failure type for the bridge, `PythonError` of `std.python`:
`Raised(exception, message)` for an exception the function let escape
(its class name and its text), `NotCarried(detail)` for a result the
declared type does not fit or that JSON does not carry,
`Unavailable(detail)` for an interpreter that does not answer, a module
that does not import, a function that is not there or a worker that
ended, and `PermissionDenied(package)` for a package outside the grant
(the rule of AK4); `python` takes a scope and no budget.** A declaration
cannot say which exceptions a Python function raises, so one variant
carries them all with the class name to match on; the other cases are
the bridge's own. The alternative, a failure type per declaration file,
would be written by hand for every package and checked by nothing.
(user)

## AM. The Python binder (session 8)

**AM1. `renyi bind --python <package> [--module <name>] [--to
<directory>]` writes a Python module of the project from the package
itself: the declaration file `<directory>/<name>.ry` (the module named
as the package when `--module` is left out) and the module's entry in
the manifest's `python` section (written when the manifest exists,
printed otherwise), through the interpreter of decision AL3 (the
manifest's, else `RENYI_PYTHON`, else the PATH) running an inspection
script that imports the package with the directory first on its module
path and reports its functions. The functions are the names of
`__all__` that are functions when the package defines it, else the
public functions the module itself defines, in definition order; a
function's positional parameters are declared in order and required, a
default or not, `*args` and `**kwargs` are left out, a keyword-only
parameter with a default is left out and one without a default leaves
the function as a comment with the reason, as does a function whose
signature Python cannot give; a name becomes a Renyi name as a C name
does (snake case, a reserved word prefixed `py_`, a single letter
`argument_<n>`), a rename recorded in `symbols`; the purpose is the
docstring's first line as a sentence, else the signature as Python
prints it, cut to the line width.** The four questions of 2026-10-07
and their answers: (i) the same command as the C binder, with
`--python` in place of the header, rather than a command of its own:
one place for "write me the declaration file"; (ii) the module name
defaults to the package name; (iii) every positional parameter declared
and required, since the bridge passes by position (AL2) and a Renyi
call names every argument anyway; leaving out the parameters with
defaults would hide them from the caller, and passing by keyword would
be a new protocol; (iv) `__all__` first, the Python convention, so that
a package which says what it exports gets exactly that. (user)

**AM2. The types the binder writes: `int` is `Integer`, `float` is
`Float`, `str` is `Text`, `bool` is `Boolean`, `bytes` is `Bytes`;
`list[T]`, `Sequence[T]`, `MutableSequence[T]` and `Iterable[T]` are
`List of T`, `set[T]`, `frozenset[T]`, `AbstractSet[T]` and
`MutableSet[T]` are `Set of T`, `dict[str, T]`, `Mapping[str, T]` and
`MutableMapping[str, T]` are `Map of Text to T`, `Optional[T]`, `T |
None` and `Union[T, None]` are `maybe T`, `Annotated[T, ...]` is what
`T` is, a bare `list`, `set` or `dict` is the same of `JsonValue`; a
result annotated `None` is no result; everything else, a missing
annotation included (`Any`, a class, a `tuple`, a union of two types, a
`dict` with other keys), is `JsonValue` of `std.json` at that position,
and the function carries a comment naming what stands as `JsonValue`.**
The bridge carries what JSON carries (AL1), so the mapping stops where
JSON does; `JsonValue` keeps every function callable from the first
run, and the comment tells the editor where a record type would serve
better. The alternative, skipping a function whose annotation the
binder does not know (the C binder's rule), would drop most of a
typical package, whose functions are annotated partially or not at
all. (user)

## AN. The resident world (session 8)

**AN1. The toolchain keeps a resident world of a project between
calls, in a crate of its own, `renyi_workspace`: the `.ry` files under
a directory and the files of the dependencies they import, each kept
with its text, its canonical layout, its syntax tree, the fingerprint
of its declarations and the check of every item of it. A refresh stats
the directory's files and reads again only those whose size or
modification time changed (and those stamped within two seconds of
their last write, since file systems round the time), formats and
parses again only those whose text changed, declares the world again
from the kept trees when anything changed, checks again only the items
whose canonical text changed (every item when the fingerprint of any
file changed, a file appeared, disappeared or stopped parsing, or the
manifest changed) and rebuilds the map from the result. What it gives
is what a fresh `renyi index` and `renyi check` give, byte for byte,
which a test holds on the corpus, the compiler, the starter pack and a
project taken through every kind of change. `renyi mcp` holds one; a
text may overlay a file (an editor's unsaved buffer).** The owner's
four answers of 2026-10-07: (i) a file-level parse cache with
item-level rechecks rather than a whole-project rebuild or a
per-definition dependency graph: a body's check depends on the
project's declarations and on its own text and on nothing else, so an
unchanged body under unchanged declarations has the same check, moved
to where the item now sits; the fingerprint is the text of everything
a body elsewhere can see (the module's head, each function's signature
and docs, each type, ability and constant whole, each test's name,
needs and recording), so a change to a body costs one check and a
change to a signature costs them all, the cheap rule that is also
exact; (ii) a stat of the files on every call rather than a file
watcher: no thread, no platform notification API, the same answer on
every machine; (iii) a crate of its own rather than a part of
`renyi_check` or `renyi_index`: the checker and the index stay the
whole-project functions the self-hosted judges mirror, and the language
server, the watch of `serve` and the embedding API take the crate
without the binary; (iv) AN2. Measured on `compiler/` (18 files, 23.6
thousand lines; a development build on Windows): the first
`project_map` call of `renyi mcp` 1.5 seconds, every later one 12
milliseconds, the map rendered each time. Closes open item R5-5 of
`05-agent-tooling.md`, which decision T4 had left at the file level.
(user)

**AN2. The language server is a standard LSP server, `renyi lsp`, over
standard input and output, on the resident world of AN1: one
`Workspace` per workspace folder, the editor's unsaved buffers as
overlays.** A protocol every editor speaks rather than a VS Code
extension of its own: the extension of decision AI2 gains a client of
a few lines, and every other editor gets the same server. It follows
AN1; the scope of its first version (diagnostics, hover with the
purpose and the signature, go to definition, document symbols) is
recorded with it. (user)

**AN3. The server checks the files as written, not in canonical layout:
the workspace of AN1 has a mode for it (`as_written`), in which the
tree, the diagnostics and the map refer to each file's text on disk or
in its overlay, as `renyi check` reads it, and the map is built only
when asked for. The server publishes the diagnostics of every file of
the folder, open or not, as `renyi check` reports them (the layout's
included), after every change and only for the files whose diagnostics
changed; it synchronizes whole texts, counts positions in UTF-16 code
units (the protocol's default) and answers hover for a library
definition from the library's declaration files, with no location to go
to. The VS Code extension carries the client, bundled into one file at
release time, and the setting `renyi.path` names the binary.** An
editor's positions must be those of the text in the buffer, which the
formatter would move; the map's hashes and line counts are a tool's
business, not the editor's, so the server never pays for them. Whole
texts rather than incremental changes: a file is parsed again whenever
it changed, so applying deltas would buy nothing but a second copy of
the text. Every file of the folder rather than the open ones: an error
a change causes in another file is the point of a project-wide check,
and the Problems panel is where it is seen. (derived)

## AO. The watch of `serve` (session 8)

The third of M5's four steps in the owner's order (2026-10-08). The
owner's four answers, each the recommended option.

**AO1. `renyi serve [--watch] [options] <file.ry> [arguments]`: without
`--watch`, `renyi run`; with it, the files of the program's project are
looked at every half second while no request is waiting, through a stat
by the resident world of AN1, and when one changed the program is
compiled again; when it checks clean, `main` is run again on the new
version between two requests, the listening socket kept open and handed
to the new program so that no request is lost, and the reload's message
names what the semantic diff found, a changed signature among the rest;
a version with errors is reported and the last good one keeps
serving.** This replaces the mechanism Q4 described, the swap of the
changed definitions by content hash with a changed signature refused;
Q4's promise (the new code at the next request, no request lost,
nothing to migrate) stands. (i) Running `main` again rather than
patching definitions: a Renyi service has no state outside its request
(Q4's own argument), so a restart between requests is the swap with
nothing to migrate, and the VM keeps its program borrowed and compiled
to machine code whole; reference-counted code objects and a swap at
the next call (open item R7-3) are not needed. (ii) A clean check
rather than a refused signature change: the whole program is checked,
so a changed signature either fits every caller or the check fails;
what the diff finds is reported, not refused. (iii) A command of its
own rather than an option of `run`: `run` stays the one-shot command,
and `serve` reads as what a service is started with; `run` and `test`
refuse `--watch`. (iv) A poll of the resident world rather than a file
watcher, AN1's rule (ii) again: no thread, no platform API, the same
answer on every machine; a change to an unrelated file of the
directory costs one compile, which finds the program's sources
unchanged and reloads nothing. Closes R7-3 of `07-system-design.md`.
(user)

## AP. The embedding API (session 8)

The fourth of M5's four steps in the owner's order (2026-10-08), which
completes M5. The owner's four answers, each the recommended option,
then the derived choices.

**AP1. The embedding API is a Rust API first, `renyi::Sandbox`: a host
loads a module from a source file or a text with a `renyi::Grant` and
calls its public functions by name with `Value` arguments, the value or
the failure coming back; the grant is spelled as a `needs` clause is, in
a text or in a JSON file with the memory budget beside it (`{"grant":
"console, network.http(\"host\") at most 60 per minute", "memory": "256
megabytes"}`), and `renyi run --sandbox grant.json` runs `main` under the
same grant; the memory budget is a setting of the sandbox, not a clause
of the grammar, counted by a counting allocator as the bytes a call
holds above the level at its start, and a call that exceeds it fails as
a whole with `OverMemory`; every public function of the module is
callable, with the JSON Schema of those marked `expose as tool` beside
them.** The C API of A5 is a layer over this one, for later; Python and
JS hosts go through it. (i) Rust first: the isolation and the grant are
the work, a language binding is a shell around them. (ii) The `needs`
syntax for the grant: one spelling for what a program may do, wherever
it is written, read by the language's own parser (`parse_grant`), so
budgets and guards come with it and nothing enters the frozen surface
(V11). (iii) A setting rather than `at most 256 megabytes memory` in the
grammar (open item R7-1, closed): the budget is the host's to set, not
the program's to declare; a counting allocator because the VM's values
are Rust allocations, so the allocator sees every byte, and the count
costs one load per allocation while no budget is in force. (iv) Every
public function rather than those with `expose as tool`: visibility is
the module's own word on what may be called, and the tool clause is
documentation for agents (D6), which the API reports beside it. (user)

**AP2. A call whose `needs` the grant does not cover is refused before it
runs, as `--deny` refuses a program at start (E3); inside the call the
grant stack narrows as under `run`. The grant's budgets count across
the calls of one sandbox as one run; each call runs on a VM of its own,
and nothing else survives between calls. A budgeted run stays on the
interpreter, as a narrated or profiled one does (AG3), and the budget is
read at every call, primitive and loop turning. One budget is in force
at a time in a process, and it counts the process's allocations. The
official binary declares the counting allocator, `renyi::Allocator`
(mimalloc under the count), in `src/main.rs` rather than in the library,
so that a host keeps its own allocator and wraps it in `renyi::Counting`
when it wants the budget; a grant with `memory` is refused at load
without one. A bytecode file is refused by `Sandbox::load`, since it
carries no visibility; a format that does is a change to
`compiler/bytecode.ry` and the file, for later. `renyi run --sandbox`
refuses to start when `main` needs a capability the grant does not
cover, naming it; `record` and `serve` take the option too, and the
effective grant is what the manifest names.** A refusal before the call
tells the host what the grant lacks in one message, where a boundary
failure would come back as the function's own failure type; the
interpreter alone because the safe points of the generated code are not
all the interpreter's, and a sandbox is about safety, not speed; the
allocator in the binary because a library that declares the global
allocator cannot be linked into a host that declares its own. (derived)

## AQ. The profile-guided round on strings and JSON (session 9)

**AQ1. The first of the interspersed performance items the owner set on
2026-10-07 (the profile of decision AG5 on the two benchmarks the
positioning compares with CPython), done and measured: `std.json`
decodes without cloning the type's metadata per record and without
building the error-path strings of every item and field, and renders
text with no `Json` tree in between; the reader and the writer of
`renyi_json` copy runs of plain bytes at once; the one-character texts
of the ASCII range come from a table kept per thread; a compiled pattern
of `std.regex` and `Text.matches` is kept per thread; and an
interpolation joins its pieces where they lie on the stack.** The round
began on the owner's machine (the profiles of `perf` on the release
build under WSL, about a thousand samples each) and was paused on
2026-10-08 with the change written as a script (commit 44763c4); session
9, the first in the cloud environment, applied it, ran the gates and
measured it. What the profiles said: on `bench/strings.ry` (a hundred
thousand pieces joined, 1.2 million characters counted) 40% of the time
in page faults, 19% in the interpreter loop, 11% under `characters()`
(an allocation per one-character text, a 29 MB vector grown by
doubling), 7% moving a primitive's arguments into the scratch buffer
(1.2 million calls of `contains`), 6% cloning and dropping values; on
`bench/json_round_trip.ry` (two thousand records rendered and read back
fifty times) 18% in `decode`, 13% in the error-path strings
`"{path}[{index}]"` and `"{path}.{key}"` made for every item and field
whether or not an error follows, 10% in the reader (`from_utf8` once per
UTF-8 sequence), 6% in `encode`, 15% in the allocator, 5% cloning the
`TypeMeta` per record. The change, in five parts: (i) the decoder keeps
its position as a chain of borrowed segments (`Path`) rendered only for
a `Mismatch` or `Constraint` error, borrows the type's metadata and a
field's key under the exact naming (`Cow`), substitutes a field's type
only for a type with parameters, and `parse` no longer copies its
argument; (ii) the encoder walks a value once into a `Sink`: `TreeSink`
builds the `Json` tree `encode` returns (the recording,
`http.post_json`, `server.ok_json` and the Python bridge keep it) and
`TextSink` writes `render`'s text with no tree in between, held equal to
`write_json` by a unit test on both layouts; the reader copies a run of
plain bytes at once and `write_string` the runs between escapes; (iii)
`Value::character(c)`: the ASCII one-character texts from a per-thread
table, used by `characters()`, `split("")` and a loop over a text, the
list allocated once at its exact length; (iv) a compiled pattern cached
per thread (256 at most, then the cache is emptied), for `std.regex` and
`Text.matches`; (v) `Vm::op_concat` joins the pieces where they lie on
the stack, the text sized first, an Integer written straight into it.
Conformance case 54 (`json_paths.ry`) holds what these paths print: the
path of every `Mismatch` and `Constraint` error, the control characters
and the non-ASCII text of a document on both layouts, the three namings,
`characters()`, `split("")`, a loop over a text, every kind of value in
a hole, and a thousand matches of one pattern and three hundred of
different ones; its expected output was printed by the binary before the
change. Measured on 2026-10-08 in the cloud environment (Linux, four
cores, the release build with rustc 1.97.0, the two binaries from the
same tree with and without the change, `cachegrind` for the user-space
instructions and `tools/bench.py --runs 5` back to back for the
wall-clock time): user-space instructions on `strings` 1.74 to 1.39
billion on machine code (-20%) and 1.97 to 1.62 billion on the
interpreter (-18%); on `json_round_trip` 2.34 to 1.00 billion (-57%) and
2.36 to 1.01 billion (-57%); on the compiler's self-check (`checker.ry`
on `bodies.ry`) 13.40 to 13.14 billion (-2.0%) and 13.12 to 12.84
billion (-2.1%). Wall-clock time, the best of five: `strings` 282 to 226
ms on machine code and 284 to 262 ms on the interpreter against
CPython's 106 ms; `json_round_trip` 314 to 126 ms and 303 to 125 ms
against 230 ms (from 0.7 to 1.8 times CPython's speed); the self-check
2895 to 2679 ms and 2572 to 2496 ms; `primes`, `records` and `hello`
unchanged within the noise of the machine, which moved a repeated
measurement by a tenth (CPython's own `primes` twin by a fifth between
the two rounds), so that a wall-clock difference under a tenth says
nothing here and the instruction counts are the measure, as in AG6. The
CI numbers of commit a2b0702 to beat were `strings` 179 ms and
`json_round_trip` 386 ms against CPython's 43 and 89 ms. Where the rest
goes, by `cachegrind` on the interpreter after the change: on `strings`,
40% in the interpreter loop (`run_frames`), 30% in the primitive
boundary once per character (`call_primitive`, the move of the arguments
into the scratch buffer, `run_primitive`, `text_contains`, the drops of
the arguments and of the drain, `guarded`, `effect_of`), 7% dropping and
4% cloning values, 5% the `contains` search itself, 3% `iterator_next`,
2% `characters`, 2% `op_concat`, the allocator under 2%; the page faults
of the 29 MB list of 1.2 million one-character texts, now one allocation
at its exact length, are kernel time that no instruction count shows,
which is why the interpreter's wall-clock gain is smaller than its
instruction gain. On `json_round_trip`, 21% in the reader (`string`,
`value`, `expect`), 8.5% in `write_string`, 8% in `decode_at`, 4% in
`walk`, 7% growing vectors and texts, 9% in the allocator, 5% in the
interpreter loop, 2% parsing Integers, 3% in the keys (`Naming::key`,
`TextSink::key`), 1% finding a field by name among the object's entries:
the plain cost of reading and writing the format. Not done, on purpose:
`Text` stays `Rc<str>` (decision X3), so `change out to "{out}{piece}"`
in `compiler/` still copies the accumulated text per piece; the pure
primitive boundary stays as AG6 left it (the move of the arguments into
the scratch buffer, 1.2 million times on `strings`, is the next item);
`Text.matches` is not on the compiler's path (three calls in
`project.ry` and `refine.ry`), so the cache changes nothing for the
self-check. Against the targets the owner named with the item (strings
and JSON at CPython's speed, the compiler's self-check twice as fast):
JSON is past CPython (1.8 times its speed here, from 0.7); strings
stands at half of CPython's speed (from four tenths), with the loop and
the primitive boundary per character left, which are the baseline JIT's
to take out; the self-check moved by two percent, as AG6 foresaw, since
its time is in the value operations of the interpreter, not in strings
or JSON. What follows, in the owner's order: the baseline JIT (AG5, item
iii), then the showcase application. (user)

## AR. The baseline JIT (session 9)

**AR1. The baseline JIT of decision AG5 (iii) is built in this order,
each step measured and kept by the rule of AG6 (the measure is the
compiler's self-check, `compiler/checker.ry` on `compiler/bodies.ry`, by
its count of user-space instructions on the interpreter and on machine
code, with `bench/records.ry` and four micro-benchmarks beside it; a
step stays when it cuts the self-check's instructions, and the round
ends when a step brings under 2% or the self-check runs twice as fast as
AG6 left it): (i) fused operations in the bytecode itself, `LoadField`
first (a field of a local, read in its slot), so that both tiers gain
and the interpreter's look at the next op (AG6) goes; (ii) direct calls
between generated functions, with the parameters and the result the
checker types as Integer, Boolean or Float passed in registers, the
frame still pushed on the VM's frames for the handlers, the grant and
the hand-back; (iii) the layout of `Value` pinned (`repr(C)`), so that
the generated code clones, drops, loads, stores and reads fields inline,
with the slow paths as helpers.** The owner's four answers of
2026-10-08, asked after the round of AQ with the micro-benchmarks of the
two tiers in hand (instructions per turn of a loop, `cachegrind`, the
release build; machine code against the interpreter): a call with an
Integer parameter 668 against 1133; a field of a local 1043 against 818;
a call with a record parameter that reads a field of it 1535 against
1183; a record built and a field of it read 1357 against 1480. The
machine code of AG1 does for a boxed value what the interpreter does,
through a helper per op on the VM's stack, so it loses on the operations
ordinary programs are made of, where AG6's interpreter reads a field in
the slot and the helper boundary clones and drops the holder; the
self-check gains nothing from it (AG5). Step (iii) was the owner's
choice over the two steps the session recommended (the fusions and the
calling convention first, the layout only after a profile of what
remained): the ceiling of the helper-call design is the interpreter's
own speed less the dispatch, and the self-check's time is in the clone,
the drop and the stack traffic (AG5), which only inline code reaches.
Fused operations in the bytecode rather than in the code generator
alone, the question AG6 had left as a format question (Z1): the
interpreter is the reference and most of what runs, and a fusion both
tiers share is one rule in the two emitters instead of a look at the
next op in the loop; the format number moves with each addition (Z2).
(user)

**AR2. Stage 1 of AR1, done and measured: `Op::LoadField { slot, name,
site }` is `Load(slot)` followed by `Field { name, site }`, the field of
a local read in its slot, so that the holder is neither cloned nor
dropped; both emitters emit it for `x.field` and `self.field` on a local
that is not a move receiver and for the field of a variant pattern, the
interpreter's look at the next op (AG6) is gone, the generated code
calls one helper where it called two, and the bytecode file is format
4.** In the compiler written in Renyi, 3,859 of the 4,480 `Field` ops
follow a `Load` (static), and on the self-check `Load` was 29% and
`Field` 6% of 101 million ops run. Measured as AR1 says (`cachegrind`,
the release build, the binary of commit 2fcab99 against the stage): the
self-check 13.14 to 12.51 billion instructions on machine code (-4.7%)
and 12.84 to 12.63 billion on the interpreter (-1.6%);
`bench/records.ry` 571 to 516 million (-9.7%) and 655 to 653 million
(-0.3%); per turn of the micro-benchmarks, machine code then the
interpreter: the field of a local 1043 to 902 and 818 to 811, the call
with a record parameter 1535 to 1394 and 1183 to 1161, the record built
and read 1357 to 1216 and 1480 to 1464, the call with an Integer
parameter 668 unchanged and 1133 to 1104 (the look at the next op that
every `Load` paid). The step stays by the rule. The profiler's report
has a table of operation pairs since this stage (`operations by pair`,
the kinds run one after the other in one code object), which is how the
next fused operations are chosen; on the self-check after `LoadField`
the pairs that remain are `Load Load` 6.1 million, `Load Call` 6.0,
`Load Const` 4.7 and `Const Binary` 4.7, `Store Load` 3.8, `JumpIfFalse
Load` 3.0, `Binary JumpIfFalse` 2.4, `Store LoadField` 2.1, `Binary
Store` 1.6, `Dup JumpIfTrue` and `Binary Dup` 1.6 (the short circuit of
`or`), `Load IsVariant` and `IsVariant JumpIfFalse` 1.1 (the dispatch of
a `match`), of 96 million; the pairs across a statement's end (`Store
Load`, `JumpIfFalse Load`, `Store Jump`) fuse nothing, and each of the
others saves a dispatch and a push or a pop, about a percent of the
self-check apiece by the rule of thumb of 30 to 40 instructions per op
saved, which is under the 2% a step must bring: the fusions stop here
and the round goes on to stage 2. (user)

**AR3. Stage 2 of AR1, done and measured: a generated function calls a
compiled callee's body directly, its Integer, Boolean and Float
arguments and result in registers, through a second function per code
object.** Every code object now compiles to two Cranelift functions: the
body (`renyi_direct_*`), whose signature is the VM, the frame's base,
the pc to enter at, whether a direct call entered, then the typed
parameters in slot order, and which returns a status (`D_RETURNED` with
the bits of a typed result; `D_BOXED` or `D_FAILURE` with the value on
the caller's stack; `D_DEOPT`, `D_INTERRUPT`, `D_STAY`) beside the bits;
and the trampoline (`renyi_code_*`), with the `Entry` signature the VM
calls as before, which at the start takes the typed parameters from the
frame's slots under the checks the prologue used to make (a parameter
that does not fit hands the frame back), at a loop header passes zeros
for the body to overwrite from the slots, and maps the body's status to
the entry's; the body leaves a typed result in a register only for a
direct caller, and on the stack as before for a trampoline's, so that a
call from the interpreter costs what it cost. A call site to a declared
function with a code object loads the callee's body from a table the JIT
keeps by code object (filled when the code is compiled), asks
`rt_direct_entry` when the table has none yet (the callee is compiled
when it is hot enough), and goes through `rt_call` as before when there
is none or when the count of generated frames on the machine stack,
which the generated code reads and moves in place by its address in the
JIT, is at `DEPTH_LIMIT`; else it boxes the operands of the boxed
parameters onto the stack where they lie, has `rt_direct_frame` push the
callee's frame (the boxed arguments moved into their slots by a mask of
the parameters, the others `Nothing` until a hand-back writes the
registers into them), and calls the body with the typed operands from
its registers. A typed result comes back in a register and a boxed one
on the stack, both handled in place; a failure, a frame handed to the
interpreter (run to its end) and an interrupt (its frame abandoned) go
through `rt_direct_after`, which leaves what `rt_call` would. A site is
direct only when every typed parameter has a typed operand in the
caller; the analysis holds a parameter's slot to the kind its declared
type gives it (a function that stores another kind into the slot stays
with the interpreter), so that the caller derives the callee's signature
from the function's declared types alone and a callee compiled later
fits. The return of a typed result of the declared kind leaves the frame
without a push (`rt_leave_typed`); a boxed value where the declared
result is typed is unboxed on the way out when it fits
(`rt_leave_unbox`), else returned boxed. On the self-check with every
code object compiled, all 555 compile (the analysis rule rejects none),
5.23 million calls go from generated code and none to the interpreter;
with the default tiering 112 compile and 126 thousand of 4.1 million
calls reach the interpreter (cold callees). Measured as AR1 says,
against the binary of stage 1: the self-check 12.51 to 12.32 billion
instructions on machine code (-1.5%) and 12.63 to 12.91 billion on the
interpreter (+2.2%, whose loop the stage does not touch: the code layout
of the binary, which AG6 saw move the loop by up to 3%);
`bench/records.ry` 516 to 518 million on machine code; per turn of the
micro-benchmarks on machine code: a call with an Integer parameter whose
argument is in a register (a literal loop bound; a bound read from the
arguments is a `maybe Integer`, boxed, and its loop variable with it)
542 to 305 (-44%), the same call with a boxed argument, through
`rt_call` and the trampoline, 668 to 719 (+8%: the trampoline's own call
and the status mapped), the call with a record parameter 1394 to 1236
(-11%), the field of a local and the record built unchanged. The step
cuts the self-check and stays; it brings under the 2% at which AR1's
rule ends the round, but the layout inlining of stage 3 is what the
owner chose the round for and the register convention is its ground, so
the session goes on to stage 3 and leaves the question to the owner, who
reads this entry first. What the profile of the self-check on machine
code says after the stage: the frame protocol (`rt_direct_frame` 3.4%,
the locals' resize 2.1%, `leave_frame` 1.9%, `frame_grant` 1.2%) and the
primitive boundary (`status` 3.5%, `call_from_stack` 2.6%,
`call_primitive` 2.2%, the argument move 2.1%) are the next largest
items after the clone (4.1%) and the drop (6.9%) of values and the
interpreter's own run of `main` and the cold code (7.8%). (user)

**AR4. Stage 3 of AR1, done and measured: the layout of `Value` and of
`Int` is pinned (`repr(C, u8)`), the VM's stack, frames, handlers and
field-site cache and the fields of a record and of a variant are pinned
vectors (`Pinned<T>`: the pointer, the length and the capacity, in that
order), and the generated code does the value operations, the frame
protocol of a direct call and the field read in place, a helper left
only for each slow path.** The constants of the layout live in
`value::layout` and a unit test holds them to the types: the tag at
offset 0 in declaration order, the payload at 8; an `Rc` payload points
at its allocation, whose first word is the strong count (an `Rc<str>`
and an `Rc<[u8]>` are fat pointers whose address is the allocation too);
an `Integer` holds its `Int` with the small-or-big tag at 8 and the
payload at 16; the thirteen tags with an `Rc` payload are one mask
(`RC_TAGS`); a `Record` and a `Variant` lie sixteen bytes into their
allocation, the type, the tag and the field vector at fixed offsets.
Three sub-stages, each measured as AR1 says (the binary before against
the binary after, `cachegrind`, the release build). (3a and 3b) The
pinned layouts, then `Load`, `LoadMove`, `Store`, `Pop`, `Dup`, `Const`
and `Nothing` on a boxed value in place: the clone is the three words
copied and the strong count incremented when the tag is in the mask or
the Integer is big, the drop the count decremented and `rt_drop_at` when
it was one, a push past the capacity `rt_grow`; the top of the stack is
boxed (`push_typed_value`) and unboxed (the tags read, the value that
does not fit stays for the interpreter as before, a non-Boolean
condition crashes through `rt_take_bool`) in place, and `IsNothing`,
`IsFailure`, `JumpIfAbsent` and `JumpIfFailure` read the tag. The
self-check 12.32 to 12.10 billion instructions on machine code (-1.8%)
and 12.91 to 12.90 on the interpreter; `bench/records.ry` 518 to 459
million (-11.5%) and 663 to 658; per turn of the micro-benchmarks on
machine code: the typed call 305 to 311, the call with a boxed argument
719 to 633, the field of a local 901 to 731, the record call 1236 to
1055, the record built and read 1215 to 1109. (3c) The frame of a direct
call pushed in place (the boxed arguments moved to their parameters'
slots by the mask, which is static, every other local's tag written
`Nothing`, the record written with the caller's grant when the callee
narrows nothing, else the grant from `rt_frame_grant`; `rt_room` and
`rt_grow_frames` for the capacities; the count of calls kept), and every
return leaves the frame in place: the boxed locals and the boxed
operands under the result released inline up to `RELEASES_INLINE` of
them, else through `rt_truncate`, the handlers cut to the frame's
height, the frame popped, a boxed result moved to the frame's first
slot, a typed one back in a register to a direct caller and boxed on the
stack to a trampoline's, the unbox on the way out of AR3 by the tags;
`rt_direct_frame`, `rt_leave_*`, `rt_return_*` and
`Vm::push_frame_direct` are gone. The self-check 12.10 to 11.69 billion
(-3.4%) and 12.90 to 12.94 on the interpreter; `bench/records.ry` 459 to
464 million (+1.1%); per turn: the typed call 311 to 119 (-62%), the
boxed call 633 to 587, the record call 1055 to 873, the field and the
record built unchanged. (3d) `LoadField` reads the site's cache entry
and the holder's tag in place: a record or a variant of the cached type
with the cached tag and the index in range gives its field copied with
one more reference, anything else goes through `rt_load_field`, which
fills the cache as before; and the pinned vector's `insert`, `extend`,
`split_off` and `resize` work on the three words instead of going
through a `Vec`. As first written, with a path per holder and a push
that checked the capacity each time, the step measured the self-check
11.69 to 11.70 billion (+0.1%) while `bench/records.ry` fell 464 to 429
million (-7.5%) and, per turn, the field of a local 730 to 648, the
record call 873 to 790, the record built 1109 to 1005: the IR had grown
from 131 to 236 thousand Cranelift instructions in 19 to 34 thousand
blocks since 3b, and Cranelift's own work from 5.6% to 12.5% of the
self-check, which ate the run-time gain. The IR made compact (the
reference-count test one branch, both payload words read and one
selected; the stack's capacity checked once in the prologue for the
frame's locals and its deepest operand stack, so that no push checks;
one path for a record and a variant in the field read, the type at one
offset in both and the rest selected; four releases in place at a
return, else `rt_truncate`): the self-check 11.69 to 11.66 billion
(-0.25%), the interpreter 12.94 to 12.90, `bench/records.ry` 464 to 437
million (-5.8%), per turn the typed call 119 to 131, the boxed call 587
to 605, the field of a local 730 to 677, the record call 873 to 849, the
record built 1109 to 1041 (the first form's 648, 790 and 1005 were lower
per turn: the test without branches and the selected path cost
instructions at run time that the compile time pays back on the
self-check); the IR 200 thousand Cranelift instructions in 15 thousand
blocks, Cranelift's own work 10.3% of the self-check and the generated
code's 16.6%. The stage as a whole, against the binary of stage 2: the
self-check 12.32 to 11.66 billion (-5.4%), the interpreter 12.91 to
12.90, `bench/records.ry` 518 to 437 million (-15.6%); the round so far,
against the binary AQ left: the self-check 13.14 to 11.66 billion
(-11.3%). Each sub-stage cut the self-check and stays by the rule, 3b
and 3d under the 2% at which AR1 ends the round, so the round's end is
the owner's call, with this in hand: Cranelift's compile time is now a
cost every inline sequence pays at each of the 112 compilations, and a
cheaper register allocator does not pay (`single_pass` halves
Cranelift's time, 658 to 339 ms on the self-check, and loses more in the
code: the self-check +4.2%, `bench/records.ry` +11%, the record call per
turn 789 to 1025; `RENYI_NATIVE_REGALLOC` keeps the comparison as a
development aid); what remains on the self-check's profile after the
stage: the generated code 16.6%, the interpreter's `run_frames` 8.7%
(`main` and the 443 code objects called but cold under `HOT_FACTOR`),
Cranelift and regalloc2 10.3%, the drop glue 5.1% and `Value::clone`
2.2% in the helpers and the primitives, the primitive boundary
(`call_from_stack` 3.0%, `status` 2.9%, `call_primitive` 2.4%,
`run_primitive` 1.3%), the boxed binary operations (`binary_values` and
`rt_binary` 1.8% each), the interpreter's `push_frame_in_place` 1.7% for
the cold callees, `Value::eq` 1.7% and mimalloc about 3%. (user)

**AR5. The tiering of the cold code, the owner's choice for the round's
next step (2026-10-08): `HOT_FACTOR` is 8000, where it was 2000, and the
loop-header entry keeps the same factor as a call.** The measure is
AR1's (the self-check's instructions, `cachegrind`, the release build of
stage 3; `RENYI_NATIVE_HOT` sets the factor for one run, so one binary
measured every value). The factor swept: 500 gives 13.15 billion
instructions (205 code objects compiled, 9,676 ops, Cranelift 1.1
seconds), 1000 12.09 (149 compiled), 2000 11.66 (112 compiled, 4,703
ops), 4000 11.24 (77 compiled), 6000 11.16 (62 compiled), 8000 11.13 (51
compiled, 1,477 ops, Cranelift 146 ms), 12000 11.23 (40 compiled), 16000
11.39 (38 compiled), 32000 11.78 (23 compiled): Cranelift's own work is
proportional to the size of what it compiles, about a quarter of a
million instructions per op (the IR is 42 Cranelift instructions per op
and each costs about 6,000 to compile), while a compiled op saves the
interpreter's dispatch and boxing, a few tens of instructions each time
it runs, so a code object pays for its compilation only after some ten
thousand runs of each of its ops; the 443 code objects called but cold
under 2000 ran about a billion instructions in the interpreter, and
compiling them would have cost three. `bench/records.ry` goes the other
way, 433 million at 500, 437 at 2000, 442 at 4000, 451 at 8000, 469 at
16000: a short program with one hot loop wants that loop compiled at
once and nothing else, and every doubling of the factor is more turns
interpreted before the entry at the loop header. A second factor for
that entry, so that a running loop compiles sooner than a function that
is called (`LOOP_FACTOR` 500, `RENYI_NATIVE_HOT_LOOP`), was built and
measured: with the call factor at 2000 the self-check rose from 11.66 to
12.05 billion (137 compiled: the loops of twenty-five more code objects,
whose turns did not pay for them), with 4000 from 11.24 to 11.77 and
with 8000 from 11.13 to 11.67; the mechanism is not kept, and
`bench/records.ry` pays the 3.2% (437 to 451 million) for the
self-check's 4.5% (11.66 to 11.13 billion), by the rule. What a better
rule would need is a prediction of the runs to come, which a count of
the runs so far does not give; the shape to try next, if the round goes
on, is a cheaper compilation for the first tier (an IR without the
inline sequences of AR4 for a code object that just became warm, the
full one when it stays hot), so that the cost of being wrong about a
function falls instead of the threshold rising. (user)

**AR6. The baseline JIT's round ends (the owner's answer of 2026-10-08),
with the machine code doing the value operations, the frame protocol and
the field read in place and the tiering at 8000.** The round against the
binary AQ left: the compiler's self-check 13.14 to 11.13 billion
instructions on machine code (-15.3%) and 12.84 to 12.90 on the
interpreter (+0.5%, the code layout of AR3); `bench/records.ry` 571 to
451 million (-21%) and 655 to 654 on the interpreter; per turn of the
micro-benchmarks on machine code, the typed call 668 to 130 (-81%; 605
with a boxed argument), the field of a local 1043 to 677 (-35%), the
call with a record argument 1535 to 849 (-45%), the record built and
read 1357 to 1041 (-23%), the interpreter's per turn up 2 to 3% with the
layout. AR1's rule had ended the round twice (AR3, AR4) before the owner
did; what the profile leaves is in AR5 and the handoff: the generated
code itself (16.6% of the self-check), the cold code in the interpreter
(8.7%), the primitive boundary (about 9%), Cranelift's compile time
(half of what it was under 2000), the clone and the drop in the helpers
and the primitives (about 7%); the shape to try first in a later round
is the cheaper first tier of AR5. The owner's other answers of the day:
the measure stays the self-check's total instructions, compile time
included; the micro-benchmarks live in `bench/micro/` with
`tools/measure_native.sh`; stage 3d stays in its compact form. (user)

## AS. `renyi build` (session 9)

**AS1. `renyi build` writes an image file, `.ryi`: the program's
bytecode with the machine code of every code object, which `run`,
`record`, `test` and `reproduce` take in place of a `.ry` or a `.ryc`
and run without compiling; `build --exe`, a second step, copies the
`renyi` binary and appends the image, the self-contained executable of
A1. The generated code holds no address: every helper, the table of
compiled bodies, the counters and the constants are reached through the
VM pointer, so the image is a plain block of bytes placed in executable
memory at load, with a header naming the `renyi` version, the format of
the generated code and the target with its CPU features; a mismatch
refuses the image with a diagnostic whose fix is to build again. `build`
compiles every code object the analysis accepts (the rest stays
bytecode, as in a run), at the optimisation level the self-check
measures best once compile time is no longer paid at run time; the JIT
uses the same address-free code.** The owner's four answers of
2026-10-08, after the baseline JIT's round closed (AR6) with the
wall-clock picture of the release build on this machine: `primes` 64 ms
(CPython 587), `strings` 133 (84), `records` 66 (218), `json_round_trip`
98 (161), the self-check 1963 ms on machine code and 1918 on the
interpreter, from 2759 and 2025 before the two rounds; machine code now
wins on ordinary programs, which AG5 made the condition for the image.
What an image removes is the cost that remains in every run: Cranelift's
compilation (five percent of the self-check at `HOT_FACTOR` 8000, and
most of a short program's time), the warm-up during which a program runs
on the interpreter before its code is hot (`primes` lost twelve
milliseconds to the raised factor of AR5), and the cold code the tiering
leaves to the interpreter (nine percent of the self-check), while a
build-time compilation can afford Cranelift's optimising level, which a
run cannot. The alternatives not taken: the machine code as a section of
the `.ryc` JSON (one format, but a large file and a slow load for what
is a binary artifact); object files with relocations through
`cranelift-object` and a loader of ours, or a native executable through
the system linker (a C toolchain on the user's machine, against H1's one
binary); only the hot code objects of a recorded run (a smaller image
for one more step); a silent fall-back to the bytecode on a mismatch (a
run that is quietly slower is the failure mode the language refuses
elsewhere). The address-free form costs an indirect call per helper and
a load per constant, measured by AR1's rule as the first stage; it also
frees the JIT from `cranelift-jit`: the code is compiled by Cranelift's
context alone and placed by the VM. (user)

**AS2. Stage A of AS1, done and measured: the generated code holds no
address. Every helper is called through the VM's table of helpers
(`NativeState::helpers`, in the order of `codegen::SIGNATURES`), the
table of compiled bodies, the count of generated frames and the count of
calls are read through the VM (`NativeState::direct_table`, `depth`,
`calls`), a constant is read through the VM's table of each code
object's constants, and the trampoline reaches its body through the
table of bodies; the code is compiled by Cranelift's context alone,
refused when it carries a relocation (none does: the 555 code objects of
the compiler compile), and placed by the VM in executable memory it
protects itself (`CodeArena`: fresh pages per function, written, flushed
from the instruction cache and made executable once, as `cranelift-jit`
did, through `region` and the icache crate it used), so `cranelift-jit`
and `cranelift-module` are gone from the dependencies.** Measured as AR1
says against the binary of AR5 (`cachegrind`, the release build, the
same hotness factor): the self-check 11.13 to 11.18 billion instructions
on machine code (+0.46%) and 12.90 to 12.96 on the interpreter (+0.45%,
the code layout); `bench/records.ry` 451 to 452 million (+0.15%); per
turn of the micro-benchmarks the typed call 130 to 135, the boxed call
605 to 603, the field 677, the record call 849 to 853, the record built
1041 to 1040. The half of a percent is the indirect call in place of a
direct one, the load of a table's pointer and the two loads a constant
takes, and it is the price of an image that needs no relocation (AS1);
the step stays by the owner's decision, not by AR1's rule, which it
fails by that half percent. (user)

**AS3. Stage B of AS1, done and measured: the image file `.ryi` and
`renyi build`, which compiles every code object at Cranelift's `speed`
level unless `--opt none` is asked.** The file: the magic, the image
format, a header (the `renyi` version, the code format the generated
code assumes of the VM, the target with every CPU feature Cranelift
detected, the optimisation level), the bytecode as the `.ryc` text, and
per code object the body, the trampoline, the loop headers and the deopt
points, or a mark for one the analysis left to the interpreter
(`native/image.rs`; the writer, the reader and a round-trip test).
`run`, `record`, `test` and `reproduce` take it in place of a source:
the program comes from the embedded bytecode, the manifest's code hash
is the bytecode's as for a `.ryc`, and the VM's JIT places the code of
every object in one allocation and marks it ready, so nothing compiles
at run time and no code object is cold. A header that names another
`renyi`, another code format or another target is refused with the
message naming both sides and the fix, to build again; equality of the
feature set is the rule (an image built on an older processor would run
on a newer one and is refused all the same; a portable image is a later
question). Measured as AR1 says on the release build: the compiler's
self-check 11.18 billion instructions on the JIT run, 9.23 as an image
at `none` (-17.5%) and 9.07 at `speed` (-18.9%); `bench/records.ry` 452
million on the JIT run, 423 and 418 as images; the compiler's image is
17.6 MB for 955 code objects and builds in 3.7 and 4.4 seconds. `speed`
is the default by AR1's rule since a build pays the compile time once,
and `none` stays available for a build that must be quick or must match
the JIT's code exactly. What the round's measurements also showed, with
cachegrind's cache simulation on the self-check: the machine code runs
14% fewer instructions than the interpreter and misses the first-level
instruction cache ten times as often (105 million misses against 10),
which is why the two tiers tie in wall-clock (about 1.9 seconds each)
despite the count; the generated code is large (18 KB a code object),
and its size, not its instruction count, is the lever the next
performance round pulls first. (user)

**AS4. Stage C of AS1, done: `renyi build --exe` writes the
self-contained executable of A1, this `renyi` binary copied with the
image appended and a trailer of 24 bytes (the image's offset, its
length, the magic `RENYIEXE`); at startup the binary reads its own last
24 bytes and, when the trailer is there, runs the image with its whole
command line as the program's arguments, before any command of `renyi`
is read. The default name is the program's stem (`.exe` on Windows);
the file is made executable on Unix and signed ad hoc on macOS when
`codesign` is at hand, the command being the message otherwise.** The
reasons: no linker and no C toolchain, by H1's one binary, so that
`build --exe` works wherever `build` does and ships nothing new; the
trailer at the end rather than a section inside the binary, because
bytes appended to an ELF, a Mach-O or a PE leave it an executable the
loader accepts unchanged, while a section means a format-aware rewrite
per platform; the check at startup, one open and one read of 24 bytes
of the binary's own file, is what every command of `renyi` pays for it.
What the executable is not: smaller than the toolchain (the whole binary
comes first, its commands unreachable rather than removed: the
compiler's executable is 35 MB, the binary and its image), compressed
(a decompression at every start, for a file the page cache serves), a
wrapper for a `.ryi` already built (`build --exe` builds from the
sources or the bytecode, as `build` does, so that one command does the
whole job; a stub binary without the toolchain would be a second
artifact to build and ship), or portable beyond the image's own rule
(the same `renyi`, code format and CPU features; a mismatch is refused
at startup with the same message and the same fix). Measured on the
quiet machine, the best of five runs, as the closing wall-clock picture
of the round the image ends: the compiler's self-check 1841 ms on the
JIT run, 1950 on the interpreter, 1691 as an image at `none` and 1511
at `speed` (-18% against the JIT run, the compile time, the warm-up and
the cold code gone); `bench/records.ry` 62 ms on the JIT run, 54 as an
image at either level; `bench/primes.ry` 42 and 35; `hello` 10 and 11
(the load of 235 KB of code for thirteen code objects costs a short
program a millisecond: an image is for a program that runs long enough
to compile). The executable runs as its image does. The cache
simulation also settled the question AS3 left open about the level:
Cranelift's `speed_and_size` generates the same code as `speed` within
nine bytes (17,455,606 against 17,455,597) with the same first-level
instruction-cache misses (158.0 million on the self-check), so the size
of the generated code is a question for the sequences `codegen.rs`
emits, not for Cranelift's level. (user)

## AT. The size of the generated code (session 9)

**AT1. A round on the size of the generated code, measured by
KCachegrind's estimate of the cycles over cachegrind's counts on the
compiler's self-check: `Ir + 10 (I1 misses + D1 misses) + 10
conditional mispredicts + 20 indirect mispredicts + 100 LL misses`,
deterministic, on the JIT run, the interpreter and the image, with the
wall-clock reported at the end of each stage (`tools/measure_size.sh`);
a change is kept when the estimate of the JIT run falls. The order: the
cuts that cost no instruction first (AT2), then the bytecode of the
image in a binary encoding (the `.ryc` stays JSON, decision Z1
unchanged), then the reference-count sequences as shared stubs if the
rule keeps them; the record's layout gains a tag word so that a record
and a variant share the prefix the field read reads.** The owner's four
answers of 2026-10-08, after `renyi build` closed (AS4). The census of
the compiler's image (`tools/image_census.py`) that opened the round:
17.5 MB, of which 9.0 MB is the bytecode as JSON text and 8.3 MB the
machine code of 955 code objects, 46,193 ops, 179 bytes of body per op
and 8.7 KB per code object; a least-squares attribution over the code
objects (r² 0.98) charges `LoadField` 462 bytes each (22% of the
bodies), `Load` 173 (21%), `Call` 440 (21%), `Store` 171 (11%),
`Const` 148 (7%), `Return` 223 and `ReturnNothing` 363 (10% together),
a deopt point 384 (4%). The measurements of AS3 and AS4 said why the
size matters: the machine code runs 14% fewer instructions than the
interpreter on the self-check and misses the first-level instruction
cache ten times as often, and Cranelift's `speed_and_size` level changes
nothing, so the lever is in the sequences `codegen.rs` emits. Why the
estimate rather than AR1's instruction count: a sequence replaced by a
call to a shared stub costs instructions and saves bytes, the trade this
round exists to make, and the count alone would refuse every such step;
why not the wall-clock alone: its noise on this machine (about 3% on a
two-second run) hides the steps, so the estimate decides and the
wall-clock confirms the stage; why KCachegrind's weights and not ours:
they are the published ones and the point is a stable, deterministic
rule, not a model of this processor. Why the image's bytecode in a
binary encoding belongs to the round: loading the compiler's image costs
152 ms against 172 ms for compiling the compiler from source, almost all
of it the parse of 9 MB of JSON, so the image barely starts faster than
the source, which its size hides; the `.ryc` stays JSON because it is
read by people and written by the compiler in Renyi. Why the record's
tag word: the field read's fast path selected between the record's and
the variant's offsets twice and checked the index against the length;
with `{ty, tag, fields}` at the same offsets in both (a record's tag is
`usize::MAX`, as the field cache already has it) one path serves both,
and a hit needs no range check because the index was cached from a
holder of the same type and tag; eight bytes more per record is the
price, accepted. The starting point, the binary of AS4 (71965aa): the self-check on the JIT run 11.18
billion instructions, 116.6 million first-level instruction misses,
75.1 million data misses, 2.17 million last-level misses, 35.4 million
conditional and 85.0 million indirect mispredicts, 15.37 billion
estimated cycles; on the interpreter 12.96 billion instructions, 12.0
million instruction misses and 185.3 million indirect mispredicts (the
dispatch), 18.01 billion; as an image at `speed` 9.07 billion
instructions, 160.2 million instruction misses, 12.63 billion. What
the indirect mispredicts say: the interpreter's dispatch costs it 3.7
billion of its estimate, and the generated code's calls of helpers
through the table 1.7 billion of its own, a tenth. (user)

**AT2. Stage 1 of AT1, done and measured: the cuts that cost no
instruction. The callee's prologue writes `Nothing` to every local that
is not a boxed parameter, once per callee, where the frame push of a
direct call wrote them at every call site; the field read's fast path
reads a record and a variant on one path through the shared prefix
`{ty, tag, fields}` (the record's tag word is `usize::MAX`) and drops the
range check, which a cached index of the same type and tag does not
need; the guards of one op that reach it with the same operand state
share one block that hands the frame to the interpreter (the three paths
of a call's result shared theirs); the direct call no longer asks the
JIT for a body the table lacks (the call goes through the interpreter,
which compiles a hot callee on the way and enters it through its
trampoline, and the next call finds the table filled); and the count of
calls from generated code is gone from the state and the report. The
code format is 2.** Measured by AT1's rule against the binary of AS4:
the self-check on the JIT run 15.37 to 15.13 billion estimated
cycles (-1.6%: 11.18 to 11.02 billion instructions, 116.6 to 106.4
million instruction misses), on the interpreter 18.01 to 17.96 (-0.3%),
as an image 12.63 to 12.42 (-1.7%: 9.07 to 8.88 billion instructions,
160.2 to 155.6 million misses); the compiler's machine code 8.26 to
7.83 MB (-5.2%; 179 to 170 bytes of body per op, 825 to 709 deopt
points), its image 17.46 to 17.02 MB. The cuts that moved the
instruction count were the field read's (the two selects, a load and a
compare per hit), the call counter's (three per call) and the ask
path's; the locals written by the callee moved bytes, not
instructions. (user)

**AT3. Stage 2 of AT1, done and measured: the image carries the
program in a binary encoding (`crates/renyi_vm/src/binary.rs`: an
integer LEB128, a text its length then its bytes, an option one byte
then the value, a list its count then its items, a variant one byte
then its fields, the maps in key order; the same content as the
bytecode file, written in the order `file.rs` writes it, checked on
loading as the file is) together with the hash of the program's
bytecode file, which the run manifest names as the code hash, so that
a recording made from the `.ryc` reproduces against the image and the
other way round. The `.ryc` stays the JSON of Z1. The image format is
2.** Why an encoding of the program and not of the JSON tree: the parse
of the text was only part of the load; the tree's million small
strings (every number is one) and the lookups by key were the rest, and
an encoding read straight into the program skips both. Why the hash is
stored rather than recomputed: rendering the JSON at load would cost
what the encoding saves. Why not the `.ryc` itself in binary: it is
read by people and written by the compiler in Renyi too (Z3), and an
image is written by `renyi build` alone. The census tool takes the
bytecode file beside the image for the ops, since the image no longer
holds them as text. Measured: the compiler's image 17.02 to 8.71 MB (the program
8.99 MB of JSON to 0.68 MB), its load 152 to 69 ms, against 166 ms for
compiling the compiler from source and 165 for reading its `.ryc`
(`renyi run <file> /nonexistent`, which fails before `main`); the
self-check as an image 12.42 to 11.37 billion estimated cycles (-8.4%:
8.88 to 8.13 billion instructions, the parse of the JSON gone), the JIT
run and the interpreter within the noise of the binary's own layout
(+0.1% and -0.3%, the generated code untouched); `hello` as an image 42
KB in place of 235 and 7 ms in place of 8 for the source, the image
faster than the source now on the shortest program too; in wall-clock
the self-check 1448 ms as an image against 1723 on the JIT run, the
best of five each. (user)

**AT4. Stage 3a of AT1, done and measured: the stack's height is static
in the generated code and the frame pointer stays in a register. At
every op the frame's part of the stack is its locals and the boxed
operands of the abstract state (a typed operand is in a register), so
the stack's length is the frame's base plus a constant the code
generator knows; the generated code computes it instead of loading it
from the VM and stores it where a helper or the interpreter reads it
(`height`, `length_at`, `store_height`), and addresses every slot and
operand from one register that holds the stack's pointer plus the
base, read once at the entry and again after every call out of the
generated code, which alone can move the stack (`frame_var`,
`reload_frame`, `address_at`, `top_address`, `last_address`).** The
owner's order of 2026-10-08 for the round's third stage: these two
cuts first, then the shared stubs, each by the rule. Why they cost
nothing: a load, a multiply and two additions per stack access become
an addressing mode of the load or store that follows, and the one load
of the pointer per op becomes one per helper call; the invariant they
rest on, that the real stack and the abstract state agree at every op,
is the one the room check of AR4 already rested on (the deepest operand
stack of a body is read off the states). The one subtlety: after a
result is popped from the state but left on the stack (a boxed return,
`IsNothing`), the value lies at the state's height, not below it, and
the generator says so where it reads it. Measured by AT1's rule against
the binary of AT3: the self-check on the JIT run 15.15
to 14.92 billion estimated cycles (-1.5%: 11.02 to 10.87 billion
instructions, 108.8 to 102.2 million instruction misses), as an image
11.37 to 11.15 (-2.0%: 8.13 to 7.99 billion instructions, 151.2 to
142.6 million misses), the interpreter unchanged (17.90 billion); the
compiler's machine code 7.83 to 7.91 MB (+1.0%: the frame pointer's
reload after every helper call and its block parameters at the merges
outweigh the address arithmetic folded away, while the instructions and
the misses fell, which is what the rule weighs); in wall-clock, back to
back on a busy machine, the self-check 1678 against 1780 ms on the JIT
run (best of five). Kept. A flag per helper saying whether it can grow
the stack would spare the reload after the ones that cannot (`rt_drop_at`
above all, the free of every release) and take some of the bytes back:
a step for a later stage, as is the shared stub of 3b. (user)

**AT5. The image's code section is mapped executable from the file. The
file lays every body and trampoline sixteen-aligned in one section at an
offset that is a multiple of 16 KB (`SECTION_ALIGN`, every page size the
toolchain runs on), the table before it says where each lies, and a run
maps the section with read and execute permission straight from the
file (`memmap2`), so that no machine code is copied and only the pages
the program runs are faulted in; where the system refuses an executable
mapping of the file (a `noexec` mount) the section is read and placed as
compiled code is, and an image in memory (a test's) is placed the same
way. `renyi build --exe` pads the executable to the same alignment
before the image, so that the embedded image's section maps from the
executable's own file. The image format is 3.** The owner's answer of
2026-10-08 to the question the load's 69 ms raised: find the cost and
fix the largest within the round. What the profile found: the system
calls of a load take four milliseconds in all; the time went to the
first touch of fresh memory, twice over, once for the 8.7 MB read of the
file into a new buffer (44 ms under `strace`) and once for the copy of
the 7.8 MB of machine code into the arena's new pages (49 ms by the
JIT's own clock), a cost this machine (a Firecracker VM) makes heavy
and every machine pays in proportion; a file-backed mapping pays none of
it for the bytes it does not touch, and the page cache holds the rest.
Why a section rather than mapping each body: one mapping, one
alignment, one table. Why 16 KB: Apple silicon's page is that large,
and an image built on one machine should map on another of the same
target. Measured: the compiler's image loads in 16 ms where the run exits
at once (`renyi run <image>` with no arguments: the checker prints its
usage), against 27 ms with the section copied and the check as it was,
105 ms for the source and 86 ms for the `.ryc`; with the standard
library loaded by the checker before it fails on a missing file, 46 ms
against 167 for the source; `hello` 6 ms as an image against 8 from the
source; the load's instructions 62 to 32 million, the decode of the
program about 10 million of them and the consistency check 1 million
where it was 32, since it formatted a message for every op of the
program before knowing whether the check failed (the message is built on
failure now, the second cut of this decision); the image 8.8 MB (the
alignment adds up to 16 KB), the compiler's executable 26.6 MB. The
first measurements, under `strace` on a machine whose memory had not
been touched, put the read at 44 ms and the placing at 49; warm, the
placing cost 5 ms and the read a few; the mapping removes both whatever
the machine's state, and the page cache holds the code. (user)

**AT6. Stage 3b of AT1, measured: the reference counts stay in place.
Two variants were built on AT4's binary and measured by the rule: the
frame pointer no longer reloaded after a helper that cannot move the
stack (`STACK_SAFE`: the free of a release, the grant of a frame, the
growth of the frames, the typed takes and resumes, the tag tests), and,
on top of it, `retain` and `release` as calls to the helpers
(`rt_retain_at`, which keeps a clone's count, and `rt_drop_at`, which
drops the value whatever the count) in place of AR4's inline sequences.
The calls stay, and the frame pointer is
not reloaded after a helper that cannot move the stack: `retain` and
`release` are the two calls, the inline sequences of AR4 are gone, and
the compiler's machine code is 5.6 MB where AR4 had made it 7.9.** The owner's order of 2026-10-08 ended with this
stage, each variant by the rule. The numbers, against AT4's binary: the skipped reloads alone
raised the JIT run's estimate 1.0% and the code 0.7% (the pointer, live
across the call, saved and restored around it where the reload had
left it dead); the calls on top of them cut the machine code 7.91 to
5.62 MB (-29%; 122 bytes of body per op where the round began at 179),
the JIT run's estimate 14.92 to 14.83 billion cycles (-0.7%: 10.87 to
10.68 billion instructions, 102.2 to 86.4 million instruction misses,
85.1 to 98.8 million indirect mispredicts, the model's charge for the
calls through the table), the image's 11.15 to 11.18 (+0.3%: misses
142.6 to 96.3 million, indirect mispredicts 38.9 to 67.9 million); the
calls without the skipped reloads measured 14.85 and 11.21 billion, a
fifth of a percent behind, so the list stays by the rule, each of its
fifteen names a helper that grows nothing and runs no program code,
and a helper not listed is reloaded after, which is always sound. Why
the call beats the sequence: `Value::clone` and the drop are a match on
the tag with one increment or decrement, where the generic sequence
tested the tag's bit, the big-integer case and both payload words at
every site, and the sites were thousands. In wall-clock against the
binary of AT3, the best of five: the self-check 1886 to 1775 ms on the
JIT run (-5.9%), 1369 to 1351 as an image, the interpreter and
`bench/records.ry` and `bench/primes.ry` level. The stubs of AT1's plan
turned out to be the helpers the VM already had. (user)

**AT7. The round closed by the owner on 2026-10-09, every stage of
AT1's plan measured and in (AT2 to AT6).** Against the binary the round
began from (71965aa, the one AS4 left): the compiler's machine code
8.26 to 5.62 MB (-32%, 179 to 122 bytes of body per op), its image
17.5 to 6.5 MB, the self-check's estimated cycles 15.37 to 14.83
billion on the JIT run (-3.5%) and 12.63 to 11.18 as an image (-11.5%),
the image's load 152 to 16 ms where the run exits at once (most of the
152 was the checker's own work on the standard library, which the
profile of AT5 told apart), the self-check in wall-clock 1841 to 1775
ms on the JIT run and 1511 to 1351 as an image, `hello` 10 to 6 ms as
an image. What the round found beyond its plan: the two copies of a
load were the largest item on this machine because fresh memory's
first touch is dear (AT5); a static invariant the room check of AR4
already rested on removed a load from every stack access (AT4); and
the inline reference counts of AR4, the round's largest source of
bytes, lost to the VM's own helpers (AT6), the finding the round
opened on: a sequence inlined for speed can cost more in the cache than
it saves in instructions, and only the measurement tells. What is left
for a later round: `RELEASES_INLINE` (four calls against one
`rt_truncate` on a return), the indirect call through the helper table
(a tenth of the estimate in the model's charge; a direct call would
need a relocation per site, against AS1), and `Int` flattened into
`Value` for a one-compare clone and drop. The round the owner chose
next is the one benchmark behind CPython, the per-character loop of
`bench/strings.ry` (128 against 85 ms; `primes` 9.7, `records` 3.4 and
`json_round_trip` 1.7 times faster than CPython, `hello` 6 ms against
Python's 18): by its profile on the interpreter, a boxed constant
copied and retained per iteration, a one-character text allocated per
glyph, and a primitive call through the boundary per glyph; its measure
and its cuts are its own opening decision, after the profile on the JIT
tier. (user)

## AU. Static types to the code generator (session 9)

**AU1. The round that brings the checker's static types to the code
generator, the cure the owner asked for in place of another cut, in
this order, each stage measured and kept by the rule below: (i) the
call to a primitive without the boundary: a native may carry a typed
entry from a fixed family of signatures (`fn(&[Value]) -> Option<T>`
for `T` one of `bool`, `i64`, `f64` and `Value`; `None` means the
arguments are not the plain values the fast path takes and the general
path runs), called by the generated code with its arguments borrowed
where they lie on the stack and its result in a register; the
registration API's `Native` gains the optional entry (an addition to
AK2), the standard library's hottest primitives take one; a pure
primitive without one (no `needs`) runs on a flat path that enters none
of the boundary's checks, since none applies to it; an operand a typed
call borrows is neither retained before nor released after, when the
op that pushed it was a `Load` or a `Const`; (ii) the loop over a list
in the generated code: the iterator in a fixed layout that the code
reads and advances itself, as it does for a range (AG3); (iii) the
typed bytecode, format 5: both checkers record the type of every
expression of a body (`Target::Typed`, by span, as `Target::Result`
already travels), both emitters write a table of the types a program
uses and, per code object, the type of what each op pushes, and the VM
checks the annotations against its own inference of the three scalars
on the corpus before it trusts them; (iv) what the types buy: the field
read at an index known statically, the ability method chosen
statically, the loop variable typed, the retains and releases elided
wherever the consumer borrows; (v) left to later rounds: the
representation (small texts inline, `Int` flattened into `Value`, lists
of unboxed Integers, values in registers across ops), which the types
make worth doing. The measure: AT1's rule decides (the compiler's
self-check by KCachegrind's estimate of the cycles on the JIT run, the
interpreter and the image, `tools/measure_size.sh`), the four
benchmarks against CPython in wall-clock confirm (`tools/bench.py`); a
stage stays when the self-check's estimate on the JIT run falls, or
when a benchmark gains more than 5% while the self-check loses no more
than 1%. The round's first goal is `bench/strings.ry` past CPython, its
second the self-check a quarter faster than AT7 left it.** The owner's
four answers of 2026-10-09, asked after the profile of
`bench/strings.ry` on the JIT tier; the owner chose the typed bytecode
over the session's recommendation of an inference inside the VM
(below), and folded the strings round AT7 had announced into this one.
The root cause, as the profile shows it: Renyi is statically typed,
but its bytecode and its VM are not; the checker's types die at the
emitter, every value carries a tag, every op reads it, the baseline
JIT recovers the three scalars by abstract interpretation (AR1) and
boxes everything else on the VM's stack with a helper call per op, so
the generated code is the interpreter unrolled: on the self-check it
runs 14% fewer instructions than the interpreter (AT1) and 7% faster
in wall-clock, and `bench/primes.ry` is 9.7 times faster than CPython
because it is all Integers. The strings loop (`"0123456789".contains(glyph)`
over 1,188,895 glyphs; 130 ms against CPython's 84 on this machine,
the first half of the program 23 ms of it) runs 966 instructions per
glyph on the JIT tier (callgrind, 1,149 million in all): 411 of them
(43%) cross the primitive boundary (`rt_call` 23, `call_from_stack`
101, `call_primitive` 87, `run_primitive` 48, `status` 40, `guarded`
24, `effect_of` 22, the drop of the two arguments 41, the copy into
the scratch buffer 25), 117 (12%) are the search itself
(`is_contained_in` 72, `text_contains` 45), 80 (8%) the iteration
(`rt_iter_next` 29, `iterator_next` 37, the item's clone), 55 (6%) the
retain and the clone of the constant and the glyph, 67 (7%) the
generated code, and the rest the first half of the program (the text
built from a hundred thousand pieces: `rt_to_text` 59 million through
`core::fmt`, `rt_concat` 48 million, `append` through the same
boundary). The glyphs themselves allocate nothing since AQ (the ASCII
table of `Value::character`). Why the typed bytecode rather than an
inference in the VM from the signatures the bytecode already carries
(`param_types`, `returns`, the field types of every shape): the owner's
choice; the inference would have been a second checker inside the VM,
sure only where the first was, and a call of a generic function or an
empty list literal has its type at the call site alone; with the
checker recording what it knows, the VM infers nothing and the two
checkers are held equal on one more thing by the judge of W7, as they
already are on `Target::Result` and `Target::Number`. Why the typed
natives through a signature family and not per primitive: the
generated code is address-free (AS1) and calls through tables; a
family of four result kinds over borrowed arguments is one table and
four helpers, and the natives stay ordinary Rust functions that an
extension can write too. Why the typed call takes `Option`: the fast
path sees plain values; a guarded argument, a big Integer or a failure
returns `None` and the general path does what it always did, so a
typed entry can never be less correct than the native it stands beside.
Why stage (i) comes before the typed bytecode: it needs no type the
bytecode lacks (the callee is known at every call site and its result
type is in `returns`), it removes the largest item of the profile, and
the typed bytecode's calls will target the same entries. What the types
do not change: the generated code still reads the tag before it takes
a fast path, since a value of Integer type may be big or guarded at run
time (AR1), so a type is the choice of the fast path and never a
promise the code relies on for its safety; the recordings, the replay,
the narration and the `.ryc` as JSON (Z1) stay as they are. (user)

**AU2. Stage (i) of AU1 is in: the call to a primitive without the
boundary. A `Native` may carry a typed entry (`Native::with_typed`,
`Typed::Bool`, `Int`, `Float` or `Value` over `fn(&[Value]) ->
Option<T>`), which `Registry::verify` holds to the declaration (the kind
of the declared result; no `needs`, since the entry skips the boundary),
and forty-two primitives of the prelude carry one (the lengths, the
emptiness tests, `contains`, `starts_with`, `ends_with`, `index_of`,
`get`, `contains_key`, `at`, `first`, `last`, `join`, `sum` of Integers,
the trims and cases of a text, `split`, `lines`, `characters`,
`replace`, `take`, `drop`, `reversed`, the absolute values, `at_least`
and `at_most` of Integers, `to_float`, `square_root`, `rounded`). The
generated code calls such a primitive through `rt_call_typed` with the
arguments the `Load`s of boxed slots and the boxed `Const`s before the
call pushed borrowed (no retain before, no release after; `borrowed_operands`,
the mask of the call's arguments) and takes a scalar answer from the out
slot into its register; when the entry declines, the general path runs
with the borrowed arguments given their reference, and its boxed answer
comes back as `BOXED`, unboxed as a call through `rt_call` is, with the
hand-back to the interpreter at the op after the call. A pure primitive
without an entry runs through `rt_call_pure` on the flat path
(`Vm::call_pure_from_stack`: the memory check, the arguments into the
scratch buffer, `plain_in_place`, the native, `guarded`); the
interpreter's `call!` takes the typed entry in place
(`call_typed_in_place`) and the flat path too. `CallKind` per function
(`General`, `Pure`, `Typed(kind)`, from the registry when the VM or
`renyi build` starts) is what the code generator compiles by; the
helper looks the entry up again at run time, so an image compiled with
an entry runs under a binary without one, through the general path.**
Measured by AU1's rule against the binary of AT7 (7e09ca6): the
self-check's estimate on the JIT run 14.83 to 13.86 billion cycles
(-6.5%; 10.68 to 9.87 billion instructions, -7.5%), the interpreter
17.92 to 17.23 (-3.9%; 12.85 to 12.21 billion instructions), the
image 11.18 to 10.27 (-8.2%; 7.97 to 7.16 billion instructions,
-10.2%; the machine code 5.62 to 5.63 MB, 122 bytes of body per op as
before); in wall-clock `bench/strings.ry` 134 to 94 ms (CPython 81 on
the same runs; the interpreter 175 to 157),
`primes` 41, `records` 59, `json_round_trip` 96 ms by `tools/bench.py`
(CPython 528, 166 and 156), the self-check 1694 ms on the JIT run and
1256 as an image. The profile of strings after the stage (callgrind,
733 million instructions from 1,149, 617 per glyph from 966):
`rt_call_typed` 111 instructions a call where the boundary took 411,
the entry's own match on the two texts 20, the search 72 as before;
what is left of the loop is the iteration (`rt_iter_next` and
`iterator_next`, 82 a glyph, stage ii), the generated code's own 67,
the loop variable's retain and release (54), and the first half of the
program, now a fifth of the whole: `rt_to_text` through `core::fmt`
(586 instructions per Integer written into a text), `rt_concat`, and
`append`, which frees and allocates its `Rc` on every call
(`take_list` then `Value::list`), 400 instructions each. Why `Option`
and a fixed family rather than a signature per primitive: AU1. Why the
borrowed operands are only the `Load`s and `Const`s just before the
call: they push exactly one value each and pop none, so they are the
call's last arguments in order without an analysis of the stack
effects of every op; `LoadMove` is not among them (its operand is the
slot's reference, moved). Why the entry is looked up again at run time
rather than its address placed in the code: AS1, the code holds no
address, and a binary with other extensions may lack the entry. (user)

**AU3. The small cuts the profile of AU2 named, in: `rt_call_typed`
syncs the pc only where something can go wrong (the general path, an
interrupt) and drops nothing when every argument was borrowed
(`consume_arguments`); a machine-word Integer writes its digits into a
text without `core::fmt` (`integer::push_digits`, `digits`; in `render`
and `op_concat`, so in every `{...}` hole and every `to_text`); `append`,
`append_all`, `set` and `add` on a list, a map or a set, and `without`,
change the collection through `Rc::make_mut` and keep its allocation
where `take_list` and `Value::list` had freed one and allocated another
on every call.** Measured by AU1's rule against AU2's binary: the
self-check's estimate on the JIT run 13.86 to 13.83 billion cycles
(-0.15%), the interpreter 17.23 to 17.20 (-0.2%), the image 10.27 to
10.22 (-0.4%): the cuts are the strings loop's, the self-check only
holds; `bench/strings.ry` 94 to 84 ms best of seven, 87 by
`tools/bench.py` against CPython's 79 on the same runs (0.9 of
CPython's time), 77 as an image, which is past it; the first half of the
program 39 to 30 ms. The profile of strings (callgrind): 733 to 669
million instructions (563 a glyph); `rt_call_typed` 111 to 80 a call,
an Integer written into a text 586 to 108 instructions, `append` 400 to
50; the loop is now the iteration (`rt_iter_next` and `iterator_next`,
82 a glyph; stage ii), the search (72, Rust's `memchr` on ten bytes for
a one-byte needle), the typed call's helper (80), the generated code
(56), the entry's own match on its two texts (20) and the loop
variable's retain and release (about 40). (user)

**AU4. Stage (ii) of AU1 is in: the loop over a list in the generated
code. The list iterator is `ListIter` (the items' address and count, the
position, the list that keeps the items where they are), in a fixed
layout inside a `Native` that is `repr(C, u8)` now (`layout::TAG_NATIVE`,
`NATIVE_TAG`, `NATIVE_PAYLOAD`, `NATIVE_ITERATOR`, `ITER_ITEMS`,
`ITER_LEN`, `ITER_POSITION`, held to the types by a test); at
`IterNext` on a slot the analysis did not type as a range, the generated
code reads the slot's tag and the native's, and for a list iterator
compares the position with the count, copies the item onto the stack
with one more reference and advances the position in place, leaving
the loop at the exit otherwise; anything else in the slot (a range
iterator made from a Range value, a value no analysis typed) goes
through `rt_iter_next` as before. The interpreter's `iterator_next`
reads the same struct (`ListIter::next`).** Measured by AU1's rule
against AU3's binary: the self-check's estimate on the JIT run 13.83 to
13.66 billion cycles (-1.3%; 9.78 to 9.74 billion instructions, the
instruction misses 77 to 66 million), the interpreter 17.20 to 17.27
(+0.4%, within the rule: the JIT run decides; `ListIter::next` branches
where the old iterator did not), the image 10.22 to 10.08 (-1.4%; the machine code 5.63 to 5.70 MB, 123 bytes of body per op, the inline walk's price); `bench/strings.ry` 84 to 83 ms best of
seven and 87 to 78 by `tools/bench.py`, CPython 79 on the same runs: at
par, the round's first goal reached in wall-clock on this machine but
not yet with a margin; 76 ms as an image. The profile of strings
(callgrind): 669 to 625 million instructions, 525 a glyph; the
iteration's 82 (`rt_iter_next`, `iterator_next`, the clone) became 20
in the generated code and 21 in `rt_retain_at` for the item's reference.
What a list iterator's items pointer rests on: a list another holder
changes is copied first (`Rc::make_mut`, decision O1) and a list the
iterator alone holds is changed by nobody, so the items never move
while the loop runs; the interpreter and the generated code agree on
the position's meaning (the index of the next item, never past the
count), so a frame may change hands in the middle of a loop as it does
for a range (AG3). Why the tag checks stay: the slot's kind is `Iter`
for every loop whose source the analysis did not see as a range of
small Integers, and such a source may be a Range value at run time. (user)

**AU5. Three micro cuts after AU4, in: the typed call's general path and
its value answer live in their own functions (`typed_call_general`,
`typed_value_answer`, `typed_over_memory`), so that `rt_call_typed`
itself is the fast path alone; `contains` on a text looks for a one-byte
part in a text of at most 32 bytes by a plain loop (`text_has`), where
Rust's search set up more than it searched; `push_digits` pushes the
digits as characters instead of checking them as UTF-8.** Measured by
AU1's rule against AU4's binary: the self-check's estimate on the JIT run
13.66 to 13.75 billion cycles (+0.6%, the instructions +0.1% and the
instruction misses 66 to 73 million: the code's layout moved, the work
did not; within the rule's one percent), the interpreter 17.27 to 17.24
(-0.2%), the image 10.08 to 10.12 (+0.4%); `bench/strings.ry` 83 to 78
ms best of seven and 78 to 77 by `tools/bench.py`, CPython 79 to 82 on
the same runs; the profile of strings 625 to 589 million instructions
(-5.7%; 496 a glyph): `rt_call_typed` 80 to 72 a call, the entry with
its search 92 to 70, the digits of an Integer unchanged (the characters
cost what the UTF-8 check cost). The stage stays by the rule's second
clause: a benchmark gained more than 5% and the self-check lost less
than 1%. What the loop of strings is now, by the glyph: the generated
code's own 77 instructions (the walk of the list, the push of the
glyph, the store into its slot, the branch), the typed call's helper
72, the entry's match and search 70, the release of the previous glyph
and the retain of the next about 40, the rest the first half of the
program and the list of glyphs built and dropped. (user)

**AU6. Stage (iii) of AU1 is in: the typed bytecode, format 5. Both
checkers record the type of every expression of a body at its span, as
`Target::Typed` (`check.rs`: `infer` wraps `infer_inner` and notes the
span and the type; `finish_body` zonks each and records the ones without
a variable left, as it does `Target::Result`; `bodies.ry`: the same,
`record_typed` appending the body's noted types to the references at
once). Both emitters write, per op, the index of that type in the
program's table of types (`result_types`, which holds the context types
of `ResultType` ops and every noted type now, first use first):
`Compiler::emit` and `emit.ry`'s `emit` look the op's span up
(`type_index_at`, `type_index`); `emit_untyped` writes none, for the one
op that pushes something other than its expression's value under that
expression's span (the comparison of a literal pattern). `Code::types`
travels in the JSON (`"types"` per code object, `null` where none), in
the binary encoding (encoding 2) and so in the image (format 4); the
file's consistency check holds the list's length to the ops' and every
index inside the table. `Op::pushes_its_expression` names the ops whose
annotation is the type of what they push (`Const`, `Load`, `Call`,
`Field`, `Binary`, ...); the others (`ToText`, `IterNext`, `Unpack`, the
jumps) carry their expression's type too but push something else or
nothing, so the VM may read the annotation for the former alone.
`abs_of_type` sees through a refined subtype to its base (its values are
the base's). The test `tests/typed.rs` holds the annotations to the
inference of the baseline JIT over the corpus, the clean conformance
programs and the compiler: wherever the analysis types what an op pushes
as one of the three scalars, the noted type must be that scalar's, or a
`maybe` of it; the two emitters are held equal on the new field by the
judge of Z3 and the two checkers on the new target by W7.** Measured:
the compiler's checker as a bytecode file 8.99 to 9.67 MB (+7.6%;
46,272 ops of which 30,309 (65%) carry a type, 425 types in the table);
the VM reads nothing of it yet (stage iv), and still the self-check's
estimate rose, 13.75 to 16.67 billion cycles on the JIT run (+21%),
17.24 to 20.20 on the interpreter, 10.12 to 12.75 as an image, because
the program measured changed, not the VM: the self-check is the checker
written in Renyi checking `bodies.ry`, and that checker now notes the
type of every expression, through lists held in a record, which Renyi
copies on every append (a field read clones the list, so `append` finds
it shared), and joins into the module's references body by body; the
Rust front end's own share is small (`renyi check compiler/checker.ry`
538 to 561 million instructions, +4%). The judge of Z3 and W7 runs in
117 s. The next commit bounds the copies. Why every op and not the pushing ops
alone: the emitters would have had to agree on a stack-effect predicate
per op kind in two languages, where one lookup per emitted op agrees by
construction; the file pays 7.6%. Why the table is `result_types` and
not a second one: one index space for the two uses, one dedup in each
emitter, and the `.ryc` keeps its key. Why the one `emit_untyped`: the
literal pattern's comparison is the only op the emitters write under a
span whose expression has another type than what the op pushes and
whose kind `pushes_its_expression` lists; the query terminals' `Add`,
`Nothing` and `Load` under the query's span push what the query's type
says. What the test found on the way: the literal of a refined field
(`Version(major: 1, ...)`) is typed with the refined subtype, which the
inference now reads as the base; a `maybe Integer` binding the analysis
proves always an Integer is compatible, not a disagreement. (user)

**AU7. The typed notes recorded cheaply, after AU6's measurement: the
checker written in Renyi keeps a body's notes in chunks of 64
(`typed`, `typed_chunks`: a list held in a record is copied on every
append, since the field read clones it and `append` finds it shared,
so the copy is bounded), keeps the typed references of each body apart
(`typed_references`, one list per body) and joins them into the module's
references once (`all_references`); the Rust emitter finds a type's
index in the table through a map (`type_indices`) instead of a scan.**
Measured by AU1's rule against AU6's binary: the self-check's estimate
on the JIT run 16.67 to 14.58 billion cycles (-12.5%; 6.0% above AU5's
13.75, before the notes), the interpreter 20.20 to 18.04 (AU5 17.24),
the image 12.75 to 10.66 (AU5 10.12); the judges of Z3 and W7 in 102 s
(117 s at AU6). What is left of the notes' cost is the Renyi checker's
own: a `SpanTy` record and two record copies of the checker per
expression, the zonk of each note at the body's end, a reference record
per note. What this showed about the language: the pattern `record with
field: record.field.append(x)` copies the list on every call, in the
compiler everywhere (`references`, `diagnostics`, `vars`, `scopes`); an
emitter that recognised the update of a field of a uniquely held record
and moved the field out and back in would make the compiler itself
faster by a large factor on its hottest paths, a candidate for a later
round. (user)

**AU8. Stage 4 of the typed round, steps A and B: the types in the
abstract state and the field read at a static index. The analysis
carries the noted type with every boxed value (`Abs::Boxed(Option<u32>)`,
the index into `result_types`; `SlotKind::Boxed(Option<u32>)` by the
stores; `Analysis::slot_types` from the loads' annotations, else the
stores', else the parameter's declared type); the bytecode file carries
every parameter's type as an index into the table (`param_type_indices`
of a function, looked up in the table once it is complete and never added
to it, so that the table stays what the expressions made it; format 6,
binary encoding 3, image format 5), and `abs_of_params` decides by it, so
a parameter of a refined subtype of Integer is passed in a register as an
Integer is. A `LoadField` whose slot has a record type, and a `Field` whose
holder on the stack has one, check the holder's tag and type against the
expected type and read the field at the index the type's shape gives, with
no cache (`static_field`); the kind of what a field read pushes comes from
the op's own annotation (`abs_of_field_at`), the name-based join only
without one. The generated code keys its landings, its hand-backs and its
state check against the analysis by the kinds alone: the analysis joins
the states of every round of its fixed point and loses a type where a kind
changed on the way.** Measured by AU1's rule against AU7's binary: the
self-check's estimate on the JIT run 14.58 to 14.55 billion cycles
(-0.2%), the interpreter 18.04 to 18.04, the image 10.66 to 10.54 (-1.2%);
the machine code 5.83 MB, 125.8 bytes of body per op, 771 deopt points.
Why so little, from a profile of the JIT run (callgrind, 10.28 billion
instructions): the generated code runs 9.5% of the instructions; the
interpreter loop 19.6% and its frames 3.3% (the self-check compiles 30
code objects, the rest stays below the hotness rule of AR5); reference
counts 17.1%; the VM's glue around the helpers 17.4% (`push_frame_in_place`,
`op_load_field`, `plain_all`, `refined_record`, `call_pure_from_stack`,
`op_with`, `guarded`, `make_mut`, `frame_grant`); allocation and `memcpy`
8.6%; the helpers 7.4%; comparisons 6.6%; Cranelift 4.8%; the Rust front
end's check of the compiler 4.5%; the natives 3.3%. Of the 94 `LoadField`
sites in the compiled code objects 46 take the static path and 12 of 12
`Field` sites; 35 of the rest read a slot no annotation types (a binding
from a `match` or an `otherwise`) and 9 a field every variant of a sum type
has (`Ty`), which a later step could serve too. The step stays by the
rule's first clause. What the profile says about the plan: step iii of
AU1 (the ability method chosen statically) has nothing to buy, the
compiler emits no `CallAbility` on its hot paths (a concrete receiver's
method is a direct `Call` already); the loop variable typed (step iv's
third item) types what the loads' annotations type already; the cost is
in the interpreter's share, the reference counts and the copies of
records and lists that `record with field: record.field.append(x)` makes,
not in what the generated code computes. (user)

**AU9. The round's order after AU8's profile, the owner's answers of
2026-10-09: first the image cache of `renyi run` (the steady state of an
image from a program's second run on: the self-check's image row is 28%
below its JIT run), then, in this round, the update of a uniquely held
record in place (`with` and `append` without a copy), the comparisons on
borrowed operands, and the parameters borrowed across direct calls. Two
items of AU1's step iv are dropped: the ability method chosen statically
(the compiler emits no `CallAbility` on its hot paths; a concrete
receiver's method is a direct `Call` already) and the loop variable typed
from the list's item type (the loads' annotations type it already). The
rule stays AT1's: the cold JIT run, without the cache, is the row that
decides (the measuring scripts run with the cache off); the run from the
cache is reported beside it.** Measured on AU8's binary for the record,
the hotness factor of AR5 against the self-check's estimate: 2000 14.56,
3000 14.56, 4000 14.48, 6000 14.48, 8000 14.55, 16000 15.01 billion; the
factor stays 8000, the differences below it being within half a percent
and the cache changing what the factor is for. (user)

**AU10. Stage 5 of the typed round: the image cache of `renyi run`. A
run of `run`, `record` or `test` (and `serve` without `--watch`, which
is `run`) that compiled machine code leaves the program's image (AS1)
in a cache directory of the user's, built after the run by the same
binary, `renyi build --cache <file>`, spawned in the background with no
standard streams; every later run of the same program by the same
`renyi` on the same machine loads its code from the cache and compiles
nothing, and `reproduce` reads it too. An entry is named by a hash of
the `renyi` version, the host's target and the program's binary
encoding (AT3); a hit is an image whose header fits the machine and
whose program is, byte for byte, the program the run compiled, so that
a changed, corrupted or foreign file is a miss and the cache never
changes what runs. The directory is `renyi/images` under the system's
cache directory or `RENYI_CACHE_DIR`; the images together are kept
under 256 MB, the least recently used going first (a hit refreshes the
file's modification time); a lock file keeps a second build of the same
program from starting while one runs (ten minutes, then it is taken for
dead). The cache is off under `--no-cache` (`run`, `record`, `test`),
under `RENYI_NO_CACHE`, for a run on the interpreter (`--interpret`,
`--explain`, `--profile`), for a program loaded from an image, and for
everything cargo runs (`.cargo/config.toml`), so that a test runs the
same way every time; CI sets it too.** Why in the background: the build
of the compiler's image takes about three seconds at `speed`, more than
the run it follows; a script an agent runs once pays nothing, and the
second run finds the image. Measured on the self-check (AU9: the cold
JIT run decides and stays as it was, the cache being off there):
the run from the cache 11.23 billion cycles against the cold JIT run's
14.23 (-21%; the image alone 10.31, the difference being the front end's
check of the compiler's sources and the encoding compared, 0.75 billion
instructions); the cold JIT row moved from AU8's 14.55 to 14.23 with
its instructions equal within 0.1% (10.27 against 10.28 billion), the
change being in the simulated data-cache misses, which follow the
binary's layout and not this stage. Wall-clock on this machine, with a
cachegrind run on another core: the self-check 2.2 to 2.5 s cold, 1.6 to
1.8 s from the cache; `hello` 8 ms either way; the first run with the
cache 2.1 s, the image of 6.8 MB in the cache two seconds later. (user)

**AU11. Stage 6 of the typed round: the update of a uniquely held record
in place (bytecode format 7, binary encoding 4, image format 6, code
format 4). Two ops: `WithSlot { slot, fields }`, whose base is the
record in the slot, moved out (`Nothing` is left) and updated in place
when it is held once (`Rc::make_mut`; the allocation stays either way,
where `with` freed the record's and allocated another), the refinements
of the type checked on the result as `with` checks them; and `TakeField
{ slot, name, site }`, which pushes a field of the record in the slot,
moved out when the record is held once (`Nothing` is left in the field
until the `WithSlot` that follows puts the field's new value in) and
cloned when the record is shared, through the site's cache as
`LoadField` reads. Three rules, in both emitters, which the judge of Z3
holds equal: (a) `change x to x with U` and `return x with U`, `x` a
local: when every update is `f: x.f.method(args)` with the names
distinct and no argument pinning `x`, each receiver `x.f` is a
`TakeField` and the record is updated with `WithSlot` (the takes); else,
when no update may leave the loop (`break`, `continue`), a `WithSlot`
with the fields read as they are (the move); else the copy as before.
(b) `change x to CALL`: `x` is moved into the call with `LoadMove` when
it is the receiver (O1) or, new, when it is the one argument that is `x`
itself, and nothing else in the call pins `x`. (c) `pins(expr, name)`,
the predicate behind both: the expression reads the name anywhere, or
holds a `break` or `continue` outcome; the second is new for O1's rule
too, which moved the receiver out and let an argument leave the loop
with the slot `Nothing`.** What makes the takes safe: a hole exists only
in a record held once, which only the slot sees; the slot is dead
(`return`) or stored into (`change`) right after the statement; a
handled region is opened inside an expression only, never around a
statement, so a failure during the updates leaves the frame; a `break`
or `continue` inside an update is excluded by the rule; the taken field
is read nowhere else, since no argument mentions `x` and the names are
distinct; a record that is shared, or a holder that is not a record, is
read as before. The `let y be x with ...` form (the plan's rule c) needs
the liveness of `x` after the statement and is left out of this stage:
the two hottest `with` sites of the Renyi checker (`fresh`, `infer`)
have that form, so the next profile decides whether it is worth a
liveness rule in both emitters. In the compiler's checker program: 71 of
159 `with` sites are updates in the slot, 17 of them with their field
taken, and 429 values are moved into calls where 180 were (`LoadMove`);
the bytecode file 9,751,963 to 9,743,893 bytes. Counted on the
self-check with the VM instrumented for the purpose: of the 206,427
updates run, 70,922 are `WithSlot` and 23,848 find the record held once;
of the 8,258 takes, 2,305 move the field; the rest find the record
shared, by a `Done` record that carries the checker beside its value
(`let after be done.checker`, then `done with checker: updated`) or by a
binding still live after the statement. Measured by AU1's rule against
AU10's binary (586f4b1), both binaries on the same machine: the
self-check's estimate on the cold JIT run 14.23 to 14.16 billion cycles
(-0.5%; 10.27 to 10.26 billion instructions), the interpreter 17.76 to
17.80 (+0.2%; 12.73 to 12.72 billion instructions, the instruction
misses up with the binary's layout, within the rule), the image 10.31 to
10.23 (-0.7%; 7.19 to 7.18 billion instructions; the machine code 5.79
MB, 125.2 bytes of body per op, 771 deopt points), the run from the
cache 11.23 to 11.16 (-0.7%); in wall-clock by `tools/bench.py` on this
machine `bench/records.ry` 53 to 52 ms and the self-check 1,763 to 1,774
ms on the JIT run, 1,364 to 1,313 ms as an image, best of seven with the
two binaries alternating, which is noise: no change in wall-clock. The
stage stays by the rule's first clause, and the gain is small for the
reason the counts give: the hot updates of the checker find their record
shared, so the in-place path they were built for rarely runs; what would
make it run is the move of the last read of a binding (a liveness rule)
and the field taken out of a record that is updated with the result
later, which the plan of AU1's stage iv did not foresee and which the
owner is asked to order. (user)

**AU12. The owner's answers of 2026-10-09 after AU11's finding: (i)
stage 7 now, the comparisons on borrowed operands, and stage 8 after it;
the copies that the checker's shared records keep are left to the
representation round (AU1's step v), where a liveness pass over the
bytecode (the last read of a slot as a move) would serve every form at
once, rather than more syntactic rules in two emitters now; (ii) the
round's second goal, the self-check a quarter faster than AT7 left it
(14.83 billion estimated cycles), is judged on the cold JIT run, the row
AU9 made the one that decides, and not on the run from the image cache
(11.16 against 14.83 already, which removes the compiling and not the
running); after stages 7 and 8 a new profile decides the stages that
follow, the representation items of AU1's step v among them, until the
goal is met or the owner closes the round; (iii) whether a command of
the binary runs the front end written in Renyi in place of the Rust one
is decided after this round, with the self-check's speed then, and
nothing is added now (the cost measured in session 10: checking
`compiler/bodies.ry` takes 0.083 s with the Rust front end and 1.87 s
with the Renyi checker on the cold JIT tier).** Asked in a batch of
three after AU11's measurement; the owner took the recommended option of
each. (user)

**AU13. Stage 7 of the typed round: the comparisons on borrowed
operands, in the generated code alone. A `Binary` comparison (`is`, `is
not`, `is less than`, `is at most`, `is greater than`, `is at least`) on
two boxed operands goes through `rt_compare`, which compares them where
they lie on the stack and answers the Boolean into the out slot, from
where the generated code takes it into a register: two small Integers,
two texts, two Booleans, `Nothing` beside a value, and two variants
without fields of a type without its own `equals` are answered there;
anything else (a record, a variant with fields, a Decimal, a Float, a
declared `equals` or `compare`) takes the general path, `binary_values`,
whose boxed answer comes back as `BOXED` and is taken into the register
as a typed call's is. The operands a `Load` of a boxed slot, a boxed
`Const` or `Nothing` pushed just before the comparison are borrowed as
the typed calls' arguments are (AU2): pushed without a reference of
their own and consumed without a release; the general path gives them
their reference first. A Boolean carries no origins, so a guarded
operand is looked through. No op, format or emitter changes: the
interpreter compares as before; the image's code format is 5 (the helper
among the helpers).** Why the comparison and not every binary operator:
the others on boxed operands are arithmetic on Decimals and big
Integers, rare in the self-check, and their answers are boxed. Why
`Nothing` joins the pushes that borrow: `x is not nothing` is a fifth of
the checker's comparisons, and `Nothing` holds no reference. Why a
`LoadField` operand is not borrowed yet: its helper path wraps a guarded
holder's field anew, so the code cannot tell a borrowed push from an
owned one without the helper saying so; the 33 `LoadField ... is` sites
of the checker program wait for that. Why `equal` asks the left
operand's type alone: the VM's `equal` does, so `Nothing` on the left is
answered at once and a record on the left with a declared `equals` is
not. Of the 722 comparisons in the checker program, 233 are `Load Const
is` on a text, the shape the self-check runs most (`Const Binary` 4.7
million of its 6.8 million `Binary` ops). Measured by AU1's rule against
AU11's binary (aaa44f6), both on this machine after a restart of the
container (cachegrind's simulated cache misses change between boots, the
same binary giving 1.15 and 2.60 million last-level misses on two boots
with its instructions equal within 0.01%, so the two binaries of a
comparison are measured on one boot): the self-check's estimate on the
cold JIT run 14.44 to 14.12 billion cycles (-2.2%; 10.26 to 9.95 billion
instructions, -3.1%), the interpreter 18.07 to 18.09 (+0.15%; the
instructions equal within 0.01%, 12.72 billion, once the three fast
paths `small_binary`, `text_binary` and `boolean_binary` were inlined by
force: with a second caller the compiler had stopped inlining
`text_binary` into `binary_values`, which had cost the interpreter one
percent of its instructions), the image 10.40 to 10.05 (-3.4%; 7.18 to
6.80 billion instructions, -5.2%; the machine code 5.79 MB, 125.2 bytes
of body per op, 771 deopt points), the run from the cache 11.38 to 11.03
(-3.1%; 7.94 to 7.56 billion instructions). The profile of the JIT run
(callgrind): `rt_binary` (188 million instructions) and its answer
through `status` are gone, `rt_compare` runs 307 million with the fast
comparisons inlined, `binary_values` 236 to 92 million, the retains of
the operands 94 to 68 million and the drops of values 664 to 595
million. Wall-clock on this machine, noisy today (eight percent between
runs of one binary): the self-check 2.57 to 2.42 s on the JIT run and
2.00 to 1.77 s as an image, best of seven with the two binaries
alternating, `bench/json_round_trip.ry` 133 to 121 ms, the other
benchmarks within the noise. The stage stays by the rule's first clause.
(user)

**AU14. The owner's answers of 2026-10-09 after AU13's measurement and
the profile of the stage: (i) stage 8 of AU9, the parameters borrowed
across direct calls, is skipped: the retains and the releases of the
generated code are 2% each of the self-check and the parameters' share
of them smaller, the implementation would touch every exit of a frame
(the interpreter's return, the unwinding on a failure, the hand-back,
the generated code's return) with a borrowed mask per frame, and the
callee's own moves out of the slot (`WithSlot`, `TakeField`, `LoadMove`)
would have to be excluded; it stays recorded here for a later profile
that shows the parameters' retains to matter; (ii) the next stage is a
faster compilation tier, so that more code runs compiled on the cold JIT
run: the profile puts 45% of the run under `rt_call`, the calls that
found no generated code and ran their callee on the interpreter, with 30
code objects compiled under the hotness rule, and the image row, every
code object compiled, runs 29% fewer instructions; the session's
measurement of the compile cost followed, and the owner's third answer,
after it, chose the most thorough route (AU15), asking whether the
compile thread and a template tier combine, which the session answered:
the thread first, the template tier decided by its measurement; (iii)
the measure of that stage is the self-check's wall-clock, best of seven
with the two binaries alternating on one machine, with the cachegrind
rows reported beside it, since a compile thread's instructions count in
cachegrind's total as much as the interpreter's.** The measurement of
the compile cost that the second answer asked for, on the binary of
AU13: under the hotness factor 8000 the self-check compiles 54 code
objects (1,580 ops) in about 140 ms, two thirds of it register
allocation, 42 microseconds an op (about 130 thousand instructions),
with 29 Cranelift instructions and 2.1 blocks per op; compiling
everything called (564 code objects, factor 0) takes 1.28 s and the run
2.76 s against 1.76 s at 8000; the factors 2000, 500 and 100 are slower
than 8000 by 13 to 27%; Cranelift's single-pass register allocator
halves the compile time and the code it makes runs 10% slower. For more
code to be compiled profitably on the interpreter's thread, the cost per
op would have to fall five to ten times, which no knob of the current
tier gives. (user)

**AU15. Stage 8 of the typed round, in place of AU9's third item
(skipped by AU14): the JIT's compilation on a thread of its own. A hot
code object is handed to a compile thread, started at the first hot one,
with Cranelift's target and contexts of its own; the interpreter runs
the code object until its machine code comes back through a channel, and
places it at the next ask for that code object (a call, or a loop
header). The hotness factor on that path is `HOT_FACTOR_BACKGROUND`,
100, where it was 8000 (AR5) when the interpreter's thread paid for
every compilation: a code object is handed over once it has run a
hundred times its size in ops. The interpreter's thread stays the
compiler under `RENYI_NATIVE_SYNC`, under `RENYI_NATIVE_HOT=0`
(everything compiled at its first call, which the tests rely on to run
the generated code) and on a machine with one hardware thread, at the
factor 8000 as before; `RENYI_NATIVE_HOT=N` sets the factor on either
path. `renyi build` compiles on its own thread as before. The thread
reads the program through a raw pointer (`ProgramRef`), sound because
the program is never written while it runs and the thread is joined when
the JIT is dropped, before the program the VM borrows; the constants'
reference counts, which the interpreter changes, are read by nobody on
the thread.** Why a thread and not a cheaper tier first: AU14's
measurement; the thread removes the compile cost from the interpreter's
path on every machine with a second hardware thread, where the template
tier would have cut it on one. Why the factor 100: measured by this
stage's rule (AU14: the self-check's wall-clock, best of seven with the
binaries alternating on one machine), the self-check on the JIT run
against AU13's binary (1,727 ms): on the interpreter's thread at 8000
1,744 ms (+1%, the same code), on the compile thread at 8000 1,755 ms
(+1.6%: 54 code objects, the handing over and the placing for nothing),
at 2000 1,572 ms (-9.0%), at 500 1,557 ms (-9.9%), at 100 1,527 ms
(-11.6%), at 20 1,578 ms (-8.6%); `bench/records.ry` 52 to 49 ms at 100
(46 at 20), `primes` 41 to 37, `strings` 81 to 79, `json_round_trip`
unchanged. Confirmed on the binary with the factor built in, against
AU13's, alternating best of seven: the self-check 1,723 to 1,548 ms
(-10.1%), `primes` 42 to 37 ms (-11%), `strings` 77 to 75,
`json_round_trip` 91 to 92, `records` 49 to 51 (within the noise of a 50
ms program), `hello` 8 to 7. The cachegrind rows beside it, as AU14's
rule asks: the JIT run 9.95 to 13.22 billion instructions and 14.12 to
18.29 billion estimated cycles, the compile thread's 3.3 billion
instructions (about 210 code objects at the factor 100) counted in the
total as much as the interpreter's, which is why the rule of AT1 would
have refused this stage and AU14 changed the measure for it; the
interpreter 12.72 billion instructions as before, the image 6.80 and the
run from the cache 7.56, unchanged, since neither compiles at run time.
What the thread does not change: the interpreter stays the reference and
runs every code object whose code is not back yet; the recordings, the
replay and the narration run on it alone; a run's result is the same on
either path, since the generated code hands a frame back wherever its
assumptions fail (AR1). What is left open: the pass table of
`RENYI_NATIVE_REPORT` is the interpreter's thread's, empty when the
thread compiled; a template tier (AU14's question) is decided by a
profile of this stage on one and two hardware threads. (user)

**AU16. The owner's answers of 2026-10-09 after AU15's measurement: (i)
the template tier of AU14's question (a first tier of machine-code
templates per op without Cranelift, which would serve the machine with
one hardware thread, where the compile thread brings nothing, and
shorten the wait for the code everywhere) is to be built, as the next
large item after the stage below; (ii) the next stage now is the
liveness analysis over the bytecode: the last read of a slot as a move,
so that a value reaches its consumer held once and the updates in place
of AU11 find it so, estimated at three to six percent of the self-check
from the counts of AU11 (nine updates in ten found their record shared),
chosen over the representation items of AU1's step v, the two micro-cuts
(a fieldless variant as a constant, a borrowed `LoadField` operand) and
the closing of the round.** The measurement that framed the questions:
the self-check's wall-clock on the binary of AU15 against AU13's,
alternating best of seven, on four hardware threads -10.1%, on two
(`taskset`) -6.2%, on one +0.8% (the old path runs there); the profile
of the image row (6.80 billion instructions, every code object
compiled): the drops of values 10.9%, the clones 4.8%, the frames pushed
4.9%, the copies of `with` about 3%, the comparisons 3.1%, the field
reads through the cache 1.5%; the cold row's goal of AU1 (a quarter
below AT7) stands at -4.8% and is out of reach of stages of one to three
percent each. (user)

**AU17. Stage 9 of the typed round: the last read of a slot as a move, a
liveness pass in the VM (`renyi_vm/src/liveness.rs`). Before a program
runs (`run_program`, `run_tests`, `reproduce`, the sandbox of AP1) and
before its image is built (`renyi build`, the cache of AU10), the VM
takes a copy of the program and replaces every `Load` of a slot that no
path reads again before the slot is written or the frame ends by a
`LoadMove`, so that the value reaches its consumer held once and an
update in place (O1, AU11) finds it so. The liveness is the usual
backward one over the ops: the successors of an op are the next op
unless the op leaves the frame or jumps away, the target of a jump, the
exit of an `IterNext`, and, for every op between a `PushHandler` and its
target, that target, since a failure anywhere in the handled region
lands there with the slots as they are; the reads are `Load`,
`LoadMove`, `LoadField`, `TakeField`, `WithSlot`, `IterNext`,
`CheckDeadline` and `UnwindStack`, the writes `Store`, `IterInit`,
`Deadline` and `MarkStack`. The bytecode file, the image's program and
the cache's key stay what the emitters wrote: the pass is the VM's own
business, as the machine code is (AG1), so the two emitters and their
judges are untouched. The generated code borrows a moved load for a
typed call or a comparison as it borrows a plain one
(`borrowed_operands` of `codegen.rs`): the entries read their arguments
where they lie and update nothing in place, so the slot keeps its value,
which nothing reads again, and the helper counts the copy out; moving
such an operand out and releasing it after the call, as the first build
did, cost `bench/strings.ry` 10%.** Why in the VM and not in the two
emitters, which the owner's answer named: one analysis instead of two
held equal by a judge, no change to the bytecode file, and the moves
reach a program loaded from a bytecode file or an image too; the cost is
a copy of the program per run: on the checker program (958 code objects,
46,238 ops) the copy takes 0.9 ms and the pass 5.0 ms in all, 0.3% of
the self-check. Why the handled regions are intervals in the op order:
the emitters keep `PushHandler` and `PopHandler` balanced on every path
(a `break` or `continue` pops the regions it leaves before its jump), so
every op that can fail into a handler lies between the push and the
target, and the ops in that interval that run with the handler already
popped get an edge they do not need, which keeps a value live a little
longer at worst. What the pass changes in the checker program: 6,261 of
its 9,611 `Load`s become moves (the emitters wrote 429 `LoadMove`s;
6,690 after); of the 24.6 million `Load`s the self-check runs, 13.0
million are moves now; counted on the interpreter with the VM
instrumented as for AU11: of the 206,427 updates run, 36,551 find the
record held once where 23,848 did, and of the 8,266 takes 3,491 move the
field where 2,305 did; the rest still find the record shared, by the
`Done` record that carries the checker beside its value (the field taken
out of a record that is updated with the result later, the item left for
a later stage). Measured by AU1's rule against AU15's binary (30db0f3),
both binaries on one boot of the machine: the self-check's estimate on
the cold JIT run 18.29 to 17.89 billion cycles (-2.2%; 13.22 to 12.94
billion instructions, the compile thread's work counted in), the
interpreter 18.09 to 17.54 (-3.0%; 12.72 to 12.60 billion instructions),
the image 10.04 to 9.71 (-3.3%; 6.80 to 6.69 billion instructions; the
machine code 5.79 to 5.69 MB, 123.1 bytes of body per op, 771 deopt
points), the run from the cache 11.02 to 10.68 (-3.2%); the first build,
which moved the borrowed operands out, measured 18.15 on the cold run
and 9.80 on the image. In wall-clock, best of fifteen with the two
binaries alternating: the self-check 1,538 to 1,464 ms (-4.9%) on four
hardware threads, 1,575 to 1,469 (-6.8%) on two (`taskset`), 1,772 to
1,713 (-3.3%) on one; `bench/records.ry` -2.3%, `json_round_trip` -2.2%,
`primes` -1.1%, `strings` within noise; on the interpreter
(`--interpret`) the self-check 2,107 to 1,945 ms (-7.7%) and `strings`
182 to 149 ms (-18%), more than the estimate says, as the clones it no
longer makes touched the counts of values the simulated caches hold. The
stage stays by the rule's first clause; the estimate of AU16 (three to
six percent) is met on the interpreter and on the image and not on the
cold JIT run, where the compile thread's work is a sixth of the total.
(user)

**AU18. The owner's answers of 2026-10-09 on the design of the template
tier (AU16): (i) the machine code is emitted by a small x86-64 assembler
of the VM's own, each op a fixed sequence that keeps every value boxed
on the VM's stack and calls the helpers of the Cranelift tier for
everything but the trivial ops, over copy-and-patch stencils (the same
code for a build step and relocations more) and over Cranelift stripped
of its optimizations (half the cost at best, AU14, fifty times short of
what compiling at the first call needs); (ii) a code object gets its
template code at its first call, synchronously, on every machine alike
(microseconds per code object), over a small factor on the interpreter
and over the whole program at load; (iii) the template code counts the
hotness itself, at its entry and at its loop headers, and asks for the
promotion to the Cranelift tier with today's factors (100 on the compile
thread, 8000 synchronously where there is none), the Cranelift code
replacing the template entry at the next call, images and the cache
staying the Cranelift tier's, over a promotion only beside a compile
thread and over no promotion at run time; (iv) x86-64 in this round
(Linux, macOS and Windows: the System V and the Windows x64 conventions
both), aarch64 as a later stage with the sequences shared, the other
targets keeping the interpreter and the Cranelift tier.** The design as
put to the owner (the handoff's section "The template tier: the design
study"): the analysis of `infer.rs` settles the code object (the static
depth, the handled regions, the block starts; one it rejects stays with
the interpreter); the frame is in the interpreter's layout at every op
(the locals, then the operands, the stack's length `base + locals +
depth` known statically as AT4 has it), so there is no hand-back and an
entry at a loop header fills no registers and refuses nothing; the
function has the `Entry` signature of a trampoline (the VM, the base,
the pc; a status out: `RETURNED` with the value on the caller's stack,
`INTERRUPT`); the trivial ops are inline (`Const`, `Nothing`, `Load`,
`LoadMove`, `Store`, `Pop`, `Dup`, the jumps, the tag tests, the Boolean
branches), every other op a call to the helper the Cranelift tier calls
for boxed operands followed by the status check (`CONTINUE` falls
through, `FAILURE` lands on the floor of the innermost handled region
through `rt_settle_handler` or leaves through `rt_unhandled`, any other
status leaves the function); calls go through `rt_call`, `rt_call_typed`
(with the mask of `borrowed_operands`) and `rt_call_pure`; the code
holds no address (the helpers, the constants and the direct table
through `NativeState`, AS1), keeps the VM, the base and the stack's
pointer in callee-saved registers and reloads the pointer after a helper
that may grow the stack; every native test and the conformance suite run
on the tier too (`RENYI_NATIVE_TIER=template` forbids the promotion);
the measure is AT1's rows with the wall-clock of the self-check on one,
two and four hardware threads beside them, the goal the self-check on
one thread within 15% of the image row. The measurement that framed the
questions, on AU17's binary: the cold JIT run against the image
(`tools/bench.py`, best of five) on one hardware thread 1,647 to 1,213
ms (+36%; `primes` +34%, `records` +26%), on two 1,473 to 1,221 (+21%),
on four 1,467 to 1,214 (+21%); on one thread 54 code objects of the
self-check are compiled (factor 8000) and 510 called ones stay cold,
with 176,735 calls from generated code to the interpreter; the
interpreter's dispatch (`run_frames` less the helpers) is 46% of its
instructions, which is what the tier removes while it keeps the ops'
work, so template code should run at about two thirds of the
interpreter's instructions on boxed code; the short programs of an agent
(7 to 25 ms, nearly all the front end) are not what the tier is for, the
runs from a tenth of a second upwards on the machine with one thread
are. (user)

**AU19. The template tier as built (decisions AU16 and AU18; code format
6, image format 7): `crates/renyi_vm/src/native/template/`. `x64.rs`, a
small x86-64 assembler: the thirty-six instruction forms the sequences
need, labels with relative jumps patched at the end, so that the code
holds no address, and the two conventions (System V: the arguments in
rdi, rsi, rdx, rcx, r8, r9; Windows x64: rcx, rdx, r8, r9, a shadow
space of 32 bytes and the arguments past the fourth above it), with unit
tests of the encodings and a dump of every form for `objdump`. `mod.rs`,
the sequence per op: the frame in the interpreter's layout at every op,
every value boxed on the VM's stack; the VM, the frame's base, the
frame's address, the helpers' table, the base in bytes and the address
of the code object's count in callee-saved registers; the trivial ops
inline (`Const`, its three words copied from the code's table with a
retain unless plain or borrowed; `Nothing`; `Load` and `LoadMove` with
the borrowing of AU2, AU13 and AU17; `Store`, `Pop` and `Dup` with the
release or the retain through the helpers, as AT6 has it; the jumps; the
tag tests of `JumpIfAbsent` and `JumpIfFailure`; the Boolean branches
with the interpreter's crash on another value), every other op a call to
the helper the Cranelift tier calls for boxed operands; the stack's
length written after every op that changes the depth. The statuses as
the interpreter's arms treat them: a call, a construction, an update and
an expired deadline settle a failure (the landing of the innermost
handled region through `rt_settle_handler`, else `rt_unhandled`), while
an operator, a field, a text, a global, a range and an iteration push a
failure as a value and leave only on an interrupt (the Cranelift tier
sends such a status to its exit as an interrupt, which no program has
met). The tiers: `Jit::entry` gives the Cranelift code when it is ready,
else the template code, made at the first call and packed into one
executable chunk (`CodeArena::place_packed`: the pages a template lies
on writable while it is written, executable again before anything runs);
the template code counts the ops it runs as the interpreter counts them,
a basic block's length added to the code object's count as the block
starts, so that the same code objects are compiled at the same points as
before; past the threshold (the size times 100 with the compile thread,
8000 without) a back edge calls `rt_osr`, which asks for the Cranelift
code (compiled on the spot, or handed to the compile thread) and enters
it at the loop header when it is ready and has the entry, the frame
being in the layout the interpreter hands over; when it does not, the
count starts again. `RENYI_NATIVE_TIER=template` keeps everything on
templates, `=cranelift` turns the tier off; other targets than x86-64
keep the interpreter and the Cranelift tier. A defect of the Cranelift
tier met in the review of the promotion: a frame handed back to the
interpreter inside a handled region lost the region (the generated code
keeps the regions static, the interpreter on `Vm::handlers`), so a
failure after the hand-back crashed as unhandled where the interpreter
lands on the fallback; a deopt point now carries the regions open at it,
which `rt_deopt` pushes over the frame's floor (image format 7).** The
tests: `tests/template.rs` (four programs on templates alone against the
interpreter: every source of a loop, every kind of call, updates in
place, a failure handled and passed on, a crash located, a recursion
past the depth of native frames), `tests/promotion.rs` (the factor 1,
synchronous, so that loops and calls are promoted at once: three code
objects on templates, three compiled, two running loops handed over),
`tests/native.rs` (the hand-back inside a handled region), the
conformance suite and the judges with `RENYI_NATIVE_TIER=template`, and
the self-check's output equal on templates. Not run: the Windows x64
convention, since neither this container nor CI has a Windows machine;
it was checked by reading (the argument registers, the shadow space, the
arguments past the fourth, the alignment, the callee-saved registers the
code uses and those it never touches), and its first run is `cargo test`
on the owner's machine, where `tests/template.rs` forces the tier. No
unwind tables are registered for template code, as for the Cranelift
tier's, so a panic inside a helper called from either aborts the
process. Two corrections came from the first measurement, both in the
design above: the first build added the whole code object's size to its
count at every entry, which made short functions that return early look
hot (on one hardware thread 88 code objects compiled where AU17's binary
compiled 54, 252 ms of Cranelift where it took 140, the self-check 6.6%
slower; on four 9.7%); and it placed every template in fresh pages of
its own, 564 mappings with every function at the same page offset, which
cachegrind does not see (it models no TLB) and which made the self-check
6.8% slower on one thread although the estimate fell 5.9%. Measured
against AU17's binary (d8ee092), both on one boot of a machine slower
than the one of AU17's entry by about 15%. By AT1's estimate under
cachegrind: the synchronous JIT run (`RENYI_NATIVE_SYNC`, the path of
one hardware thread, which the compile thread's timing does not make
vary) 13.99 to 13.12 billion cycles (-6.2%; 9.87 to 9.45 billion
instructions); the template code alone (`RENYI_NATIVE_TIER=template`)
14.73 against the interpreter's 17.55 (-16%; 11.12 against 12.60 billion
instructions); the interpreter unchanged (17.55 to 17.55); the image
9.71 to 9.78 and the run from the cache 10.68 to 10.74 (+0.8% and +0.6%,
the instruction misses up with the binary's layout, the machine code the
same); the default JIT run, with the compile thread, is no longer a
measure under cachegrind, whose serialized threads gave AU17's binary
12.94, 14.40 and 16.70 billion instructions in three runs. In
wall-clock, the two binaries alternating: on one hardware thread the
self-check -1.3% (best of fifteen) and -4.8% (best of seven, 1,992 to
1,897 ms), `strings` -3.0%, `json_round_trip` -1.9%, `primes` -0.6%,
`records` +0.2%; on two -3.1%, the benchmarks within 2%; on four the
median of twenty-five runs -1.1% and the best +1.9%, which is noise, the
benchmarks within 2%. The stage stays by AU1's rule (the cold run's
estimate falls) and by the owner's measure (no regression on any count
of hardware threads); the goal of AU18 is not reached: on one hardware
thread the JIT run takes 1,897 ms against the image's 1,335 (+42%, where
AU18 asked for 15%), and the templates alone 2,248 against the
interpreter's 2,347 (-4.2% in wall-clock where the estimate says -16%).
Where the gain goes: every call from template code goes through
`rt_call`, `call_from_stack`, `run_top_frame`, `Jit::entry` and
`run_generated`, where the interpreter's own calls push a frame and go
on in its loop, and the self-check makes 5.4 million calls; the hot
helpers (the field read, the retain, the release) stay calls; the
follow-ups that address both are put to the owner. (user)

**AU20. The owner's answers of 2026-10-10 after AU19's measurement: (i)
the next steps of the round are both of the follow-ups put to the owner,
in this order: first the template tier's own (direct calls between
template functions, so that a call whose callee has machine code pushes
the frame and enters it without `rt_call`, `call_from_stack`,
`run_top_frame` and `Jit::entry`, and the field read through the site's
cache in place, as the Cranelift tier reads it), then the representation
items of AU1's step v (small texts inline, `Int` flattened into `Value`,
lists of unboxed Integers, values in registers across ops); (ii) a
Windows job in CI runs the VM's tests on every push, so that the Windows
x64 convention of the template tier runs (`.github/workflows/ci.yml`,
the job `windows`); (iii) the goal of AU18 stays the measure of the
follow-ups: the self-check's JIT run on one hardware thread within 15%
of its image; (iv) the template tier stays on by default, as measured.**
The alternatives the owner declined: for (i), closing the round and
deciding the self-hosted front end (AU12), or the aarch64 encoder; for
(ii), verifying on the owner's machine alone, or the tier off on Windows
until it had run; for (iii), the goal relaxed to 25% or AT1's rule
alone; for (iv), the tier off until the follow-ups pay, or on only where
there is no compile thread. (user)

**AU21. The template tier's follow-ups as built (decision AU20, its
first part): direct calls between template functions and the field read
in place. The entry table (`Jit::entries`, reached through
`NativeState::entries`) names per code object the machine code a call
enters: the Cranelift tier's trampoline once it is ready, else the
template, else nothing. `Op::Call` of a declared function in template
code checks the callee's entry and the depth of native frames, pushes
the callee's frame in place as `Vm::push_frame_in_place` pushes it (room
on the stack and for the record, the stack's length to the callee's
locals, its fresh locals `Nothing`, written in place up to four and by
`rt_clear_slots` past that, since a Cranelift callee may hand its frame
to the interpreter at its start, before its own prologue clears them;
the grant of Q1; the record), and calls the entry with a trampoline's
signature; the status decides the rest: the result where the arguments
were, a failure landing on the handler as the interpreter settles it;
the frame handed to the interpreter, which `rt_finish_frame` runs to its
end; an interrupt, after which `rt_abandon_frame` abandons the frame as
`run_top_frame` does. A callee without machine code, or a call at the
depth limit, goes through `rt_call`, which makes the callee's template
on its way; these rare paths lie after the body. Since a direct call no
longer passes `Jit::entry`, a template whose count is past the threshold
at its start asks `rt_promote` for its Cranelift code, and the entry
table names the trampoline once it is placed. `LoadField` in template
code reads the holder's type and tag against the site's cache entry and
copies the field with a retain, as the Cranelift tier reads it (AR4); a
miss calls `rt_load_field`, which fills the cache.** The tests:
`tests/direct.rs` (the default factor, synchronous: a short callee
promoted while its caller stays on template code, then called with an
Integer past the machine word, which hands its frame back at the
trampoline's start; a callee that narrows the grant and one that does
not), with the template, promotion and native tests and the conformance
suite and the judges on the template tier. The Windows job of AU20
passed on its first push (CI run 78), `tests/template.rs` among its
tests, so the Windows x64 convention has run. The template code of the
self-check's run grows from 1.58 to 2.76 MB. Code format 7 (the four
helpers and the state's `entries`). Measured on one boot against AU19's
binary (bbea69b). By AT1's estimate under cachegrind: the synchronous
JIT run 12.87 to 11.71 billion cycles (-9.0%; 9.45 to 8.32 billion
instructions, -12.0%, the instruction misses up from 92 to 101 million
with the larger template code); the template code alone 14.50 to 12.18
(-16.0%; 11.12 to 8.79 billion instructions, -21.0%), now 29% below the
interpreter's 17.24 where AU19's was 16% below it; the interpreter and
the image, whose code the stage does not touch, 17.24 and 9.67 on this
boot. In wall-clock, the two binaries alternating, best of eleven: on
one hardware thread the self-check 1,595.9 to 1,398.8 ms (-12.4%),
`primes` -3.2%, `records` -1.2%, `json_round_trip` -0.7%, `strings`
+0.5%; on two the self-check -2.6%, the benchmarks -0.2% to -4.7%; on
four the self-check -1.3%, the benchmarks -1.0% to -2.1%. The stage
stays by AU1's rule (the estimate falls) and by the owner's measure (no
count of hardware threads loses). The goal of AU18 is nearer and not
reached: on one hardware thread the JIT run, alternating with the
image, takes 1,461.5 ms against the image's 1,162.9 (+25.7%, best of
fifteen; +21% by the estimate, 11.71 against 9.67), where AU19 left it
at +42%. (user)

**AU22. The owner's answers of 2026-10-10 after the design study of the
representation items (the handoff's section "The representation items:
the design study"): (i) the order: small texts inline first (a text of
at most 15 bytes held in the value itself, two of them compared by the
generated code), then records and variants in one allocation (the
fields in the counted block after the header), each a stage measured
by AT1's rule; (ii) lists of unboxed Integers and `Int` flattened into
`Value` are deferred until a numeric program calls for them; (iii) the
three cuts the census found outside the list come first, each a stage
of its own: the guard bookkeeping skipped in a run whose grant guards
nothing, a fieldless variant read from the cache by the generated code,
one shared empty list; (iv) the goal of AU18 is measured on the run
part: the self-check's JIT run on one hardware thread, without the
front end's compilation of the checker program, within 15% of its
image; the front end's speed is an item of its own.** Why (ii): no
program of the five makes a list of Integers worth unboxing (`primes`
counts over ranges; the self-check makes 3,249 such lists of 741,923);
the flattened `Int` alone estimates below 1% of the self-check, whose
Integers live in registers on the Cranelift tier, and it pays mainly as
the step to a 16-byte `Value`, which would cap the inline texts at 7
bytes. Why (iv): of the gap the study measured (the JIT run 1,456 ms on
one hardware thread, the image 1,212, +20%), the front end that
compiles the checker program, 24,000 lines, takes 155 ms, about two
thirds; a representation item makes the run and the image faster alike
and so widens the ratio as stated, while an agent's script of a few
hundred lines spends a few milliseconds in the front end and a repeated
run loads its image from the cache (AU10). The run part is the time
from the program loaded to the end of `main`, which `RENYI_NATIVE_REPORT`
now prints after the JIT's report for the JIT run and for the image
alike; by the study's numbers it stands about 7% above the image. The
alternatives the owner declined: for (i), records first, the texts
alone, values in registers first; for (ii), the flattened `Int` after
the texts and the records, or both items as AU1 listed them; for
(iii), the cuts after the representation items, or none; for (iv), the
goal kept as stated with a round on the front end's speed after the
representation items or in a round of its own, or AT1's rule alone.
(user)

**AU23. The first cut of AU22 as built: the guard bookkeeping skipped
in a run whose grant guards nothing. The VM keeps `guarding`, set by
`begin_run` when the grant (with a sandbox's) carries a guard and never
cleared, so that a VM that runs examples and tests one after the other
follows the data again from the first guarded one; while it is false no
value carries origins (only `Value::guarded` with a guard's bit makes
one), and the constructions of the interpreter and of the generated
code's helpers (`MakeList`, `Construct` and `ConstructVariant`;
`rt_make_list`, `rt_construct` and `rt_construct_variant`) take their
operands as they are, without `plain_all` and without `guarded`; a
primitive's result is wrapped only when there are origins to add
(`run_native`).** The first build took another way: the slow paths of
`guarded`, `into_plain` and `plain_all` moved out of line and their fast
paths inlined everywhere. It ran fewer instructions on every program
(the self-check's synchronous JIT run -2.0% by the estimate, `records`
-3%) and was slower in wall-clock on `records` (+3% on the benchmark,
+4% on the same program with two million points), which a build with
`guarded` kept out of line put back: the inlined moves of a 24-byte
value cost what the count of instructions does not show (cachegrind
models no store forwarding). Calling no wrapper at the call sites, as
built, gained instead (the two-million-point program -10.8% on one
hardware thread). The test: `tests/guards.rs`,
`a_guard_after_a_run_without_one_follows_the_data_into_a_record` (a test
without a guard, then one that writes the text of a record built from
the secret; with the flag never set the record is not tagged and the
text leaks, as the test was checked to show), beside the guard tests
that run with the flag set. Measured on one boot against AU21's binary
(fb15b1c). By AT1's estimate under cachegrind: the synchronous JIT run
11.71 to 11.44 billion cycles (-2.4%; instructions -2.8%), the template
code alone 12.18 to 11.91 (-2.2%), the image 9.67 to 9.35 (-3.3%), the
interpreter 17.24 to 17.24 (instructions -0.8%, its branch misses up
with the layout of its loop). In wall-clock, the two binaries
alternating with the order swapped every turn, best and median: on one
hardware thread the self-check -1.0% and -2.3%, `records` -7.0% and
-5.9%, `strings` -1.0% and -1.5%, `primes` -1.0% and -0.3%,
`json_round_trip` +0.4% and +1.4%; on two the self-check -2.0% and
+0.7%, `records` -9.6% and -12.0%, the others within 1.5%; on four the
self-check -6.5% and -7.2%, `records` -11.7% and -10.6%, `strings` -2.6%
and -5.0%, `json_round_trip` and `primes` within 2%. The stage stays by
AU1's rule and by the owner's measure. (user)

**AU24. The second cut of AU22 as built: a variant without fields read
from the cache in place by the generated code. The VM keeps the
variants without fields it has built (decision AG6) in a vector with a
fixed layout, `Vm::unit_variants`, `Nothing` in a slot until its
variant is built; `vm::unit_slot` gives a variant's slot from the
program's types as the VM lays them out. `ConstructVariant` of a variant
without fields reads the slot's tag in both tiers: once built, the value
is copied onto the stack and the count of its block, the block's first
word, raised by one in place; the first time, `rt_construct_variant`
builds and keeps it, on a path after the body in template code. Code
format 8.** The tests: `tests/native.rs` and `tests/template.rs`,
`variants_without_fields_are_copied_from_their_slot_and_counted` (3,000
lights cycled and kept in a list, compared and matched, and 2,000
shapes with and without fields, against the interpreter; without the
raised count the template test crashes, as it was checked to), with the
conformance suite and the judges on the template tier. Measured on one
boot against AU23's binary (2abc849). By AT1's estimate under
cachegrind: the synchronous JIT run 11.44 to 11.29 billion cycles
(-1.3%; instructions -1.7%), the template code alone 11.91 to 11.71
(-1.7%), the image 9.35 to 9.21 (-1.5%), the interpreter 17.24 to 17.21
(-0.2%). In wall-clock, alternating with the order swapped every turn,
best and median: on one hardware thread the self-check -3.9% and -2.3%,
the benchmarks within 1% but `records` (-0.8% and -2.8%) and `strings`
(+0.0% and -5.1%); on two the self-check -1.0% and -0.5%, the
benchmarks within 1.2%; on four the self-check +0.0% and +0.9%, the
benchmarks within noise (`records` +2.3% and -0.8%, `strings` -0.6% and
+3.0%). The stage stays by AU1's rule and by the owner's measure. (user)

**AU25. The third cut of AU22 as built: one shared empty list.
`Value::list` of no items gives a list that every such list shares, kept
per thread as the ASCII table keeps the one-character texts; a list
appended to later is copied out of it first, as any list held twice is
(`Rc::make_mut`).** No test is new: the corpus, the conformance suite
and every test that builds a list from `[]` and appends to it run the
copy. Measured on one boot against AU24's binary (f26c1ca), the cut is
neutral. By AT1's estimate under cachegrind: the synchronous JIT run
11.292 to 11.289 billion cycles (-0.03%; instructions -0.3%, the branch
misses up 1.8 million, of the copies the shared list costs where a list
made empty is appended to, which most of the self-check's are: the
saving is the lists that stay empty), the template code alone level
(+0.04%), the image level (-0.04%), the interpreter -0.3%. In
wall-clock the self-check moved between -3.6% and +1.0% (best of
fifteen and of twenty-one) on one hardware thread, +0.2% on two and
-2.7% on four, and `strings`, which makes no empty list and runs the
same instructions on both binaries, +3.3% on one thread over sixty-one
runs: the layout of the binary, which every change moves. The cut stays
by AU1's rule (the estimate falls, if barely) as the smallest of the
three; the census's estimate of 0.4% assumed lists that stay empty.
(user)

**AU26. The first representation item of AU22 as built: small texts
held in the value. A text of at most 15 bytes is `Value::SmallText`, a
tag of its own (20, appended so that no other tag moves) with the length
at the payload and the bytes after it, zero to the end of the value; a
longer one stays `Value::Text`. `Value::text` and `Value::character`
choose by the length, so that a text has one form for its characters,
and every reader takes either: `as_text`, `natives::text` and
`plain_text`, the renderer and its ordering, the comparisons, the
concatenation, the JSON, the bytecode file and the image's program
(which write a text constant as before, so that the judges stay equal).
Equality compares the characters, and a text hashes as its characters
in either form. The ASCII table of `Value::character` goes: a character
is a small text. In both tiers, `is` and `is not` on operands of which
the checker noted one a Text compare two small texts in place (both
tags read, the sixteen bytes past them compared as two words, the
Boolean written, nothing to release); anything else takes `rt_compare`
as before. Code format 9.** The first build copied a short text with
`copy_from_slice`, which called `memcpy` and `memset` for every
character made: `strings` ran 1.1% more instructions and lost 7% in
wall-clock. The copy now reads and writes two overlapping words, halves
or quarters as the length asks, without a call, and a character is
encoded straight into the value. The tests: in `value.rs`, a short text
at every length from 0 to 15 and the characters of one to four bytes,
equality and hashing across the two forms, and the layout test with the
new tag; in `tests/native.rs` and `tests/template.rs`,
`texts_held_in_the_value_compare_in_place_and_others_as_before` (texts
of 1 to 28 bytes and a two-byte character compared both ways and
ordered, the characters of a line against a constant, against the
interpreter; with the answer inverted in either tier the test fails, as
it was checked to); the conformance suite and the judges on both tiers.
Measured on one boot against AU25's binary (1c27240). By AT1's estimate
under cachegrind: the synchronous JIT run 11.29 to 10.46 billion cycles
(-7.4%; instructions -5.6%, the indirect branch misses 67.8 to 47.1
million with the calls to `rt_compare` gone), the template code alone
11.71 to 10.64 (-9.1%), the interpreter 17.17 to 16.15 (-5.9%), the
image 9.21 to 8.38 (-9.0%). In wall-clock, alternating with the order
swapped every turn, best and median: the self-check -7.8% and -5.3% on
one hardware thread, -7.4% and -3.9% on two, -6.8% and -7.0% on four;
the benchmarks within 3% either way (`strings` +2.8% and +1.1% on one
thread with 6.9% fewer instructions, the layout again). The study had
estimated 4 to 5%. The stage stays by AU1's rule and by the owner's
measure. (user)

**AU27. The second representation item of AU22 as built: records and
variants in one allocation. A record or a variant is a `Composite`, a
thin pointer to one counted block that holds the count, the number of
fields, the type, the tag (a record's `usize::MAX`) and the fields
themselves; `Deref` gives its `Shape` (the type, the tag and the fields
as a slice: a type with a slice tail), so that the code that read
`record.ty` and `record.fields` reads them as before. A clone raises the
count in place, a drop lowers it and the last holder frees the block out
of line, as an `Rc` does; `make_mut` copies the block before an update
unless it is held once, `get_mut` gives it when it is. The type and the
tag lie where the former `Rc` held them (16 and 24 bytes from the
block's start) and the fields start at 32, so that a field read in
either tier adds the index to the block's address where it first loaded
a pointer to the fields. `Construct` and `ConstructVariant` of a type
without a refinement make the composite from the operands on top of the
stack, moved into the block with no vector between
(`Vm::construct_from_top` and `construct_variant_from_top`); the guarded
path, the refined types and the natives build from a vector as before,
its values moved into the block. Code format 10.** Two corrections came
from the profile of the first build, which ran no fewer instructions
than AU26's (+0.1%) though the allocations fell: the composite's drop
was a call at every lowered count where `Rc` lowers it in place and
calls only to free (the fast path is now inline, `Composite::free` out of
line), and its clone checked the count against `usize::MAX` with a call
to `abort`, which gave every clone of every value a frame of its own
(the clone is now a plain increment, as the generated code raises a
count, with a debug assertion in its place). The tests: in `value.rs`,
the block's layout (the count, the length, the type, the tag and the
fields where the generated code reads them; the count raised and
lowered by a clone and its drop) and
`a_composite_is_copied_before_an_update_unless_it_is_held_once`
(`make_mut` on a shared and on a unique block, `get_mut`, `from_top`, an
empty composite); in `tests/native.rs` and `tests/template.rs`,
`records_in_one_block_are_built_read_copied_and_compared` (2,000
segments of two points with texts long and short kept in a list, a
point copied by `with` while shared and compared, variants with fields
matched, against the interpreter; with the clone's count not raised the
test aborts, as it was checked to); the conformance suite and the judges
on both tiers. Measured on one boot against AU26's binary (5be5e60). By
AT1's estimate under cachegrind: the synchronous JIT run 10.46 to 9.97
billion cycles (-4.6%; instructions -3.8%), the template code alone
10.64 to 10.18 (-4.4%), the interpreter 16.15 to 15.81 (-2.1%), the
image 8.38 to 7.87 (-6.1%). In wall-clock, alternating with the order
swapped every turn, best and median: the self-check -10.5% and -8.3% on
one hardware thread, -3.9% and -2.0% on two, -4.4% and -6.6% on four;
`records` -10.5% to -12.9%; `json_round_trip` between -0.7% and +4.6%,
whose records come from the decoder's vectors and are copied into their
blocks (0.9% more instructions); `strings` and `primes` within 2% at the
best. The stage stays by AU1's rule and by the owner's measure. (user)

**AU28. The owner's answers of 2026-10-10 after AU27: (i) the next
stage is the retain and the release in place in template code: a tag
test and a raised count where a copy of a value calls `rt_retain_at`
today, a lowered count with a call only when it reaches zero where a
dead value calls `rt_drop_at`, judged by AT1's rule (the template code
grows, and AT6 found calls cheaper than counts in place on the
Cranelift tier); (ii) after it, as a small stage, the natives build a
record in its block directly (the decoder of `std.json` first), which
takes back the 0.9% of instructions AU27 cost `json_round_trip`; (iii)
the front end's speed is a round of its own after this one.** The
alternatives the owner declined: for (i), the hottest natives in place
(`List.at` on a small Integer, `Text.contains` on small texts), the
compile cost on one hardware thread, or the round closed and the
self-hosted front end decided (AU12); for (ii), before the next stage,
or not at all; for (iii), within this round, or not planned. (user)

**AU29. The first item of AU28 as built: the retain and the release in
place in template code. Where a copy of a value called `rt_retain_at`,
the code reads the tag and tests its bit in a mask of the tags whose
payload points at a counted block (`RC_TAGS`, the composites of AU27
among them) and of the Integer's: a value with neither holds no count
and a short jump skips the rest; a counted block has its first word,
the count, raised in place; an Integer goes out of line, where a big
one raises its block's count and a small one nothing. Where a dead
value called `rt_drop_at`, the same test lowers a counted block's count
in place, and only a count that reaches zero goes out of line, where it
is raised back to one and the helper drops the value, which frees the
block and what it holds; a big Integer goes to the helper, a small one
nowhere. The assembler has two forms more, `bt` of two 32-bit
registers and the conditional jump with a one-byte offset, patched with
the others when the code is finished and refused there when its target
lies out of reach. An image holds no template code: the code format
stays 10.** Why the template tier takes what AT6 refused the Cranelift
tier: there the call replaced AR4's generic sequence, which tested the
tag's bit, the big Integer and both payload words at every site; here
the sequence tests one bit and skips, and the call it replaces went
through the helpers' table, so that the indirect branches the model
counts as missed fell from 47.3 to 28.9 million on the template code
alone. The template code grew: the self-check's 564 code objects (30,602
ops) took 2,789,448 bytes and take 3,480,698 (+24.8%; 91 to 114 bytes
per op), and the instruction misses rose 70.3 to 84.5 million; AT1's
estimate weighs both. The tests: in `x64.rs`, the encoding of `bt` and
of a short jump over two bytes, and a short jump past its reach
refused; in `tests/template.rs`,
`counts_raised_and_lowered_in_place_free_the_last_and_spare_big_integers`
(3,000 records with a text past sixteen bytes and an Integer past the
machine word, a third kept in a list, the last Integer kept across the
loop, against the interpreter; with the raise or the lowering taken out
the test aborts, as it was checked to); the conformance suite and the
judges on both tiers. Measured on one boot against AU27's binary
(14ce593). By AT1's estimate under cachegrind: the template code alone
10.18 to 9.65 billion cycles (-5.2%; instructions -4.1%), the
synchronous JIT run 9.97 to 9.78 (-1.9%; instructions -1.7%), the
interpreter 15.81 to 15.83 and the image 7.87 to 7.90 with their
instructions equal to a hundredth of a percent (the binary's layout).
In wall-clock, alternating with the order swapped every turn, best and
median: the self-check -5.6% and -7.6% on one hardware thread, -0.8%
and -1.8% on two, -3.6% and -6.0% on four; the benchmarks between
-4.2% and +3.2%, faster in 18 of their 24 numbers (`json_round_trip`, which
the next stage takes on, between -1.9% and +3.2%). The run part of AU22, the JIT run over the image on one
hardware thread, alternated eleven times and measured twice: +9.7% and
+9.1% at the best, +7.7% and +3.0% at the median, where AU27's binary
measured +11.0% and +13.6%, +12.2% and +19.7% on the same boot: the
goal of AU18 (within 15%) is reached on this boot at the best and the
median, the JIT run part 1,071.9 to 1,035.3 ms at the best (-3.4%). The
stage stays by AU1's rule and by the owner's measure. (user)

**AU30. The second item of AU28 as built: the decoder of `std.json`
builds a record in its block. The fields of a record or a variant are
decoded onto the VM's stack in their order (`decode_fields`) and moved
from there into the block (`Composite::from_top`), as the bytecode's
constructions move theirs since AU27, with no vector between; a type
with a refinement to check takes its fields off the stack into a vector
and goes through `construct` or `construct_variant` as before, and so
does a variant without fields, which stays shared (AG6). A field that
does not decode, or an interrupt, cuts the stack back to the height it
had, and `decode` asserts in a debug build that it leaves the stack as
it found it. The natives that build a record from a vector of known
length (the library's records and errors, the rows of `sqlite.query`,
the nodes of a `JsonValue`) keep it: no measured program spends on
them.** Before AU27 a decoded record kept the decoder's vector and put
an `Rc` around it, two allocations; AU27 made them the vector and the
block, with a copy between; the block is now the only one. The decoder
calls `Composite::from_top` itself rather than `Vm::construct_from_top`,
for a reason found by measurement. The first build called the VM's
function, which LLVM had inlined into its two callers (the
interpreter's loop and `rt_construct`, the generated code's helper)
and kept out of line once the decoder made a third: the self-check's
synchronous JIT run executed 47.6 million instructions more (0.7%), the
calls the profile put on the two functions. The second build forced
the inlining everywhere: the rows of the generated code came back
level, but the interpreter's loop compiled otherwise and ran 1.2% more
instructions with 43% more branch misses. The third leaves the VM's
functions with their two callers, and the interpreter's loop and the
helpers are the machine code of AU29, to the byte in size;
`Composite::from_top`, `construct` and `construct_variant` were out of
line in every build, so that a caller more changes nothing. The tests:
in `tests/semantics.rs`,
`decoded_records_are_built_in_their_block_and_keep_their_refinements`
(a record and a variant with a refined field decoded where the field
holds and where it does not, the variant refused after the record
beside it was decoded, a field that does not decode in the middle of a
record, the runs after each reading their values whole, interpreted and
not; with the refinements skipped the test fails, as it was checked
to); the assertion's power check (with the cut taken out of
`decode_fields`, `json_paths` of the conformance suite aborts on it, as
it was checked to); `json_paths` itself on both tiers; the judges. No
test ran the refined records of the decoder before: the corpus's two
programs that decode one (`config.ry`, `active_users.ry`) do it in
functions no example or test calls. Measured on one
boot against AU29's binary (8b642fd). By AT1's estimate under
cachegrind on `json_round_trip`: the synchronous JIT run 1.067 to 1.033
billion cycles (-3.2%; instructions 938.4 to 901.3 million, -3.9%), the
template code alone -3.3%, the interpreter -3.1%; on the self-check
every row with its instructions equal to a hundredth of a percent (the
estimates 0.1% to 0.9% lower with the binary's layout). In wall-clock,
alternating with the order swapped every turn, best and median:
`json_round_trip` -2.0% and -1.9% on one hardware thread, -1.1% and
-7.2% on two, -1.1% and -1.3% on four; the self-check, whose
instructions are equal, within the noise (+1.1% and +0.4%, -0.4% and
+0.9%, +0.7% and -2.3%); the other benchmarks between -5.8% and +2.8%.
The stage stays by AU28's measure: it takes back more than the 0.9% of
instructions AU27 cost the benchmark. By AU1's rule alone it is level:
the self-check's estimate falls only with the layout, and the
benchmark's gain is below the 5% of the rule's second branch. (user)

**AU31. The owner's answers of 2026-10-10 after AU30: (i) the next
stage is AU29's sequence on the Cranelift tier, the retain and the
release in place in the code Cranelift makes where AT6 kept the calls
to `rt_retain_at` and `rt_drop_at` (AT6 had measured them against AR4's
longer generic sequence), judged by AT1's rule; then the hottest
natives in place in the generated code of both tiers (`List.at` with a
small Integer index, `Text.contains`, `List.contains`), a stage of its
own; (ii) the goal of AU18 is called met only when a measurement on
another boot of the container (a later session, after a restart) finds
the run part within 15% again; (iii) the natives that build a record
from a vector (the rows of `sqlite.query`, the library's records and
errors, the nodes of a `JsonValue`, which also look their type up by
name for every node) stay as they are until a profile points at them;
(iv) the front end's round, when it opens, starts with a study: the
profile of the parse, the check and the compile on the self-check and
the corpus, with a cache keyed by the sources' hash that skips the
front end among the candidates, and the owner orders the stages from
it.** The alternatives the owner declined: for (i), the hottest natives
first, the compile cost first, or the round closed for the front end's;
for (ii), the goal called met on this boot's two measurements, or a
stricter measure (the median within 10%, or three boots); for (iii),
the `JsonValue` nodes now, or every such native as one stage; for (iv),
the cache keyed by the sources first, or the checker's hot spots
directly. (user)

**AU32. The first item of AU31 as built: AU29's sequence on the
Cranelift tier, and the compile thread stopped at the end of a run.
In the code Cranelift makes, `retain` and `release` test the tag's bit
in `COUNTED_OR_INTEGER` (now one constant in `value::layout` for both
tiers): a value with neither goes on; a counted block has its first
word, the count, raised or lowered in place; an Integer goes to a cold
block, where a small one holds no count, a big one's count is raised
in place on a retain and the value goes to `rt_drop_at` on a release; a
count lowered to zero goes to a cold block that raises it back to one
for `rt_drop_at`, which frees the block, the one call a release has.
`rt_retain_at` is gone from the helpers: code format 11. The JIT's
compile thread, when the VM is dropped, stops after the compilation
under way, where closing its channel alone let it compile every
request still queued before the process could end.** Four builds were
measured against AU30's binary (81aa432) on one boot, all four in AT1's
rows. (a) A release with two calls, one for the last reference and one
for a big Integer: the machine code of the self-check's image 5.73 to
7.69 MB, the synchronous JIT run's instructions -0.2%, the image's
-5.0%. (b) Every Integer through the helpers, no test of a small one in
place: 7.43 MB, the image's estimate 1.0% behind (a). (c) As (a) with a
big Integer's retain through `rt_retain_at`: 7.91 MB and more
instructions than AU30 on the JIT run, since a call in a cold block
costs the register allocation of the whole body. (d) As (a) with the
release's two calls one: 7.65 MB (+33.6%; 5,976 to 7,986 bytes of body
per code object) and 0.2% fewer instructions than (a); it is the stage.
AT6 had kept the calls against AR4's sequence, which tested the big
Integer and read both payload words at every site; this one tests one
bit and skips. The compile thread's correction came from the
measurement: with (d)'s code, larger by a third, the time from the end
of `main` to the end of the process on two hardware threads rose from
187 to 324 ms, the thread compiling its queue for code nothing would
run; the comment on `Worker`'s drop had said that only the compilation
under way finishes. The tests: in `tests/native.rs`,
`counts_raised_and_lowered_in_place_free_the_last_and_spare_big_integers`
(records with long texts and a weight past the machine word as a `maybe
Integer`, boxed, so that its copies are counted in this tier's code,
which keeps an Integer in a register and hands a big one to the
interpreter; against the interpreter; with the counted block's raise
taken out the test aborts, with the big Integer's it crashes, as it was
checked to); the conformance suite and the judges on both tiers. By
AT1's estimate under cachegrind, against AU30's binary: the image 7.89
to 7.53 billion cycles (-4.6%; instructions -5.1%, the indirect
branches missed -47%, the instruction misses +31%); the synchronous JIT
run level within the layout's spread, which two builds each of (a) and
(d), the generated code the same within each pair, put at -0.4% and
+0.1%, -0.3% and +0.4% (its instructions -0.2% and -0.4%; the template
row, whose code did not change, moved +1.1% in the last); the
interpreter level; `strings` -2.2% and `records` -3.8% on the JIT run. Cranelift's work on the 54 code objects of the synchronous
run: 44.0 to 57.0 thousand instructions of IR (+30%) in 3.3 to 6.4
thousand blocks, 76-90 to 84-93 ms. In wall-clock on the self-check,
alternating fifteen times: the compile thread's correction alone level
on one hardware thread, -6.3% and -5.3% (best and median) on two, -4.8%
and -3.9% on four; the counts on top of it -2.7% and -3.7% on one,
-5.4% and -8.1% on two, -10.8% and -9.5% on four, the run part of AU22
-5.0%, -7.2% and -13.1% at the best. The stage against AU30's binary,
alternating with the order swapped every turn, best and median: the
self-check -9.3% and -3.0% on one hardware thread, -13.5% and -15.5% on
two, -9.4% and -14.0% on four; `strings` between -13.1% and -16.0%,
`records` between -3.8% and -6.9%, `json_round_trip` and `primes`
within the noise (-3.7% to +5.1%). The run part of AU22 on one hardware
thread, eleven alternations: the image 929.7 to 776.8 ms at the best
(-16.4%), the JIT run 1,009.3 to 991.5 (-1.8%), so that the JIT run over
the image went from +8.6% to +27.6% (+8.2% to +28.7% at the median):
the image gains everywhere, the JIT run only where Cranelift's code
runs, and its code is on templates for most of the run. AU18's goal
moves away by a stage that makes both faster; AU31's measurement on
another boot takes it as it is now. The stage stays by AU1's rule: a
benchmark gains more than 5% (`strings` 13.1% to 16.0% in wall-clock)
while the self-check's estimate moves within its spread. (user)

**AU33. The owner's answers of 2026-10-10 after the study of the typed
calls: (i) `List.at` in place in the generated code of both tiers reads
the list's items through the layout of `Vec<Value>` probed when the VM
starts: the offsets of the pointer and of the length within the vector
are kept in the VM's native state and loaded by the code, which holds
no assumption about the layout, so that neither the code nor an image
depends on how the compiler laid the vector out, and a test holds the
probe; (ii) `Text.contains` and `List.contains` each get a lean helper
that the generated code calls directly with its operands' place and
that answers a Boolean, with no typed entry's dispatch, no loop over
the arguments and no memory check, the search staying in Rust; (iii)
the general path of a typed call becomes lean as a stage of its own
after this one: the generated code releases the arguments the call does
not borrow (in place, AU29 and AU32) and pushes the answer itself, and
the entry is called through a thin shim per kind of answer; (iv) AU18's
goal is met with AU30's binary, whose run part measured within 15% of
the image on two boots (+9.1% and +7.6% at the best); the ratio is
recorded in every entry from now on and is no longer a goal, the JIT
run measured by AT1's rule and wall-clock against the stage before.**
The study, on AU32's binary: the self-check's synchronous JIT run made
2.77 million typed calls, `List.at` 1.39 million of them (its own work
46 instructions a call, the path around it about 150: the dispatch of
`rt_call_typed` about 66, `typed_value_answer` about 83), `Text.contains`
0.66 million (108 and 66), `List.contains` 0.16 million (290 and 66),
the others 0.56 million; `List.at` in place would be 15 to 20
instructions. On the boot after AU32's, the run part measured +25.2%
for AU32's binary (its image 20% faster than AU30's, its JIT run 7%)
and +7.6% for AU30's. The alternatives the owner declined: for (i), the
offsets compiled into the code with the image's header recording them,
or the list in a vector of fixed layout; for (ii), small texts and a
one-byte needle in place, or the two left as they are; for (iii), the
protocol within this stage, or not before a new profile; for (iv), the
ratio kept as a goal, or a fixed reference (the run part within 15% of
AU30's image). (user)

**AU34. The stage of AU33 (i) and (ii) as built: `List.at` in place,
`Text.contains` and `List.contains` through lean helpers.
`layout::list_layout` finds, on a vector of known address, length and
capacity, where a list's block keeps the address of its items and their
count, counted from the start of the `Rc`'s allocation, and the VM keeps
both offsets in its native state (`list_items`, `list_len`); a build
whose vector is not three such words has no `List.at` in place. A typed
call carries the native it may do in place (`CallKind::Typed { kind,
in_place }`, `InPlace`), named by the prelude's functions
(`natives::prelude::in_place`), which no other extension declares. In
both tiers, `List.at` on a list and a small Integer loads the two
offsets, compares the index with the count unsigned, copies the item
with one more reference (the sequences of AU29 and AU32) or writes
`Nothing`, and releases the list after the copy unless the call borrows
it, since the release may free it (the template code, which has no
register to hold the item across the release, copies it over the
index's operand first); a guarded or big operand, or a failure among
the items, takes the general path with the operands untouched.
`rt_text_contains` and `rt_list_contains` take the operands' place and
the mask of those the call borrows, and answer 0 or 1 from the typed
entry with the others released, or `DECLINED` (2) with nothing touched,
when the general path follows. Code format 12.** After the stage the
self-check's synchronous JIT run makes 0.56 million calls through
`rt_call_typed` where it made 2.77 million, and 0.37 million through
`typed_value_answer` where 1.76; a lean helper's own work is 20
instructions a call where the dispatch was about 66. The tests: in
`value.rs`, `a_list_s_items_and_length_lie_where_the_probe_finds_them`
(lists of 0 to 40 items read through the offsets); in `tests/native.rs`
and `tests/template.rs`,
`list_items_read_in_place_and_contains_through_lean_helpers` (items of
every kind at every position and past both ends, lists held by slots
and lists made for the call, an index past the machine word on the
general path in a function of its own, against the interpreter; with
the item's retain taken out the test aborts on both tiers, with the
helpers' answer forced false it fails on both, as it was checked to);
the conformance suite and the judges on both tiers. The first test
program held the index past the machine word in `main`, whose frame the
Cranelift tier then handed to the interpreter for the whole loop, so
that its check passed without the retain; the index moved to a function
of its own. Measured on the boot after AU32's, against AU32's binary
(7353f11) measured again on it (its rows within 0.01% of the earlier
boot's). By AT1's estimate under cachegrind: the synchronous JIT run
9.76 to 9.30 billion cycles (-4.7%; instructions -4.2%), the template
code alone 9.67 to 9.16 (-5.3%), the image 7.53 to 7.03 (-6.7%;
instructions -5.8%), the interpreter level. The machine code of the
self-check's image +0.7%, its template code +0.6%. In wall-clock,
alternating with the order swapped every turn, best and median: the
self-check -6.4% and -0.3% on one hardware thread, -2.1% and -4.8% on
two, -5.2% and -6.4% on four; the four benchmarks, which call the three
natives little, within the noise (-2.7% to +4.8%). The run part of AU22
on one hardware thread: the JIT run 1,008.0 to 927.4 ms at the best
(-8.0%), the image 789.2 to 773.6 (-2.0%); the ratio, recorded as AU33
says, +19.9% at the best and +23.4% at the median. The stage stays by
AU1's rule. What the general path still carries: 0.56 million calls of
the self-check (`List.last` 0.18 million, `List.join` 0.16, `List.length`
0.07, the rest under 0.03 each), whose dispatch and value answers cost
about 63 million instructions, 0.9% of the run: the most the stage of
AU33 (iii) can take back. (user)

**AU35. The owner's answers of 2026-10-10 after the progress report:
(i) the typed round closes, its three goals met (`bench/strings.ry`
past CPython, the self-check's estimate on the JIT run 37% under AT7's
14.83 billion cycles where a quarter was asked, AU18's ratio met with
AU30's binary on two boots, AU33); the general typed path of AU33 (iii)
is not built, a candidate worth at most 0.9% of the self-check; the
front end's round opens with the study of AU31 (iv); (ii) the binary
keeps the Rust front end, the judges keep the compiler written in Renyi
equal to it, and from now on every entry that measures speed records
the ratio of the Renyi compiler's time to the Rust front end's on the
same file with the same binary (checking, parsing and compiling
`compiler/bodies.ry`); the switch is decided when the ratio is within
3×, the threshold the question proposed.** The report the answers
followed, measured on one boot, best of seven on four hardware threads:
the self-check (the Renyi checker on `compiler/bodies.ry`) 1,854 ms on
AU10's binary and 1,052 on AU34's (-43%), 1,415 and 834 from the image
(-41%), 2,095 and 1,632 on the interpreter (-22%); session 12 alone,
AU27's binary against AU34's, -13.0% on one hardware thread and -18.0%
on four at the best; the benchmarks against CPython on the JIT run,
`primes` 41 ms against 485 (11.8 times as fast), `records` 36 against
152 (4.3), `json_round_trip` 93 against 151 (1.6), `strings` 68 against
73 (1.1, where AU10's binary took 82, 0.9); the front ends on
`compiler/bodies.ry` (7,774 lines and what it imports): checking 77 ms
with the Rust front end, 820 with the Renyi checker from its image and
1,101 on the JIT run (10.6 to 14.2 times), parsing 78 and 348 (4.4),
compiling 103 and 1,132 (11.0), where session 10 had measured checking
at 0.083 s and 1.87 s (22.5 times). What `renyi build` is, as the report
stated it: ahead-of-time machine code for every function of a program
in an image that runs in the Renyi runtime, the executable of
`build --exe` that binary with the image appended, the image cache
that makes every later run of a program load its code, the front end
still run before a hit. The alternatives the owner declined: for (i),
the append's copies first, or the general path as AU33 ordered; for
(ii), an opt-in switch to an image of the Renyi compiler built into the
binary, the switch for `compile` and `build` only, or the switch for
every command now. (user)

**AU36. The owner's answers of 2026-10-10 after the front end's study:
(i) the Rust front end first, in the order A0 (every file parsed once:
the resolver's trees handed to the check, and `check` without a parse
of its own of the main file), A1 (the parser's token comparisons by
variant, inlined, and `advance` without the clone where the token is
dropped), A4 (the library's fixed cost: the prelude and the modules a
program imports declared, the item lists shared rather than cloned),
then A2 (a cache keyed by the sources, which skips the front end when
every file a compile read hashes as it did); (ii) then the compiler
written in Renyi and the VM under it: B5 (a comparison of two boxed
values the checker typed Integer, inline in both tiers), B1 (the lexer
rewritten for speed with the library as it is), B2 (the parser's
keywords and symbols compared by tag, no record built per look) and B3
(small functions inlined on the Cranelift tier), the library unchanged;
(iii) the gate of AU35 (ii) is measured against the Rust front end of
AU34's binary (commit 44782aa) on the same boot, a fixed reference,
with the ratio against the same binary recorded beside it; (iv) before
the compiler's sources change, a copy of today's compiler goes under
`bench/` as the program of AT1's measure, which stays fixed while
`compiler/` improves; the ratio of (iii) is taken on `compiler/`.** The
study the answers followed is the handoff's section "The front end's
round", on AU34's binary: `renyi check compiler/bodies.ry` (nine files,
13,280 lines) runs 437 million instructions, 314 million of them in the
parse, which parses every file twice and the main file three times
(the resolver for the imports, the check, and `diagnose`); the token
comparisons are calls (`TokenKind::eq`, 18% of the run); the library's
fifteen declaration files are parsed and declared at every start, half
of an empty program's check; the Renyi checker runs 4.99 billion
instructions on the same files, its lexer 56 times the Rust lexer's,
its parser 17 times, the rest 15 times, and a scan in one loop over the
same characters costs a sixth of the lexer as written. One operation of
each kind the compiler does, in instructions from an image: a call 96,
a character read from a list and compared 135, the lexer's `is_lower`
370, a record of three fields made and read 440, the parser's step on a
shared cursor 1,340, its test for a keyword 870, an append to a list
held once 370, where compiled Rust spends from none to a few. The
estimates: A0 and A1 together `check` -52% and the self-check's front
end -34%; A2 `hello` from 7.2 to about 5 ms; B1 and B2 the Renyi
checker from 850 to about 500 to 550 ms; the gate of (iii) about 215 ms
on that file. The owner chose, for (ii), the option "the same, plus
B3" after an option that added a text method answering code points;
the entry reads it as B5, B1 and B2 with B3 and without the method,
which B1's measure can bring back as a question. The alternatives the
owner declined: for (i), the cache first or the compiler in Renyi
first; for (ii), the three stages without B3, with the code point
method, or no change to the compiler in Renyi in this round; for (iii),
the same binary as AU35 (ii) states it, or no speed work on the Rust
front end before the switch; for (iv), a measure that follows the
compiler. (user)

**AU37. Every file parsed once by the Rust front end (stage A0 of
AU36).** The resolver (`renyi_package::resolve`) parses each file as the
check takes it (`parse_file`: a bound module's declarations, any other
file as a module) and keeps the trees beside the files
(`Resolved::trees`); the check declares the modules from them
(`check_parsed_project_in`; `check_project_in` parses and calls it);
`renyi check` parses the main file once, reports its errors alone as
before, and otherwise hands the tree to the resolver (`resolve_parsed`)
and checks what it returns (`check_resolved_in`); `compile_file`, which
every command that compiles calls, and the package commands' index check
the resolver's trees. The resolver reads a bound module's imports from
its declarations' parse, as the compiler in Renyi does (`project.ry`),
where it parsed the text as a module before: the imports are the same,
the mode changing the bodies only. The commands that check canonical
texts (`index`, `tools`) and the resident world keep their own parses.
The outputs are unchanged: the gates passed, the judges among them.
Measured on one boot against AU34's binary, callgrind for the
instructions and the best of nine alternating runs for the times:
`renyi check compiler/bodies.ry` 437.3 to 243.5 million instructions
(-44.3%), 72.1 to 47.3 ms (-34.5%); `check compiler/checker.ry` 561.0
to 371.3 million (-33.8%), 91.8 to 69.8 ms (-24.0%); `compile
compiler/checker.ry` 989.9 to 801.5 million (-19.0%), 155.2 to 132.6 ms
(-14.6%); the thirty examples checked one at a time 208.3 to 198.1 ms
summed (-4.9%); an empty program and `hello` within noise (-0.6% and
-1.5% in instructions: the library's fixed cost, which is A4's).
AT1's rows on the same boot (AU34's binary measured again first, equal
to its rows of the boot before): the synchronous JIT run 6,752.9 to
6,561.9 million instructions and 9,297.9 to 9,075.1 million estimated
cycles (-2.40%), the templates alone -2.40%, the interpreter 15,834.5 to
15,601.3 million (-1.47%), the image unchanged (7,025.9 and 7,026.1: it
runs no front end). The ratio of AU35 (ii), as AU36 (iii) records it on
`compiler/bodies.ry` (the Renyi compiler from images built by this
binary): checking 786.9 ms against AU34's Rust front end's 76.4 (10.3
times, the gate) and this binary's 48.9 (16.1 times); parsing 314.9
against 68.9 (4.6) and 62.2 (5.1; `parse --json` itself is unchanged,
282.4 million instructions on both binaries, the difference noise);
compiling 1,107.9 against 104.7 (10.6) and 89.1 (12.4). The stage stays
by AU1's rule.

**AU38. The Rust parser's token comparisons by variant (stage A1 of
AU36).** A token kind carries texts in three of its variants, so its
derived comparison is a function the compiler does not inline, and
`peek` called it at every look for a line break: `TokenKind::eq` was 18%
of `renyi check compiler/bodies.ry` on AU34's binary. The parser now
tests a kind with `matches!` wherever the kind compared is known (`peek`,
`peek_second`, `skip_newlines`, the line ends, the parameter test) and
through `TokenKind::is` where it is given (`at`, `eat`, `expect`): an
inlined comparison with the answer of `==` (a reserved word compares its
word, a kind that carries a text compares it all, any other kind its
variant alone); `at_word`, `eat_word` and `Token::is_word` match the word
in place; `advance` keeps its clone of the token for the callers that
use it, and the 74 that drop it call `bump`, which only moves the
cursor; `expect` and `expect_word` answer the span, all their callers
used, where they cloned the token; `identifier` and `type_name`, the most
frequent, read the kind and the span where they lie. The lexer's two
comparisons of a kind (the token pushed against `raw`, the dot before a
member) are `matches!` too. Measured on one boot against A0's binary
(AU37), callgrind and the best of nine alternating runs: `renyi check
compiler/bodies.ry` 242.9 to 186.8 million instructions (-23.1%), 47.0
to 40.5 ms (-13.8%); `check compiler/checker.ry` 371.3 to 286.7 million
(-22.8%), 66.8 to 60.0 ms (-10.1%); `compile compiler/checker.ry` 801.6
to 716.7 million (-10.6%), 129.7 to 126.4 ms (-2.5%); an empty program's
check 11.2 to 8.6 million (-23.7%: the library's declaration files are
parsed too), 6.9 to 6.5 ms; `run examples/hello.ry` 13.8 to 11.1
million (-19.4%), 6.7 to 6.4 ms; the thirty examples checked one at a
time 180.5 to 172.4 ms summed (-4.5%). AT1's rows: the synchronous JIT
run 6,561.9 to 6,478.6 million instructions and 9,075.1 to 8,890.5
million estimated cycles (-2.03%), the templates -0.96%, the interpreter
-0.56%, the image 7,026.1 to 7,044.5 (+0.26%, the same instructions: the
layout of the binary, which AU32 saw move a row by half a percent). Since
AU34's binary, `check compiler/bodies.ry` -57% in instructions. The
ratio as AU36 (iii) records it: checking 773.3 ms against AU34's Rust
front end's 69.9 (11.1 times, the gate) and this binary's 41.3 (18.7);
parsing 317.6 against 72.1 (4.4) and 59.5 (5.3); compiling 1,149.9
against 99.5 (11.6) and 84.2 (13.7). The stage stays by AU1's rule.

**AU39. The module trees shared, not copied, by the declarations (the
first part of stage A4 of AU36).** Three passes of the declarations
(`World::resolve_types`, `resolve_abilities`, `resolve_functions`) walk
a module's items while they fill the world's tables, and each copied
the module's whole item list first, bodies included, to free the world
for writing: the library's fifteen modules at every start and every
module of a project. A module's tree is now held by a count
(`ModuleInfo::ast: Rc<ast::Module>`), and each pass holds the tree by
another count while it walks it; every reader of the tree reads it as
before. Measured on one boot against A1's binary (AU38), callgrind and
the best of nine alternating runs: an empty program's check 8.56 to 6.73
million instructions (-21.3%), 6.2 to 5.7 ms; `run examples/hello.ry`
11.13 to 9.31 million (-16.4%), 6.5 to 5.9 ms; `renyi check
compiler/bodies.ry` 186.8 to 155.1 million (-16.9%), 46.8 to 38.4 ms
(-18.0%); `check compiler/checker.ry` 286.9 to 236.9 million (-17.4%),
63.4 to 52.3 ms (-17.5%); `compile compiler/checker.ry` 716.6 to 667.2
million (-6.9%), 132.3 to 116.5 ms (-11.9%); the thirty examples checked
one at a time 168.1 to 155.4 ms summed (-7.5%). AT1's rows: the
synchronous JIT run 6,478.6 to 6,429.4 million instructions and 8,890.5
to 8,770.9 million estimated cycles (-1.35%), the templates -1.31%, the
interpreter -0.49%, the image -0.34% (the same instructions). Since
AU34's binary, `check compiler/bodies.ry` 437.3 to 155.1 million
instructions (-64.5%). The ratio as AU36 (iii) records it: checking
775.2 ms against AU34's Rust front end's 72.3 (10.7 times, the gate) and
this binary's 35.0 (22.2); parsing 312.1 against 74.2 (4.2) and 58.5
(5.3); compiling 1,125.6 against 104.8 (10.7) and 81.0 (13.9). The stage
stays by AU1's rule. The second part of A4, the prelude and the modules
a program imports declared instead of the fifteen, was measured with a
binary built for the measure and not kept: an empty program's check a
further -38% in instructions (5.78 to 5.01 ms), `run examples/hello.ry`
-31% (6.51 to 5.55 ms), `check compiler/bodies.ry` -1.8%, and the
bytecode file of `hello` from 230.9 to 115.4 KB, since a program
carries the metadata of every function the world declares (200 of
`hello`'s 201 are the library's). It changes every program's bytecode,
which the compiler in Renyi must write the same (the judges), and the
checker's fixes that name a library module not imported (`write
\`import std.environment\``, `write \`import std.json exposing
JsonValue\``) then need the names of every library module's types and
abilities without declaring them: its question goes to the owner.

**AU40. The owner's answers of 2026-10-10 after the first part of A4:
(i) the second part of A4 is done: a program declares the prelude, the
library modules it imports and the library modules those import, not
the fifteen; a fix that names a library module the program does not
import (`write \`import std.environment\``, `write \`import std.json
exposing JsonValue\``) reads the whole library then, on that error's
path only, with no index kept beside the declaration files; the Rust
front end and the compiler in Renyi change in one commit, and the
bytecode of every program changes with them; (ii) A2's cache keyed by
the sources serves `run`, `record` and `test`: on a hit the image the
cache holds runs without the front end, the warnings the compile printed
are printed again, and `record`'s code hash is kept in the entry; (iii)
an entry holds the content hash of every file the compile read and the
places where a manifest was looked for and not found, and a hit needs
all of them as they were; (iv) the cache is on by default under the
image cache's switch (`--no-cache`, `RENYI_NO_CACHE`,
`RENYI_CACHE_DIR`).** The measures the questions gave: with a binary
built for the measure, the second part of A4 took a further 38% of an
empty program's check in instructions and `run examples/hello.ry` from
6.51 to 5.55 ms, and halved `hello`'s bytecode file (AU39); the front
end the cache skips is about a tenth of the self-check's JIT run. The
alternatives the owner declined: for (i), keeping the fifteen modules,
or deciding after A2; for (ii), the cache for `check` too, or for `run`
alone; for (iii), the sizes and the modification times first, the
hashes only when they changed; for (iv), off by default. (user)

**AU41. The self-check frozen under `bench/selfcheck/` (AU36 iv).** The
program of AT1's measure, the compiler written in Renyi checking its own
`bodies.ry`, is now a copy of the thirteen files `compiler/checker.ry`
reaches, as they stood at decision AU39 (commit f1ba01b), under
`bench/selfcheck/`; `tools/bench.py`, `tools/measure_size.sh` and
`tools/measure_native.sh` run `bench/selfcheck/checker.ry` on
`bench/selfcheck/bodies.ry`, and CI checks the copy and holds it in
canonical layout with the other benchmark programs. `compiler/` goes on
changing (the second part of A4 is the first change), the judges hold
it equal to the Rust front end, and the ratio of AU35 (ii) and AU36 (iii)
is taken on `compiler/`; the copy changes only when the language or the
library leaves it unable to check, and then as little as it needs. AU36
(iv) placed the copy before the compiler's sources change: the second
part of A4 changed them before the copy was made, and its first
measure, on `compiler/` itself, showed the self-check's instructions
+0.2% to +0.4% and its estimate +0.5% to +1.5% from the two fields the
change adds to the Renyi world; the copy was taken from the commit
before that change, and the stage is measured on it.

**AU42. A program declares the library modules it needs (the second
part of stage A4, AU40 i).** The checker declares the prelude, `std.json`,
every library module a file of the program imports and the library
modules those import, transitively, in the library's order, instead of
the fifteen (`Library::world_for`, with `Library::needed` for the
closure); the resident world of AN1 declares the same, and the compiler
written in Renyi the same (`project.ry`'s `library_world` and
`needed_library`). `std.json` is always among them: the runtime decodes
a recorded result and what a Python module returns through its decoder,
whatever the program imports, and builds its `JsonError` when a value
does not fit, which the conformance case of the Python bridge showed
when it was left out. A fix that names a library module the program does
not import reads the whole library on that error's path only: the
closest module to an unknown one among every module's name
(`World::module_names`), the import that would expose an unknown type or
ability (`World::library_module_declaring`, which parses the library
modules not declared), the import of a namespace used without it
(`World::library_module_named`); the world keeps the library and how many
of its modules it declared for them, and the compiler in Renyi keeps the
library's trees. Every program's bytecode changes with the modules
declared (`examples/hello.ry`'s file from 230.9 to 124.8 KB), both
compilers alike: the gates passed, the judges among them. Measured on one
boot against AU39's binary, callgrind and the best of nine alternating
runs: an empty program's check 6.73 to 4.36 million instructions
(-35.3%), 5.5 to 4.7 ms (-14.2%); `check examples/hello.ry` -27.2%, 6.3 to
5.7 ms; `run examples/hello.ry` 9.31 to 6.69 million (-28.2%), 6.9 to 6.5
ms; `check compiler/bodies.ry` -1.6%, 35.8 to 34.5 ms; `compile
compiler/checker.ry` -0.8%, 121.6 to 117.9 ms; the thirty examples
checked one at a time 179.4 to 159.5 ms summed (-11.1%). AT1's rows on
the frozen self-check of AU41, both binaries on the same boot: the
instructions fall on every row (the synchronous JIT run 6,427.4 to 6,423.4
million, the image 4,994.1 to 4,989.3), the estimate rises on every row,
by 1.25% on the synchronous JIT run (8,768.1 to 8,877.5 million), 0.24%
on the templates, 0.11% on the interpreter and 0.21% on the image, the
whole of it simulated misses of the instruction cache and the branch
predictor (the synchronous row's I1 misses 105.2 to 115.0 million) from
the binary's layout, since the code the self-check runs did not change;
the times on the same boot, alternating runs, fall instead: the
self-check -3.5% at the best on one hardware thread and -1.9% on four,
`records` -5.6% and -6.2%, `strings` -6.9% and -7.3%, `primes` -4.5% and
-4.8%, `json_round_trip` -2.7% and -2.4%, each benchmark's front end
lighter. The stage stays: the letter of AU1's rule asks the estimate to
fall or a benchmark to gain 5% with the estimate losing at most 1%, and
the estimate lost 1.25% to the layout while two benchmarks gained more
than 5% and every time fell; the entry records the departure. The ratio
as AU36 (iii) records it: checking 777.9 ms against AU34's Rust front
end's 72.7 (10.7 times, the gate) and this binary's 35.9 (21.7); parsing
313.2 against 66.7 (4.7) and 56.1 (5.6); compiling 1,073.8 against 100.1
(10.7) and 75.7 (14.2).

**AU43. The owner's answer of 2026-10-10 on what a hit of A2's cache
loads: every run compiled from its sources leaves the compiled program
in the cache, keyed by the sources, and a hit loads that program in
place of the front end; a program whose run compiled machine code
leaves its image as well (AU10, unchanged), which the hit then finds
by the program as before.** The question followed from AU10's rule: an
image is left only by a run that compiled machine code, so a short
program such as `examples/hello.ry` has none, and a cache whose hits
load images alone (AU40 ii as it stood) would have spared it nothing,
where the front end is most of its time. The program is kept in the
binary encoding of AT3, about half its bytecode file. The alternatives
the owner declined: an image built in the background for every program
run, or hits for the programs with an image only. (user)

**AU44. Stage A2 of the front end's round as built: the cache keyed by
the sources (AU40 ii to iv, AU43).** Every `run`, `record` or `test` of
a program compiled from its sources, and the `build --cache` a run
leaves in the background, goes through one path, `compile_cached` in
`crates/renyi/src/lib.rs`: the entry, a `.rys` file in the cache's
directory beside the images, is named by a hash of the binary (its
version and its executable's path, size and modification time, so that a
build with the same version and another front end misses, with the names
and sizes of the library's declaration files, the extensions' among
them), the working directory and the path as given, since a compiled
program names its files as they are given; it holds a format line, one
line of JSON (every file the compile read with the content hash of its
text, or `null` for one looked for and not there, the manifests looked
for in the directories above the program and the `.ry` looked for before
a `.renyi` among them; the warnings the compile printed; the manifest's
code hash and dependencies when the run wanted a manifest, else `null`)
and then the program in the binary encoding of AT3. A hit is an entry
whose files all read as they did (`renyi_package::unchanged`: each read
again and hashed) and which holds a manifest when one is wanted: the
warnings are printed again, the program is decoded, no front end runs,
and the image cache's entry is named from the encoding held (AU10,
unchanged, which a run that compiled machine code still fills). A miss
compiles inside `renyi_package::noting`, every file read through
`read_text` noted (the main file's read too), computes the code hash and
the dependencies when a manifest is wanted, writes the entry to a
`.part` file and renames it, and goes on; a compile with errors stores
nothing. The entries of both caches count against the 256 MB together,
the least recently used going first; `--no-cache` and `RENYI_NO_CACHE`
turn both off; `RENYI_CACHE_REPORT=1` prints a line on a hit and on a
store, for the tests. `serve`, `reproduce`, `check`, `compile`, `build`
to a file and the MCP server keep the front end. The first build hashed
the text of the library's declaration files for the key at every run:
1.7 million instructions on this machine, which has no SHA extensions,
more than the front end a hit of `hello` skips (the hit 6.33 million
instructions against 6.72 with the cache off); the executable's identity
covers the library compiled into it, so the key takes the modules' names
and sizes, and the hit fell to 4.53 million (-32.7% against the cache
off; a miss 7.91 million, +17.6%). Measured on one boot against the
binary before the stage (d27fd5a), callgrind for the instructions and
the best of seven alternating runs for the times: `run
examples/hello.ry` 7.4 ms with the cache off on the binary before and
7.6 on this one, 5.8 on a hit (-24%; 6.73 to 4.53 million instructions)
and 7.6 on a miss (7.92 million instructions, the medians of seven 8.5
against 9.7 ms), where the study had estimated about 5 ms for a hit; the
self-check (`bench/selfcheck/checker.ry` on its `bodies.ry`): the cold
JIT run 1,041 ms on the binary before and 1,006 on this one (the code it
runs is unchanged), the second run as a user sees it 822 ms before (the
front end, then the image from the image cache) and 762 now (both caches
hit, -7.3%), 823 and 731 on one hardware thread (-11.2%), the medians of
three 903 to 783 and 849 to 780; the cost of a miss, the files read
again and hashed (600 KB), the program encoded (761 KB) and the entry
written, 1,064 ms against the cold run's 1,006 for the self-check and
nothing measurable for `hello` (the instructions +17.7%); the image's
build in the background after the miss is as before. AT1's rows on the
self-check, cachegrind on both binaries: the rows that run with the
cache off are unmoved, the interpreter 11,613.9 to 11,613.4 million
instructions and its estimate +0.02%, the image 4,992.4 million on both
and its estimate -0.19%, the synchronous JIT run 6,460.0 to 6,460.5
million instructions (+0.01%) with its estimate 9,097.0 to 9,030.3
million (-0.73%, the layout: the simulated misses of the instruction
cache fell), the default JIT row within the compile thread's variation
(+1.0%); the `cached run` row, the second run of a program as a user
sees it, which now hits both caches, 5,440.7 to 5,071.2 million
instructions (-6.8%) and its estimate 7,782.6 to 7,251.4 million
(-6.8%): what it pays above the row that runs the image given on the
command line, the front end and the encoding hashed before, 448.2
million instructions, is now 78.8 million, the files read again and
hashed (sha256 in software here: about 55 instructions a byte over the
self-check's 600 KB), the encoding hashed for the image cache's name and
the program decoded. The ratio of AU36 (iii) is measured with the cache
off and does not move with this stage, the Rust front end being AU42's:
on this boot, nine alternating runs on a quiet machine, checking 795.9
ms against the Rust front end's 38.3 on this binary (20.8 times) and
38.7 on the binary before the stage (20.5), against AU34's 72.7 the 10.7
of AU42; parsing 316.5 against 61.3 (5.2) and 59.2 (5.3); compiling
1,039.2 against 81.2 (12.8) and 74.1 (14.0).

**AU45. Stage B5 of the front end's round as built (AU36, stage 6): a
comparison of two Integers of which at least one is boxed, inline in
both tiers.** Before a comparison on boxed operands calls `rt_compare`
(AU13), the generated code of both tiers now tests, when both operands
are Integers by the checker's type (`is_integer` in `native/infer.rs`,
beside `is_text`, from the type the bytecode notes for what an op
pushes, AU1 stage iv) or by the analysis (a small Integer in a register,
`Abs::Int`, as a literal bound is), that the boxed ones are small (the
value's tag and the Int's tag), compares the payloads with the op's
condition (`is`, `is not`, `is less than`, `is at most`, `is greater
than`, `is at least`), a register operand read from its register, and
takes the Boolean into its register on the Cranelift tier
(`compare_small_integers` in `native/codegen.rs`, after
`compare_small_texts`) or writes it over the left operand on the
template tier (`template/mod.rs`, where every value lies boxed in its
slot and the tags are tested alike); the boxed operands are dropped
without a release, since a small Integer holds no count; a big Integer
on either side, or a value the checker typed otherwise (`Number`, a
`maybe`), falls through to the helper as before. Code format 13. The
case it serves: an Integer read from a list, a field, a `maybe` or an
`otherwise`, which the abstract interpretation cannot hold in a
register, compared with another such or with a literal, as a lexer over
code points compares at every character; `bench/micro/ scan_codes.ry` is
that scan, a lexer's classification of forty codes read from a list
against literal bounds, measured per turn by `tools/measure_native.sh`.
Held to the interpreter by a test on each tier (every op, Integers from
a list, a field and a `maybe`, against each other and against literals,
two big Integers among the values), the judges and the conformance suite
on the template tier too. The first build took only two boxed operands
and left a boxed Integer beside a literal, the common shape, to the
helper: the scan's instructions per turn did not move; the build
measured here takes both shapes. Measured on one boot against AU44's
binary, cachegrind and callgrind for the instructions, the best of five
alternating runs for the times: `scan_codes` 33,793 instructions a turn
on the Cranelift tier and 33,384 on the template tier before, 17,525 and
16,877 now (-48% and -49%; the interpreter's 65,489 unchanged); AT1's
rows on the self-check: the synchronous JIT run 6,460.5 to 6,424.9
million instructions (-0.55%) and its estimate 9,030.3 to 8,961.2
million (-0.77%), the image 4,992.4 to 4,984.6 million (-0.16%; its
estimate +0.19%, the layout), the interpreter unchanged (11,613.4 to
11,613.1 million), the `cached run` row 5,071.2 to 5,063.3 million
(-0.16%); the self-check's `rt_compare` 152.2 to 123.3 million
instructions inclusive (2.36% to 1.92% of the synchronous run; what
remains compares texts not held in the value, records, variants and
Integers the checker did not type); the four benchmarks, synchronous,
`primes` 294.5 to 290.3 million instructions (-1.4%) and the other three
unchanged, none of them comparing a boxed Integer in its loop; in
wall-clock, `scan_codes` 54.7 to 24.0 ms on four hardware threads and
57.7 to 26.1 on one (-56% and -55%), the self-check 976.7 to 954.8 ms on
four (-2.2%) and 1,016.6 to 952.2 on one (-6.3%), `primes` 40 to 36 ms,
the other benchmarks within the run-to-run noise of their 5 ms. The
default JIT row and the benchmarks counted with the compile thread on
vary with its timing under cachegrind (`records` read 307 to 357 million
that way and 241.2 million on both binaries synchronously): the
synchronous counts are the ones to compare, as item 2 of the handoff's
"Start here" says. The ratio of AU36 (iii), with AU34's binary built
again from 44782aa on this boot: checking 755.9 ms against AU34's Rust
front end's 75.8 (10.0 times, the gate; 10.7 at AU42) and this binary's
38.2 (19.8); parsing 313.0 against 58.7 (5.3) and 63.3 (4.9); compiling
1,035.2 against 99.0 (10.5) and 80.4 (12.9); the Renyi checker's time
fell from 806 ms at AU44 to 756, the stage's gain on the program the
round is for.

**AU46. Stage B1 of the front end's round as built (AU36, stage 6): the
Renyi lexer rewritten for speed with the library as it is.**
`compiler/lexer.ry` keeps its tokens, its diagnostics (codes, messages,
fixes, spans and recovery, AD1) and its public functions, and the judges
hold it byte-equal to the Rust lexer as before; what changed is how it
runs. `lex_range` is one loop over the characters that skips a blank and
takes a line break itself and calls `next_step` for the rest, with the
previous token kept in a binding rather than read from the list at every
step; a step's product is a sum, `Single(token)` on the common path,
else `Several(tokens, diagnostics, next)`, so that a token costs its own
record and one variant, not a record, a list and a copy; the scans are
loops of their own without the closure and the call per character that
`scan_while` and `char_at` made (`scan_word`, `scan_name_tail`,
`scan_alphanumeric`, `scan_digits`, `scan_spaces`, `scan_operator_tail`,
`plain_run_end`), each reading `chars.at(position) otherwise ""` in
place (AU34) and classifying by `contains` on the three short texts of
the character classes, the likeliest class tested first; the reserved
words, the phrases, the phrase starters and the symbols are `Set`s built
once from their lists (`to_set`, a constant evaluated on first use); a
word's text is sliced once and its token built from it; a comparison
with a variant (`token.kind is Symbol(text: ".")`, which built the
variant) is a match on the kind, the position compared first; a plain
run of a text literal is copied by one slice, not a concatenation per
character. Three ways to classify a character were measured first, per
character on the JIT tier: a search in a short text 341 instructions, a
`Set` lookup 649, two ordering comparisons 305; the search stays, as
AU36 said, for its reading. `compiler/lex_only.ry`, a driver that lexes
a file as many times as asked and prints a count, measures the lexer
alone (the token dump's printing is 40% of `tokens.ry`). Measured on
AU45's binary, callgrind synchronous, per lex of `compiler/bodies.ry` in
the steady state (three runs against one): 982.9 to 514.5 million
instructions (-47.7%), in four cuts of 634.5 (the scans and the sets),
576.7 (the step as a sum, the previous token kept), 542.1 (the word's
token built once, the dot's position first) and 514.5 (the plain runs
sliced); `renyi run compiler/tokens.ry compiler/bodies.ry` 2,197.8 to
1,740.4 million instructions (-20.8%, the dump's printing unchanged in
it) and 388.1 to 335.3 ms; the lexer alone 112.4 to 64.8 ms a lex in the
steady state (-42%). The ratio of AU36 (iii), the Rust side unchanged,
seven alternating runs on a quiet machine: checking 649.2 ms against
AU34's Rust front end's 73.3 (8.9 times, the gate, from 10.0 at AU45)
and this binary's 36.5 (17.8); parsing 258.0 against 62.0 (4.2, from
5.3) and 58.1 (4.4); compiling 953.0 against 105.3 (9.1, from 10.5) and
78.6 (12.1); the Renyi checker on `compiler/bodies.ry` 756 to 649 ms,
since every front end's command runs the lexer first. The judges and the
conformance suite passed; the frozen self-check of AU41 does not move.
What remains in the lexer by the profiler: the scan of a word (28%, the
`contains` per character its floor with the library as it is: a method
answering code points is the lever B5 was the condition for, a library
addition for the owner to decide), the loop itself (21%), the word's
lookups (9%).

**AU47. Stage B2 of the front end's round, the first cuts: the Renyi
parser's keyword and symbol tests and its token reads.** `is_word` and
`is_symbol` in `compiler/parser.ry`, the funnels of every keyword and
symbol test, built a variant to compare the token's kind with
(`token.kind is Word(spelling: spelling)`); they match the kind and
compare the texts; `token_at` answers `cursor.tokens.at(index) otherwise
end_token(cursor)` in place of a match with a crash arm, and `peek`
reads the token under the cursor itself in place of two nested calls.
The judges hold the parser byte-equal as before. `compiler/parse_only.ry
<file> [turns]`, the driver of the measure (the file lexed once, parsed
`turns` times), gives the parser alone as the difference between three
turns and one under callgrind: 1,197.1 to 1,113.7 million instructions
per parse of `compiler/bodies.ry` after the first cut and 1,039.2 after
the second (-13.2%); 118.7 ms a parse in the steady state after them;
the ratio of AU36 (iii), seven alternating runs, the Rust side
unchanged: checking 630.3 ms against AU34's Rust front end's 73.3 (8.6
times, the gate, from 8.9 after B1 and 10.0 at AU45) and this binary's
38.8 (16.3); parsing 263.7 against 62.1 (4.2) and 65.9 (4.0); compiling
909.8 against 97.8 (9.3) and 77.9 (11.7). What the profiler leaves, and
the rest of B2 as AU36 set it: `peek` 16% (a `Peek` record per look, 54
call sites), `binary_chain` 6%, `token_at` and `after_breaks` 9%
together, `is_symbol` and `is_word` 7%, the cursor copied by `with
position:` at 34 sites: the cursor that builds no `Peek` per look and
the keywords and symbols as variants without fields compared by tag
(AU36), which the handoff's plan for B2 lays out, are the stage's
remaining cuts.

**AU48. B2, the rest: a look as an index with no record per look, and
the expression levels as one loop.** `compiler/parser.ry` looked at the
next significant token through `peek`, which built a `Peek` record (the
token and the cursor after the look) per look at 54 sites, and every
consumer read both back; the expression parser went through eight levels
(`or`, `and`, `not`, a comparison, `with`, the additive, the
multiplicative and `power`), each a function that peeked once, tested
its own operators and built a `Parsed` record on the way back, so that
one operand cost eight looks and eight records, and a statement's end
eight evaluations of the line-break rule. Two cuts. (i)
`peek_at(cursor)` answers the index of the next significant token (the
position, or past the line breaks the layout rules skip), `token_at`
reads the token at it and `moved(cursor, look)` is the cursor there (the
cursor itself when the look did not move, else `with position:`), so a
look builds nothing; the sites that consume what they looked at advance
with `cursor with position: look + 1`, in place when the cursor is held
once; `second_token` takes the look's index; `bump`, `past_word` and
`past_symbol` are `advance`, `expect_word` and `expect_symbol` for the
callers that drop the token (13, 32 and 11 sites), and the three keep
their `Peek` for the callers that keep the token's span, `expect_word`
and `expect_symbol` reading it back as `previous(after)`; `expression`'s
`otherwise`, `not`, `with` and `power` test their word inline; `peek`
stays at the 11 sites of rare paths (a hole, the module header, an item,
a loop source, a match fallback). (ii) `binary(cursor, lowest)` parses
an expression of the operators from the level `lowest` up in one loop:
the head is a `not` with its operand (when `lowest` allows it) or a
postfix expression; then while the next token is a binary operator whose
level lies between `lowest` and a ceiling, it is consumed and its right
operand parsed from the next level (from the same for `power`, which
associates to the right); a comparison lowers the ceiling below itself
(one comparison per operand, as the levels had it), `with` takes its
update list and lowers the ceiling below itself, and a `not` head lowers
it below the comparison; `binary_operator` maps a token to its
`BinaryOp` in one match, `level_of` gives an operator's level;
`or_expression` and `additive` are the entries at their levels. The same
tree, byte for byte: the quick judge on the 117 programs of the corpus,
the conformance suite, `compiler/`, `bench/`, the starter pack and the
library's declarations, the four judges of
`crates/renyi/tests/selfhost.rs`, and 24 probes of the levels' edges (a
`not` before a comparison, `with` on either side of a comparison,
chained `power`, a second `with` or a second comparison refused,
operators without operands, a `(` left open) through both parsers. The
parser alone (`compiler/parse_only.ry`, the difference between three
turns and one under callgrind with `RENYI_NATIVE_SYNC=1`, AU47's
measure; the compile thread's share varies from run to run under
valgrind, so the default mode does not measure): 1,039.2 to 998.6
million instructions per parse of `compiler/bodies.ry` after (i) (-3.9%:
a look's record was a smaller part of its cost than its calls) and 809.6
after (ii) (-18.9%; -22.1% in all, against the 800 the stage estimated);
96 ms a parse in the steady state (twenty turns less ten, by
wall-clock), from 118.7 at AU47. The ratio of AU36 (iii), seven
alternating runs, this binary AU45's (the VM unchanged): checking 578.6
ms (from 630.3) against AU34's Rust front end's 90.2 on this run (6.4
times; 7.9 against the 73.3 the reference took at AU47's run) and this
binary's 40.6 (14.3); parsing 231.7 (from 263.7) against 62.7 (3.7) and
60.7 (3.8); compiling 873.0 (from 909.8) against 122.1 (7.2) and 83.3
(10.5). What callgrind attributes on the three-turn run beside the
generated code: the frees of values 11%, `rt_compare` 6.6% inclusive
(the keyword and symbol tests among them, and the lexer's one-character
comparisons), `leave_frame` 5.5%, `rt_construct` 4.5%, the `with` copies
3.5%, `rt_is_variant` 2.2% (a helper call per `match` arm on a variant,
the subject cloned for it and dropped after), the reads of module
constants 0.5%. The keywords and symbols as variants without fields, the
cut AU36 named, would turn the keyword tests' text comparisons into tag
tests: bounded by `rt_compare`'s share, it waits for the measure after
the VM's own cuts. The variant test in place in both tiers, which every
`match` of every program pays a helper call for, is the next cut.

**AU49. A variant test in place on both tiers.** `Op::IsVariant(tag)`,
the test every `match` arm on a variant pattern runs (both emitters put
`Load(slot)`, `IsVariant(tag)` and `JumpIfFalse` before an arm's body:
`compile/pattern.rs` and `compiler/emit.ry`), was a helper call on both
native tiers: `rt_is_variant` popped the value, unwrapped a guarded one,
compared the tag and wrote the answer for the generated code to read
back, so an arm cost the clone of the subject, the call and the drop
inside it; the parser's three-turn run spent 2.2% in the helper (AU48),
and the checker matches on the tree and the types everywhere. Both tiers
now test in place: the value's tag at its address; for `TAG_VARIANT`,
its block's tag at `RECORD_TAG` through the `Rc` at `PAYLOAD` (the
layout of AU27), compared with the op's; the value released where it
lies (AU29's sequence on the template tier, AU32's on the Cranelift
tier) and the answer a Boolean, in a register on the Cranelift tier and
boxed where the value lay on the template tier; a typed top (an Integer,
a Boolean or a Float in a register) answers false without a read; a
`Guarded` value (tag 19, `TAG_GUARDED` in `value::layout`, the wrapper
of decision P3 that `plain()` unwraps) takes the helper, the Cranelift
tier's cold block with the stack's length stored for the pop and the
template tier's out-of-line sequence (`Cold::Guarded`). Code format 14.
Tests: one per tier matches fieldless variants, variants with fields and
a guarded JSON document (read through a capability with `only to` and
parsed by `json.parse`, whose result carries the origins) in loops,
against the interpreter; the layout test pins `TAG_VARIANT` and
`TAG_GUARDED`; the conformance suite and the four judges pass on the
template tier too. Measured by AT1's rule: the self-check's synchronous
JIT run 6,424.9 to 6,370.0 million instructions (-0.9%), 8,961.2 to
8,918.3 million cycles by the estimate (-0.5%); the image's run 4,984.6
to 4,927.3 million instructions (-1.1%), 7,185.8 to 7,088.5 by the
estimate (-1.4%), the cached run the same (-1.1%, -1.3%); the
interpreter unchanged; the image 8,678,512 to 8,824,368 bytes (+1.7%,
166.7 to 169.9 bytes of body per op: the sequence is longer than a
call); the micro benchmarks of AR1 unchanged, none of them matching a
variant; the parser alone 809.6 to 798.0 million instructions per parse
of `compiler/bodies.ry` (-1.4%). The ratio of AU36 (iii): checking 546.0
ms (578.6 on AU48's run, a noisier one: the reference binary took 90.2
ms then and 72.7 now) against AU34's Rust front end's 72.7 (7.5 times,
the gate) and this binary's 36.4 (15.0); parsing 226.3 against 59.9
(3.8) and 62.2 (3.6); compiling 849.6 against 102.4 (8.3) and 77.0
(11.0). The measure's lesson, now in the scripts: the compile thread's
share varies from run to run under valgrind, so `tools/measure_size.sh`
prints a `JIT run, sync` row beside the JIT run, the row two binaries
compare on, and `tools/measure_native.sh` exports `RENYI_NATIVE_SYNC=1`
(the micro benchmarks of two binaries differed by up to 15% without it
and are equal with it). What stays: the clone of the subject before the
test and its release after; a test that borrows the slot (`TestVariant {
slot, tag }`, the answer alone pushed) would spare both at the price of
a new op through both emitters, the bytecode file, the binary encoding,
the interpreter and both tiers, held equal by Z3's judge; a candidate
for the measure after B3, not a stage of its own.

**AU50. Small callees expanded into their callers on the Cranelift tier:
built, measured, off.** B3, the last stage of AU36's list, in the shape
the owner chose on 2026-10-11 (four questions, every recommended answer
taken): (1) a rewrite of the bytecode the Cranelift tier sees, not of
the program, so that the analysis, the translation of every op, the
interpreter, the template tier, the bytecode file and Z3's judge stay as
they are; (2) a hand-back to the interpreter from inside an expanded
callee rebuilds the frames the interpreter would have had, the caller's
stopped after the call and one per expanded callee around the point; (3)
the callees admitted: at most 20 ops, no loop, no `needs`, not
recursive, a handler, a crash and nested calls allowed (a callee's own
small callees once more), a caller growing by at most half its size,
measured by AT1's rule; (4) the gate of AU35 (ii) reconsidered after the
stage. As built (`native/inline.rs`, commit fb0cc2d): before
`codegen::compile` sees a code object, `expand` scans its ops for a
`Call` to a declared function with code that fits (`CodeKind::Function`,
as many parameters as arguments, at most `CALLEE_LIMIT` ops, no
capability since its frame would narrow the grant, no backward jump,
none of `IterInit`, `IterNext`, `ListPush`, `GroupInsert`, `GroupFold`,
`SortByKey`, `Deadline`, `CheckDeadline`, `MarkStack`, `UnwindStack`,
`Check` and `Fail`, the last because `rt_fail` leaves the frame, which
would be the caller's; not the caller nor a callee the expansion lies
in; an analysis that settles with every reachable `Return` leaving
exactly its result and no handled region of the callee's own open) and
replaces it by a `Jump` into a region appended after the caller's own
ops: the stores of the arguments into the region's slots (fresh slots
past the caller's locals and every earlier region's), the callee's ops
with their slots, constants (appended to the caller's pool) and jump
targets moved, every `Return` a jump to the region's exit, which
releases the region's slots as leaving the callee's frame would
(`LoadMove`, `Pop` per slot) and jumps to the op after the call with the
result on top, a reachable `ReturnNothing` a jump to a tail that pushes
`Nothing` first; a caller's candidates are admitted smallest first while
the budget lasts (the ops a region takes: the arguments, the callee's
ops, two per local, one or three for the tails), then the calls inside
the new regions once more (`NESTING_LIMIT` 2). The caller's own ops keep
their pcs, so a loop header, the template tier's promotion and the deopt
points of the caller's ops are as before; the generated code's prologue
stores the expanded height, since the frame the caller pushed has the
code object's own locals and the first helper that pushes would
otherwise push into a region's slot; an entry at a loop header moves the
operands up past the regions' slots and writes `Nothing` into them
(`make_room_for_expansion`), and the frame leave releases the code
object's own locals alone, the regions' holding `Nothing` at a return
(commit 2cc361c: counted as holders, they took every caller with a
region over the inline limit of four releases and through `rt_truncate`,
whose drops cost the parser's steady state 1.5%). Around it: the JIT
keeps the regions per code object and a constant pool per expanded one
(`Jit::pools`, the caller's constants followed by every expanded
callee's, which `NativeState::constants` points at and `Vm::constant_at`
reads for the helpers); the image carries the regions (image format 8;
`tools/image_census.py` reads it); a crash inside a region is located at
the callee's line (`inline::location`, `Jit::inlined_location`, from
`Vm::abandon`); a hand-back at a pc past the caller's ops
(`rebuild_inlined_frames` in `runtime.rs`) takes the expanded frame
apart into the caller's frame (its locals, then its operands below the
call, its marks recomputed) stopped after the call, or at the call when
the point lies in a region's prologue (the arguments already stored go
back onto the operands and the interpreter makes the call), and a frame
per expanded callee around the point, with its slots, the operand depth
its own analysis gives at that op, the handled regions open at the point
under the frames that own them and the frames' floors; a direct call
site that gets the hand-back runs the callee's frame to its end under
the frames built above it (`Jit::rebuilt_frames`, read by
`rt_direct_after`); a hand-back at a caller's own op drops the regions'
slots first. `RENYI_NATIVE_INLINE=1` turns the expansion on,
`RENYI_NATIVE_INLINE_BUDGET` sets the ops a caller may gain and
`RENYI_NATIVE_INLINE_LIMIT` the largest callee (for the measure of
another rule, and for small programs whose callers would expand nothing
under the half rule, which the tests set), `RENYI_NATIVE_INLINE_DUMP`
prints every expansion, why a call was not expanded, the regions' slot
kinds and every rebuilt frame, `RENYI_NATIVE_IR=<name>` a code object's
Cranelift IR. Tests (`tests/inline.rs`, against the interpreter): the
doors of an expanded callee (a return, a nothing, a failure caught by
the caller, one caught by the callee's own handler, a crash located at
the callee's line, nested one level, in loops) and the hand-backs (an
overflow inside the callee's body, nested one level, the loop's mark and
the caller's state rebuilt; a big Integer stored into an Integer
parameter before the body began, the call then made by the interpreter);
the whole suite, the conformance suite on both tiers and the four judges
pass with the expansion on. Two bugs the probes found, for the record:
the stack's length at entry (the first helper pushed into a region's
slot and a moved argument read back as a stale value), and the direct
call's entry frame after a hand-back that built frames (the innermost
frame's result was taken as the callee's). What the rule admits on the
compiler written in Renyi (the dump): the parser's three-turn run
expands 24 callees into 10 of its 34 compiled code objects, the
checker's run 12 into 8 of 47; the budget refuses 27 calls (`token_at`
seven times, `is_symbol` three, `moved` two), the size limit 32
(`peek_at` itself is 56 ops, `bump` 22, `second_token` 28, `level_of`
53), a loop 7, the recursion 2, a `fail` 1 (`past_word`, `past_symbol`);
`peek_at`, 56 ops with a budget of 28, takes `end_token` (15) and
refuses its four `token_at` sites (19 each). Measured by AT1's rule on
this binary, the expansion on against off, since the binary with it off
is AU49's to the fourth digit on the image and the cached rows (the
synchronous JIT run differs by the layout alone: 6,390.6 against 6,369.7
million instructions, +0.3%, and 9,040.6 against 8,918.3 by the
estimate, +1.4%, all of it simulated misses of the instruction cache, as
AU42 saw): the synchronous JIT run 6,390.6 to 6,430.5 million
instructions (+0.6%), 9,040.6 to 9,033.8 by the estimate (-0.1%); the
image's run 4,926.2 to 4,960.9 (+0.7%), 7,088.2 to 7,180.1 by the
estimate (+1.3%); the cached run 5,008.0 to 5,039.1 (+0.6%), 7,174.4 to
7,260.9 (+1.2%); the interpreter unchanged; the image 8,824,368 to
9,786,304 bytes (+10.9%, 169.9 to 190.0 bytes of body per op, 771 to
1,049 deopt points). The rules loosened for the question's sake (callees
up to 60 ops, a budget of 1,000 ops a caller): the synchronous JIT run
6,973.8 (+9.1%), 9,817.7 by the estimate (+8.6%); the image's run
5,228.2 (+6.1%), 7,852.3 (+10.8%, the misses of the instruction cache
110.6 to 144.1 million); the image 19,516,672 bytes (+121%, 395.7 bytes
per op). The parser alone (three turns less one of
`compiler/parse_only.ry` on `compiler/bodies.ry` under callgrind,
`RENYI_NATIVE_SYNC=1`): 799.6 million instructions per parse off (798.0
at AU49), 845.6 on (+5.8%; 853.6 before 2cc361c), 1,344.7 loosened
(+68%); the difference is the compile: the three-turn run's
`codegen::compile` 606 to 705 million inclusive, 50 of the 54 million a
parse, the same 34 code objects and 1,731 ops giving 15% more Cranelift
instructions (67,446 to 77,702 in 7,705 to 8,799 blocks) and 65% more
compile time (181 to 298 ms), the register allocation being superlinear
in a function's size. The steady state (every code object compiled at
its first call, `RENYI_NATIVE_HOT=0`, three turns less one): 572.6
million per parse off, 574.5 on (+0.3%; 583.2 before 2cc361c), 635.6
loosened (+11.0%). The micro benchmarks of AR1 (instructions a turn): a
typed Integer call 595 to 562 (-33), a call with a record argument 727
to 626 (-101), a field read and a record built unchanged: an expanded
call does save, but AU21's direct call was already lean, and what a
region adds (a retain at the caller's load and a release at the exit per
boxed argument where the typed call borrows, the stores and the tails)
takes most of it back, while the hot sites stay calls under the budget
and the bigger functions miss the instruction cache more. The ratio of
AU36 (iii), `tools/ratio.py target/renyi-au50 target/renyi-au34`:
checking 577.1 ms against the reference's 68.7 (8.4 times, the gate) and
this binary's 36.1 (16.0), parsing 215.9 against 60.8 (3.6) and 58.7
(3.7), compiling 882.4 against 106.3 (8.3) and 80.2 (11.0); the
expansion off checking 583.0 against 78.8 (7.4) and 38.5 (15.2), parsing
213.2 against 66.9 (3.2) and 64.5 (3.3), compiling 821.1 against 101.0
(8.1) and 79.4 (10.3), on a machine loaded by the site's build, where
the reference binary itself took 68.7, 78.8 and 75.4 ms in the three
tables, so the tables compare within themselves and the expansion's cost
is the Renyi checker's 577 against 583 on a reference 13% apart;
loosened checking 679.1 against 75.4 (9.0) and 37.4 (18.1), parsing
250.4 against 62.3 (4.0) and 59.0 (4.2), compiling 988.0 against 108.9
(9.1) and 78.1 (12.6). The rule of AU1 decides: the cold run does not
fall and no benchmark of the four gains, so the expansion is off unless
`RENYI_NATIVE_INLINE=1`; the mechanism, the tests and the hand-back stay
for the owner's evaluation, which AU36's answer (4) placed here. The
levers that remain, for that evaluation: the compile cost (a cheaper
allocation for a big function, or expansion only into small callers,
where the hot calls are not); the borrowing of a region's boxed
arguments (a parameter the callee only reads could be aliased without a
retain, sparing some 30 instructions a call); the admission of the hot
sites (a snapshot of the hotness in the compile request, the callee
called most first, or one region shared by a callee's sites with a
return dispatch); a smaller protocol for the call itself instead of its
removal; and the compiler's own code (`peek_at`'s four `token_at` sites
as one, the keywords as fieldless variants deferred at AU48, the lexer's
`contains` per character and the checker's loops over every character
for the line starts as library primitives).

**AU51. The owner's answers after B3, and the next stage.** Asked at the
end of the session of 2026-10-11, with AU50's numbers (four questions,
every recommended answer taken): (1) the gate of AU35 (ii) stays at 3
times the Rust front end's time on `compiler/bodies.ry`, and the work
goes on finding levers, the binary keeping the Rust front end meanwhile;
(2) the next lever is the compiler's own code, where the profile is
known: `peek_at`'s four `token_at` sites as one, the keywords and
symbols as variants without fields (AU48's deferred cut), the lexer's
`contains` per character and the checker's loops over every character
for the line starts and the layout check as library primitives or as
loops over lines; (3) the expansion of AU50 stays in the tree, off
unless `RENYI_NATIVE_INLINE=1`, with its tests and its hand-back, for a
later refinement or measure; (4) the site's fourth round is final as it
stands. The stage this opens, B4 in the handoff's naming (stage 28 of
the roadmap): the cuts in the order of their expected gain per effort,
each judged byte-equal by the four judges, measured by the drivers
(`compiler/lex_only.ry`, `parse_only.ry`, `checker.ry` under callgrind
with `RENYI_NATIVE_SYNC=1`) and by the ratio of AU36 (iii), and recorded
in its own entry: (i) the checker's line starts and layout check over
lines instead of characters (`report.line_starts`,
`checker.check_layout`: 4.8% and 6.5% of the checker's interpreter
samples); (ii) the parser's look computed once per move of the cursor
and read from it by every test, instead of `peek_at` per test (`peek_at`
7%, `token_at` 3.7%, `after_breaks` 2.2% of the parse's samples); (iii)
the keywords and symbols as variants without fields compared by tag, in
place since AU49 (`rt_compare` 6.6% of the three-turn parse); (iv) the
lexer's scan over a character class as a library primitive, a library
addition through the sketch, the declarations, the natives and the
checker's copy (`scan_word` 15%, `lex_range` 12%). The rule for each:
AU1's measure on the self-check does not apply (the frozen self-check is
`bench/selfcheck/`, which these cuts do not touch); a cut stays when its
driver's instructions fall and the judges pass.

**AU52. B4's first cuts: the checker over lines, the parser's look in
the cursor, the lexer's scans as two list primitives.** The stage AU51
opened, its cuts (i), (ii) and (iv), each judged byte-equal by the four
judges and the binary's tests. (i) `report.line_starts` takes the text
and measures the pieces of `Text.split("\n")` instead of walking every
character, and `checker.check_layout` walks the lines from those starts,
reading only each line's trailing run of spaces and tabs (`tokens.ry`
drops its own copy of the loop): the Renyi checker on
`compiler/bodies.ry` 4,898.3 to 4,802.1 million instructions (-2.0%; the
profiler's 11% was the interpreter's weight, the compiled loops being
cheaper). (ii) The parser's `Cursor` carries `look`, the index of the
next significant token past the line breaks the layout rules skip,
computed once wherever a cursor is built at a new position or nesting
(`at_position`, `with_nesting`, from `look_from`, the former `peek_at`;
the two constructors and the 47 sites that moved a cursor go through
them), and every test reads the field: the parser alone 799.4 to 792.9
million instructions per parse (-0.8%: the look was computed a few times
per position, not many). (iv) Two primitives on `List of Item` where
`Item can Hash` (a library addition, not a surface change:
`docs/design/04-stdlib-sketch.md`, `library/std/prelude.ry` and the
checker's copy, the natives with typed entries, the cheat sheet and its
two copies, three sentences of which were tightened to keep its budget,
2,999 of 3,000 tokens): `run_of(members: Set of Item, start, stop)`, the
index after the longest run of members of the set from `start` below
`stop`, `start` itself when the item there is not in the set, and
`run_until(stops, start, stop)`, the index of the first item of the set
from `start` below `stop`, or `stop`; the bounds are clamped to the list
as `slice`'s are; a test in `tests/semantics.rs`. The lexer's seven
scans (`scan_word`, `scan_name_tail`, `scan_alphanumeric`,
`scan_digits`, `scan_spaces`, `scan_operator_tail`, `line_end`), loops
of one character per turn with one to three `contains` tests each, are
one call each over a character class kept as a set beside the texts it
comes from (`word_set`, `name_tail_set`, `alphanumeric_set`,
`digit_set`, `digit_or_underscore_set`, `space_set`,
`operator_tail_set`, `line_break_set`): the lexer alone
(`compiler/lex_only.ry` on `bodies.ry`, three turns less one) LEX_BEFORE
to LEX_AFTER million instructions per lex (-2.4%); the Renyi checker
on `bodies.ry` 4,750.9 million (-1.1% against (i)'s 4,802.1, -3.0% for
the three cuts). The ratio of AU36 (iii) after the three cuts: checking
590.5 ms against the reference's 78.5 (7.5 times, the gate) and this
binary's 38.7 (15.3), parsing 207.9 against 65.6 (3.2) and 62.2 (3.3),
compiling 888.9 against 102.1 (8.7) and 81.8 (10.9): the ratio stays at
7.5, the three cuts' 3% in instructions under the wall-clock's noise.
What stays for the stage: (iii), the keywords and symbols as variants
without fields (`rt_compare` 6.6% of the three-turn parse), and whatever
the profile shows next.

**AU53. B4's cut (iii): the reserved words, phrases and symbols as
Integer codes.** AU48 deferred the keywords as variants without fields,
bounded by `rt_compare`'s share (6.6% of the three-turn parse: every
`is_word(token, "function")` and `is_symbol(token, ",")` compared two
texts through the helper, and `binary_operator` tested up to fourteen
literal patterns per call). The cut takes a cheaper shape than variants:
a new module `compiler/words.ry` holds the reserved words, the phrases
and the symbols (moved from the lexer, which imports them back), gives
each a code (`word_codes`, `symbol_codes`, built once from the lists;
the words from 1 in list order, the symbols from 1000) and a public
constant per spelling (`word_function`, `word_is_at_least`,
`symbol_comma`, 116 of them, each with its purpose line), with
`word_code`, `symbol_code` and `spelling_of` for the few sites that hold
a text. The lexer's `Token` carries `code` (a `Word`'s spelling or a
`Symbol`'s text looked up once at lex time, 0 for every other kind; at
first through `make_token` and `code_of`, then as the third step below
says); the parser's `is_word`, `is_symbol`, `eat_*`, `past_*` and
`expect_*` take a code and compare `token.code is code`, an Integer
compare in place since AU2, and `binary_operator` is a chain of such
compares on the token's code; 153 call sites were rewritten mechanically
(a scanner over each call's argument list, the literal to its constant;
a variable text goes through `word_code` or `symbol_code`), the `match`
arms with literal field patterns stay as they were (`when Word(spelling:
"let")`, some twenty, off the hot paths but the statement and item
dispatch). The token dump, the tree and every diagnostic are unchanged:
the tokens judge and the parse judge on 118 files, the four judges and
the binary's tests pass. Measured in three steps, each by its driver
under callgrind (`RENYI_NATIVE_SYNC=1`; the parser alone three turns
less one of `compiler/parse_only.ry` on `bodies.ry`, the lexer alone
likewise with `lex_only.ry`, the checker one run of `checker.ry`).
First, the cut alone: the parser 792.7 to 806.1 million instructions per
parse (+1.7%), because a read of a module constant is `Op::Global`,
which both tiers compiled as a call to `rt_global` (the VM's table of
built constants behind a status check), and the parser now reads one at
every test where it compared a literal before; the lexer 506.6 to 562.0
million per lex (+10.9%), because every token went through `make_token`
and `code_of`, three calls and a map lookup; the checker's run 4,750.9
to 4,903.0 million (+3.2%). Second, the VM's part, a module constant
read in place: the VM notes every constant it builds in a table the JIT
owns (`Jit::note_global` from `Vm::global`; `GlobalSlot`, the value with
one reference of its own behind a flag, reached through
`NativeState::globals`); both tiers copy the value from the table in
place once the flag is set and retain it there, and call `rt_global`
only until then (the template's `Op::Global`, and the Cranelift tier's
with the helper on a cold block); a compilation on the Cranelift tier
also takes a snapshot of the constants built to a small Integer
(`small_globals`, handed to the compile thread with its request) and
reads such a constant as the literal, an Int operand in a register
(`infer::analyse` takes the slice; `codegen`); code format 15; a test
per tier, `module_constants_are_copied_in_place_once_built`. Third, the
lexer's part: the one lookup that gives the code stands where the set
test stood (`words.word_codes.get(text) otherwise 0` in `lex_word`,
`symbol_codes.get(ch)` in `next_step`; a phrase looks its own code up
once more), the hot tokens are built in place with `code: 0`, and
`make_token` serves the cold sites and the parser's end-of-file token.
After the three: the lexer alone 494.4 million per lex (-2.4% against
506.6), the parser alone 764.6 million per parse (-3.5% against 792.7),
the checker's run 4,691.7 million (-1.2% against 4,750.9; the cold phase
keeps part of the parser's gain, the compile and the first parse on
templates); AT1's measure of the VM's part on the self-check
(`tools/measure_size.sh`, the synchronous row, the binary before the
VM's part against this one): 6,389.0 to 6,376.4 million instructions
(-0.2%) and 9,002.1 to 8,955.4 million by the estimate (-0.5%); the
image row 4,930.2 to 4,918.3 million (-0.2%) and 7,084.1 to 7,039.8
million by the estimate (-0.6%), the image 14 KB larger (8,838,240
bytes) for the sequence per read. The ratio of AU36 (iii) after the four
cuts (`tools/ratio.py`): checking 589.8 and 606.6 ms on two runs of this
boot against the reference's 82.5 and 74.9 (7.2 and 8.1 times, the gate;
7.5 after the three cuts with the reference at 78.5) and this binary's
37.6 and 36.4 (15.7 and 16.7), parsing 221.4 and 229.4 against 61.9 and
67.4 (3.6 and 3.4), compiling 899.3 and 825.9 against 122.9 and 111.3
(7.3 and 7.4): unchanged within the wall-clock's noise, which is wider
than the cut's 1.2% in instructions. Also in this commit: AU52's natives
table gave `index_of`'s typed entry to `run_until` and none to
`index_of`, which `tests/extension.rs` refused (CI red on e91cec0);
fixed.

**AU54. B4's fifth cut: two lookups by index, the lexer's last append in
place, and a shared list appended with one copy.** The profile of the
checker's run after AU53 (callgrind's callers and a census build that
named the Renyi site of every `append` that found its list shared):
`Rc::make_mut` in `list_append` cloned 4.9 million list items per run,
60,108 appends on shared lists, 84% of the items in `bodies.record` (the
body's references, a copy of the whole list per reference, since the
checker record arrives shared), then `infer`, `declare_function` and the
lexer's end-of-file append (24 copies of a file's tokens); and every
clone was at the list's exact length, so the push that followed moved it
again through `realloc` (`memcpy` 4.5% of the run, 143 million of it
under `realloc`). Beside it, `find_binding` built a reversed copy of the
scopes for each of its 62,285 lookups, and the emitter's `lookup` did
the same twice per name. The cut: (i) `bodies.find_binding` and
`emit.lookup` walk their scopes by index from the innermost, as
`update_binding` did; (ii) `lexer.lex` appends the end-of-file token
with `change lexed to lexed with tokens: lexed.tokens.append(eof)`,
AU11's form, so the field is taken and the list appended in place; (iii)
the natives `append` and `append_all` copy a shared list once, with room
for what is appended (`Rc::get_mut` for the list held once, else
`Vec::with_capacity` and one `extend`), where `make_mut` cloned it at
its length and the push moved it again. Judged byte-equal by the quick
judges on 118 files and the four judges. The measure (callgrind,
`RENYI_NATIVE_SYNC=1`): the VM's part alone, on the committed sources,
the checker's run 4,691.7 to 4,558.4 million instructions (-2.8%); with
the two Renyi parts, 4,544.4 million (-3.1%); the lexer alone 494.4 to
491.4 million per lex (-0.6%); AT1's measure of the VM's part on the
self-check (the synchronous row, AU53's binary against this one):
6,376.4 to 6,235.9 million instructions (-2.2%) and 8,955.4 to 8,746.6
million by the estimate (-2.3%); the image row 4,918.3 to 4,775.6
million (-2.9%) and 7,039.8 to 6,851.8 million by the estimate (-2.7%),
the interpreter -1.2%, the image unchanged in size. What the profile
leaves: the copies that remain are the checker's own shape, a function
that takes `checker: Checker` and updates a mutable copy of it while the
parameter slot still holds the record, so every `with` copies the record
and every taken field is cloned (AU11 counted 47,000 of its 71,000
updates in the slot finding the record shared); the liveness rule AU11
left to the next profile (a `let` or a `return` that reads a local for
the last time moves it) is the next cut.

**AU55. B4's sixth cut: the last read of a local moves it (the liveness
rule AU11 deferred), a taken field read in place on both tiers, and the
checker's values read before its checkers.** AU54 left the copies that
remain to the checker's own shape: a function takes `checker: Checker`
and derives a mutable copy while the parameter slot still holds the
record, so the record is shared wherever it is updated. The rule, in
both emitters (`crates/renyi_vm/src/compile/liveness.rs`, the same in
`compiler/emit.ry`, held equal by the compile judge): a read of a local
is its last when it is the only read of that local at its statement's
own level, the local is read nowhere in the statement's nested blocks
nor after the statement in its block or in the blocks around it, and
every loop around the statement binds the local anew on each turn (a
read inside a query, or inside a loop's filter, key or condition, runs
more than once and is never last); a write is no mention, since a read
after it sees what it stored; a nested block whose last statement is
`return`, `fail` or `crash` never falls through, so its reads need no
later-statement and no loop check. Such a read is a `LoadMove`, and
`x.f` on such a local a `TakeField` (AU11's op), so a record or a
collection held once stays held once into the call, the construct or the
update that follows. The analysis walks each block from its last
statement, the names mentioned after it kept as a set, a frame per
enclosing block (the names mentioned after its statement, and the names
a loop binds), two traversals per node. `TakeField` went through
`rt_take_field` on both tiers while `LoadField` read in place through
the site's cache (AU21), and the rule turns 816 field reads of the
checker into takes, so the first measure came out worse (the checker's
run 4,544.4 to 4,659.6 million); both tiers now take a field in place as
they read one: a record of the cached type held once gives up the field,
`Nothing` left in its place, a shared one gives a copy with one more
reference, a variant or a miss goes through the helper. The checker's
code (`compiler/bodies.ry`) read against the rule in two ways, both
rewritten by a script and judged byte-equal by the checker judge: 27
reads of the parameter `checker` after the derived value existed (a
stale read, where the Rust checker's `&mut self` reads the latest state)
now read the derived value; and 113 `Done` results read their `.value`
after their `.checker`, keeping the record alive at the take, so each
binds its value first (`let done_value be done.value`) and the two `done
with checker:` updates construct a fresh `Done`, which makes the
`.checker` read the last read of `done`. In the checker's bytecode 4,914
loads become moves and 816 field reads takes. The measure (callgrind,
`RENYI_NATIVE_SYNC=1`, against AU54): the checker's run 4,544.4 to
4,470.1 million (-1.6%), the parser alone 764.6 to 767.9 million per
parse (+0.4%), the lexer alone 491.4 to 498.1 million per lex (+1.4%:
its code is bound by the hash of each character against a set, and a
take in place tests the count where a read did not); the Rust front
end's compile of the checker 723.9 to 758.2 million for the analysis
(+4.7%, the hashing of the names in its sets; interning them to bits is
the fix left); AT1's measure of the tiers' part on the self-check
6,235.9 to 6,260.7 million instructions (+0.4%) and 8,746.6 to 8,792.5
million by the estimate (+0.5%); the image row 4,775.6 to 4,775.0
million in instructions and 6,851.8 to 6,884.4 million by the estimate
(+0.5%), the code 1.4% larger (the frozen compiler of AU39 keeps the
shape the rule does not reach, and pays the take's count test and the
larger code); the ratio of AU36 (iii) checking 557.9 ms against the
reference's 81.4 (6.9 times, the gate; 7.2 and 8.1 after AU53, with the
Renyi checker at 589.8 and 606.6) and this binary's 38.4 (14.5), parsing
215.6 against 75.3 (2.9) and 64.0 (3.4), compiling 922.5 against 143.3
(6.4) and 90.6 (10.2), the best of five runs: the checker's time is the
first of the stage's cuts to move outside the wall-clock's noise, and
the compile row pays the analysis written in Renyi. CLAUDE.md's rows for
`compile/`, the tiers and `compiler/` say what is there now.

**AU56. B4's seventh cut: the frame leave drops only the slots that hold
a counted block.** After AU55 the largest VM cost of the checker's run
was the return: `rt_return` 399 million instructions inclusive (8.9%),
of which `leave_frame`'s truncation of the frame's slots called the drop
glue of `Value` on every slot, 10.6 million times per run, most of them
on plain values (`Nothing`, a Boolean, a small Integer, a Float, a text
held in the value) with nothing to drop.
`Pinned<Value>::truncate_values` reads each slot's tag and drops only a
value whose tag is in `COUNTED_OR_INTEGER` (the mask the generated code
tests before a retain or a release, decisions AU29 and AU32); the frame
leave of the generated code (`runtime::leave_frame`), the interpreter's
`leave!` and `rt_truncate` use it. The measure (callgrind,
`RENYI_NATIVE_SYNC=1`, against AU55): the checker's run 4,470.1 to
4,432.9 million (-0.8%), the parser alone 767.9 to 763.4 million per
parse (-0.6%), the lexer alone 498.1 to 493.7 million per lex (-0.9%);
AT1's measure on the self-check the synchronous row 6,260.7 to 6,223.8
million instructions (-0.6%) and 8,792.5 to 8,771.4 million by the
estimate (-0.2%), the image row 4,775.0 to 4,726.6 million (-1.0%) and
6,884.4 to 6,811.1 million by the estimate (-1.1%), the interpreter
unchanged. Also in the stage's record: the cheat sheet stood at 3,001
tokens by the gate's tokenizer after AU52 (the byte estimate this
environment can take said 2,999), which kept CI red from d172887 to
fa27d73; two phrases a word shorter (81c5ef9).

**AU57. B4's eighth cut: `List.with_at`, a replacement by index in
place; the hash of a small text by its two words, measured slower and
left out.** `lists.replace_at(items, index, item)`, by which the
compiler written in Renyi updates a list by position (twenty call
sites: the checker's variables and scopes in `bodies.ry`, the world's
modules, types, functions, abilities and parameters in `declare.ry`,
the emitter's ops, codes, types and fields in `emit.ry`), built three
lists per call: `take(index)`, `append(item)` and `append_all` of
`drop(index + 1)`. The prelude gains `with_at(self: List of Item,
index: Integer, item: Item) returns List of Item for any Item`, a
library addition (the sketch, `library/std/prelude.ry` and the
checker's copy, the native `list_with_at` in `natives/prelude.rs`, the
cheat sheet and its two copies, a test in
`crates/renyi_vm/tests/semantics.rs`): the list with the item at the
index replaced, in place when nothing else holds the list and with one
copy otherwise, the list as it was when the index is out of range,
which is what `replace_at` answered; `replace_at` is now that one call.
The cut's other half hashed a text held in the value (`SmallText`, at
most 15 bytes, which `Value::text` always uses for a text that short)
as its two words in place of its bytes, to spare the loop over them;
but the VM's maps and sets hash with the standard library's SipHash,
for which sixteen bytes cost more than the one to three bytes of a
typical token and the terminator (`Hasher::write` 26.1 to 50.3 million
instructions per lex), and with both halves the lexer alone went from
493.7 to 517.5 million instructions per lex (+4.8%), the parser alone
763.4 to 764.7 million per parse (+0.2%) and the checker's run 4,432.9
to 4,438.4 million (+0.1%), so the hash stays as it was. What the
profile names instead is a hasher of the VM's own for its maps and sets
(a multiplicative hash over the words, keyed per run, since SipHash's
keys are what defends a map against keys chosen to collide), a decision
of its own. The measure of `with_at` alone (callgrind,
`RENYI_NATIVE_SYNC=1`, against AU56): the checker's run 4,432.9 to
4,391.6 million (-0.9%), the parser alone 763.4 to 763.3 million per
parse and the lexer alone 493.7 to 493.6 million per lex (both
unchanged: neither calls it); the four judges byte-equal. The stage's
total: the checker's run 4,898.3 to 4,391.6 million (-10.3%). The cheat
sheet: the gate's two tokenizers, which the container's proxy keeps
tiktoken from fetching, are in the `tiktoken-rs` crate on crates.io
(the handoff says how to count with them offline); with `with_at`
listed and two phrases a word shorter it stands at 2,997 tokens by
o200k_base and 2,992 by cl100k_base, where the byte estimate says
2,992.

**AU58. The owner's answers after B4: a new backend for the VM, its
design study first.** Asked at the end of the session of 2026-10-11
(four questions, every recommended answer taken), with AU56's numbers
and AU57 under measure. The question set out the distance to the gate
of AU35 (ii) and what could cover it: cuts of one to three percent
each will not (B4's eight came to 10.3% of the checker's run), and a
new backend could (values unboxed, counts elided where ownership is
static, arguments in registers, records laid out as structures, and
the checker's data structures rewritten to suit them), months of work
by the question's estimate. The answers: (1) the new backend, its
design study first: the instructions of the checker's run attributed
by category, the backend's design, its stages with an estimate each,
then the stages one by one in the sessions after; the gate stays at 3
times, the road to it acknowledged as months long; (2) AU57's two cuts
recorded and pushed as usual (its second half was measured slower and
is left out, as AU57 records); (3) the measure stays the checker's run
on `compiler/bodies.ry` under callgrind and the ratio of AU36 (iii),
with a table by category beside them in each entry (the generated
code, the counts and the frames, the helpers, the allocation, the
hashing); (4) the first stage, which the study is to examine again:
the calling convention, scalar arguments and results in registers and
a parameter that is only read passed without a count, on both tiers,
with a flag per parameter in the bytecode (the return's helper was
8.9% of the checker's run before AU56, its largest single item). A
correction to the question's figures, made the same day: it put the
distance at seven times in instructions, comparing the Renyi checker
with this binary's own Rust front end (about 3,230 million by the
profile's shares against 153 million, 21 times), but the gate's
measure by AU36 (iii) is AU34's binary. Measured (callgrind, the image
cache off): the Renyi checker run from its image, as `tools/ratio.py`
runs it, 3,080.0 million instructions on today's `compiler/bodies.ry`;
AU34's `renyi check` of the same file 442.8 million (7.0 times, beside
the wall-clock's 6.9); this binary's `renyi check` 179.4 million (17.2
times). The gate asks the checker's image run to fall to 1,328.3
million, 2.3 times fewer instructions, as the wall-clock says (the
Renyi checker 557.9 ms against three times the reference's 81.4, 244);
the study takes the gate's measure. The session ended at the owner's
word, to save the progress and prepare the handoff, before the study
began.

## AV. The icon and the wordmark (session 13)

**AV1. The mark is a war hammer on the accent tile; the wordmark is the
name signed in Ms Madi.** The owner asked on 2026-10-11 for an icon that
does not read as the R language's, as the R on the accent tile did, and
chose over five rounds of rendered options (AskUserQuestion, each sheet
with the candidates in both themes, at 16 to 128 pixels, in the site's
header and in a browser tab): (1) the character 人 three ways and a dot
in brackets, which the owner set aside ("nothing to do with Chinese
characters; diverge"); (2) six directions (a kite, brackets as a kite, a
dot between angle brackets, a spark in brackets, glasses, a signature)
with eight dropped and why (an aperture read as a camera, a seal as a
gear, a grid of grants as an app launcher, a colon as a list, a tethered
dot as a pin, a spark on a line as a wand, a glass box as a package, a
kite-shaped icon too small in a square slot), after which the owner asked
for the name as a signature; (3) the name in forty-five script fonts, the
eight most like a signature shown with four marks to go beside it (a nib,
the signature's initial, a cross on a line, a wax seal): the owner took
Ms Madi, and the signature in the site's header, and asked for an R or Ry
made of a nib or other objects; (4) six such marks (a nib as the R's leg,
an R signed in one stroke with the nib at its end, Ry in the wordmark's
hand, an R cut out of a nib, a lens as the bowl, a ribbon's swallowtail
as the leg), after which the owner asked for Ry on a finer grid, or an
object such as an axe or a war hammer; (5) Ry on three grids (lit cells
of nine, a finer grid of sixteen, graph paper) and five objects (a gavel,
recommended for its meaning, the war hammer, an axe, a stamp, an anvil):
the owner took the war hammer. The colours were the owner's to leave
open, so they are the site's: the accent tile with the hammer in the
colour on the accent, in both themes. What is built: `tools/brand.py`
holds the mark (a square hammer with a flared face, a curved beak and a
top spike on a long haft, drawn upright on a grid of 24 and turned 40
degrees; the full drawing, with a grip band and a pommel, from 48 pixels
up, the plain one, a thicker haft, below) and the wordmark (the name in
Ms Madi, Robert Leuschke's script under the SIL Open Font License 1.1,
shaped by HarfBuzz and kept as one path, so that no page loads a font),
and writes the files made from them: `docs/assets/renyi-mark.svg`, the
README's logo in a light and a dark version (`renyi-logo-light.svg`,
`renyi-logo-dark.svg`, chosen by a `picture` element), the VS Code
extension's icon (`editors/vscode/images/icon.png`, 128 pixels) and the
icon of a `.ry` file in light and dark themes (`file-light.svg`,
`file-dark.svg`, the plain drawing), and the repository's social preview
(`docs/assets/social-preview.png`, 1,280 by 640: the mark, the
signature, the README's first sentence in Inter, the site's address),
the PNG files as screenshots by a headless Chromium. `tools/site.py`
takes the mark, the tab's icon and the wordmark from it and checks that
its colours are the site's; the header shows the mark and the signature,
the name kept for screen readers. The social preview reaches GitHub
through the repository's settings (Settings, General, Social preview),
an upload only the owner can make. The site's design stays as AU51 (4)
left it but for the header's brand. (user)

## AW. The new backend (session 14)

**AW1. The design study of the new backend (AU58) and the owner's
answers: the Cranelift tier evolved in stages, the gate on the check
row, stage 0 first, the read-only flag in the bytecode, `Done` by a
convention per function, the natives' in-place entry through AK1's
API, the character tables with a hasher of the VM's own, the iterator
in two slots.** The study is the handoff's section "Stage 29, the new
backend: the design study (session 14)", measured on 2026-10-11 on the
release build of 7cbbbe0 for Linux under WSL against AU34's binary
built from 44782aa the same way, by AU58's measure (callgrind on the
Renyi checker run from its image on `compiler/bodies.ry`, the image
built under `valgrind --tool=none`, the caches off). The attribution:
3,085.3 million instructions, of which the generated code 1,011.8
(32.8%), the counts and drops 448.2 (14.5%), the natives with their
calling helpers 358.8 (11.6%), mimalloc 258.9 (8.4%), the hashing and
equality 219.8 (7.1%), the records 208.1 (6.7%), the helpers' glue
(`status`, `into_plain`) 130.9 (4.2%), the comparisons 124.0 (4.0%),
the texts and copies 121.1 (3.9%), the calls through the VM's path
114.1 (3.7%), the load 46.2 (1.5%), the iterators 40.6 (1.3%); the
parse row 1,256.9 and the compile row 4,844.9 with the same shape; the
reference's `check` 441.6, `parse --json` 284.7, `compile` 365.6, so
the ratios by instructions are 6.99, 4.41 and 13.25 against the gate's
3. The findings: a call of a declared function costs about 130
instructions besides its body (2,770,541 calls, about 400 million); a
third of the frames leave through `rt_truncate` at 329 each (917,264
leaves, 301.8 million inclusive), which drops by the tags at run time
where the analysis knows most slots are moved out; 306,392 calls enter
generated code through the VM's path at about 435 each because an
Integer parameter's operand is boxed at the site, half of them a
module constant an image cannot know as a literal; a record update
costs about 1,070 (201,205 updates, 219.0 million) and a record's free
232 (1,084,633 frees, 251.2 million) while a construct costs 63; every
allocation goes through mimalloc's aligned entry and 641,578 through
its generic path; the natives' calling glue is about 150 million and
the lexer's character classes 409,270 of the run's 555,512 hashed
lookups; 521,364 comparisons go through the helper, 387,234 of them of
two values of a field-less variant type; a `for each` allocates an
iterator (238,534, about 95 million); the generated code's own
sequences are about 600 million for 55 million ops, the retains and
releases the larger half. The design weighed two roads: (A) the
Cranelift tier evolved in stages, each measured, the judges and the
conformance suite green at every step; (B) a new code generator with
the operand stack and the locals in registers, the frame record lazy,
the counts elided by dataflow, about 10% more than (A)'s stages for
three times the work and months without a measure. The stages of (A)
with their estimates: stage 0, the allocator entered by mimalloc's
plain functions for an alignment of at most 16 and a module constant
whose code object is a literal known to the JIT without the snapshot
(165 million, 5%); stage 1, the calling convention, a typed
parameter's boxed operand unboxed at the site, the leave by the static
ownership state of the slots, a parameter only read passed borrowed
under a flag per parameter in the bytecode (the callee's leave skips
it when entered directly, every other entry owns it, `rt_deopt`
retains it), the frame record cheaper (400 million, 13%); stage 2, the
records: `with` by static index with the copy in place, the two-field
result record returned in the caller's slots when the caller takes it
apart at once (`Done`), the frees in place (400 million, 13%); stage 3,
the natives answering in place through a second entry per native, the
character classes as 128-bit tables in `run_of` and `run_until`, the
hasher of the VM's own, `status` and `into_plain` off the path (315
million, 10%); stage 4, two values of a field-less variant type
compared in place, `match` by a table, the iterator in two slots (185
million, 6%); stage 5, the moves of AU17 written in the image and the
code objects decoded on demand (50 million); stage 6, AU50's expansion
measured again under the new convention (25 million); the checker's
own shape beside them (the lexer's text of a run, the lists shared
when appended, the linear searches, the parser's look): about 250
million. The sum, about 1,540 million, puts the check row near 1,550
million, 3.5 times the reference, the gate's 3.0 reached with the
checker's own cuts or (B)'s register-resident values after the stages.
The owner's answers, every recommended option (AskUserQuestion, two
batches of four): (1) road (A), the stages, over (B) and over "stages
0 to 2 then the choice"; (2) the gate of AU35 (ii) measured on the
check row alone, AU58's measure, the compile row (13.25 by
instructions, the emitter's shape) left to the emitter's own round
after it, over all three rows and over the check and parse rows; (3)
stage 0 first, over stage 1 as AU58 (4) named it and over stage 2; (4)
the read-only flag in the bytecode, a Boolean per parameter computed
by both emitters from the finished ops and held equal by the compile
judge, over the VM deriving it at load and over the leave by state
alone; (5) `Done` without its allocation by the convention per
function, over the checker rewritten and over leaving it; (6) a second
entry per native answering in place through AK1's registration API,
the standard library's hot natives first, over lean helpers for the
hot ones only (AU34's way) and over none; (7) the character classes as
tables and the hasher of the VM's own, over the tables alone and over
none; (8) the iterator in two slots as a bytecode change in both
emitters with the judges, over a lighter native in the slot and over
none. The order: stage 0, stage 1 as 1a, 1c, 1b, 1d, the character
tables and the hasher, stage 2 as 2a, 2b, 2c, the natives' entry and
the glue, stage 4, the checker's own cuts between the stages, stages 5
and 6 last; each measured by AU58's measure with the table by
category from the study's scripts, AT1's rows where the tiers change,
the benchmarks, the four judges and the conformance suite on both
tiers, kept by AT1's rule, its entry recording the table. (user)

**AW2. Stage 0 of the new backend: the allocator entered by mimalloc's
plain functions, and the module constants an image builds before any
run.** The first stage of AW1's order, on 2026-10-11, measured as AW1
prescribes. (i) The allocator: `renyi::Allocator` wrapped the
`mimalloc` crate's `MiMalloc`, which calls `mi_malloc_aligned`,
`mi_zalloc_aligned` and `mi_realloc_aligned` for every allocation;
mimalloc's aligned fast path serves only when the page's free list has
a block, and its generic path over-allocates every block whose size is
not a power of two (`mi_malloc_aligned_at_generic`, 621,420 of the
checker's 2,381,541 allocations in the study). The binary depends on
`libmimalloc-sys` itself now, and `Plain` calls `mi_malloc`,
`mi_zalloc` and `mi_realloc` for an alignment of at most 8 (every
block mimalloc hands out is a multiple of eight bytes at a multiple of
eight from a page area that starts sixteen-aligned, so the plain
entries honour it), the aligned entries above it; the VM's blocks
(`Composite`, `Layout::from_size_align(size, align_of::<Value>())`),
its vectors and its texts ask for eight or less, hashbrown's tables for
sixteen. (ii) The constants: the study's item was a module constant
whose code object is a `Const` and a `Return`, known to the JIT
without the snapshot; but the checker's constants are computed, not
literal (of its 184, 30 are a literal and 118 a native call on a
literal, `symbol_code(",")` and `word_code("ability")`, the rest lists,
sets and maps built from literals and from one another), so that rule
alone reached none of the parser's `is_symbol(code: words.symbol_comma)`
sites. Two parts then: `Jit::new` fills the table of module constants
(`GlobalSlot`, decision AU53) with every literal one, so that a run and
a build know them before the VM builds them; and `image::build`
evaluates on the interpreter every constant `pure_constants` admits and
notes the values with the JIT, so that a small Integer among them is a
literal in the image's code (`Jit::small_globals`) and the call sites
that pass it reach the direct path. The values go with the build's VM;
the run builds its own on the first read as before (reference section
6), the same values by the purity. A constant's code is effect-free
(the checker admits no `needs` in one, `capability-missing`; a foreign
or a Python function needs `foreign` or `python`), so what it builds is
the same at every run unless a deadline passed while it was built (`run
concurrently within` needs nothing): `pure_constants` takes a constant
whose code reaches, through the declared functions it calls and the
constants it reads, no deadline, no ability call (a method on a type
parameter; one on a known type is a declared function), no call of a
function value and only natives without `needs`, as a fixed point over
the code objects; what an op or a native reaches through an ability's
implementation on a record (`hash`, `equals`, `to_text`) it does not
follow, such an implementation with a deadline in it having been a race
between two runs already. A constant whose build fails is left to the
run, which crashes at its first read as before, and a new VM builds the
rest; a constant whose build does not end would hang the build, as it
hangs the run that reads it. Neither part touches a format. The test:
`crates/renyi_vm/tests/native.rs`, a program with a literal constant,
one computed through a declared function, a list, one that fails, one
through an ability on a type parameter and one through a function
value, the pure five named, the run from the image equal to the
interpreter's. The measure (callgrind under WSL on the release build
for Linux, the images built under `valgrind --tool=none`, against the
binary of 7cbbbe0 measured the same way in AW1, the reference AU34's
44782aa): the check row 3,085.3 to 2,889.9 million instructions (-6.3%;
the study estimated 165 million, 5%), the parse row 1,256.9 to 1,162.8
(-7.5%), the compile row 4,844.9 to 4,608.8 (-4.9%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6) 6.54,
4.08 and 12.61 against the gate's 3 (6.99, 4.41 and 13.25 before); the
Rust front end's own rows on this binary, which the allocator serves
too, `check` 178.8 to 173.4 (-3.0%), `parse --json` 266.8 to 246.7
(-7.5%), `compile` 471.1 to 445.1 (-5.5%). The check row by category,
AW1's table against this one, in millions: the generated code 1,011.8
to 996.9, mimalloc 258.9 to 146.8 (the allocator's part, 112.1 against
the 100 estimated), the counts and drops 448.2 to 444.5, the calls
through the VM's path 114.1 to 57.3 (`call_from_stack` 306,330 to
141,589 calls, `rt_call` 296,968 to 132,227: the constants' part, with
the generated code's 14.9 and the glue's 5.7 about 80 against the 65
estimated), the helpers' glue 130.9 to 125.2, the records 208.1 to
206.8, the natives 358.8 to 358.2, the hashing 219.8 to 219.9, the
comparisons 124.0 to 123.9, the texts and copies 121.1 to 121.0, the
load 46.2 to 46.0, the iterators 40.6 to 40.3. `tools/ratio.py` under
WSL on the same binaries gives 1.9, 1.6 and 2.9 by wall-clock, the best
of five, but the reference's rows there are the file reads through
`/mnt/d` (266.8 ms for a `check` of 441.6 million instructions), so the
ratio by instructions is the one recorded, as AW1 (2) fixed. AT1's rows
on the frozen self-check (`tools/measure_size.sh` under WSL, the binary
before against this one): the synchronous JIT row 6,237.9 to 6,013.9
million instructions (-3.6%) and 8,883.9 to 8,659.1 million by the
estimate (-2.5%), the image row 4,742.8 to 4,538.9 (-4.3%) and 6,943.9
to 6,690.1 by the estimate (-3.7%), the cached run 4,821.3 to 4,618.7
(-4.2%), the interpreter 11,451.3 to 11,240.2 (-1.8%) and 15,726.1 to
15,399.6 by the estimate (-2.1%); the image 8,947,664 to 8,945,136
bytes. The four benchmarks and hello by user-space instructions (`perf
stat`, the JIT synchronous, in millions): `primes` 289.1 to 288.8,
`strings` 433.5 to 430.1 (-0.8%), `records` 236.5 to 209.4 (-11.5%),
`json_round_trip` 872.7 to 838.1 (-4.0%), `hello` 5.67 to 5.42 (-4.4%).
The four judges byte-equal, the conformance suite 55 of 55 on both
tiers, the template tier's own tests with the tier forced, 384 tests.
Found on the way and fixed in a2c4685 before this stage's commit: on
Windows every run of an image had the mapping of its code section
refused ("access denied": a mapping with `PAGE_EXECUTE_READ` needs
`GENERIC_EXECUTE` on the handle, which `File::open` does not ask) and
copied the section instead since decision AT5, which the two tests of
`crates/renyi/tests/build.rs` that assert "mapped from the file" showed
on the owner's machine; `image::open_file` opens for reading and, on
Windows, execution, the executable of decision AS4 opens itself the
same way, and the report names a refused mapping with the system's
reason.

**AW3. Stage 1a of the new backend: a typed parameter's boxed operand
taken from the stack at the call site.** The first item of stage 1 in
AW1's order, on 2026-10-11, measured as AW1 prescribes against stage
0's binary (2632cde). Before it, the slow calls that remained after
stage 0 were attributed again (the lane's `study_map.py`, extended to
rank the calls that enter a code object through its trampoline, that is
through the VM's path): 141,577 into 62 code objects, half of them
`type_at` (69,493), then `lex_reserved` (18,762), `assign_app`
(13,546), `var_at` (7,460), `line_layout` (7,209), `argument` (6,501, a
function value through `bracketed`), `module_at`, `lookup_param`,
`parameter` (a function value), `function_at`, `call_known`; every one
but the two function values an Integer parameter whose operand the
generated code holds boxed although the checker typed it Integer: a
variant's field bound by a pattern (`when Named(id)`, `when App(id:
actual_id, ...)`, `when some(function_id)`), which `Unpack` pushes
without a type and the slot's `Load` notes, a `maybe` after its
`otherwise` (`words.word_codes.get(text) otherwise 0`, `starts.at(line)
otherwise 0`), whose join with the literal is boxed, or a difference of
two such (`stop - start`). AR3's direct call wanted a typed operand for
a typed parameter and sent these through `rt_call` at about 435
instructions each (the frame pushed by the VM, the trampoline,
`rt_param_int`). The change: the direct condition of `Op::Call` in
`native/codegen.rs` admits, for a typed parameter, a boxed operand the
checker noted as the parameter's type (`noted_kind` in
`native/infer.rs`: the note's type read as `abs_of_type` reads a
parameter's, so a refined subtype of Integer counts as Integer, as it
does for the parameter); and `call_direct` tests such an operand's tags
in place before the table lookup (an Integer that is small, a Boolean,
a Float: the same tests `unbox_top` makes), goes to the helper's path
with the state untouched when one fails (a big or a guarded Integer,
which the trampoline's `rt_param_int` would have handed to the
interpreter as before), and otherwise reads the payload from the stack
into the register the body's signature takes, leaving the operand on
the stack for the frame push to move into the parameter's slot, where
the trampoline's entry reads it: the slot holds a plain small Integer,
which no leave counts, instead of whatever the stack held. Nothing else
moves: an operand in the middle of the arguments is not shifted out,
which would cost a copy per operand above it; the frame's move is the
same copy the boxed parameters get, and none when every parameter is on
the stack, as in `type_at(world, id)` and `assign_app`. The interpreter
and the template tier are untouched, as the study set for 1a (the
template tier passes boxed arguments and the trampoline unboxes them);
no format changes. The test: `crates/renyi_vm/tests/native.rs`, an
Integer, a Boolean and a Float parameter each called with a variant's
field bound by a pattern and with a `maybe` after its `otherwise`, a
big Integer through the same site, both tiers equal to the interpreter;
the IR of the caller shows the two tag tests and the branch to the
helper's path before the table lookup, the payload read before the
frame push. The measure (callgrind under WSL on the release build for
Linux, the images built under `valgrind --tool=none`, against stage 0's
binary measured the same way in AW2, the reference AU34's 44782aa): the
check row 2,889.9 to 2,826.6 million instructions (-2.2%; the study
estimated 65 million after stage 0, 2.1%), the parse row 1,162.8 to
1,152.9 (-0.9%), the compile row 4,608.8 to 4,535.7 (-1.6%); the ratios
by instructions against the reference's rows (441.6, 284.7 and 365.6)
6.40, 4.05 and 12.41 against the gate's 3 (6.54, 4.08 and 12.61 after
stage 0); the Rust front end's own rows unchanged (`check` 173.2,
`parse --json` 246.7, `compile` 444.9). The calls through the
trampolines 141,577 to 9,380 (`call_from_stack` 141,589 to 9,392,
`rt_call` 132,227 to 30): what remains is the parser's function values
(`argument` 6,501, `parameter` 1,952, `field_pattern` 463,
`variant_field` 275, `expression` 170, each the item parser `bracketed`
and its kin take as a value, `rt_call_value`) and the eleven module
constants the run builds at their first read. The check row by
category, AW2's table against this one, in millions: the generated code
996.9 to 994.9, the calls through the VM's path 57.3 to 7.4 (the
stage's part, 49.9, with the glue's 4.5, the counts' 3.8, the natives'
2.3, the generated code's 2.0 and the copies' 0.7 about 63 against the
65 estimated), the helpers' glue 125.2 to 120.7, the counts and drops
444.5 to 440.7, the natives 358.2 to 355.9, the texts and copies 121.0
to 120.3, mimalloc 146.8 both, the records 206.8 both, the hashing
219.9 to 219.8, the comparisons 123.9 both, the load 46.0 both, the
iterators 40.3 both. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 0's binary against this one):
the synchronous JIT row 6,013.9 to 6,008.2 million instructions (-0.1%)
and 8,659.1 to 8,640.4 million by the estimate (-0.2%), the image row
4,538.9 to 4,496.7 (-0.9%) and 6,690.1 to 6,622.5 by the estimate
(-1.0%), the cached run 4,618.7 to 4,576.3 (-0.9%), the interpreter
11,240.2 to 11,240.5 (unchanged); the image 8,945,136 to 9,008,656
bytes (the tag tests, 173.9 bytes of body per op from 172.5). The four
benchmarks and hello by user-space instructions (`perf stat`, the JIT
synchronous) are unchanged to the sixth figure (`primes` 288.8 million,
`strings` 430.1, `records` 209.4, `json_round_trip` 838.1, `hello`
5.42): no such site lies on their paths. The four judges byte-equal,
the conformance suite 55 of 55 on both tiers, the template tier's own
tests with the tier forced, 385 tests. Stage 1c next, the leave by the
static ownership state: on this binary 917,264 frames still leave
through `rt_truncate`, 301.5 million inclusive, finding 2's target.

**AW4. Stage 1c of the new backend: the frame's leave by the static
ownership state of the slots.** The second item of stage 1 in AW1's
order, on 2026-10-11, measured as AW1 prescribes against stage 1a's
binary (7bb927b). Finding 2 of the study: a third of the frames left
through `rt_truncate` (917,264 leaves, 301.5 million instructions
inclusive on stage 1a's binary, 10.7% of the check row), which cuts the
stack to the frame's base and drops every slot by its tag at run time,
because the leave in place released a frame's locals one by one only
when the slots that are not typed, with the boxed operands above them,
were at most four (`RELEASES_INLINE`), and the callers with many locals
(`next_step`, `resolve`, `substitute`, `lex_word`, `expr_span`,
`find_binding`) went to the helper although AU55's moves leave most of
their slots holding `Nothing` at the return. The change: `owned_slots`
in `native/infer.rs` computes, per op, the slots that may hold a
counted block on entry to it, as a fixed point over the ops' flow with
the analysis's entry states and handler regions: a boxed parameter from
the entry; a boxed slot from the store of anything but a plain scalar
until the `LoadMove` that moves it out (a moved load a typed call or a
comparison borrows keeps the slot's value, decision AU17, and counts as
no move) or the `WithSlot` that moves the record out; a slot of another
kind (an iterator, a deadline, a mark) always, a typed slot never; a
failure flows to its handler with the state before the op; an op the
analysis does not reach has every slot counted. The leave in place
(`leave_frame_inline` in `native/codegen.rs`, and the boxed result's
leave through it) releases the slots so counted at the return's op,
with the boxed operands above the locals, inline when they are at most
eight (`RELEASES_INLINE` four to eight, the study's figure), and calls
`rt_truncate` otherwise; a slot skipped holds `Nothing` or a plain
value, which the interpreter's own leave by tags would have found
nothing to drop in either. The interpreter and the template tier are
untouched: the interpreter writes the slots as the same ops say, so a
frame it hands over at a loop header fits the state there, and the
template tier's return still goes through `rt_return`, which drops by
tags (its leave by the same analysis is left for a later stage, the
measure being the check row); no format changes. The tests:
`crates/renyi_vm/tests/native.rs`, a frame with thirteen locals (texts,
a list, a record updated, a failing call caught by `otherwise`) left by
three returns, both tiers equal to the interpreter; and
`crates/renyi/tests/sandbox.rs`, the same frame left two hundred
thousand times under a memory budget of sixteen megabytes on both
tiers, through the binary, which a slot skipped wrongly would exceed (a
run that keeps every result alive does, checked by hand); the IR of the
frame shows no call of `rt_truncate` and the releases inline. The
measure (callgrind under WSL on the release build for Linux, the images
built under `valgrind --tool=none`, against stage 1a's binary measured
the same way in AW3, the reference AU34's 44782aa): the check row
2,826.6 to 2,729.4 million instructions (-3.4%; the study estimated 200
million, 6.5%), the parse row 1,152.9 to 1,132.9 (-1.7%), the compile
row 4,535.7 to 4,395.7 (-3.1%); the ratios by instructions against the
reference's rows (441.6, 284.7 and 365.6) 6.18, 3.98 and 12.02 against
the gate's 3 (6.40, 4.05 and 12.41 after stage 1a); the Rust front
end's own rows unchanged (`check` 173.4, `parse --json` 246.7,
`compile` 445.6). The leaves through `rt_truncate` 917,264 to 32,514
(16.0 million inclusive, from 301.5); the drops out of line through
`rt_drop_at`, the last reference's, 150,010 to 668,735 calls (201.1
million inclusive, from 34.3): the frees the helper's loop made are
made one by one now, which is why the gain is 97 million against the
200 the study estimated, the study having counted the helper's
inclusive cost as the leave's where the drops of the last references
are the values' own cost and remain; the leave itself fell from about
329 instructions to about 20. The check row by category, AW3's table
against this one, in millions: the generated code 994.9 to 1,014.6, the
counts and drops 440.7 to 321.9, the helpers' glue 120.7 to 120.7, the
calls through the VM's path 7.4 to 7.4, mimalloc 146.8 to 146.8, the
records 206.8 to 206.8, the natives 355.9 to 355.9, the hashing 219.8
to 221.8, the comparisons 123.9 to 123.9, the texts and copies 120.3 to
120.3, the load 46.0 to 46.0, the iterators 40.3 to 40.3. AT1's rows on
the frozen self-check (`tools/measure_size.sh` under WSL, stage 1a's
binary against this one): the synchronous JIT row 6,008.2 to 5,981.1
million instructions (-0.5%) and 8,640.4 to 8,622.5 by the estimate
(-0.2%), the image row 4,496.7 to 4,376.1 (-2.7%) and 6,622.5 to
6,507.6 by the estimate (-1.7%), the cached run 4,576.3 to 4,455.7
(-2.6%), the interpreter 11,240.5 to 11,240.4 (unchanged); the image
9,008,656 to 9,837,312 bytes (+9.2%: the releases inline at every
return, 191.8 bytes of body per op from 173.9). The four benchmarks and
hello by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 288.8 to 288.9 (+0.0%), `strings` 430.1 to 429.9
(-0.0%), `records` 209.4 to 208.9 (-0.2%), `json_round_trip` 838.1 to
839.6 (+0.2%), `hello` 5.42 to 5.42 (unchanged). The four judges
byte-equal, the conformance suite 55 of 55 on both tiers, the template
tier's own tests with the tier forced, 387 tests. Stage 1b next, the
read-only flag per parameter in the bytecode, then 1d.

**AW5. Stage 1b of the new backend: a read-only parameter passed
borrowed between compiled bodies.** The third item of stage 1 in AW1's
order, on 2026-10-12, measured as AW1 prescribes against stage 1c's
binary (1cfcd7d). The study's item (1b): a direct call retained every
boxed argument at the caller's `Load` and the callee released it at its
leave, a pair of count operations per argument on 2,770,541 calls with
about one and a half such arguments each, although most parameters are
only read. The flag, as AW1 (4) chose: `Code` gains `read_only`, a
Boolean per parameter, in the bytecode file (`file.rs`, format 7 to 8,
after `locals`), the binary encoding (`binary.rs`, 4 to 5), the image
(`image.rs`, image format 8 to 9, code format 15 to 16) and
`compiler/bytecode.ry`; both emitters compute it from the finished ops
(`Code::read_only_params` in `bytecode.rs`, called by the Rust
emitter's `finish`; `read_only_params` in `compiler/emit.ry`), held
equal by the compile judge: a parameter is read-only when no op writes
its slot, no `Store`, `WithSlot`, `TakeField`, `IterInit`, `Deadline`
or `MarkStack`. A `LoadMove` does not disqualify the flag, where the
study's rule had it: AU55 makes the last read of nearly every parameter
a move, so the study's rule would have flagged almost none; which moves
a borrow tolerates is the tier's question, answered below. On the
checker's own program, 2,133 of its 2,328 parameters are read-only; 177
are not for a `TakeField` (166 for that alone: a record read once
through a field on its last use, as AU55 compiles it, which a
`TakeField` that clones when the slot is borrowed could admit later)
and 29 for a `WithSlot`. The protocol in `native/codegen.rs`: the
body's signature (`direct_signature`) gains an `i64` after the direct
flag, the bits of the parameters the caller passed borrowed, which a
trampoline passes as zero; `borrowed_operands` marks, for a `Call` that
`direct_call` (the condition of AR3 and AW3, now one function for the
analysis and the emission) makes directly, the plain `Load`s and the
boxed `Const`s among its last arguments whose parameter the callee
accepts borrowed (`accepted_params`, below), stepping over a typed push
(one operand, in a register), and the mask per call; the caller's push
of a marked operand carries no reference, as a typed call's does (AU1,
AU17), and `call_direct` passes the mask, or gives the marked operands
a reference first on its path through `rt_call`, as the call through
the VM's path does when the emission does not take the direct call the
analysis expected. A moved load, the caller's last use of the slot, is
not borrowed by a direct call but moved into the callee as before: the
first build borrowed it too, as a typed call does, and measured 4.5%
more instructions on the check row (2,851.6 million against 2,729.4;
the counts and drops 321.9 to 424.4, the natives 355.9 to 385.6), the
callee's copy of the value then having two references where the moved
one had one, so that every append or update of a list or a record
passed by its last use copied it and the caller's leave freed the old
one (the measure is kept as the lane's `study/stage1b_moves_borrowed`).
Which parameters a call may pass borrowed is one rule both sides read,
`accepted_params` in `native/codegen.rs`: boxed, read-only, among the
first thirty-two, and never moved into an owned sink, that is, every
`LoadMove` of the slot (in the program as the VM prepares it, its last
reads moves, AU17) is an argument of a call through a typed entry or an
operand of a comparison, which read the value where it lies; computed
from the code object's ops and the program's tables alone, so that a
caller's compilation, on the compile thread or for an image, reads the
same bits the callee's does, which the protocol requires. The second
build accepted every boxed read-only parameter and had the callee move
conditionally, a `LoadMove` of such a parameter testing its bit to move
when the frame owned the slot or to copy with a reference when the
caller did; it measured the check row at 2,723.4 million, 0.2% under
stage 1c, and AT1's estimate 0.1% over it (kept as the lane's
`study/stage1b_conditional_moves`): AU55 makes most arguments moves,
which a direct call cannot borrow, so the pairs saved were those of
plain loads and constants alone, and the conditional move of every
flagged parameter cost about half of what they saved. The owner's
answer of 2026-10-12: refine first, accepting a borrow only for a
parameter never moved into an owned sink, then decide by the measure,
the stage reverted whole if the check row still falls by less than 0.5%
and AT1's estimate does not fall. In the callee now, a `LoadMove` of an
accepted parameter is one a typed entry or a comparison borrows, and
moves nothing; the conditional move remains as the fallback where the
analysis finds such a move owned after all (a comparison whose other
operand is typed); the leave in place releases an accepted slot only
when its bit is clear (`owned_slots` is unchanged); and every way out
that treats the frame as owned gives the borrowed slots a reference
first through the new helper `rt_retain_params` (a hand-back before
`rt_deopt`, the leave through `rt_truncate`, `rt_fail`,
`rt_check_failed`, `rt_unhandled`, and an interrupt at the exit, whose
frame the caller's `abandon` drops). The interpreter is untouched; the
template tier stays on the owned protocol and says so to
`borrowed_operands` (its switch `direct_calls`), which the first build
did not: the template tier shares the analysis, pushed the marked
operands without a reference and passed the frame owned through its own
call, a double free found by the test program's run with
`RENYI_NATIVE_TIER=template` (a segmentation fault; the same run under
a memory budget passed, the counting allocator hiding it). The tests:
`crates/renyi_vm/tests/native.rs`, two thousand turns of ten paths out
of a frame holding a borrowed parameter (a field read, two parameters
read into a text, the parameter returned, passed on borrowed again and
into a record, a record updated by `with` beside a borrowed one, a
failure caught by the caller, one propagated through `otherwise fail`,
a hand-back after the read at an overflow, a temporary beside a
borrowed argument, the reads in a loop), both tiers equal to the
interpreter, the masks of `main`'s eleven direct calls read from its IR
(1 for a record read by its fields beside a text whose move goes into
an interpolation, 1 for a text whose only move goes into `Text.length`,
passed on borrowed again by its callee, 2 for a record beside a list
passed by its last use, 0 where the parameter is returned, passed on by
a last use, constructed into a record or updated by `with`, where its
last read takes a field, or where the argument is a temporary or a last
use); and `crates/renyi/tests/sandbox.rs`, the same forty thousand
times under a memory budget of sixteen megabytes through the binary on
both tiers, and a crash inside a frame holding a borrowed parameter,
which exits with the crash. The compile tests' literal format numbers
moved up by one. The measure (callgrind under WSL on the release build
for Linux, the images built under `valgrind --tool=none`, against stage
1c's binary measured the same way in AW4, the reference AU34's
44782aa): the check row 2,729.4 to 2,719.3 million instructions (-0.4%;
the study estimated 80 million, 2.6%), the parse row 1,132.9 to 1,128.1
(-0.4%), the compile row 4,395.7 to 4,403.4 (+0.2%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6)
6.16, 3.96 and 12.04 against the gate's 3 (6.18, 3.98 and 12.02 after
stage 1c); the Rust front end's own rows within 0.2% of stage 1c's (`check` 173.6,
`parse --json` 246.7, `compile` 446.7). The helpers on the check row:
`rt_retain_params` 330,192 instructions (0.01%; the borrowed slots
given a reference before a hand-back or a leave through a helper;
763,432 in the second build), `rt_truncate` 16.0 million (0.6%),
`rt_drop_at` 201.1 million (7.4%), the last two as in stage 1c. The
check row by category, AW4's table against this one, in millions: the
generated code 1,014.6 to 1,004.1, the counts and drops 321.9 to 322.2,
the helpers' glue 120.7 to 120.7, the calls through the VM's path 7.4
to 7.4, mimalloc 146.8 to 146.9, the records 206.8 to 206.8, the
natives 355.9 to 355.9, the hashing 221.8 to 221.5, the comparisons
123.9 to 123.9, the texts and copies 120.3 to 120.3, the load 46.0 to
46.2, the iterators 40.3 to 40.3. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 1c's binary against this
one): the synchronous JIT row 5,981.1 to 5,975.3 million instructions
(-0.1%) and 8,622.5 to 8,590.5 by the estimate (-0.4%), the image row
4,376.1 to 4,358.8 (-0.4%) and 6,507.6 to 6,409.7 by the estimate
(-1.5%), the cached run 4,455.7 to 4,438.7 (-0.4%), the interpreter
11,240.4 to 11,241.1 (+0.0%); the image 9,837,312 to 9,833,784 bytes
(-0.0%, 3,528 bytes under stage 1c's, where the second build's
conditional moves had added 394,024, 4.0%). The four benchmarks and
hello by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 288.9 to 289.2 (+0.1%), `strings` 429.9 to 433.3
(+0.8%), `records` 208.9 to 209.0 (+0.0%), `json_round_trip` 839.6 to
839.6 (unchanged), `hello` 5.42 to 5.42 (unchanged). The four judges
byte-equal, the conformance suite 55 of 55 on both tiers, the template
tier's own tests with the tier forced, 389 tests. Stage 1d next, the
frame record made cheaper.

**AW6. Stage 1d of the new backend: the frame record made cheaper.**
The fourth and last item of stage 1 in AW1's order, on 2026-10-12,
measured as AW1 prescribes against stage 1b's binary (2367c63). The
study's item (1d): at every direct call, `push_frame_inline` read the
caller's grant from the frame record on top (the frames' pointer and
length loaded, the record's address computed, the grant loaded),
checked the stack's capacity for the callee's locals and wrote the
stack's length, and checked the frames' capacity for one more record,
about 45 instructions with the depth guard where the record's five
words cost a few. Three changes in `native/codegen.rs`, the interpreter
untouched and the template tier as it was (its push follows the
interpreter's protocol, clearing the callee's locals and writing the
length itself, since its callee may be template code with no prologue
of its own; a grant in a register there would change the `Entry`
signature through `Vm::run_generated`, for code that runs only until
the Cranelift code is ready, which the measure does not see): (i) the
frame's grant (decision Q1, an index into `Vm::grants`) travels in the
body's signature (`direct_signature`, an `i32` after the borrowed
bits), a direct caller passing its own, which it holds in a register
from its entry, or the helper's answer (`rt_frame_grant`) when the
callee narrows the grant, and the trampoline reading it from the record
on top; the call site writes the callee's record from the register;
(ii) the stack's length, which the call site wrote as the callee's base
plus its locals after a check of the capacity, is written by the
callee's prologue instead, after the check it makes anyway for its
locals and its deepest operand stack (AR4), the call site leaving the
length at the arguments; the exception is a boxed argument that moves
to a slot past the room the caller's prologue made (a typed parameter
before a boxed one, called at the caller's deepest operand), which
keeps the check at the site; (iii) the frames' capacity for the record
a direct call pushes is checked once, in the prologue of a body that
makes a direct call (`has_direct_call`), since the frames' length is
the same at every one of its calls, this frame on top, and the capacity
never falls. The code format is 17. On the test program of AW5,
`main`'s eleven direct calls, the IR fell from 4,699 to 4,405
instructions by a count of its lines. The tests: the program of every
shape of direct call (a callee that narrows the grant called from
`main` and from another that narrows, a callee that inherits the grant
and calls on, recursion past the frames' capacity and the depth limit,
typed parameters before a boxed one at the caller's deepest operand) in
`crates/renyi_vm/tests/native.rs` against the interpreter, in
`crates/renyi_vm/tests/template.rs` on the template tier, and in
`crates/renyi/tests/sandbox.rs` through the binary under a memory
budget of sixteen megabytes on both tiers; its `main`'s IR calls
`rt_grow_frames` and `rt_room` once each, in the prologue, and
`rt_frame_grant` twice, for the two narrowing callees. The measure
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against stage 1b's binary measured the
same way in AW5, the reference AU34's 44782aa): the check row 2,719.3
to 2,681.0 million instructions (-1.4%; the study estimated 55 million,
1.8%), the parse row 1,128.1 to 1,116.1 (-1.1%), the compile row
4,403.4 to 4,352.4 (-1.2%); the ratios by instructions against the
reference's rows (441.6, 284.7 and 365.6) 6.07, 3.92 and 11.90 against
the gate's 3 (6.16, 3.96 and 12.04 after stage 1b); the Rust front
end's own rows within 0.3% of stage 1b's (`check` 173.1, `parse --json`
246.7, `compile` 446.3). The helpers on the check row run as in stage
1b: `rt_frame_grant` 54,165 instructions (the narrowing callees'
grants), `rt_room` 17,926 and `rt_grow_frames` 1,215 (the growths, now
from the prologues), `rt_retain_params` 330,192, `rt_call` 378.1
million, `rt_truncate` 16.0 million, `rt_drop_at` 201.1 million; the
whole fall is in the generated code. The check row by category, AW5's
table against this one, in millions: the generated code 1,004.1 to
966.7, the counts and drops 322.2 to 322.2, the helpers' glue 120.7 to
120.7, the calls through the VM's path 7.4 to 7.4, mimalloc 146.9 to
146.9, the records 206.8 to 206.8, the natives 355.9 to 355.9, the
hashing 221.5 to 220.5, the comparisons 123.9 to 123.9, the texts and
copies 120.3 to 120.3, the load 46.2 to 46.2, the iterators 40.3 to
40.3. AT1's rows on the frozen self-check (`tools/measure_size.sh`
under WSL, stage 1b's binary against this one): the synchronous JIT row
5,975.3 to 5,897.8 million instructions (-1.3%) and 8,590.5 to 8,484.1
by the estimate (-1.2%), the image row 4,358.8 to 4,288.4 (-1.6%) and
6,409.7 to 6,288.7 by the estimate (-1.9%), the cached run 4,438.7 to
4,368.2 (-1.6%), the interpreter 11,241.1 to 11,241.3 (+0.0%); the
image 9,833,784 to 9,009,552 bytes (-8.4%: the bodies 8,831,433 to
7,981,545 bytes without the call sites' checks and the grant's loads,
the trampolines 156,448 to 186,616 with the grant's read). The four
benchmarks and hello by user-space instructions (`perf stat`, the JIT
synchronous, in millions): `primes` 289.2 to 265.0 (-8.4%), `strings`
433.3 to 432.5 (-0.2%), `records` 209.0 to 207.0 (-1.0%),
`json_round_trip` 839.6 to 839.7 (+0.0%), `hello` 5.42 to 5.42
(unchanged). The four judges byte-equal, the conformance suite 55 of 55
on both tiers, the template tier's own tests with the tier forced, 392
tests. Stage 1 is complete: the check row 2,889.9 million after stage 0
to 2,681.0, -7.2%, against the study's estimate of 400 million, 13%, by
the findings' counts. Next in AW1's order: the character tables and the
hasher.

**AW7. The character tables and the hasher of the VM's own.** The item
after stage 1 in AW1's order (the study's items (3b) and (3c), the
owner's answer (7)), on 2026-10-12, measured as AW1 prescribes against
stage 1d's binary (ddbf36a). Two changes to the natives and one to the
maps and sets, the generated code untouched. (i) `run_of` and
`run_until` (decision AU52, the lexer's scans over a character class)
scan with a table of 128 bits of the set's members that are texts of
one ASCII character (`ascii_members` in `natives/prelude.rs`): such an
item is a member exactly when its bit is set, since it equals a member
only when the member is that text, and any other item asks the set as
before. Measured alone, the tables took the check row from 2,681.0 to
2,621.5 million (-2.2%) and the hashing from 220.5 to 96.2, but the
natives from 355.9 to 420.7: the table was built per call over the
sixty-odd members of a class, about a thousand instructions a call,
70.0 million over `list_run_of_typed`'s 53,046 calls. So (ii) the
tables of the sets scanned last are kept, eight at most, in a
thread-local list with the set each belongs to, whose address names it
while the entry holds it (`class_table`); the VM clears the list when
it is dropped. (iii) The maps and sets of values (`Value::Map`,
`Value::Set`) hash with a hasher of the VM's own (`hashing.rs`:
`ValueHasher`, `ValueHashing`, the aliases `ValueMap` and `ValueSet`
every site uses) in place of the standard library's SipHash, whose
`write` cost a hundred instructions and more for the one to twelve
bytes of a typical key: one state word, every word written folded in by
a multiplication whose 128-bit product is folded to 64 bits, so that
every bit of the state depends on every bit written; a span of bytes
read as two overlapping words (the first and last eight, the first and
last four, or the first, middle and last byte of a shorter span,
sixteen at a time before that), its length in the multiplier, so that
spans whose overlapping reads agree ("aaaa" and "aaaaa") hash apart;
the state starts from a key drawn once per process from the standard
library's `RandomState`, so that keys chosen to collide in one run do
not collide in the next, the defence SipHash's keys give a server
against a flood of colliding keys, though a multiplication is easier to
reverse than SipHash once the key is known, a trade the owner may
revisit by a decision. The order of a map or a set is the order of
insertion (`IndexMap`), so the hash is not observable by a program;
`Value.hash()` (`vm::hash_of`) keeps the standard hasher with its fixed
keys, being a number a program reads. The tests: `hashing.rs`'s own
(spans of one character repeated zero to forty times hash apart
pairwise, as do a text and its prefix and two long texts differing in a
byte, and the key holds across hashers), `value.rs`'s set test through
`ValueSet`, the judges and the conformance suite, and the lexer's
tokens equal on a probe with text outside ASCII. The measure (callgrind
under WSL on the release build for Linux, the images built under
`valgrind --tool=none`, against stage 1d's binary measured the same way
in AW6, the reference AU34's 44782aa): the check row 2,681.0 to 2,527.2
million instructions (-5.7%), the parse row 1,116.1 to 1,040.1 (-6.8%;
the tables alone 1,082.4, -3.0%), the compile row 4,352.4 to 4,148.8
(-4.7%); the ratios by instructions against the reference's rows
(441.6, 284.7 and 365.6) 5.72, 3.65 and 11.35 against the gate's 3
(6.07, 3.92 and 11.90 after stage 1d); the Rust front end's own rows
within 0.1% of stage 1d's (`check` 173.2, `parse --json` 246.7,
`compile` 446.1). The functions on the check row, inclusive:
`list_run_of_typed` 125.2 million over 53,046 calls in stage 1d, 70.0
with the table built per call, 16.1 with the tables kept (`class_table`
2.5); the hashes of a `Value` 547,511 in stage 1d at 244 instructions
each (`hash_one` 133.9 million, SipHash's `write` 59.7 over 1,589,797
calls), 207,134 with the tables, the scans no longer hashing, and 12.3
million in all with the hasher, 59 each; the set lookups
(`get_index_of`) 429,399 to 20,129, the map lookups 126,054 as before;
`Value::eq` 42.3 million over 1,011,987 calls to 23.2 over 642,529, the
lookups' comparisons; SipHash's `write` 0.4 million left, in the VM's
own tables keyed by `usize`. The check row by category, AW6's table
against this one, in millions: the natives 355.9 to 366.9, the hashing
220.5 to 55.5, the generated code 966.7 to 966.7, the counts and drops
322.2 to 322.2, the helpers' glue 120.7 to 120.7, the calls through the
VM's path 7.4 to 7.4, mimalloc 146.9 to 147.1, the records 206.8 to
206.8, the comparisons 123.9 to 123.8, the texts and copies 120.3 to
120.3, the load 46.2 to 46.2, the iterators 40.3 to 40.3. AT1's rows on
the frozen self-check (`tools/measure_size.sh` under WSL, stage 1d's
binary against this one): the synchronous JIT row 5,897.8 to 5,880.1
million instructions (-0.3%) and 8,484.1 to 8,428.0 by the estimate
(-0.7%), the image row 4,288.4 to 4,270.5 (-0.4%) and 6,288.7 to
6,266.7 by the estimate (-0.3%), the cached run 4,368.2 to 4,350.4
(-0.4%), the interpreter 11,241.3 to 11,223.5 (-0.2%); the image
9,009,552 to 9,009,552 bytes (unchanged). The four benchmarks and hello
by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 265.0 to 265.0 (unchanged), `strings` 432.5 to
432.5 (unchanged), `records` 207.0 to 207.0 (unchanged),
`json_round_trip` 839.7 to 839.7 (unchanged), `hello` 5.42 to 5.42
(unchanged). The four judges byte-equal, the conformance suite 55 of 55
on both tiers, the template tier's own tests with the tier forced, 394
tests. Since stage 0 the check row 2,889.9 to 2,527.2 (-12.5%), since
the study's binary 3,085.3 (-18.1%). Next in AW1's order: stage 2, 2a
first (`with` by static index with the copy in place).

**AW8. Stage 2a of the new backend: the update by static index.** The
first item of stage 2 in AW1's order (the study's item (2a)), on
2026-10-12, measured as AW1 prescribes against the hasher's binary
(94089f9). A `with` ran through `rt_with` or `rt_with_slot` (decision
AU11), which popped the pairs into a vector, looked every field up by
name (`field_index`, a scan of the type's fields comparing texts),
checked the refinements and pushed the result: about a thousand
instructions a call, 212.4 million on the check row over 201,205 calls.
Where the checker typed the `with` (AU1's annotation on the op, which
pushes its expression), the generated code knows the record's type, so
`update_by_index` in `native/infer.rs` settles at compile time the type
and the index of every updated field: the op's annotation names a
record type without refinements and not `Date`, and the name of each
update is the text the `Const` at its depth pushed, found through the
analysis's states (every op between that `Const` and the `with` lies
deeper), the indices packed eight bits each with the count in the top
byte, at most seven updates; anything else keeps the helpers by name.
Built in two parts, each measured. (A) A helper that takes the type and
the packed indices (`rt_with_by_index`, `rt_with_slot_by_index`,
`Vm::op_with_by_index`): a holder that is a record of the type with
plain new values is updated at the indices, in place when held once and
in a copy otherwise, with no vector, no lookup and no refinement check;
a holder of another kind or a guarded new value goes the general way.
Both tiers call it, the template tier for good (the study's "a helper
that takes the indices"). (B) On the Cranelift tier the update compiled
into the site (`update_in_place` in `native/codegen.rs`): the pairs
boxed, the holder's tag and type and the new values' tags tested in
place, the block updated where it lies when its count is one and a copy
of it (`rt_record_copy`, the fields cloned, the holder's reference
given up) otherwise, each field's old value released in place (AU32's
sequence) and the new value moved in, each name released, the record
left where the first name lay (where the holder lay, for `With`) and
the slot left `Nothing`; the helper of (A) in a cold block for the
rest, leaving the same stack. The code format is 18. The tests: the
program of every shape (a parameter's record shared with the caller, a
record held once in its slot through a loop, field names longer than a
small text, a refined type and an update of eight fields through the
general helper, a fresh record updated each turn) in
`crates/renyi_vm/tests/native.rs` against the interpreter and in
`crates/renyi_vm/tests/template.rs` on the template tier. The measure
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against the hasher's binary measured the
same way in AW7, the reference AU34's 44782aa). With (A) alone, both
tiers on the helper: the check row 2,527.2 to 2,416.7 million
instructions (-4.4%), the parse row 1,040.1 to 994.0 (-4.4%), the
compile row 4,148.8 to 3,992.6 (-3.8%); the records category 206.8 to
142.7, the helpers' glue 120.7 to 99.5, mimalloc 147.1 to 135.6, the
counts and drops 322.2 to 315.4, the texts and copies 120.3 to 112.7;
`rt_with_slot_by_index` 72.6 million inclusive and `rt_with_by_index`
28.6 against 162.7 and 49.7 for the helpers by name, `make_mut` 41.2
(the copies of shared bases); AT1's synchronous row -1.5% (-1.4% by the
estimate). With (B), the update compiled into the site: the check row
2,527.2 to 2,370.7 (-6.2%), the parse row 1,040.1 to 974.3 (-6.3%), the
compile row 4,148.8 to 3,934.1 (-5.2%); the ratios by instructions
against the reference's rows (441.6, 284.7 and 365.6) 5.37, 3.42 and
10.76 against the gate's 3 (5.72, 3.65 and 11.35 after AW7); the Rust
front end's own rows within 0.1% of the hasher's (`check` 173.4, `parse
--json` 246.7, `compile` 446.3). The functions on the check row: with
(A), `op_with_by_index` 89.1 million inclusive over 201,205 calls (443
each, against about 1,050 by name), `make_mut` 41.2 of them, the copies
of the shared bases; with (B), the helpers by index are never called on
this row (the site's tests always pass), `rt_record_copy` 38.3 million
inclusive over 126,606 calls (the bases held by another value at the
update, 63 of 100, 302 each with the allocation and the fields'
clones), `Value`'s drop glue 14,611,263 calls to 13,282,873,
`rt_drop_at` 668,735 to 683,178 (the old field values released in place
that reach zero), the generated code 966.7 to 980.4 million with the
sequences, which the records' fall pays for nine times over; on the
frozen self-check the sequences cost what the helper saved, (A) -1.5%
and (B) -1.3% on the synchronous row, the check row deciding (AW1). The
check row by category, AW7's table against this one, in millions: the
records 206.8 to 79.9, the generated code 966.7 to 980.4, the helpers'
glue 120.7 to 92.7, mimalloc 147.1 to 135.7, the counts and drops 322.2
to 306.8, the texts and copies 120.3 to 112.7, the natives 366.9 to
366.9, the hashing 55.5 to 55.6, the calls through the VM's path 7.4 to
7.4, the comparisons 123.8 to 123.8, the load 46.2 to 46.2, the
iterators 40.3 to 40.3. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, the hasher's binary against this
one): the synchronous JIT row 5,880.1 to 5,802.0 million instructions
(-1.3%) and 8,428.0 to 8,356.3 by the estimate (-0.9%), the image row
4,270.5 to 4,140.7 (-3.0%) and 6,266.7 to 6,084.3 by the estimate
(-2.9%), the cached run 4,350.4 to 4,220.6 (-3.0%), the interpreter
11,223.5 to 11,229.0 (+0.0%); the image 9,009,552 to 9,137,440 bytes
(+1.4%). The four benchmarks and hello by user-space instructions
(`perf stat`, the JIT synchronous, in millions): `primes` 265.0 to
265.0 (unchanged), `strings` 432.5 to 432.5 (unchanged), `records`
207.0 to 207.0 (unchanged), `json_round_trip` 839.7 to 839.7
(unchanged), `hello` 5.42 to 5.42 (unchanged). The four judges
byte-equal, the conformance suite 55 of 55 on both tiers, the template
tier's own tests with the tier forced, 396 tests. Since stage 0 the
check row 2,889.9 to 2,370.7 (-18.0%), since the study's binary 3,085.3
(-23.2%). Next in AW1's order: stage 2b, `Done` without its allocation.

**AW9. Stage 2b of the new backend: a record of two fields returned as
a pair.** The second item of stage 2 in AW1's order (the study's item
(2b), `Done` without its allocation, which AW1 (5) settled as a
convention per function), on 2026-10-12, measured as AW1 prescribes
against stage 2a's binary (1ad4399). A function whose declared result
is a record of two fields (the checker's `Done`, the parser's `Parsed`,
the emitter's `Built`: 275 such functions with a code object in the
checker's program, 105 of them returning `Done` and 100 `Parsed`)
constructed it on its way out (`rt_construct`: the block allocated, the
fields moved in), and its caller took the fields out at once and freed
it: 677,653 constructs, 683,178 releases through `rt_drop_at`,
1,084,633 frees of a block (the nested ones counted) on the check row
of AW8's build. The convention per function: a function whose declared
result is a record of exactly two fields of a type without refinements
and not `Date`, with a code object, is a pair function (`pair_result`
in `native/infer.rs`); on the Cranelift tier its body answers a direct
caller with the two fields in the first two cells of where its frame
was, with `D_RETURNED` (`pair_return` in `native/codegen.rs`: the
fields' words read before the leave, the frame left, the words written,
the stack's length two above the base), and answers the trampoline's
caller, and any caller under provenance guards (the record then carries
the union of the fields' origins), with the record and `D_BOXED`, as
`Construct` would have made it; before the `Return` a `Construct` of
the type leaves its two fields as they lie (no block), a call of a pair
function passes its pair on, a load of a pair slot pushes its cells,
and anything else is a record taken apart (`rt_split_pair`: moved out
of a record held once, copied from a shared one, each field with the
record's origins under guards). On the caller's side the abstract
operand `Abs::Pair` of two cells, with the record's type, is what a
call of a pair function leaves when the op after takes the pair as it
lies (`pair_fused`: a `Store`, a `Field` or a `Return`), and before any
other op it becomes its record (`rt_pair_record`, in place), so that a
pair never lies under another operand; a `Store` of a pair into a slot
stored from pairs alone makes a pair slot (`SlotKind::Pair`: the first
field in the slot, the second in a cell of the frame past the code
object's locals, `frame_locals` in the generator; a slot stored into
with anything else holds records, `join_slot`); `LoadField` and
`TakeField` on a pair slot read or move the field's cell (a take at the
slot's last read leaves both cells `Nothing`); `Load` and `LoadMove`
push both cells, copied with a reference each or moved at the last
read; a `Field` on a pair leaves the field in the lower cell and
releases the other; a `WithSlot` on a pair slot makes it boxed from
there on. The hand-back (`rt_deopt`) rebuilds a pair on the stack from
its two cells and a pair slot's record from the slot and its second
cell when the slot holds its pair (an owned word per pair slot in the
point's buffer), and cuts the cells past the locals off. The entry at a
loop header from a frame the interpreter hands over: `rt_pair_fits`
checks that each pair slot holds a plain record of the type or
`Nothing` (else the entry is refused with `D_STAY`), the frame is
widened by the pair cells, and `rt_pair_resume` takes each record apart
into its cells; on the way the prologue's write of the stack's length
before the header dispatch, which lengthened the stack of an entry
refused after it (a latent defect for the expanded bodies of AU50, hit
here through the second loop of a function whose first loop's finished
range `rt_resume_range` refuses), is gone: the start path writes the
length after the dispatch, the resume path once its checks have passed.
The template tier calls a pair function through its trampoline, which
answers the record (`analyse` without pairs), as do the expanded
callees of AU50 and the typed test. The code format is 19 (the pair
kinds with their type in the hand-back points, which
`tools/image_census.py` reads). Built in two parts, each measured. (A)
The pair at the call alone. The survey of the checker's bytecode: of
the 782 calls of a pair function, 312 are followed by a `Store`, 100 by
a `Return`, 34 by a `Field`, and 244 by the `PopHandler` of an
`otherwise` (`let done be check(...) otherwise fail`, the idiom of the
checker's failure propagation); of the pair functions' returns, 423
return a `Construct` of the type, 100 a call of a pair function, 88 a
load; 305 slots are stored from pair calls alone. Measured, the check
row 2,370.7 to 2,342.0 million instructions (-1.2%) against the study's
estimate of 4.9%: `rt_construct` 677,653 to 410,681 calls, but
`rt_pair_record` 176,261 calls, two pairs of three made into their
record by the caller after all, most at the `otherwise`: the compiler
closes the handled region after the call with `PopHandler` and tests
the value (`JumpIfFailure` to the fallback, then a `Jump` to the end),
and a pair before the test was boxed for it. (B) The pair past the
test. `otherwise_end` in `native/infer.rs` recognises the three ops
after a call that ends a region, and a pair the call leaves takes the
jump to the end at once (the analysis flows the state there with the
region closed, `pair_bypass` in the generator jumps there), so that the
test is reached through its handler alone, with a failure on top
(`handler_only`: the region's `PopHandler` before it on no path), and
its jump is always taken; `pair_fused` looks at the op at the end.
Where the fallback falls through with a value (`otherwise Step(...)`),
the states join boxed and the pair becomes its record at the edge, as
before. The tests: the program of every shape (a pair stored and taken
apart, read as a field at once, relayed, kept whole in a list, held
across a loop whose Integer leaves the machine word and hands the frame
back, kept past `otherwise fail` and returned past one, taken with a
fallback value, on the success path and the failure path; a record of
three fields) in `crates/renyi_vm/tests/native.rs` against the
interpreter with everything compiled, and in
`crates/renyi_vm/tests/promotion.rs` promoted from template code (the
trampoline's path, and the entry at a loop header with a pair slot
live). The measure (callgrind under WSL on the release build for Linux,
the images built under `valgrind --tool=none`, against stage 2a's
binary measured the same way in AW8, the reference AU34's 44782aa).
With (A) alone: the check row 2,370.7 to 2,342.0 million instructions
(-1.2%), the parse row 974.3 to 966.4 (-0.8%), the compile row 3,934.1
to 3,874.0 (-1.5%); the records category 79.9 to 57.7, the generated
code 980.4 to 985.4, the helpers' glue 92.7 to 85.3, mimalloc 135.7 to
130.4, the counts and drops 306.8 to 300.0; `rt_construct` 677,653 to
410,681 calls, `rt_pair_record` 176,261, `rt_split_pair` 23,596,
`rt_drop_at` 683,178 to 617,399, `Composite::free` 1,084,633 to 991,989
(the nested ones counted), `mi_malloc` 2,180,274 to 2,087,608; AT1's
synchronous row -0.4% (-0.9% by the estimate). With (B), the pair past
the test: the check row 2,370.7 to 2,301.9 (-2.9%), the parse row 974.3
to 941.6 (-3.4%), the compile row 3,934.1 to 3,834.3 (-2.5%); the
ratios by instructions against the reference's rows (441.6, 284.7 and
365.6) 5.21, 3.31 and 10.49 against the gate's 3 (5.37, 3.42 and 10.76
after AW8); the Rust front end's own rows within 0.2% of stage 2a's
(`check` 173.2, `parse --json` 246.7, `compile` 446.3). The functions
on the check row with (B): `rt_construct` 677,653 to 410,681 calls
(71.8 million inclusive), `rt_pair_record` 67,670 (6.7),
`rt_split_pair` 23,596 (5.5), `rt_drop_at` 683,178 to 529,303 (152.9),
`Composite::free` 1,084,633 to 650,434, `mi_malloc` 2,180,274 to
1,978,953. The check row by category, AW8's table against this one, in
millions: the records 79.9 to 57.6, the generated code 980.4 to 982.2,
the helpers' glue 92.7 to 85.3, mimalloc 135.7 to 121.1, the counts and
drops 306.8 to 279.3, the texts and copies 112.7 to 109.0, the natives
366.9 to 365.0, the hashing 55.6 to 55.6, the calls through the VM's
path 7.4 to 6.9, the comparisons 123.8 to 123.8, the load 46.2 to 46.5,
the iterators 40.3 to 40.3. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 2a's binary against this
one): the synchronous JIT row 5,802.0 to 5,775.8 million instructions
(-0.5%) and 8,356.3 to 8,299.5 by the estimate (-0.7%), the image row
4,140.7 to 3,989.5 (-3.7%) and 6,084.3 to 5,846.3 by the estimate
(-3.9%), the cached run 4,220.6 to 4,069.4 (-3.6%), the interpreter
11,229.0 to 11,229.7 (+0.0%); the image 9,137,440 to 9,554,000 bytes
(+4.6%). The four benchmarks and hello by user-space instructions
(`perf stat`, the JIT synchronous, in millions): `primes` 265.0 to
265.0 (unchanged), `strings` 432.5 to 432.5 (unchanged), `records`
207.0 to 207.0 (unchanged), `json_round_trip` 839.7 to 839.7
(unchanged), `hello` 5.42 to 5.43 (+0.2%). The four judges byte-equal,
the conformance suite 55 of 55 on both tiers, the template tier's own
tests with the tier forced, 398 tests. Since stage 0 the check row
2,889.9 to 2,301.9 (-20.3%), since the study's binary 3,085.3 (-25.4%).
A candidate seen on the way, not decided: `rt_resume_range` refuses a
finished range iterator, so a function with two loops in sequence is
never entered at the second header from the interpreter. Next in AW1's
order: stage 2c, the frees in place.

**AW10. Stage 2c of the new backend: the frees in place.** The third
item of stage 2 in AW1's order (the study's item (2c)), on 2026-10-12,
measured as AW1 prescribes against stage 2b's binary (e87f3e1). The
last holder of a record freed its block through Rust's drop glue:
`Composite::free` dropped the fields as a slice, a call of the glue of
`Value` per field, which matched the tag through a jump table and, for
most fields, did nothing; a list's last holder did the same for its
elements through the drop of `Rc<Vec<Value>>`; `rt_drop_at`, which the
generated code calls for the last reference it releases in place, and
the stack's truncate went through the glue for the value itself. The
glue was called 11,647,789 times on the check row, 123.5 million
instructions self, from `Rc<Vec<Value>>`'s drop (3,805,425 of the
calls), `Composite::free` (2,272,022), `rt_compare` (1,225,584),
`call_pure_from_stack` (795,618), `Vm::binary` (559,604), `rt_drop_at`
(529,303), `rt_iter_init` (477,068), `rt_call_typed` (423,641) and the
rest. The checker's run on `bodies.ry`, instrumented for the day: of
the 3,203,871 fields the records' frees dropped, 708,069 were plain
values and 907,772 small Integers (a tag test's worth each), 1,151,018
counted blocks with another holder (a count to lower: 625,807 lists,
316,473 records, 199,026 variants) and 437,012 the last holder's
(202,092 records, 159,088 variants, 70,614 lists, 4,199 texts); of the
3,393,831 elements of the 82,713 lists the VM's own sites freed,
990,692 were plain, 2,197,214 shared blocks (2,189,471 of them records:
the checker's lists of bindings, copied when appended) and 192,294 the
last holder's. The study had the walk in the generated code; it is in
the VM's Rust, `release_in_place` in `value.rs`, one loop for every
site and no code grown, with the generated code's cost per value (its
release in place, decision AU32): a tag test for a plain value, the
count lowered in place for a shared block, and only the last holder
going further: of a record or a variant to `Composite::free_block`,
which releases the fields the same way and frees the block; of a list
to its elements released the same way and the vector freed empty; of
anything else, and a big Integer, to `drop_in_place`, the general way.
The first word of an `Rc` allocation is its strong count, which the
generated code's release relies on already, and the VM takes no weak
references. Built twice: the first build had one function out of line
for every value, and its prologue, paid by every plain field and
element, cost the check row 3.4% more than the glue (2,380.7 million);
the second inlines the fast path at every site (`#[inline(always)]`:
the tag test, the small Integer, the count lowered) and keeps the last
holder's work out of line (`release_last`), as the generated code's
release does. `rt_drop_at` and `Pinned::truncate_values` go through it,
so the generated code of both tiers and every frame's leave do; the
interpreter's other drops and the natives' stay Rust's, the glue's
remaining calls being stage 3's (the natives' entry and the glue). No
test is added: every program frees its values, and the corpus, the
conformance suite on both tiers, the sandbox's memory budget (the frees
go through the counting allocator as before) and the four judges hold
the change. What the measure found: by instructions the walk costs
nearly what the glue cost. The check row 2,301.9 to 2,296.2 million
(-0.2%) against the study's estimate of 3.2%: the glue's calls
11,647,789 to 6,030,437, but the counts and drops category 279.3 to
219.1 while the VM's own, where the table puts `value.rs`, 28.6 to
83.0, the two together 307.9 to 302.1 (-1.9%, the row's whole fall);
`rt_drop_at` 529,303 calls, 152.9 million inclusive to 148.4. The
frees' cost is the inspection of every field and element and the
deallocation, which no walk removes, not the glue's dispatch, which the
study had taken for it. What the walk does remove is the glue's
indirect branch per field: on the frozen self-check the synchronous JIT
row's indirect branches 17.3 million to 13.3 (-23.4%), its
mispredictions 32.5 to 32.4, and the row's estimate 8,299.5 to 8,192.5
million (-1.3%) on -0.4% by instructions; the stage stays by AT1's rule
(the estimate falls) and by the owner's rule for a marginal stage (the
check row within 0.5%, the estimate down). The measure in full
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against stage 2b's binary measured the
same way in AW9, the reference AU34's 44782aa): the check row 2,301.9
to 2,296.2 million instructions (-0.2%), the parse row 941.6 to 939.5
(-0.2%), the compile row 3,834.3 to 3,809.2 (-0.7%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6)
5.20, 3.30 and 10.42 against the gate's 3 (5.21, 3.31 and 10.49 after
AW9); the Rust front end's own rows within 0.2% of stage 2b's (`check`
173.5, `parse --json` 246.7, `compile` 446.2). The functions on the
check row: the glue of `Value` 11,647,789 to 6,030,437 calls (123.5 to
62.6 million self), `release_last` 928,387 calls,
`Composite::free_block` 650,434 (153.6 million inclusive, against
`Composite::free`'s 650,434 calls before), `Rc<Vec<Value>>`'s drop
201,340 calls as before but calling the glue 3,805,425 to 886,989 times
(the vector empty when its last holder released it in place),
`rt_drop_at` 529,303 calls (148.4 million inclusive), `mi_malloc`
1,978,953 to 1,978,953, `mi_free` 1,777,179 to 1,777,179. The check row
by category, AW9's table against this one, in millions: the counts and
drops 279.3 to 219.1 and the VM's own 28.6 to 83.0, the records 57.6 to
57.6, the generated code 982.2 to 982.2, the helpers' glue 85.3 to
85.3, mimalloc 121.1 to 121.1, the texts and copies 109.0 to 109.0, the
natives 365.0 to 365.0, the hashing 55.6 to 55.6, the calls through the
VM's path 6.9 to 6.9, the comparisons 123.8 to 123.8, the load 46.5 to
46.5, the iterators 40.3 to 40.3. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 2b's binary against this
one): the synchronous JIT row 5,775.8 to 5,750.6 million instructions
(-0.4%) and 8,299.5 to 8,192.5 by the estimate (-1.3%), its indirect
branches 17.3 to 13.3 million (-23.4%) and its mispredictions 32.5 to
32.4, the image row 3,989.5 to 3,972.3 (-0.4%) and 5,846.3 to 5,773.1
by the estimate (-1.3%), the cached run 4,069.4 to 4,052.1 (-0.4%), the
interpreter 11,229.7 to 11,181.9 (-0.4%); the image 9,554,000 to
9,554,000 bytes (unchanged: no generated code changed). The four
benchmarks and hello by user-space instructions (`perf stat`, the JIT
synchronous, in millions): `primes` 265.0 to 265.0 (unchanged),
`strings` 432.5 to 431.9 (-0.1%), `records` 207.0 to 204.2 (-1.4%),
`json_round_trip` 839.7 to 841.6 (+0.2%), `hello` 5.43 to 5.43
(unchanged). The four judges byte-equal, the conformance suite 55 of 55
on both tiers, the template tier's own tests with the tier forced, 398
tests. Since stage 0 the check row 2,889.9 to 2,296.2 (-20.5%), since
the study's binary 3,085.3 (-25.6%); stage 2 in all, 2,527.2 to
2,296.2. Next in AW1's order: stage 3, the natives' entry and the glue,
where the glue's remaining 6,030,437 calls are.

**AW11. Stage 3a of the new backend: the natives' in-place entry.** The
first item of stage 3 in AW1's order (the study's item (3a); its items
(3b) and (3c), the character tables and the hasher, are AW7), on
2026-10-12, measured as AW1 prescribes against stage 2c's binary
(f48282f). A pure native was called two ways from the generated code.
Through `rt_call_typed` when it carried a typed entry (decision AU1):
the helper looked the entry up, matched its kind, called it on the
arguments where they lay, wrote a scalar to the out slot or pushed a
value through `typed_value_answer`, consumed the arguments by the mask
and tested the memory budget, 42.0 million instructions self on 527,824
calls of the check row beside the entries' own work, and 19.0 million
more on the 239,842 values answered. Through `rt_call_pure` otherwise:
`call_pure_from_stack` drained the arguments into the scratch vector,
ran the native on them through `run_native` and answered through
`Result` and `status`, 47.2 million self on 341,454 calls (`append`
187,030, `slice` 94,734, `with_at` 22,718, `quotient` 13,677,
`append_all` 12,847) with a copy of the arguments, the vector's drop
glue and the glue of `Value` on every argument (795,618 of the glue's
6,030,437 calls) beside, and `status` 53.3 million on 1,567,934 calls
from every helper that answers a value. The two `contains` went through
AU34's lean helpers. The study's item (3a) prescribed a second entry
per native answering in place, and the owner chose it through AK1's
registration API over lean helpers for the hot ones only (AW1, answer
6). `InPlace` in `extension.rs` is that entry. `in_place!(Int:
list_length_typed)` makes one from a typed entry, for the four kinds,
and `in_place!(takes list_append_in_place)` from a function `fn(&mut
[Value], u32) -> Option<Value>` that takes its arguments
(`take_or_clone` of `natives`), given the mask of those the caller
still holds, which it copies rather than updates. The macro writes an
`extern "C"` shim, `fn(vm, at, count, borrowed, pc) -> status`, which
reads or takes the arguments where they lie on the stack, releases the
ones the call owns (a taken one is `Nothing` there, a tag test), writes
the answer over the first and answers `CONTINUE`, `FAILURE` for a
failure value, `INTERRUPT` with the interrupt parked, or `DECLINED`
with nothing touched. `Native::with_in_place` registers it, the
registry holds it to the declaration as it holds a typed entry (no
`needs`), and the VM keeps the shims in a table per function
(`NativeState::in_place`), which the generated code of both tiers calls
directly on one to thirty-two arguments: the Cranelift tier in
`call_in_place` (the shim's address loaded from the table and called
indirectly, a scalar answer read from the boxed value in place, the
typed call as the fallback when the entry declines), the template tier
in its `call_in_place` with the arguments in the convention's
registers, and the interpreter in `Vm::call_in_place` before the typed
entry. A moved operand of an entry that takes is the call's to consume,
so that `append` on a list the call owns pushes in place as the flat
path did, where a typed call lends it (decision AU17): `taking_entry`
in the borrow analysis. The standard library registers 26 entries:
`quotient`, `at_least` and `at_most` of Integer; `length`, `is_empty`,
`contains`, `starts_with` and `ends_with` of Text; `length`,
`is_empty`, `first`, `last`, `with_at`, `slice`, `append`,
`append_all`, `contains`, `run_of`, `run_until`, `join` and `sum` of
List; `length`, `get` and `contains_key` of Map; `length` and
`contains` of Set (`with_at`, `append` and `append_all` take; `slice`
and `quotient` through the read-only entries `list_slice_typed` and
`integer_quotient_typed`, written for the stage). `List.at` keeps
AU34's read in place with the typed call as its fallback, and AU34's
lean helpers `rt_text_contains` and `rt_list_contains` go, the entries
having taken their place (code format 20). Built twice. The first build
gave every entry one shape: a guard tested, the arguments released by a
loop over them, the memory budget tested after every answer, and in the
generated code the length stored twice and the status tested twice
(answered, then declined or failed). The check row fell 3.9% (2,296.2
to 2,207.4 million), but the `strings` benchmark, whose digits loop
calls `Text.contains` 1,188,895 times, rose 12.2% (431.9 to 484.6
million): the shims cost 93.3 million self on its 1,288,897 calls
against the lean helper's 23.8 million on its 1,188,895, 72
instructions a call against 20, and the generated code (every chunk of
the arena) 109.8 to 121.1 million. The second build gives a scalar
entry that reads its operands the lean helper's shape, which the
owner's choice did not ask it to give up: no guard tested (a typed
entry answers on plain values alone, a guarded value matching no plain
pattern, which `rt_call_typed` relies on too), no budget tested (a
scalar answer retains nothing and the arguments released only lower the
memory in use; the budget is tested at the next helper that allocates,
as the lean helpers had it), the arguments the call owns released by a
loop over the bits set (`release_owned`, one test when there are none),
and in the generated code one test of the status after the call with
the length stored for the answer alone; an entry that answers a value
or takes its operands keeps the failure test and the budget, and the
generated code tests its status as before (`Entry::Takes` tells the
tiers). The tests: `tests/extension.rs` registers an in-place entry
from `twice`'s typed entry and a taking one on a new native `grow`,
which grows a list the call owns in place to a thousand items on both
tiers and copies one the caller keeps, and the check refuses an
in-place entry on a function with `needs`; the corpus, the conformance
suite on both tiers and the four judges hold the standard library's
entries. A correction to AW10 on the way: its self counts of the glue
of `Value`, 123.5 to 62.6 million, were summed over the annotator's
whole listing, whose source listings repeat the table's counts and add
the callee lines; over the function table alone they are 111.8 to 50.8
million (the calls as AW10 states them), and every self count here is
summed that way. What the measure found: the check row 2,296.2 to
2,203.9 million (-4.0%); the shims 1,050,545 calls, 133.2 million self
with the entries' bodies inlined into them (`list_append`,
`list_slice`, `list_length_typed` and the rest no longer appear as
functions; `list_join_typed` 97,732 calls and `map_get_typed` 121,757
do); on `strings` the shims 68.0 million self on 1,288,897 calls (53
instructions a call) and the generated code 115.9 million, the
benchmark +5.0% against stage 2c's lean helper: what remains over the
helper's 20 is the uniform call of five arguments, the shim's prologue,
which keeps the operands across the entry's call, and the answer boxed
and read back, which a scalar shim of two arguments with the entry
inlined would recover, an item for a later stage of the round;
`records` -17.4%, its lists appended in place through the taking entry.
The measure in full (callgrind under WSL on the release build for
Linux, the images built under `valgrind --tool=none`, against stage
2c's binary measured the same way in AW10, the reference AU34's
44782aa): the check row 2,296.2 to 2,203.9 million instructions
(-4.0%), the parse row 939.5 to 905.1 (-3.7%), the compile row 3,809.2
to 3,672.4 (-3.6%); the ratios by instructions against the reference's
rows (441.6, 284.7 and 365.6) 4.99, 3.18 and 10.04 against the gate's 3
(5.20, 3.30 and 10.42 after AW10); the Rust front end's own rows within
0.1% of stage 2c's (`check` 173.3, `parse --json` 246.7, `compile`
446.6). The functions on the check row: the shims 1,050,545 calls,
133.2 million self; `rt_call_typed` 527,824 to 1,908 calls (42.0 to 0.1
million self), `typed_value_answer` 239,842 to 1,896 (19.0 to 0.1),
`rt_call_pure` 341,454 to 10,448, `call_pure_from_stack` 341,454 to
10,448 (47.2 to 1.3), `status` 1,567,934 to 1,236,928 (53.3 to 42.1),
`into_plain` 816,882 to 816,882 (27.0 to 27.0), the glue of `Value`
6,030,437 to 3,933,657 (50.8 to 33.3), `mi_malloc` 1,978,953 to
1,978,955, `mi_free` 1,777,179 to 1,777,181; `list_append` 187,030 to 0
calls, `list_slice` 94,734 to 0 (both inlined into their shims now),
`map_get_typed` 121,757 to 121,757, `list_join_typed` 97,732 to 97,732
(`list_length_typed`, 76,950 calls before, and `text_contains_typed`,
154,702, are inlined into their shims). The check row by category,
AW10's table against this one, in millions: the natives 365.0 to 307.3,
the helpers' glue 85.3 to 74.0, the counts and drops 219.1 to 190.2,
the VM's own 83.0 to 89.6, the records 57.6 to 57.6, the generated code
982.2 to 986.4, mimalloc 121.1 to 121.0, the texts and copies 109.0 to
103.8, the hashing 55.6 to 55.8, the calls through the VM's path 6.9 to
6.9, the comparisons 123.8 to 123.8, the load 46.5 to 46.4, the
iterators 40.3 to 40.3. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 2c's binary against this
one): the synchronous JIT row 5,750.6 to 5,613.6 million instructions
(-2.4%) and 8,192.5 to 8,019.3 by the estimate (-2.1%), its indirect
branches 13.3 to 12.7 million (-4.3%) and its mispredictions 32.4 to
31.9, the image row 3,972.3 to 3,828.9 (-3.6%) and 5,773.1 to 5,617.0
by the estimate (-2.7%), the cached run 4,052.1 to 3,908.7 (-3.5%), the
interpreter 11,181.9 to 11,098.7 (-0.7%); the image 9,554,000 to
9,601,808 bytes (+0.5%). The four benchmarks and hello by user-space
instructions (`perf stat`, the JIT synchronous, in millions): `primes`
265.0 to 265.2 (+0.1%), `strings` 431.9 to 453.7 (+5.0%), `records`
204.2 to 168.7 (-17.4%), `json_round_trip` 841.6 to 841.2 (within
0.05%), `hello` 5.43 to 5.44 (+0.2%). The four judges byte-equal, the
conformance suite 55 of 55 on both tiers, the template tier's own tests
with the tier forced, 399 tests. Since stage 0 the check row 2,889.9 to
2,203.9 (-23.7%), since the study's binary 3,085.3 (-28.6%). Next in
AW1's order: stage 3b, `status` and `into_plain` off the path (the
study's item (3d)), then stage 4.

**AW12. Stage 3b of the new backend: `status` and `into_plain` off the
path.** The last item of stage 3 in AW1's order, on 2026-10-12,
measured as AW1 prescribes against stage 3a's binary (9828e2e). Every
helper that answered a value went through `status`: a `Result` built
and matched, the value pushed and tested for a failure, 1,236,928 calls
on the check row after AW11 (42.1 million instructions self), from
`rt_construct` (410,681 calls, 34.1 million self),
`rt_construct_variant` (280,398, 23.3), `rt_compare`'s general path
(279,802 calls through `status`, of the helper's 521,364),
`rt_list_push` (158,109, with `op_list_push` 15.6 million) and the
rest. And every value that might carry a guard went through
`into_plain` out of line, 816,882 calls (27.0 million), from
`op_list_push` (twice per push), `iterate` and `iterator_of` (once each
per loop), and through `guarded` (263,285 calls, 5.0 million), where
nearly every value is plain and nearly every origin is none: a call, a
match and a move each, for a test of one byte. The stage takes the glue
off the path where the answer cannot fail or the value is plain. (1)
`Value::into_plain` and `Value::guarded` test in place and keep the
unwrapping and the wrapping out of line (`unguarded`, `with_origins`),
and `origins` and `plain` are inlined. (2) A `Construct` of a plain
record type (a record without a refinement to check, not the date,
whose construction validates: `plain_record_type`) and a
`ConstructVariant` with fields of a type without refinements
(`plain_variant_type`) cannot fail, so both tiers call
`rt_construct_plain` and `rt_construct_variant_plain`, which build the
value in place of its fields and push it, with no `Result`, no status
to test and no pc; under a provenance guard they lift the fields'
origins onto the value as the general helpers do; the interpreter's own
path and the general helpers are unchanged. (3) `rt_list_push` pushes a
plain item onto a plain list where the list lies, one tag test each,
with no unwrapping, no origins and no `Result` between; a guarded value
or a holder that is no list goes through `op_list_push` as before. (4)
`rt_compare`'s general path pushes its boxed answer itself. (5)
`iterator_of` takes a plain list as it is, with no origins, no
unwrapping, no special and no listing. Code format 21 (two helpers
more). The tests: the corpus, the conformance suite on both tiers, the
four judges and `tests/guards.rs` (the guarded paths of the helpers)
hold the stage, with a new test in `tests/native.rs`,
`plain_builders_lift_the_origins_of_guarded_fields_as_the_general_helpers_do`,
which runs a guarded program (records and variants with fields built
from the secret, collected lists, loops over a guarded list and a plain
one) on machine code against the interpreter. What the measure found:
the check row 2,203.9 to 2,099.1 million (-4.8%); `status` 1,236,928 to
108,008 calls (42.1 to 3.7 million self), `into_plain` 816,882 to 0
(27.0 to 0.0), `guarded` 263,285 to 0, `rt_construct` 410,681 to 28
with `rt_construct_plain` 410,653 (26.7 million self against
`rt_construct`'s 34.1 before), `rt_construct_variant` 280,398 to 42
with `rt_construct_variant_plain` 280,356 (19.1 against 23.3),
`rt_list_push` 3.2 to 8.3 million self with `op_list_push` 158,109 to 0
calls, `iterator_of` 19.1 to 11.2 million with `iterate` 238,534 to 0
calls; the helper glue category 74.0 to 3.7 million, the records 57.6
to 46.0, the iterators 40.3 to 25.3, the comparisons 123.8 to 131.6,
the generated code 986.4 to 981.3. The measure in full (callgrind under
WSL on the release build for Linux, the images built under `valgrind
--tool=none`, against stage 3a's binary measured the same way in AW11,
the reference AU34's 44782aa): the check row 2,203.9 to 2,099.1 million
instructions (-4.8%), the parse row 905.1 to 878.7 (-2.9%), the compile
row 3,672.4 to 3,531.7 (-3.8%); the ratios by instructions against the
reference's rows (441.6, 284.7 and 365.6) 4.75, 3.09 and 9.66 against
the gate's 3 (4.99, 3.18 and 10.04 after AW11); the Rust front end's
own rows within 0.1% of stage 3a's (`check` 173.4, `parse --json`
246.7, `compile` 446.2). The functions on the check row: `status`
1,236,928 to 108,008 calls (42.1 to 3.7 million self), `into_plain`
816,882 to 0 (27.0 to 0.0; `unguarded` 0 calls), `guarded` 263,285 to 0
(5.0 to 0.0; `with_origins` 0), `rt_construct` 410,681 to 28 (71.8 to
0.0 million inclusive) and `rt_construct_plain` 410,653 (50.4
inclusive, 26.7 self), `rt_construct_variant` 280,398 to 42 (46.5 to
0.0) and `rt_construct_variant_plain` 280,356 (32.8 inclusive, 19.1
self), `rt_list_push` 158,109 to 158,109 (51.0 to 20.9 inclusive) with
`op_list_push` 158,109 to 0, `rt_compare` 521,364 to 521,364 (127.5 to
125.8 inclusive), `rt_iter_init` 74.1 to 43.3 inclusive with `iterate`
238,534 to 0 calls, the shims 1,050,545 calls (133.2 million self), the
glue of `Value` 3,933,657 to 3,775,548 (33.3 to 32.5), `mi_malloc`
1,978,955 to 1,978,955, `mi_free` 1,777,181 to 1,777,181. The check row
by category, AW11's table against this one, in millions: the natives
307.3 to 297.2, the helpers' glue 74.0 to 3.7, the counts and drops
190.2 to 189.4, the VM's own 89.6 to 90.0, the records 57.6 to 46.0,
the generated code 986.4 to 981.3, mimalloc 121.0 to 121.1, the texts
and copies 103.8 to 103.8, the hashing 55.8 to 55.7, the calls through
the VM's path 6.9 to 6.9, the comparisons 123.8 to 131.6, the load 46.4
to 46.4, the iterators 40.3 to 25.3. AT1's rows on the frozen
self-check (`tools/measure_size.sh` under WSL, stage 3a's binary
against this one): the synchronous JIT row 5,613.6 to 5,456.5 million
instructions (-2.8%) and 8,019.3 to 7,800.2 by the estimate (-2.7%),
its indirect branches 12.7 to 11.9 million (-6.6%) and its
mispredictions 31.9 to 31.9, the image row 3,828.9 to 3,694.0 (-3.5%)
and 5,617.0 to 5,486.6 by the estimate (-2.3%), the cached run 3,908.7
to 3,774.1 (-3.4%), the interpreter 11,098.7 to 11,204.0 (+0.9%, the interpreter's own
paths with the tests inlined, not measured further: the tiers' rows
are AT1's measure); the
image 9,601,808 to 9,567,648 bytes (-0.4%). The four benchmarks and
hello by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 265.2 to 265.2 (unchanged), `strings` 453.7 to
450.7 (-0.7%), `records` 168.7 to 157.1 (-6.9%), `json_round_trip`
841.2 to 841.1 (within 0.05%), `hello` 5.44 to 5.44 (unchanged). The
four judges byte-equal, the conformance suite 55 of 55 on both tiers,
the template tier's own tests with the tier forced, 400 tests. Since
stage 0 the check row 2,889.9 to 2,099.1 (-27.4%), since the study's
binary 3,085.3 (-32.0%); stage 3 in all, 2,296.2 to 2,099.1. Next in
AW1's order: stage 4, two values of a field-less variant type compared
in place, `match` by a table, the iterator in two slots.

**AW13. Stage 4a of the new backend: `is` and `is not` answered in
place, and two values of a variant type compared in the generated
code.** The first item of stage 4 in AW1's order, on 2026-10-12,
measured as AW1 prescribes against stage 3b's binary (1b25b85). The
check row's 521,364 comparisons went through `rt_compare` (66.8 million
instructions self, 125.8 inclusive), whose fast path answered small
Integers, texts, Booleans, `Nothing` beside a value and two variants
without fields of a type without its own `equals` (199,851 calls of
`has_declared_equals`), and whose general path took every other `is`
and `is not` (309,995 calls of `binary_values`, 30,193 of them
`rt_binary`'s): the operands popped, `binary_values`'s own fast arms
tried again, `binary` (9.1 million self), `equal` (10.6) and at last
`Value::eq` (23.1 million self over 644,592 calls, 279,802 of them from
`equal`, the rest the natives' and the hashed lookups'), the Boolean
boxed, pushed and unboxed by the generated code: about 250 instructions
around an equality of about 35. The stage: (1) `compare_in_place`
answers `is` and `is not` on any operands as structural equality,
`Value::eq` on the plain values, unless the left operand's type
declares its own `equals`, which the general path keeps running
(`has_declared_equals` inlined); an ordering is still answered for two
small Integers, texts or Booleans and takes the general path otherwise.
(2) The generated code of both tiers compares two values the checker
typed as one variant type without its own `equals` and with at most 64
variants (`sum_compared` in `native/infer.rs`, which also gives the
tags that name a variant with fields, one bit per tag) in place: when
both lie as variants (a guarded value does not, and takes the helper),
the type and the tag their blocks hold are compared; different ones
answer, and the same tag answers when it names a variant without fields
and goes to the helper otherwise, which compares the fields, with the
operands untouched; the answer goes to its register (on the template
tier to the out slot, then boxed where the left operand lay), and the
operands not borrowed (AU13's masks) are released in place
(`compare_sum_variants` in `native/codegen.rs` and in
`native/template/mod.rs`, beside AU26's small texts and AU45's small
Integers; the template assembler gained the 64-bit `bt`). The first
build compared only the types none of whose variants has a field, and
its measure found that path taken by 238 of the check row's
comparisons: the checker compares `Ty` and `TokenKind` values, and both
types have variants with fields beside the field-less ones it mostly
compares; the second build is the one measured. The tests: the corpus,
the conformance suite on both tiers, the four judges, AU13's comparison
test and a new one in `tests/native.rs` and `tests/template.rs`,
`variants_compare_in_place_by_tag_and_fielded_ones_through_the_helper`,
which runs field-less variants in both orders and with both operators,
different tags and the same tag of a type with fielded variants, the
same tag with fields, a guarded one read out of a guarded record, a
`maybe`, a type with its own `equals` and lists on machine code against
the interpreter. What the measure found: the check row 2,099.1 to
1,995.8 million (-4.9%); `rt_compare` 521,364 to 43,402 calls (66.8 to
3.5 million self, 125.8 to 5.1 inclusive), `binary_values` 309,995 to
30,193 (12.3 to 1.6), `binary` 279,802 to 0 (9.1 to 0.0), `equal`
279,802 to 0 (10.6 to 0.0), `has_declared_equals` 199,851 to 0
(inlined), `Value::eq` 644,592 to 414,005 (23.1 to 15.4); the
comparisons category 131.6 to 34.5 million, the hashing and equality
55.7 to 48.3, the counts and drops 189.4 to 173.8, the generated code
981.3 to 998.0. The measure in full (callgrind under WSL on the release
build for Linux, the images built under `valgrind --tool=none`, against
stage 3b's binary measured the same way in AW12, the reference AU34's
44782aa): the check row 2,099.1 to 1,995.8 million instructions
(-4.9%), the parse row 878.7 to 827.8 (-5.8%), the compile row 3,531.7
to 3,260.6 (-7.7%); the ratios by instructions against the reference's
rows (441.6, 284.7 and 365.6) 4.52, 2.91 and 8.92 against the gate's 3
(4.75, 3.09 and 9.66 after AW12); the Rust front end's own rows within
0.1% of stage 3b's (`check` 173.5, `parse --json` 246.7, `compile`
446.4). The functions on the check row: `rt_compare` 521,364 to 43,402
calls (66.8 to 3.5 million self, 125.8 to 5.1 inclusive),
`has_declared_equals` 199,851 calls (3.4 million self) to none of its
own, inlined, `binary_values` 309,995 to 30,193 (48.8 to 2.3
inclusive), `binary` 279,802 to 0 (9.1 to 0.0 self), `equal` 279,802 to
0 (20.1 to none inclusive), `Value::eq` 644,592 to 414,005 (23.1 to
15.4 self), `Vm::compare` (the sort's) 366,276 to 366,276 (17.6 to
17.6), `rt_binary` 30,193 to 30,193, the shims 1,050,545 calls (133.2
million self), the glue of `Value` 3,775,548 to 2,016,576 (32.5 to
17.0), `mi_malloc` 1,978,955 to 1,978,955, `mi_free` 1,777,181 to
1,777,181. The check row by category, AW12's table against this one, in
millions: the comparisons 131.6 to 34.5, the hashing and equality 55.7
to 48.3, the counts and drops 189.4 to 173.8, the generated code 981.3
to 998.0, the natives 297.2 to 297.2, the VM's own 90.0 to 90.0, the
records 46.0 to 46.0, mimalloc 121.1 to 121.1, the texts and copies
103.8 to 103.8, the calls through the VM's path 6.9 to 6.9, the
helpers' glue 3.7 to 3.7, the load 46.4 to 46.4, the iterators 25.3 to
25.3. AT1's rows on the frozen self-check (`tools/measure_size.sh`
under WSL, stage 3b's binary against this one): the synchronous JIT row
5,456.5 to 5,248.2 million instructions (-3.8%) and 7,800.2 to 7,546.0
by the estimate (-3.3%), its indirect branches 11.9 to 10.7 million
(-9.5%) and its mispredictions 31.9 to 31.2, the image row 3,694.0 to
3,473.0 (-6.0%) and 5,486.6 to 5,166.3 by the estimate (-5.8%), the
cached run 3,774.1 to 3,552.7 (-5.9%), the interpreter 11,204.0 to
11,203.6 (within 0.05%); the image 9,567,648 to 9,588,256 bytes
(+0.2%). The four benchmarks and hello by user-space instructions
(`perf stat`, the JIT synchronous, in millions): `primes` 265.2 to
265.2 (unchanged), `strings` 450.7 to 450.7 (unchanged), `records`
157.1 to 157.1 (unchanged), `json_round_trip` 841.1 to 841.1
(unchanged), `hello` 5.44 to 5.44 (unchanged). The four judges
byte-equal, the conformance suite 55 of 55 on both tiers, the template
tier's own tests with the tier forced, 402 tests. Since stage 0 the
check row 2,889.9 to 1,995.8 (-30.9%), since the study's binary 3,085.3
(-35.3%). Next in AW1's order: the rest of stage 4, the iterator in two
slots (4b) and `match` by a table (4c).

**AW14. Stage 4b of the new backend: a loop's iterator in two slots of
the frame.** The second item of stage 4 in AW1's order (the owner's
answer 8 there: the iterator in two slots as a bytecode change in both
emitters with the judges), on 2026-10-12, measured as AW1 prescribes
against stage 4a's binary (cf06592). A `for each` over anything but a
range held in registers allocated a native iterator (`Native::Iterator`
with a `ListIter`: the items' address, their count, the position and
the list, in a counted block): 238,534 on the check row through
`rt_iter_init` (43.3 million instructions inclusive, `iterator_of` 11.2
self), freed when the frame left (238,534 drops of the native, 6.5
million self, 33.8 inclusive with the block's free), and the generated
code read the position through the slot, the block and the native every
turn and wrote it back. The stage: (1) both emitters allocate two
consecutive slots per loop (`loop_slots` in `compile/mod.rs` and in
`compiler/emit.ry`, at every `IterInit`: the plain loop, the two passes
of a sorted one, a query's sources and its sorted pass, the compile
judge holding them equal): the first holds the source listed (the list
itself; the items of a set, a map, a text or a guarded source as a
list; a range of small Integers as its own iterator value, as before),
the second the position as a small Integer. (2) The interpreter's
`IterInit` writes both and `IterNext` reads the item at the position
and advances it (`iterator_of`, `iterator_next`); the helpers
`rt_iter_init` and `rt_iter_next` do the same, and the template tier
calls them as before. (3) The analysis marks the second slot an Integer
(`SlotKind::Int`), so the Cranelift tier keeps the position in a
register: `IterInit` moves a plain list from the stack into the slot
where it lies, the slot's old value released, and sets the register to
0 (anything else through the helper, which lists it; a range in
registers sets the register too); `IterNext` tests the slot for a list,
reads the items' address and their count through the list's block as
`List.at` does (AU34: the layout words from the native state), compares
the position, copies the item with one more reference and advances the
register; a range iterator value in the slot takes the helper as
before; a hand-back boxes the position as any Integer slot (`rt_deopt`)
and an entry at a loop header reads it back (`rt_resume_int`), so a
loop goes on in the interpreter from where the code was and comes back.
(4) The bytecode file is format 9 and the image format 22 (the slots);
`Native::Iterator`, `ListIter` and the layout constants of the native
iterator are gone. The tests: the corpus, the conformance suite on both
tiers, the four judges (the compile judge on the slots of every loop of
the corpus, the suite and `compiler/`), the loop tests of
`tests/native.rs` and `tests/template.rs`, and a new one in both,
`loops_in_two_slots_keep_their_list_and_position_across_a_hand_back`: a
product that leaves the machine word inside a loop hands the frame back
and the loop goes on in the interpreter, the list grown in its own body
keeps the loop's items, an inner loop uses its slots again on every
row, a sorted loop and a query over two sources run, on machine code
against the interpreter. What the measure found on the check row:
`rt_iter_init` 238,534 to 0 calls (43.3 million inclusive to none:
every loop's source there is a plain list, placed by the generated
code), `iterator_of` 238,534 to 0 (11.2 to 0.0 self), `rt_iter_next`
not called before either (the generated code read a list iterator in
place since AU34), the native's drops 238,534 to 0 (6.5 to 0.0 self),
`mi_malloc` 1,978,955 to 1,740,421, `mi_free` 1,777,181 to 1,538,721,
`rt_truncate` 97,106 to 97,106 (28.6 to 26.6 inclusive), the glue of
`Value` 2,016,576 to 1,191,936 (17.0 to 10.2). The measure in full
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against stage 4a's binary measured the
same way in AW13, the reference AU34's 44782aa): the check row 1,995.8
to 1,930.3 million instructions (-3.3%), the parse row 827.8 to 827.6
(within 0.05%), the compile row 3,260.6 to 3,156.9 (-3.2%); the ratios
by instructions against the reference's rows (441.6, 284.7 and 365.6)
4.37, 2.91 and 8.63 against the gate's 3 (4.52, 2.91 and 8.92 after
AW13); the Rust front end's own rows within 0.1% of stage 4a's (`check`
173.3, `parse --json` 246.7, `compile` 446.2). The check row by
category, AW13's table against this one, in millions: the iterators
25.3 to 0.0, the counts and drops 173.8 to 152.2, mimalloc 121.1 to
108.8, the generated code 998.0 to 1,000.9, the comparisons 34.5 to
34.5, the hashing and equality 48.3 to 47.4, the natives 297.2 to
297.2, the VM's own 90.0 to 88.9, the records 46.0 to 46.0, the texts
and copies 103.8 to 96.7, the calls through the VM's path 6.9 to 6.9,
the helpers' glue 3.7 to 3.7, the load 46.4 to 46.5. AT1's rows on the
frozen self-check (`tools/measure_size.sh` under WSL, stage 4a's binary
against this one): the synchronous JIT row 5,248.2 to 5,224.1 million
instructions (-0.5%) and 7,546.0 to 7,481.6 by the estimate (-0.9%),
its indirect branches 10.7 to 10.6 million (-1.5%) and its
mispredictions 31.2 to 31.7, the image row 3,473.0 to 3,401.3 (-2.1%)
and 5,166.3 to 5,103.8 by the estimate (-1.2%), the cached run 3,552.7
to 3,481.0 (-2.0%), the interpreter 11,203.6 to 11,217.7 (+0.1%); the
image 9,588,256 to 9,742,192 bytes (+1.6%). The four benchmarks and
hello by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 265.2 to 265.9 (+0.3%), `strings` 450.7 to 442.9
(-1.7%), `records` 157.1 to 160.9 (+2.4%), `json_round_trip` 841.1 to
843.9 (+0.3%), `hello` 5.44 to 5.44 (unchanged). The four judges
byte-equal, the conformance suite 55 of 55 on both tiers, the template
tier's own tests with the tier forced, 403 tests. Since stage 0 the
check row 2,889.9 to 1,930.3 (-33.2%), since the study's binary 3,085.3
(-37.4%). Next in AW1's order: the last item of stage 4, `match` by a
table (4c), then the checker's own cuts between the stages, stages 5
and 6.

**AW15. Stage 4c of the new backend: `match` by a table.** The last
item of stage 4 in AW1's order, on 2026-10-12, measured as AW1
prescribes against stage 4b's binary (8eed964). A `match` tested its
arms in turn: for every arm whose pattern names a variant, the subject
loaded, its tag tested (`IsVariant`, in place since AU49) and a jump
past the arm when it did not fit, so that a value of the last of n
variants paid n tests, each a load, a test and a branch, and the
checker's and the parser's matches over the kinds of expressions, types
and tokens paid most of theirs on the arms before the right one. The
stage: (1) a new op, `JumpByTag { slot, table }`, with the jump tables
of a code object beside its ops (`Code.tables`): the op peeks at the
frame slot and, for a variant whose tag is below the table's length,
jumps to the table's entry for the tag; anything else, a guarded value,
a `Nothing`, a failure, a tag past the table, falls through to the
arms' own tests, which stay as they were. (2) Both emitters emit it
after the subject's store when at least three arms name a variant
(`TABLE_ARMS` in `compile/expr.rs`, the same number in
`compiler/emit.ry`), the table filled once the arms' positions are
known (`match_table` in both): for every tag up to the highest an arm
names, the first arm in order that can match it, entered past its own
tag test (three ops: the load, the test, the jump) when it names the
tag and at its start when its pattern is anything else (a binding, a
typed name, a literal, a `some`, a record matched like a single
variant: any of them may fit), else the `otherwise` or the crash after
the arms; a guard or a nested pattern that fails goes on down the arms
as before. The compile judge holds the two emitters equal on the tables
of every program of the corpus, the suite and `compiler/`. (3) The
interpreter reads the slot's plain value; the Cranelift tier reads the
slot's kind and the block's tag in place and enters the arm's block
through a jump table (`br_table`), the edges coerced as a branch's are,
the fall-through the block after; the template tier compares the tag
with each entry in turn (`cmp`, `je`), the entries block starts of its
counting. The analysis follows the table's targets among the flows
(`native/infer.rs`, both passes), as do the last reads (`liveness.rs`)
and the inliner, which appends an expanded callee's tables to the
caller's with the targets shifted. (4) The bytecode file is format 10,
the binary encoding 6 and the image format 23. The tests: the corpus,
the conformance suite on both tiers, the four judges, and a new test in
`tests/native.rs` and `tests/template.rs`,
`matches_by_a_table_enter_the_arm_by_the_tag_and_fall_through_otherwise`:
six arms with a guard that fails to a later arm of the same tag, a
nested literal that fails to the next arm, field-less arms, a tag past
the table, an overflow inside an arm entered through the table (the
frame handed back), a binding arm entered at its start, a `maybe`
subject and a two-arm match without a table, and a guarded subject that
falls through, on machine code against the interpreter. What the
measure found on the check row: the generated code 1,000.9 to 962.7
million, the comparisons 34.5 to 34.5, the counts and drops 152.2 to
152.2, the glue of `Value` 1,191,936 to 1,191,936 calls (10.2 to 10.2
million self), `mi_malloc` 1,740,421 to 1,740,842. The measure in full
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against stage 4b's binary measured the
same way in AW14, the reference AU34's 44782aa): the check row 1,930.3
to 1,896.4 million instructions (-1.8%), the parse row 827.6 to 822.8
(-0.6%), the compile row 3,156.9 to 3,097.6 (-1.9%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6)
4.29, 2.89 and 8.47 against the gate's 3 (4.37, 2.91 and 8.63 after
AW14); the Rust front end's own rows within 0.2% of stage 4b's (`check`
173.1, `parse --json` 246.7, `compile` 447.2). The check row by
category, AW14's table against this one, in millions: the generated
code 1,000.9 to 962.7, the comparisons 34.5 to 34.5, the counts and
drops 152.2 to 152.2, mimalloc 108.8 to 108.7, the hashing and equality
47.4 to 47.7, the natives 297.2 to 297.2, the VM's own 88.9 to 88.9,
the records 46.0 to 46.0, the texts and copies 96.7 to 96.8, the calls
through the VM's path 6.9 to 6.9, the helpers' glue 3.7 to 3.7, the
load 46.5 to 50.4, the iterators 0.0 to 0.0. AT1's rows on the frozen
self-check (`tools/measure_size.sh` under WSL, stage 4b's binary
against this one): the synchronous JIT row 5,224.1 to 5,156.1 million
instructions (-1.3%) and 7,481.6 to 7,395.6 by the estimate (-1.1%),
its indirect branches 10.6 to 10.5 million (-0.7%) and its
mispredictions 31.7 to 30.4, the image row 3,401.3 to 3,371.1 (-0.9%)
and 5,103.8 to 4,998.4 by the estimate (-2.1%), the cached run 3,481.0
to 3,451.4 (-0.9%), the interpreter 11,217.7 to 11,061.4 (-1.4%); the
image 9,742,192 to 9,770,848 bytes (+0.3%). The four benchmarks and
hello by user-space instructions (`perf stat`, the JIT synchronous, in
millions): `primes` 265.9 to 265.9 (unchanged), `strings` 442.9 to
442.9 (unchanged), `records` 160.9 to 160.9 (unchanged),
`json_round_trip` 843.9 to 843.9 (unchanged), `hello` 5.44 to 5.44
(unchanged). The four judges byte-equal, the conformance suite 55 of 55
on both tiers, the template tier's own tests with the tier forced, 405
tests. Since stage 0 the check row 2,889.9 to 1,896.4 (-34.4%), since
the study's binary 3,085.3 (-38.5%). Stage 4 is done. Next in AW1's
order: the checker's own cuts between the stages, then stages 5 and 6.

**AW16. The checker's own cuts between the stages: the variants indexed
by name, the parser's cursor with its token, the lexer's previous token
as two Integers and the text of a run as a primitive, the resolver's
loop, the lists updated in the slot.** The item of AW1's order after
stage 4, on 2026-10-12, measured as AW1 prescribes against stage 4c's
binary (805ea3a). The cuts are in the compiler written in Renyi
(`compiler/*.ry`), one library primitive beside them; each was gated by
`renyi check`, the canonical layout and the four judges byte-equal,
then kept or dropped by the lane's quick measure, the check row by
`perf stat` (user-space instructions) with the images rebuilt from the
edited sources, 1,860.8 million before any cut. (1) The variants of a
module's scope indexed by name once the types are final
(`index_variants` and `variants_of` at the end of `resolve_all`, the
`variants` field of `ModuleInfo`, in `compiler/declare.ry`):
`lookup_variant` rebuilt its candidates (the module's types, the
exposed ones, the prelude's, distinct and sorted) and scanned every sum
type's variants on each of its 650 calls, 102.6 million instructions
inclusive, 5.4% of the row; it is one map read now. The row 1,860.8 to
1,766.6 (-5.1%). (2) The parser's cursor carries its looked-at token
(`token` in `Cursor`): `at_position` computes the look and the token
together (what `look_from` did) and binds them before the update, so
that the cursor is updated in its slot (AU11, AW8) where the update
read the cursor twice in one statement and copied it on every move
(`rt_record_copy` 124,609 calls); `moved` changes the position alone,
the look and the token at it being the same already; 39 sites read
`cursor.token` in place of `token_at(cursor, look)`. 1,766.6 to 1,719.5
(-2.7%). (3) The lexer's previous token as two Integers, its stop and
its code (`previous_stop` and `previous_code` through `lex_range`,
`next_step`, `lex_word`, `lex_text` and `camel_case_word`), compared in
place against `words.symbol_dot` and `words.word_raw` where a `maybe
Token` was matched on at every word and every text literal; a reserved
word's text sliced once (`lex_reserved` takes the word's text) where it
was sliced three times; and the parser sets the comments aside in one
pass over the tokens where two queries ran. 1,719.5 to 1,628.7 (-5.3%).
(4) `resolve` follows a variable's binding in one match per step
(`is_bound_var` gone) where it looked the variable up twice per step.
1,628.7 to 1,615.8 (-0.8%). (5) The lexer's text of a run as a library
primitive, `slice_text(self: List of Text, start: Integer, stop:
Integer) returns Text`, a library addition through the sketch,
`library/std/prelude.ry`, the checker's copy and `natives/prelude.rs`
(a typed and an in-place entry; a test in `tests/semantics.rs`), as
AU52 (iv) added `run_of`: the texts from start to stop as one text, the
bounds clamped as `slice` clamps them, where `slice` then `join`
allocated a list, raised and lowered a count per character and
allocated the text (135.1 million inclusive for 94,725 slices, 7.1%);
the lexer's 19 slices, the parser's `text_of` and the checker's
`source_text` use it; the cheat sheet does not list it, its budget
standing at 2,997 of 3,000 tokens. 1,615.8 to 1,561.4 (-3.4%). (6) The
checker's scopes flattened into one list of bindings with the scope
starts measured worse (1,561.4 to 1,575.7, +0.9%: a pop truncates by
`take`, which copies) and was dropped; in its place the lists of scopes
and of inference variables are updated in the checker's slot,
`checker.scopes.with_at(...)` and `checker.vars.with_at(...)` in the
update, where `lists.replace_at(items: checker.scopes, ...)` read the
list out of the record (a second holder, which the native copied) on
every binding declared, marked as used or retagged and on every
variable bound. (7) The parser's bracket loop looks the closing
symbol's code up once, not per element, and `zonk` resolves a type
once. The row 1,561.4 to 1,554.3 (-0.5%) with (7), to 1,548.3 (-0.4%)
with (6). The sum by the quick measure: 1,860.8 to 1,548.3 million
(-16.8%). What the profile leaves (`study_incl.py`, the lane's new
script, ranks the generated functions by their inclusive cost, what a
cut in the Renyi sources removes): the record copies 124,609 to 109,613
only, since the parser's result records (`Parsed`, `Peek`, `Eaten`) are
read after their cursor is taken at 186 sites, which keeps the cursor
shared so that `at_position` copies it on most moves (AU55's rewrite
for `Done` applied to them is the next cut there); the clones 4,753,908
to 3,822,834, mimalloc's allocations 1,740,842 to 1,378,394, the lists
joined 97,732 to 3,011; and the backend's own costs, the call of a
five-op helper (`is_word` and `is_symbol`, 178,401 calls at 119
instructions each) and the clone a field read makes, which stages 5 and
6 and AU50's expansion are for. The measure in full (callgrind under
WSL on the release build for Linux, the images built under `valgrind
--tool=none`, against stage 4c's binary measured the same way in AW15,
the reference AU34's 44782aa): the check row 1,896.4 to 1,586.1 million
instructions (-16.4%), the parse row 822.8 to 711.9 (-13.5%), the
compile row 3,097.6 to 2,794.9 (-9.8%); the ratios by instructions
against the reference's rows (441.6, 284.7 and 365.6) 3.59, 2.50 and
7.64 against the gate's 3 (4.29, 2.89 and 8.47 after AW15); the Rust
front end's own rows within 0.6% of stage 4c's (`check` 173.8, `parse
--json` 246.4, `compile` 449.7). The check row by category, AW15's
table against this one, in millions: the generated code 962.7 to 800.8,
the comparisons 34.5 to 6.5, the counts and drops 152.2 to 131.6,
mimalloc 108.7 to 87.9, the hashing and equality 47.7 to 35.2, the
natives 297.2 to 255.8, the VM's own 88.9 to 79.5, the records 46.0 to
46.1, the texts and copies 96.8 to 79.5, the calls through the VM's
path 6.9 to 8.5, the helpers' glue 3.7 to 3.6, the load 50.4 to 50.4,
the iterators 0.0 to 0.0. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 4c's binary against this one;
the tiers are the same, the one native added): the synchronous JIT row
5,156.1 to 5,160.2 million instructions (+0.1%) and 7,395.6 to 7,381.0
by the estimate (-0.2%), its indirect branches 10.5 to 10.5 million
(+0.1%) and its mispredictions 30.4 to 30.2, the image row 3,371.1 to
3,374.3 (+0.1%) and 4,998.4 to 5,043.5 by the estimate (+0.9%), the
cached run 3,451.4 to 3,464.0 (+0.4%), the interpreter 11,061.4 to
11,068.3 (+0.1%); the image 9,770,848 to 9,770,928 bytes (within
0.05%). The four benchmarks and hello by user-space instructions (`perf
stat`, the JIT synchronous, in millions): `primes` 265.9 to 266.0
(within 0.05%), `strings` 442.9 to 443.0 (within 0.05%), `records`
160.9 to 160.9 (unchanged), `json_round_trip` 843.9 to 843.9
(unchanged), `hello` 5.44 to 5.47 (+0.6%). The four judges byte-equal,
the conformance suite 55 of 55 on both tiers, 406 tests. Since stage 0
the check row 2,889.9 to 1,586.1 (-45.1%), since the study's binary
3,085.3 (-48.6%). Next in AW1's order: stages 5 and 6.

**AW17. Stage 5a of the new backend: the moves of AU17 written in the
image and in the cache keyed by the sources.** The first half of stage
5 of AW1's order (the load), on 2026-10-12, measured as AW1 prescribes
against the cuts' binary (c7b8f6c). AU17 ran its liveness pass on a
copy of the program before every run, so that the bytecode file, the
image's program and the cache's key stayed what the emitters wrote: on
the checker's image run the copy and the pass cost 37.5 million
instructions, 2.4% of the row (`liveness::prepared` inclusive in the
cuts' profile), at every run of a program whose machine code was
compiled from the prepared program already. Now the pass runs once,
where a program enters the VM, and marks it: `Program::prepared`, a
flag the binary encoding keeps (encoding 7, the flag after the format)
and the bytecode file does not (the `.ryc` stays the emitters' program,
held equal by the judges); `liveness::prepare` applies the moves in
place and sets the flag, and `liveness::prepared` answers the program
itself (`Cow::Borrowed`) when it is prepared and a prepared copy
otherwise, which the runner, the sandbox and `image::build` take.
`renyi build` writes the prepared program in the image, so that a run
from an image, a file's or the cache's, skips the pass; the binary's
`compile_cached` prepares the program on every path (an image's comes
prepared; a bytecode file's and a fresh compile's are prepared there)
and stores the prepared encoding in the cache keyed by the sources, so
that a run from that cache skips it too and the image cache's entry,
named from that encoding, holds the image's program byte for byte as
before. This reverses the part of AU17 that kept the image's program
and the cache's key as the emitters wrote them; the bytecode file keeps
that promise, and `tests/build.rs` now holds the image's program to the
bytecode file's prepared by the pass. An image built from a program
that comes prepared (`build --cache`) carries in `code_hash` the hash
of the prepared rendering, which nothing reads: a run through the cache
takes its hash from the sources, and `build` from a file prepares a
copy and hashes the file's program. A test in `tests/native.rs` holds
the pass to once: a prepared program is borrowed, a second pass over it
finds nothing, and the flag survives the encoding. The measure in full
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against the cuts' binary measured the
same way in AW16, the reference AU34's 44782aa): the check row 1,586.1
to 1,547.9 million instructions (-2.4%), the parse row 711.9 to 701.6
(-1.4%), the compile row 2,794.9 to 2,743.6 (-1.8%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6)
3.51, 2.46 and 7.50 against the gate's 3 (3.59, 2.50 and 7.64 after
AW16); the Rust front end's own rows within 0.1% of the cuts' (`check`
173.8, `parse --json` 246.4, `compile` 450.0). The check row by
category, AW16's table against this one, in millions: the load 50.4 to
26.1, the generated code 800.8 to 800.8, the comparisons 6.5 to 6.5,
the counts and drops 131.6 to 131.0, mimalloc 87.9 to 79.8, the hashing
and equality 35.2 to 35.4, the natives 255.8 to 255.8, the VM's own
79.5 to 79.5, the records 46.1 to 46.1, the texts and copies 79.5 to
74.4, the calls through the VM's path 8.5 to 8.5, the helpers' glue 3.6
to 3.6, the iterators 0.0 to 0.0. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, the cuts' binary against this one;
the tiers are the same, the pass skipped on the image row and on the
cached run, the self-check's program from its sources prepared once
without the copy): the synchronous JIT row 5,160.2 to 5,157.5 million
instructions (-0.1%) and 7,381.0 to 7,332.3 by the estimate (-0.7%),
its indirect branches 10.5 to 10.6 million (+1.2%) and its
mispredictions 30.2 to 30.6, the image row 3,374.3 to 3,328.7 (-1.4%)
and 5,043.5 to 4,967.1 by the estimate (-1.5%), the cached run 3,464.0
to 3,408.8 (-1.6%), the interpreter 11,068.3 to 11,065.9 (within
0.05%); the image 9,770,928 to 9,770,928 bytes (unchanged). The four
benchmarks and hello by user-space instructions (`perf stat`, the JIT
synchronous, in millions): `primes` 266.0 to 265.8 (-0.1%), `strings`
443.0 to 442.6 (-0.1%), `records` 160.9 to 160.7 (-0.1%),
`json_round_trip` 843.9 to 843.1 (-0.1%), `hello` 5.47 to 5.20 (-4.9%).
The four judges byte-equal, the conformance suite 55 of 55 on both
tiers, 407 tests. Since stage 0 the check row 2,889.9 to 1,547.9
(-46.4%), since the study's binary 3,085.3 (-49.8%). Next in AW1's
order: stage 5b, the code objects decoded on demand, then stage 6.

**AW18. Stage 5b of the new backend: the code objects decoded on
demand.** The second half of stage 5 of AW1's order (the load), on
2026-10-12, measured as AW1 prescribes against stage 5a's binary
(16a0dd6). The program of an image, and of an entry of the cache keyed
by the sources, was decoded whole at the load, 32.2 million
instructions on the checker's image run (2.1%, `binary::decode`
inclusive in stage 5a's profile), most of it the ops of 946 code
objects whose machine code the image carries and the run never reads as
ops, with their spans, their types and their jump tables. Now a code
object's head (the name, the module, the kind, the function, the
parameters, the locals, the read-only flags, the constants) is decoded
with the program, and its body (`Body` in `bytecode.rs`: the ops, the
spans, the types, the tables) is decoded at its first read:
`write_code` writes the body as a length-prefixed block after the head
(encoding 8), `read_code` keeps the block's bounds in the shared bytes
(`binary::Encoded`: an `Arc` of the program's encoding, which
`decode_shared` takes and `decode` makes from a slice, and the image
holds its program as such an `Arc`), and `Code::body` decodes it
through a `OnceLock`, which the compile thread of AU15 and the
interpreter may reach at once, checked then as the file's loader checks
a body (`file::check_body` against `file::Limits`, the counts of the
program's tables, filled in once the program is read; a body that does
not decode stops the run as a corrupt op would, the file's loader
having no body to check at the load). Every read goes through
`Code::body`, `ops`, `spans`, `types` and `tables`, every write through
`body_mut` and `set_body` (the compiler's `emit`, the jump patches, the
pass of AU17, the expansion of AU50), and the interpreter's loop binds
the body once per frame beside the code object; a program from the
compiler or from the bytecode file holds every body decoded, as before,
and `file::check` checks the bodies it finds decoded. What an image run
decodes: the module constants that are literals (`literal_constant`
reads their two ops at `Jit::new`), the bodies the interpreter runs or
the hand-backs reach, and nothing else; a run from the cache keyed by
the sources decodes each body as the JIT compiles it or the interpreter
enters it. A test in `tests/native.rs` holds the bodies undecoded after
the decode, decoded by their first read, equal to the encoder's, and a
block cut short refused with the program. The measure in full
(callgrind under WSL on the release build for Linux, the images built
under `valgrind --tool=none`, against stage 5a's binary measured the
same way in AW17, the reference AU34's 44782aa): the check row 1,547.9
to 1,528.5 million instructions (-1.3%), the parse row 701.6 to 696.2
(-0.8%), the compile row 2,743.6 to 2,719.7 (-0.9%); the ratios by
instructions against the reference's rows (441.6, 284.7 and 365.6)
3.46, 2.45 and 7.44 against the gate's 3 (3.51, 2.46 and 7.50 after
AW17); the Rust front end's own rows within 0.2% of stage 5a's (`check`
173.9, `parse --json` 246.4, `compile` 450.9). The check row by
category, AW17's table against this one, in millions: the load 26.1 to
10.0, the generated code 800.8 to 800.8, the comparisons 6.5 to 6.5,
the counts and drops 131.0 to 131.0, mimalloc 79.8 to 76.8, the hashing
and equality 35.4 to 35.2, the natives 255.8 to 255.8, the VM's own
79.5 to 79.6, the records 46.1 to 46.1, the texts and copies 74.4 to
74.3, the calls through the VM's path 8.5 to 8.5, the helpers' glue 3.6
to 3.6, the iterators 0.0 to 0.0. AT1's rows on the frozen self-check
(`tools/measure_size.sh` under WSL, stage 5a's binary against this one;
the tiers are the same, the bodies decoded as the JIT compiles them on
the JIT rows and as the interpreter enters them on its row, from the
image's bytes on the image row and the cached run): the synchronous JIT
row 5,157.5 to 5,161.6 million instructions (+0.1%) and 7,332.3 to
7,479.8 by the estimate (+2.0%), its indirect branches 10.6 to 10.5
million (-1.6%) and its mispredictions 30.6 to 30.1, its
instruction-cache misses 88.8 to 98.8 million and its last-level misses
3.2 to 3.7 (the estimate's rise is theirs: the generated code's
instructions the same, 1,617.9 million, with its misses 63.3 to 68.0
million, and the helpers' (`runtime.rs`) 304.8 million the same with
their misses 4.1 to 6.7, the same functions placed differently in the
cache's sets; the instructions within 0.1%), the image row 3,328.7 to
3,311.8 (-0.5%) and 4,967.1 to 4,965.0 by the estimate (within 0.05%),
the cached run 3,408.8 to 3,392.8 (-0.5%), the interpreter 11,065.9 to
10,942.5 (-1.1%); the image 9,770,928 to 9,770,928 bytes (unchanged).
The four benchmarks and hello by user-space instructions (`perf stat`,
the JIT synchronous, in millions): `primes` 265.8 to 265.8 (unchanged),
`strings` 442.6 to 442.6 (unchanged), `records` 160.7 to 160.7
(unchanged), `json_round_trip` 843.1 to 843.1 (unchanged), `hello` 5.20
to 5.20 (unchanged). The four judges byte-equal, the conformance suite
55 of 55 on both tiers, 408 tests. Since stage 0 the check row 2,889.9
to 1,528.5 (-47.1%), since the study's binary 3,085.3 (-50.5%). Stage 5
is done. Next in AW1's order: stage 6, AU50's expansion measured again
under the new convention.
