//! `std.screen` and the suspension (decision AW31): a host that answers
//! at once drives a frame loop; one that answers later suspends the run
//! inside `screen.present`, and `resume` continues it with the input and
//! notes the call in the recording, which a replay answers from; a run
//! without a host crashes at the call, as does a suspension inside a
//! callback, which runs on a nested loop.

use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::rc::Rc;
use std::time::{Duration, Instant};

use renyi_syntax::SourceFile;
use renyi_vm::natives::json::Json;
use renyi_vm::recording::Outcome;
use renyi_vm::screen::{Answer, Colour, Input, Screen, Shape, SharedScreen};
use renyi_vm::{
    compile_project, run_main, run_program, Interrupt, Options, Program, Recording, RunOutcome, Vm,
};

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

impl Capture {
    fn text(&self) -> String {
        String::from_utf8(self.0.borrow().clone()).expect("utf-8")
    }
}

/// A host for the tests: it keeps every frame it is shown and answers
/// at once, `quit` once `quit_after` frames went by, or later when
/// `later`.
#[derive(Default)]
struct Host {
    frames: Vec<Vec<Shape>>,
    later: bool,
    quit_after: usize,
}

impl Screen for Host {
    fn present(&mut self, frame: Vec<Shape>) -> Answer {
        self.frames.push(frame);
        if self.later {
            return Answer::Later;
        }
        Answer::Now(Input {
            quit: self.frames.len() > self.quit_after,
            elapsed_ms: 16,
            ..Input::default()
        })
    }

    fn suspends(&self) -> bool {
        self.later
    }
}

/// A host a replay must never ask.
struct Untouched;

impl Screen for Untouched {
    fn present(&mut self, _: Vec<Shape>) -> Answer {
        panic!("a replay answers `present` from the recording; the host was asked");
    }
}

fn options(stdout: &Capture, screen: Option<SharedScreen>) -> Options {
    Options {
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(Capture::default()),
        stdin: Box::new(std::io::empty()),
        screen,
        ..Options::default()
    }
}

const GAME: &str = "module game
  purpose: A frame loop of three turns that stops when the host asks.

import std.console
import std.screen exposing Colour, Shape

public function main() needs screen, console
  purpose: Present a frame a turn, print the input, stop when the host asks.

  let mutable turn be 0
  repeat until turn is at least 3
    let input be screen.present(frame_for(turn))
    if input.quit then break end
    console.print(\"frame {turn}: elapsed {input.elapsed_ms}\")
    change turn to turn + 1
  end
  console.print(\"done after {turn}\")
end

function frame_for(turn: Integer) returns List of Shape
  purpose: A frame with one column and one label.

  return [
    Column(screen_x: turn, top: 10, bottom: 20, colour: Colour(red: 255, green: 0, blue: 0)),
    Label(screen_x: 5, screen_y: 5, text: \"turn {turn}\", colour: Colour(red: 0, green: 0, blue: 0))
  ]
end
";

const PRINTED: &str = "frame 0: elapsed 16\nframe 1: elapsed 16\ndone after 2\n";

fn frame_of_turn(turn: i64) -> Vec<Shape> {
    vec![
        Shape::Column {
            screen_x: turn,
            top: 10,
            bottom: 20,
            colour: Colour {
                red: 255,
                green: 0,
                blue: 0,
            },
        },
        Shape::Label {
            screen_x: 5,
            screen_y: 5,
            text: format!("turn {turn}"),
            colour: Colour {
                red: 0,
                green: 0,
                blue: 0,
            },
        },
    ]
}

#[test]
fn a_host_that_answers_at_once_drives_the_frame_loop() {
    let program = compile("game.ry", GAME);
    let host = Rc::new(RefCell::new(Host {
        quit_after: 2,
        ..Host::default()
    }));
    let stdout = Capture::default();
    let outcome = run_main(&program, options(&stdout, Some(host.clone())));
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(stdout.text(), PRINTED);
    let frames = &host.borrow().frames;
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0], frame_of_turn(0));
    assert_eq!(frames[2], frame_of_turn(2));
}

/// The game driven by a host that answers later: the run suspends at
/// every frame and `resume` brings the input; what it printed and the
/// recording of the session.
fn suspended_session() -> (Program, String, Recording) {
    let program = compile("game.ry", GAME);
    let main = program.main.expect("main");
    let needs = program.function_metas[main].needs.clone();
    let host = Rc::new(RefCell::new(Host {
        later: true,
        ..Host::default()
    }));
    let stdout = Capture::default();
    let mut vm = Vm::new(
        &program,
        Options {
            record: true,
            // the host keeps the run on the interpreter by itself
            interpret: false,
            ..options(&stdout, Some(host.clone()))
        },
    );
    vm.begin_run(&needs, "game").expect("the run begins");
    assert!(matches!(vm.run_main(), Err(Interrupt::Suspended)));
    assert!(vm.is_suspended());
    assert_eq!(host.borrow().frames, vec![frame_of_turn(0)]);
    let go = Input {
        elapsed_ms: 16,
        ..Input::default()
    };
    assert!(matches!(vm.resume(go), Err(Interrupt::Suspended)));
    assert_eq!(host.borrow().frames.len(), 2);
    assert!(matches!(vm.resume(go), Err(Interrupt::Suspended)));
    assert_eq!(host.borrow().frames[2], frame_of_turn(2));
    let finished = vm.resume(Input { quit: true, ..go });
    assert!(matches!(finished, Ok(None)), "{finished:?}");
    assert!(!vm.is_suspended());
    let recording = vm.take_recording().expect("a recording");
    drop(vm);
    (program, stdout.text(), recording)
}

#[test]
fn a_host_that_answers_later_suspends_the_run_and_resume_continues_it() {
    let (_, printed, recording) = suspended_session();
    assert_eq!(printed, PRINTED);
    let primitives: Vec<&str> = recording
        .calls
        .iter()
        .map(|call| call.primitive.as_str())
        .collect();
    assert_eq!(
        primitives,
        [
            "std.screen.present",
            "std.console.print",
            "std.screen.present",
            "std.console.print",
            "std.screen.present",
            "std.console.print",
        ]
    );
    let sequences: Vec<u64> = recording.calls.iter().map(|call| call.sequence).collect();
    assert_eq!(sequences, [1, 2, 3, 4, 5, 6]);
    let first = &recording.calls[0];
    assert_eq!(first.capability, "screen");
    assert_eq!(first.arguments.len(), 1);
    assert_eq!(first.arguments[0].0, "frame");
    assert!(matches!(&first.arguments[0].1, Json::Array(shapes) if shapes.len() == 2));
    let Outcome::Success(Json::Object(fields)) = &recording.calls[4].outcome else {
        panic!(
            "the last call's outcome is the input: {:?}",
            recording.calls[4]
        );
    };
    assert!(fields.contains(&("quit".to_string(), Json::Boolean(true))));
    assert!(fields.contains(&("elapsed_ms".to_string(), Json::Number("16".to_string()))));
}

#[test]
fn a_replay_answers_present_from_the_recording_without_the_host() {
    let (program, printed, recording) = suspended_session();
    let stdout = Capture::default();
    let untouched: SharedScreen = Rc::new(RefCell::new(Untouched));
    let run = run_program(
        &program,
        Options {
            replay: Some(recording),
            replay_output: true,
            ..options(&stdout, Some(untouched))
        },
    );
    assert_eq!(run.outcome, RunOutcome::Finished);
    assert!(run.unused.is_empty(), "{:?}", run.unused);
    assert_eq!(stdout.text(), printed);
}

#[test]
fn a_run_without_a_host_crashes_at_present() {
    let program = compile("game.ry", GAME);
    let stdout = Capture::default();
    let outcome = run_main(&program, options(&stdout, None));
    let RunOutcome::Crashed { message, location } = outcome else {
        panic!("{outcome:?}");
    };
    assert_eq!(
        message,
        "the screen is not available here: the run has no host for `screen`"
    );
    assert!(location.is_some());
    assert_eq!(stdout.text(), "");
}

#[test]
fn a_host_that_answers_later_under_the_runner_is_a_crash_that_says_so() {
    let program = compile("game.ry", GAME);
    let host = Rc::new(RefCell::new(Host {
        later: true,
        ..Host::default()
    }));
    let stdout = Capture::default();
    let outcome = run_main(&program, options(&stdout, Some(host.clone())));
    let RunOutcome::Crashed { message, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        message.starts_with("suspended at `screen.present`"),
        "{message}"
    );
    assert_eq!(host.borrow().frames.len(), 1);
}

#[test]
fn the_size_and_every_shape_cross_the_boundary_with_the_colours_clamped() {
    let program = compile(
        "shapes.ry",
        "module shapes
  purpose: Every shape across the boundary, a colour out of range clamped.

import std.console
import std.screen exposing Colour, Shape

public function main() needs screen, console
  purpose: Present one frame of three shapes and print the size and the input.

  let input be screen.present([
    Column(screen_x: 0, top: 0, bottom: 199, colour: Colour(red: 300, green: 0, blue: 128)),
    Rectangle(screen_x: 10, screen_y: 20, width: 30, height: 40, colour: Colour(red: 1, green: 2, blue: 3)),
    Label(screen_x: 5, screen_y: 5, text: \"score 12\", colour: Colour(red: 255, green: 255, blue: 255))
  ])
  console.print(\"{screen.width()} by {screen.height()}\")
  console.print(\"forward {input.forward}, fire {input.fire}, elapsed {input.elapsed_ms}\")
end
",
    );
    struct Keys(Vec<Vec<Shape>>);
    impl Screen for Keys {
        fn present(&mut self, frame: Vec<Shape>) -> Answer {
            self.0.push(frame);
            Answer::Now(Input {
                forward: true,
                fire: true,
                elapsed_ms: 33,
                ..Input::default()
            })
        }
    }
    let host = Rc::new(RefCell::new(Keys(Vec::new())));
    let stdout = Capture::default();
    let outcome = run_main(&program, options(&stdout, Some(host.clone())));
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        stdout.text(),
        "320 by 200\nforward true, fire true, elapsed 33\n"
    );
    let white = Colour {
        red: 255,
        green: 255,
        blue: 255,
    };
    assert_eq!(
        host.borrow().0,
        vec![vec![
            Shape::Column {
                screen_x: 0,
                top: 0,
                bottom: 199,
                colour: Colour {
                    red: 255,
                    green: 0,
                    blue: 128
                },
            },
            Shape::Rectangle {
                screen_x: 10,
                screen_y: 20,
                width: 30,
                height: 40,
                colour: Colour {
                    red: 1,
                    green: 2,
                    blue: 3
                },
            },
            Shape::Label {
                screen_x: 5,
                screen_y: 5,
                text: "score 12".to_string(),
                colour: white,
            },
        ]]
    );
}

#[test]
fn a_suspension_inside_a_callback_is_a_crash_that_says_where() {
    let port = 19000 + (std::process::id() % 2000) as u16;
    let source = format!(
        r#"module service
  purpose: A server whose handler presents a frame.

import std.screen exposing Colour, Shape
import std.server exposing Request, Response, Port, StartError

function handle(request: Request) returns Response needs screen
  purpose: Present a frame per request, labelled with its path.

  let frame be [
    Label(screen_x: 5, screen_y: 5, text: request.path, colour: Colour(red: 255, green: 0, blue: 0))
  ]
  let input be screen.present(frame)
  if input.quit then return server.ok("quit") end
  return server.ok("ok")
end

public function main() or fails with StartError needs screen, network.socket
  purpose: Serve until the limit.

  server.serve(port: Port({port}), handler: handle) otherwise fail
end
"#
    );
    let server = std::thread::spawn(move || {
        let program = compile("service.ry", &source);
        // the handler runs on the loop `server.serve` starts, inside the
        // primitive: a host that answers later cannot be resumed there
        let host = Rc::new(RefCell::new(Host {
            later: true,
            ..Host::default()
        }));
        let stdout = Capture::default();
        let outcome = run_program(
            &program,
            Options {
                serve_limit: Some(1),
                ..options(&stdout, Some(host.clone()))
            },
        )
        .outcome;
        let frames = host.borrow().frames.clone();
        (outcome, frames)
    });
    let started = Instant::now();
    let mut stream = loop {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)) {
            break stream;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    let _ = stream.write_all(b"GET /maze HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    let mut answer = Vec::new();
    // the run crashes inside the handler, so the answer may be nothing
    let _ = stream.read_to_end(&mut answer);
    let (outcome, frames) = server.join().expect("the server thread");
    let RunOutcome::Crashed { message, .. } = outcome else {
        panic!("{outcome:?}");
    };
    assert!(
        message.starts_with("screen.present cannot wait here"),
        "{message}"
    );
    assert_eq!(
        frames,
        vec![vec![Shape::Label {
            screen_x: 5,
            screen_y: 5,
            text: "/maze".to_string(),
            colour: Colour {
                red: 255,
                green: 0,
                blue: 0
            },
        }]]
    );
}

#[test]
fn the_console_host_draws_the_frame_as_text_and_reads_the_keys_from_a_line() {
    use renyi_vm::screen::ConsoleScreen;

    // the grid of the text view's own test: a column under a label
    let white = Colour {
        red: 255,
        green: 255,
        blue: 255,
    };
    let frame = vec![
        Shape::Column {
            screen_x: 120,
            top: 0,
            bottom: 99,
            colour: white,
        },
        Shape::Label {
            screen_x: 0,
            screen_y: 150,
            text: "HI".to_string(),
            colour: white,
        },
    ];
    assert_eq!(ConsoleScreen::text_of(&frame, 4, 2), " @  \nHI  ");

    let program = compile(
        "keys.ry",
        "module keys
  purpose: A frame loop that prints the keys the host sends.

import std.console
import std.screen exposing Colour, Shape

public function main() needs screen, console
  purpose: Present frames until the host quits, printing the keys of each.

  let mutable turn be 0
  repeat until turn is at least 5
    let input be screen.present([
      Rectangle(screen_x: 0, screen_y: 0, width: 160, height: 200, colour: Colour(red: 255, green: 255, blue: 255)),
      Label(screen_x: 160, screen_y: 0, text: \"T{turn}\", colour: Colour(red: 0, green: 0, blue: 0))
    ])
    let keys be \"w {input.forward} s {input.backward} a {input.turn_left} d {input.turn_right}\"
    console.print(\"{turn}: {keys} f {input.fire} q {input.quit}\")
    if input.quit then break end
    change turn to turn + 1
  end
end
",
    );
    let stdout = Capture::default();
    // the host writes the frames to the same stream the program prints to;
    // the lines are the keys of three frames, then the end of the input
    let host: SharedScreen = Rc::new(RefCell::new(ConsoleScreen::new(
        Box::new(stdout.clone()),
        Box::new(std::io::Cursor::new(b"wf\n\nsad\n".to_vec())),
        8,
        2,
    )));
    let outcome = run_main(&program, options(&stdout, Some(host)));
    assert_eq!(outcome, RunOutcome::Finished);
    let prompt =
        "w forward, s backward, a left, d right, f fire, q quit, enter for the next frame> ";
    let expected = format!(
        "@@@@T0  \n@@@@    \n{prompt}0: w true s false a false d false f true q false\n\
         @@@@T1  \n@@@@    \n{prompt}1: w false s false a false d false f false q false\n\
         @@@@T2  \n@@@@    \n{prompt}2: w false s true a true d true f false q false\n\
         @@@@T3  \n@@@@    \n{prompt}3: w false s false a false d false f false q true\n"
    );
    assert_eq!(stdout.text(), expected);
}
