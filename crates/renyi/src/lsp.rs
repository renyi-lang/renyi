//! `renyi lsp`: the language server (decision AN2) over standard input
//! and output, following the Language Server Protocol: one `Workspace` of
//! `renyi_workspace` over the workspace folder, the files as written
//! (what `renyi check` reads), the editor's unsaved buffers as overlays.
//! Diagnostics are published for every file of the folder after every
//! change, as `renyi check` reports them, the layout's included; hover
//! gives the signature and the purpose of what is under the cursor;
//! definition goes to its declaration; document symbols list a file's
//! items. Positions count UTF-16 code units, the protocol's default. The
//! base protocol frames each message with a `Content-Length` header.

mod describe;

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::process::ExitCode;

use renyi_check::ModuleId;
use renyi_syntax::layout::check_layout;
use renyi_syntax::{Diagnostic, Severity, SourceFile};
use renyi_vm::natives::json::{read_json, write_json, Json};
use renyi_workspace::Workspace;

use crate::library;
use crate::mcp::{as_text, field, obj, text};
use describe::{offset_of, position_of, symbols_of, what_is_at, Symbol};

const PARSE_ERROR: i64 = -32700;
const INVALID_PARAMS: i64 = -32602;
const METHOD_NOT_FOUND: i64 = -32601;
const NOT_INITIALIZED: i64 = -32002;

/// `renyi lsp`: serve an editor until it says `exit` or closes the input.
pub fn serve(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        eprintln!("{}", crate::USAGE);
        return ExitCode::FAILURE;
    }
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    let mut server = Server::new();
    loop {
        let body = match read_message(&mut input) {
            Ok(Some(body)) => body,
            Ok(None) => break,
            Err(detail) => {
                eprintln!("renyi lsp: {detail}");
                break;
            }
        };
        for message in server.handle(&body) {
            if write_message(&mut output, &message).is_err() {
                return ExitCode::FAILURE;
            }
        }
        if server.exited {
            break;
        }
    }
    // the protocol: 0 after a shutdown request, 1 otherwise
    if server.shutdown {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

// --------------------------------------------------------------- framing

/// One message: `Content-Length: n`, other headers, an empty line, then
/// `n` bytes of JSON. None at the end of the input.
fn read_message(input: &mut impl BufRead) -> Result<Option<String>, String> {
    let mut length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let read = input
            .read_line(&mut line)
            .map_err(|error| format!("cannot read the input: {error}"))?;
        if read == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if length.is_some() {
                break;
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = Some(
                value
                    .trim()
                    .parse()
                    .map_err(|_| format!("a Content-Length that is not a number: {value}"))?,
            );
        }
    }
    let length = length.ok_or_else(|| "a message without Content-Length".to_string())?;
    let mut body = vec![0; length];
    input
        .read_exact(&mut body)
        .map_err(|error| format!("the input ended inside a message: {error}"))?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|_| "a message that is not UTF-8".to_string())
}

fn write_message(output: &mut impl Write, message: &Json) -> std::io::Result<()> {
    let mut body = String::new();
    write_json(message, &mut body, None, 0);
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

// ---------------------------------------------------------------- server

struct Server {
    /// The workspace, from `initialize` on.
    workspace: Option<Workspace>,
    /// The library's declaration files, for what hover shows of a
    /// library definition.
    library: Vec<SourceFile>,
    /// The open documents: the file's path as the URI gives it, and the
    /// URI the editor spells it with.
    open: BTreeMap<String, String>,
    /// The diagnostics last published, by URI, so that only a change is
    /// sent again.
    published: BTreeMap<String, Json>,
    shutdown: bool,
    exited: bool,
}

type Response = Result<Json, (i64, String)>;

impl Server {
    fn new() -> Server {
        Server {
            workspace: None,
            library: library()
                .modules()
                .map(|(name, source)| SourceFile::new(name, source))
                .collect(),
            open: BTreeMap::new(),
            published: BTreeMap::new(),
            shutdown: false,
            exited: false,
        }
    }

    /// One message in; the response, when it was a request, after any
    /// notifications it caused.
    fn handle(&mut self, body: &str) -> Vec<Json> {
        let mut out = Vec::new();
        let message = match read_json(body) {
            Ok(message) => message,
            Err((detail, _)) => {
                out.push(error_message(
                    &Json::Null,
                    PARSE_ERROR,
                    &format!("parse error: {detail}"),
                ));
                return out;
            }
        };
        let id = field(&message, "id").cloned();
        let method = field(&message, "method")
            .and_then(as_text)
            .unwrap_or_default();
        let params = field(&message, "params").cloned().unwrap_or(Json::Null);
        match id {
            Some(id) => {
                let response = match self.request(&method, &params, &mut out) {
                    Ok(result) => result_message(&id, result),
                    Err((code, message)) => error_message(&id, code, &message),
                };
                out.push(response);
            }
            None => self.notification(&method, &params, &mut out),
        }
        out
    }

    fn request(&mut self, method: &str, params: &Json, out: &mut Vec<Json>) -> Response {
        match method {
            "initialize" => self.initialize(params),
            "shutdown" => {
                self.shutdown = true;
                Ok(Json::Null)
            }
            _ if self.workspace.is_none() => Err((
                NOT_INITIALIZED,
                "the server has not been initialized".to_string(),
            )),
            "textDocument/hover" => self.hover(params, out),
            "textDocument/definition" => self.definition(params, out),
            "textDocument/documentSymbol" => self.document_symbols(params, out),
            other => Err((METHOD_NOT_FOUND, format!("method not found: {other}"))),
        }
    }

    fn notification(&mut self, method: &str, params: &Json, out: &mut Vec<Json>) {
        match method {
            "initialized" => self.publish(out),
            "exit" => self.exited = true,
            "textDocument/didOpen" => {
                let Some(uri) = document_uri(params) else {
                    return;
                };
                let Some(Json::Text(content)) =
                    field(params, "textDocument").and_then(|doc| field(doc, "text"))
                else {
                    return;
                };
                let content = content.clone();
                self.overlay(&uri, Some(content));
                self.publish(out);
            }
            "textDocument/didChange" => {
                let Some(uri) = document_uri(params) else {
                    return;
                };
                // full synchronization: the last change carries the whole
                // text
                let Some(Json::Array(changes)) = field(params, "contentChanges") else {
                    return;
                };
                let Some(Json::Text(content)) = changes.last().and_then(|c| field(c, "text"))
                else {
                    return;
                };
                let content = content.clone();
                self.overlay(&uri, Some(content));
                self.publish(out);
            }
            "textDocument/didClose" => {
                let Some(uri) = document_uri(params) else {
                    return;
                };
                self.overlay(&uri, None);
                self.publish(out);
            }
            "textDocument/didSave" => self.publish(out),
            _ => {}
        }
    }

    fn initialize(&mut self, params: &Json) -> Response {
        let folder = field(params, "workspaceFolders")
            .and_then(|folders| match folders {
                Json::Array(folders) => folders.first(),
                _ => None,
            })
            .and_then(|folder| field(folder, "uri"))
            .and_then(as_text)
            .or_else(|| field(params, "rootUri").and_then(as_text));
        let root = match folder {
            Some(uri) => uri_to_path(&uri)
                .ok_or_else(|| (INVALID_PARAMS, format!("not a file URI: {uri}")))?,
            None => field(params, "rootPath")
                .and_then(as_text)
                .unwrap_or_else(|| ".".to_string()),
        };
        self.workspace = Some(Workspace::new(root, &library()).as_written());
        Ok(obj(vec![
            (
                "capabilities",
                obj(vec![
                    ("textDocumentSync", Json::Number("1".to_string())),
                    ("hoverProvider", Json::Boolean(true)),
                    ("definitionProvider", Json::Boolean(true)),
                    ("documentSymbolProvider", Json::Boolean(true)),
                ]),
            ),
            (
                "serverInfo",
                obj(vec![
                    ("name", text("renyi")),
                    ("version", text(env!("CARGO_PKG_VERSION"))),
                ]),
            ),
        ]))
    }

    /// A document's text as the editor has it, or the disk again.
    fn overlay(&mut self, uri: &str, content: Option<String>) {
        let Some(path) = uri_to_path(uri) else {
            return;
        };
        let Some(workspace) = self.workspace.as_mut() else {
            return;
        };
        match content {
            Some(content) => {
                workspace.set_overlay(&path, content);
                self.open.insert(path, uri.to_string());
            }
            None => {
                workspace.clear_overlay(&path);
                self.open.remove(&path);
            }
        }
    }

    /// The workspace read again where it changed; a directory that
    /// cannot be read is reported on the standard error.
    fn refresh(&mut self) {
        if let Some(workspace) = self.workspace.as_mut() {
            if let Err(detail) = workspace.refresh() {
                eprintln!("renyi lsp: {detail}");
            }
        }
    }

    /// Every file's diagnostics, as `renyi check` reports them, sent for
    /// each file whose diagnostics differ from those last sent.
    fn publish(&mut self, out: &mut Vec<Json>) {
        self.refresh();
        let Some(workspace) = self.workspace.as_ref() else {
            return;
        };
        let mut sent = Vec::new();
        for name in workspace.files() {
            let Some(file) = workspace.file(name) else {
                continue;
            };
            let mut diagnostics: Vec<Diagnostic> =
                workspace.diagnostics(name).unwrap_or(&[]).to_vec();
            diagnostics.extend(check_layout(file));
            diagnostics.sort_by_key(|diagnostic| diagnostic.span.start);
            let rendered = Json::Array(
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic_json(file, diagnostic))
                    .collect(),
            );
            let uri = self.uri_of(file);
            sent.push((uri, rendered));
        }
        for (uri, rendered) in sent {
            if self.published.get(&uri) == Some(&rendered) {
                continue;
            }
            out.push(notification(
                "textDocument/publishDiagnostics",
                obj(vec![("uri", text(&uri)), ("diagnostics", rendered.clone())]),
            ));
            self.published.insert(uri, rendered);
        }
    }

    /// The URI of a file: the editor's own spelling for an open document,
    /// else one made from its path.
    fn uri_of(&self, file: &SourceFile) -> String {
        let workspace = self.workspace.as_ref().expect("a workspace");
        let relative = relative_name(workspace, &file.name);
        match self
            .open
            .iter()
            .find(|(open, _)| relative_name(workspace, open) == relative)
        {
            Some((_, uri)) => uri.clone(),
            None => path_to_uri(&file.name),
        }
    }

    /// The file and the module of a request's document, after a refresh.
    fn document(&mut self, params: &Json, out: &mut Vec<Json>) -> Option<(String, ModuleId)> {
        let uri = document_uri(params)?;
        let path = uri_to_path(&uri)?;
        self.publish(out);
        let module = self.workspace.as_ref()?.module_of(&path)?;
        Some((path, module))
    }

    fn hover(&mut self, params: &Json, out: &mut Vec<Json>) -> Response {
        let Some((path, module)) = self.document(params, out) else {
            return Ok(Json::Null);
        };
        let workspace = self.workspace.as_ref().expect("a workspace");
        let file = workspace.file(&path).expect("a file of the workspace");
        let Some(offset) = position_param(params, &file.text) else {
            return Ok(Json::Null);
        };
        let checked = workspace.checked().expect("a refreshed workspace");
        let library = &self.library;
        let source_of = |module: ModuleId| -> Option<&SourceFile> {
            workspace.file_of(module).or_else(|| {
                let name = &checked.world.modules[module].name;
                library.iter().find(|file| file.name == *name)
            })
        };
        let Some((span, described)) = what_is_at(checked, &source_of, module, offset) else {
            return Ok(Json::Null);
        };
        let mut value = format!("```renyi\n{}\n```", described.signature);
        if let Some(purpose) = &described.purpose {
            value.push_str("\n\n");
            value.push_str(purpose);
        }
        Ok(obj(vec![
            (
                "contents",
                obj(vec![("kind", text("markdown")), ("value", text(&value))]),
            ),
            ("range", range_json(&file.text, span.start, span.end)),
        ]))
    }

    fn definition(&mut self, params: &Json, out: &mut Vec<Json>) -> Response {
        let Some((path, module)) = self.document(params, out) else {
            return Ok(Json::Null);
        };
        let workspace = self.workspace.as_ref().expect("a workspace");
        let file = workspace.file(&path).expect("a file of the workspace");
        let Some(offset) = position_param(params, &file.text) else {
            return Ok(Json::Null);
        };
        let checked = workspace.checked().expect("a refreshed workspace");
        let source_of = |module: ModuleId| workspace.file_of(module);
        let Some((_, described)) = what_is_at(checked, &source_of, module, offset) else {
            return Ok(Json::Null);
        };
        // a library definition has no file to go to
        let Some(target) = workspace.file_of(described.module) else {
            return Ok(Json::Null);
        };
        Ok(obj(vec![
            ("uri", text(&self.uri_of(target))),
            (
                "range",
                range_json(&target.text, described.name.start, described.name.end),
            ),
        ]))
    }

    fn document_symbols(&mut self, params: &Json, out: &mut Vec<Json>) -> Response {
        let Some((path, module)) = self.document(params, out) else {
            return Ok(Json::Array(Vec::new()));
        };
        let workspace = self.workspace.as_ref().expect("a workspace");
        let file = workspace.file(&path).expect("a file of the workspace");
        let checked = workspace.checked().expect("a refreshed workspace");
        let symbols = symbols_of(&checked.world.modules[module].ast, file);
        Ok(Json::Array(
            symbols
                .iter()
                .map(|symbol| symbol_json(&file.text, symbol))
                .collect(),
        ))
    }
}

/// A file's name relative to the workspace root, with forward slashes.
fn relative_name(workspace: &Workspace, name: &str) -> String {
    let name = name.replace('\\', "/");
    let root = workspace.root().display().to_string().replace('\\', "/");
    let prefix = format!("{}/", root.trim_end_matches('/'));
    name.strip_prefix(&prefix).unwrap_or(&name).to_string()
}

// ------------------------------------------------------------ rendering

fn diagnostic_json(file: &SourceFile, diagnostic: &Diagnostic) -> Json {
    let mut message = diagnostic.message.clone();
    if let Some(fix) = &diagnostic.fix {
        message.push_str("\nfix: ");
        message.push_str(fix);
    }
    let severity = match diagnostic.severity {
        Severity::Error => "1",
        Severity::Warning => "2",
    };
    obj(vec![
        (
            "range",
            range_json(&file.text, diagnostic.span.start, diagnostic.span.end),
        ),
        ("severity", Json::Number(severity.to_string())),
        ("code", text(diagnostic.code)),
        ("source", text("renyi")),
        ("message", text(&message)),
    ])
}

fn position_json(text: &str, offset: usize) -> Json {
    let (line, character) = position_of(text, offset);
    obj(vec![
        ("line", Json::Number(line.to_string())),
        ("character", Json::Number(character.to_string())),
    ])
}

fn range_json(text: &str, start: usize, end: usize) -> Json {
    obj(vec![
        ("start", position_json(text, start)),
        ("end", position_json(text, end.max(start))),
    ])
}

fn symbol_json(text: &str, symbol: &Symbol) -> Json {
    let mut fields = vec![
        ("name", Json::Text(symbol.name.clone())),
        ("kind", Json::Number(symbol.kind.to_string())),
        (
            "range",
            range_json(text, symbol.range.start, symbol.range.end),
        ),
        (
            "selectionRange",
            range_json(text, symbol.selection.start, symbol.selection.end),
        ),
    ];
    if let Some(detail) = &symbol.detail {
        fields.push(("detail", Json::Text(detail.clone())));
    }
    if !symbol.children.is_empty() {
        fields.push((
            "children",
            Json::Array(
                symbol
                    .children
                    .iter()
                    .map(|child| symbol_json(text, child))
                    .collect(),
            ),
        ));
    }
    obj(fields)
}

/// The `textDocument.uri` of a request or a notification.
fn document_uri(params: &Json) -> Option<String> {
    field(params, "textDocument")
        .and_then(|document| field(document, "uri"))
        .and_then(as_text)
}

/// The byte offset of a request's `position` in the text.
fn position_param(params: &Json, text: &str) -> Option<usize> {
    let position = field(params, "position")?;
    let number = |key: &str| -> Option<usize> {
        match field(position, key)? {
            Json::Number(value) => value.parse().ok(),
            _ => None,
        }
    };
    offset_of(text, number("line")?, number("character")?)
}

fn notification(method: &str, params: Json) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("method", text(method)),
        ("params", params),
    ])
}

fn result_message(id: &Json, result: Json) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", id.clone()),
        ("result", result),
    ])
}

fn error_message(id: &Json, code: i64, message: &str) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", id.clone()),
        (
            "error",
            obj(vec![
                ("code", Json::Number(code.to_string())),
                ("message", text(message)),
            ]),
        ),
    ])
}

// ------------------------------------------------------------------ URIs

/// The path of a `file:` URI, percent-decoded, with forward slashes; on
/// Windows the leading slash before a drive letter is dropped.
pub(crate) fn uri_to_path(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let decoded = percent_decode(rest);
    let bytes = decoded.as_bytes();
    let drive =
        bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':';
    Some(if drive {
        decoded[1..].to_string()
    } else {
        decoded
    })
}

/// A `file:` URI for a path: forward slashes, each byte outside the
/// unreserved set percent-encoded, a drive letter's colon encoded as
/// editors spell it.
pub(crate) fn path_to_uri(path: &str) -> String {
    let path = path.replace('\\', "/");
    let mut out = String::from("file://");
    if !path.starts_with('/') {
        out.push('/');
    }
    for (index, byte) in path.bytes().enumerate() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
            || (byte == b':' && index != 1);
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = (bytes[index] == b'%' && index + 2 < bytes.len())
            .then(|| std::str::from_utf8(&bytes[index + 1..index + 3]).ok())
            .flatten()
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match escaped {
            Some(byte) => {
                out.push(byte);
                index += 3;
            }
            None => {
                out.push(bytes[index]);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_and_paths_round_trip() {
        // example paths of both conventions, as an editor sends them
        assert_eq!(
            uri_to_path("file:///d%3A/Projects/x/a%20b.ry").as_deref(),
            Some("d:/Projects/x/a b.ry")
        );
        assert_eq!(
            uri_to_path("file:///srv/p/a.ry").as_deref(),
            Some("/srv/p/a.ry")
        );
        assert_eq!(uri_to_path("untitled:x"), None);
        assert_eq!(
            path_to_uri("d:\\Projects\\x\\a b.ry"),
            "file:///d%3A/Projects/x/a%20b.ry"
        );
        assert_eq!(path_to_uri("/srv/p/a.ry"), "file:///srv/p/a.ry");
        assert_eq!(
            uri_to_path(&path_to_uri("/srv/p/a.ry")).as_deref(),
            Some("/srv/p/a.ry")
        );
        assert_eq!(percent_decode("%zz%4"), "%zz%4", "not an escape");
    }

    #[test]
    fn messages_are_framed_with_their_length() {
        let mut out = Vec::new();
        write_message(&mut out, &obj(vec![("a", text("b"))])).expect("written");
        assert_eq!(out, b"Content-Length: 9\r\n\r\n{\"a\":\"b\"}");
        let mut input = std::io::Cursor::new(
            b"Content-Length: 9\r\nContent-Type: application/json\r\n\r\n{\"a\":\"b\"}".to_vec(),
        );
        assert_eq!(
            read_message(&mut input).expect("read").as_deref(),
            Some("{\"a\":\"b\"}")
        );
        assert_eq!(read_message(&mut input).expect("read"), None);
    }
}
