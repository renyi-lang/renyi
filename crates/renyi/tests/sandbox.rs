//! The embedding API (decisions Q3 and AP1): a module loaded with a
//! grant answers calls to its public functions and nothing else; a call
//! that needs more than the grant is refused before it runs; budgets
//! count across the calls of one sandbox; a guarded value cannot leave
//! through another sink; a memory budget stops a call that grows past
//! it; and `renyi run --sandbox` on the command line.

use std::path::PathBuf;
use std::process::Command;

use renyi::{CallError, Grant, Sandbox, Value};

/// The counting allocator the memory budget needs (decision AP1).
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch directory under the workspace's `target`, as a path with
/// forward slashes.
fn scratch(name: &str) -> String {
    let directory = root().join("target/sandbox").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean scratch directory");
    }
    std::fs::create_dir_all(&directory).expect("the scratch directory");
    directory.to_str().expect("a path").replace('\\', "/")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

const CALC: &str = "module calc
  purpose: A module for the test of the sandbox.

import std.console
import std.filesystem exposing FileError, Path

type Oops
  purpose: A failure for the test.
  has value: Integer
end

public function add(left: Integer, right: Integer) returns Integer
  purpose: Add two numbers.

  return left + right
end

function hidden() returns Integer
  purpose: Not for the host.

  return 42
end

public function secret_number() returns Integer
  purpose: The number the hidden function knows.

  return hidden()
end

public function shout(text: Text) returns Text needs console
  purpose: Print the text and return it in capitals.

  console.print(text)
  return text.to_upper()
end

public function risky(number: Integer) returns Integer or fails with Oops
  purpose: Fail below zero.

  if number is at least 0 then return number end
  fail with Oops(value: number)
end

public function boom() returns Integer
  purpose: Crash on purpose.

  crash with \"boom\"
end

public function grow(size: Integer) returns Integer
  purpose: Build a list of size items and return its length.

  let items be
    for each index from 1 to size
    collect index
  return items.length()
end

public function peek(path: Text) returns Text or fails with FileError needs filesystem.read
  purpose: Read the file at the path.

  let target be Path(path)
  return filesystem.read_text(target) otherwise fail
end

public function stash(secret: Text, path: Text) or fails with FileError needs filesystem.write
  purpose: Write the secret to the path.

  let target be Path(path)
  filesystem.write_text(path: target, content: secret) otherwise fail
end
";

fn sandbox(grant: &str) -> Sandbox {
    Sandbox::load_source("calc.ry", CALC, Grant::parse(grant).expect("a grant"))
        .expect("the module loads")
}

#[test]
fn a_module_answers_its_public_functions_under_the_grant() {
    let mut calc = sandbox("console");
    assert_eq!(calc.module(), "calc");
    assert_eq!(calc.warnings(), "");
    let names: Vec<&str> = calc
        .functions()
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "add",
            "secret_number",
            "shout",
            "risky",
            "boom",
            "grow",
            "peek",
            "stash"
        ]
    );
    let shout = calc
        .functions()
        .iter()
        .find(|function| function.name == "shout")
        .expect("shout");
    assert_eq!(shout.parameters, 1);
    assert_eq!(shout.needs, ["console"]);
    assert_eq!(
        shout.purpose.as_deref(),
        Some("Print the text and return it in capitals.")
    );
    assert!(
        shout.signature.contains("needs console"),
        "{}",
        shout.signature
    );
    assert!(!shout.exposed_as_tool);

    assert_eq!(
        calc.call("add", vec![Value::integer(2), Value::integer(3)]),
        Ok(Value::integer(5))
    );
    assert_eq!(calc.call("secret_number", vec![]), Ok(Value::integer(42)));
    assert_eq!(
        calc.call("hidden", vec![]),
        Err(CallError::Refused(
            "`calc` has no public function `hidden`".to_string()
        ))
    );
    assert_eq!(
        calc.call("add", vec![Value::integer(1)]),
        Err(CallError::Refused(
            "`add` takes 2 arguments, not 1".to_string()
        ))
    );
    assert_eq!(
        calc.call("shout", vec![Value::text("hi")]),
        Ok(Value::text("HI"))
    );
    assert_eq!(
        calc.call("risky", vec![Value::integer(-1)]),
        Err(CallError::Failed("Oops(value: -1)".to_string()))
    );
    assert_eq!(
        calc.call("risky", vec![Value::integer(7)]),
        Ok(Value::integer(7))
    );
    match calc.call("boom", vec![]) {
        Err(CallError::Crashed { message, location }) => {
            assert_eq!(message, "boom");
            assert!(
                location
                    .as_deref()
                    .is_some_and(|at| at.starts_with("calc.ry:")),
                "{location:?}"
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_call_outside_the_grant_is_refused_before_it_runs() {
    let mut calc = sandbox("");
    assert_eq!(
        calc.call("shout", vec![Value::text("hi")]),
        Err(CallError::Refused(
            "`shout` needs `console`, which the grant does not cover".to_string()
        ))
    );
    assert_eq!(
        calc.call("add", vec![Value::integer(1), Value::integer(1)]),
        Ok(Value::integer(2))
    );
    // the directory given, not another one
    let dir = scratch("elsewhere");
    let mut calc = sandbox(&format!("filesystem.read(\"{dir}/inside\")"));
    std::fs::create_dir_all(format!("{dir}/inside")).expect("the inside directory");
    std::fs::write(format!("{dir}/inside/a.txt"), "in").expect("a.txt");
    std::fs::write(format!("{dir}/b.txt"), "out").expect("b.txt");
    assert_eq!(
        calc.call("peek", vec![Value::text(format!("{dir}/inside/a.txt"))]),
        Ok(Value::text("in"))
    );
    match calc.call("peek", vec![Value::text(format!("{dir}/b.txt"))]) {
        Err(CallError::Failed(error)) => {
            assert!(error.starts_with("PermissionDenied("), "{error}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn budgets_count_across_the_calls_of_one_sandbox() {
    let dir = scratch("budget");
    let path = format!("{dir}/note.txt");
    std::fs::write(&path, "hello").expect("note.txt");
    let mut calc = sandbox(&format!("filesystem.read(\"{dir}\") at most 2 per run"));
    assert_eq!(
        calc.call("peek", vec![Value::text(&path)]),
        Ok(Value::text("hello"))
    );
    assert_eq!(
        calc.call("peek", vec![Value::text(&path)]),
        Ok(Value::text("hello"))
    );
    match calc.call("peek", vec![Value::text(&path)]) {
        Err(CallError::Failed(error)) => assert!(error.starts_with("OverBudget("), "{error}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_guarded_value_leaves_only_through_the_sinks_of_its_guard() {
    let dir = scratch("guard");
    let mut calc = sandbox(&format!(
        "console, filesystem.read(\"{dir}\") only to console, filesystem.write(\"{dir}\")"
    ));
    let secret = calc
        .guarded(Value::text("key"), &format!("filesystem.read(\"{dir}\")"))
        .expect("a guarded value");
    assert_ne!(secret.origins(), 0);
    let shouted = calc
        .call("shout", vec![secret.clone()])
        .expect("console is a sink");
    assert_eq!(shouted.plain(), &Value::text("KEY"));
    let leak = format!("{dir}/leak.txt");
    match calc.call("stash", vec![secret, Value::text(&leak)]) {
        Err(CallError::Failed(error)) => assert!(error.starts_with("Guarded(origin:"), "{error}"),
        other => panic!("{other:?}"),
    }
    assert!(!std::path::Path::new(&leak).exists());
    assert_eq!(
        calc.call("stash", vec![Value::text("plain"), Value::text(&leak)]),
        Ok(Value::Nothing)
    );
    assert_eq!(std::fs::read_to_string(&leak).expect("leak.txt"), "plain");
    assert!(calc
        .guarded(Value::text("x"), "console")
        .unwrap_err()
        .contains("no `only to` guard on `console`"));
}

#[test]
fn a_memory_budget_stops_a_call_that_grows_past_it() {
    let mut grant = Grant::parse("").expect("an empty grant");
    grant.memory = Some(8 << 20);
    let mut calc = Sandbox::load_source("calc.ry", CALC, grant).expect("the module loads");
    assert_eq!(
        calc.call("grow", vec![Value::integer(1000)]),
        Ok(Value::integer(1000))
    );
    match calc.call("grow", vec![Value::integer(4_000_000)]) {
        Err(CallError::OverMemory { limit, used }) => {
            assert_eq!(limit, 8 << 20);
            assert!(used > limit, "{used}");
        }
        other => panic!("{other:?}"),
    }
    // the next call runs again, the budget counted afresh
    assert_eq!(
        calc.call("grow", vec![Value::integer(10)]),
        Ok(Value::integer(10))
    );
}

#[test]
fn run_takes_a_sandbox_grant_from_a_file() {
    let dir = scratch("cli");
    let wide = format!("{dir}/wide.json");
    let narrow = format!("{dir}/narrow.json");
    let broken = format!("{dir}/broken.json");
    std::fs::write(
        &wide,
        "{\"grant\": \"console, environment\", \"memory\": \"64 megabytes\"}\n",
    )
    .expect("wide.json");
    std::fs::write(&narrow, "{\"grant\": \"console\"}\n").expect("narrow.json");
    std::fs::write(&broken, "{\"grant\": \"teleport\"}\n").expect("broken.json");
    let run = |grant: &str| {
        Command::new(env!("CARGO_BIN_EXE_renyi"))
            .current_dir(root())
            .args(["run", "--sandbox", grant, "examples/hello.ry", "Renyi"])
            .output()
            .expect("the renyi binary runs")
    };
    let ok = run(&wide);
    assert!(ok.status.success(), "{}", text(&ok.stderr));
    assert_eq!(text(&ok.stdout), "Hello, Renyi!\n");
    let refused = run(&narrow);
    assert_eq!(refused.status.code(), Some(1));
    assert_eq!(
        text(&refused.stderr),
        "renyi: the sandbox does not grant `environment`, which `main` needs\n"
    );
    let bad = run(&broken);
    assert_eq!(bad.status.code(), Some(1));
    assert!(
        text(&bad.stderr).contains("`teleport`, which is not a capability"),
        "{}",
        text(&bad.stderr)
    );
    let test = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(["test", "--sandbox", &wide, "examples/hello.ry"])
        .output()
        .expect("the renyi binary runs");
    assert_eq!(test.status.code(), Some(1));
    assert!(
        text(&test.stderr).contains("takes only"),
        "{}",
        text(&test.stderr)
    );
}
