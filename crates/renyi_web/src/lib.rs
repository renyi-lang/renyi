//! The toolchain for the browser (decision AW29, stage 33a): the front
//! end, the checker and the interpreter of `renyi_vm` without its JIT,
//! its FFI, its HTTP client and its SQLite, built for
//! `wasm32-unknown-unknown` as a module with a C ABI of a few plain
//! functions, which the page's script calls (`docs/assets/play.js`;
//! `tools/wasm_check.js` drives the same module under node).
//!
//! The page gives the files of a program as texts (`renyi_file`, the
//! main file and the files it imports, by the names its imports resolve
//! to), then asks for a check (`renyi_check`) or a run (`renyi_run`),
//! and reads the answer's text (`renyi_text`, `renyi_text_len`): the
//! diagnostics as `renyi check` renders them, or what the program
//! printed with how it ended. The files stand in for the file system
//! through `renyi_package::reads::provide`, so the resolver, the checker
//! and the emitter run as they do in the binary. A run is interpreted,
//! under a grant of what the page can serve (the console, the clock,
//! the random numbers, the environment), and a program that needs more
//! is refused with the capability named. One import, the clock
//! (`renyi_host_now_ms`), comes from the page.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::rc::Rc;

use renyi_check::{check_parsed_project_in, Resolved};
use renyi_syntax::diagnostics::render_text;
use renyi_syntax::{Diagnostic, SourceFile};
use renyi_vm::grant::parse_capability;
use renyi_vm::{compile_project, Interrupt, Narrowing, Options, Program, Registry, Vm};

/// The statuses a check or a run answers.
pub const FINISHED: i32 = 0;
/// `main` failed; the text ends with the failure.
pub const FAILED: i32 = 1;
/// The run crashed; the text ends with the message and the location.
pub const CRASHED: i32 = 2;
/// `environment.exit`; the code through `renyi_exit_code`.
pub const EXITED: i32 = 3;
/// The program does not check; the text is the diagnostics.
pub const REJECTED: i32 = 4;
/// `main` needs a capability the page does not serve; the text names it.
pub const REFUSED: i32 = 5;

/// What the page gave and what the last call answered.
struct Session {
    files: BTreeMap<String, String>,
    arguments: Vec<String>,
    text: Vec<u8>,
    exit_code: i32,
}

thread_local! {
    static SESSION: RefCell<Session> = const {
        RefCell::new(Session {
            files: BTreeMap::new(),
            arguments: Vec::new(),
            text: Vec::new(),
            exit_code: 0,
        })
    };
}

/// The capabilities the page serves: the console (its log), the clock,
/// the random numbers and the environment (which answers nothing).
fn grant() -> Vec<renyi_check::effects::Capability> {
    ["console", "time", "random", "environment"]
        .iter()
        .map(|name| parse_capability(name).expect("a capability of the tree"))
        .collect()
}

/// The program compiled from the files, with the warnings rendered, or
/// the diagnostics that stop it.
pub fn compile(files: &BTreeMap<String, String>, name: &str) -> Result<(Program, String), String> {
    let Some(text) = files.get(name) else {
        return Err(format!("no file named `{name}` was given\n"));
    };
    renyi_package::reads::provide(Some(files.clone()));
    let file = SourceFile::new(name, text);
    let Resolved {
        files: sources,
        problems,
        trees,
    } = renyi_check::resolve(&file);
    let library = Registry::standard().library();
    let checked = check_parsed_project_in(&library, &sources, trees, &problems);
    renyi_package::reads::provide(None);
    let mut failed = false;
    let mut diagnostics = String::new();
    for module in &checked.modules {
        if module.diagnostics.is_empty() {
            continue;
        }
        failed |= module.diagnostics.iter().any(Diagnostic::is_error);
        diagnostics.push_str(&render_text(&sources[module.file], &module.diagnostics));
    }
    if failed {
        return Err(diagnostics);
    }
    Ok((compile_project(&checked, &sources), diagnostics))
}

/// The check of the program whose main file is `name`: the status and
/// the diagnostics.
pub fn check(files: &BTreeMap<String, String>, name: &str) -> (i32, String) {
    match compile(files, name) {
        Ok((_, warnings)) => (FINISHED, warnings),
        Err(diagnostics) => (REJECTED, diagnostics),
    }
}

/// Everything a run writes, the standard output and the standard error
/// in the order written.
#[derive(Clone)]
struct Log(Rc<RefCell<Vec<u8>>>);

impl Write for Log {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The run of the program whose main file is `name`: the status, what
/// the program wrote followed by how it ended, and the exit code.
pub fn run(
    files: &BTreeMap<String, String>,
    name: &str,
    arguments: Vec<String>,
) -> (i32, String, i32) {
    let (program, warnings) = match compile(files, name) {
        Ok(compiled) => compiled,
        Err(diagnostics) => return (REJECTED, diagnostics, 0),
    };
    // the last reads of the slots as moves (decision AU17)
    let program = renyi_vm::liveness::prepared(&program).into_owned();
    let Some(main) = program.main else {
        return (
            CRASHED,
            "crashed: the program has no `main` function\n".to_string(),
            0,
        );
    };
    let meta = &program.function_metas[main];
    let (needs, module) = (meta.needs.clone(), meta.module.clone());
    // what the page cannot serve is refused before the run, with the
    // capability named, as the sandbox of decision AP1 refuses a call
    let granted = grant();
    if let Some(need) = needs
        .iter()
        .find(|need| !renyi_check::effects::covered(&granted, need, true))
    {
        return (
            REFUSED,
            format!(
                "`main` needs `{}`, which the page cannot serve\n",
                need.spelling()
            ),
            0,
        );
    }
    let log = Log(Rc::new(RefCell::new(warnings.into_bytes())));
    let options = Options {
        arguments,
        stdout: Box::new(log.clone()),
        stderr: Box::new(log.clone()),
        stdin: Box::new(std::io::empty()),
        narrowing: Narrowing {
            sandbox: Some(grant()),
            ..Narrowing::default()
        },
        // the browser has no machine code (decision AW29)
        interpret: true,
        registry: Registry::standard(),
        ..Options::default()
    };
    let mut vm = Vm::new(&program, options);
    if let Err(message) = vm.begin_run(&needs, &module) {
        return (REFUSED, format!("{message}\n"), 0);
    }
    let (status, ending, exit_code) = match vm.run_main() {
        Ok(None) => (FINISHED, String::new(), 0),
        Ok(Some(error)) => {
            let text = vm
                .text_for_console(&error, "the failure of `main`")
                .unwrap_or_else(|_| vm.describe(&error));
            (FAILED, format!("main failed with {text}\n"), 0)
        }
        Err(Interrupt::Crash { message, location }) => match location {
            Some(location) => (CRASHED, format!("crashed: {message} at {location}\n"), 0),
            None => (CRASHED, format!("crashed: {message}\n"), 0),
        },
        Err(Interrupt::Exit(code)) => (EXITED, String::new(), code),
        Err(Interrupt::Reload) => (
            CRASHED,
            "crashed: stopped for a new version\n".to_string(),
            0,
        ),
        Err(Interrupt::Suspended) => (
            CRASHED,
            "crashed: suspended at `screen.present`, which this page does not resume yet\n"
                .to_string(),
            0,
        ),
        Err(Interrupt::OverMemory { limit, used }) => (
            CRASHED,
            format!("crashed: exceeded the memory budget of {limit} bytes ({used} bytes held)\n"),
            0,
        ),
    };
    drop(vm);
    let mut text = String::from_utf8_lossy(&log.0.borrow()).into_owned();
    text.push_str(&ending);
    (status, text, exit_code)
}

// ------------------------------------------------------------ the C ABI

/// A buffer of `len` bytes for the page to write into, freed by
/// `renyi_free` with the same length.
#[no_mangle]
pub extern "C" fn renyi_alloc(len: usize) -> *mut u8 {
    let mut bytes: Vec<u8> = Vec::with_capacity(len.max(1));
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer
}

/// # Safety
/// `pointer` and `len` are what `renyi_alloc` answered and was asked.
#[no_mangle]
pub unsafe extern "C" fn renyi_free(pointer: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(pointer, 0, len.max(1)));
}

/// The text at a pointer the page wrote, copied out.
unsafe fn text_at(pointer: *const u8, len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(pointer, len)).into_owned()
}

/// A file of the program: added, or replaced when the name is known.
///
/// # Safety
/// The two texts lie at the pointers for their lengths.
#[no_mangle]
pub unsafe extern "C" fn renyi_file(
    name: *const u8,
    name_len: usize,
    text: *const u8,
    text_len: usize,
) {
    let (name, text) = (text_at(name, name_len), text_at(text, text_len));
    SESSION.with(|session| {
        session.borrow_mut().files.insert(name, text);
    });
}

/// Every file forgotten.
#[no_mangle]
pub extern "C" fn renyi_clear() {
    SESSION.with(|session| session.borrow_mut().files.clear());
}

/// An argument for the next run (`environment.arguments`).
///
/// # Safety
/// The text lies at the pointer for its length.
#[no_mangle]
pub unsafe extern "C" fn renyi_argument(text: *const u8, len: usize) {
    let argument = text_at(text, len);
    SESSION.with(|session| session.borrow_mut().arguments.push(argument));
}

/// The check of the program whose main file is named; the diagnostics in
/// the text.
///
/// # Safety
/// The name lies at the pointer for its length.
#[no_mangle]
pub unsafe extern "C" fn renyi_check(name: *const u8, len: usize) -> i32 {
    let name = text_at(name, len);
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        let (status, text) = check(&session.files, &name);
        session.text = text.into_bytes();
        status
    })
}

/// The run of the program whose main file is named, with the arguments
/// given since the last run; what it wrote and how it ended in the text.
///
/// # Safety
/// The name lies at the pointer for its length.
#[no_mangle]
pub unsafe extern "C" fn renyi_run(name: *const u8, len: usize) -> i32 {
    let name = text_at(name, len);
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        let arguments = std::mem::take(&mut session.arguments);
        let (status, text, exit_code) = run(&session.files, &name, arguments);
        session.text = text.into_bytes();
        session.exit_code = exit_code;
        status
    })
}

/// The last answer's text: its bytes, valid until the next call.
#[no_mangle]
pub extern "C" fn renyi_text() -> *const u8 {
    SESSION.with(|session| session.borrow().text.as_ptr())
}

#[no_mangle]
pub extern "C" fn renyi_text_len() -> usize {
    SESSION.with(|session| session.borrow().text.len())
}

/// The code of the last run that exited.
#[no_mangle]
pub extern "C" fn renyi_exit_code() -> i32 {
    SESSION.with(|session| session.borrow().exit_code)
}

/// The toolchain's version, in the text.
#[no_mangle]
pub extern "C" fn renyi_version() {
    SESSION.with(|session| {
        session.borrow_mut().text = format!("renyi {}", env!("CARGO_PKG_VERSION")).into_bytes();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO: &str = "module hello
  purpose: Say hello.

import std.console

public function main() needs console
  purpose: Print a greeting and the arguments.

  console.print(\"Hello from the browser\")
end
";

    fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect()
    }

    #[test]
    fn a_program_is_checked_and_run_from_its_texts() {
        let files = files(&[("hello.ry", HELLO)]);
        assert_eq!(check(&files, "hello.ry"), (FINISHED, String::new()));
        let (status, text, code) = run(&files, "hello.ry", Vec::new());
        assert_eq!((status, code), (FINISHED, 0));
        assert_eq!(text, "Hello from the browser\n");
    }

    #[test]
    fn an_import_resolves_among_the_files_given() {
        let files = files(&[
            (
                "main.ry",
                "module main
  purpose: Greet through another module.

import greeting
import std.console

public function main() needs console
  purpose: Print the greeting.

  console.print(greeting.text(\"world\"))
end
",
            ),
            (
                "greeting.ry",
                "module greeting
  purpose: A greeting.

public function text(name: Text) returns Text
  purpose: Hello to the name.
  example: text(\"you\") is \"hello, you\"

  return \"hello, {name}\"
end
",
            ),
        ]);
        assert_eq!(check(&files, "main.ry"), (FINISHED, String::new()));
        let (status, text, _) = run(&files, "main.ry", Vec::new());
        assert_eq!((status, text.as_str()), (FINISHED, "hello, world\n"));
    }

    #[test]
    fn a_program_that_does_not_check_is_rejected_with_its_diagnostics() {
        let files = files(&[(
            "hello.ry",
            &HELLO.replace("console.print(", "console.prin("),
        )]);
        let (status, text) = check(&files, "hello.ry");
        assert_eq!(status, REJECTED);
        assert!(text.contains("prin"), "{text}");
        assert_eq!(run(&files, "hello.ry", Vec::new()).0, REJECTED);
        assert_eq!(check(&files, "other.ry").0, REJECTED);
    }

    #[test]
    fn a_program_that_needs_what_the_page_cannot_serve_is_refused() {
        let files = files(&[(
            "reader.ry",
            "module reader
  purpose: Read a file.

import std.console
import std.filesystem exposing Path

public function main() needs console, filesystem.read
  purpose: Print a file.

  let text be filesystem.read_text(Path(\"notes.txt\")) otherwise \"\"
  console.print(text)
end
",
        )]);
        assert_eq!(check(&files, "reader.ry").0, FINISHED);
        let (status, text, _) = run(&files, "reader.ry", Vec::new());
        assert_eq!(status, REFUSED, "{text}");
        assert!(text.contains("filesystem"), "{text}");
    }

    #[test]
    fn a_failure_a_crash_and_an_exit_are_told_apart() {
        let failing = "module failing
  purpose: Fail on purpose.

import std.console

public type Oops
  purpose: An error.
  has reason: Text
  can ToText
end

public function main() or fails with Oops needs console
  purpose: Print, then fail.

  console.print(\"before\")
  fail with Oops(reason: \"on purpose\")
end
";
        let (status, text, _) = run(&files(&[("failing.ry", failing)]), "failing.ry", Vec::new());
        assert_eq!(status, FAILED, "{text}");
        assert!(text.starts_with("before\n"), "{text}");
        let exiting = "module exiting
  purpose: Exit with a code.

import std.environment

public function main() needs environment
  purpose: Exit with 7.

  environment.exit(7)
end
";
        let (status, _, code) = run(&files(&[("exiting.ry", exiting)]), "exiting.ry", Vec::new());
        assert_eq!((status, code), (EXITED, 7));
    }
}
