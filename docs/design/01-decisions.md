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
