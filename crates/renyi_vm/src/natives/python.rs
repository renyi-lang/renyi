//! The Python bridge (decisions AJ2, AJ3 and AL1 to AL4): a function of a
//! Python module of the project runs in a worker process, one per run,
//! started at the first call with the interpreter the manifest, the
//! environment or the PATH names (decision AL3) and ended with the VM. A
//! call is one JSON object on the worker's standard input and one on its
//! standard output (decision AL2, the program `python_worker.py` beside
//! this file): the arguments encoded as `std.json` renders them, by
//! position, and the result decoded by the declared type. What goes wrong
//! is the failure `PythonError` of `std.python` (decision AL4). The
//! boundary (`Vm::call_primitive`) does the rest: the grant, the
//! recording, the replay, which never starts the worker, and the guards.
//! What the Python side does is outside every guarantee of the VM.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use renyi_check::types::Ty;
use renyi_check::FunctionId;

use super::crash;
use super::json::{self, read_json, write_json, Json, Naming};
use crate::compile::Python;
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// The worker's program, passed to the interpreter with `-c`.
const WORKER: &str = include_str!("python_worker.py");

/// The environment variable that names the interpreter when the manifest
/// does not (decision AL3).
pub const INTERPRETER_VARIABLE: &str = "RENYI_PYTHON";

/// The worker process of a run: the interpreter that answered, its
/// version, and the pipes the calls go through.
pub struct Worker {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    pub interpreter: String,
    pub version: String,
    calls: u64,
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Call the Python function with the arguments; the worker is started at
/// the first call and kept.
pub fn run(
    vm: &mut Vm,
    function: FunctionId,
    origins: u64,
    args: &[Value],
) -> Result<Value, Interrupt> {
    let program = vm.program;
    let meta = &program.function_metas[function];
    let qualified = vm.qualified(function);
    let binding = meta.python.clone().ok_or_else(|| {
        crash(format!(
            "`{qualified}` is not a function of a Python module"
        ))
    })?;
    let mut arguments = Vec::with_capacity(args.len());
    for value in args {
        arguments.push(json::encode(vm, value, Naming::Exact)?);
    }
    if vm.python.is_none() {
        match Worker::start(&binding) {
            Ok(worker) => vm.python = Some(worker),
            Err(detail) => return unavailable(vm, detail),
        }
    }
    let worker = vm.python.as_mut().expect("the worker just started");
    let answer = match worker.call(&binding.package, &binding.symbol, arguments) {
        Ok(answer) => answer,
        Err(detail) => {
            // a worker that stopped answering is not asked again
            vm.python = None;
            return unavailable(vm, format!("`{qualified}`: {detail}"));
        }
    };
    let value = decode(vm, &qualified, &answer, meta.returns.as_ref())?;
    Ok(value.guarded(origins))
}

/// The worker's answer as a value of the declared result type, or the
/// failure it reports.
fn decode(
    vm: &mut Vm,
    qualified: &str,
    answer: &Json,
    returns: Option<&Ty>,
) -> Result<Value, Interrupt> {
    let Json::Object(fields) = answer else {
        return unavailable(
            vm,
            format!(
                "`{qualified}`: the Python worker answered with {}",
                answer.kind()
            ),
        );
    };
    if let Some(error) = field(fields, "error") {
        let Json::Object(error) = error else {
            return unavailable(
                vm,
                format!("`{qualified}`: the Python worker reported an error without its parts"),
            );
        };
        let part = |name: &str| match field(error, name) {
            Some(Json::Text(text)) => text.clone(),
            _ => String::new(),
        };
        let (kind, message) = (part("kind"), part("message"));
        return match part("where").as_str() {
            "call" => fail(vm, "Raised", vec![Value::text(kind), Value::text(message)]),
            "result" => fail(
                vm,
                "NotCarried",
                vec![Value::text(format!(
                    "`{qualified}` returned a value JSON does not carry: {kind}: {message}"
                ))],
            ),
            _ => unavailable(
                vm,
                format!("`{qualified}`: the Python side does not provide it: {kind}: {message}"),
            ),
        };
    }
    // a result the declaration does not mention is dropped
    let Some(ty) = returns else {
        return Ok(Value::Nothing);
    };
    let result = field(fields, "result").unwrap_or(&Json::Null);
    match json::decode(vm, result, ty, "$", Naming::Exact)? {
        Ok(value) => Ok(value),
        Err(error) => {
            let shown = vm.to_text(&error)?;
            fail(
                vm,
                "NotCarried",
                vec![Value::text(format!(
                    "the result of `{qualified}` does not fit its declared type: {shown}"
                ))],
            )
        }
    }
}

fn field<'a>(fields: &'a [(String, Json)], name: &str) -> Option<&'a Json> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

fn fail(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Value, Interrupt> {
    vm.fail_variant("std.python", "PythonError", variant, fields)
}

fn unavailable(vm: &Vm, detail: String) -> Result<Value, Interrupt> {
    fail(vm, "Unavailable", vec![Value::text(detail)])
}

/// The interpreters to try, in order (decision AL3): the manifest's, else
/// the environment's, else the PATH's `python3` and `python` (`python`
/// first on Windows, where `python3` is usually the Store's stub).
fn candidates(binding: &Python) -> Vec<String> {
    if let Some(interpreter) = &binding.interpreter {
        return vec![interpreter.clone()];
    }
    if let Ok(interpreter) = std::env::var(INTERPRETER_VARIABLE) {
        if !interpreter.is_empty() {
            return vec![interpreter];
        }
    }
    if cfg!(windows) {
        vec!["python".to_string(), "python3".to_string()]
    } else {
        vec!["python3".to_string(), "python".to_string()]
    }
}

impl Worker {
    /// The first interpreter of the candidates that starts and answers the
    /// bridge's greeting.
    pub fn start(binding: &Python) -> Result<Worker, String> {
        let mut tried = Vec::new();
        for interpreter in candidates(binding) {
            match Worker::spawn(&interpreter, &binding.root) {
                Ok(worker) => return Ok(worker),
                Err(detail) => tried.push(format!("`{interpreter}`: {detail}")),
            }
        }
        Err(format!(
            "no Python interpreter answered: {}",
            tried.join("; ")
        ))
    }

    fn spawn(interpreter: &str, root: &str) -> Result<Worker, String> {
        let mut child = Command::new(interpreter)
            .arg("-c")
            .arg(WORKER)
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| error.to_string())?;
        let input = child.stdin.take().expect("the worker's input is piped");
        let mut output = BufReader::new(child.stdout.take().expect("the worker's output is piped"));
        let mut line = String::new();
        let greeting = match output.read_line(&mut line) {
            Ok(0) => Err("it ended without the bridge's greeting".to_string()),
            Ok(_) => read_json(line.trim_end())
                .map_err(|(detail, _)| format!("its first line is not JSON: {detail}")),
            Err(error) => Err(format!("cannot read from it: {error}")),
        };
        let version = greeting.and_then(|json| match &json {
            Json::Object(fields) => match (field(fields, "ready"), field(fields, "version")) {
                (Some(Json::Boolean(true)), Some(Json::Text(version))) => Ok(version.clone()),
                _ => Err("its first line is not the bridge's greeting".to_string()),
            },
            _ => Err("its first line is not the bridge's greeting".to_string()),
        });
        match version {
            Ok(version) => Ok(Worker {
                child,
                input,
                output,
                interpreter: interpreter.to_string(),
                version,
                calls: 0,
            }),
            Err(detail) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(detail)
            }
        }
    }

    /// One request and its answer.
    pub fn call(
        &mut self,
        module: &str,
        function: &str,
        arguments: Vec<Json>,
    ) -> Result<Json, String> {
        self.calls += 1;
        let request = Json::Object(vec![
            ("call".to_string(), Json::Number(self.calls.to_string())),
            ("module".to_string(), Json::Text(module.to_string())),
            ("function".to_string(), Json::Text(function.to_string())),
            ("arguments".to_string(), Json::Array(arguments)),
        ]);
        let mut text = String::new();
        write_json(&request, &mut text, None, 0);
        text.push('\n');
        self.input
            .write_all(text.as_bytes())
            .and_then(|_| self.input.flush())
            .map_err(|error| format!("cannot write to the Python worker: {error}"))?;
        let mut line = String::new();
        match self.output.read_line(&mut line) {
            Ok(0) => return Err("the Python worker ended".to_string()),
            Ok(_) => {}
            Err(error) => return Err(format!("cannot read from the Python worker: {error}")),
        }
        let answer = read_json(line.trim_end()).map_err(|(detail, _)| {
            format!("the Python worker answered with something that is not JSON: {detail}")
        })?;
        let answered = match &answer {
            Json::Object(fields) => match field(fields, "call") {
                Some(Json::Number(number)) => number.parse::<u64>().ok(),
                _ => None,
            },
            _ => None,
        };
        if answered != Some(self.calls) {
            return Err(format!(
                "the Python worker answered call {} for call {}",
                answered
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "?".to_string()),
                self.calls
            ));
        }
        Ok(answer)
    }
}
