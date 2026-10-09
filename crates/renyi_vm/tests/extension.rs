//! The registration API (decisions AJ1 and AK1 to AK4): the standard
//! library passes its own check, an extension of three natives checks,
//! runs, is recorded and is refused at the boundary like a library module,
//! and the check names what is declared without a native or implemented
//! without a declaration.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use renyi_check::check_project_in;
use renyi_syntax::SourceFile;
use renyi_vm::natives::{arg, crash, plain_small, small, text};
use renyi_vm::{
    compile_project, Extension, Interrupt, Native, Options, Program, Registry, RunOutcome, Typed,
    Value, Vm,
};

const DEMO_MODULE: &str = r#"module demo
  purpose: The extension of the test: three natives.

import std.filesystem exposing Path

public type PeekError is one of
  purpose: What peek reports.
  PermissionDenied(path: Path)
  Missing(path: Path)
end

public function twice(amount: Integer) returns Integer
  purpose: Twice the amount.
public function shout(text: Text) needs console
  purpose: Print the text in capitals with a bang.
public function peek(path: Path) returns Text or fails with PeekError needs filesystem.read
  purpose: The first line of the file.
"#;

fn twice(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(small(arg(args, 0))? * 2))
}

/// How often the typed entry of `twice` was asked.
static TWICE_ASKED: AtomicUsize = AtomicUsize::new(0);

/// The typed entry of `twice` (decision AU1): it answers for a
/// non-negative small Integer and declines for the rest, so that the test
/// sees both ways through the call.
fn twice_typed(args: &[Value]) -> Option<i64> {
    TWICE_ASKED.fetch_add(1, Ordering::Relaxed);
    let value = plain_small(&args[0])?;
    if value < 0 {
        return None;
    }
    value.checked_mul(2)
}

fn shout(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let line = text(arg(args, 0))?.to_uppercase();
    writeln!(vm.stdout, "{line}!").map_err(|error| crash(format!("cannot write: {error}")))?;
    Ok(Value::Nothing)
}

fn peek(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?.to_string();
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(Value::text(content.lines().next().unwrap_or(""))),
        Err(_) => vm.fail_variant("demo", "PeekError", "Missing", vec![Value::text(path)]),
    }
}

const DEMO: Extension = Extension {
    name: "demo",
    version: "0.1.0",
    modules: &[("demo", DEMO_MODULE)],
    natives: &[
        Native::function("demo", "twice", twice).with_typed(Typed::Int(twice_typed)),
        Native::function("demo", "shout", shout),
        Native::function("demo", "peek", peek),
    ],
};

/// A program that calls `twice` hot (a loop the machine code runs) and
/// cold, on numbers its typed entry answers for and on numbers it
/// declines.
const TYPED_PROGRAM: &str = r#"module demo_program
  purpose: Call the extension's typed entry hot and cold.

import demo
import std.console

function doubled_all(limit: Integer) returns Integer
  purpose: The sum of twice each number up to the limit and of twice its negation: zero.

  let mutable total be 0
  for each index from 1 to limit
    change total to total + demo.twice(index) + demo.twice(0 - index)
  end
  return total
end

public function main() needs console
  purpose: Print the sum and two doublings.

  console.print("{doubled_all(1000)} {demo.twice(21)} {demo.twice(0 - 21)}")
end
"#;

const PROGRAM: &str = r#"module demo_program
  purpose: Use the demo extension.

import demo exposing PeekError
import std.filesystem exposing Path
import std.console

public function main() or fails with PeekError needs console, filesystem.read("data")
  purpose: Call the extension's natives.

  console.print(demo.twice(21).to_text())
  demo.shout("hello")
  let secret be demo.peek(Path("elsewhere/secret.txt")) otherwise fail
  console.print(secret)
end
"#;

/// A writer the test can read back after the run.
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

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("utf-8")
    }
}

fn compile(registry: &Registry, source: &str) -> Program {
    let files = vec![SourceFile::new("demo_program.ry", source)];
    let checked = check_project_in(&registry.library(), &files, &[]);
    for module in &checked.modules {
        let errors: Vec<_> = module.diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "{errors:?}");
    }
    compile_project(&checked, &files)
}

#[test]
fn the_standard_library_is_the_first_extension() {
    let registry = Registry::standard();
    assert_eq!(registry.verify(), Ok(()));
    let names: Vec<&str> = registry.extensions().iter().map(|e| e.name).collect();
    assert_eq!(names, vec!["std"]);
    assert!(registry.extras().is_empty());
}

#[test]
fn a_lookup_takes_the_full_receiver_then_its_head_then_the_bare_name() {
    let registry = Registry::standard();
    assert!(registry
        .lookup("std.prelude", "sum", Some("List of Integer"))
        .is_some());
    assert!(registry
        .lookup("std.prelude", "sum", Some("List of Text"))
        .is_none());
    assert!(registry
        .lookup("std.prelude", "length", Some("List of Item"))
        .is_some());
    assert!(registry
        .lookup("std.filesystem", "read_text", Some("Path"))
        .is_some());
    assert!(registry.lookup("std.time", "now", None).is_some());
    assert!(registry.lookup("demo", "twice", Some("Integer")).is_none());
}

#[test]
fn an_extension_checks_runs_and_is_refused_at_the_boundary() {
    let registry = Registry::standard().with(DEMO);
    assert_eq!(registry.verify(), Ok(()));
    assert_eq!(registry.extras(), vec!["demo 0.1.0".to_string()]);
    let program = compile(&registry, PROGRAM);
    let stdout = Capture::default();
    let options = Options {
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(Capture::default()),
        record: true,
        registry,
        ..Options::default()
    };
    let run = renyi_vm::run_program(&program, options);
    // `twice` and `shout` ran; `peek` was denied through the extension's
    // own failure type (decision AK4), and `main` failed with it
    assert_eq!(stdout.text(), "42\nHELLO!\n");
    match &run.outcome {
        RunOutcome::Failed(error) => {
            assert!(
                error.contains("PermissionDenied") && error.contains("elsewhere/secret.txt"),
                "{error}"
            );
        }
        other => panic!("{}", renyi_vm::describe_outcome(other)),
    }
    // the call of `shout` is one recorded primitive under `console`
    let recording = run.recording.expect("a recording");
    let shout = recording
        .calls
        .iter()
        .find(|call| call.primitive == "demo.shout")
        .expect("the call of shout");
    assert_eq!(shout.capability, "console");
}

#[test]
fn a_native_of_an_extension_fails_with_its_own_type() {
    let registry = Registry::standard().with(DEMO);
    let source = PROGRAM.replace("filesystem.read(\"data\")", "filesystem.read");
    let program = compile(&registry, &source);
    let options = Options {
        stdout: Box::new(Capture::default()),
        stderr: Box::new(Capture::default()),
        registry,
        ..Options::default()
    };
    match renyi_vm::run_main(&program, options) {
        RunOutcome::Failed(error) => assert!(error.contains("Missing"), "{error}"),
        other => panic!("{}", renyi_vm::describe_outcome(&other)),
    }
}

#[test]
fn a_typed_entry_answers_on_both_tiers_and_declines_to_the_native() {
    // every code object compiled before its first call (as `native.rs`
    // runs), so that the loop's calls go through the generated code
    std::env::set_var("RENYI_NATIVE_HOT", "0");
    let registry = Registry::standard().with(DEMO);
    assert_eq!(registry.verify(), Ok(()));
    let program = compile(&registry, TYPED_PROGRAM);
    for interpret in [false, true] {
        let stdout = Capture::default();
        let asked = TWICE_ASKED.load(Ordering::Relaxed);
        let outcome = renyi_vm::run_main(
            &program,
            Options {
                stdout: Box::new(stdout.clone()),
                stderr: Box::new(Capture::default()),
                registry: registry.clone(),
                interpret,
                ..Options::default()
            },
        );
        assert_eq!(outcome, RunOutcome::Finished, "interpret: {interpret}");
        assert_eq!(stdout.text(), "0 42 -42\n", "interpret: {interpret}");
        // the entry was asked at every call: it answered the non-negative
        // ones and declined the negative ones to the native
        assert!(
            TWICE_ASKED.load(Ordering::Relaxed) - asked >= 2002,
            "interpret: {interpret}"
        );
    }
}

#[test]
fn the_check_holds_a_typed_entry_to_its_declaration() {
    fn wrong_kind(_: &[Value]) -> Option<bool> {
        None
    }
    fn skips(_: &[Value]) -> Option<Value> {
        None
    }
    const WRONG_KIND: &[Native] = &[
        Native::function("demo", "twice", twice).with_typed(Typed::Bool(wrong_kind)),
        Native::function("demo", "shout", shout),
        Native::function("demo", "peek", peek),
    ];
    let problem = Registry::standard()
        .with(Extension {
            natives: WRONG_KIND,
            ..DEMO
        })
        .verify()
        .unwrap_err();
    assert_eq!(
        problem,
        "extension `demo`: the typed entry of `demo.twice` on `Integer` answers a Boolean where the declaration returns `Integer`"
    );
    const SKIPS_THE_BOUNDARY: &[Native] = &[
        Native::function("demo", "twice", twice),
        Native::function("demo", "shout", shout).with_typed(Typed::Value(skips)),
        Native::function("demo", "peek", peek),
    ];
    let problem = Registry::standard()
        .with(Extension {
            natives: SKIPS_THE_BOUNDARY,
            ..DEMO
        })
        .verify()
        .unwrap_err();
    assert_eq!(
        problem,
        "extension `demo`: `demo.shout` on `Text` has a typed entry but needs `console`; a typed entry skips the boundary"
    );
}

#[test]
fn the_check_names_a_declaration_without_a_native() {
    const NATIVES: &[Native] = &[
        Native::function("demo", "twice", twice),
        Native::function("demo", "shout", shout),
    ];
    let extension = Extension {
        natives: NATIVES,
        ..DEMO
    };
    let problem = Registry::standard().with(extension).verify().unwrap_err();
    assert_eq!(
        problem,
        "extension `demo`: `demo.peek` on `Path` is declared but has no native"
    );
}

#[test]
fn the_check_names_a_native_without_a_declaration() {
    const NATIVES: &[Native] = &[
        Native::function("demo", "twice", twice),
        Native::function("demo", "shout", shout),
        Native::function("demo", "peek", peek),
        Native::method("demo", "whisper", "Text", shout),
    ];
    let extension = Extension {
        natives: NATIVES,
        ..DEMO
    };
    let problem = Registry::standard().with(extension).verify().unwrap_err();
    assert_eq!(
        problem,
        "extension `demo`: the native `demo.whisper` on `Text` implements no declared function"
    );
}

#[test]
fn the_check_refuses_a_capability_outside_the_reference() {
    const MODULE: &str = "module gadget\n  purpose: A capability the reference does not know.\n\npublic function blink() needs gpu\n  purpose: Blink.\n";
    const NATIVES: &[Native] = &[Native::function("gadget", "blink", twice)];
    let extension = Extension {
        name: "gadget",
        version: "0.1.0",
        modules: &[("gadget", MODULE)],
        natives: NATIVES,
    };
    let problem = Registry::standard().with(extension).verify().unwrap_err();
    assert!(
        problem.starts_with("extension `gadget`: `gadget`: ") && problem.contains("gpu"),
        "{problem}"
    );
}

#[test]
fn the_check_refuses_a_module_declared_twice_or_under_another_name() {
    let twice_declared = Registry::standard()
        .with(DEMO)
        .with(DEMO)
        .verify()
        .unwrap_err();
    assert_eq!(
        twice_declared,
        "the module `demo` is declared by the extensions `demo` and `demo`"
    );
    let renamed = Extension {
        modules: &[("demonstration", DEMO_MODULE)],
        ..DEMO
    };
    let problem = Registry::standard().with(renamed).verify().unwrap_err();
    assert_eq!(
        problem,
        "extension `demo`: the declaration file registered as `demonstration` declares `module demo`"
    );
}
