# Renyi Standard Library Sketch (M0)

Status: draft. Date: 2026-10-05. Companion to `02-syntax-sketch.md`.

This document fixes the names and signatures of everything the example corpus
calls that the language itself does not define: the prelude (base types, their
methods, core abilities, built-in errors), the core modules and the extension
packages. It is a sketch like the syntax sketch: enough to keep thirty programs
consistent and to write the readability test, not a reference manual.

Rules that generate the library:

1. **The prelude has types and methods, no free functions.** `Integer`,
   `Decimal`, `Float`, `Boolean`, `Text`, `Bytes`, `List`, `Map`, `Set`,
   `Range`, `Pair`, `Duration`, `maybe`, the core abilities and the built-in
   error types are available everywhere without an import.
2. **A method is a function whose first parameter is `self: Type`**, declared in
   the module that defines the type (decision K1). `text.trim()` and
   `connection.query(...)` follow the same rule. Abilities exist for
   polymorphism, not for attaching methods.
3. **Modules are imported on demand and called qualified** (`json.parse`).
   Core modules: `std.console`, `std.environment`, `std.time`, `std.random`.
   Extension packages, released in lockstep with the compiler (decision G2):
   `std.filesystem`, `std.json`, `std.http`, `std.server`, `std.csv`,
   `std.sqlite`, `std.regex`.
4. **One error type per module**, a sum type whose variants carry the context a
   caller needs. "Absent" is `maybe`; "went wrong" is a failure.
5. **Effects are declared exactly.** Constructors and conversions are pure;
   anything that touches the world names its capability (syntax sketch,
   section 11).
6. **Names.** Whole English words, no abbreviations (`length`, not `len`).
   Conversions are `to_x()`. Predicates start with `is_`, `contains`,
   `starts_with`, `ends_with`. A method that produces a changed copy is named
   by its action (`append`, `trim`, `sorted`, `without`); nothing mutates.
   Two or more arguments are always named at the call site.

Every declaration below is a Renyi signature without a body. The corpus lint
reads the `function` lines of this file and rejects any method or module
function the examples call that is not declared here.

---

## 1. Prelude

### 1.1 Numbers

Operators: `+ - * /` on two values of one numeric type; `remainder`; `power`
(the exponent is an `Integer` of at least 0 for `Integer` and `Decimal`, any
`Float` for `Float`). `/` is defined on `Decimal` and `Float` only (decision
J9). Division by zero, a negative square root, and overflow of `Float` or
`Decimal` are crashes. `Decimal` is IEEE 754 decimal128 (decision J16);
`Float` is binary64 and never holds NaN or infinity.

```
function to_decimal(self: Integer) returns Decimal
function to_float(self: Integer) returns Float
function to_text(self: Integer) returns Text
function quotient(self: Integer, divisor: Integer) returns Integer
  purpose: Division rounded toward zero; the remainder is `self remainder divisor`.
function absolute(self: Integer) returns Integer
function at_least(self: Integer, other: Integer) returns Integer
  purpose: The larger of the two values.
function at_most(self: Integer, other: Integer) returns Integer
  purpose: The smaller of the two values.

function rounded(self: Decimal, places: Integer) returns Decimal
  purpose: Rounded half away from zero to the given number of decimal places; renders with exactly that many.
function truncated(self: Decimal) returns Integer
  purpose: The integer part, rounded toward zero.
function to_float(self: Decimal) returns Float
function to_text(self: Decimal) returns Text
  purpose: Plain decimal notation, never exponent notation.
function absolute(self: Decimal) returns Decimal
function at_least(self: Decimal, other: Decimal) returns Decimal
function at_most(self: Decimal, other: Decimal) returns Decimal

function rounded(self: Float, places: Integer) returns Float
function truncated(self: Float) returns Integer
function square_root(self: Float) returns Float
function to_decimal(self: Float) returns Decimal
  purpose: The nearest decimal128 value.
function to_text(self: Float) returns Text
  purpose: The shortest text that reads back as the same value, always with a decimal point (5.0).
function absolute(self: Float) returns Float
function at_least(self: Float, other: Float) returns Float
function at_most(self: Float, other: Float) returns Float

function to_text(self: Boolean) returns Text
```

### 1.2 Text and Bytes

`Text` is immutable UTF-8. Indices and lengths count characters (Unicode
scalar values). `Bytes` is an immutable byte sequence (decision K4).

```
function length(self: Text) returns Integer
function is_empty(self: Text) returns Boolean
function trim(self: Text) returns Text
function trim_start(self: Text) returns Text
function trim_end(self: Text) returns Text
function to_lower(self: Text) returns Text
function to_upper(self: Text) returns Text
function split(self: Text, separator: Text) returns List of Text
  purpose: Pieces between separators; adjacent separators give empty pieces.
function lines(self: Text) returns List of Text
  purpose: Split at line breaks, without the breaks.
function characters(self: Text) returns List of Text
function contains(self: Text, part: Text) returns Boolean
function starts_with(self: Text, prefix: Text) returns Boolean
function ends_with(self: Text, suffix: Text) returns Boolean
function index_of(self: Text, part: Text) returns maybe Integer
function replace(self: Text, old: Text, new: Text) returns Text
  purpose: Every occurrence of old replaced by new.
function pad_left(self: Text, width: Integer) returns Text
  purpose: Spaces added on the left until the text is at least width characters.
function pad_right(self: Text, width: Integer) returns Text
function repeat(self: Text, times: Integer) returns Text
function take(self: Text, count: Integer) returns Text
  purpose: The first count characters, or the whole text when it is shorter.
function drop(self: Text, count: Integer) returns Text
function reversed(self: Text) returns Text
function matches(self: Text, pattern: Text) returns Boolean
  purpose: Whether the whole text matches the regular expression (decision K3).
function to_integer(self: Text) returns Integer or fails with InvalidNumber
function to_decimal(self: Text) returns Decimal or fails with InvalidNumber
function to_float(self: Text) returns Float or fails with InvalidNumber
function to_bytes(self: Text) returns Bytes
  purpose: The UTF-8 encoding.
function to_text(self: Text) returns Text
  purpose: The text itself; exists so generic code can call to_text on anything.

function length(self: Bytes) returns Integer
function is_empty(self: Bytes) returns Boolean
function to_text(self: Bytes) returns Text or fails with InvalidEncoding
  purpose: Decode as UTF-8.
function to_base64(self: Bytes) returns Text
```

### 1.3 Collections

`List of Item` is ordered. `Map of Key to Value` and `Set of Item` keep
insertion order (decision K9), so iteration and `keys()` are deterministic;
their keys and items need `Hash`. `Pair of Left, Right` has fields `left` and `right`; a
two-variable loop header (`for each key, value in map`) destructures a pair
(decision K8). `Range` is an inclusive range of `Integer`.

```
function length(self: List of Item) returns Integer for any Item
function is_empty(self: List of Item) returns Boolean for any Item
function at(self: List of Item, index: Integer) returns maybe Item for any Item
  purpose: The item at a zero-based index, or nothing when the index is out of range.
function first(self: List of Item) returns maybe Item for any Item
function last(self: List of Item) returns maybe Item for any Item
function rest(self: List of Item) returns List of Item for any Item
  purpose: Everything after the first item; empty for an empty list.
function without_last(self: List of Item) returns List of Item for any Item
function without_index(self: List of Item, index: Integer) returns List of Item for any Item
function take(self: List of Item, count: Integer) returns List of Item for any Item
function drop(self: List of Item, count: Integer) returns List of Item for any Item
function append(self: List of Item, item: Item) returns List of Item for any Item
function append_all(self: List of Item, others: List of Item) returns List of Item for any Item
function reversed(self: List of Item) returns List of Item for any Item
function sorted(self: List of Item) returns List of Item for any Item where Item can Compare
function distinct(self: List of Item) returns List of Item for any Item where Item can Hash
  purpose: First occurrences only, order kept.
function contains(self: List of Item, item: Item) returns Boolean for any Item
function index_of(self: List of Item, item: Item) returns maybe Integer for any Item
function largest(self: List of Item) returns maybe Item for any Item where Item can Compare
function smallest(self: List of Item) returns maybe Item for any Item where Item can Compare
function with_index(self: List of Item) returns List of Pair of Item, Integer for any Item
function to_set(self: List of Item) returns Set of Item for any Item where Item can Hash
function flattened(self: List of List of Item) returns List of Item for any Item
function join(self: List of Text, separator: Text) returns Text
function sum(self: List of Integer) returns Integer
function sum(self: List of Decimal) returns Decimal
function sum(self: List of Float) returns Float

function length(self: Map of Key to Value) returns Integer for any Key, Value
function is_empty(self: Map of Key to Value) returns Boolean for any Key, Value
function get(self: Map of Key to Value, key: Key) returns maybe Value for any Key, Value
function set(self: Map of Key to Value, key: Key, value: Value) returns Map of Key to Value
  for any Key, Value
  purpose: A copy with the key bound to the value, added or replaced.
function without(self: Map of Key to Value, key: Key) returns Map of Key to Value for any Key, Value
function contains_key(self: Map of Key to Value, key: Key) returns Boolean for any Key, Value
function keys(self: Map of Key to Value) returns List of Key for any Key, Value
function values(self: Map of Key to Value) returns List of Value for any Key, Value
function entries(self: Map of Key to Value) returns List of Pair of Key, Value for any Key, Value
function merged(self: Map of Key to Value, other: Map of Key to Value) returns Map of Key to Value
  for any Key, Value
  purpose: Both maps; on a shared key the other map wins.

function length(self: Set of Item) returns Integer for any Item
function is_empty(self: Set of Item) returns Boolean for any Item
function contains(self: Set of Item, item: Item) returns Boolean for any Item
function add(self: Set of Item, item: Item) returns Set of Item for any Item
function without(self: Set of Item, item: Item) returns Set of Item for any Item
function union(self: Set of Item, other: Set of Item) returns Set of Item for any Item
function intersection(self: Set of Item, other: Set of Item) returns Set of Item for any Item
function difference(self: Set of Item, other: Set of Item) returns Set of Item for any Item
  purpose: The items of self that are not in other.
function is_subset_of(self: Set of Item, other: Set of Item) returns Boolean for any Item
function sorted(self: Set of Item) returns List of Item for any Item where Item can Compare
function to_list(self: Set of Item) returns List of Item for any Item

function contains(self: Range, value: Integer) returns Boolean
function to_list(self: Range) returns List of Integer
function length(self: Range) returns Integer
```

The list, map and set methods that take a `self` with type parameters are
written here with an explicit `for any` clause; in the prelude source the
clause sits on each declaration exactly as shown.

### 1.4 Core abilities

Inside an `ability` declaration, `Self` names the implementing type
(decision K7). All six are derivable with `can`; `Equal` is derived for every
data type without being named.

| Ability | Method | Used by |
|---------|--------|---------|
| `Equal` | `equals(self, other: Self) returns Boolean` | `is`, `is not`, `contains`, `index_of` |
| `Compare` | `compare(self, other: Self) returns Ordering` | the four ordering phrases, `sorted`, `sorted by`, `largest`, `smallest` |
| `Hash` | `hash(self) returns Integer` | `Set` items, `Map` keys, `distinct`, `to_set` |
| `ToText` | `to_text(self) returns Text` | interpolation, `console.print` of non-text values |
| `ToJson`, `FromJson` | `to_json(self) returns JsonValue`, `from_json(value: JsonValue) returns Self or fails with JsonError` | `std.json`, `std.http`, `std.server` |
| `FromRow` | `from_row(row: Row) returns Self or fails with DbError` | `std.sqlite` |

`Ordering is one of Less, Same, Greater`. `can Compare by field, field`
derives lexicographic comparison over the listed fields.

### 1.5 Built-in types and errors

```
public type Pair of Left, Right
  has left: Left
  has right: Right
end

public type Duration
  purpose: A length of time with millisecond precision; constructed by std.time.
  can Compare
end
function to_milliseconds(self: Duration) returns Integer
function to_seconds(self: Duration) returns Decimal

public type ConstraintViolation
  purpose: A refined type was constructed from a value that fails its condition.
  has type_name: Text
  has detail: Text
end

public type InvalidNumber
  purpose: Text that is not a number of the requested kind.
  has input: Text
end

public type InvalidEncoding
  purpose: Bytes that are not valid UTF-8.
  has detail: Text
end

public type TimedOut
  purpose: A `within` deadline expired before every task finished.
  has after: Duration
end
```

Error types are records; a record is matched like a single variant
(`when InvalidNumber(input) then`, decision K6).

---

## 2. std.console

```
public function print(text: Text) needs console
  purpose: Write the text and a line break to standard output.
public function print_error(text: Text) needs console
  purpose: Write the text and a line break to standard error.
public function read_line() returns maybe Text needs console
  purpose: The next line of standard input without its break, or nothing at end of input.
```

Values that are not `Text` are printed through interpolation:
`console.print("{total}")` (decision K11).

## 3. std.environment

```
public function arguments() returns List of Text needs environment
  purpose: The command-line arguments after the program name.
public function get(name: Text) returns maybe Text needs environment
  purpose: The value of an environment variable.
public function current_directory() returns Path needs environment
public function exit(code: Integer) needs environment
  purpose: Stop the program with the exit code; 0 means success.
```

## 4. std.time

```
public type Instant
  purpose: A point in time, UTC, millisecond precision.
  can Compare
end
public type Date
  purpose: A calendar date in the proleptic Gregorian calendar.
  has year: Integer
  has month: Integer where month is at least 1 and month is at most 12
  has day: Integer where day is at least 1 and day is at most 31
  can Compare by year, month, day
end
public type Weekday is one of
  Monday
  Tuesday
  Wednesday
  Thursday
  Friday
  Saturday
  Sunday
end
public type InvalidDate
  purpose: Text that is not a date of the form YYYY-MM-DD, or names a day that does not exist.
  has input: Text
end

public function now() returns Instant needs time
public function today() returns Date needs time
  purpose: The current date in UTC.
public function sleep(duration: Duration) needs time
public function milliseconds(count: Integer) returns Duration
public function seconds(count: Integer) returns Duration
public function minutes(count: Integer) returns Duration
public function hours(count: Integer) returns Duration
public function parse_date(text: Text) returns Date or fails with InvalidDate
public function parse_instant(text: Text) returns Instant or fails with InvalidDate
  purpose: Read an ISO 8601 timestamp such as 2024-05-01T13:45:00Z.

public function plus_days(self: Date, count: Integer) returns Date
public function minus_days(self: Date, count: Integer) returns Date
public function days_until(self: Date, other: Date) returns Integer
  purpose: Days from self to other; negative when other is earlier.
public function weekday(self: Date) returns Weekday
public function to_text(self: Date) returns Text
  purpose: YYYY-MM-DD.

public function plus(self: Instant, duration: Duration) returns Instant
public function minus(self: Instant, duration: Duration) returns Instant
public function elapsed_since(self: Instant, other: Instant) returns Duration
public function date(self: Instant) returns Date
public function to_text(self: Instant) returns Text
  purpose: ISO 8601 in UTC, such as 2024-05-01T13:45:00Z.
```

A `Date` built from literals (`Date(year: 2024, month: 2, day: 30)`) is checked
at compile time, including the length of the month; a `Date` built from
runtime values needs `otherwise`.

## 5. std.random

```
public function integer(lowest: Integer, highest: Integer) returns Integer needs random
  purpose: A uniformly distributed integer, both bounds included.
public function decimal() returns Decimal needs random
  purpose: A uniformly distributed value from 0 up to but excluding 1.
public function choice(items: List of Item) returns maybe Item for any Item needs random
public function shuffled(items: List of Item) returns List of Item for any Item needs random
```

---

## 6. std.filesystem

```
public type Path is Text where value is not ""
  purpose: A file-system path, absolute or relative to the working directory.
public type Entry is one of
  purpose: What a path points at.
  File(size: Integer)
  Directory
  Other
end
public type FileError is one of
  NotFound(path: Path)
  PermissionDenied(path: Path)
  AlreadyExists(path: Path)
  InvalidEncoding(path: Path)
  Io(path: Path, detail: Text)
end

public function name(self: Path) returns Text
  purpose: The last segment.
public function parent(self: Path) returns maybe Path
public function join(self: Path, segment: Text) returns Path
public function extension(self: Path) returns maybe Text
  purpose: The part after the last dot of the name, without the dot.

public function read_text(path: Path) returns Text or fails with FileError needs filesystem.read
public function read_bytes(path: Path) returns Bytes or fails with FileError needs filesystem.read
public function write_text(path: Path, content: Text) or fails with FileError needs filesystem.write
  purpose: Create or replace the file.
public function write_bytes(path: Path, content: Bytes) or fails with FileError needs filesystem.write
public function append_text(path: Path, content: Text) or fails with FileError needs filesystem.write
public function exists(path: Path) returns Boolean needs filesystem.read
public function inspect(path: Path) returns Entry or fails with FileError needs filesystem.read
public function list(path: Path) returns List of Path or fails with FileError needs filesystem.read
  purpose: The direct children of a directory, sorted by name.
public function create_directory(path: Path) or fails with FileError needs filesystem.write
  purpose: Create the directory and any missing parents.
public function remove(path: Path) or fails with FileError needs filesystem.write
  purpose: Delete a file or an empty directory.
public function copy(source: Path, target: Path) or fails with FileError needs filesystem
public function move(source: Path, target: Path) or fails with FileError needs filesystem
```

`PermissionDenied` is also how a scoped capability mismatch surfaces at run
time (decision J11).

## 7. std.json

```
public type JsonValue is one of
  purpose: Any JSON document, for data whose shape is not known in advance.
  JsonObject(fields: Map of Text to JsonValue)
  JsonArray(items: List of JsonValue)
  JsonText(value: Text)
  JsonNumber(value: Decimal)
  JsonBoolean(value: Boolean)
  JsonNull
  can ToJson
  can FromJson
end
public type Naming is one of
  purpose: How field names map to keys.
  ExactNames
  CamelCase
  KebabCase
end
public type JsonError is one of
  Malformed(detail: Text, line: Integer)
  Mismatch(path: Text, expected: Text, found: Text)
  Constraint(path: Text, detail: Text)
end

public function parse(text: Text) returns Value or fails with JsonError
  for any Value where Value can FromJson
  purpose: Decode the text into the type the context expects.
public function parse_with(text: Text, naming: Naming) returns Value or fails with JsonError
  for any Value where Value can FromJson
public function render(value: Value) returns Text for any Value where Value can ToJson
  purpose: Compact JSON on one line.
public function render_indented(value: Value) returns Text for any Value where Value can ToJson
public function render_with(value: Value, naming: Naming) returns Text
  for any Value where Value can ToJson
```

Derivation rules for `ToJson` and `FromJson`: a record is an object keyed by
its field names, or by the `as` names (decision J14); a sum type is a flat
object with a `kind` key holding the variant name next to the variant's
fields, `{"kind": "Circle", "radius": 2.5}` and `{"kind": "Point"}`, and a
variant with a field named `kind` cannot derive the abilities (decision K10);
`maybe` is the value or `null`, and a missing key reads as `nothing`; `List`
is an array; `Map of Text to V` is an object; `Integer`, `Decimal` and `Float`
are numbers; `Date` and `Instant` are their `to_text()` strings; `Bytes` is
base64. Refinements are checked while decoding and reported as `Constraint`.

## 8. std.http

```
public type Url is Text where value is an absolute URL with a scheme and a host
  purpose: Checked at compile time for literals.
public type Response
  has status: Integer
  has headers: Map of Text to Text
  has body: Text
  has bytes: Bytes
end
public type HttpError is one of
  Unreachable(url: Url, detail: Text)
  Timeout(url: Url)
  HostNotAllowed(host: Text)
  Status(url: Url, status: Integer, body: Text)
end

public function get(url: Url) returns Response or fails with HttpError needs network.http
public function get_with(url: Url, headers: Map of Text to Text) returns Response
  or fails with HttpError
  needs network.http
public function post(url: Url, body: Text, headers: Map of Text to Text) returns Response
  or fails with HttpError
  needs network.http
public function post_json(url: Url, value: Value) returns Response
  or fails with HttpError
  needs network.http
  for any Value where Value can ToJson
  purpose: POST the value as JSON with the matching content type.
public function put(url: Url, body: Text, headers: Map of Text to Text) returns Response
  or fails with HttpError
  needs network.http
public function delete(url: Url) returns Response or fails with HttpError needs network.http
public function request(method: Text, url: Url, body: Text, headers: Map of Text to Text)
  returns Response
  or fails with HttpError
  needs network.http
```

A response with a status outside 200 to 299 is a failure, `Status`, carrying
the status and the body (decision K2); a success path therefore always holds
a 2xx response. `body` is the bytes decoded as UTF-8 with replacement
characters for invalid sequences; `bytes` is the raw body. Each request has a
30-second limit and fails with `Timeout`; a block deadline uses `within`.
`HostNotAllowed` reports a scoped-capability mismatch (decision J11).

## 9. std.server

```
public type Port is Integer where value is at least 1 and value is at most 65535
public type Request
  has method: Text
  has path: Text
  has query: Map of Text to Text
  has headers: Map of Text to Text
  has body: Text
end
public type Response
  has status: Integer
  has headers: Map of Text to Text
  has body: Text
end
public type StartError is one of
  PortInUse(port: Port)
  PermissionDenied(port: Port)
end

public function serve(port: Port, handler: function(Request) returns Response)
  or fails with StartError
  needs network.socket
  purpose: Answer every request with the handler until the process stops.
public function ok(body: Text) returns Response
  purpose: Status 200 with a text/plain body.
public function ok_json(value: Value) returns Response for any Value where Value can ToJson
  purpose: Status 200 with the value rendered as application/json.
public function not_found() returns Response
public function bad_request(detail: Text) returns Response
public function respond(status: Integer, body: Text) returns Response
public function with_header(self: Response, name: Text, value: Text) returns Response
```

The handler's own effects flow to `serve` through the function type (syntax
sketch, section 3), so a handler that reads files makes `main` need
`filesystem.read`.

## 10. std.csv

```
public type Row
  purpose: One data row of a file with a header line.
  has line: Integer
end
public type CsvError is one of
  Malformed(line: Integer, detail: Text)
  MissingHeader
end

public function get(self: Row, column: Text) returns maybe Text
  purpose: The cell under the named header, or nothing when the column is missing or empty.
public function cells(self: Row) returns List of Text
public function parse(text: Text) returns List of Row or fails with CsvError
  purpose: Read a file whose first line names the columns.
public function parse_without_header(text: Text) returns List of List of Text or fails with CsvError
public function render(header: List of Text, rows: List of List of Text) returns Text
  purpose: Quote cells that contain commas, quotes or line breaks.
```

## 11. std.sqlite

```
public type Connection
  purpose: An open database file.
end
public type Parameter is one of
  purpose: A value bound to a `?` placeholder.
  IntegerValue(value: Integer)
  DecimalValue(value: Decimal)
  TextValue(value: Text)
  BooleanValue(value: Boolean)
  NullValue
end
public type DbError is one of
  CannotOpen(path: Path, detail: Text)
  Failed(sql: Text, detail: Text)
  Mismatch(column: Text, expected: Text, found: Text)
end

public function open(path: Path) returns Connection or fails with DbError needs filesystem
  purpose: Open the file, creating it when it does not exist.
public function integer(value: Integer) returns Parameter
public function decimal(value: Decimal) returns Parameter
public function text(value: Text) returns Parameter
public function boolean(value: Boolean) returns Parameter
public function absent() returns Parameter
public function query(self: Connection, sql: Text, parameters: List of Parameter)
  returns List of Row
  or fails with DbError
  needs filesystem.read
  for any Row where Row can FromRow
  purpose: Run a statement that returns rows and decode each into the type the context expects.
public function execute(self: Connection, sql: Text, parameters: List of Parameter)
  returns Integer
  or fails with DbError
  needs filesystem.write
  purpose: Run a statement that returns no rows; the result is the number of changed rows.
public function close(self: Connection) needs filesystem
```

`FromRow` is derivable: columns are matched to fields by name or `as` name;
`INTEGER` decodes to `Integer`, `REAL` to `Float`, `TEXT` to `Text`, `NUMERIC`
to `Decimal`, `NULL` to `nothing` for a `maybe` field. Anything else is
`Mismatch`.

## 12. std.regex

Regular expressions use the syntax of the Rust `regex` crate: no
backreferences, no look-around, linear-time matching. `Text.matches` lives in
the prelude (decision K3); everything else is here.

```
public type Pattern is Text where value is a valid regular expression
  purpose: Checked at compile time for literals.

public function find_all(self: Pattern, text: Text) returns List of Text
  purpose: Every non-overlapping match, in order.
public function captures(self: Pattern, text: Text) returns maybe List of Text
  purpose: The capture groups of the first match, or nothing when there is none.
public function replace_all(self: Pattern, text: Text, replacement: Text) returns Text
public function split(self: Pattern, text: Text) returns List of Text
```

---

## 13. Not in v1

Candidates for later extension packages, in no order: `std.process` (start
programs, capability `process`), `std.socket` (raw sockets), `std.crypto`
(hashes, HMAC), `std.yaml` and `std.toml`, `std.markdown`, an HTTP client
with persistent settings (default headers, per-client timeouts, retries), and
streaming variants of the file and HTTP functions for data that does not fit
in memory.

## 14. Open questions

R3-1 to R3-3 were decided (decisions K9 to K11). New questions are listed here
as they arise.
