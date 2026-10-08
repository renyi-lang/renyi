//! `renyi lsp`: the language server over pipes (decision AN2): the
//! handshake, diagnostics published for every file of the folder and
//! again when a buffer changes, hover, definition and document symbols
//! on a two-file project, positions in UTF-16 units, the clean exit.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use renyi_vm::natives::json::{read_json, write_json, Json};

const SHAPES: &str = "module shapes
  purpose: Shapes for the server test.

public type Square
  purpose: A square by its side.
  has side: Integer
end

public function area(square: Square) returns Integer
  purpose: The area of the square.

  return square.side * square.side
end
";

const REPORT: &str = "module report
  purpose: Print the area of a square.

import std.console
import shapes exposing Square

public function main() needs console
  purpose: Print the area.

  let square be Square(side: 3)
  console.print(\"\u{1F600} {shapes.area(square)}\")
end
";

fn project() -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/lsp/project");
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean project directory");
    }
    std::fs::create_dir_all(&directory).expect("the project directory");
    std::fs::write(directory.join("shapes.ry"), SHAPES).expect("shapes.ry");
    std::fs::write(directory.join("report.ry"), REPORT).expect("report.ry");
    directory.canonicalize().expect("a canonical path")
}

/// A `file:` URI as an editor spells it: forward slashes, the drive
/// letter's colon encoded, the `\\?\` prefix of a canonical Windows path
/// dropped.
fn uri(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    let text = text.replacen(':', "%3A", 1);
    if text.starts_with('/') {
        format!("file://{text}")
    } else {
        format!("file:///{text}")
    }
}

fn framed(message: &Json) -> Vec<u8> {
    let mut body = String::new();
    write_json(message, &mut body, None, 0);
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

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

fn number(value: usize) -> Json {
    Json::Number(value.to_string())
}

fn request(id: usize, method: &str, params: Json) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("id", number(id)),
        ("method", text(method)),
        ("params", params),
    ])
}

fn notification(method: &str, params: Json) -> Json {
    obj(vec![
        ("jsonrpc", text("2.0")),
        ("method", text(method)),
        ("params", params),
    ])
}

fn document(uri: &str) -> Json {
    obj(vec![("uri", text(uri))])
}

fn at(uri: &str, line: usize, character: usize) -> Json {
    obj(vec![
        ("textDocument", document(uri)),
        (
            "position",
            obj(vec![
                ("line", number(line)),
                ("character", number(character)),
            ]),
        ),
    ])
}

/// Send the messages to a fresh server and collect every message it
/// writes, in order, once it has exited.
fn exchange(messages: &[Json]) -> (Vec<Json>, bool) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the renyi binary starts");
    {
        let mut stdin = child.stdin.take().expect("a piped stdin");
        for message in messages {
            stdin
                .write_all(&framed(message))
                .expect("the message is written");
        }
    }
    let mut output = Vec::new();
    child
        .stdout
        .take()
        .expect("a piped stdout")
        .read_to_end(&mut output)
        .expect("the output is read");
    let status = child.wait().expect("the server exits");
    (unframe(&output), status.success())
}

fn unframe(mut bytes: &[u8]) -> Vec<Json> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        let text = std::str::from_utf8(bytes).expect("utf-8 output");
        let (headers, rest) = text.split_once("\r\n\r\n").expect("a framed message");
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length:"))
            .expect("a Content-Length")
            .trim()
            .parse()
            .expect("a number");
        let body = &rest.as_bytes()[..length];
        messages.push(
            read_json(std::str::from_utf8(body).expect("utf-8 body")).expect("a JSON message"),
        );
        bytes = &rest.as_bytes()[length..];
    }
    messages
}

fn get<'a>(json: &'a Json, path: &[&str]) -> &'a Json {
    let mut current = json;
    for key in path {
        current = match current {
            Json::Object(fields) => {
                &fields
                    .iter()
                    .find(|(name, _)| name == key)
                    .unwrap_or_else(|| panic!("no field `{key}` in {current:?}"))
                    .1
            }
            Json::Array(items) => &items[key.parse::<usize>().expect("an index")],
            other => panic!("`{key}`: {other:?} has no fields"),
        };
    }
    current
}

fn text_of(json: &Json) -> &str {
    match json {
        Json::Text(text) => text,
        other => panic!("not text: {other:?}"),
    }
}

fn number_of(json: &Json) -> usize {
    match json {
        Json::Number(value) => value.parse().expect("a whole number"),
        other => panic!("not a number: {other:?}"),
    }
}

/// The response to a request, by id.
fn response(messages: &[Json], id: usize) -> &Json {
    messages
        .iter()
        .find(|message| {
            matches!(
                message,
                Json::Object(fields) if fields.iter().any(|(k, v)| k == "id" && *v == number(id))
            )
        })
        .unwrap_or_else(|| panic!("no response to {id}"))
}

/// Every `publishDiagnostics` for a URI, in order.
fn published<'a>(messages: &'a [Json], uri: &str) -> Vec<&'a Json> {
    messages
        .iter()
        .filter(|message| {
            matches!(message, Json::Object(fields)
                if fields.iter().any(|(k, v)| k == "method" && *v == text("textDocument/publishDiagnostics")))
                && text_of(get(message, &["params", "uri"])) == uri
        })
        .map(|message| get(message, &["params", "diagnostics"]))
        .collect()
}

fn line_and_character(source: &str, needle: &str) -> (usize, usize) {
    let offset = source.find(needle).expect("the needle is in the text");
    let before = &source[..offset];
    let line = before.matches('\n').count();
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, before[start..].encode_utf16().count())
}

#[test]
fn the_server_publishes_diagnostics_and_answers_hover_definition_and_symbols() {
    let directory = project();
    let root = uri(&directory);
    let shapes = uri(&directory.join("shapes.ry"));
    let report = uri(&directory.join("report.ry"));
    // an unknown name in the text's hole, after the emoji: the column is
    // counted in UTF-16 units
    let broken = REPORT.replace("{shapes.area(square)}", "{nothing_here}");
    let (bad_line, bad_character) = line_and_character(&broken, "nothing_here");
    let (hover_line, hover_character) = line_and_character(REPORT, "area(square)");
    let (print_line, print_character) = line_and_character(REPORT, "print(");
    let (area_line, area_character) = line_and_character(SHAPES, "area(square: Square)");

    let (messages, clean_exit) = exchange(&[
        request(
            1,
            "initialize",
            obj(vec![
                ("processId", Json::Null),
                ("rootUri", text(&root)),
                ("capabilities", obj(Vec::new())),
            ]),
        ),
        notification("initialized", obj(Vec::new())),
        notification(
            "textDocument/didOpen",
            obj(vec![(
                "textDocument",
                obj(vec![
                    ("uri", text(&report)),
                    ("languageId", text("renyi")),
                    ("version", number(1)),
                    ("text", text(&broken)),
                ]),
            )]),
        ),
        notification(
            "textDocument/didChange",
            obj(vec![
                (
                    "textDocument",
                    obj(vec![("uri", text(&report)), ("version", number(2))]),
                ),
                (
                    "contentChanges",
                    Json::Array(vec![obj(vec![("text", text(REPORT))])]),
                ),
            ]),
        ),
        request(
            2,
            "textDocument/hover",
            at(&report, hover_line, hover_character),
        ),
        request(
            3,
            "textDocument/definition",
            at(&report, hover_line, hover_character),
        ),
        request(
            4,
            "textDocument/hover",
            at(&report, print_line, print_character),
        ),
        request(
            5,
            "textDocument/documentSymbol",
            obj(vec![("textDocument", document(&shapes))]),
        ),
        request(6, "textDocument/hover", at(&report, 0, 0)),
        request(7, "workspace/symbol", obj(vec![("query", text(""))])),
        request(8, "shutdown", Json::Null),
        notification("exit", Json::Null),
    ]);
    assert!(clean_exit, "exit after shutdown is a clean exit");

    // the handshake
    let initialized = response(&messages, 1);
    assert_eq!(
        get(initialized, &["result", "capabilities", "textDocumentSync"]),
        &number(1)
    );
    assert_eq!(
        get(initialized, &["result", "capabilities", "hoverProvider"]),
        &Json::Boolean(true)
    );
    assert_eq!(
        text_of(get(initialized, &["result", "serverInfo", "name"])),
        "renyi"
    );

    // diagnostics: both files clean at first, then the opened buffer's
    // error at its position, then clean again once the buffer is restored
    let shapes_published = published(&messages, &shapes);
    assert_eq!(shapes_published, vec![&Json::Array(Vec::new())]);
    let report_published = published(&messages, &report);
    assert_eq!(report_published.len(), 3, "{report_published:?}");
    assert_eq!(report_published[0], &Json::Array(Vec::new()));
    // the binding `square` is unused now too, an error of its own before
    // this one
    let Json::Array(items) = report_published[1] else {
        panic!("not an array: {:?}", report_published[1]);
    };
    let error = items
        .iter()
        .find(|item| get(item, &["code"]) == &text("unknown-name"))
        .unwrap_or_else(|| panic!("no unknown-name among {items:?}"));
    assert_eq!(get(error, &["severity"]), &number(1));
    assert_eq!(text_of(get(error, &["source"])), "renyi");
    assert!(text_of(get(error, &["message"])).contains("nothing_here"));
    assert!(text_of(get(error, &["message"])).contains("\nfix: "));
    assert_eq!(number_of(get(error, &["range", "start", "line"])), bad_line);
    assert_eq!(
        number_of(get(error, &["range", "start", "character"])),
        bad_character
    );
    assert_eq!(report_published[2], &Json::Array(Vec::new()));

    // hover on a call: the signature and the purpose of the function
    let hover = text_of(get(
        response(&messages, 2),
        &["result", "contents", "value"],
    ));
    assert!(
        hover.contains("```renyi\narea(square: Square) returns Integer\n```"),
        "{hover}"
    );
    assert!(hover.contains("The area of the square."), "{hover}");

    // definition: the function's name in the other file
    let definition = get(response(&messages, 3), &["result"]);
    assert_eq!(text_of(get(definition, &["uri"])), shapes);
    assert_eq!(
        number_of(get(definition, &["range", "start", "line"])),
        area_line
    );
    assert_eq!(
        number_of(get(definition, &["range", "start", "character"])),
        area_character
    );

    // hover on a library call: its declaration and purpose
    let hover = text_of(get(
        response(&messages, 4),
        &["result", "contents", "value"],
    ));
    assert!(hover.contains("print(text: Text) needs console"), "{hover}");
    assert!(
        hover.contains("Write the text and a line break to standard output."),
        "{hover}"
    );

    // the outline of shapes.ry
    let symbols = get(response(&messages, 5), &["result"]);
    let Json::Array(symbols) = symbols else {
        panic!("not an array: {symbols:?}");
    };
    let names: Vec<&str> = symbols
        .iter()
        .map(|symbol| text_of(get(symbol, &["name"])))
        .collect();
    assert_eq!(names, ["Square", "area"]);
    assert_eq!(
        get(&symbols[0], &["kind"]),
        &number(23),
        "a record is a struct"
    );
    assert_eq!(
        text_of(get(&symbols[0], &["children", "0", "name"])),
        "side"
    );
    assert_eq!(get(&symbols[1], &["kind"]), &number(12), "a function");
    assert_eq!(
        number_of(get(&symbols[1], &["selectionRange", "start", "line"])),
        area_line
    );

    // nothing under the cursor, an unknown method, the shutdown
    assert_eq!(get(response(&messages, 6), &["result"]), &Json::Null);
    assert_eq!(
        get(response(&messages, 7), &["error", "code"]),
        &Json::Number("-32601".to_string())
    );
    assert_eq!(get(response(&messages, 8), &["result"]), &Json::Null);
}

#[test]
fn a_request_before_initialize_is_refused_and_input_closing_is_not_a_clean_exit() {
    let (messages, clean_exit) = exchange(&[request(
        1,
        "textDocument/hover",
        at("file:///nowhere/a.ry", 0, 0),
    )]);
    assert!(!clean_exit);
    assert_eq!(
        get(response(&messages, 1), &["error", "code"]),
        &Json::Number("-32002".to_string())
    );
}
