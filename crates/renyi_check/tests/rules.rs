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
