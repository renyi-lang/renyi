# Renyi Cheat Sheet

Renyi is a statically typed language whose syntax is regular English. Every
concept has one spelling. Blocks end with `end`. Indentation is never meaning.
There is no `=`, no `==`, no `null`, no exceptions, no anonymous functions.

## Module
```
module billing.invoices
  purpose: Compute customer invoices.

import std.json                              # use as json.parse(...)
import std.http as web                       # renamed namespace
import accounts.models exposing User, UserId # types and abilities only
```
Imported functions are called qualified (`json.parse(text)`), own-module
ones bare. Definitions are private unless `public`.

## Function
```
public function total_price(items: List of Item, rate: TaxRate)
  returns Money
  or fails with PricingError
  needs network.http
  for any Item where Item can Priced
  purpose: Add up every item and apply the tax rate.
  tags: billing, money
  see also: tax_for
  example: total_price(items: [Item(price: 10.00)], rate: TaxRate(0.1)) is 11.00

  let subtotal be for each item in items sum item.price
  return subtotal + tax.tax_for(amount: subtotal, rate: rate)
end
```
A function whose first parameter is `self: Text` is a method: `text.trim()`.
Clause order: `returns`, `or fails with`, `needs`, `for any`, `purpose:`,
`tags:`, `see also:`, `deprecated:`, `expose as tool`, `example:` lines, blank
line, body. Omit `returns` when nothing is returned. The `module` line and
every `public` function, type and constant must carry a `purpose:` line; the
checker rejects one without it.

## Calls
```
let page be web.get(url) otherwise fail
let total be money.add(left: 1, right: 2)          # two or more: all named, in order
retry.run(action: fetch_page, attempts: 3)         # pass a function by name
```
One argument is never named: `web.get(url)`, not `web.get(url: url)`.

## Bindings
```
let total be 0                       # immutable
let users: List of User be json.parse(text) otherwise fail   # pin a type
let mutable count be 0               # mutable local
set count to count + 1               # the only way to change it
ignore connection.execute(sql)       # discard the result of a call with effects
```
A name is bound once per scope; shadowing is an error. Every binding must be
used, and an unused result is an error (`set items to items.append(item)`),
as is `ignore` of a pure call's result.

## Types
```
public type User
  purpose: A registered account holder.
  has name: Text
  has age: Integer where age is at least 0      # refinement
  has email: maybe Email                        # optional field
  has kind: Text as "type"                      # external name for JSON
  can Compare by name                           # derived ability
  can ToJson
end

public type Shape is one of
  purpose: What the plotter can draw.
  Circle(radius: Decimal)
  Rectangle(width: Decimal, height: Decimal)
  Point
end

public type Email is Text where value.matches(email_pattern)
  purpose: A syntactically valid address.
public type UserId is Integer                   # distinct alias
  purpose: The key of a user row.
public type Pair of Left, Right
  purpose: Two values carried together.
  has left: Left
  has right: Right
end
```
Construct: `User(name: "Ann", age: 30, email: nothing)`, `Circle(radius: 2.5)`
(fields always named), `Point`, `UserId(7)` (a subtype wraps one value).
Update: `user with age: 31`. Refined construction or update can fail:
`Email(input) otherwise fail with BadInput`. A literal the checker can
evaluate needs no `otherwise` (`Port(8080)`); a variable needs one even
after a check. Generic types: `List of T`, `Map of K to V`, `Set of T`,
`maybe T`.

## Abilities
```
ability Describable
  function describe(self) returns Text
end

ability Describable for Shape
  function describe(self) returns Text
    match self
      when Circle(radius) then return "circle of radius {radius}"
      when Point then return "point"
    end
  end
end
```
Derivable: `Equal`, `Compare by`, `Hash`, `ToText` (a variant prints as its
bare name, `Green`; a record in constructor form), `ToJson`, `FromJson`.
`Iterable of Item` is implemented (`to_list(self) returns List of Item`) and
`for each` walks the type. Generic: `ability Sized for Stack of Item` then
`for any Item`.

## Expressions
```
a + b   a - b   a * b   a / b   a remainder b   a power b
a is b   a is not b   a is less than b   a is at most b
a is greater than b   a is at least b
a and b   a or b   not a
"Hello {user.name}, total {total}"         # interpolation; a literal brace is \{
raw "^[0-9]{4}$"                           # no holes, no escapes
from 1 to 10   from 0 to 100 by 5         # inclusive ranges
[1, 2, 3]   {"key": value}   nothing   true   false
```
Numbers: `Integer` (unbounded), `Decimal` (decimal128, literals like `19.99`),
`Float`. No implicit conversion: `count.to_decimal()`. `is` compares
values of one type: `32.0 is 32.00`. `/` needs `Decimal` or `Float` operands;
`a.quotient(b)` divides two Integers down.
`a.at_least(b)` is the larger of two values, `a.at_most(b)` the smaller.

## Optionals and errors
```
let name be user.nickname otherwise user.name       # default
let text be files.read_text(path) otherwise fail    # propagate
let config: Config be json.parse(text) otherwise fail with BadConfig(detail: "not JSON")
let user be accounts.find(id) otherwise return nothing
fail with NotFound(path: path)                      # return a failure
crash with "unreachable: {state}"                   # terminate; bugs only

match action()                                      # when the error is needed
  when success(outcome) then return outcome
  when failure(error) then log.warn(error.to_text())
end
```
A `T` is accepted where `maybe T` or a success is expected. Reading out always
names the other case, and `otherwise` is allowed only there. It takes a default
value or a way out: `fail`, `fail with E(...)`, `return value`, `break`,
`continue`, `crash with "text"`.
`type X is Base` is a subtype: an `X` passes as a `Base`, never the reverse.

## Control flow
```
if age is at least 18 then
  set adults to adults + 1
otherwise if age is at least 13 then
  set teens to teens + 1
otherwise
  set children to children + 1
end

match shape
  when Circle(radius) where radius is greater than 10 then return Large
  when Circle(radius) then return Small
  when Rectangle(width: wide, height: tall) then return area.classify(width: wide, height: tall)
  otherwise return Unknown
end

match user.nickname
  when some(nickname) then console.print(nickname)
  when nothing then console.print(user.name)
end

for each line in invoice.lines
  set total to total + line.amount
end
for each key, value in settings
  console.print("{key}: {value}")
end
repeat until attempts is at least 3
  set attempts to attempts + 1
  if done then break end
end
```
Matching is exhaustive. A variant with fields matches by its bare name when
no field is needed: `when Circle then`. `if` and `match` are also expressions
when every branch is one expression: `let label be if done then "yes"
otherwise "no" end`.
Loop headers accept `where` and `sorted by`.

## Queries (instead of lambdas)
```
let emails be
  for each user in users
  where user.is_active and user.age is at least 18
  sorted by user.created_at descending
  collect user.email

for each order in orders where order.is_paid sum order.total     # Decimal
for each order in orders where order.is_paid count               # Integer
for each order in orders sorted by order.created_at first        # maybe Order
for each order in orders all order.is_shipped                    # Boolean
for each order in orders any order.is_refunded                   # Boolean
for each user in users group by user.country collect user.email # Map
for each sale in sales group by sale.region sum sale.amount     # Map of Text to Decimal
for each order in orders, line in order.lines collect line.sku
for each url in urls concurrently collect web.get(url) otherwise fail
```

## Effects
`needs` lists capabilities; callers must declare a superset; no `needs` means
pure. Capabilities: `console`, `filesystem.read`, `filesystem.write`,
`network.http`, `network.socket`, `environment`, `time`, `random`. A parent
covers its children. A literal argument narrows a scope:
`filesystem.read("data")`, `network.http("api.example.com")`; no argument
covers every scope. `main` declares the program's whole grant. Only there: `at most 60 per minute` (budget; also `per run`) and
`only to console` (data read through it may leave only there).
```
public function main() or fails with AppError
  needs console, network.http("api.example.com") at most 60 per minute
  purpose: Print today's report.
  ...
end
```

## Concurrency
```
run concurrently within time.seconds(5)
  let users be accounts.fetch_all() otherwise fail
  let orders be billing.fetch_open() otherwise fail
end
```
All tasks finish or the first failure cancels the rest; an expired `within`
deadline cancels them and fails with `TimedOut`.

## Tests and tools
```
test "tax is applied to the subtotal"
  check invoices.total_price(items: items, rate: TaxRate(0.1)) is 11.00
end
test "the forecast is read" needs network.http replays "fixtures/forecast.json"
  check weather.fetch(city: "Berlin").temperature is 21.5
end
```
A recording (`renyi record`) answers every effect of a `replays` test offline.
`expose as tool` publishes a function to agents (schema from the parameters,
description from `purpose:`, permissions from `needs`; `renyi tools` prints it).
`deprecated: since 2.0, replaced by new_name` warns existing callers.

## Library (names and parameters; nothing else exists)
```
Text: length() is_empty() trim() trim_start() trim_end() to_lower() to_upper()
  split(separator) lines() characters() contains(part) starts_with(prefix)
  ends_with(suffix) index_of(part) replace(old, new) pad_left(width)
  pad_right(width) repeat(times) take(length) drop(length) reversed()
  matches(pattern) to_integer() to_decimal() to_float() to_bytes()
List: length() is_empty() at(index) first() last() rest() without_last()
  without_index(index) take(length) drop(length) slice(start, stop) append(item)
  append_all(others) reversed() sorted() distinct() contains(item)
  index_of(item) largest() smallest() with_index() to_set() flattened()
  join(separator) sum()
Map: length() is_empty() get(key) set(key, value) without(key)
  contains_key(key) keys() values() entries() merged(other)
Set: length() is_empty() contains(item) add(item) without(item) union(other)
  intersection(other) difference(other) is_subset_of(other) sorted() to_list()
Numbers: to_decimal() to_float() to_text() quotient(divisor) absolute()
  at_least(other) at_most(other) rounded(places) truncated() square_root()
Modules: std.console (print, print_error, read_line); std.environment
  (arguments, get, exit); std.time (now, today, seconds, parse_date, Date,
  Instant, Duration); std.random; std.filesystem (Path, read_text, write_text,
  exists, list); std.json (parse, render); std.http (Url, get, post_json);
  std.server; std.csv; std.sqlite; std.regex
```
One parameter is positional, more are named: `line.split(",")`,
`price.rounded(2)`, `text.replace(old: "a", new: "b")`.

## Names and reserved words
snake_case for values and functions, PascalCase for types, abilities and
variants. ASCII only. No single-letter names. Reserved:
```
ability all also and any as at be break by can check collect concurrently
continue count crash deprecated descending each end example expose exposing
fail fails failure false first for from function greater group has if ignore
import in is lazy least less let match maybe module most mutable needs not
nothing of one only or otherwise per power public purpose raw remainder repeat
replays return returns run see self set some sorted success sum tags test than
then to tool true type until when where with within
```
Any word is valid after a dot.
