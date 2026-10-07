//! `std.process` (decision AE1): a program is started by name or by path
//! with its arguments one by one, runs to its end and is reported as a
//! `Completion`. The child's standard output and standard error are read
//! on two threads while the parent waits, so that a child which fills a
//! pipe does not stall; a time limit polls the child and kills it when the
//! limit passes. The grant and the budget are checked before a primitive
//! of this module runs (`call_primitive`): a program outside the scope is
//! `ProgramNotAllowed`, a call past the budget `OverBudget`.

use std::io::{Read, Write};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::rc::Rc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use indexmap::IndexMap;

use super::{arg, crash, list, text, NativeFn};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str) -> Option<NativeFn> {
    Some(match name {
        "execute" => execute,
        "execute_with" => execute_with,
        "attempt" => attempt,
        "attempt_with" => attempt_with,
        "defaults" => defaults,
        _ => return None,
    })
}

/// How often a limited run looks at the child.
const POLL: Duration = Duration::from_millis(5);

/// The fields of `Options`: how the program is started.
#[derive(Default)]
struct Start {
    directory: Option<String>,
    environment: Vec<(String, String)>,
    input: String,
    limit: Option<Duration>,
}

fn execute(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let program = text(arg(args, 0))?.to_string();
    let arguments = arguments(arg(args, 1))?;
    complete(vm, &program, &arguments, Start::default(), true)
}

fn execute_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let program = text(arg(args, 0))?.to_string();
    let arguments = arguments(arg(args, 1))?;
    let start = options(vm, arg(args, 2))?;
    complete(vm, &program, &arguments, start, true)
}

fn attempt(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let program = text(arg(args, 0))?.to_string();
    let arguments = arguments(arg(args, 1))?;
    complete(vm, &program, &arguments, Start::default(), false)
}

fn attempt_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let program = text(arg(args, 0))?.to_string();
    let arguments = arguments(arg(args, 1))?;
    let start = options(vm, arg(args, 2))?;
    complete(vm, &program, &arguments, start, false)
}

fn defaults(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    vm.library_record(
        "std.process",
        "Options",
        vec![
            Value::Nothing,
            Value::Map(Rc::new(IndexMap::new())),
            Value::text(""),
            Value::Nothing,
        ],
    )
}

fn arguments(value: &Value) -> Result<Vec<String>, Interrupt> {
    list(value)?
        .iter()
        .map(|item| text(item).map(str::to_string))
        .collect()
}

/// The `Options` record read by its field names.
fn options(vm: &Vm, value: &Value) -> Result<Start, Interrupt> {
    let Value::Record(record) = value else {
        return Err(crash("a library function expected Options"));
    };
    let field = |name: &str| {
        vm.program
            .types
            .field_index(record.ty, name)
            .and_then(|index| record.fields.get(index))
    };
    let directory = field("directory")
        .and_then(Value::as_text)
        .map(str::to_string);
    let mut environment = Vec::new();
    if let Some(Value::Map(entries)) = field("environment") {
        for (name, value) in entries.iter() {
            environment.push((text(name)?.to_string(), text(value)?.to_string()));
        }
    }
    let input = field("input")
        .and_then(Value::as_text)
        .unwrap_or_default()
        .to_string();
    let limit = match field("limit") {
        Some(Value::Duration(ms)) => Some(Duration::from_millis((*ms).max(0) as u64)),
        _ => None,
    };
    Ok(Start {
        directory,
        environment,
        input,
        limit,
    })
}

fn error(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Value, Interrupt> {
    vm.fail_variant("std.process", "ProcessError", variant, fields)
}

/// Start the program, feed its input, wait for its end within the limit
/// and report its completion; `strict` makes a status other than 0 the
/// failure `Exited`.
fn complete(
    vm: &mut Vm,
    program: &str,
    arguments: &[String],
    start: Start,
    strict: bool,
) -> Result<Value, Interrupt> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(directory) = &start.directory {
        command.current_dir(directory);
    }
    command.envs(start.environment.iter().map(|(name, value)| (name, value)));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
            return error(vm, "NotFound", vec![Value::text(program)]);
        }
        Err(cause) => {
            return error(
                vm,
                "CannotStart",
                vec![Value::text(program), Value::text(cause.to_string())],
            );
        }
    };
    let output = child.stdout.take().map(drain);
    let errors = child.stderr.take().map(drain);
    let feeder = child.stdin.take().map(|mut stdin| {
        let input = start.input;
        std::thread::spawn(move || {
            // a child that ends without reading its input closes the pipe
            // first, which is no failure of the call; the pipe is dropped
            // when the thread ends, so that the child sees the end of its
            // input
            let _ = stdin.write_all(input.as_bytes());
        })
    });
    let status = match wait(&mut child, start.limit) {
        Ok(Some(status)) => status,
        Ok(None) => {
            // the limit passed: the child is killed and reaped, its pipes
            // close, and the readers end with whatever it wrote
            let _ = child.kill();
            let _ = child.wait();
            collect(output);
            collect(errors);
            join(feeder);
            return error(vm, "Timeout", vec![Value::text(program)]);
        }
        Err(cause) => {
            return Err(crash(format!("cannot wait for `{program}`: {cause}")));
        }
    };
    let output = collect(output);
    let errors = collect(errors);
    join(feeder);
    let status = code_of(status);
    let output_text = String::from_utf8_lossy(&output).into_owned();
    let errors_text = String::from_utf8_lossy(&errors).into_owned();
    if strict && status != 0 {
        return error(
            vm,
            "Exited",
            vec![
                Value::text(program),
                Value::integer(status),
                Value::text(output_text),
                Value::text(errors_text),
            ],
        );
    }
    vm.library_record(
        "std.process",
        "Completion",
        vec![
            Value::integer(status),
            Value::text(output_text),
            Value::text(errors_text),
            Value::Bytes(Rc::from(output)),
        ],
    )
}

/// Read a pipe to its end on a thread of its own.
fn drain<R: Read + Send + 'static>(mut pipe: R) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        // a pipe that breaks midway leaves what was read; the child's
        // status says the rest
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

fn collect(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

fn join(feeder: Option<JoinHandle<()>>) {
    if let Some(handle) = feeder {
        // the feeder ends when its input is written or the pipe breaks;
        // neither is a failure of the call
        let _ = handle.join();
    }
}

/// The child's status, or `None` when the limit passed first.
fn wait(child: &mut Child, limit: Option<Duration>) -> std::io::Result<Option<ExitStatus>> {
    let Some(limit) = limit else {
        return child.wait().map(Some);
    };
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if started.elapsed() >= limit {
            return Ok(None);
        }
        // `Child::wait` has no deadline, so a limited run polls; the pause
        // bounds the polling, it masks no race
        std::thread::sleep(POLL);
    }
}

/// The exit status as a number: the code, or 128 plus the signal that
/// ended the child, as a shell reports it.
fn code_of(status: ExitStatus) -> i64 {
    if let Some(code) = status.code() {
        return code as i64;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal as i64;
        }
    }
    -1
}
