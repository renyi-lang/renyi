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
