# Renyi Language Reference

Status: normative for the surface syntax, the static rules and the run-time
behaviour of Renyi version 1, from decision V12 (2026-10-06). It is derived
from the syntax sketch (`design/02-syntax-sketch.md`, now the design
record), the formal grammar (`grammar.ebnf`) and the implementation under
`crates/`. Where this document and the sketch differ, this document holds
and the difference is a defect, settled by a decision entry
(`design/01-decisions.md`); where this document is silent, the sketch and
the decisions apply. The library (the methods of the base types and the
modules under `std`) is specified in `design/04-stdlib-sketch.md` and
declared in `library/std/`; this document names a library function only
where a language rule depends on it.

Two tests hold this document to the implementation. Every grammar excerpt
below is a rule of `grammar.ebnf`, quoted verbatim, and every rule of that
file is quoted here (`crates/renyi_syntax/tests/reference.rs`); the rules
of that file accept exactly the programs the parser accepts
(`crates/renyi_syntax/tests/grammar.rs`). Appendix A lists every
diagnostic code the implementation emits, and only those (the same test).

Each section has four parts: the grammar, the meaning, the static rules
with the code of the diagnostic that reports a violation, and what happens
at run time. A code is an error unless it is marked as a warning; `renyi
check` exits with status 1 when any error is reported, and a program with
an error does not run. Every diagnostic carries a fix, one line that says
what to write instead (decision D3).

---

## 0. Reading this document

**Notation.** Grammar excerpts are W3C EBNF, as in the XML specification:
`A ::= B` defines `A`; `A B` is a sequence; `A | B` a choice; `A?`, `A*`
and `A+` are optional, zero-or-more and one-or-more; `( )` groups; `A - B`
matches what `A` matches unless `B` matches the same tokens; `'text'` is a
token with that spelling. The terminals are the tokens of section 1, by
their spelling for reserved words, phrases and punctuation, and by class
for the rest: `Identifier`, `TypeName`, `Member`, `ReservedWord`,
`Integer`, `Decimal`, `Text`, `PlainText`, `RawText`, `ClauseText` and
`Newline`.

**The surface is frozen.** Decision V11 froze the grammar; a change to the
surface is a new decision entry first, then the sketch, the grammar, the
cheat sheet, the formatter and the conformance suite in one commit.

**The implementation.** One binary, `renyi`, with the commands of appendix
B. The conformance suite under `tests/conformance/` is the contract
between this document and any implementation: programs with the output,
the exit status and the diagnostic codes an implementation must produce.

---

## 1. Lexical structure

The lexical level turns the source text into tokens. The grammar of
sections 2 to 15 is written over those tokens; nothing in it depends on
spaces or indentation.

### 1.1 Source text

A source file is UTF-8 without a byte-order mark, with LF line endings. A
carriage return is an error (`crlf`); a tab character is an error (`tab`);
indentation is two spaces and carries no meaning. A line wider than 100
columns, counted in characters, is a warning (`line-width`), and spaces or
tabs at the end of a line are a warning (`trailing-whitespace`); `renyi
format` removes both.

A comment runs from `#` to the end of the line. Comments are not tokens:
the parser never sees them, and the formatter keeps them where they stand.
Comments are for why; what belongs in `purpose:` clauses (section 13).

### 1.2 Tokens

| Token | Form | Notes |
|-------|------|-------|
| reserved word, phrase | section 17 | a phrase (`is at least`, `for each`) is one token: its words separated by exactly one space, on one line; the lexer takes the longest phrase |
| `Identifier` | `[a-z][a-z0-9_]*` | a value, function, field, parameter or module-segment name; not a reserved word (1.3) |
| `TypeName` | `[A-Z][A-Za-z0-9]*` | a type, ability or variant name |
| `Member` | any word directly after `.` | reserved words included: `event.type`, `items.first()` |
| `ReservedWord` | a reserved word that is not a phrase | only as the name of a method (section 3) |
| `Integer` | `[0-9][0-9_]*` | `1_000_000`; an underscore separates digits and cannot end the literal |
| `Decimal` | `[0-9][0-9_]*\.[0-9]+` | `19.99` |
| `Text` | `"..."` on one line, or `"""` block text | with holes and escapes (1.5) |
| `PlainText` | a `Text` without holes | where the language needs a constant: a test name, a capability scope, a recording's path, an external name |
| `RawText` | the `"..."` after `raw` | no holes, no escapes, one line |
| `ClauseText` | the rest of the line after `purpose:`, `tags:`, `see also:` or `deprecated:` | one token (1.6) |
| punctuation | `( ) [ ] { } , : . + - * /` | |
| `Newline` | a line break that ends a statement, clause or head | 1.7 |

Every other character is an error. The symbols of other languages name the
Renyi spelling in their fix: `=` (`equals-sign`: `let`, `change` or `is`),
`<` and `>` with the operators built on them (`angle-comparison`: the four
ordering phrases), `;` (`semicolon`), and `! & | % ^ ~ ? @ $ ` \ '`
(`symbolic-operator`: `and`, `or`, `not`, `remainder`, `power`); any other
character is `unknown-character`. A keyword of another language where a
statement or an item starts (`while`, `def`, `else`, `null`, `class`,
`switch`, `throw`, ...) is reported with the Renyi form (`foreign-keyword`).

### 1.3 Names

A value name is at least two characters (`single-letter-identifier`), has
no leading, trailing or doubled underscore (`identifier-shape`), and is not
a reserved word (`reserved-word`, with a rename). A name in another
language's casing is one token with the Renyi spelling as its fix: a
camelCase word where a value name is expected is `identifier-shape`
(`userName` becomes `user_name`), a PascalCase word where a value name is
expected is `identifier-shape`, a snake_case word where a type is expected
is `type-name-shape` (`user_record` becomes `UserRecord`), and a type name
with an underscore is `type-name-shape`.

After a dot any word is a member name, so a method may be declared under a
reserved word (`function first(self: List of Item)`): a method is only ever
called after a dot (decision N1). The member follows its dot directly;
`user. name` is an error (`expected`, "a member name directly after the
dot").

### 1.4 Numbers

An `Integer` literal is a whole number of any size; it takes the type
`Decimal` or `Float` when the context expects one. A `Decimal` literal is
exact; it takes the type `Float` only when the context expects `Float`. A
literal that runs into letters, or ends with an underscore, is
`number-shape`. A `-` directly before a number literal negates the
literal; before anything else it is an error (`unary-minus`): a computed
value is negated as `0 - value`.

### 1.5 Text

A `"..."` literal stays on its line; a quote that is not closed on the line
is `unterminated-text`. The escapes are `\n`, `\t`, `\"`, `\\` and `\{`;
any other `\` sequence is `unknown-escape`. `{expression}` is a hole: the
text interpolates the value, which must have the ability `ToText` (section
7). A hole holds exactly one expression (`hole-syntax`), is not empty
(`empty-hole`), holds no text literal (`text-in-hole`: bind the text to a
name first) and is closed on the line (`unterminated-hole`). A literal
brace is written `\{`.

Block text starts with `"""` and nothing else on that line
(`block-text-start`), runs from the next line to the line before the
closing `"""`, which stands alone on its line (`unterminated-block-text`),
loses the indentation common to its non-blank lines, joins its lines with
LF and has no trailing line break. Holes and escapes apply as in a
one-line literal.

`raw "..."` is text with no holes and no escapes, for regular expressions
and for JSON or SQL samples; it stays on one line.

A `PlainText` is a text literal without holes. The language needs one for
a test's name, a capability's scope, a recording's path and a field's
external name; a hole there is an error (`expected`, "plain text").

### 1.6 Clause text

When one of the words `purpose`, `tags`, `see also` or `deprecated` starts
a line (only spaces before it) and is followed directly by `:`, the rest
of the line, trimmed, is one `ClauseText` token: reserved words and
punctuation have no meaning in it. The colon directly after the word is
what makes the clause; `purpose : text` is an error (`expected`, "the
text of the clause").

### 1.7 Lines

A line break is a `Newline` token, except in four places, where it is not a
token at all: inside parentheses, brackets or braces; after a comma; before
a continuation word; and after a line break that is already a token (a run
of line breaks is one `Newline`). The file ends with a `Newline`, supplied
when the last line has no line break. The continuation words are `where`,
`sorted by`, `group by`, `collect`, `sum`, `count`, `first`, `any`, `all`,
`returns`, `or fails with`, `needs`, `for any`, `with`, `and`, `or`, `is`
and `fails`; the list is one list, read without context (decision V12).
`otherwise` is not a continuation word: at the start of a line it is the
branch of an `if` or a `match`, so the `otherwise` that guards a value stays
on the line of that value (`otherwise-line`), and a long statement breaks
inside parentheses instead.

A `Newline` ends a statement, a clause, a field, a variant, an arm's head
or a head (the line that opens a block), as the grammar shows. Two line
rules are not visible in the grammar: `return` keeps its value on its line,
since `return` alone is a statement, and the value after `be` or `to` may
start on the next line. The word that closes a block (`end`, `otherwise`,
`when`) may follow a statement on its line (`if done then break end`); it
may not follow a head, a field or a variant (`expected`, "write `end` on
its own line").

---

## 2. Modules and imports

### Grammar

```ebnf
Module              ::= Newline? ModuleHeader (Import | Item)*
Declarations        ::= Newline? ModuleHeader (Import | DeclaredItem)*
ModuleHeader        ::= 'module' DottedName Newline DocClause*
DottedName          ::= Identifier ('.' Member)*
Import              ::= 'import' DottedName ('as' Identifier)?
                        ('exposing' TypeName (',' TypeName)*)? Newline
Item                ::= 'public'? (Function | TypeDefinition | Ability | Constant)
                      | Implementation
                      | Test
DeclaredItem        ::= 'public'? (FunctionDeclaration | TypeDefinition | Ability | Constant)
                      | ImplDeclaration
                      | Implementation
                      | Test
```

### Meaning

One file is one module. The file starts with `module name`
(`module-header`); the name is the file's path from the project root with
`/` replaced by `.`, and `.ry` and `.renyi` are equivalent extensions. The
documentation clauses of section 13 follow the header; the module's
`purpose:` is required. Then imports and items, in any order; definitions
are private unless marked `public`, and the public ones are the module's
API, which `renyi index --diff` reads to decide version numbers.

`import a.b.c` brings the module in as the namespace `c`, so that its
functions are called `c.function(...)`; `as` renames the namespace.
`exposing` lists types and abilities to use unqualified; exposing a sum
type exposes its variants too. Functions are never imported unqualified:
every call names its module, which is what makes a chunk of code
self-describing. There is no wildcard import. The functions of the current
module are called unqualified.

The prelude (`library/std/prelude.ry`) is in scope without an import: the
base types `Integer`, `Decimal`, `Float`, `Boolean`, `Text`, `Bytes`,
`List`, `Map`, `Set`, `Range`, `Pair`, `Duration` and `Ordering`, the
optional type `maybe`, the core abilities of section 5, the built-in error
types `ConstraintViolation`, `InvalidNumber`, `InvalidEncoding`, `TimedOut`
and `Guarded`, and the methods of the base types. The modules `std.console`,
`std.environment`, `std.time`, `std.random`, `std.filesystem`, `std.json`,
`std.http`, `std.server`, `std.csv`, `std.sqlite`, `std.regex`,
`std.process`, `std.foreign` and `std.python` are imported by name, and
so are the modules of the extensions a toolchain is built with (decision
AJ1; the guide is `extensions.md`), which `renyi version` lists.

A program's own imports resolve from its project root: the directory of
the nearest `renyi.json` in the file's directory or above it (up to the
working directory for a relative path), else the file's directory
(decision J17). `renyi.json` (decision AC1) names the project's
dependencies and their registry, a directory or a URL, (decision AF1)
its foreign modules, each with the shared libraries it is bound to, and
(decision AL1) its Python modules, each with the Python module it is
bound to, and the interpreter that runs them;
`import <name>` and
`import <name>.<path>` reach a dependency's root module `<name>.ry` or its
file `<path>.ry`, at the version the lockfile `renyi.lock.json` beside the
manifest names, read from the registry directory or, for a URL registry,
from `.renyi/packages/<name>/<version>/` under the project root. Inside a
package a module is named from the package's root, and the checker names it
`<name>.<module>` (the root module `<name>`); an import there reaches the
package's own files, or one of the package's own dependencies.

`Declarations` is the form of a library declaration file: the same module,
with every function a head and its clauses and no body, an
implementation's methods among them (`ImplDeclaration`).

### Static rules

- The module name equals the path (`module-name`), and the module has a
  `purpose:` (`purpose-missing`).
- An import names a module that exists (`unknown-module`); a module whose
  import has errors reports them once as `import-errors` and is checked no
  further against it.
- A dependency of the manifest is in the lockfile and at the registry
  (`package-missing`), its `package.json` is the one the lockfile names
  (`package-mismatch`, and the package is read all the same), and
  `renyi.json`, `renyi.lock.json` and a `package.json` are readable, with
  only the fields their formats have (`manifest-invalid`); the three are
  reported on the import, or at the start of the file for the manifest and
  the lockfile.
- A name is declared once per module (`duplicate-name`): a type, an
  ability, a function or a constant, with the methods of one type counted by
  their receiver; a namespace is imported once.
- A private definition of another module is not visible (`private-name`);
  `exposing` names a public type or ability.
- A namespace name is taken within the module: no binding or parameter may
  be called like an import (`shadowing`, "is the namespace of an import").

### At run time

A program is a module with a `main` function (section 11) and the modules
it imports, compiled together. Nothing runs at module level: a constant is
evaluated on first use (section 6).

---

## 3. Functions

### Grammar

```ebnf
Function            ::= FunctionHead Newline DocClause* Block 'end' Newline
FunctionDeclaration ::= FunctionHead Newline DocClause*
FunctionHead        ::= 'function' (Identifier Parameters | ReservedWord MethodParameters) Signature
Parameters          ::= '(' (Parameter (',' Parameter)*)? ')'
MethodParameters    ::= '(' SelfParameter (',' Parameter)* ')'
Parameter           ::= SelfParameter | Identifier ':' Type
SelfParameter       ::= 'self' (':' Type)?
Signature           ::= ('returns' Type)? ('or fails with' Type ('or' Type)*)?
                        ('needs' Capabilities)? ForAny?
ForAny              ::= 'for any' TypeName (',' TypeName)*
                        ('where' Constraint ('and' Constraint)*)?
Constraint          ::= TypeName 'can' Type
```

### Meaning

A function is a head, its signature clauses, its documentation clauses
(section 13), a blank line and a body of statements closed by `end`.
Parameters are typed; there are no default values and no variadic
parameters. The signature clauses come in one order, each optional:
`returns Type` names the result (omitted when the function returns
nothing); `or fails with E1 or E2` names the error types the function can
fail with (section 9); `needs` lists the capabilities its body uses
directly (section 11); `for any T, U where T can Ability` introduces type
parameters and their constraints, an ability with its type arguments each
(section 4). The clauses share the head
line when they fit in 100 columns, otherwise each stands on its own line;
the tokens are the same.

A **method** is a function whose first parameter is `self`, declared in the
module of its type and called with the dot: `public function trim(self:
Text) returns Text` makes `text.trim()`. A method may be declared under a
reserved word, since it is only ever called after a dot. Inside an ability
or an implementation `self` needs no type (section 5). `Self` names the
implementing type in an ability's declaration.

A **call** writes `name(arguments)`; a method call `receiver.method(args)`
passes the receiver as `self`; a zero-argument method keeps its parentheses
(`items.length()`), which tells it from a field. A call with one argument
is positional; a call with two or more names every argument in declaration
order. A function is passed as a value by name, `retry(action: fetch)`, and
called through a parameter of function type with positional arguments.

A **function type** is `function(Url) returns Page or fails with HttpError
needs network.http`. The effects of a function passed as an argument are
charged where it is passed, whatever the parameter's type lists (decision
B1); a higher-order function declares only its own effects.

### Static rules

- A public function has a `purpose:` (`purpose-missing`).
- `self` is the first parameter (`self-position`); a method is declared in
  the module that declares its type (`method-module`); a method is called
  on a value (`method-call`) and a function that is not a method is not
  called with a dot (`not-a-method`).
- A parameter is declared once (`duplicate-name`).
- A function that `returns` a type ends every path with `return`, `fail
  with` or `crash with` (`missing-return`).
- A body spans at most 60 source lines (`body-length`) and nests `if`,
  `match`, `for each`, `repeat until` and `run concurrently` at most four
  deep (`nesting-depth`); the fix is a named function.
- A call passes as many arguments as there are parameters
  (`argument-count`); with one argument no name, with two or more every
  name, in declaration order (`argument-name`, `argument-order`); a call
  through a function type is positional. The callee exists
  (`unknown-function`, `unknown-method`, with the closest name or the
  module to import as the fix) and is a function (`not-callable`).
- A type parameter takes no type arguments, a generic type takes as many
  as it declares, and a constraint names an ability with as many as it
  declares (`type-arity`); every type named exists
  (`unknown-type`).
- `example:` lines belong to pure functions only (`example-effects`).

### At run time

Arguments are evaluated left to right; a function value is the function
itself, not a closure, since there is nothing to capture. A function
passed as an argument runs under the grant in force where it is called, so
the higher-order function's own `needs` narrows it (decisions Q1 and V9).

---

## 4. Types

### Grammar

```ebnf
Type                ::= 'maybe' Type
                      | FunctionType
                      | TypeName ('of' TypeArguments)?
TypeArguments       ::= Type 'to' Type | Type (',' Type)*
FunctionType        ::= 'function' '(' (Type (',' Type)*)? ')' ('returns' Type)?
                        ('or fails with' Type ('or' Type)*)? ('needs' Capabilities)?
TypeDefinition      ::= 'type' TypeName TypeParameters?
                        (RecordDefinition | SumDefinition | SubtypeDefinition)
TypeParameters      ::= 'of' TypeName (',' TypeName)*
RecordDefinition    ::= Newline DocClause* (FieldLine | Derive)* 'end' Newline
FieldLine           ::= 'has' Identifier ':' Type ('where' Expression)? ('as' PlainText)? Newline
Derive              ::= 'can' TypeName ('by' Identifier (',' Identifier)*)? Newline
SumDefinition       ::= 'is one of' Newline DocClause* (Variant | Derive)* 'end' Newline
Variant             ::= TypeName ('(' VariantField (',' VariantField)* ')')? Newline
VariantField        ::= Identifier ':' Type ('where' Expression)?
SubtypeDefinition   ::= 'is' Type ('where' Expression)? Newline DocClause*
```

### Meaning

One keyword, four clause words: a product with `has`, a sum with `is one
of`, a subset with `where`, an ability with `can`.

A **type expression** is a type name with its arguments after `of`
(`List of Text`, `Map of Text to Integer`, `Pair of Text, Integer`),
`maybe T` for an optional value, or a function type. Type arguments nest to
the right: in `List of Pair of Text, Integer` the comma belongs to `Pair`.

A **record** (`has`) has public fields. Construction names every field,
`User(name: "Ann", age: 30, email: nothing)`; a `maybe` field still needs
its argument. Update copies with changes: `user with age: 31`. `has kind:
Text as "type"` gives a field the external name that `ToJson`, `FromJson`
and `FromRow` use, for keys that are reserved words or contain punctuation.

A **sum type** (`is one of`) lists variants, each a record with zero or
more fields: `Circle(radius: Decimal)`, `Point`. A variant without fields
is written bare, in a declaration, a construction and a pattern.
Matching is exhaustive (section 8).

A **refinement** (`where`) is a pure Boolean condition over the field's
name, or over `value` for a refined subtype. When every refined argument
of a construction is a literal, the condition is checked at compile time
and the construction is ordinary: `Circle(radius: 2.5)`. When a refined
argument is a run-time value the construction can fail, and the program
says so with `otherwise` by the rule of section 9: `Email(input) otherwise
fail with BadInput(detail: "x")`. The error is the built-in
`ConstraintViolation(type_name, detail)`. An update with `with` of a
refined field is checked the same way (decision U9).

A **subtype** (`type UserId is Integer`, `type Email is Text where ...`)
is accepted wherever its base is and keeps the methods of the base; the
base is never accepted as the subtype. Going down is a construction,
`UserId(7)`, fallible when the subtype is refined. Arithmetic on two
subtype values gives the base; wrap the result to keep the name. This is
the one place the type system has subtyping. Three library types are
checked when built from a literal: a `Url` is absolute, with a scheme and a
host, and a `Path` is not empty and holds no control character
(`invalid-literal`); a `Date` built from three literals has a day its month
has (`constraint-violation`); a `Pattern` is a valid regular expression, in
the syntax of the Rust regex crate (`regex-invalid`).

**Derived abilities** (`can`): every type that is not a function type has
`Equal`; `can Compare by field, field` derives the order by those fields;
`can ToText`, `can ToJson`, `can FromJson` and `can Hash` derive from the
structure, and `can FromRow` (`std.sqlite`) matches columns to fields. An
ability with other methods is implemented by hand (section 5).

**Generics.** `type Pair of Left, Right` introduces type parameters, in
scope for the fields; a function introduces them with `for any` (section
3). A constraint gives its ability the type arguments the ability
declares, any types in scope: `for any Bag, Item where Bag can Iterable
of Item`. In the body the parameter is walked by `for each` with items of
type `Item`, and the ability's methods are called on it with `Item` in
place of the ability's parameter (`bag.to_list()` is a `List of Item`).
A call passes a type that has the ability with matching arguments, those
of its implementation with the implementation's parameters read as the
type's arguments, which binds what the constraint leaves open: `Item` is
inferred from `Deck`'s `Iterable of Text` (decision AB1).

### Static rules

- A construction names every field of a record and no other
  (`argument-name`, `argument-count`, `unknown-field`); a sum type is
  constructed by one of its variants (`construct-sum`); a library type
  without fields is made by a function of its module (`construct-opaque`);
  a type name alone is not a value (`type-as-value`); a variant with fields
  is not written bare (`missing-fields`); when two sum types in scope share
  a variant's name, the context says which or the binding is annotated
  (`ambiguous-variant`).
- A refinement that a literal fails is reported where the literal stands
  (`constraint-violation`); a refined construction or update from a
  run-time value takes `otherwise` (`missing-otherwise`).
- A field's condition reads that field alone, and a subtype's condition
  `value`; a condition that names another field of the type is
  `refinement-field` (decision Y4).
- A variant of a type with `ToJson` or `FromJson` has no field named `kind`,
  the key JSON uses for the variant's name (`kind-field`, decision K10).
- `can` names an ability in scope (`unknown-ability`), and `by` names
  fields of the type (`unknown-field`).
- A type is declared once (`duplicate-name`); a type name resolves
  (`unknown-type`); a generic type takes the arguments it declares
  (`type-arity`).

### At run time

A record is a value; every operation returns a new value. A refined
construction or update evaluates its condition and gives a failure
`ConstraintViolation` when it does not hold. Two values of one type with
`Equal` compare by structure; a subtype value is its base value with the
subtype's name.

---

## 5. Abilities

### Grammar

```ebnf
Ability             ::= 'ability' TypeName TypeParameters?
                        ('where' Requirement ('and' Requirement)*)? Newline
                        DocClause* MethodDeclaration* 'end' Newline
Requirement         ::= 'self' 'can' Type
MethodDeclaration   ::= FunctionHead Newline DocClause*
Implementation      ::= 'ability' TypeName ('of' Type (',' Type)*)? 'for' Type ForAny? Newline
                        Function* 'end' Newline
ImplDeclaration     ::= 'ability' TypeName ('of' Type (',' Type)*)? 'for' Type ForAny? Newline
                        MethodDeclaration* 'end' Newline
```

### Meaning

An ability declares functions over `self`; `Self` in a declaration names
the implementing type. A type gains an ability through an implementation
block, `ability Describable for Shape ... end`, or a `can` derivation
(section 4); the methods are called with the dot. An ability may require
others of its implementing types: `ability Printable where self can
ToText`, with the ability's own parameters in scope for the arguments of
a requirement (`ability Countable of Item where self can Iterable of
Item`). An ability may take type parameters after `of`; an implementation
names the arguments, any types, in its head: `ability Iterable of Card
for Deck`, `ability Iterable of Pair of Key, Value for Map of Key to
Value`. An implementation for a generic type introduces the type
parameters with a `for any` clause after its head. Default method bodies
are not in version 1 (decision J10).

The core abilities of the prelude: `Equal` (`equals`), `Compare`
(`compare`, returning `Ordering`: `Less`, `Same`, `Greater`), `Hash`
(`hash`), `ToText` (`to_text`), `ToJson`, `FromJson` and `Iterable of Item`
(`to_list(self) returns List of Item`). `for each` and the queries of
section 10 walk any type that implements `Iterable`, through the list
`to_list` returns (decision V10); the prelude implements `Iterable` for
`List`, `Set`, `Map` (its pairs), `Range` and `Text` (its characters), so
that a constraint `Bag can Iterable of Item` accepts them and `to_list`
exists on each (decision AB1).

### Static rules

- An implementation defines every method the ability declares
  (`missing-method`) and no other (`unknown-method`), with the declared
  signature once `Self` and the
  ability's type parameters are substituted (`method-signature`); an
  ability's method never declares `needs`, since an ability's methods have
  no effects (`method-signature`).
- An implementation for a type without an ability the ability requires,
  with the arguments the requirement names once the ability's parameters
  are substituted, is an error (`missing-ability`); a type parameter
  constrained to an ability has what that ability requires.
- An ability is named with as many type arguments as it declares, in an
  implementation head, a constraint and a requirement (`type-arity`); a
  type implements an ability once, whatever the arguments
  (`duplicate-implementation`); an implementation's methods are as
  visible as the ability (decision AB1).
- An implementation is never `public` (`public-implementation`); a method
  is never `public` by itself, the ability's visibility covers it
  (`public-method`).
- A public ability has a `purpose:` (`purpose-missing`); an ability named
  in `can`, a constraint or a requirement exists (`unknown-ability`).

### At run time

A method call dispatches on the type of `self`; an implementation is
chosen when the program is compiled, since every value's type is known.
`Equal` and `Hash` of a derived type follow its structure; `Compare by`
compares the listed fields in order. A declared `equals` decides `is` and
`is not`; the collections (`Set` items, `Map` keys, `distinct`, `to_set`,
`contains`, `index_of`) compare and hash by structure and consult no
declared `equals` or `hash` (decision Y3).

---

## 6. Bindings and mutation

### Grammar

```ebnf
Constant            ::= 'let' Identifier ':' Type 'be' Newline? Expression Newline DocClause*
```

The statements `let`, `let mutable` and `change` are alternatives of the
`Statement` rule of section 8.

### Meaning

`let name be expression` binds a name once; `let name: Type be expression`
pins the type, which is how a generic result such as `json.parse` learns
its target. `let mutable name be expression` binds a local that `change
name to expression` changes; `change` is the only way to change it, and a
mutable binding never escapes its function. A name is bound once per
function: rebinding it, or shadowing it in a nested scope, is an error;
sibling scopes (two loop bodies, two `when` arms) may reuse a name. A
top-level `let` is a constant: it has a type, and a public one has a
`purpose:`.

The language has no `=`.

### Static rules

- A name is bound once per function (`shadowing`); it is not the namespace
  of an import nor the name of a function of the module (`shadowing`).
- Every binding is used: a `let`, a parameter, a loop variable or a pattern
  binding that is never read is an error with the fix "remove it, or use
  it" (`unused-binding`, decision J8); the loop variable of a query whose
  terminal is `count` is unread, and the fix is the source's `length()`
  (decision M2).
- A call whose result is not used is an error with the likely fix
  (`change items to items.append(item)`) (`unused-result`); `ignore call`
  discards the result of a call that has effects; discarding the result of
  a pure call is dead code (`ignore-pure`), and `ignore` on a call that
  returns nothing is an error (`ignore-nothing`) (decisions J15, R6).
- `change` changes a mutable binding (`immutable-binding`) that exists
  (`unknown-name`); a name that is read exists (`unknown-name`, with the
  closest name, the foreign spelling such as `null` for `nothing`, or the
  module to import as the fix).
- A `let` binds a value: a call that returns nothing has none to bind
  (`type-mismatch`).
- A `let` without a type annotation gives a number literal its own type,
  `Integer` or `Decimal`; the binding keeps it (`type-mismatch` where the
  other is needed). With an annotation the literal takes the annotated type
  when it fits (section 7).

### At run time

A binding holds a value; `change` replaces it. A constant is evaluated on
its first use and kept; a constant whose construction fails crashes the
program ("the constant `name` could not be built").

---

## 7. Expressions

### Grammar

```ebnf
Expression          ::= OrExpression ('otherwise' Outcome)*
Outcome             ::= OrExpression
                      | 'fail' ('with' OrExpression)?
                      | 'return' OrExpression?
                      | 'crash' 'with' OrExpression
                      | 'break'
                      | 'continue'
OrExpression        ::= AndExpression ('or' AndExpression)*
AndExpression       ::= NotExpression ('and' NotExpression)*
NotExpression       ::= 'not' NotExpression | Comparison
Comparison          ::= WithExpression (ComparisonWord WithExpression)?
ComparisonWord      ::= 'is' | 'is not' | 'is less than' | 'is at most'
                      | 'is greater than' | 'is at least'
WithExpression      ::= Additive ('with' FieldUpdate (',' FieldUpdate)*)?
FieldUpdate         ::= Identifier ':' Additive
Additive            ::= Multiplicative (('+' | '-') Multiplicative)*
Multiplicative      ::= Power (('*' | '/' | 'remainder') Power)*
Power               ::= Postfix ('power' Power)?
Postfix             ::= Receiver ('.' Member Arguments?)*
Receiver            ::= Identifier Arguments?
                      | TypeName Arguments?
                      | Literal
                      | 'nothing'
                      | 'self'
                      | '(' Expression ')'
                      | Range
                      | IfExpression
                      | MatchExpression
                      | Query
Call                ::= Identifier Arguments
                      | Receiver ('.' Member Arguments?)* '.' Member Arguments
Arguments           ::= '(' (Argument (',' Argument)*)? ')'
Argument            ::= (Identifier ':')? Expression
Literal             ::= Integer | Decimal | '-' (Integer | Decimal)
                      | Text | 'raw' RawText
                      | 'true' | 'false'
                      | ListLiteral | MapLiteral
ListLiteral         ::= '[' (Expression (',' Expression)*)? ']'
MapLiteral          ::= '{' (MapEntry (',' MapEntry)*)? '}'
MapEntry            ::= OrExpression ':' Expression
Range               ::= 'from' Additive 'to' Additive ('by' Additive)?
```

### Meaning

Precedence, high to low: member access and calls; `power` (right
associative); `* / remainder`; `+ -`; `with`; the six comparison phrases
(not chained); `not`; `and`; `or`; `otherwise`. Parentheses group.

**Arithmetic** works on two values of one numeric type: `Integer`
(unbounded), `Decimal` (IEEE 754 decimal128, decision J16: exact for
literals, sums and products of everyday values, division rounded half-even
at the 34th digit) or `Float` (IEEE 754 binary64). A number literal takes
the type its context expects. `/` divides `Decimal` or `Float` values; two
`Integer`s divide with `dividend.quotient(divisor)` (decision J9). `power`
takes an `Integer` exponent for an `Integer` or `Decimal` base and a `Float`
exponent for a `Float` base; `remainder` is the remainder of a division.
There is no implicit conversion: `count.to_decimal()`, `price.to_float()`,
`amount.to_text()`, `text.to_integer()`, `text.to_decimal()`,
`text.to_float()` (the parsers fail with `InvalidNumber`).

**Comparison.** `is` and `is not` compare two values of one type that has
`Equal`; a `maybe T` compares with `nothing` and with a `T`. The four
ordering phrases need `Compare`. Two `Decimal`s compare by numeric value,
`32.0 is 32.00` (decision R5).

**Logic.** `and`, `or` and `not` take `Boolean`s; `and` and `or`
short-circuit.

**Text.** A literal interpolates its holes with `ToText`. A literal brace is
`\{`.

**Optionals.** `nothing` is the absent value of every `maybe T`. A `T` is
accepted where a `maybe T` is expected; reading a `maybe T` out names the
other case, with `otherwise` or a `match` (section 8).

**Collections.** `[1, 2, 3]` is a `List`; `{"a": 1}` is a `Map` in JSON
notation (decision J2), its keys needing `Hash`; a `Set` is built with
`to_set()`; `from 1 to 10` is a `Range`, inclusive at both ends, with an
optional step `by`. Lists are zero-indexed. The operations of the base
types are methods listed in `design/04-stdlib-sketch.md`. A count given to
`repeat`, `pad_left`, `pad_right` or `rounded` fits a machine word; a
larger `Integer` crashes with `this Integer is too large for the
operation` (decision Y3).

**Update.** `value with field: expression, field: expression` copies a
record with the named fields changed; a refined field is checked again
(section 4).

**`otherwise`.** `expression otherwise outcome` reads a `maybe` or the
success of a fallible call: the outcome is a default value of the inner
type, or a way out: `fail`, `fail with E(...)`, `return [value]`, `break`,
`continue`, `crash with "text"` (section 9).

**Conditionals as expressions.** `if` and `match` stand where a value is
expected (section 8).

### Static rules

- Both operands of an arithmetic operator have one numeric type
  (`type-mismatch`; the fix for texts is interpolation); `/` does not
  divide two `Integer`s (`integer-division`).
- The sides of a comparison have one type (`type-mismatch`); `is` needs
  `Equal` and the ordering phrases need `Compare` (`missing-ability`);
  conditions are `Boolean` (`type-mismatch`).
- A hole's value, and what `console.print` prints, has `ToText`; a map key
  has `Hash` (`missing-ability`).
- A `maybe T` is not used where a `T` is needed (`maybe-value`); the fix
  is `otherwise` or a `match`.
- A `with` update names fields of the record (`argument-name`,
  `unknown-field`).
- A number literal fits its context: an `Integer` literal is an `Integer`,
  `Decimal` or `Float`; a `Decimal` literal is a `Decimal` or `Float`; a
  literal given to a refined subtype meets the condition (`type-mismatch`,
  `constraint-violation`).

### At run time

Integer arithmetic never overflows, but an `Integer` exponent is between
zero and one million. A `Decimal` operation whose result does not fit
decimal128, a `Float` operation whose result is not finite, a division by
zero (`/`, `quotient`, `remainder`), the square root of a negative number,
a conversion of a `Float` that is not finite and rounding to more places
than the type holds crash the program (section 9). `is` on two `Decimal`s compares by value; on records
and variants by structure; on a `maybe` by presence and then value.

---

## 8. Control flow

### Grammar

```ebnf
Block               ::= Newline? (Statement Newline)* Statement?
Statement           ::= 'let' 'mutable'? Identifier (':' Type)? 'be' Newline? Expression
                      | 'change' Identifier 'to' Newline? Expression
                      | 'if' Expression 'then' Block
                        ('otherwise' 'if' Expression 'then' Block)* ('otherwise' Block)? 'end'
                      | 'match' Expression Newline MatchArm* ('otherwise' Block)? 'end'
                      | 'for each' LoopSource ('where' Expression)? Ordering? Newline Block 'end'
                      | 'repeat until' Expression Newline Block 'end'
                      | 'run concurrently' ('within' Expression)? Newline Block 'end'
                      | 'return' Expression?
                      | 'fail' ('with' Expression)?
                      | 'crash' 'with' Expression
                      | 'break'
                      | 'continue'
                      | 'ignore' Expression
                      | 'check' Expression
                      | Call
                      | OrExpression ('otherwise' Outcome)+
MatchArm            ::= 'when' Pattern ('where' Expression)? 'then' Block
LoopSource          ::= Identifier (',' Identifier)* ('in' (OrExpression - Range) | Range)
Ordering            ::= 'sorted by' OrExpression 'descending'?
Pattern             ::= TypeName ('(' FieldPattern (',' FieldPattern)* ')')?
                      | 'nothing'
                      | ('some' | 'success' | 'failure') '(' Pattern ')'
                      | Identifier (':' Type)?
                      | Literal
FieldPattern        ::= Identifier (':' Pattern)?
IfExpression        ::= 'if' Expression 'then' Newline? Outcome Newline?
                        ('otherwise' 'if' Expression 'then' Newline? Outcome Newline?)*
                        'otherwise' Newline? Outcome Newline? 'end'
MatchExpression     ::= 'match' Expression Newline? (MatchExpressionArm Newline?)*
                        ('otherwise' Newline? Outcome Newline?)? 'end'
MatchExpressionArm  ::= 'when' Pattern ('where' Expression)? 'then' Newline? Outcome
```

### Meaning

A **statement** is one of the keyword forms, a call, or a value guarded by
`otherwise`; one statement per line. A block ends with `end`.

**`if`.** `if condition then ... otherwise if condition then ... otherwise
... end`; `then` and `end` are mandatory. An `if` with one statement and no
`otherwise` fits on one line: `if done then break end`.

**`match`.** `match value` with `when pattern [where guard] then` arms and
an optional `otherwise` arm last; the arms cover every case of the subject.
Patterns: a variant with punned fields `Circle(radius)`, renamed fields
`Circle(radius: outer)`, a bare variant name when no field is needed,
`nothing`, `some(name)`, a literal, a typed binding `error: HttpError`, a
name that binds the whole value. A record type is matched like a single
variant, `GaveUp(attempts)` (decision K6). A fallible call as the subject
is matched with `success(value)` and `failure(error)` arms, and inside
`failure(...)` the usual patterns apply to the error (section 9).

**`for each`.** `for each item in collection` walks a `List`, a `Set`, a
`Map` (as pairs: `for each key, value in map`), a `Range`, a `Text` (by
character) or any type with an `Iterable` implementation (section 5), a
type parameter constrained to one among them (section 4); a
loop over a range literal drops `in`: `for each index from 1 to 10 by 2`,
the one spelling. The head accepts `where condition` and `sorted by key
[descending]`. `break` leaves the loop, `continue` advances it.

**`repeat until condition`** runs the body until the condition holds; the
condition is tested before each pass, so the body may run zero times
(decision M9). There is no bottom-tested loop and no `while`.

**`run concurrently`** is section 12; `return`, `fail` and `crash` are
section 9; `ignore` is section 6; `check` is section 14.

**Conditionals as expressions.** `if` and `match` stand where a value is
expected when every branch is a single outcome: a value, or a way out
(`fail`, `fail with`, `return`, `crash with`, `break`, `continue`). The
value branches have one type; an `if` expression has an `otherwise`; the
`otherwise` arm of a `match` expression is the last. `let label be if done
then "yes" otherwise "no" end`.

### Static rules

- A statement is a call, a guarded value, or starts with a keyword
  (`statement-shape`); a block has its `end` (`missing-end`).
- A `match` is exhaustive (`not-exhaustive`, naming the cases the arms
  miss): the variants of a sum type and the cases of their fields, `nothing`
  and `some` of a `maybe`, `true` and `false` of a `Boolean`, `success` and
  `failure` of a fallible call with each of its declared errors; a subject
  whose cases cannot be listed (a number, a text) needs `otherwise`. A
  pattern fits the subject
  (`pattern-mismatch`): a variant of its type, `nothing` or `some` on a
  `maybe`, `success` or `failure` on a fallible call, a typed binding
  naming one of the errors. A guard is a `Boolean` (`type-mismatch`).
- A loop binds one variable, or two for a map or a list of pairs
  (`loop-variables`); the source can be walked (`type-mismatch`, naming
  `Iterable`); a `sorted by` key has `Compare` (`missing-ability`); a range
  loop is spelled without `in` (`range-loop`).
- `break` and `continue` stand inside a loop (`outside-loop`).
- A statement after `return`, `fail` or `crash` in the same block can never
  run (`unreachable`, a warning).
- Four levels of nesting is the limit (`nesting-depth`, section 3).

### At run time

`if` evaluates its conditions in order and runs the first branch whose
condition holds. `match` tries its arms in order; a guard that fails moves
to the next arm. `for each` evaluates its source once, then walks it: a
list in order, a set and a map in insertion order, a range from its start
to its end in steps, a text by character, an `Iterable` through the list
`to_list` returns; `where` skips items; `sorted by` orders the items by
the key before the first pass, stably. A range whose step is zero crashes.
A `repeat until` loop tests its condition before each pass.

---

## 9. Errors

### Grammar

The forms belong to the `Signature` rule of section 3 (`or fails with`),
the `Expression` and `Outcome` rules of section 7 (`otherwise` and the
ways out) and the `Statement` rule of section 8 (`fail`, `return`,
`crash`).

### Meaning

A function that can fail says so: `or fails with E` in its signature;
several error types are joined with `or` into an anonymous union. `fail
with ErrorValue` returns a failure; `return value` returns a success, and
success values are wrapped implicitly: a `T` is accepted where a success
is expected.

On a fallible call `otherwise` is mandatory, and it is allowed only there
and on a `maybe` value (decision M3). It takes a default value, or a way
out: `otherwise fail` propagates the same error; `otherwise fail with
E(...)` translates it; `otherwise return [value]` leaves the function;
`otherwise break` and `otherwise continue` leave or advance the enclosing
loop; `otherwise crash with "text"` stops the program. When the error value
itself is needed, the call is matched: `match action() when
success(outcome) then ... when failure(error) then ... end`, with the usual
patterns inside `failure(...)`: `when failure(NotFound(path)) then`, `when
failure(error: JsonError) then`.

`crash with "message"` terminates the program with the message. It is for
impossible states, never for expected failures. The runtime also crashes on
the arithmetic conditions of section 7, on a constant that fails to build
(section 6), on a range whose step is zero, and where a library primitive
is given what its declaration cannot refuse: a `Float` that is not finite
where JSON or a `Decimal` is made, a closed database connection, a port it
cannot listen on, a regular expression built at run time that is not valid.
The message says which.

### Static rules

- A fallible call is read with `otherwise` or matched
  (`missing-otherwise`); `otherwise` after a value that can neither fail
  nor be `nothing` is an error with the fix "remove `otherwise`"
  (`superfluous-otherwise`); `otherwise fail` on a `maybe` has no error to
  pass on (`otherwise-fail-maybe`).
- `fail` alone stands only after `otherwise` (`bare-fail`); `fail with`
  names a value of a type the function declares, and `otherwise fail`
  passes on errors the function declares (`error-not-declared`).
- `return` carries a value when the function returns one
  (`missing-value`) and none when it does not, or in a test
  (`return-value`).
- The message of `crash with` is a `Text` (`type-mismatch`).

### At run time

A failure is a value that travels up through the `otherwise` of each call
until a default, a way out or a `match` takes it. A failure that leaves
`main` ends the program: `renyi run` prints `main failed with` and the
error's text to the standard error and exits with status 1. A crash
prints `crash:` and the message, with the location when it is known, and
exits with status 2. `environment.exit(code)` ends the program with that
status.

---

## 10. Queries

### Grammar

```ebnf
Query               ::= 'for each' LoopSource (',' LoopSource)*
                        ('concurrently' ('within' OrExpression)?)?
                        ('where' OrExpression)? Ordering?
                        ('group by' OrExpression QueryTerminal? | QueryTerminal)
QueryTerminal       ::= 'collect' Expression
                      | 'sum' Expression
                      | 'count'
                      | 'first'
                      | 'any' Expression
                      | 'all' Expression
```

### Meaning

A query is the one form for collection work; the language has no anonymous
functions. It reads as a set comprehension and compiles to a single fold.

| Clause | Role | Result |
|--------|------|--------|
| `for each x in xs[, y in ys]` | the sources; several nest and flatten | |
| `concurrently [within d]` | run each iteration as a task (section 12) | |
| `where condition` | filter | |
| `sorted by key [descending]` | order | |
| `collect expression` | map | `List of T` |
| `sum expression` | add up | the numeric type of the expression |
| `count` | how many | `Integer` |
| `first` | the first item | `maybe T` |
| `any condition`, `all condition` | existential, universal | `Boolean` |
| `group by key [terminal]` | partition; a terminal after it applies per group | `Map of K to List of T`, or `Map of K to` the terminal's result |

A query on one line is allowed when it fits; otherwise it starts on the
line after `be` with one clause per line, which the continuation words of
section 1 allow. `group by key` may be followed by any terminal, which is
then applied to each group: `for each sale in sales group by sale.region
sum sale.amount` is a `Map of Text to Decimal` (decision M1).

### Static rules

- A query ends with a terminal or `group by` (`query-shape`).
- The loop variable is read by a clause or by `first`, which returns it;
  `for each line in lines count` leaves `line` unread, which is an error
  with the fix `lines.length()` (`unused-binding`, decisions J8 and M2).
- `sum` adds numbers (`type-mismatch`); `any` and `all` take `Boolean`s
  (`type-mismatch`); a `group by` key has `Hash` and a `sorted by` key has
  `Compare` (`missing-ability`); a `within` deadline is a `Duration` and
  needs `TimedOut` declared (section 12).
- The rules of loops apply to the sources (section 8).

### At run time

The sources are evaluated once, nested sources for each item of the outer;
`where` skips; `sorted by` orders before the terminal, stably; `collect`
keeps the order; `group by` keeps the groups in the order their keys first
appear. Over no items `collect` gives the empty list, `sum` zero, `count`
zero, `first` `nothing`, `any` false and `all` true. `concurrently` runs
the iterations as the tasks of section 12.

---

## 11. Effects and capabilities

### Grammar

```ebnf
Capabilities        ::= Capability (',' Capability)*
Capability          ::= CapabilityPath Budget? ('only to' CapabilityPath ('or' CapabilityPath)*)?
CapabilityPath      ::= DottedName ('(' PlainText ')')?
Budget              ::= 'at most' Integer 'per' ('second' | 'minute' | 'hour' | 'day' | 'run')
```

### Meaning

A `needs` clause lists the capabilities a function uses directly; callers
declare a superset; a function without `needs` is pure. The tree:

```
console              read and write the terminal
filesystem.read      read files and directories
filesystem.write     create, modify, delete files
network.http         HTTP client
network.socket       raw sockets
environment          environment variables and command-line arguments
time                 the clock
random               random numbers
process              start other programs (`std.process`)
foreign              call the C functions of a foreign module (decision AF1)
python               call the functions of a Python module through the bridge (decision AJ3)
```

A parent (`filesystem`, `network`) covers its children. A capability may
carry one literal **scope** that narrows it: a path prefix for
`filesystem` and its children, a host for `network` and its children, a
variable name for `environment`, a program name for `process`, a package
for `python`; `console`, `time`, `random` and `foreign` take none. A
declaration without a scope covers every scope. A granted capability
covers a needed one when its path is the same or an ancestor and its
scope contains the needed scope: a path prefix contains its sub-paths
(`filesystem.read("data")` covers `filesystem.read("data/2024")`), a
host, a variable, a program name or a package contains only itself; a
needed capability without a scope is covered by a
scoped grant only when the callee checks its actual path or host at run
time, which the library primitives do.

The entry point declares the program's whole grant:

```
public function main() or fails with AppError
  needs console, network.http("api.example.com") at most 60 per minute
```

Two **grant clauses** may follow a capability in the `needs` of `main` or
of a `test`, and only there. `at most COUNT per UNIT` is a budget (decision
P2): the units are `second`, `minute`, `hour` and `day` (sliding windows)
and `run` (the whole program); budgets apply to `network`, `process` and
`filesystem` and their children. `only to SINK or SINK` is a guard
(decision P3): every value that entered the program through the capability
carries that origin, as does every value computed from it, and it may
leave the program only through the listed sinks.

### Static rules

- Every capability a body uses, directly or through a call, is covered by
  the function's `needs` (`capability-missing`); a function passed as an
  argument is charged where it is passed (section 3). A call into a
  dependency (decision AC1) is covered like a library call: the kind of
  the capability statically, its scope by the grant stack at run time,
  and the diagnostic names the package ("`announce` (package `greeting`
  1.0.0) needs `console`, which `main` does not declare").
- A **foreign module** (decision AF1) is a declaration file of the
  project that the manifest's `foreign` section binds to shared
  libraries (`"foreign": {"libc": {"library": ["ucrtbase", "libc.so.6",
  "libSystem.B.dylib"], "symbols": {"renyi_name": "c_symbol"}}}`: the
  libraries tried in order, `symbols` optional); the main file is one by
  its path from the project root, an import by its qualified name. Each
  of its functions needs `foreign` and nothing else, declares no failures
  and no type parameters (`foreign-signature`); its parameters are the
  width types of `std.foreign`, `Float`, `Boolean`, `Text` or `Bytes` and
  its result one of those but `Bytes`, `maybe Text` or nothing
  (`foreign-type`); it takes at most six words, `Bytes` counting two
  (`foreign-arity`). `renyi bind` writes such a file from a C header.
- A **Python module** (decision AL1) is a declaration file of the
  project that the manifest's `python` section binds to a Python module
  (`"python": {"interpreter": "python3", "modules": {"analysis":
  {"package": "analysis", "symbols": {"renyi_name": "python_name"}}}}`:
  `interpreter`, `package` (the module's own name when left out) and
  `symbols` optional); it is found by its path or its qualified name as
  a foreign module is. Each of its functions needs
  `python("<package>")`, the package of its binding, and nothing else,
  fails with `PythonError` of `std.python` and nothing else and takes no
  type parameters (`python-signature`); its parameters are types that
  can `ToJson` and its result a type that can `FromJson`, or nothing
  (`python-type`), since a call crosses as JSON both ways; `renyi bind
  --python` writes such a file from the package (decisions AM1 and
  AM2). The guide is `python.md`.
- A sink after `only to` is a capability of the tree
  (`unknown-capability`); a scope is a text literal (`capability-scope`).
- A budget or a guard stands in the `needs` of `main` or a test
  (`grant-clause`); a guard whose sinks are all outside the grant can let
  nothing leave (`guard-no-sink`, a warning).
- Across a package boundary only the capability kind is checked
  statically; the scope is the grant stack's business.

### At run time

`renyi run program.ry` grants exactly what `main` needs. A primitive checks
its actual path or host against the effective grant, the intersection of
the program's grant with the declared scopes along the call chain (the
grant stack, decision Q1), and reports a mismatch through its module's
error type: `PermissionDenied(path)` in `FileError`,
`HostNotAllowed(host)` in `HttpError`, `PermissionDenied(path)` in
`DbError`, `PermissionDenied(port)` in `StartError`,
`ProgramNotAllowed(program)` in `ProcessError` (decisions J11, Y1, AE1); a
call past a budget is the module's `OverBudget` the same way. The variant
is found by its name and its one field among the function's declared
failure types, with the scope as the field (the first argument when the
capability carries no scope, as `serve` names its port), so a function of
an extension reports through its own type (decision AK4). A primitive
that cannot fail (`filesystem.exists`, `environment.get`) crashes instead,
naming the function whose `needs` narrowed the grant. A path scope
contains a path by its text once `.`, `..` and the separators are
normalized: nothing is resolved on disk, so a symbolic link under the
scope reaches wherever it points. The command line
narrows the grant and can only tighten it: `--deny capability` refuses to
start when any function needs it, `--allow-host`, `--allow-read` and
`--allow-write` narrow a scope, `--at-most capability=count/unit` adds a
budget.

A budget keeps a counter per capability and scope; the primitive call that
would exceed it fails with the module's `OverBudget` variant
(`HttpError.OverBudget(host)`, `FileError.OverBudget(path)`), which the
program handles like any failure. A guard tags the values a guarded
capability produces and everything computed from them; an outgoing
primitive (a `console` print, a `filesystem.write`, an `http` request's
URL, headers or body) that would send a tagged value toward a sink the
guard does not list fails with the built-in `Guarded(origin, sink)` before
anything leaves. The design is `design/06-runtime-guarantees.md`.

A function of a foreign module (decision AF1) is a primitive bound at its
first call: the first library of its list that loads is kept for the run
and the symbol looked up; the arguments are marshalled by their C types
(`Text` as a NUL-terminated copy, `Bytes` as a pointer and a length, an
integer by its width) and the result read back by its (a narrower integer
by its width, `maybe Text` nothing for a null pointer). A library or a
symbol that is missing, a `Text` holding a NUL character and a null
pointer where `Text` was declared are crashes at the call. The call is
recorded like any primitive and a replay answers it without calling; a
guarded value refuses to cross; `foreign` takes no scope and no budget;
`renyi run` says "this program can call native code through `libc`" on
its standard error when `main` grants `foreign`. What the C function does
is outside every guarantee of the VM.

A function of a Python module (decision AL1) is a primitive that runs in
the worker of the bridge (decision AL2): one Python process per run,
started at the first call with the project root on its module path and
ended with the VM, found through the manifest's `interpreter`, else the
variable `RENYI_PYTHON`, else `python3` and `python` on the PATH
(decision AL3). The arguments cross as `std.json` renders them, by
position in the declared order; the result comes back as JSON and is
read by the declared type; a result the declaration does not mention is
dropped. What the Python function prints goes to the standard error. The
call fails with `PythonError` (decision AL4): `Raised(exception,
message)` when the function let an exception escape,
`NotCarried(detail)` when the result does not fit the declared type or
JSON does not carry it, `Unavailable(detail)` when no interpreter
answers, the module does not import, the function is not there or the
worker ended (the next call starts one again), `PermissionDenied(package)`
for a package outside the grant. The call is recorded like any primitive
and a replay answers it without starting the worker; a guarded value
refuses to cross; `python` takes a scope and no budget; `renyi run` says
"this program can run Python through `analysis`" on its standard error
when `main` grants `python`. What the Python side does is outside every
guarantee of the VM.

---

## 12. Concurrency

### Grammar

`run concurrently` is an alternative of the `Statement` rule (section 8);
`concurrently` and `within` are clauses of the `Query` rule (section 10).

### Meaning

```
run concurrently
  let users be accounts.fetch_all() otherwise fail
  let orders be billing.fetch_open() otherwise fail
end
```

Each statement inside `run concurrently` is a task. The block waits for
all tasks; the first failure cancels the others and propagates through the
enclosing function's `or fails with`. Bindings made inside are visible
after the block. `for each ... concurrently collect ...` is the parallel
query. There is no `async`, no `await`, no thread handle and no shared
mutable state.

`run concurrently within time.seconds(5)` and `for each url in urls
concurrently within time.seconds(5) collect ...` set a deadline. When it
expires the remaining tasks are cancelled and the block fails with the
built-in `TimedOut(after)`, which the enclosing function lists in `or fails
with` (decision J12).

### Static rules

- A task is independent: it does not read a binding another task of the
  block makes, and it does not change a binding made outside the block
  (`task-independence`).
- A `within` deadline is a `Duration` (`type-mismatch`), and the enclosing
  function declares `TimedOut` (`error-not-declared`).
- `break` and `continue` do not leave the block (`outside-loop`).

### At run time

The first runtime executes the tasks of a block or a query one after the
other, in source order, and checks the deadline between them (decision
S2); the meaning is the same, since tasks are independent, and a later
runtime may overlap their waits. A failed task stops the block: the tasks
after it do not run, and the failure propagates.

---

## 13. Documentation clauses

### Grammar

```ebnf
DocClause           ::= ('purpose' | 'tags' | 'see also' | 'deprecated') ':' ClauseText Newline
                      | 'expose as tool' Newline
                      | 'example' ':' Example Newline
Example             ::= Expression 'fails' 'with' Pattern
                      | WithExpression 'is' WithExpression
```

### Meaning

The clauses follow the signature of a function, the head of a type, an
ability or a constant, and the `module` line; the canonical order, which
`renyi format` writes, is `purpose:`, `tags:`, `see also:`, `deprecated:`,
`expose as tool`, then any number of `example:` lines. A blank line
separates them from a body.

| Clause | Content | Checked by |
|--------|---------|------------|
| `purpose: text` | one sentence; required on the module, every public definition and every tool | the checker (presence) |
| `example: expression is value`, `example: expression fails with Pattern` | executable; must hold | `renyi test` |
| `tags: word, word` | retrieval tags | exported by `renyi index` |
| `see also: name, name` | related definitions | the checker (existence) |
| `deprecated: since 2.0, replaced by new_name` | the deprecation of decision C8c | the checker |
| `expose as tool` | publish as an agent tool (section 15) | `renyi tools`, `renyi mcp` |

The text of `purpose:`, `tags:`, `see also:` and `deprecated:` runs to the
end of its line and is not subject to reserved words (section 1). An
`example:` holds an expression and its outcome: `is value`, or `fails with
Pattern`; a long one breaks before `is` or `fails with`.

### Static rules

- The module, every public function, type, ability and constant, and
  every function exposed as a tool has a `purpose:` (`purpose-missing`).
- `see also:` names a function, type, constant or ability of the module,
  `module.name` of an import, or `Type.method` (`unknown-reference`).
- A call to a deprecated definition is a warning (`deprecated`) and an
  error under `renyi check --strict`.
- An `example:` has one of the two shapes (`example-shape`); `fails with`
  stands on a call that can fail (`example-fails`); its expected value has
  the result's type and the result has `Equal`; a function with examples is
  pure (`example-effects`).

### At run time

`renyi test` evaluates every `example:` of every file it is given and
reports one that does not hold, with the value it got.

---

## 14. Tests

### Grammar

```ebnf
Test                ::= 'test' PlainText ('needs' Capabilities)? ('replays' PlainText)? Newline
                        Block 'end' Newline
```

### Meaning

`test "name" [needs caps] [replays "path"] ... end` stands at the top level
of any module. `check condition` is the assertion. Inside a test,
`otherwise fail` and `fail with` end the test as failed with the error
value as the message, so set-up needs no ceremony. `renyi test` runs the
tests and the `example:` clauses of the files it is given.

`replays "fixtures/forecast.json"` runs the test against a recording
(decision P1): a file written by `renyi record` that holds every effect
call of one real run (capability, primitive, arguments, result or failure,
time and random values). During the test every effectful call is answered
from the recording, so the test is deterministic and offline; a call the
recording does not hold fails the test and names the call. The test's
`needs` still names the capabilities, because the recording is checked
against them.

### Static rules

- `check` belongs in a test (`check-outside-test`); a test returns nothing
  (`return-value`).
- A test's `needs` covers what its body uses (`capability-missing`), and it
  may carry the grant clauses of section 11.
- A test's name and a recording's path are plain text (section 1).

### At run time

A test runs under its own grant; the recording of a `replays` test names
the grant it was made under, which the test's `needs` must cover, as it
must cover every recorded call (decision Y2). A `check` whose condition is false fails
the test with the condition's text; a failure that reaches the test's end
fails it with the error's text; a test that reaches `end` passes. `renyi
test` exits with status 1 when any test or example fails. `renyi test
--strict` also reports the entries of a recording the test never reached;
`renyi test --refresh "name" [--redact name]` re-records one test's fixture
by running its body live under its `needs`; `--explain` narrates the run.

---

## 15. Agent tools

### Grammar

`expose as tool` is a documentation clause (section 13).

### Meaning

A function with `expose as tool` is published to agents. `renyi tools
[path]` prints the manifest: a JSON array with one object per tool, whose
keys are `name` (`module.function`), `description` (the `purpose:`),
`input_schema` (one property per parameter), `output_schema` (from
`returns`, null for none), `fails` (the declared error types),
`permissions` (from `needs`), `module`, `function`, `file` and `line`.
The schemas follow the `ToJson` and `FromJson` rules of the library
sketch; a type that refers to itself is written once under `$defs`. `renyi
mcp [path]` serves the toolchain, the manifest among it, to an agent host
over standard input and output.

### Static rules

- A tool's parameters have `FromJson` and its result has `ToJson`
  (`tool-type`); a tool has a `purpose:`, which is its description
  (`purpose-missing`).

### At run time

A tool is an ordinary function; the host that calls it decodes the
arguments with `FromJson`, runs it under the grant its `needs` names, and
encodes the result with `ToJson`.

---

## 16. Canonical formatting

`renyi format` owns all layout and has no options; `renyi format --check`
reports a file that differs from its canonical form. The canonical form:

- Two-space indentation, 100-column limit, LF, one trailing newline.
- One blank line between top-level definitions; none inside a clause
  group; one between the clause group and the body; the clauses in the
  order of section 13.
- Signature clauses share the head line when they fit, otherwise one per
  line indented once, in canonical order; a `needs` clause wider than the
  line breaks after its commas, one capability per line indented once
  more; an `exposing` list wider than the line breaks after its commas
  likewise, one name per line indented once.
- A long query after `be` or `to` starts on the next line, one clause per
  line; after `return` it stays on the line and its clauses nest under it.
- Long conditions break before `and` and `or`.
- `otherwise` never starts a line: a statement that does not fit breaks
  inside the parentheses of the longer argument list, the call's before
  `otherwise` or the fallback's, then the other's, then both. A record
  update that does not fit breaks before `with`.
- Parameters, arguments or fields that do not fit go one per line indented
  one level, with the closing parenthesis on its own line at the head's
  indentation; signature clauses then follow on their own lines.
- A refinement `where` that does not fit breaks before `where` and before
  each `and` or `or`.
- An `if` statement with one statement and no `otherwise` is written on
  one line when it fits; every other `if`, and every `match`, loop and `run
  concurrently`, spans lines. A `when ... then` or `otherwise` arm keeps a
  single statement on its line when it fits.
- An `example:` that does not fit breaks before `is` or `fails with`,
  indented one level; when the call itself does not fit, its arguments go
  one per line and `) is value` follows the closing parenthesis.
- Documentation prose stays on its line whatever its width; a single token
  that cannot be broken (a long text literal) may exceed the width.
- A single blank line between statements is kept; several collapse to one.
- Comments stay where they were written; a comment inside a bracketed list
  or a clause group makes the group break one element per line.
- The formatter never changes tokens. What it would have to normalise is
  an error with a fix instead (decision V12): a trailing comma in a
  bracketed list, a variant's empty parentheses, a space after a dot, an
  `otherwise` arm before the last of a `match` expression, a head and `end`
  on one line, a hole in a plain text, `public` on a method.

---

## 17. Reserved words and phrases

88 reserved words. Any of them used as a name is an error with a rename
(`reserved-word`); after a dot they are allowed as member names, and a
method may be declared under one.

```
ability all also and any as at be break by can change check collect
concurrently continue count crash deprecated descending each end example
expose exposing fail fails failure false first for from function greater
group has if ignore import in is lazy least less let match maybe module most
mutable needs not nothing of one only or otherwise per power public purpose
raw remainder repeat replays return returns run see self some sorted success
sum tags test than then to tool true type until when where with within
```

Phrase table (17 phrases, single tokens, longest match, exactly one space
between the words):

```
is not | is less than | is at most | is greater than | is at least
or fails with | is one of | for each | for any | run concurrently
repeat until | sorted by | group by | see also | expose as tool
at most | only to
```

Words that appear only inside a phrase (`at`, `least`, `most`, `than`,
`less`, `greater`, `also`, `see`, `tool`, `expose`, `sorted`, `group`,
`one`, `each`, `repeat`, `until`, `only`) are reserved so that a word has
one role everywhere; `fails` also stands alone in `fails with`, and `run`
as the unit of a budget; `lazy` is reserved for a later version (decision
B4).

The continuation words of section 1: `where`, `sorted by`, `group by`,
`collect`, `sum`, `count`, `first`, `any`, `all`, `returns`, `or fails
with`, `needs`, `for any`, `with`, `and`, `or`, `is`, `fails`.

---

## Appendix A. Diagnostics

Every code the implementation emits, with its severity and the section that
states the rule. `E` is an error, `W` a warning; `deprecated` is a warning
that `renyi check --strict` makes an error.

| Code | Severity | Section |
|------|----------|---------|
| `ambiguous-variant` | E | 4 |
| `angle-comparison` | E | 1 |
| `argument-count` | E | 3 |
| `argument-name` | E | 3, 4, 7 |
| `argument-order` | E | 3 |
| `bare-fail` | E | 9 |
| `block-text-start` | E | 1 |
| `body-length` | E | 3 |
| `capability-missing` | E | 11 |
| `capability-scope` | E | 11 |
| `check-outside-test` | E | 14 |
| `constraint-violation` | E | 4, 7 |
| `construct-opaque` | E | 4 |
| `construct-sum` | E | 4 |
| `crlf` | E | 1 |
| `deprecated` | W | 13 |
| `duplicate-implementation` | E | 5 |
| `duplicate-name` | E | 2, 4 |
| `empty-hole` | E | 1 |
| `equals-sign` | E | 1 |
| `error-not-declared` | E | 9, 12 |
| `example-effects` | E | 3, 13 |
| `example-fails` | E | 13 |
| `example-shape` | E | 13 |
| `expected` | E | 1 |
| `external-name` | E | 4 |
| `foreign-arity` | E | 11 |
| `foreign-keyword` | E | 1 |
| `foreign-signature` | E | 11 |
| `foreign-type` | E | 11 |
| `grant-clause` | E | 11 |
| `guard-no-sink` | W | 11 |
| `hole-syntax` | E | 1 |
| `identifier-shape` | E | 1 |
| `ignore-nothing` | E | 6 |
| `ignore-pure` | E | 6 |
| `immutable-binding` | E | 6 |
| `import-errors` | E | 2 |
| `integer-division` | E | 7 |
| `invalid-literal` | E | 4 |
| `kind-field` | E | 4 |
| `line-width` | W | 1 |
| `loop-variables` | E | 8 |
| `manifest-invalid` | E | 2 |
| `maybe-value` | E | 7 |
| `method-call` | E | 3 |
| `method-module` | E | 3 |
| `method-signature` | E | 5 |
| `missing-ability` | E | 5, 7, 8, 10 |
| `missing-end` | E | 8 |
| `missing-fields` | E | 4 |
| `missing-method` | E | 5 |
| `missing-otherwise` | E | 4, 9 |
| `missing-return` | E | 3 |
| `missing-value` | E | 9 |
| `module-header` | E | 2 |
| `module-name` | E | 2 |
| `nesting-depth` | E | 3, 8 |
| `not-a-method` | E | 3 |
| `not-callable` | E | 3 |
| `not-exhaustive` | E | 8 |
| `number-shape` | E | 1 |
| `otherwise-fail-maybe` | E | 9 |
| `otherwise-line` | E | 1 |
| `outside-loop` | E | 8 |
| `package-mismatch` | E | 2 |
| `package-missing` | E | 2 |
| `pattern-mismatch` | E | 8 |
| `private-name` | E | 2 |
| `public-implementation` | E | 5 |
| `public-method` | E | 5 |
| `purpose-missing` | E | 2, 3, 5, 13, 15 |
| `python-signature` | E | 11 |
| `python-type` | E | 11 |
| `query-shape` | E | 10 |
| `range-loop` | E | 8 |
| `refinement-field` | E | 4 |
| `regex-invalid` | E | 4 |
| `reserved-word` | E | 1, 17 |
| `return-value` | E | 9, 14 |
| `self-position` | E | 3 |
| `semicolon` | E | 1 |
| `shadowing` | E | 2, 6 |
| `single-letter-identifier` | E | 1 |
| `statement-shape` | E | 8 |
| `superfluous-otherwise` | E | 9 |
| `symbolic-operator` | E | 1 |
| `tab` | E | 1 |
| `task-independence` | E | 12 |
| `text-in-hole` | E | 1 |
| `tool-type` | E | 15 |
| `trailing-whitespace` | W | 1 |
| `type-arity` | E | 3, 4 |
| `type-as-value` | E | 4 |
| `type-mismatch` | E | 6, 7, 8, 9, 10, 12 |
| `type-name-shape` | E | 1 |
| `unary-minus` | E | 1 |
| `unknown-ability` | E | 4, 5 |
| `unknown-capability` | E | 11 |
| `unknown-character` | E | 1 |
| `unknown-escape` | E | 1 |
| `unknown-field` | E | 4, 7 |
| `unknown-function` | E | 3 |
| `unknown-method` | E | 3 |
| `unknown-module` | E | 2 |
| `unknown-name` | E | 6 |
| `unknown-reference` | E | 13 |
| `unknown-type` | E | 3, 4 |
| `unreachable` | W | 8 |
| `unterminated-block-text` | E | 1 |
| `unterminated-hole` | E | 1 |
| `unterminated-text` | E | 1 |
| `unused-binding` | E | 6, 10 |
| `unused-result` | E | 6 |

`external-name` reports an `as` that is not followed by a text literal
(section 4); `capability-scope` a scope that is not a text literal, or one
on a capability that takes none (section 11).

---

## Appendix B. The implementation

One binary, `renyi` (decision H1), written in Rust under `crates/`; the
Python scripts under `tools/` are development aids.

| Command | Does | Exit status |
|---------|------|-------------|
| `renyi check [--json] [--strict] <file>...` | lex, parse, type and effect check; the layout warnings of section 1 | 1 when any error |
| `renyi format [--check] <file>...` | rewrite in canonical layout (section 16) | `--check`: 1 when a file differs |
| `renyi tokens <file>` | the token stream | 1 on a lexical error |
| `renyi parse [--json] [--declarations] <file>` | the syntax tree; `--declarations` reads a library declaration file, whose functions have no bodies | 1 on a parse error |
| `renyi index [--json \| --budgets \| --diff <base>] [path]` | the project map, its budgets, the semantic diff (`design/05-agent-tooling.md`) | |
| `renyi tools [path]` | the tool manifest (section 15) | |
| `renyi run [options] <file> [arguments]` | check, then run `main` under its grant | 0; 1 when `main` fails; 2 on a crash; the code of `environment.exit` |
| `renyi run --manifest ...` | also print the run manifest (toolchain, the extensions beyond the standard library, code hash, the dependencies with the lockfile's version and hash of each, grant, arguments, environment, outcome, output hash) | as `run` |
| `renyi record [--to <file>] [options] <file> [arguments]` | run `main` and write a recording of its effects, the manifest in its header | as `run` |
| `renyi reproduce <recording> [<file>]` | replay a recording under its manifest and compare the outcome and the output; the code hash and the dependencies must be the manifest's, a different toolchain or extension list is a warning | 1 when they differ |
| `renyi test [--strict] [--refresh <name> [--redact <name>]] [--explain] [--interpret] <file>...` | run every `example:` and `test` | 1 when any fails |
| `renyi compile [--to <file.ryc>] <file>` | check, then write the program as a bytecode file (default `<name>.ryc`); `run`, `record`, `test` and `reproduce` take a `.ryc` file in place of a source | 1 when any error |
| `renyi add <name> [<version>]` | a dependency (decision AC1): the versions chosen for every requirement (the same major, at least the version required, the highest the registry has, the chosen packages' own requirements included), every package fetched and verified, the effects of the package added printed, `renyi.json` and `renyi.lock.json` written | 1 when refused |
| `renyi update [--accept-effects]` | every dependency to the highest version its requirement allows; a version whose effects widen is refused without the flag, and with it when a `main` that reaches the package does not declare the new capability | 1 when refused |
| `renyi audit` | every locked dependency's effects against each `main` that reaches it, and the capabilities of a `main` no dependency uses | 1 when a `main` does not cover a dependency it reaches |
| `renyi fetch` | the locked packages from the registry, each file verified against its hash and the effect manifest against the sources; into `.renyi/packages/` for a URL registry | 1 when refused |
| `renyi publish [--to <directory>]` | the project, checked clean, into a directory registry as a new version with its `package.json` (the files' hashes, the effect manifest); the version must be what the semantic diff against the highest published version demands (decision G1), and a published version is never overwritten | 1 when refused |
| `renyi bind <header.h> --module <name> --library <name>[,<name>...] [--to <directory>]` | a C header's prototypes as a foreign module (decision AF1): the declaration file `<name>.ry` in canonical layout, a prototype the boundary cannot carry left as a comment with the reason, and the module's entry in the `renyi.json` of the directory (written when the manifest exists, printed otherwise) | 1 when refused |
| `renyi bind --python <package> [--module <name>] [--to <directory>]` | a Python package's functions as a Python module (decisions AM1 and AM2): the declaration file `<name>.ry` (named as the package when `--module` is left out) from what the interpreter of decision AL3 reports of the package, the types of AM2 with `JsonValue` where they stop and a comment naming what stands as `JsonValue`, a function the bridge cannot call by position left as a comment with the reason, and the module's entry in the `renyi.json` of the directory (written when the manifest exists, printed otherwise) | 1 when refused, when no interpreter answers or when the package does not import |
| `renyi mcp [path]` | serve the toolchain to an agent host | |
| `renyi version` | the toolchain's version, then one line per extension it is built with (decision AK1) | |

The options of `run` and `record`: `--explain` narrates the run on the
standard error; `--replay <recording>` (run only) answers every effect from
the recording; `--deny`, `--allow-host`, `--allow-read`, `--allow-write`
and `--at-most` narrow the grant (section 11); `--redact <name>` keeps a
secret out of a recording; `--profile` counts every operation, call and
primitive call, samples where the time goes, and prints the report on the
standard error when the run ends (decision X4). By default the VM runs
the hot code objects of a program as machine code it generates in the
process (decision AG1), with the same results; `--interpret` (also an
option of `test`) keeps everything on the interpreter, as `--explain`
and `--profile` do by themselves.

A bytecode file is the derived JSON of the types of `compiler/bytecode.ry`
(decision Z1): `format` first, then the modules with their source paths
and line starts, the prelude's ids, the types, the implementations, the
abilities, the functions with their signatures and grants, the
constants, the tests, the examples and the code objects, each a list of
operations with a span per operation and its constants; every span
counts characters, every number is a JSON number, and a constant's
digits are a string. `renyi run` and the other commands load it with the
VM's own JSON reader and refuse a file that does not fit, naming the
place (decision Z2); the run manifest's code hash of a program loaded
from a file is the SHA-256 of the file's bytes (decision Z4). The emitter
written in Renyi writes the same document, held equal byte for byte to
`renyi compile`'s by `crates/renyi/tests/selfhost.rs` (decision Z3).

The tree `renyi parse --json` prints is the derived JSON (section 7 of
`design/04-stdlib-sketch.md`) of the types of `compiler/ast.ry`, the
syntax tree of the front end written in Renyi (decision W1): a record is
an object whose keys are its fields in declaration order, a variant an
object with `kind` first and its fields after it, a `maybe` without a
value `null`, and a span `{"start", "stop"}` in characters, the end
exclusive. `renyi run compiler/parse.ry [--declarations] <file>` prints
the same document, and `crates/renyi/tests/selfhost.rs` holds the two
equal byte for byte over the corpus, the conformance programs, the
library and the compiler's own sources. `renyi run compiler/checker.ry
[--json] [--strict] [--library <dir>] <file>...` prints what `renyi
check [--json] [--strict]` prints, computed by the checker written in
Renyi (`compiler/declare.ry` and `compiler/bodies.ry`, decision W5; the
library's declaration files are read from `library/std` under the
working directory unless `--library` names another directory), and the
same test holds the two equal, with and without `--strict`, over the
corpus, the conformance programs and the compiler's own sources
(decision W7).

Diagnostics are printed as `file:line:column: severity [code]: message`
with the fix on the next line, or as JSON with `--json`. The conformance
suite (`tests/conformance/`, decision V8) fixes, for a set of programs,
the output, the exit status and the codes an implementation must produce;
`python tools/conformance.py <binary>` runs it against any binary.
