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
//! the random numbers, the environment, the screen), and a program that
//! needs more is refused with the capability named. One import, the
//! clock (`renyi_host_now_ms`), comes from the page.
//!
//! The page is the host of `std.screen` (decision AW33, stage 33d). A
//! program that presents a frame suspends inside the call
//! (`Interrupt::Suspended`, decision AW31): the run answers `SUSPENDED`
//! with what the program printed so far, the frame is read as text, one
//! shape per line (`renyi_frame`, `renyi_frame_len`), the page draws it
//! and resumes the run with the keys held and the milliseconds elapsed
//! (`renyi_resume`), which answers as `renyi_run` does, the next frame
//! included. The suspended run, its program and its VM, lives in the
//! session until it ends or the next run or `renyi_clear` abandons it.
//! A strip and a picture name an image (decision AW35), which the page
//! resolves to the files served with the program; the sounds the run
//! asked for during a call come with its answer, one name per line
//! (`renyi_sounds`, `renyi_sounds_len`), for the page to play.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::mem::ManuallyDrop;
use std::rc::Rc;

use renyi_check::{check_parsed_project_in, Resolved};
use renyi_syntax::diagnostics::render_text;
use renyi_syntax::{Diagnostic, SourceFile};
use renyi_vm::grant::parse_capability;
use renyi_vm::screen::{Answer, Input, Screen, Shape};
use renyi_vm::value::Value;
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
/// The run is suspended at `screen.present` with a frame to draw
/// (`renyi_frame`); the text is what the program wrote during the call.
pub const SUSPENDED: i32 = 6;

/// What the page gave, what the last call answered, and the run the
/// last call left suspended.
struct Session {
    files: BTreeMap<String, String>,
    arguments: Vec<String>,
    text: Vec<u8>,
    exit_code: i32,
    live: Option<Live>,
    /// The frame of the suspended run as text, one shape per line.
    frame: Vec<u8>,
    /// The sounds the last call played, one name per line.
    sounds: Vec<u8>,
}

impl Session {
    /// A run's or a resumption's answer kept for the page: the text,
    /// the exit code, the sounds, the frame and the run when it is
    /// suspended; the status.
    fn answer(&mut self, outcome: Outcome, live: Option<Live>) -> i32 {
        self.text = outcome.text.into_bytes();
        self.exit_code = outcome.exit_code;
        self.sounds = outcome
            .sounds
            .iter()
            .map(|sound| format!("{}\n", one_line(sound)))
            .collect::<String>()
            .into_bytes();
        self.frame = live
            .as_ref()
            .map(Live::frame)
            .unwrap_or_default()
            .into_bytes();
        self.live = live;
        outcome.status
    }
}

thread_local! {
    static SESSION: RefCell<Session> = const {
        RefCell::new(Session {
            files: BTreeMap::new(),
            arguments: Vec::new(),
            text: Vec::new(),
            exit_code: 0,
            live: None,
            frame: Vec::new(),
            sounds: Vec::new(),
        })
    };
}

/// The capabilities the page serves: the console (its log), the clock,
/// the random numbers, the environment (which answers nothing) and the
/// screen (the page itself, decision AW33).
fn grant() -> Vec<renyi_check::effects::Capability> {
    ["console", "time", "random", "environment", "screen"]
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

impl Log {
    /// What was written since the previous call, taken out.
    fn drain(&self) -> String {
        let bytes = std::mem::take(&mut *self.0.borrow_mut());
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

/// The page as the host of `std.screen` (decision AW33): it keeps the
/// frame and answers `Later`, so that the run suspends inside `present`
/// and the page draws the frame before it resumes the run with the
/// input; it keeps the sounds asked for since the previous answer, for
/// the page to play with the next (decision AW35).
struct PageScreen {
    frame: Vec<Shape>,
    sounds: Vec<String>,
}

impl Screen for PageScreen {
    fn present(&mut self, frame: Vec<Shape>) -> Answer {
        self.frame = frame;
        Answer::Later
    }

    fn suspends(&self) -> bool {
        true
    }

    fn play(&mut self, sound: &str) {
        self.sounds.push(sound.to_string());
    }
}

/// A run suspended at `screen.present`: its VM with the frames kept,
/// the program the VM borrows, the log it writes into and the host that
/// holds its frame. The VM is dropped before the program, by `Drop`.
pub struct Live {
    vm: ManuallyDrop<Vm<'static>>,
    /// The program, from `Box::into_raw`: held by a raw pointer so that
    /// no box asserts a unique hold on it while the VM borrows it.
    program: *mut Program,
    log: Log,
    screen: Rc<RefCell<PageScreen>>,
}

impl Drop for Live {
    fn drop(&mut self) {
        // SAFETY: the VM is dropped once, here, before the program it
        // borrows; the program was made by `Box::into_raw` in `run` and
        // is freed once, here, after the VM.
        unsafe {
            ManuallyDrop::drop(&mut self.vm);
            drop(Box::from_raw(self.program));
        }
    }
}

impl Live {
    /// The frame the run presented, as text, one shape per line:
    /// `column <x> <top> <bottom> <red> <green> <blue>`,
    /// `rectangle <x> <y> <width> <height> <red> <green> <blue>`,
    /// `label <x> <y> <red> <green> <blue> <text>`,
    /// `strip <x> <top> <bottom> <column> <shade> <image>` or
    /// `picture <x> <y> <width> <height> <shade> <image>`, the text and
    /// the image's name to the end of the line with a line break in
    /// them made a space.
    pub fn frame(&self) -> String {
        frame_text(&self.screen.borrow().frame)
    }
}

/// What a run or a resumption answers: the status, what the program
/// wrote during the call followed by how it ended, the exit code and
/// the sounds the call asked for, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub status: i32,
    pub text: String,
    pub exit_code: i32,
    pub sounds: Vec<String>,
}

impl Outcome {
    fn new(status: i32, text: impl Into<String>, exit_code: i32) -> Outcome {
        Outcome {
            status,
            text: text.into(),
            exit_code,
            sounds: Vec::new(),
        }
    }
}

/// The run of the program whose main file is `name`, to its end or to
/// its first frame: the outcome and, when the run is suspended, the run
/// itself for `resume`.
pub fn run(
    files: &BTreeMap<String, String>,
    name: &str,
    arguments: Vec<String>,
) -> (Outcome, Option<Live>) {
    let (program, warnings) = match compile(files, name) {
        Ok(compiled) => compiled,
        Err(diagnostics) => return (Outcome::new(REJECTED, diagnostics, 0), None),
    };
    // the last reads of the slots as moves (decision AU17)
    let program = renyi_vm::liveness::prepared(&program).into_owned();
    let Some(main) = program.main else {
        return (
            Outcome::new(CRASHED, "crashed: the program has no `main` function\n", 0),
            None,
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
            Outcome::new(
                REFUSED,
                format!(
                    "`main` needs `{}`, which the page cannot serve\n",
                    need.spelling()
                ),
                0,
            ),
            None,
        );
    }
    let log = Log(Rc::new(RefCell::new(warnings.into_bytes())));
    let screen = Rc::new(RefCell::new(PageScreen {
        frame: Vec::new(),
        sounds: Vec::new(),
    }));
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
        screen: Some(screen.clone()),
        ..Options::default()
    };
    // the program outlives this call: the VM borrows it until `Live`
    // drops both, the VM first
    let program = Box::into_raw(Box::new(program));
    // SAFETY: the program lives at this address until `Live::drop` frees
    // it, after the VM that borrows it.
    let borrowed: &'static Program = unsafe { &*program };
    let mut live = Live {
        vm: ManuallyDrop::new(Vm::new(borrowed, options)),
        program,
        log,
        screen,
    };
    if let Err(message) = live.vm.begin_run(&needs, &module) {
        return (Outcome::new(REFUSED, format!("{message}\n"), 0), None);
    }
    let result = live.vm.run_main();
    settle(live, result)
}

/// The suspended run continued with the host's input, to its end or to
/// its next frame.
pub fn resume(mut live: Live, input: Input) -> (Outcome, Option<Live>) {
    let result = live.vm.resume(input);
    settle(live, result)
}

/// How a run or a resumption ended, with what the program wrote and the
/// sounds it asked for during the call; the run itself when it is
/// suspended, else dropped.
fn settle(mut live: Live, result: Result<Option<Value>, Interrupt>) -> (Outcome, Option<Live>) {
    let (status, ending, exit_code) = match result {
        Ok(None) => (FINISHED, String::new(), 0),
        Ok(Some(error)) => {
            let text = live
                .vm
                .text_for_console(&error, "the failure of `main`")
                .unwrap_or_else(|_| live.vm.describe(&error));
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
        Err(Interrupt::Suspended) => (SUSPENDED, String::new(), 0),
        Err(Interrupt::OverMemory { limit, used }) => (
            CRASHED,
            format!("crashed: exceeded the memory budget of {limit} bytes ({used} bytes held)\n"),
            0,
        ),
    };
    let mut text = live.log.drain();
    text.push_str(&ending);
    let sounds = std::mem::take(&mut live.screen.borrow_mut().sounds);
    let outcome = Outcome {
        status,
        text,
        exit_code,
        sounds,
    };
    (outcome, (status == SUSPENDED).then_some(live))
}

/// A frame as text, one shape per line (`Live::frame`).
fn frame_text(frame: &[Shape]) -> String {
    let mut text = String::new();
    for shape in frame {
        let line = match shape {
            Shape::Column {
                screen_x,
                top,
                bottom,
                colour,
            } => format!(
                "column {screen_x} {top} {bottom} {} {} {}",
                colour.red, colour.green, colour.blue
            ),
            Shape::Rectangle {
                screen_x,
                screen_y,
                width,
                height,
                colour,
            } => format!(
                "rectangle {screen_x} {screen_y} {width} {height} {} {} {}",
                colour.red, colour.green, colour.blue
            ),
            Shape::Label {
                screen_x,
                screen_y,
                text: label,
                colour,
            } => format!(
                "label {screen_x} {screen_y} {} {} {} {}",
                colour.red,
                colour.green,
                colour.blue,
                one_line(label)
            ),
            Shape::Strip {
                screen_x,
                top,
                bottom,
                image,
                column,
                shade,
            } => format!(
                "strip {screen_x} {top} {bottom} {column} {shade} {}",
                one_line(image)
            ),
            Shape::Picture {
                screen_x,
                screen_y,
                width,
                height,
                image,
                shade,
            } => format!(
                "picture {screen_x} {screen_y} {width} {height} {shade} {}",
                one_line(image)
            ),
        };
        text.push_str(&line);
        text.push('\n');
    }
    text
}

/// The text on one line: a line break in it made a space.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|letter| {
            if letter == '\n' || letter == '\r' {
                ' '
            } else {
                letter
            }
        })
        .collect()
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

/// Every file forgotten, and a run suspended at a frame abandoned with
/// its frame and its sounds.
#[no_mangle]
pub extern "C" fn renyi_clear() {
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.files.clear();
        session.live = None;
        session.frame.clear();
        session.sounds.clear();
    });
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
/// given since the last run, to its end or to its first frame: what it
/// wrote and how it ended in the text, the frame in `renyi_frame`. A run
/// still suspended is abandoned first.
///
/// # Safety
/// The name lies at the pointer for its length.
#[no_mangle]
pub unsafe extern "C" fn renyi_run(name: *const u8, len: usize) -> i32 {
    let name = text_at(name, len);
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        session.live = None;
        let arguments = std::mem::take(&mut session.arguments);
        let (outcome, live) = run(&session.files, &name, arguments);
        session.answer(outcome, live)
    })
}

/// The suspended run continued with the keys held (0 or 1 each) and the
/// milliseconds since the previous frame, to its end or to its next
/// frame; the answer as `renyi_run`'s. With no run suspended, a crash
/// that says so.
#[no_mangle]
pub extern "C" fn renyi_resume(
    forward: i32,
    backward: i32,
    turn_left: i32,
    turn_right: i32,
    fire: i32,
    quit: i32,
    elapsed_ms: i32,
) -> i32 {
    let input = Input {
        forward: forward != 0,
        backward: backward != 0,
        turn_left: turn_left != 0,
        turn_right: turn_right != 0,
        fire: fire != 0,
        quit: quit != 0,
        elapsed_ms: i64::from(elapsed_ms),
    };
    SESSION.with(|session| {
        let mut session = session.borrow_mut();
        let Some(live) = session.live.take() else {
            let outcome = Outcome::new(
                CRASHED,
                "crashed: no run is suspended at `screen.present`\n",
                0,
            );
            return session.answer(outcome, None);
        };
        let (outcome, live) = resume(live, input);
        session.answer(outcome, live)
    })
}

/// The frame of the run the last call left suspended, as text
/// (`Live::frame`): its bytes, valid until the next call; empty when no
/// run is suspended.
#[no_mangle]
pub extern "C" fn renyi_frame() -> *const u8 {
    SESSION.with(|session| session.borrow().frame.as_ptr())
}

#[no_mangle]
pub extern "C" fn renyi_frame_len() -> usize {
    SESSION.with(|session| session.borrow().frame.len())
}

/// The sounds the last run or resumption asked for, as text, one name
/// per line in the order asked: its bytes, valid until the next call;
/// empty when the call played nothing.
#[no_mangle]
pub extern "C" fn renyi_sounds() -> *const u8 {
    SESSION.with(|session| session.borrow().sounds.as_ptr())
}

#[no_mangle]
pub extern "C" fn renyi_sounds_len() -> usize {
    SESSION.with(|session| session.borrow().sounds.len())
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

    /// A frame a turn: a column at the turn and a label, the turn
    /// advanced by the elapsed time, `fire` printed, `quit` obeyed.
    const FRAMES: &str = "module frames
  purpose: Present a frame a turn until the host asks to stop or three are drawn.

import std.console
import std.screen exposing Colour, Shape

public function main() needs console, screen
  purpose: Print the turn, present its frame, then act on the input.

  let blue be Colour(red: 0, green: 0, blue: 255)
  let white be Colour(red: 255, green: 255, blue: 255)
  let mutable turn be 0
  repeat until turn is at least 3
    console.print(\"frame {turn}\")
    let input be screen.present([
      Column(screen_x: turn, top: 0, bottom: 9, colour: blue),
      Label(screen_x: 4, screen_y: 2, text: \"turn {turn}\", colour: white)
    ])
    if input.fire then console.print(\"fired\") end
    if input.quit then break end
    change turn to turn + input.elapsed_ms
  end
  console.print(\"drawn {turn}\")
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
        let (outcome, live) = run(&files, "hello.ry", Vec::new());
        assert_eq!(
            outcome,
            Outcome::new(FINISHED, "Hello from the browser\n", 0)
        );
        assert!(live.is_none());
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
        assert_eq!(
            run(&files, "main.ry", Vec::new()).0,
            Outcome::new(FINISHED, "hello, world\n", 0)
        );
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
        assert_eq!(run(&files, "hello.ry", Vec::new()).0.status, REJECTED);
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
        let (outcome, _) = run(&files, "reader.ry", Vec::new());
        assert_eq!(outcome.status, REFUSED, "{}", outcome.text);
        assert!(outcome.text.contains("filesystem"), "{}", outcome.text);
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
        let (outcome, _) = run(&files(&[("failing.ry", failing)]), "failing.ry", Vec::new());
        assert_eq!(outcome.status, FAILED, "{}", outcome.text);
        assert!(outcome.text.starts_with("before\n"), "{}", outcome.text);
        let exiting = "module exiting
  purpose: Exit with a code.

import std.environment

public function main() needs environment
  purpose: Exit with 7.

  environment.exit(7)
end
";
        let (outcome, _) = run(&files(&[("exiting.ry", exiting)]), "exiting.ry", Vec::new());
        assert_eq!((outcome.status, outcome.exit_code), (EXITED, 7));
    }

    #[test]
    fn a_frame_loop_suspends_at_every_frame_and_the_page_resumes_it() {
        let files = files(&[("frames.ry", FRAMES)]);
        let (outcome, live) = run(&files, "frames.ry", Vec::new());
        assert_eq!(outcome, Outcome::new(SUSPENDED, "frame 0\n", 0));
        let live = live.expect("the run is suspended at its first frame");
        assert_eq!(
            live.frame(),
            "column 0 0 9 0 0 255\nlabel 4 2 255 255 255 turn 0\n"
        );
        // the input reaches the program: `fire` printed, the turn advanced
        // by the elapsed time; the text is what the call wrote
        let input = Input {
            fire: true,
            elapsed_ms: 1,
            ..Input::default()
        };
        let (outcome, live) = resume(live, input);
        assert_eq!(outcome, Outcome::new(SUSPENDED, "fired\nframe 1\n", 0));
        let live = live.expect("the run is suspended at its second frame");
        assert!(
            live.frame().starts_with("column 1 0 9 0 0 255\n"),
            "{}",
            live.frame()
        );
        let input = Input {
            quit: true,
            ..Input::default()
        };
        let (outcome, live) = resume(live, input);
        assert_eq!(outcome, Outcome::new(FINISHED, "drawn 1\n", 0));
        assert!(live.is_none());
    }

    #[test]
    fn a_label_is_one_line_of_the_frame_whatever_its_text() {
        use renyi_vm::screen::Colour;

        let frame = [
            Shape::Strip {
                screen_x: 3,
                top: 10,
                bottom: 90,
                image: "wall\n2".to_string(),
                column: 70,
                shade: 60,
            },
            Shape::Picture {
                screen_x: 0,
                screen_y: 168,
                width: 320,
                height: 32,
                image: "bar".to_string(),
                shade: 100,
            },
            Shape::Label {
                screen_x: 1,
                screen_y: 2,
                text: "two\nlines\r".to_string(),
                colour: Colour {
                    red: 1,
                    green: 2,
                    blue: 3,
                },
            },
            Shape::Rectangle {
                screen_x: 0,
                screen_y: 0,
                width: 320,
                height: 200,
                colour: Colour {
                    red: 0,
                    green: 0,
                    blue: 0,
                },
            },
        ];
        assert_eq!(
            frame_text(&frame),
            "strip 3 10 90 70 60 wall 2\npicture 0 168 320 32 100 bar\n\
             label 1 2 1 2 3 two lines \nrectangle 0 0 320 200 0 0 0\n"
        );
    }

    /// A sound before the first frame and one after it.
    const SOUNDS: &str = "module sounds
  purpose: Play a sound, present a frame of a strip, play another.

import std.console
import std.screen exposing Shape

public function main() needs console, screen
  purpose: Play, present, play, print the elapsed time.

  screen.play(\"shot\")
  let input be screen.present([
    Strip(screen_x: 3, top: 10, bottom: 90, image: \"wall\", column: 70, shade: 60)
  ])
  screen.play(\"door\")
  console.print(\"elapsed {input.elapsed_ms}\")
end
";

    /// The last answer's sounds, as the page reads them.
    fn sounds() -> String {
        // SAFETY: as for `text`, by `renyi_sounds`'s promise
        unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(
                renyi_sounds(),
                renyi_sounds_len(),
            ))
            .into_owned()
        }
    }

    #[test]
    fn the_sounds_a_call_asked_for_come_with_its_answer() {
        let files = files(&[("sounds.ry", SOUNDS)]);
        let (outcome, live) = run(&files, "sounds.ry", Vec::new());
        assert_eq!(outcome.status, SUSPENDED, "{}", outcome.text);
        assert_eq!(outcome.sounds, ["shot"]);
        let live = live.expect("the run is suspended at its frame");
        assert_eq!(live.frame(), "strip 3 10 90 70 60 wall\n");
        let input = Input {
            elapsed_ms: 7,
            ..Input::default()
        };
        let (outcome, live) = resume(live, input);
        assert_eq!(
            outcome,
            Outcome {
                sounds: vec!["door".to_string()],
                ..Outcome::new(FINISHED, "elapsed 7\n", 0)
            }
        );
        assert!(live.is_none());
        // through the C ABI: one name per line, until the next call
        let name = "sounds.ry";
        // SAFETY: the texts lie at their pointers for their lengths, as
        // the functions ask
        unsafe {
            renyi_clear();
            renyi_file(name.as_ptr(), name.len(), SOUNDS.as_ptr(), SOUNDS.len());
            assert_eq!(renyi_run(name.as_ptr(), name.len()), SUSPENDED);
            assert_eq!(sounds(), "shot\n");
            assert_eq!(renyi_resume(0, 0, 0, 0, 0, 0, 7), FINISHED);
            assert_eq!(
                (text(), sounds()),
                ("elapsed 7\n".to_string(), "door\n".to_string())
            );
            renyi_clear();
            assert_eq!(renyi_sounds_len(), 0);
        }
    }

    /// The last answer's text, as the page reads it.
    fn text() -> String {
        // SAFETY: the bytes lie at the pointer for the length until the
        // next call, as `renyi_text` promises
        unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(renyi_text(), renyi_text_len()))
                .into_owned()
        }
    }

    /// The last answer's frame, as the page reads it.
    fn frame() -> String {
        // SAFETY: as for `text`, by `renyi_frame`'s promise
        unsafe {
            String::from_utf8_lossy(std::slice::from_raw_parts(renyi_frame(), renyi_frame_len()))
                .into_owned()
        }
    }

    #[test]
    fn the_c_abi_drives_a_suspended_run() {
        let name = "frames.ry";
        // SAFETY: the texts lie at their pointers for their lengths, as
        // the functions ask
        unsafe {
            renyi_clear();
            renyi_file(name.as_ptr(), name.len(), FRAMES.as_ptr(), FRAMES.len());
            assert_eq!(renyi_resume(0, 0, 0, 0, 0, 0, 16), CRASHED);
            assert!(text().contains("no run is suspended"), "{}", text());
            assert_eq!(renyi_run(name.as_ptr(), name.len()), SUSPENDED);
            assert_eq!(text(), "frame 0\n");
            assert_eq!(
                frame(),
                "column 0 0 9 0 0 255\nlabel 4 2 255 255 255 turn 0\n"
            );
            assert_eq!(renyi_resume(0, 0, 0, 0, 0, 1, 16), FINISHED);
            assert_eq!((text(), frame()), ("drawn 0\n".to_string(), String::new()));
            // a run still suspended is abandoned by the next run and by
            // `renyi_clear`
            assert_eq!(renyi_run(name.as_ptr(), name.len()), SUSPENDED);
            assert_eq!(renyi_run(name.as_ptr(), name.len()), SUSPENDED);
            renyi_clear();
            assert_eq!(renyi_frame_len(), 0);
            assert_eq!(renyi_resume(0, 0, 0, 0, 0, 1, 16), CRASHED);
        }
    }
}
