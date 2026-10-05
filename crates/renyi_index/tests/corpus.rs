//! The project map on the example corpus and on small programs: every
//! definition gets a record, the records of `invoice` match a hand count,
//! hashes are stable, rename-invariant and change-propagating, and the
//! metrics and effects follow their definitions.

use std::collections::HashSet;
use std::path::PathBuf;

use renyi_index::{index_files, load_project, Definition, Header, Index, Kind, Metrics};
use renyi_syntax::SourceFile;

fn header() -> Header {
    Header {
        project: "test".into(),
        revision: "0".into(),
        toolchain: "renyi test".into(),
    }
}

fn corpus() -> Index {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let files = load_project(&dir).expect("the corpus loads");
    assert!(files.len() >= 30);
    index_files(&files, header())
}

fn program(source: &str) -> Index {
    index_files(&[SourceFile::new("demo.ry", source)], header())
}

fn find<'a>(index: &'a Index, module: &str, name: &str) -> &'a Definition {
    index
        .definitions
        .iter()
        .find(|d| d.module == module && d.name == name)
        .unwrap_or_else(|| panic!("no definition {module}.{name}"))
}

fn ids(index: &Index, module: &str) -> Vec<(String, String)> {
    index
        .definitions
        .iter()
        .filter(|d| d.module == module)
        .map(|d| (d.name.clone(), d.id.clone()))
        .collect()
}

#[test]
fn the_corpus_indexes_cleanly() {
    let index = corpus();
    assert_eq!(index.modules.len(), 30);
    for module in &index.modules {
        assert_eq!(module.errors, 0, "{} has errors", module.name);
        assert!(module.canonical, "{} is not canonical", module.name);
        assert!(!module.ids.is_empty(), "{} has no definitions", module.name);
        assert_eq!(module.definitions, module.ids.len());
    }
    let distinct: HashSet<&String> = index.definitions.iter().map(|d| &d.id).collect();
    assert_eq!(distinct.len(), index.definitions.len(), "ids are distinct");
    for definition in &index.definitions {
        assert!(definition.id.starts_with("sha256:"), "{}", definition.id);
        assert_eq!(definition.id.len(), "sha256:".len() + 64);
        assert!(definition.line >= 1 && definition.end_line >= definition.line);
        assert_eq!(
            definition.metrics.lines,
            definition.end_line - definition.line + 1
        );
        assert!(definition.metrics.depth <= 4, "{}", definition.qualified());
        assert_eq!(definition.tests, definition.tested_by.len());
    }
    // the same text indexes to the same ids
    let again = corpus();
    assert_eq!(index.definitions, again.definitions);
    // the text and JSON forms mention every definition
    let text = renyi_index::to_text(&index);
    let json = renyi_index::to_json(&index);
    for definition in &index.definitions {
        assert!(
            text.contains(&definition.qualified()),
            "{}",
            definition.qualified()
        );
        assert!(json.contains(&definition.id));
    }
}

#[test]
fn invoice_records_match_a_hand_count() {
    let index = corpus();
    let module = index
        .modules
        .iter()
        .find(|m| m.name == "invoice")
        .expect("invoice");
    assert_eq!((module.definitions, module.public), (7, 5));
    assert!(module.effects.is_empty());
    assert!(
        module.file.ends_with("examples/invoice.ry"),
        "{}",
        module.file
    );

    let total = find(&index, "invoice", "total");
    assert_eq!(total.kind, Kind::Function);
    assert!(total.public);
    assert_eq!(total.signature, "total(invoice: Invoice) returns Decimal");
    assert_eq!(
        total.purpose.as_deref(),
        Some("Subtotal after the percentage discount, rounded to cents.")
    );
    assert_eq!(total.see_also, vec!["subtotal"]);
    assert_eq!(
        total.calls,
        vec!["invoice.subtotal", "std.prelude.Decimal.rounded"]
    );
    assert_eq!(total.uses, vec!["invoice.Invoice", "std.prelude.Decimal"]);
    assert_eq!(
        total.tested_by,
        vec![
            "invoice.test:a ten percent discount is taken from the subtotal",
            "invoice.test:an invoice without lines totals zero",
        ]
    );
    assert_eq!((total.examples, total.tests), (0, 2));
    assert_eq!((total.line, total.end_line), (34, 41));
    assert_eq!(
        total.metrics,
        Metrics {
            lines: 8,
            depth: 0,
            branches: 0,
            effects: 0,
            // two tests here, and two functions of invoice_report, which imports invoice
            fan_in: 4,
            fan_out: 2,
            library_calls: 1,
        }
    );

    let subtotal = find(&index, "invoice", "subtotal");
    assert_eq!(subtotal.calls, vec!["invoice.line_total"]);
    assert_eq!(subtotal.metrics.branches, 1, "one query");
    assert_eq!(subtotal.metrics.fan_in, 2, "total and one test");

    let line_total = find(&index, "invoice", "line_total");
    assert_eq!(line_total.examples, 1);
    assert_eq!(line_total.calls, vec!["std.prelude.Integer.to_decimal"]);
    assert_eq!(
        line_total.metrics.fan_in, 1,
        "subtotal only; examples are not references"
    );

    let line = find(&index, "invoice", "Line");
    assert_eq!(line.kind, Kind::Type);
    assert_eq!(
        line.signature,
        "Line(description: Text, quantity: Integer where quantity is at least 1, \
         unit_price: Decimal where unit_price is at least 0)"
    );
    assert_eq!(
        line.metrics.fan_in, 4,
        "line_total, Invoice, one test and invoice_report"
    );
    assert_eq!(line.tests, 1);

    let test = find(
        &index,
        "invoice",
        "test:an invoice without lines totals zero",
    );
    assert_eq!(test.kind, Kind::Test);
    assert_eq!(test.signature, "\"an invoice without lines totals zero\"");
    assert_eq!(test.uses, vec!["invoice.Invoice"]);
    assert_eq!(test.calls, vec!["invoice.total"]);
}

#[test]
fn shapes_records_cover_abilities_and_implementations() {
    let index = corpus();
    let implementation = find(&index, "shapes", "Measurable for Shape");
    assert_eq!(implementation.kind, Kind::Implementation);
    let implements = implementation.implements.as_ref().expect("implements");
    assert_eq!(implements.ability, "shapes.Measurable");
    assert_eq!(implements.target, "shapes.Shape");
    assert_eq!(implementation.metrics.branches, 3, "three match arms");
    assert_eq!(implementation.metrics.depth, 1);

    let area = find(&index, "shapes", "Shape.area");
    assert_eq!(area.kind, Kind::Method);
    assert_eq!(area.signature, "area(self) returns Decimal");
    assert_eq!(area.implements, implementation.implements);
    assert!(area.calls.is_empty());

    let describe = find(&index, "shapes", "describe");
    assert_eq!(
        describe.calls,
        vec!["shapes.Shape.area", "std.prelude.Decimal.rounded"]
    );
    assert_eq!(describe.metrics.branches, 3);

    let main = find(&index, "shapes", "main");
    assert_eq!(main.effects_declared, vec!["console"]);
    assert_eq!(main.effects_transitive, vec!["console"]);
    assert_eq!(
        main.calls,
        vec!["shapes.describe", "shapes.largest", "std.console.print",]
    );
    assert_eq!(main.metrics.effects, 1);
    assert_eq!(main.metrics.library_calls, 1);

    let ability = find(&index, "shapes", "Measurable");
    assert_eq!(ability.kind, Kind::Ability);
    assert_eq!(ability.signature, "Measurable: area(self) returns Decimal");
    assert!(
        ability.metrics.fan_in >= 1,
        "the implementation refers to it"
    );
}

const CALLER: &str = "module demo
  purpose: Hash tests.

public function double(value: Integer) returns Integer
  purpose: Twice the answer.

  return helper(value) * 2
end

function helper(value: Integer) returns Integer
  return value + 1
end
";

#[test]
fn renaming_keeps_hashes_and_a_body_change_propagates() {
    let before = ids(&program(CALLER), "demo");
    let renamed = CALLER.replace("helper", "assistant");
    let after_rename = ids(&program(&renamed), "demo");
    assert_eq!(
        before[0].1, after_rename[0].1,
        "the caller's hash survives the rename"
    );
    assert_eq!(
        before[1].1, after_rename[1].1,
        "the renamed definition keeps its hash"
    );
    assert_eq!(after_rename[1].0, "assistant");

    let changed = CALLER.replace("value + 1", "value + 2");
    let after_change = ids(&program(&changed), "demo");
    assert_ne!(before[1].1, after_change[1].1, "the body change is seen");
    assert_ne!(before[0].1, after_change[0].1, "and reaches the caller");
}

#[test]
fn mutual_recursion_gets_distinct_stable_hashes() {
    let source = "module demo
  purpose: Mutual recursion.

public function is_even(number: Integer) returns Boolean
  purpose: Whether the number is even.
  example: is_even(4) is true

  if number is 0 then return true end
  return is_odd(number - 1)
end

public function is_odd(number: Integer) returns Boolean
  purpose: Whether the number is odd.
  example: is_odd(3) is true

  if number is 0 then return false end
  return is_even(number - 1)
end
";
    let first = ids(&program(source), "demo");
    let second = ids(&program(source), "demo");
    assert_eq!(first, second);
    assert_ne!(first[0].1, first[1].1);
    let index = program(source);
    let even = find(&index, "demo", "is_even");
    assert_eq!(even.calls, vec!["demo.is_odd"]);
    assert_eq!(even.metrics.fan_in, 1);
    assert_eq!(even.metrics.fan_out, 1);
    assert_eq!(even.metrics.branches, 1);
    assert_eq!(even.metrics.depth, 1);
}

#[test]
fn depth_and_branches_follow_their_definitions() {
    let source = "module demo
  purpose: Metrics.

public function classify(value: Integer) returns Text
  purpose: Name a number.

  if value is less than 0 then
    return \"negative\"
  otherwise if value is 0 then
    return \"zero\"
  otherwise
    for each step from 1 to value
      if step is 3 and value is greater than 5 then
        return \"big\"
      end
    end
    return \"positive\"
  end
end
";
    let index = program(source);
    let classify = find(&index, "demo", "classify");
    assert_eq!(classify.metrics.depth, 3, "if, loop, if");
    assert_eq!(
        classify.metrics.branches, 5,
        "two if conditions, a loop header, an inner if, one `and`"
    );
}

#[test]
fn effects_and_failures_are_transitive() {
    let source = "module demo
  purpose: Effects.

import std.console

public type Oops is one of
  purpose: An error.
  Oops
end

public function main() needs console
  purpose: Greet twice.

  greet()
  let answer be risky() otherwise 0
  console.print(\"{answer}\")
end

function greet() needs console
  console.print(\"hi\")
end

function risky() returns Integer or fails with Oops
  fail with Oops
end
";
    let index = program(source);
    let main = find(&index, "demo", "main");
    assert_eq!(main.effects_declared, vec!["console"]);
    assert_eq!(main.effects_transitive, vec!["console"]);
    assert!(main.fails_declared.is_empty());
    assert_eq!(main.fails_transitive, vec!["Oops"]);
    assert_eq!(
        main.calls,
        vec!["demo.greet", "demo.risky", "std.console.print"]
    );
    assert_eq!(main.metrics.branches, 1, "the otherwise fallback");
    assert_eq!(main.metrics.fan_out, 2);
    assert_eq!(main.metrics.library_calls, 1);
    let greet = find(&index, "demo", "greet");
    assert_eq!(greet.metrics.fan_in, 1);
    assert_eq!(greet.effects_transitive, vec!["console"]);
    let risky = find(&index, "demo", "risky");
    assert_eq!(risky.fails_declared, vec!["Oops"]);
    assert_eq!(risky.uses, vec!["demo.Oops", "std.prelude.Integer"]);
    let oops = find(&index, "demo", "Oops");
    assert_eq!(
        oops.metrics.fan_in, 1,
        "risky names it; main reaches it only through the call"
    );
}

#[test]
fn the_corpus_is_within_the_default_budgets() {
    // decision R7: the defaults sit just above the corpus maxima
    let over = renyi_index::over_budget(&corpus(), &renyi_index::Budgets::default());
    assert!(over.is_empty(), "{over:?}");
}

#[test]
fn values_over_budget_are_reported() {
    let mut source = String::from("module demo\n  purpose: Too wide.\n\nimport std.console\nimport std.environment\nimport std.time\nimport std.random\nimport std.filesystem exposing Path\nimport std.http exposing Url\n\n");
    for n in 0..11 {
        source.push_str(&format!(
            "public function f{n}() returns Integer\n  purpose: Number {n}.\n\n  return {n}\nend\n\n"
        ));
    }
    source.push_str(
        "public function wide() returns Integer needs console, environment, time, random, filesystem, network.http\n  purpose: Touches everything and calls every f.\n\n  console.print(\"x\")\n  ignore environment.arguments()\n  ignore time.now()\n  ignore random.integer(lowest: 0, highest: 1)\n  ignore filesystem.read_text(Path(\"x\")) otherwise \"\"\n  ignore http.get(Url(\"https://example.com\")) otherwise return 0\n  return f0() + f1() + f2() + f3() + f4() + f5() + f6() + f7()\nend\n",
    );
    let index = program(&source);
    assert_eq!(index.modules[0].errors, 0, "the program must check");
    let over = renyi_index::over_budget(&index, &renyi_index::Budgets::default());
    assert!(
        over.iter()
            .any(|l| l.starts_with("module demo: 12 public definitions (budget 10)")),
        "{over:?}"
    );
    assert!(
        over.iter()
            .any(|l| l.contains("transitive effect paths (budget 5)")),
        "{over:?}"
    );
    assert!(
        over.iter()
            .any(|l| l.starts_with("function demo.wide: fan-out 8 (budget 7)")),
        "{over:?}"
    );
}

#[test]
fn a_file_with_errors_is_still_indexed() {
    let index = program(
        "module demo
  purpose: Broken.

public function ok() returns Integer
  purpose: Fine.

  return 1
end

public function broken() returns Integer
  purpose: Calls a function that does not exist.

  return missing()
end
",
    );
    assert_eq!(index.modules[0].errors, 1);
    assert_eq!(index.modules[0].definitions, 2);
    let broken = find(&index, "demo", "broken");
    assert!(broken.calls.is_empty());
}
