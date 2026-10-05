# Renyi Syntax Sketch (M0)

Status: draft for the readability test. Date: 2026-10-05.

This document turns the round-1 decisions (`01-decisions.md`) into a concrete
surface. It is a sketch, not a specification: it fixes the shape of every
construct so that the example corpus and the cheat sheet can be written, and it
lists the points that round 2 must settle. The formal grammar is written at M1
from this document and the corpus.

Three rules generate most of what follows:

1. **Clause grammar.** Every construct is a head followed by labelled clauses.
   Clauses are optional and have one canonical order. Blocks end with `end`.
2. **One spelling.** Each concept has exactly one word. Universal mathematical
   notation (`f(x)`, `x: Type`, `+ - * /`, `[ ]`) is kept; nothing else is a
   symbol.
3. **Wrap implicitly, unwrap explicitly.** A value becomes a `maybe` or a
   success result without ceremony. Getting a value *out* always names the
   failure case (`otherwise`, `match`).

---

## 1. Lexical structure

**Encoding and layout.** UTF-8 without BOM, LF line endings. One statement per
line. Indentation is two spaces and carries no meaning. Maximum line width is
100 columns; `renyi format` owns all layout and has no options.

**Comments.** `#` to end of line. Comments are for *why*; *what* belongs in
`purpose:` clauses, which are syntax, not comments.

**Identifiers.**

| Kind | Pattern | Examples |
|------|---------|----------|
| value, function, field, parameter, module segment | `[a-z][a-z0-9_]*`, at least two characters, no trailing or doubled underscore | `user`, `total_price`, `is_active` |
| type, ability, variant | `[A-Z][A-Za-z0-9]*` | `User`, `Compare`, `NotFound` |

ASCII only. Single-letter names are rejected. A name that equals a reserved word
is rejected with a rename suggestion (`count` becomes `item_count`). After a
dot, any word is allowed as a member name (`event.type`), and for the same
reason a method, a function whose first parameter is `self` and which is
therefore only ever called after a dot, may be declared under any word
(`function first(self: List of Item)`, `function set(self: Map of Key to
Value, ...)`; decision N1).

**Reserved words.** 85 words, listed in section 17. Multi-word keywords such as
`is at least` and `or fails with` are single tokens; the lexer matches the
longest phrase in the fixed phrase table (section 17). Exactly one space
separates the words of a phrase; a phrase cannot span lines.

**Literals.**

| Literal | Type | Notes |
|---------|------|-------|
| `42`, `1_000_000` | `Integer` | arbitrary precision; takes type `Decimal` or `Float` when the context expects it |
| `19.99`, `0.5` | `Decimal` | exact; takes type `Float` only when the context expects `Float` |
| `"text"` | `Text` | `{expression}` interpolates any value that `can ToText`; escapes `\n \t \" \\ \{` |
| `"""` ... `"""` | `Text` | multi-line; the text runs from the line after the opening quotes to the line before the closing quotes, without a trailing newline; common leading indentation is removed |
| `raw "text"` | `Text` | no interpolation and no escapes; for regular expressions and for JSON or SQL samples |
| `true`, `false` | `Boolean` | |
| `nothing` | `maybe T` | the absent value |
| `[1, 2, 3]`, `[]` | `List of T` | |
| `{"a": 1, "b": 2}`, `{}` | `Map of K to V` | JSON notation, kept as universal (open item R2-1) |
| `from 1 to 10`, `from 0 to 100 by 5` | `Range` | inclusive on both ends; optional step |

**Interpolation.** Every `"..."` literal interpolates `{expression}`, so a
literal brace is written `\{`. A hole may not contain a string literal; bind the
text first (`let separator be ", "` then `"{items.join(separator)}"`).
`raw "^[0-9]{4}$"` has no holes and no escapes (decision J5).

**Statement continuation.** A statement ends at the newline unless a bracket is
open or the next non-blank line is indented deeper than the line that started
the statement and starts with a continuation word: `otherwise`, `where`,
`sorted by`, `group by`, `collect`, `sum`, `count`, `first`, `any`, `all`,
`returns`, `or fails with`, `needs`, `for any`, `with`, `and`, `or`. The
indentation condition is what tells the `otherwise` of an `if` statement, at
the `if`'s column, from the `otherwise` of a fallible call, one level deeper.
The value after `be` or `return` may start on the next, deeper line; a
documentation clause continues on any deeper line. The formatter breaks long
lines only before continuation words.

---

## 2. Modules and imports

One file is one module; the module name is the path from the project root with
`/` replaced by `.`. The header must match the path. `.renyi` and `.ry` are
equivalent.

```
module billing.invoices
  purpose: Compute and render customer invoices.
  tags: billing, money

import std.json
import std.http as web
import accounts.models exposing User, UserId
```

- `import a.b.c` brings the module in as the namespace `c`; its functions are
  called as `c.function(...)`. `as` renames the namespace when two modules
  share a last segment.
- `exposing` lists **types and abilities** to use unqualified. Exposing a sum
  type also exposes its variants, so `import std.time exposing Weekday` lets a
  module match on `Saturday`. Functions are never imported unqualified: every
  call site names its module, which is what makes a chunk self-describing.
- A namespace name is taken within the importing module: after
  `import invoice`, no local binding or parameter may be called `invoice`.
- There is no wildcard import.
- Functions defined in the current module are called unqualified.
- The prelude provides the base types (`Integer`, `Decimal`, `Float`, `Text`,
  `Bytes`, `Boolean`, `List`, `Map`, `Set`, `Range`, `Pair`, `Duration`),
  `maybe`, the core abilities and the built-in error types. It provides no
  free functions; operations on base types are methods (`items.length()`,
  `text.trim()`), listed in `04-stdlib-sketch.md`.

Definitions are private unless marked `public`. A module's public definitions
are its API; the compiler diffs this API to decide version numbers.

---

## 3. Functions

```
public function active_adult_emails(path: Path)
  returns List of Email
  or fails with FileError or JsonError
  needs filesystem.read
  purpose: Read users from a JSON file and return the emails of active users who are at least 18.
  tags: users, export
  example: active_adult_emails(Path("tests/fixtures/users.json")) is [Email("ann@example.com")]

  let text be filesystem.read_text(path) otherwise fail
  let users: List of User be json.parse(text) otherwise fail
  let emails be
    for each user in users
    where user.is_active and user.age is at least 18
    collect user.email
  return emails.sorted()
end
```

**Head.** `[public] function name(param: Type, ...)`. Parameters are typed;
there are no defaults and no variadics in v1.

**Signature clauses**, in canonical order, each optional:

| Clause | Meaning |
|--------|---------|
| `returns Type` | result type; omitted when the function returns nothing |
| `or fails with E1 or E2` | the function can fail with these error types |
| `needs cap, cap` | capabilities used directly by the body (section 11) |
| `for any T where T can Ability` | type parameters and their constraints |

When head and signature clauses fit in 100 columns they share one line; when
they do not, each clause gets its own line indented once. Same tokens, one
grammar.

**Documentation clauses**, after the signature: `purpose:` (required when
public), `tags:`, `see also:`, `deprecated:`, `expose as tool`, then any number
of `example:` lines (section 13). A blank line separates clauses from the body.

**Body.** Statements, then `end`. A body is at most about 60 lines and nests at
most four levels deep. `return expression` exits; `return` alone exits a
function that returns nothing. Falling off the end of a function that
`returns` something is a compile error.

**Calls.**

```
let total be invoices.total_price(items: cart.items, rate: region.tax_rate)
let trimmed be text.trim()
let page be web.get(url) otherwise fail
```

- A call with one argument is positional; naming it is a compile error with
  the fix "drop the name" (decision M4). A call with two or more arguments
  names every argument, in declaration order. Names are mandatory because a
  swapped pair of same-typed arguments is the most common silent error in
  generated code.
- `receiver.method(args)` calls an ability function whose first parameter is
  `self`. A zero-argument method keeps its parentheses so it is never confused
  with a field.
- `namespace.function(args)` calls a function from an imported module.
- A function is passed as a value by name: `retry(action: fetch_page)`.

**Methods.** A function whose first parameter is `self: T` is a method of `T`
and is called with the dot: `public function trim(self: Text) returns Text`
makes `text.trim()`. It must be declared in the module that defines `T`.
Base-type and standard-library methods are declared this way (decision K1);
abilities are for polymorphism.

**Function types.** `function(Url) returns Page or fails with HttpError needs
network.http`. Effects of a function-typed parameter flow to the call site
automatically; a higher-order function declares only its own effects.

---

## 4. Types

One keyword, four clause words. Product with `has`, sum with `is one of`,
subset with `where`, constraint with `can`.

```
public type User
  purpose: A registered account holder.
  has id: UserId
  has name: Text
  has age: Integer where age is at least 0
  has email: maybe Email
  can Compare by name
  can ToJson
  can FromJson
end

public type Shape is one of
  purpose: A closed plane figure.
  Circle(radius: Decimal where radius is greater than 0)
  Rectangle(width: Decimal, height: Decimal)
  Point
end

public type Email is Text where value.matches(email_pattern)
  purpose: A syntactically valid e-mail address.

public type UserId is Integer
  purpose: Identifies a user; distinct from other integers.
```

**Records** (`has`). Fields are public. Construction names every field,
`User(id: UserId(7), name: "Ann", age: 30, email: nothing)`, `Circle(radius:
2.5)`; a subtype wraps one positional value, `UserId(7)`. Update copies with changes: `let older be user with
age: user.age + 1`. `has kind: Text as "type"` gives a field the external name
that `ToJson`, `FromJson` and `FromRow` use, for keys that are reserved words
or contain punctuation; `json.parse(text: text, naming: CamelCase)` maps a
whole record by convention (decision J14).

**Variants** (`is one of`). Each variant is a record with zero or more fields;
`Point` has none. Construction: `Circle(radius: 2.5)`, `Point`. Matching is
exhaustive (section 8). `can` clauses follow the variants.

**Refinements** (`where`). The condition is a pure Boolean expression over the
field name, or over `value` for a refined alias. When every refined argument is
a literal, the compiler checks the condition at compile time and the
construction is ordinary: `Circle(radius: 2.5)`. When any refined argument is a
runtime value the construction can fail and must say so: `let email be
Email(input) otherwise fail with InvalidInput(detail: "bad email")`. The error
type is the built-in `ConstraintViolation`. Decoders such as `json.parse` check
constraints at the boundary and report violations as their own error.

**Named subtypes** (`type UserId is Integer`, `type Email is Text where ...`).
`type X is Base` makes `X` a subtype of `Base`: an `X` is accepted wherever a
`Base` is expected and keeps the methods of `Base`, but a `Base` is never
accepted as an `X`. Going up needs no syntax because no information changes;
going down is a construction, `UserId(7)`, which is fallible when `X` is
refined. This is the one place the type system has subtyping. Arithmetic on
two `Money` values yields the base `Decimal`; wrap the result to keep the
name.

**Derived abilities** (`can`). `Equal` is derived for every data type. `can
Compare by field, field` derives ordering by the listed fields. `can ToText`,
`can ToJson`, `can FromJson`, `can Hash` are derivable. Any other ability is
implemented by hand (section 5).

**Generics.** Type parameters are introduced with `for any` on the definition
that uses them and constrained with `where`:

```
public type Pair of Left, Right
  has left: Left
  has right: Right
end

public function largest(items: List of Item) returns maybe Item
  for any Item where Item can Compare
  purpose: The greatest element, or nothing for an empty list.
  ...
end
```

`List of Item`, `Map of Text to Integer`, `Set of UserId`, `maybe Email`,
`Pair of Text, Integer`.

---

## 5. Abilities

An ability declares functions over `self`. A type gains the ability through an
implementation block or a `can` derivation.

```
public ability Describable
  purpose: Anything that can describe itself in one line of text.
  function describe(self) returns Text
end

ability Describable for Shape
  function describe(self) returns Text
    match self
      when Circle(radius) then return "a circle of radius {radius}"
      when Rectangle(width, height) then return "a {width} by {height} rectangle"
      when Point then return "a point"
    end
  end
end
```

Calls use the dot: `shape.describe()`. Inside a declaration `Self` names the
implementing type: `function compare(self, other: Self) returns Ordering`
(decision K7). Abilities can require other abilities
(`ability Printable where self can ToText`). An implementation for a generic
type introduces the type parameters with a `for any` clause after its head:

```
ability Sized for Stack of Item
  for any Item
  function size(self) returns Integer
    return self.items.length()
  end
end
```

Default method bodies are not in v1 (open item R2-5).

Core abilities in the prelude: `Equal`, `Compare`, `Hash`, `ToText`, `ToJson`,
`FromJson`, `Iterable`.

---

## 6. Bindings and mutation

```
let subtotal be items.sum_of_price()
let users: List of User be json.parse(text) otherwise fail
let mutable total be 0
set total to total + line.amount
```

- `let name be expression` defines an immutable binding. `let name: Type be
  expression` pins the type; this is how a generic result such as
  `json.parse` learns its target type.
- `let mutable name be expression` defines a mutable local. `set name to
  expression` is the only way to change it. Mutable bindings cannot escape the
  function; there are no references.
- A name is bound once per scope. Rebinding it, or shadowing it in a nested
  scope, is a compile error. Sibling scopes (two loop bodies, two `when`
  branches) may reuse a name.
- Every binding must be used. A `let`, loop variable, parameter or pattern
  binding that is never read is a compile error with the fix "remove it"
  (decision J8).
- A call whose result is not used is a compile error; the message proposes the
  likely fix (`set items to items.append(item)`). `ignore expression` discards
  a result deliberately (decision J15).
- Top-level `let` requires a type and is a constant: `public let max_retries:
  Integer be 3`, followed by a `purpose:` clause when public.

The language has no `=`.

---

## 7. Expressions

**Arithmetic.** `+ - * /` on two values of the same numeric type. `2 power 10`,
`total remainder 7`. `/` requires `Decimal` or `Float` operands; dividing two
`Integer`s is a compile error that points to `dividend.quotient(divisor)`
(integer division) or `dividend.to_decimal() / divisor` (decision J9).
Division by zero is a crash. `Decimal` is IEEE 754 decimal128: 34 significant
digits, exact for literals, sums and products of everyday values, division
rounded half-even at the 34th digit, overflow a crash (decision J16). `Float`
is IEEE 754 binary64. `value.at_least(other)` and `value.at_most(other)` give
the larger or the smaller of two values; there is no two-argument `max`
(decision J13).
Precedence, high to low: `power`; `* / remainder`; `+ -`; comparison phrases;
`not`; `and`; `or`. Parentheses group.

**Comparison.** Six phrases, no symbols: `is`, `is not`, `is less than`, `is at
most`, `is greater than`, `is at least`. Ordering requires `can Compare`.

**Logic.** `and`, `or`, `not`. Both `and` and `or` short-circuit.

**Text.** `"Hello {user.name}, you owe {amount}"`. Interpolated values use
`ToText`. Methods used by the corpus: `trim()`, `split(separator)`, `lines()`,
`to_lower()`, `to_upper()`, `pad_left(width)`, `pad_right(width)`,
`repeat(times)`, `take(count)`, `matches(pattern)`, `length()`.

**Optionals.** A `T` is accepted where a `maybe T` is expected (implicit wrap).
Reading requires `otherwise`: `let name be user.nickname otherwise user.name`,
or a `match` with `when some(name)` and `when nothing`.

**Collections.** Lists: `items.length()`, `items.is_empty()`, `items.at(index)`,
`items.first()` and `items.last()` (all three `maybe T`), `items.rest()`,
`items.without_last()`, `items.without_index(index)`, `items.take(count)`,
`items.drop(count)`, `items.append(item)`, `items.append_all(others)`,
`items.sorted()`, `items.contains(item)`, `items.join(separator)`,
`items.sum()`, `items.with_index()` (pairs of `item, index`), `items.to_set()`.
Maps: `map.get(key)` (`maybe V`), `map.set(key: k, value: v)`,
`map.without(key)`, `map.contains_key(key)`, `map.keys()`, `map.values()`.
Sets: `set.contains(item)`, `set.union(other)`, `set.intersection(other)`,
`set.difference(other)`, `set.is_subset_of(other)`, `set.sorted()` (a list).
`items.with_index()` and `map.entries()` return lists of `Pair of Left,
Right`; a two-variable loop header destructures a pair (decision K8). `Bytes`
holds binary data: `bytes.length()`, `bytes.to_text()` (fails with
`InvalidEncoding`), `text.to_bytes()` (decision K4).
Every operation returns a new value; mutation is always `set xs to
xs.append(item)`, and the VM mutates in place when the value is uniquely
referenced. The complete surface of the base types belongs to the standard
library sketch (`04-stdlib-sketch.md`), which the corpus lint checks against.

**Ranges.** `from 1 to 10` is inclusive on both ends; `from 0 to 100 by 5` adds
a step. Lists are zero-indexed, so a loop over indices is `for each position
from 0 to items.length() - 1`.

**Conditionals as expressions.** `if` and `match` may stand where a value is
expected: `let label be if count is 0 then "none" otherwise "some" end`. Every
branch is then a single expression or a way out (`fail`, `fail with`, `return`,
`crash with`), exactly like the right-hand side of `otherwise`; all value
branches have one type, and an `if` expression must have an `otherwise`.

**Conversions.** Always explicit and always methods: `count.to_decimal()`,
`price.to_float()`, `amount.to_text()`, `text.to_integer()`, `text.to_decimal()`
and `text.to_float()` (the three parsers fail with `InvalidNumber`).
`value.rounded(places)` on `Decimal` and `Float`; `value.square_root()` on
`Float`.

---

## 8. Control flow

```
if user.age is at least 18 then
  set adults to adults + 1
otherwise if user.age is at least 13 then
  set teens to teens + 1
otherwise
  set children to children + 1
end

match shape
  when Circle(radius) where radius is greater than 100 then
    return Large
  when Circle(radius) then
    return Small
  when Rectangle(width: w_side, height: h_side) then
    return classify(width: w_side, height: h_side)
  otherwise
    return Unknown
end

for each line in invoice.lines
  set total to total + line.amount
end

for each key, value in settings
  console.print("{key} = {value}")
end

repeat until attempts is at least 3
  set attempts to attempts + 1
  if web.get(url) is not nothing then break end
end
```

- `if condition then` ... `otherwise if condition then` ... `otherwise` ...
  `end`. `then` is mandatory; `end` is mandatory even for one-line bodies. A
  one-line `if` statement has no `otherwise`: an `if` with branches spans
  lines, so that `otherwise` at the `if`'s column is never read as the
  fallback of the statement before it.
- `match value` with `when pattern [where guard] then` branches and an optional
  `otherwise` default. Matching must be exhaustive. Patterns: variant with
  punned fields `Circle(radius)`, renamed fields `Circle(radius: outer)`, a
  bare variant name `Circle` when no field is needed, literals, `nothing`,
  `some(name)`, a typed binding `error: HttpError` for error unions,
  `otherwise` for the rest. A record type is matched like a single variant,
  `GaveUp(attempts)` (decision K6). Binding a field and not using it is a
  compile error (decision J8).
- `for each item in collection` iterates lists, sets, ranges and text
  (by character); `for each key, value in map` destructures pairs. A loop
  header accepts the query clauses `where` and `sorted by`: `for each size in
  sizes sorted by size.characters descending`. `break` and `continue` are
  allowed.
- `repeat until condition` runs the body until the condition holds; the
  condition is tested before each pass, so the body may run zero times
  (decision M9). `while` is a foreign keyword that the compiler rewrites to
  `repeat until` with the opposite condition. There is no bottom-tested loop:
  a flag or `break` covers that case (decision M10).
- Four levels of nesting is the maximum; the fix is a named function.

---

## 9. Errors

```
public type FileError is one of
  NotFound(path: Path)
  PermissionDenied(path: Path)
  InvalidFormat(detail: Text)
end

public function load_config(path: Path) returns Config or fails with FileError
  needs filesystem.read
  purpose: Read and decode the configuration file.

  let text be filesystem.read_text(path) otherwise fail
  let config: Config be json.parse(text) otherwise fail with InvalidFormat(detail: "config is not valid JSON")
  if config.workers is 0 then fail with InvalidFormat(detail: "workers must be positive") end
  return config
end
```

- `or fails with E` in the signature makes the function fallible. Several error
  types are joined with `or`; the compiler builds the anonymous union.
- `fail with ErrorValue` returns a failure. `return value` returns a success;
  success values are wrapped implicitly.
- On a fallible call, `otherwise` is mandatory, and it is allowed only there
  and on a `maybe` value: `otherwise` after a value that cannot fail is a
  compile error with the fix "remove `otherwise`" (decision M3). It takes
  either a default value (`otherwise 0`) or a way out: `otherwise fail` propagates the same
  error, `otherwise fail with E(...)` translates it, `otherwise return value`
  leaves the function, `otherwise break` and `otherwise continue` leave or
  advance a loop, `otherwise crash with "text"` stops the program.
- When the error value itself is needed, match the call: `match action() when
  success(outcome) then ... when failure(error) then ... end`. Inside
  `failure(...)` the usual variant patterns apply: `when failure(NotFound(path))
  then`, `when failure(error: JsonError) then`.
- `crash with "message"` terminates the program. It is for impossible states,
  never for expected failures. The runtime also crashes on arithmetic domain
  errors; there are no other implicit crashes.

---

## 10. Queries

The query form replaces anonymous functions for collection work. It reads as a
set comprehension and compiles to a single fold.

```
let emails be
  for each user in users
  where user.is_active and user.age is at least 18
  sorted by user.created_at descending
  collect user.email

let revenue be for each order in orders where order.is_paid sum order.total
let paid_count be for each order in orders where order.is_paid count
let oldest be for each order in orders sorted by order.created_at first
let all_shipped be for each order in orders all order.is_shipped
let any_refund be for each order in orders any order.is_refunded
let by_country be for each user in users group by user.country collect user.email
let skus be for each order in orders, line in order.lines collect line.sku
let pages be for each url in urls concurrently collect web.get(url) otherwise fail
```

| Clause | Role | Result |
|--------|------|--------|
| `for each x in xs[, y in ys]` | source(s); several sources nest and flatten | |
| `concurrently` | run the body of each iteration as a parallel task | |
| `where condition` | filter | |
| `sorted by key [descending]` | order | |
| `collect expression` | map | `List of T` |
| `sum expression` | add up | numeric |
| `count` | how many | `Integer` |
| `first` | first match | `maybe T` |
| `any condition` / `all condition` | existential / universal | `Boolean` |
| `group by key [terminal]` | partition; a terminal clause after it applies per group | `Map of K to List of T`, or `Map of K to` the terminal's result |

A query on one line is allowed when it fits; otherwise it starts on the line
after `be` with one clause per line.

`group by key` may be followed by any terminal clause, which is then applied
to each group: `for each sale in sales group by sale.region sum sale.amount`
is a `Map of Text to Decimal` (decision M1). The loop variable of a query is
read when a clause mentions it or when the terminal is `first`, which returns
it; `for each line in lines count` leaves `line` unread, which decision J8
rejects with the fix `lines.length()` (decision M2).

---

## 11. Effects and capabilities

A `needs` clause lists the capabilities a function uses directly. The checker
requires callers to declare a superset. A function without `needs` is pure.

Built-in capability tree for v1:

```
console              read and write the terminal
filesystem.read      read files and directories
filesystem.write     create, modify, delete files
network.http         HTTP client
network.socket       raw sockets
environment          environment variables and command-line arguments
time                 the clock
random               random numbers
process              start other processes
foreign              call code across the FFI boundary
```

Naming a parent (`needs filesystem`) grants its children.

**Scopes.** A capability may carry one literal argument that narrows it: a
path prefix for `filesystem` and its children (`filesystem.read("data")`), a
host for `network` and its children (`network.http("api.example.com")`), a
variable name for `environment("HOME")`, a program name for `process("git")`.
`console`, `time`, `random` and `foreign` take no argument. A declaration
without an argument covers every scope. The checker requires a caller to cover
each callee: the same or an ancestor capability, with no argument or with one
that contains the callee's (`filesystem` covers `filesystem.read("data")`,
which covers `filesystem.read("data/2024")`). At run time a primitive compares
the actual path or host with the calling function's declared scope and reports
a mismatch through its ordinary error type: `PermissionDenied(path)` in
`FileError`, `HostNotAllowed(host)` in `HttpError` (decision J11).

A program's entry point declares what the whole program may do:

```
public function main() or fails with AppError
  needs console, network.http
  purpose: Download today's report and print a summary.
  ...
end
```

`renyi run report.ry` grants exactly `console` and `network.http`. `renyi run
--deny network report.ry` fails at startup with the list of functions that need
the denied capability; `--allow-host api.example.com` and `--allow-read data`
narrow a scope from the command line. `example:` clauses run only on pure functions; effectful
code is tested with `test` blocks that declare their own `needs`.

---

## 12. Concurrency

```
run concurrently
  let users be accounts.fetch_all() otherwise fail
  let orders be billing.fetch_open() otherwise fail
end
let report be reports.join(users: users, orders: orders)
```

Each statement inside `run concurrently` is a task. The block waits for all
tasks; the first failure cancels the others and propagates through the
enclosing function's `or fails with`. Bindings made inside are visible after
the block. `for each ... concurrently collect ...` is the parallel query. There
is no `async`, no `await`, no thread handle, and no shared mutable state.

`run concurrently within time.seconds(5)` and `for each url in urls
concurrently within time.seconds(5) collect ...` set a deadline. When it
expires the remaining tasks are cancelled and the block fails with the
built-in `TimedOut`, which the enclosing function lists in `or fails with`
(decision J12).

---

## 13. Documentation clauses

| Clause | Content | Checked by |
|--------|---------|------------|
| `purpose: text` | one sentence, required on public definitions and modules | compiler (presence) |
| `example: expression is value` or `example: expression fails with Pattern` | executable; must hold | `renyi test` |
| `tags: word, word` | retrieval tags | exported by `renyi index` |
| `see also: name, name` | related definitions | compiler (existence) |
| `deprecated: since 2.0, replaced by new_name` | three-tier deprecation (decision C8c) | compiler, `renyi migrate` |
| `expose as tool` | publish as an agent tool | `renyi tools`, `renyi serve --mcp` |

Free text inside `purpose:`, `tags:` and the reason of `deprecated:` is not
subject to reserved words. A clause longer than one line continues on lines
indented deeper than the clause word.

---

## 14. Tests

```
test "total price applies the tax rate to the subtotal"
  let items be [Item(price: 10.00), Item(price: 5.00)]
  check invoices.total_price(items: items, rate: TaxRate(0.1)) is 16.50
end

test "loading a missing config reports the path" needs filesystem.read
  match config.load_config(Path("does-not-exist.json"))
    when NotFound(path) then check path is Path("does-not-exist.json")
    otherwise check false
  end
end
```

`test "name" [needs caps]` ... `end` at the top level of any module. `check
condition` is the assertion. Inside a test, `otherwise fail` and `fail with`
end the test as failed with the error value as the message, so set-up needs no
ceremony: `let older be parse("1.9.9") otherwise fail`. `renyi test` runs tests
and `example:` clauses.

---

## 15. Agent tools

```
public function convert_currency(amount: Decimal, source: Currency, target: Currency)
  returns Decimal
  or fails with RateUnavailable
  needs network.http
  purpose: Convert an amount between currencies at today's rate.
  expose as tool

  ...
end
```

The compiler produces the tool manifest: JSON Schema from the parameter types,
description from `purpose:`, required permissions from `needs`. Parameter and
return types must be representable in JSON (records, variants, lists, maps,
base types).

---

## 16. Canonical formatting

- Two-space indentation, 100-column limit, LF, one trailing newline.
- One blank line between top-level definitions; none inside a clause group;
  one between the clause group and the body.
- Signature clauses share the head line when they fit, otherwise one per line
  indented once, in canonical order.
- A long query starts on the line after `be`, one clause per line.
- Long conditions break before `and` / `or`.
- A statement whose `otherwise` does not fit breaks before `otherwise`, with
  the continuation indented one level.
- Parameters, arguments or fields that do not fit go one per line indented one
  level, with the closing parenthesis on its own line at the head's indentation;
  signature clauses then follow on their own lines.
- A refinement `where` that does not fit breaks before `where` and before each
  `and` / `or`.
- An `if` statement with one statement and no `otherwise` is written on one
  line when it fits (`if done then break end`); every other `if`, and every
  `match`, loop and `run concurrently`, spans lines.
- A `when ... then` or `otherwise` arm keeps a single statement on its line
  when it fits.
- An `example:` that does not fit breaks before `is` or `fails with`, indented
  one level; when the call itself does not fit, its arguments go one per line
  and `) is value` follows the closing parenthesis.
- Documentation prose (`purpose:`, `tags:`, `deprecated:`) wraps at the width
  onto lines indented one level deeper than the clause word.
- A single blank line between statements is kept; several collapse to one.
- A single token that cannot be broken (a long string literal) may exceed the
  width; the formatter leaves it alone.
- Comments stay where they were written: a full-line comment before a
  statement, a list element, a query clause, an `otherwise` line, a `when`
  arm or a field keeps its line, and a comment at the end of a line stays on
  that line; a comment inside a bracketed list or a clause group makes the
  group break one element per line.
- The formatter never changes tokens: it cannot rename, add `end`, or convert
  `=` to `be`. Those are compiler errors with suggested fixes.

---

## 17. Reserved words and phrases

85 reserved words. Any of them used as an identifier is a compile error with a
rename suggestion; after a dot they are allowed as member names.

```
ability all also and any as at be break by can check collect concurrently
continue count crash deprecated descending each end example expose exposing
fail fails failure false first for from function greater group has if ignore
import in is lazy least less let match maybe module most mutable needs not
nothing of one or otherwise power public purpose raw remainder repeat return
returns run see self set some sorted success sum tags test than then to tool
true type until when where with within
```

Phrase table (single tokens, longest match):

```
is not | is less than | is at most | is greater than | is at least
or fails with | is one of | for each | for any | run concurrently
repeat until | sorted by | group by | see also | expose as tool
```

Words that appear only inside a phrase (`at`, `least`, `most`, `than`, `less`,
`greater`, `also`, `see`, `tool`, `expose`, `sorted`, `group`, `one`, `each`,
`fails`, `run`, `repeat`, `until`) are still reserved so that a word has one role everywhere.

---

## 18. Round 2 items (settled)

The open items R2-1 to R2-17 were decided in round 2 and are recorded as
entries J1 to J17 of `01-decisions.md`; the sketch above reflects them. New
open items start at R3-1 and are listed here when they arise.
