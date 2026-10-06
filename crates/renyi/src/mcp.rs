//! `renyi mcp [path]`: the toolchain served to an agent host over standard
//! input and output, following the Model Context Protocol (decision O5;
//! design document 05, section 7). One JSON-RPC message per line in, one per
//! line out; anything the server has to say for itself goes to standard
//! error.
//!
//! The server speaks both eras of the protocol (decision T1). A request that
//! carries `io.modelcontextprotocol/protocolVersion` in its `_meta` is
//! answered statelessly as revision 2026-07-28 says (`server/discover`, a
//! `resultType` and the server's identity in every result, an
//! `UnsupportedProtocolVersion` error for any other version), and an
//! `initialize` request opens the handshake of the revisions up to
//! 2025-11-25. The tools are the same in both.
//!
//! The project map is rebuilt from the served directory whenever a source
//! file's text differs from the one the map was built from (decision T4).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

use renyi_index::{Definition, Index};
use renyi_syntax::diagnostics::{render_json, render_text};
use renyi_syntax::{format, SourceFile};
use renyi_vm::grant::parse_capability;
use renyi_vm::natives::json::{read_json, write_json, Json};
use renyi_vm::{Narrowing, Options, RunOutcome};

use crate::{
    compile_sources, diagnose, load_recording, parse_budget, read_source, scoped, toolchain,
    CompileError,
};

/// The revision this server answers per-request metadata with.
const MODERN: &str = "2026-07-28";
/// The handshake revisions this server answers `initialize` with, the
/// latest first.
const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const VERSION_KEY: &str = "io.modelcontextprotocol/protocolVersion";
const CLIENT_CAPABILITIES_KEY: &str = "io.modelcontextprotocol/clientCapabilities";
const SERVER_INFO_KEY: &str = "io.modelcontextprotocol/serverInfo";
/// How long a client may keep the tool list and the discovery result: a
/// day, since neither changes while the server runs.
const CACHE_TTL_MS: &str = "86400000";
const CHEAT_SHEET: &str = include_str!("../../../docs/cheatsheet.md");
const INSTRUCTIONS: &str = "The Renyi toolchain. Read cheat_sheet before writing Renyi. Look library \
names up with library_lookup instead of guessing them. project_map, definition and effects describe \
the served project; check and format close the loop on source text; run and run_tests execute \
programs on the VM under the grants they declare; diff tells what changed since a saved map or a git \
revision and what each change reaches.";

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const UNSUPPORTED_VERSION: i64 = -32022;

const TOOL_NAMES: &[&str] = &[
    "cheat_sheet",
    "library_lookup",
    "project_map",
    "definition",
    "effects",
    "check",
    "format",
    "run",
    "run_tests",
    "diff",
];

/// `renyi mcp [path]`: serve the directory (the current one by default)
/// until standard input closes.
pub fn serve(args: &[String]) -> ExitCode {
    let root = match args {
        [] => ".".to_string(),
        [path] if !path.starts_with("--") => path.clone(),
        _ => {
            eprintln!("{}", crate::USAGE);
            return ExitCode::FAILURE;
        }
    };
    if !Path::new(&root).is_dir() {
        eprintln!("renyi: {root} is not a directory");
        return ExitCode::FAILURE;
    }
    if let Err(error) = std::env::set_current_dir(&root) {
        eprintln!("renyi: cannot enter {root}: {error}");
        return ExitCode::FAILURE;
    }
    let served = std::env::current_dir()
        .map(|path| path.display().to_string())
        .unwrap_or(root);
    eprintln!("renyi mcp: serving {served}");
    let mut server = Server::new();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = server.handle(&line) {
            let mut out = stdout.lock();
            if writeln!(out, "{response}")
                .and_then(|()| out.flush())
                .is_err()
            {
                break;
            }
        }
    }
    ExitCode::SUCCESS
}

// ---------------------------------------------------------------- messages

struct RpcError {
    code: i64,
    message: String,
    data: Option<Json>,
}

fn rpc_error(code: i64, message: impl Into<String>) -> RpcError {
    RpcError {
        code,
        message: message.into(),
        data: None,
    }
}

type Fields = Vec<(&'static str, Json)>;

struct Server {
    map: Option<Map>,
    library: Vec<LibraryEntry>,
}

/// The project map with the texts it was built from.
struct Map {
    files: Vec<SourceFile>,
    /// Every file in canonical layout: the text the map's lines refer to.
    canonical: Vec<String>,
    index: Index,
}

impl Server {
    fn new() -> Server {
        Server {
            map: None,
            library: library_entries(),
        }
    }

    /// One message in, at most one message out (a notification has no
    /// answer).
    fn handle(&mut self, line: &str) -> Option<String> {
        let message = match read_json(line) {
            Ok(json) => json,
            Err((detail, _)) => {
                let error = error_message(
                    &Json::Null,
                    PARSE_ERROR,
                    &format!("parse error: {detail}"),
                    None,
                );
                return Some(render_line(&error));
            }
        };
        let id = field(&message, "id").cloned();
        let Some(Json::Text(method)) = field(&message, "method") else {
            let id = id?;
            return Some(render_line(&error_message(
                &id,
                INVALID_REQUEST,
                "the message has no method",
                None,
            )));
        };
        let method = method.clone();
        let params = field(&message, "params")
            .cloned()
            .unwrap_or_else(|| Json::Object(Vec::new()));
        // a notification: nothing to answer, whatever it says
        let id = id?;
        let meta = field(&params, "_meta");
        let versioned = meta.and_then(|meta| field(meta, VERSION_KEY)).is_some();
        let modern = method != "initialize" && (versioned || method == "server/discover");
        let outcome = match meta.filter(|_| versioned && modern) {
            Some(meta) => check_meta(meta).and_then(|()| self.dispatch(&method, &params, modern)),
            None => self.dispatch(&method, &params, modern),
        };
        let response = match outcome {
            Ok(mut fields) => {
                if modern {
                    fields.insert(0, ("resultType", text("complete")));
                    fields.push(("_meta", obj(vec![(SERVER_INFO_KEY, server_info())])));
                }
                result_message(&id, obj(fields))
            }
            Err(error) => error_message(&id, error.code, &error.message, error.data),
        };
        Some(render_line(&response))
    }

    fn dispatch(&mut self, method: &str, params: &Json, modern: bool) -> Result<Fields, RpcError> {
        match method {
            "initialize" => {
                let requested = field(params, "protocolVersion")
                    .and_then(as_text)
                    .unwrap_or_default();
                let version = if LEGACY.contains(&requested.as_str()) {
                    requested
                } else {
                    LEGACY[0].to_string()
                };
                Ok(vec![
                    ("protocolVersion", text(&version)),
                    ("capabilities", obj(vec![("tools", obj(Vec::new()))])),
                    ("serverInfo", server_info()),
                    ("instructions", text(INSTRUCTIONS)),
                ])
            }
            "ping" => Ok(Vec::new()),
            "server/discover" => Ok(vec![
                ("supportedVersions", Json::Array(vec![text(MODERN)])),
                ("capabilities", obj(vec![("tools", obj(Vec::new()))])),
                ("instructions", text(INSTRUCTIONS)),
                ("ttlMs", Json::Number(CACHE_TTL_MS.to_string())),
                ("cacheScope", text("public")),
            ]),
            "tools/list" => {
                let mut fields = vec![(
                    "tools",
                    Json::Array(TOOL_NAMES.iter().map(|name| tool_json(name)).collect()),
                )];
                if modern {
                    fields.push(("ttlMs", Json::Number(CACHE_TTL_MS.to_string())));
                    fields.push(("cacheScope", text("public")));
                }
                Ok(fields)
            }
            "tools/call" => {
                let name = field(params, "name").and_then(as_text).ok_or_else(|| {
                    rpc_error(INVALID_PARAMS, "tools/call needs the name of a tool")
                })?;
                if !TOOL_NAMES.contains(&name.as_str()) {
                    return Err(rpc_error(INVALID_PARAMS, format!("Unknown tool: {name}")));
                }
                let arguments = field(params, "arguments")
                    .cloned()
                    .unwrap_or_else(|| Json::Object(Vec::new()));
                let (content, is_error) = match self.call_tool(&name, &arguments) {
                    Ok(content) => (content, false),
                    Err(content) => (content, true),
                };
                Ok(vec![
                    (
                        "content",
                        Json::Array(vec![obj(vec![
                            ("type", text("text")),
                            ("text", text(&content)),
                        ])]),
                    ),
                    ("isError", Json::Boolean(is_error)),
                ])
            }
            other => Err(rpc_error(
                METHOD_NOT_FOUND,
                format!("Method not found: {other}"),
            )),
        }
    }

    /// The text a tool answers with; an `Err` is a tool execution error
    /// (`isError`), which the model can act on, not a protocol error.
    fn call_tool(&mut self, name: &str, arguments: &Json) -> Result<String, String> {
        match name {
            "cheat_sheet" => Ok(CHEAT_SHEET.to_string()),
            "library_lookup" => {
                let query = required_text(arguments, "query")?;
                Ok(self.lookup(&query))
            }
            "project_map" => {
                let module = optional_text(arguments, "module")?;
                let json = flag(arguments, "json")?;
                let map = Map::refresh(&mut self.map)?;
                let index = match module {
                    Some(module) => {
                        if !map.index.modules.iter().any(|m| m.name == module) {
                            return Err(format!(
                                "the served project has no module named `{module}`"
                            ));
                        }
                        Index {
                            header: map.index.header.clone(),
                            modules: map
                                .index
                                .modules
                                .iter()
                                .filter(|m| m.name == module)
                                .cloned()
                                .collect(),
                            definitions: map
                                .index
                                .definitions
                                .iter()
                                .filter(|d| d.module == module)
                                .cloned()
                                .collect(),
                        }
                    }
                    None => map.index.clone(),
                };
                Ok(if json {
                    renyi_index::to_json(&index)
                } else {
                    renyi_index::to_text(&index)
                })
            }
            "definition" => {
                let name = required_text(arguments, "name")?;
                let map = Map::refresh(&mut self.map)?;
                let definition = find_definition(&map.index, &name)?;
                let file = map
                    .files
                    .iter()
                    .position(|file| display_path(&file.name) == definition.file)
                    .ok_or_else(|| {
                        format!(
                            "the map names a file the project lacks: {}",
                            definition.file
                        )
                    })?;
                let source: Vec<&str> = map.canonical[file]
                    .lines()
                    .skip(definition.line.saturating_sub(1))
                    .take(definition.end_line + 1 - definition.line)
                    .collect();
                Ok(format!(
                    "{}source ({}:{}-{}):\n{}\n",
                    renyi_index::definition_json(definition).render(),
                    definition.file,
                    definition.line,
                    definition.end_line,
                    source.join("\n")
                ))
            }
            "effects" => {
                let name = required_text(arguments, "name")?;
                let map = Map::refresh(&mut self.map)?;
                let definition = find_definition(&map.index, &name)?;
                let by_name: HashMap<String, &Definition> = map
                    .index
                    .definitions
                    .iter()
                    .map(|d| (d.qualified(), d))
                    .collect();
                let mut out = String::new();
                let mut shown = HashSet::new();
                effects_tree(&self.library, &by_name, definition, 0, &mut shown, &mut out);
                Ok(out)
            }
            "check" => check_tool(arguments),
            "format" => format_tool(arguments),
            "run" => run_tool(arguments),
            "run_tests" => run_tests_tool(arguments),
            "diff" => {
                let base = required_text(arguments, "base")?;
                let json = flag(arguments, "json")?;
                let map = Map::refresh(&mut self.map)?;
                let old = crate::maps::load_base(&base, Path::new("."), &toolchain())?;
                let diff = renyi_index::diff(&old, &map.index);
                Ok(if json {
                    renyi_index::diff_json(&diff)
                } else {
                    renyi_index::render_diff(&diff)
                })
            }
            other => Err(format!("unknown tool {other}")),
        }
    }

    /// Library declarations whose module, head or purpose contain every
    /// word of the query, the ones named by a word first.
    fn lookup(&self, query: &str) -> String {
        let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
        if words.is_empty() {
            return "give a name or a few words to look for".to_string();
        }
        let mut hits: Vec<(usize, &LibraryEntry)> = self
            .library
            .iter()
            .filter_map(|entry| {
                let haystack = format!(
                    "{} {} {}",
                    entry.module,
                    entry.head,
                    entry.purpose.as_deref().unwrap_or("")
                )
                .to_lowercase();
                if !words.iter().all(|word| haystack.contains(word.as_str())) {
                    return None;
                }
                let name = entry.name.to_lowercase();
                let rank = if words.contains(&name) {
                    0
                } else if words.iter().any(|word| name.contains(word.as_str())) {
                    1
                } else {
                    2
                };
                Some((rank, entry))
            })
            .collect();
        hits.sort_by_key(|(rank, _)| *rank);
        if hits.is_empty() {
            return format!("nothing in the standard library matches `{query}`");
        }
        const LIMIT: usize = 25;
        let mut out = String::new();
        for (_, entry) in hits.iter().take(LIMIT) {
            out.push_str(&format!("{}: {}\n", entry.module, entry.head));
            if let Some(purpose) = &entry.purpose {
                out.push_str(&format!("  purpose: {purpose}\n"));
            }
        }
        if hits.len() > LIMIT {
            out.push_str(&format!(
                "... and {} more; narrow the query\n",
                hits.len() - LIMIT
            ));
        }
        out
    }
}

impl Map {
    /// The map of the served directory, rebuilt when any file changed.
    fn refresh(slot: &mut Option<Map>) -> Result<&Map, String> {
        let files = renyi_index::load_project(Path::new("."))?;
        let fresh = slot.as_ref().is_some_and(|map| {
            map.files.len() == files.len()
                && map
                    .files
                    .iter()
                    .zip(&files)
                    .all(|(old, new)| old.name == new.name && old.text == new.text)
        });
        if !fresh {
            let header = renyi_index::Header {
                project: renyi_index::project_name(Path::new(".")),
                revision: renyi_index::git_revision(Path::new(".")),
                toolchain: toolchain(),
            };
            let index = renyi_index::index_files(&files, header);
            let canonical = files
                .iter()
                .map(|file| format(file).unwrap_or_else(|_| file.text.clone()))
                .collect();
            *slot = Some(Map {
                files,
                canonical,
                index,
            });
        }
        Ok(slot.as_ref().expect("the map was just built"))
    }
}

/// The definition, its effects and failures, and everything it calls,
/// indented by depth; a definition shown before is not expanded again.
fn effects_tree(
    library: &[LibraryEntry],
    by_name: &HashMap<String, &Definition>,
    definition: &Definition,
    depth: usize,
    shown: &mut HashSet<String>,
    out: &mut String,
) {
    let indent = "  ".repeat(depth);
    let qualified = definition.qualified();
    let again = !shown.insert(qualified.clone());
    out.push_str(&format!(
        "{indent}{} {qualified}  effects: {} -> {}  fails: {} -> {}{}\n",
        definition.kind.name(),
        list_or_none(&definition.effects_declared),
        list_or_none(&definition.effects_transitive),
        list_or_none(&definition.fails_declared),
        list_or_none(&definition.fails_transitive),
        if again { "  (shown above)" } else { "" }
    ));
    if again {
        return;
    }
    for call in &definition.calls {
        match by_name.get(call) {
            Some(callee) => effects_tree(library, by_name, callee, depth + 1, shown, out),
            None => match library_entry(library, call) {
                Some(entry) => {
                    out.push_str(&format!("{indent}  {call}  library: {}\n", entry.head))
                }
                None if call.starts_with("std.") => {
                    out.push_str(&format!("{indent}  {call}  library\n"));
                }
                None => out.push_str(&format!(
                    "{indent}  {call}  ability method: every implementation is reached\n"
                )),
            },
        }
    }
}

/// The library declaration a qualified call names (`std.http.get`,
/// `std.prelude.Text.trim`).
fn library_entry<'a>(library: &'a [LibraryEntry], call: &str) -> Option<&'a LibraryEntry> {
    let (prefix, name) = call.rsplit_once('.')?;
    let (module, method_of) = match prefix.rsplit_once('.') {
        Some((module, last)) if last.starts_with(|c: char| c.is_ascii_uppercase()) => {
            (module, Some(last.to_string()))
        }
        _ => (prefix, None),
    };
    library
        .iter()
        .find(|entry| entry.module == module && entry.name == name && entry.method_of == method_of)
}

/// A modern request must name a version this server speaks and the
/// client's capabilities.
fn check_meta(meta: &Json) -> Result<(), RpcError> {
    let requested = field(meta, VERSION_KEY)
        .and_then(as_text)
        .unwrap_or_default();
    if requested != MODERN {
        return Err(RpcError {
            code: UNSUPPORTED_VERSION,
            message: "Unsupported protocol version".to_string(),
            data: Some(obj(vec![
                ("supported", Json::Array(vec![text(MODERN)])),
                ("requested", text(&requested)),
            ])),
        });
    }
    if field(meta, CLIENT_CAPABILITIES_KEY).is_none() {
        return Err(rpc_error(
            INVALID_PARAMS,
            format!("the request lacks {CLIENT_CAPABILITIES_KEY} in its _meta"),
        ));
    }
    Ok(())
}

/// A definition by its qualified name, or by its bare name when that is
/// unique in the project.
fn find_definition<'a>(index: &'a Index, name: &str) -> Result<&'a Definition, String> {
    if let Some(definition) = index.definitions.iter().find(|d| d.qualified() == name) {
        return Ok(definition);
    }
    let bare: Vec<&Definition> = index
        .definitions
        .iter()
        .filter(|d| d.name == name)
        .collect();
    match bare.as_slice() {
        [definition] => Ok(definition),
        [] => {
            let similar: Vec<String> = index
                .definitions
                .iter()
                .filter(|d| d.qualified().contains(name))
                .map(Definition::qualified)
                .take(10)
                .collect();
            Err(if similar.is_empty() {
                format!("no definition named `{name}`; names are qualified: module.name, module.Type.method")
            } else {
                format!(
                    "no definition named `{name}`; did you mean {}?",
                    similar.join(", ")
                )
            })
        }
        several => Err(format!(
            "`{name}` names {} definitions; qualify it: {}",
            several.len(),
            several
                .iter()
                .map(|d| d.qualified())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn display_path(name: &str) -> String {
    name.replace('\\', "/")
}

fn list_or_none(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join(", ")
    }
}

// ------------------------------------------------------------------- tools

fn tool_json(name: &str) -> Json {
    let (description, schema) = match name {
        "cheat_sheet" => (
            "The whole Renyi language on one page (docs/cheatsheet.md): syntax, types, effects and the \
             library. Read it before writing Renyi.",
            schema(Vec::new(), &[]),
        ),
        "library_lookup" => (
            "Find standard library declarations by name or by a few words (case-insensitive; every word \
             must occur in the module name, the signature or the purpose). Each match is given with \
             its signature, effects and purpose. Use it before calling a library function, so that \
             names and arguments are not invented.",
            schema(
                vec![("query", string_property("a name or a few words, such as `get_with` or `read file`"))],
                &["query"],
            ),
        ),
        "project_map" => (
            "The project map of the served directory: one record per module and per definition with \
             signature, purpose, effects, failures, edges, metrics and content hash. `module` narrows \
             it to one module; `json` gives the JSON shape of `renyi index --json` instead of the one-line-per-definition text.",
            schema(
                vec![
                    ("module", string_property("a module name, to show only that module")),
                    ("json", boolean_property("true for the JSON shape; the text form by default")),
                ],
                &[],
            ),
        ),
        "definition" => (
            "One definition's record and its source text, by qualified name: `module.name`, \
             `module.Type.method`, `module.Ability for Target`, `module.test:name`; a bare name works \
             when it is unique in the project.",
            schema(vec![("name", string_property("the qualified name"))], &["name"]),
        ),
        "effects" => (
            "The transitive effect and failure graph under a definition: every project definition it \
             reaches, each with its declared and reachable effects and failure types, and the library \
             primitives at the leaves with their declarations.",
            schema(vec![("name", string_property("the qualified name"))], &["name"]),
        ),
        "check" => (
            "Parse, type-check and effect-check a program and report its diagnostics as `renyi check \
             --json` prints them (an empty list is clean). Give `path`, a file under the served \
             directory, or `source`, the text of a module (with `path` naming the file it would be, so \
             that its imports resolve).",
            schema(
                vec![
                    ("source", string_property("the text of a module")),
                    ("path", string_property("a file under the served directory")),
                ],
                &[],
            ),
        ),
        "format" => (
            "The canonical layout of Renyi source text, what `renyi format` writes; the parse \
             diagnostics when it does not parse.",
            schema(vec![("source", string_property("the text of a module"))], &["source"]),
        ),
        "run" => (
            "Check a program and run its `main` on the VM with the arguments, under the grant it \
             declares narrowed by the options (the same as `renyi run`). Answers with what the program \
             printed and how it ended. Its effects really happen unless `replay` names a recording.",
            schema(
                vec![
                    ("path", string_property("the program, a file under the served directory")),
                    ("arguments", strings_property("the command-line arguments")),
                    ("deny", strings_property("capabilities to refuse, such as `network`; the run does not start when any function needs one")),
                    ("allow_host", strings_property("hosts `network.http` is narrowed to")),
                    ("allow_read", strings_property("path prefixes `filesystem.read` is narrowed to")),
                    ("allow_write", strings_property("path prefixes `filesystem.write` is narrowed to")),
                    ("at_most", strings_property("budgets to add, `<capability>=<count>/<unit>`, such as `network.http=10/run`")),
                    ("replay", string_property("a recording to answer every effect from; nothing is written or sent")),
                    ("explain", boolean_property("narrate the run: purposes, arguments, results and effects")),
                ],
                &["path"],
            ),
        ),
        "run_tests" => (
            "Run every `example:` line and `test` block of a program, `replays` tests answered from \
             their recordings (the same as `renyi test`), and answer with the report.",
            schema(
                vec![
                    ("path", string_property("the program, a file under the served directory")),
                    ("strict", boolean_property("a recorded call a test never reaches fails it")),
                    ("explain", boolean_property("narrate each test")),
                ],
                &["path"],
            ),
        ),
        "diff" => (
            "What changed in the served project since a base: a saved map (a file `renyi index --json` \
             or project_map with `json` wrote) or a git revision such as `HEAD` or `v1.2.0`. One entry \
             per changed definition with the changes (added, removed, renamed, signature, visibility, \
             effects, failures, body), the definitions each change reaches, and the version bump the \
             changes force (the same as `renyi index --diff`).",
            schema(
                vec![
                    ("base", string_property("a saved map file under the served directory, or a git revision")),
                    ("json", boolean_property("true for the JSON shape; the text form by default")),
                ],
                &["base"],
            ),
        ),
        _ => ("", schema(Vec::new(), &[])),
    };
    obj(vec![
        ("name", text(name)),
        ("description", text(description)),
        ("inputSchema", schema),
    ])
}

fn schema(properties: Vec<(&str, Json)>, required: &[&str]) -> Json {
    let mut fields = vec![("type", text("object")), ("properties", obj(properties))];
    if !required.is_empty() {
        fields.push((
            "required",
            Json::Array(required.iter().map(|name| text(name)).collect()),
        ));
    }
    fields.push(("additionalProperties", Json::Boolean(false)));
    obj(fields)
}

fn string_property(description: &str) -> Json {
    obj(vec![
        ("type", text("string")),
        ("description", text(description)),
    ])
}

fn boolean_property(description: &str) -> Json {
    obj(vec![
        ("type", text("boolean")),
        ("description", text(description)),
    ])
}

fn strings_property(description: &str) -> Json {
    obj(vec![
        ("type", text("array")),
        ("items", obj(vec![("type", text("string"))])),
        ("description", text(description)),
    ])
}

fn check_tool(arguments: &Json) -> Result<String, String> {
    let source = optional_text(arguments, "source")?;
    let path = optional_text(arguments, "path")?;
    let file = match (source, path) {
        (Some(source), path) => {
            let name = path.unwrap_or_else(|| module_file_name(&source));
            SourceFile::new(name, source)
        }
        (None, Some(path)) => read_source(&path)?,
        (None, None) => {
            return Err("check needs `source` (the text of a module) or `path` (a file under the served directory)".to_string())
        }
    };
    Ok(render_json(&file, &diagnose(&file)))
}

/// The file a module's text would live in (decision G3), for a source
/// checked without a path: `module billing.tax` is `billing/tax.ry`.
fn module_file_name(source: &str) -> String {
    let name = source
        .lines()
        .find_map(|line| line.strip_prefix("module "))
        .map(|rest| rest.trim().replace('.', "/"))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "source".to_string());
    format!("{name}.ry")
}

fn format_tool(arguments: &Json) -> Result<String, String> {
    let source = required_text(arguments, "source")?;
    let file = SourceFile::new("source.ry", source);
    format(&file).map_err(|diagnostics| render_text(&file, &diagnostics))
}

/// Standard output or error of a run, kept for the answer.
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
        String::from_utf8_lossy(&self.0.borrow()).into_owned()
    }
}

fn compiled(path: &str) -> Result<crate::Compiled, String> {
    compile_sources(path).map_err(|error| match error {
        CompileError::Read(message) => message,
        CompileError::Diagnostics(text) => text,
    })
}

fn run_tool(arguments: &Json) -> Result<String, String> {
    let path = required_text(arguments, "path")?;
    let compiled = compiled(&path)?;
    let mut narrowing = Narrowing::default();
    for capability in optional_strings(arguments, "deny")? {
        narrowing.deny.push(parse_capability(&capability)?);
    }
    for host in optional_strings(arguments, "allow_host")? {
        narrowing.allow.push(scoped(&["network", "http"], &host));
    }
    for prefix in optional_strings(arguments, "allow_read")? {
        narrowing
            .allow
            .push(scoped(&["filesystem", "read"], &prefix));
    }
    for prefix in optional_strings(arguments, "allow_write")? {
        narrowing
            .allow
            .push(scoped(&["filesystem", "write"], &prefix));
    }
    for budget in optional_strings(arguments, "at_most")? {
        narrowing.budgets.push(parse_budget(&budget)?);
    }
    for denied in &narrowing.deny {
        let functions = renyi_vm::denied_functions(&compiled.program, denied);
        if !functions.is_empty() {
            return Err(format!(
                "`{}` is denied, but these functions need it: {}",
                denied.spelling(),
                functions.join(", ")
            ));
        }
    }
    let replay = match optional_text(arguments, "replay")? {
        Some(file) => Some(load_recording(&file)?),
        None => None,
    };
    let stdout = Capture::default();
    let stderr = Capture::default();
    let options = Options {
        arguments: optional_strings(arguments, "arguments")?,
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(stderr.clone()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        narrowing,
        replay,
        explain: flag(arguments, "explain")?,
        ..Options::default()
    };
    let run = renyi_vm::run_program(&compiled.program, options);
    let mut out = compiled.diagnostics;
    out.push_str(&stdout.text());
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    let narration = stderr.text();
    if !narration.is_empty() {
        out.push_str("--- standard error ---\n");
        out.push_str(&narration);
        if !narration.ends_with('\n') {
            out.push('\n');
        }
    }
    if !run.unused.is_empty() {
        out.push_str(&format!(
            "{} recorded call(s) not reached: {}\n",
            run.unused.len(),
            run.unused.join("; ")
        ));
    }
    out.push_str(&format!(
        "--- {} ---\n",
        renyi_vm::describe_outcome(&run.outcome)
    ));
    if matches!(run.outcome, RunOutcome::Finished | RunOutcome::Exited(0)) {
        Ok(out)
    } else {
        Err(out)
    }
}

fn run_tests_tool(arguments: &Json) -> Result<String, String> {
    let path = required_text(arguments, "path")?;
    let compiled = compiled(&path)?;
    let stdout = Capture::default();
    let stderr = Capture::default();
    let options = Options {
        stdout: Box::new(stdout.clone()),
        stderr: Box::new(stderr.clone()),
        stdin: Box::new(std::io::Cursor::new(Vec::new())),
        strict: flag(arguments, "strict")?,
        explain: flag(arguments, "explain")?,
        ..Options::default()
    };
    let report = renyi_vm::run_tests(&compiled.program, options);
    let mut out = compiled.diagnostics;
    let printed = stdout.text();
    if !printed.is_empty() {
        out.push_str("--- output ---\n");
        out.push_str(&printed);
        if !printed.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(&report.render());
    let narration = stderr.text();
    if !narration.is_empty() {
        out.push_str("--- standard error ---\n");
        out.push_str(&narration);
    }
    if report.failed() == 0 {
        Ok(out)
    } else {
        Err(out)
    }
}

// ----------------------------------------------------------------- library

/// One declaration of the standard library, as `library_lookup` shows it.
struct LibraryEntry {
    module: String,
    name: String,
    /// The receiver type of a method (`self: Text`).
    method_of: Option<String>,
    /// The head line with its signature clauses joined on one line.
    head: String,
    purpose: Option<String>,
}

/// Every top-level declaration of `library/std`, in library order.
fn library_entries() -> Vec<LibraryEntry> {
    let mut entries = Vec::new();
    for (module, source) in renyi_check::LIBRARY {
        let lines: Vec<&str> = source.lines().collect();
        let mut index = 0;
        while index < lines.len() {
            let Some(rest) = declaration_head(lines[index]) else {
                index += 1;
                continue;
            };
            let mut head = lines[index].trim_end().to_string();
            let mut next = index + 1;
            while next < lines.len() && is_clause_line(lines[next]) {
                head.push(' ');
                head.push_str(lines[next].trim());
                next += 1;
            }
            let mut purpose = None;
            let mut cursor = next;
            while cursor < lines.len()
                && (lines[cursor].starts_with("  ") || lines[cursor].trim().is_empty())
            {
                if let Some(text) = lines[cursor].trim().strip_prefix("purpose:") {
                    purpose = Some(text.trim().to_string());
                    break;
                }
                cursor += 1;
            }
            let name = rest.split(['(', ' ']).next().unwrap_or("").to_string();
            let method_of = head
                .split_once("(self: ")
                .and_then(|(_, after)| after.split([',', ')']).next())
                .and_then(|receiver| receiver.split_whitespace().next())
                .map(str::to_string);
            entries.push(LibraryEntry {
                module: module.to_string(),
                name,
                method_of,
                head,
                purpose,
            });
            index = next;
        }
    }
    entries
}

/// The text after the keyword of a top-level declaration head.
fn declaration_head(line: &str) -> Option<&str> {
    if line.starts_with(' ') {
        return None;
    }
    let rest = line.strip_prefix("public ").unwrap_or(line);
    ["function", "type", "ability", "constant"]
        .iter()
        .find_map(|keyword| {
            rest.strip_prefix(keyword)
                .and_then(|after| after.strip_prefix(' '))
        })
}

fn is_clause_line(line: &str) -> bool {
    line.starts_with("  ")
        && ["returns ", "or fails with ", "needs ", "for any "]
            .iter()
            .any(|clause| line.trim_start().starts_with(clause))
}

// -------------------------------------------------------------------- json

fn obj(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
    )
}

fn text(value: &str) -> Json {
    Json::Text(value.to_string())
}

fn field<'a>(json: &'a Json, key: &str) -> Option<&'a Json> {
    match json {
        Json::Object(fields) => fields.iter().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

fn as_text(json: &Json) -> Option<String> {
    match json {
        Json::Text(text) => Some(text.clone()),
        _ => None,
    }
}

fn required_text(arguments: &Json, key: &str) -> Result<String, String> {
    optional_text(arguments, key)?.ok_or_else(|| format!("`{key}` is required"))
}

fn optional_text(arguments: &Json, key: &str) -> Result<Option<String>, String> {
    match field(arguments, key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Text(text)) => Ok(Some(text.clone())),
        Some(_) => Err(format!("`{key}` must be a string")),
    }
}

fn optional_strings(arguments: &Json, key: &str) -> Result<Vec<String>, String> {
    match field(arguments, key) {
        None | Some(Json::Null) => Ok(Vec::new()),
        Some(Json::Array(items)) => items
            .iter()
            .map(|item| as_text(item).ok_or_else(|| format!("`{key}` must be a list of strings")))
            .collect(),
        Some(_) => Err(format!("`{key}` must be a list of strings")),
    }
}

fn flag(arguments: &Json, key: &str) -> Result<bool, String> {
    match field(arguments, key) {
        None | Some(Json::Null) => Ok(false),
        Some(Json::Boolean(value)) => Ok(*value),
        Some(_) => Err(format!("`{key}` must be true or false")),
    }
}

/// One line, no line breaks inside: the stdio framing.
fn render_line(json: &Json) -> String {
    let mut out = String::new();
    write_json(json, &mut out, None, 0);
    out
}

fn server_info() -> Json {
    obj(vec![
        ("name", text("renyi")),
        ("version", text(env!("CARGO_PKG_VERSION"))),
    ])
}

fn result_message(id: &Json, result: Json) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", id.clone()),
        ("result", result),
    ])
}

fn error_message(id: &Json, code: i64, message: &str, data: Option<Json>) -> Json {
    let mut error = vec![
        ("code", Json::Number(code.to_string())),
        ("message", text(message)),
    ];
    if let Some(data) = data {
        error.push(("data", data));
    }
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", id.clone()),
        ("error", obj(error)),
    ])
}
