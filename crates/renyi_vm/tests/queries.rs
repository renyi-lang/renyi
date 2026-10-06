//! Queries: the terminals after `group by` apply per group (decision M1).

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use renyi_syntax::SourceFile;
use renyi_vm::{compile_project, Options, Program, RunOutcome};

fn compile(name: &str, source: &str) -> Program {
    let file = SourceFile::new(name, source);
    let files = vec![file];
    let checked = renyi_check::check_project(&files);
    let errors: Vec<_> = checked.modules[0]
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
    compile_project(&checked, &files)
}

#[derive(Clone, Default)]
struct Capture(Rc<RefCell<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run(program: &Program) -> (RunOutcome, String) {
    let stdout = Capture::default();
    let options = Options {
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(std::io::sink()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        ..Options::default()
    };
    let outcome = renyi_vm::run_main(program, options);
    let text = String::from_utf8(stdout.0.borrow().clone()).expect("utf-8");
    (outcome, text)
}

#[test]
fn every_terminal_after_group_by_applies_per_group() {
    let source = r#"module demo
  purpose: Sums, counts, first items and conditions per group.

import std.console

type Sale
  has region: Text
  has amount: Decimal
  has paid: Boolean
end

public function main() needs console
  purpose: Print what each terminal gives per region, and the empty case.

  let sales be [
    Sale(region: "n", amount: 1.5, paid: true),
    Sale(region: "s", amount: 2.0, paid: false),
    Sale(region: "n", amount: 3.0, paid: false),
  ]
  let sums be for each sale in sales group by sale.region sum sale.amount
  for each region, total in sums sorted by region
    console.print("{region} sum {total}")
  end
  let counts be for each sale in sales group by sale.region count
  for each region, number in counts sorted by region
    console.print("{region} count {number}")
  end
  let firsts be for each sale in sales group by sale.region first
  for each region, found in firsts sorted by region
    let amount be
      match found
        when some(sale) then sale.amount
        when nothing then 0.0
      end
    console.print("{region} first {amount}")
  end
  let any_paid be for each sale in sales group by sale.region any sale.paid
  for each region, flag in any_paid sorted by region
    console.print("{region} any {flag}")
  end
  let all_paid be for each sale in sales group by sale.region all sale.paid
  for each region, flag in all_paid sorted by region
    console.print("{region} all {flag}")
  end
  let whole be for each sale in sales group by sale.region
  for each region, items in whole sorted by region
    console.print("{region} items {items.length()}")
  end
  let none: List of Sale be []
  let empty be for each sale in none group by sale.region sum sale.amount
  console.print("empty {empty.length()}")
end
"#;
    let program = compile("demo.ry", source);
    let (outcome, printed) = run(&program);
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        printed,
        "n sum 4.5\ns sum 2.0\nn count 2\ns count 1\nn first 1.5\ns first 2.0\nn any true\ns any false\nn all false\ns all false\nn items 2\ns items 1\nempty 0\n"
    );
}
