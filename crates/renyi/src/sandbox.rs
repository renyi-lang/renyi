//! The embedding API (decisions Q3 and AP1): a host loads a Renyi module
//! with a grant and calls its public functions; the module reaches
//! nothing the grant does not name, because there is no ambient I/O, no
//! reflection and no native call without `foreign`. The grant is spelled
//! as a `needs` clause is, in a JSON file or a text, with a memory budget
//! beside it, and `renyi run --sandbox grant.json` runs a program's
//! `main` under the same grant. A C API over this one is decision A5's,
//! for later.

use std::fmt;

use renyi_check::effects::{self, Capability};
use renyi_index::{index_files_in, tools_of_in, Header, Kind};
use renyi_syntax::diagnostics::render_text;
use renyi_syntax::{parse_grant, SourceFile};
use renyi_vm::grant::{self, Counter};
use renyi_vm::natives::json::{read_json, Json};
use renyi_vm::{file, memory, Interrupt, Narrowing, Options, Program, Value, Vm};

use crate::{
    compile_file, compile_sources, library, registry, toolchain, CompileError, Compiled, Flags,
};

/// What a module may do: the capabilities as a `needs` clause spells
/// them (scopes, `at most` budgets, `only to` guards), and the bytes it
/// may hold above the level at the start of a call.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grant {
    pub capabilities: Vec<Capability>,
    /// In bytes; `None` for no budget.
    pub memory: Option<u64>,
}

impl Grant {
    /// A grant from its spelling, `console, network.http("api.example.com")
    /// at most 60 per minute`; an empty text grants nothing.
    pub fn parse(text: &str) -> Result<Grant, String> {
        let parsed = parse_grant(text).map_err(|diagnostics| {
            let file = SourceFile::new("grant", text);
            format!(
                "the grant does not parse:\n{}",
                render_text(&file, &diagnostics)
            )
        })?;
        let mut capabilities = Vec::new();
        for syntax in &parsed {
            let capability = Capability::from_ast(syntax);
            if !capability.is_known() {
                return Err(format!(
                    "the grant names `{}`, which is not a capability; one of {}",
                    capability.path.join("."),
                    effects::TREE.join(", ")
                ));
            }
            if capability.scope.is_some() && !effects::takes_scope(&capability.path) {
                return Err(format!(
                    "the grant gives `{}` a scope, which it does not take",
                    capability.path.join(".")
                ));
            }
            for (path, _) in &capability.only_to {
                if !effects::TREE.contains(&path.join(".").as_str()) {
                    return Err(format!(
                        "the grant names the sink `{}`, which is not a capability",
                        path.join(".")
                    ));
                }
            }
            capabilities.push(capability);
        }
        Ok(Grant {
            capabilities,
            memory: None,
        })
    }

    /// A grant from its JSON: `{"grant": "console, filesystem.read(\"data\")",
    /// "memory": "256 megabytes"}`; both fields optional, no others.
    pub fn from_json(text: &str) -> Result<Grant, String> {
        let json = read_json(text)
            .map_err(|(detail, at)| format!("the grant does not parse: {detail} (at {at})"))?;
        let Json::Object(fields) = json else {
            return Err("a grant is a JSON object with `grant` and `memory`".to_string());
        };
        let mut grant = Grant::default();
        for (name, value) in fields {
            match (name.as_str(), value) {
                ("grant", Json::Text(spelled)) => {
                    grant.capabilities = Grant::parse(&spelled)?.capabilities;
                }
                ("grant", _) => {
                    return Err(
                        "`grant` is a text: the capabilities as a `needs` clause spells them"
                            .to_string(),
                    )
                }
                ("memory", Json::Text(spelled)) => grant.memory = Some(parse_memory(&spelled)?),
                ("memory", _) => {
                    return Err("`memory` is a text such as `256 megabytes`".to_string())
                }
                (other, _) => {
                    return Err(format!(
                        "a grant has no field `{other}`; it takes `grant` and `memory`"
                    ))
                }
            }
        }
        Ok(grant)
    }

    /// The grant in a JSON file.
    pub fn from_file(path: &str) -> Result<Grant, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("cannot read {path}: {error}"))?;
        Grant::from_json(&text).map_err(|detail| format!("{path}: {detail}"))
    }

    /// The capabilities spelled as a `needs` clause would spell them.
    pub fn spelling(&self) -> String {
        self.capabilities
            .iter()
            .map(grant::spell)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// `256 megabytes`, `64 kilobytes`, `1 gigabyte`, `4096 bytes`, in bytes.
pub fn parse_memory(text: &str) -> Result<u64, String> {
    let mut parts = text.split_whitespace();
    let (Some(count), Some(unit), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!(
            "`{text}`: a memory budget is a count and a unit, such as `256 megabytes`"
        ));
    };
    let count: u64 = count
        .parse()
        .map_err(|_| format!("`{text}`: the count `{count}` is not a whole number"))?;
    let factor: u64 = match unit {
        "byte" | "bytes" => 1,
        "kilobyte" | "kilobytes" => 1 << 10,
        "megabyte" | "megabytes" => 1 << 20,
        "gigabyte" | "gigabytes" => 1 << 30,
        _ => {
            return Err(format!(
                "`{text}`: the unit `{unit}` is not bytes, kilobytes, megabytes or gigabytes"
            ))
        }
    };
    count
        .checked_mul(factor)
        .ok_or_else(|| format!("`{text}`: the budget does not fit in 64 bits"))
}

/// Bytes for a message: `256 megabytes`, `1.5 megabytes`, `640 bytes`.
pub fn bytes_text(bytes: u64) -> String {
    const UNITS: [(u64, &str); 3] = [
        (1 << 30, "gigabytes"),
        (1 << 20, "megabytes"),
        (1 << 10, "kilobytes"),
    ];
    for (size, unit) in UNITS {
        if bytes >= size {
            let value = bytes as f64 / size as f64;
            return if (value - value.round()).abs() < 0.05 {
                format!("{} {unit}", value.round() as u64)
            } else {
                format!("{value:.1} {unit}")
            };
        }
    }
    format!("{bytes} bytes")
}

/// A public function of the loaded module, as the host sees it.
#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    /// The head and its clauses on one line, as the project map spells
    /// them.
    pub signature: String,
    pub purpose: Option<String>,
    pub parameters: usize,
    /// The `needs` clause, one capability per entry.
    pub needs: Vec<String>,
    pub exposed_as_tool: bool,
    /// For a function with `expose as tool`: the JSON Schema of its
    /// parameters as a JSON text, as `renyi tools` writes it.
    pub input_schema: Option<String>,
}

/// How a call ended, other than with its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallError {
    /// No such public function, the wrong number of arguments, or a
    /// `needs` the grant does not cover: nothing ran.
    Refused(String),
    /// The function failed with this error, rendered.
    Failed(String),
    Crashed {
        message: String,
        location: Option<String>,
    },
    /// `environment.exit(code)`.
    Exited(i32),
    /// The memory budget was exceeded: the limit, and the most bytes the
    /// call held above the level at its start.
    OverMemory { limit: u64, used: u64 },
}

impl fmt::Display for CallError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CallError::Refused(message) => write!(out, "refused: {message}"),
            CallError::Failed(error) => write!(out, "failed with {error}"),
            CallError::Crashed {
                message,
                location: Some(location),
            } => write!(out, "crashed: {message} (at {location})"),
            CallError::Crashed { message, .. } => write!(out, "crashed: {message}"),
            CallError::Exited(code) => write!(out, "exited with code {code}"),
            CallError::OverMemory { limit, used } => write!(
                out,
                "exceeded the memory budget of {} ({} held)",
                bytes_text(*limit),
                bytes_text(*used)
            ),
        }
    }
}

impl std::error::Error for CallError {}

/// A module loaded with a grant. Each call runs under the grant: the
/// function's `needs` intersected with it, its budgets counted across
/// the calls as one run, its guards on the values the host hands in
/// through `guarded`, its memory budget on each call.
pub struct Sandbox {
    program: Program,
    module: String,
    grant: Grant,
    functions: Vec<Function>,
    warnings: String,
    /// The budget counters and their clock after the last call.
    budgets: Option<(Vec<Counter>, i64)>,
}

impl Sandbox {
    /// A module from its source file, its imports resolved from the
    /// file's directory and its project, checked clean. A bytecode file
    /// is refused: it carries no visibility.
    pub fn load(path: &str, grant: Grant) -> Result<Sandbox, String> {
        if file::is_bytecode(path) {
            return Err(format!(
                "{path} is a bytecode file, which carries no visibility; give the source file"
            ));
        }
        let compiled = compile_sources(path).map_err(describe)?;
        Sandbox::of(compiled, grant)
    }

    /// A module from a text; `name` is the file name its imports resolve
    /// from (a bare name resolves from the working directory).
    pub fn load_source(name: &str, text: &str, grant: Grant) -> Result<Sandbox, String> {
        let compiled = compile_file(SourceFile::new(name, text)).map_err(describe)?;
        Sandbox::of(compiled, grant)
    }

    fn of(compiled: Compiled, grant: Grant) -> Result<Sandbox, String> {
        if grant.memory.is_some() && !memory::installed() {
            return Err(
                "the memory budget needs the counting allocator: declare `#[global_allocator] static ALLOCATOR: renyi::Allocator = renyi::Allocator;` in the binary, or wrap its allocator in `renyi::Counting`"
                    .to_string(),
            );
        }
        let header = Header {
            project: String::new(),
            revision: String::new(),
            toolchain: toolchain(),
        };
        let index = index_files_in(&library(), &compiled.sources, header);
        let main_file = &compiled.sources[0].name;
        let module = index
            .modules
            .iter()
            .find(|module| module.file == *main_file)
            .map(|module| module.name.clone())
            .ok_or_else(|| format!("{main_file} has no module in the map"))?;
        let tools = tools_of_in(&library(), &compiled.sources);
        let functions = index
            .definitions
            .iter()
            .filter(|definition| {
                definition.module == module
                    && definition.public
                    && matches!(definition.kind, Kind::Function)
            })
            .map(|definition| Function {
                name: definition.name.clone(),
                signature: definition.signature.clone(),
                purpose: definition.purpose.clone(),
                parameters: compiled
                    .program
                    .function_metas
                    .iter()
                    .find(|meta| {
                        meta.module == module && meta.name == definition.name && !meta.is_method
                    })
                    .map_or(0, |meta| meta.params.len()),
                needs: definition.effects_declared.clone(),
                exposed_as_tool: definition.exposed_as_tool,
                input_schema: tools
                    .iter()
                    .find(|tool| tool.module == module && tool.function == definition.name)
                    .map(|tool| tool.input_schema.render()),
            })
            .collect();
        Ok(Sandbox {
            program: compiled.program,
            module,
            grant,
            functions,
            warnings: compiled.diagnostics,
            budgets: None,
        })
    }

    /// The module's name.
    pub fn module(&self) -> &str {
        &self.module
    }

    pub fn grant(&self) -> &Grant {
        &self.grant
    }

    /// The public functions of the module, in source order.
    pub fn functions(&self) -> &[Function] {
        &self.functions
    }

    /// The checker's warnings on the module, as `renyi check` prints
    /// them; empty when there are none.
    pub fn warnings(&self) -> &str {
        &self.warnings
    }

    /// Call a public function by name with its arguments in order. The
    /// call is refused before it runs when the function is not public,
    /// the arguments are not as many as its parameters, or it needs a
    /// capability the grant does not cover; it runs under the grant
    /// otherwise, and a primitive outside what the function was given
    /// fails at the boundary as under `renyi run`.
    pub fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, CallError> {
        if !self.functions.iter().any(|function| function.name == name) {
            return Err(CallError::Refused(format!(
                "`{}` has no public function `{name}`",
                self.module
            )));
        }
        let id = self
            .program
            .function_metas
            .iter()
            .position(|meta| meta.module == self.module && meta.name == name && !meta.is_method)
            .expect("a public function of the module is in the program");
        let meta = &self.program.function_metas[id];
        if args.len() != meta.params.len() {
            return Err(CallError::Refused(format!(
                "`{name}` takes {} argument{}, not {}",
                meta.params.len(),
                if meta.params.len() == 1 { "" } else { "s" },
                args.len()
            )));
        }
        for need in &meta.needs {
            if !effects::covered(&self.grant.capabilities, need, true) {
                return Err(CallError::Refused(format!(
                    "`{name}` needs `{}`, which the grant does not cover",
                    need.spelling()
                )));
            }
        }
        let needs = meta.needs.clone();
        let options = Options {
            narrowing: Narrowing {
                sandbox: Some(self.grant.capabilities.clone()),
                ..Narrowing::default()
            },
            memory: self.grant.memory,
            registry: registry().clone(),
            ..Options::default()
        };
        let mut vm = Vm::new(&self.program, options);
        vm.begin_run(&needs, &self.module)
            .map_err(CallError::Refused)?;
        if let Some((counters, started)) = self.budgets.take() {
            vm.resume_budgets(counters, started);
        }
        let result = vm.call_function(id, args);
        self.budgets = Some(vm.budgets());
        match result {
            Ok(Value::Failure(error)) => Err(CallError::Failed(
                vm.text_for_console(&error, "the failure")
                    .unwrap_or_else(|_| vm.describe(&error)),
            )),
            Ok(value) => Ok(value),
            Err(Interrupt::Crash { message, location }) => {
                Err(CallError::Crashed { message, location })
            }
            Err(Interrupt::Exit(code)) => Err(CallError::Exited(code)),
            Err(Interrupt::OverMemory { limit, used }) => {
                Err(CallError::OverMemory { limit, used })
            }
            Err(Interrupt::Reload) => Err(CallError::Crashed {
                message: "stopped for a new version".to_string(),
                location: None,
            }),
        }
    }

    /// A value the host hands in under a guard of the grant (decision
    /// P3, `only to`): it, and what the module computes from it, may
    /// leave the module only through the guard's sinks.
    pub fn guarded(&self, value: Value, capability: &str) -> Result<Value, String> {
        let wanted = grant::parse_capability(capability)?;
        let scope = match (&wanted.scope, wanted.path.first().map(String::as_str)) {
            (Some(scope), Some("filesystem")) => Some(grant::normalize_path(scope)),
            (scope, _) => scope.clone(),
        };
        let guards = grant::guards(&self.grant.capabilities);
        let index = guards
            .iter()
            .position(|guard| {
                guard.capability.path == wanted.path && guard.capability.scope == scope
            })
            .ok_or_else(|| {
                format!(
                    "the grant has no `only to` guard on `{}`",
                    wanted.spelling()
                )
            })?;
        Ok(value.guarded(1u64 << index))
    }
}

fn describe(error: CompileError) -> String {
    match error {
        CompileError::Read(message) => message,
        CompileError::Diagnostics(text) => format!("the module has errors:\n{text}"),
    }
}

/// `--sandbox <file>` applied to the flags of `run`, `record` and
/// `serve`: the grant read, its capabilities into the narrowing; the
/// memory budget comes back for the options.
pub(crate) fn apply_flag(flags: &mut Flags) -> Result<Option<u64>, String> {
    let Some(path) = &flags.sandbox else {
        return Ok(None);
    };
    let grant = Grant::from_file(path)?;
    flags.narrowing.sandbox = Some(grant.capabilities);
    Ok(grant.memory)
}

/// What a command refuses under a sandbox before it starts: a
/// capability `main` needs that the grant does not cover.
pub(crate) fn refusal(program: &Program, narrowing: &Narrowing) -> Option<String> {
    let sandbox = narrowing.sandbox.as_ref()?;
    let main = program.main?;
    let uncovered: Vec<String> = program.function_metas[main]
        .needs
        .iter()
        .filter(|need| !effects::covered(sandbox, need, true))
        .map(|need| format!("`{}`", need.spelling()))
        .collect();
    if uncovered.is_empty() {
        None
    } else {
        Some(format!(
            "the sandbox does not grant {}, which `main` needs",
            uncovered.join(", ")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grant_parses_as_a_needs_clause_does() {
        let grant = Grant::parse(
            "console, network.http(\"api.example.com\") at most 60 per minute, filesystem.read(\"secrets\") only to console",
        )
        .expect("a grant");
        assert_eq!(grant.capabilities.len(), 3);
        assert_eq!(
            grant.spelling(),
            "console, network.http(\"api.example.com\") at most 60 per minute, filesystem.read(\"secrets\") only to console"
        );
        assert_eq!(Grant::parse("").expect("empty").capabilities, Vec::new());
        assert!(Grant::parse("console(\"x\")")
            .unwrap_err()
            .contains("a scope, which it does not take"));
        assert!(Grant::parse("teleport")
            .unwrap_err()
            .contains("not a capability"));
        assert!(Grant::parse("console at most")
            .unwrap_err()
            .starts_with("the grant does not parse:\n"));
    }

    #[test]
    fn a_grant_file_names_the_memory_in_words() {
        let grant = Grant::from_json("{\"grant\": \"console\", \"memory\": \"256 megabytes\"}")
            .expect("a grant");
        assert_eq!(grant.memory, Some(256 << 20));
        assert_eq!(parse_memory("1 gigabyte"), Ok(1 << 30));
        assert_eq!(parse_memory("4096 bytes"), Ok(4096));
        assert!(parse_memory("lots").is_err());
        assert!(Grant::from_json("{\"memory\": 5}").is_err());
        assert!(Grant::from_json("{\"grants\": \"console\"}")
            .unwrap_err()
            .contains("no field `grants`"));
        assert_eq!(bytes_text(256 << 20), "256 megabytes");
        assert_eq!(bytes_text((3 << 20) / 2), "1.5 megabytes");
        assert_eq!(bytes_text(640), "640 bytes");
    }
}
