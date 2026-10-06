//! Each rule of the checker on a small program: the diagnostic it must
//! raise (by code), or that it must stay silent.

use renyi_syntax::SourceFile;

fn check(source: &str) -> Vec<renyi_syntax::Diagnostic> {
    let file = SourceFile::new("t.ry", source);
    renyi_check::check_sources(&file, &[])
}

/// The codes of the errors a program raises.
fn codes(source: &str) -> Vec<String> {
    check(source)
        .into_iter()
        .filter(|d| d.is_error())
        .map(|d| d.code.to_string())
        .collect()
}

fn clean(source: &str) {
    let diagnostics = check(source);
    assert!(
        diagnostics.is_empty(),
        "{}",
        renyi_syntax::diagnostics::render_text(&SourceFile::new("t.ry", source), &diagnostics)
    );
}

fn raises(source: &str, code: &str) {
    let found = codes(source);
    assert!(
        found.iter().any(|c| c == code),
        "expected `{code}`, found {found:?}:\n{}",
        renyi_syntax::diagnostics::render_text(&SourceFile::new("t.ry", source), &check(source))
    );
}

const HEAD: &str = "module demo\n  purpose: Exercise the checker.\n\nimport std.console\n\n";

fn program(body: &str) -> String {
    format!("{HEAD}{body}")
}

#[test]
fn a_pure_function_with_an_example_is_clean() {
    clean(&program(
        "public function double(value: Integer) returns Integer\n  purpose: Twice the value.\n  example: double(2) is 4\n\n  return value * 2\nend\n",
    ));
}

#[test]
fn ignore_discards_only_the_result_of_a_call_with_effects() {
    // decision R6: a discarded pure result is dead code
    clean(&program(
        "function go() needs console\n  ignore console.read_line()\nend\n",
    ));
    raises(
        &program("function go() returns Integer\n  ignore \"x\".trim()\n  return 1\nend\n"),
        "ignore-pure",
    );
    raises(
        &program("function go() needs console\n  ignore console.print(\"x\")\nend\n"),
        "ignore-nothing",
    );
}

#[test]
fn unknown_names_and_types() {
    raises(
        &program("function go() returns Integer\n  return missing\nend\n"),
        "unknown-name",
    );
    raises(
        &program("function go(value: Nowhere) returns Integer\n  return 1\nend\n"),
        "unknown-type",
    );
    raises(
        &program("function go() returns Integer\n  return nope(1)\nend\n"),
        "unknown-function",
    );
    raises(
        &program("function go() returns Text\n  return \"x\".lowercase()\nend\n"),
        "unknown-method",
    );
}

#[test]
fn type_mismatches() {
    raises(
        &program("function go() returns Integer\n  return \"text\"\nend\n"),
        "type-mismatch",
    );
    raises(
        &program("function go() returns Integer\n  let total be 1 + \"x\"\n  return total\nend\n"),
        "type-mismatch",
    );
    raises(
        &program(
            "function go() returns Boolean\n  if 1 then return true end\n  return false\nend\n",
        ),
        "type-mismatch",
    );
}

#[test]
fn maybe_values_must_be_read_out() {
    raises(
        &program(
            "function go(items: List of Integer) returns Integer\n  return items.first()\nend\n",
        ),
        "maybe-value",
    );
    clean(&program(
        "function go(items: List of Integer) returns Integer\n  return items.first() otherwise 0\nend\n",
    ));
    clean(&program(
        "function go(items: List of Integer) returns Integer\n  match items.first()\n    when some(item) then return item\n    when nothing then return 0\n  end\nend\n",
    ));
    raises(
        &program("function go(items: List of Integer) returns Integer\n  match items.first()\n    when some(item) then return item\n  end\nend\n"),
        "not-exhaustive",
    );
}

#[test]
fn fallible_calls_need_otherwise() {
    raises(
        &program("function go(text: Text) returns Integer\n  return text.to_integer()\nend\n"),
        "missing-otherwise",
    );
    clean(&program(
        "function go(text: Text) returns Integer\n  return text.to_integer() otherwise 0\nend\n",
    ));
    // propagating needs the error in the signature
    raises(
        &program("function go(text: Text) returns Integer\n  return text.to_integer() otherwise fail\nend\n"),
        "error-not-declared",
    );
    clean(&program(
        "function go(text: Text) returns Integer or fails with InvalidNumber\n  return text.to_integer() otherwise fail\nend\n",
    ));
    // a fallible call in statement position
    raises(
        &program("function go(text: Text)\n  ignore text.to_integer()\nend\n"),
        "missing-otherwise",
    );
}

#[test]
fn superfluous_otherwise_is_an_error() {
    raises(
        &program("function go(text: Text) returns List of Text\n  return text.split(\" \") otherwise []\nend\n"),
        "superfluous-otherwise",
    );
}

#[test]
fn argument_naming_rules() {
    raises(
        &program("function twice(value: Integer) returns Integer\n  return value * 2\nend\n\nfunction go() returns Integer\n  return twice(value: 2)\nend\n"),
        "argument-name",
    );
    raises(
        &program("function add(left: Integer, right: Integer) returns Integer\n  return left + right\nend\n\nfunction go() returns Integer\n  return add(1, 2)\nend\n"),
        "argument-name",
    );
    raises(
        &program("function add(left: Integer, right: Integer) returns Integer\n  return left + right\nend\n\nfunction go() returns Integer\n  return add(right: 1, left: 2)\nend\n"),
        "argument-order",
    );
    raises(
        &program("function add(left: Integer, right: Integer) returns Integer\n  return left + right\nend\n\nfunction go() returns Integer\n  return add(left: 1)\nend\n"),
        "argument-count",
    );
    clean(&program(
        "function add(left: Integer, right: Integer) returns Integer\n  return left + right\nend\n\nfunction go() returns Integer\n  return add(left: 1, right: 2)\nend\n",
    ));
}

#[test]
fn unused_bindings_are_errors() {
    raises(
        &program("function go() returns Integer\n  let unused be 1\n  return 2\nend\n"),
        "unused-binding",
    );
    raises(
        &program("function go(value: Integer) returns Integer\n  return 2\nend\n"),
        "unused-binding",
    );
    raises(
        &program("function go(items: List of Integer) returns Integer\n  return for each item in items count\nend\n"),
        "unused-binding",
    );
    // `first` returns the variable, so it counts as read
    clean(&program(
        "function go(items: List of Integer) returns maybe Integer\n  return for each item in items first\nend\n",
    ));
    clean(&program(
        "function go(items: List of Integer) returns Integer\n  return for each item in items where item is greater than 0 count\nend\n",
    ));
}

#[test]
fn unused_results_are_errors() {
    raises(
        &program("function go(items: List of Integer)\n  let mutable kept be items\n  kept.append(1)\n  ignore kept\nend\n"),
        "unused-result",
    );
    clean(&program(
        "function go(items: List of Integer) returns List of Integer\n  let mutable kept be items\n  set kept to kept.append(1)\n  return kept\nend\n",
    ));
}

#[test]
fn integer_division_is_an_error() {
    raises(
        &program("function go(total: Integer) returns Integer\n  return total / 2\nend\n"),
        "integer-division",
    );
    clean(&program(
        "function go(total: Integer) returns Integer\n  return total.quotient(2)\nend\n",
    ));
    clean(&program(
        "function go(total: Decimal) returns Decimal\n  return total / 2\nend\n",
    ));
}

#[test]
fn effects_must_be_declared() {
    raises(
        &program("function go()\n  console.print(\"hi\")\nend\n"),
        "capability-missing",
    );
    clean(&program(
        "function go() needs console\n  console.print(\"hi\")\nend\n",
    ));
    // a scoped grant covers a sub-path, not a sibling
    let scoped = "module demo\n  purpose: Scoped capabilities.\n\nimport std.filesystem exposing Path, FileError\n\nfunction load(path: Path) returns Text or fails with FileError needs filesystem.read(\"data\")\n  return filesystem.read_text(path) otherwise fail\nend\n\nfunction go() returns Text or fails with FileError needs filesystem(\"data\")\n  return load(Path(\"data/x\")) otherwise fail\nend\n";
    clean(scoped);
    let sibling = scoped.replace("needs filesystem(\"data\")", "needs filesystem(\"other\")");
    raises(&sibling, "capability-missing");
    // a function passed by name brings its effects to the call
    let passed = "module demo\n  purpose: Effects of a function value.\n\nimport std.console\n\nfunction run_twice(action: function())\n  action()\n  action()\nend\n\nfunction say() needs console\n  console.print(\"hi\")\nend\n\nfunction go()\n  run_twice(say)\nend\n";
    raises(passed, "capability-missing");
    clean(&passed.replace("function go()\n", "function go() needs console\n"));
}

#[test]
fn examples_only_on_pure_functions() {
    raises(
        &program("public function go() needs console\n  purpose: Print.\n  example: go() is nothing\n\n  console.print(\"hi\")\nend\n"),
        "example-effects",
    );
}

#[test]
fn shadowing_and_mutation() {
    raises(
        &program(
            "function go(value: Integer) returns Integer\n  let value be 2\n  return value\nend\n",
        ),
        "shadowing",
    );
    raises(&program("function go() returns Integer\n  let total be 1\n  set total to 2\n  return total\nend\n"), "immutable-binding");
    clean(&program("function go() returns Integer\n  let mutable total be 1\n  set total to 2\n  return total\nend\n"));
}

#[test]
fn missing_return_and_unreachable() {
    raises(
        &program("function go(flag: Boolean) returns Integer\n  if flag then return 1 end\nend\n"),
        "missing-return",
    );
    clean(&program(
        "function go(flag: Boolean) returns Integer\n  if flag then\n    return 1\n  otherwise\n    return 2\n  end\nend\n",
    ));
}

#[test]
fn constructions_and_refinements() {
    let types = "public type Port is Integer where value is at least 1 and value is at most 65535\n  purpose: A port.\n\npublic type User\n  purpose: A user.\n  has name: Text\n  has age: Integer where age is at least 0\nend\n\n";
    clean(&program(&format!(
        "{types}function go() returns Port\n  return Port(8080)\nend\n"
    )));
    raises(
        &program(&format!(
            "{types}function go() returns Port\n  return Port(0)\nend\n"
        )),
        "constraint-violation",
    );
    raises(
        &program(&format!(
            "{types}function go() returns User\n  return User(name: \"Ann\", age: -1)\nend\n"
        )),
        "constraint-violation",
    );
    // a runtime value makes the construction fallible
    raises(
        &program(&format!(
            "{types}function go(number: Integer) returns Port\n  return Port(number)\nend\n"
        )),
        "missing-otherwise",
    );
    clean(&program(&format!("{types}function go(number: Integer) returns Port\n  return Port(number) otherwise Port(80)\nend\n")));
    // fields are always named, even a single one
    raises(
        &program(&format!(
            "{types}function go() returns User\n  return User(\"Ann\", 3)\nend\n"
        )),
        "argument-name",
    );
    raises(
        &program(&format!(
            "{types}function go() returns User\n  return User(name: \"Ann\", years: 3)\nend\n"
        )),
        "unknown-field",
    );
}

#[test]
fn sum_types_match_exhaustively() {
    let shape = "public type Shape is one of\n  purpose: A figure.\n  Circle(radius: Decimal)\n  Square(side: Decimal)\n  Point\nend\n\n";
    clean(&program(&format!("{shape}function area(shape: Shape) returns Decimal\n  match shape\n    when Circle(radius) then return radius * radius * 3\n    when Square(side) then return side * side\n    when Point then return 0\n  end\nend\n")));
    raises(&program(&format!("{shape}function area(shape: Shape) returns Decimal\n  match shape\n    when Circle(radius) then return radius\n    when Square(side) then return side\n  end\nend\n")), "not-exhaustive");
    raises(&program(&format!("{shape}function area(shape: Shape) returns Decimal\n  match shape\n    when Circle(radius) then return radius\n    when Square(side) then return side\n    when Point then return 0\n    when Blob then return 0\n  end\nend\n")), "pattern-mismatch");
    // an unused pattern binding is an error; a bare variant name binds nothing
    raises(&program(&format!("{shape}function kind(shape: Shape) returns Text\n  match shape\n    when Circle(radius) then return \"circle\"\n    otherwise return \"other\"\n  end\nend\n")), "unused-binding");
    clean(&program(&format!("{shape}function kind(shape: Shape) returns Text\n  match shape\n    when Circle then return \"circle\"\n    otherwise return \"other\"\n  end\nend\n")));
}

#[test]
fn generics_and_abilities() {
    clean(&program(
        "public type Stack of Item\n  purpose: A stack.\n  has items: List of Item\nend\n\npublic function empty() returns Stack of Item for any Item\n  purpose: Nothing on it.\n\n  return Stack(items: [])\nend\n\nfunction go() returns Integer\n  let mutable stack: Stack of Text be empty()\n  set stack to Stack(items: stack.items.append(\"a\"))\n  return stack.items.length()\nend\n",
    ));
    // sorting needs Compare
    raises(
        &program("public type Blob\n  purpose: Opaque.\n  has data: Text\nend\n\nfunction go(blobs: List of Blob) returns List of Blob\n  return blobs.sorted()\nend\n"),
        "missing-ability",
    );
    clean(&program(
        "public type Named\n  purpose: Sorted by name.\n  has name: Text\n  can Compare by name\nend\n\nfunction go(items: List of Named) returns List of Named\n  return items.sorted()\nend\n",
    ));
    // interpolation needs ToText
    raises(
        &program("public type Blob\n  purpose: Opaque.\n  has data: Text\nend\n\nfunction go(blob: Blob) returns Text\n  return \"{blob}\"\nend\n"),
        "missing-ability",
    );
}

#[test]
fn queries_type_their_results() {
    clean(&program(
        "public type Sale\n  purpose: One sale.\n  has region: Text\n  has amount: Decimal\nend\n\nfunction totals(sales: List of Sale) returns Map of Text to Decimal\n  return for each sale in sales group by sale.region sum sale.amount\nend\n\nfunction regions(sales: List of Sale) returns List of Text\n  return for each sale in sales where sale.amount is greater than 0 collect sale.region\nend\n\nfunction any_big(sales: List of Sale) returns Boolean\n  return for each sale in sales any sale.amount is greater than 100\nend\n",
    ));
    raises(
        &program("function go(words: List of Text) returns Integer\n  return for each word in words sum word\nend\n"),
        "type-mismatch",
    );
}

#[test]
fn tests_and_checks() {
    clean(&program(
        "function double(value: Integer) returns Integer\n  return value * 2\nend\n\ntest \"doubling\"\n  check double(2) is 4\nend\n",
    ));
    raises(
        &program("function go() returns Integer\n  check 1 is 1\n  return 1\nend\n"),
        "check-outside-test",
    );
    // a test may propagate any error and declare its own needs
    clean(&program(
        "test \"parsing\" needs console\n  let number be \"12\".to_integer() otherwise fail\n  console.print(\"{number}\")\nend\n",
    ));
}

#[test]
fn public_items_need_a_purpose() {
    raises("module demo\n  purpose: Public items.\n\npublic function go() returns Integer\n  return 1\nend\n", "purpose-missing");
    raises(
        "module demo\n\nfunction go() returns Integer\n  return 1\nend\n",
        "purpose-missing",
    );
}

#[test]
fn imports_are_checked() {
    raises("module demo\n  purpose: Imports.\n\nimport std.nowhere\n\nfunction go() returns Integer\n  return 1\nend\n", "unknown-module");
    raises("module demo\n  purpose: Imports.\n\nimport std.time\n\nfunction go() returns Date\n  return time.today()\nend\n", "unknown-type");
    clean("module demo\n  purpose: Imports.\n\nimport std.time exposing Date\n\nfunction go() returns Date needs time\n  return time.today()\nend\n");
}

#[test]
fn grant_clauses_belong_to_main_and_tests() {
    clean(&program(
        "public function main() needs console, network.http(\"api.example.com\") at most 60 per minute\n  purpose: Budgeted.\n\n  console.print(\"hi\")\nend\n",
    ));
    clean(&program(
        "test \"budgeted\" needs network.http at most 3 per run replays \"fixtures/x.json\"\n  check true\nend\n",
    ));
    raises(
        &program("function helper() needs network.http at most 60 per minute\n  ignore 1\nend\n"),
        "grant-clause",
    );
    raises(
        &program("public function main() needs console at most 3 per run\n  purpose: Budgeted console.\n\n  console.print(\"hi\")\nend\n"),
        "grant-clause",
    );
    raises(
        &program("public function main() needs filesystem.read(\"secrets\") only to nowhere\n  purpose: Guarded.\n\n  console.print(\"hi\")\nend\n"),
        "unknown-capability",
    );
}

/// The codes of every diagnostic, warnings included.
fn all_codes(source: &str) -> Vec<String> {
    check(source).iter().map(|d| d.code.to_string()).collect()
}

#[test]
fn nesting_is_limited_to_four_blocks() {
    // decision V5: the body is depth 0; the fifth block is the error
    let four = "function go(items: List of Integer) returns Integer needs console\n  let mutable total be 0\n  for each item in items\n    if item is greater than 0 then\n      match item\n        when 1 then\n          repeat until total is greater than 9\n            set total to total + 1\n          end\n        otherwise set total to total + item\n      end\n    end\n  end\n  return total\nend\n";
    clean(&program(four));
    let five = "function go(items: List of Integer) returns Integer needs console\n  let mutable total be 0\n  for each item in items\n    if item is greater than 0 then\n      match item\n        when 1 then\n          repeat until total is greater than 9\n            if total is 3 then\n              set total to total + 2\n            end\n            set total to total + 1\n          end\n        otherwise set total to total + item\n      end\n    end\n  end\n  return total\nend\n";
    raises(&program(five), "nesting-depth");
    assert_eq!(
        codes(&program(five))
            .iter()
            .filter(|c| *c == "nesting-depth")
            .count(),
        1,
        "the fifth level is reported once"
    );
}

#[test]
fn a_body_spans_at_most_sixty_lines() {
    let mut long = String::from("function go() returns Integer\n  let mutable total be 0\n");
    for _ in 0..60 {
        long.push_str("  set total to total + 1\n");
    }
    long.push_str("  return total\nend\n");
    raises(&program(&long), "body-length");
    let mut fits = String::from("function go() returns Integer\n  let mutable total be 0\n");
    for _ in 0..58 {
        fits.push_str("  set total to total + 1\n");
    }
    fits.push_str("  return total\nend\n");
    clean(&program(&fits));
}

#[test]
fn tasks_of_run_concurrently_are_independent() {
    // decision V7: a task may not change a binding made before the block
    raises(
        &program("function go() returns Integer\n  let mutable total be 0\n  run concurrently\n    set total to total + 1\n    set total to total + 2\n  end\n  return total\nend\n"),
        "task-independence",
    );
    // nor read a binding another task made
    raises(
        &program("function go() returns Integer\n  run concurrently\n    let alpha be 1\n    let beta be alpha + 1\n  end\n  return alpha + beta\nend\n"),
        "task-independence",
    );
    // independent tasks, read after the block, are fine
    clean(&program(
        "function go() returns Integer\n  let base be 10\n  run concurrently\n    let alpha be base + 1\n    let beta be base + 2\n  end\n  return alpha + beta\nend\n",
    ));
    // a task's own loop variable and its own mutable binding are its business
    clean(&program(
        "function go(items: List of Integer) returns Integer needs console\n  run concurrently\n    let total be for each item in items sum item\n    for each item in items\n      let mutable seen be 0\n      set seen to seen + item\n      console.print(\"{seen}\")\n    end\n  end\n  return total\nend\n",
    ));
}

#[test]
fn process_and_foreign_wait_for_the_package_manager() {
    raises(
        &program("public function main() needs console, process(\"git\")\n  purpose: Not yet.\n\n  console.print(\"hi\")\nend\n"),
        "capability-unavailable",
    );
    raises(
        &program("function go() needs foreign\n  ignore 1\nend\n"),
        "capability-unavailable",
    );
    raises(
        &program("test \"spawning\" needs process\n  check true\nend\n"),
        "capability-unavailable",
    );
}

#[test]
fn a_deprecated_definition_warns_its_callers() {
    let old = "function old(value: Integer) returns Integer\n  deprecated: since 0.2, replaced by fresh\n  example: old(1) is 2\n\n  return value + 1\nend\n\nfunction fresh(value: Integer) returns Integer\n  return value + 1\nend\n\n";
    let caller = format!("{old}function go() returns Integer\n  return old(1)\nend\n");
    let diagnostics = check(&program(&caller));
    let warning = diagnostics
        .iter()
        .find(|d| d.code == "deprecated")
        .expect("a deprecation warning");
    assert!(!warning.is_error());
    assert_eq!(
        warning.message,
        "`old` is deprecated: since 0.2, replaced by fresh"
    );
    assert_eq!(warning.fix.as_deref(), Some("call `fresh` instead"));
    // the definition's own body and examples do not warn
    assert_eq!(
        all_codes(&program(&format!(
            "{old}function go() returns Integer\n  return fresh(1)\nend\n"
        ))),
        Vec::<String>::new()
    );
}

#[test]
fn a_tool_takes_and_returns_json() {
    clean(&program(
        "public type Money is Decimal where value is at least 0\n  purpose: An amount.\n\npublic function convert(amount: Money, rate: Decimal) returns Decimal\n  purpose: Apply a rate.\n  expose as tool\n\n  return amount * rate\nend\n",
    ));
    raises(
        &program("public function apply(action: function(Integer) returns Integer, value: Integer) returns Integer\n  purpose: Call it.\n  expose as tool\n\n  return action(value)\nend\n"),
        "tool-type",
    );
    raises(
        &program("function convert(amount: Decimal) returns Decimal\n  expose as tool\n\n  return amount\nend\n"),
        "purpose-missing",
    );
}

#[test]
fn a_guard_without_a_reachable_sink_warns() {
    let codes = all_codes(&program(
        "public function main() needs console, filesystem.read(\"secrets\") only to network.http(\"api.example.com\")\n  purpose: Guarded.\n\n  console.print(\"hi\")\nend\n",
    ));
    assert_eq!(codes, vec!["guard-no-sink".to_string()]);
    clean(&program(
        "public function main() needs console, filesystem.read(\"secrets\") only to console\n  purpose: Guarded.\n\n  console.print(\"hi\")\nend\n",
    ));
}

#[test]
fn refinement_conditions_are_checked_as_bodies() {
    // a condition is a Boolean expression over the fields, resolved like any body
    clean(&program(
        "public type Email is Text where value.matches(\"^[^@]+@[^@]+$\")\n  purpose: An address.\n\npublic type Line\n  purpose: A line.\n  has quantity: Integer where quantity is at least 1\n  has price: Decimal where price is at least 0 and quantity is at most 1000\nend\n",
    ));
    raises(
        &program("public type Email is Text where value.nonsense()\n  purpose: An address.\n"),
        "unknown-method",
    );
    raises(
        &program("public type Line\n  purpose: A line.\n  has quantity: Integer where quantity is at least \"one\"\nend\n"),
        "type-mismatch",
    );
    raises(
        &program(
            "public type Line\n  purpose: A line.\n  has quantity: Integer where quantity\nend\n",
        ),
        "type-mismatch",
    );
}

#[test]
fn an_update_of_a_refined_field_can_fail_like_a_construction() {
    let person =
        "type Person\n  has name: Text\n  has age: Integer where age is at least 0\nend\n\n";
    // a literal is decided here
    clean(&program(&format!(
        "{person}function older(person: Person) returns Person\n  return person with age: 31\nend\n"
    )));
    raises(
        &program(&format!(
            "{person}function older(person: Person) returns Person\n  return person with age: -1\nend\n"
        )),
        "constraint-violation",
    );
    // a variable needs `otherwise`, as decision U9 says of a construction
    raises(
        &program(&format!(
            "{person}function older(person: Person, years: Integer) returns Person\n  return person with age: years\nend\n"
        )),
        "missing-otherwise",
    );
    clean(&program(&format!(
        "{person}function older(person: Person, years: Integer) returns Person\n  return person with age: years otherwise person\nend\n"
    )));
    // a field without a refinement never fails
    clean(&program(&format!(
        "{person}function renamed(person: Person, name: Text) returns Person\n  return person with name: name\nend\n"
    )));
}

fn codes_with(main: &str, util: &str) -> Vec<String> {
    renyi_check::check_sources(
        &SourceFile::new("main.ry", main),
        &[SourceFile::new("util.ry", util)],
    )
    .into_iter()
    .filter(|d| d.is_error())
    .map(|d| d.code.to_string())
    .collect()
}

const UTIL: &str = "module util\n  purpose: Helpers.\n\npublic function shout(text: Text) returns Text\n  purpose: Upper-case.\n\n  return text.to_upper()\nend\n\nfunction whisper(text: Text) returns Text\n  return text.to_lower()\nend\n\npublic let limit: Integer be 3\n  purpose: The limit.\n\nlet secret: Integer be 7\n\npublic type Word\n  purpose: A word.\n  has text: Text\nend\n\npublic function twice(self: Word) returns Text\n  purpose: Twice.\n\n  return \"{self.text}{self.text}\"\nend\n\nfunction once(self: Word) returns Text\n  return self.text\nend\n\nability Secretive\n  function reveal(self) returns Text\nend\n";

const MAIN_HEAD: &str = "module main\n  purpose: Use util.\n\nimport util exposing Word\n\n";

#[test]
fn public_definitions_of_another_module_are_reachable() {
    let codes = codes_with(
        &format!("{MAIN_HEAD}public function go() returns Text\n  purpose: Go.\n\n  let word be Word(text: \"hi\")\n  let limit be util.limit\n  let loud be util.shout(\"x\")\n  return \"{{loud}}{{word.twice()}}{{limit}}\"\nend\n"),
        UTIL,
    );
    assert_eq!(codes, Vec::<String>::new());
}

#[test]
fn private_definitions_stay_inside_their_module() {
    // a function
    let codes = codes_with(
        &format!("{MAIN_HEAD}public function go() returns Text\n  purpose: Go.\n\n  return util.whisper(\"x\")\nend\n"),
        UTIL,
    );
    assert_eq!(codes, vec!["private-name".to_string()]);
    // a function passed by name
    let codes = codes_with(
        &format!("{MAIN_HEAD}public function go(apply: function(Text) returns Text) returns Text\n  purpose: Go.\n\n  return apply(\"x\")\nend\n\npublic function main()\n  purpose: Run.\n\n  ignore go(util.whisper)\nend\n"),
        UTIL,
    );
    assert!(codes.contains(&"private-name".to_string()), "{codes:?}");
    // a constant
    let codes = codes_with(
        &format!("{MAIN_HEAD}public function go() returns Integer\n  purpose: Go.\n\n  return util.secret\nend\n"),
        UTIL,
    );
    assert_eq!(codes, vec!["private-name".to_string()]);
    // a method
    let codes = codes_with(
        &format!("{MAIN_HEAD}public function go() returns Text\n  purpose: Go.\n\n  let word be Word(text: \"hi\")\n  return word.once()\nend\n"),
        UTIL,
    );
    assert_eq!(codes, vec!["private-name".to_string()]);
    // an ability
    let codes = codes_with(
        "module main\n  purpose: Use util.\n\nimport util exposing Secretive\n\npublic function go() returns Text\n  purpose: Go.\n\n  return util.shout(\"x\")\nend\n",
        UTIL,
    );
    assert_eq!(codes, vec!["private-name".to_string()]);
}

#[test]
fn an_implementation_carries_the_abilitys_signature() {
    let head = "type Word\n  has text: Text\nend\n\nability Describable\n  function describe(self) returns Text\nend\n\n";
    clean(&program(&format!(
        "{head}ability Describable for Word\n  function describe(self) returns Text\n    return self.text\n  end\nend\n"
    )));
    // an extra parameter
    raises(
        &program(&format!(
            "{head}ability Describable for Word\n  function describe(self, extra: Integer) returns Text\n    return self.text\n  end\nend\n"
        )),
        "method-signature",
    );
    // another result
    raises(
        &program(&format!(
            "{head}ability Describable for Word\n  function describe(self) returns Integer\n    return 1\n  end\nend\n"
        )),
        "method-signature",
    );
    // a failure the ability does not declare
    raises(
        &program(&format!(
            "{head}ability Describable for Word\n  function describe(self) returns Text or fails with TimedOut\n    return self.text\n  end\nend\n"
        )),
        "method-signature",
    );
    // `Self` in the ability is the target
    clean(&program(
        "type Word\n  has text: Text\nend\n\nability Equal for Word\n  function equals(self, other: Word) returns Boolean\n    return self.text is other.text\n  end\nend\n",
    ));
    raises(
        &program(
            "type Word\n  has text: Text\nend\n\nability Equal for Word\n  function equals(self, other: Text) returns Boolean\n    return self.text is other\n  end\nend\n",
        ),
        "method-signature",
    );
}

#[test]
fn a_method_is_declared_in_the_module_of_its_type() {
    // decision K1
    raises(
        &program("function shout(self: Text) returns Text\n  return self.to_upper()\nend\n"),
        "method-module",
    );
    clean(&program(
        "type Word\n  has text: Text\nend\n\nfunction shout(self: Word) returns Text\n  return self.text.to_upper()\nend\n",
    ));
}

#[test]
fn a_higher_order_function_declares_only_its_own_effects() {
    let head = "function announce(text: Text) needs console\n  console.print(text)\nend\n\nfunction twice(action: function(Text) needs console, text: Text)\n  action(text)\n  action(text)\nend\n\n";
    // the needs of the function type are charged where the function is passed
    clean(&program(&format!(
        "{head}public function main() needs console\n  purpose: Run.\n\n  twice(action: announce, text: \"hi\")\nend\n"
    )));
    raises(
        &program(&format!(
            "{head}public function main()\n  purpose: Run.\n\n  twice(action: announce, text: \"hi\")\nend\n"
        )),
        "capability-missing",
    );
    // the needs of the function passed are charged, whatever the type lists
    // (decision B1)
    raises(
        &program(
            "function announce(text: Text) needs console\n  console.print(text)\nend\n\nfunction twice(action: function(Text), text: Text)\n  action(text)\n  action(text)\nend\n\npublic function main()\n  purpose: Run.\n\n  twice(action: announce, text: \"hi\")\nend\n",
        ),
        "capability-missing",
    );
}

#[test]
fn type_parameters_are_in_scope_inside_the_body() {
    clean(&program(
        "function first_or(items: List of Item, fallback: Item) returns Item for any Item\n  let chosen: Item be items.at(0) otherwise fallback\n  return chosen\nend\n",
    ));
}

#[test]
fn a_let_bound_literal_takes_its_own_type() {
    // sketch section 7: an Integer and a Decimal do not compare
    raises(
        &program(
            "function go() returns Boolean\n  let whole be 3\n  return whole is at least 2.5\nend\n",
        ),
        "type-mismatch",
    );
    clean(&program(
        "function go() returns Boolean\n  let whole be 3\n  return whole is at least 2\nend\n",
    ));
    clean(&program(
        "function go() returns Boolean\n  let part be 2.5\n  return part is at least 2\nend\n",
    ));
}

#[test]
fn exhaustiveness_looks_inside_the_patterns() {
    // a literal arm covers one value, never the type
    raises(
        &program(
            "function sign(value: Integer) returns Text\n  match value\n    when 0 then return \"zero\"\n    when 1 then return \"one\"\n  end\nend\n",
        ),
        "not-exhaustive",
    );
    clean(&program(
        "function sign(value: Integer) returns Text\n  match value\n    when 0 then return \"zero\"\n    otherwise return \"many\"\n  end\nend\n",
    ));
    // the pattern inside `some` counts
    raises(
        &program(
            "function first_or_zero(items: List of Integer) returns Integer\n  match items.first()\n    when some(0) then return 0\n    when nothing then return 0\n  end\nend\n",
        ),
        "not-exhaustive",
    );
    clean(&program(
        "function first_or_zero(items: List of Integer) returns Integer\n  match items.first()\n    when some(0) then return 0\n    when some(value) then return value\n    when nothing then return 0\n  end\nend\n",
    ));
    // a field pattern that is a literal leaves the variant uncovered
    let shape = "public type Shape is one of\n  purpose: A figure.\n  Circle(radius: Decimal)\n  Square(side: Decimal)\n  Point\nend\n\n";
    raises(
        &program(&format!(
            "{shape}function describe(shape: Shape) returns Text\n  match shape\n    when Circle(radius: 0) then return \"dot\"\n    when Square(side) then return \"square {{side}}\"\n    when Point then return \"point\"\n  end\nend\n"
        )),
        "not-exhaustive",
    );
    clean(&program(&format!(
        "{shape}function describe(shape: Shape) returns Text\n  match shape\n    when Circle(radius: 0) then return \"dot\"\n    when Circle(radius) then return \"circle {{radius}}\"\n    when Square(side) then return \"square {{side}}\"\n    when Point then return \"point\"\n  end\nend\n"
    )));
}

#[test]
fn a_failure_arm_covers_one_member_of_the_error_union() {
    let errors = "type Oops\n  has detail: Text\nend\n\ntype Ouch\n  has detail: Text\nend\n\nfunction risky(flag: Boolean) returns Integer or fails with Oops or Ouch\n  if flag then\n    fail with Oops(detail: \"x\")\n  end\n  fail with Ouch(detail: \"y\")\nend\n\n";
    raises(
        &program(&format!(
            "{errors}function handle(flag: Boolean) returns Integer\n  match risky(flag)\n    when success(value) then return value\n    when failure(error: Oops) then return error.detail.length()\n  end\nend\n"
        )),
        "not-exhaustive",
    );
    clean(&program(&format!(
        "{errors}function handle(flag: Boolean) returns Integer\n  match risky(flag)\n    when success(value) then return value\n    when failure(error: Oops) then return error.detail.length()\n    when failure(error: Ouch) then return error.detail.length()\n  end\nend\n"
    )));
    // a variant pattern covers one variant of one member
    let http = "public type HttpError is one of\n  purpose: What a request can fail with.\n  Status(code: Integer)\n  Network(detail: Text)\nend\n\ntype Oops\n  has detail: Text\nend\n\nfunction fetch(flag: Boolean) returns Integer or fails with HttpError or Oops\n  if flag then\n    fail with Status(code: 500)\n  end\n  fail with Oops(detail: \"x\")\nend\n\n";
    raises(
        &program(&format!(
            "{http}function handle(flag: Boolean) returns Integer\n  match fetch(flag)\n    when success(value) then return value\n    when failure(Status(code)) then return code\n    when failure(error: Oops) then return error.detail.length()\n  end\nend\n"
        )),
        "not-exhaustive",
    );
    clean(&program(&format!(
        "{http}function handle(flag: Boolean) returns Integer\n  match fetch(flag)\n    when success(value) then return value\n    when failure(Status(code)) then return code\n    when failure(Network(detail)) then return detail.length()\n    when failure(error: Oops) then return error.detail.length()\n  end\nend\n"
    )));
}
