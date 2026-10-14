//! `renyi lsp`: the language server over pipes (decision AN2): the
//! handshake, diagnostics published for every file of the folder and
//! again when a buffer changes, hover, definition and document symbols,
//! references and rename of declarations (decision AW38) and of locals
//! (decision AW39), completion, signature help and workspace symbols
//! (decision AW44) on a two-file project, positions in UTF-16 units, the
//! clean exit.

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

public ability Named
  purpose: Anything that says what it is.
  function label(self) returns Text
end

ability Named for Square
  function label(self) returns Text
    return \"square of {self.side}\"
  end
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
  console.print(square.label())
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

/// A `didChange` carrying the whole text.
fn changed(uri: &str, version: usize, content: &str) -> Json {
    notification(
        "textDocument/didChange",
        obj(vec![
            (
                "textDocument",
                obj(vec![("uri", text(uri)), ("version", number(version))]),
            ),
            (
                "contentChanges",
                Json::Array(vec![obj(vec![("text", text(content))])]),
            ),
        ]),
    )
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

/// The position with a references context.
fn with_context(mut params: Json, include_declaration: bool) -> Json {
    if let Json::Object(fields) = &mut params {
        fields.push((
            "context".to_string(),
            obj(vec![(
                "includeDeclaration",
                Json::Boolean(include_declaration),
            )]),
        ));
    }
    params
}

/// The position with the new name of a rename.
fn renaming(mut params: Json, new_name: &str) -> Json {
    if let Json::Object(fields) = &mut params {
        fields.push(("newName".to_string(), text(new_name)));
    }
    params
}

/// The locations of a references answer: URI, line and character, sorted.
fn locations_of(response: &Json) -> Vec<(String, usize, usize)> {
    let Json::Array(items) = get(response, &["result"]) else {
        panic!("not an array: {response:?}");
    };
    sorted(
        items
            .iter()
            .map(|location| {
                (
                    text_of(get(location, &["uri"])).to_string(),
                    number_of(get(location, &["range", "start", "line"])),
                    number_of(get(location, &["range", "start", "character"])),
                )
            })
            .collect(),
    )
}

fn sorted(mut items: Vec<(String, usize, usize)>) -> Vec<(String, usize, usize)> {
    items.sort();
    items
}

/// The edits of one file in a rename answer: line, character and the new
/// text, sorted.
fn edits_of(changes: &Json, uri: &str) -> Vec<(usize, usize, String)> {
    let Json::Array(items) = get(changes, &[uri]) else {
        panic!("no edits for {uri}: {changes:?}");
    };
    let mut out: Vec<(usize, usize, String)> = items
        .iter()
        .map(|edit| {
            (
                number_of(get(edit, &["range", "start", "line"])),
                number_of(get(edit, &["range", "start", "character"])),
                text_of(get(edit, &["newText"])).to_string(),
            )
        })
        .collect();
    out.sort();
    out
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
    let (square_line, square_character) = line_and_character(SHAPES, "Square\n  purpose");
    let (param_line, param_character) = line_and_character(SHAPES, "Square) returns");
    let (target_line, target_character) = line_and_character(SHAPES, "Square\n  function");
    let (exposed_line, exposed_character) = line_and_character(REPORT, "Square\n");
    let (built_line, built_character) = line_and_character(REPORT, "Square(side");
    let (label_line, label_character) = line_and_character(REPORT, "label()");
    let (ability_label_line, ability_label_character) =
        line_and_character(SHAPES, "label(self) returns Text\nend");
    let (impl_label_line, impl_label_character) =
        line_and_character(SHAPES, "label(self) returns Text\n    return");
    let (print2_line, print2_character) = line_and_character(REPORT, "print(square.label())");
    let (local_line, local_character) = line_and_character(REPORT, "square be");
    let (use1_line, use1_character) = line_and_character(REPORT, "square)");
    let (use2_line, use2_character) = line_and_character(REPORT, "square.label");
    let (self_line, self_character) = line_and_character(SHAPES, "self.side");
    // stage 35c (decision AW44): a bare name, a member and a call being
    // written, each a line the parser refuses, an import being written,
    // and positions in the clean text
    let bare_text = REPORT.replace(
        "  let square be Square(side: 3)\n",
        "  let square be Square(side: 3)\n  tot\n",
    );
    let (bare_line, bare_character) = line_and_character(&bare_text, "tot\n");
    let member_text = REPORT.replace(
        "  let square be Square(side: 3)\n",
        "  let square be Square(side: 3)\n  square.\n",
    );
    let (member_line, member_character) = line_and_character(&member_text, "  square.\n");
    let call_text = REPORT.replace(
        "  let square be Square(side: 3)\n",
        "  let square be Square(side: 3)\n  let total be square.label(\n",
    );
    let (call_line, call_character) = line_and_character(&call_text, "label(\n");
    let import_text = REPORT.replace(
        "import shapes exposing Square\n",
        "import shapes exposing Square\nimport std.\n",
    );
    let (import_line, import_character) = line_and_character(&import_text, "std.\n");
    let (area_call_line, area_call_character) = line_and_character(REPORT, "square)}");
    let (side_line, side_character) = line_and_character(REPORT, "3)");
    let (namespace_line, namespace_character) = line_and_character(REPORT, "print(\"");
    let (needs_line, needs_character) = line_and_character(REPORT, "console\n  purpose");

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
        changed(&report, 2, REPORT),
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
        request(7, "textDocument/typeDefinition", at(&report, 0, 0)),
        request(
            9,
            "textDocument/references",
            with_context(at(&report, hover_line, hover_character), true),
        ),
        request(
            10,
            "textDocument/references",
            with_context(at(&shapes, square_line, square_character), false),
        ),
        request(
            11,
            "textDocument/prepareRename",
            at(&report, label_line, label_character),
        ),
        request(
            12,
            "textDocument/rename",
            renaming(at(&report, label_line, label_character), "title"),
        ),
        request(
            13,
            "textDocument/rename",
            renaming(at(&report, hover_line, hover_character), "square"),
        ),
        request(
            14,
            "textDocument/rename",
            renaming(at(&report, hover_line, hover_character), "end"),
        ),
        request(
            15,
            "textDocument/rename",
            renaming(at(&report, hover_line, hover_character), "Area"),
        ),
        request(
            16,
            "textDocument/rename",
            renaming(at(&report, print_line, print_character), "say"),
        ),
        request(
            17,
            "textDocument/references",
            with_context(at(&report, print_line, print_character), true),
        ),
        request(
            18,
            "textDocument/hover",
            at(&shapes, param_line, param_character),
        ),
        request(
            19,
            "textDocument/references",
            with_context(at(&report, use1_line, use1_character), true),
        ),
        request(
            20,
            "textDocument/definition",
            at(&report, use2_line, use2_character),
        ),
        request(
            21,
            "textDocument/rename",
            renaming(at(&report, local_line, local_character), "box"),
        ),
        request(
            22,
            "textDocument/rename",
            renaming(at(&report, use1_line, use1_character), "main"),
        ),
        request(
            23,
            "textDocument/prepareRename",
            at(&shapes, self_line, self_character),
        ),
        request(
            24,
            "textDocument/prepareRename",
            at(&report, use2_line, use2_character),
        ),
        changed(&report, 3, &bare_text),
        request(
            25,
            "textDocument/completion",
            at(&report, bare_line, bare_character + "tot".len()),
        ),
        changed(&report, 4, &member_text),
        request(
            26,
            "textDocument/completion",
            at(&report, member_line, member_character + "  square.".len()),
        ),
        changed(&report, 5, &call_text),
        request(
            27,
            "textDocument/signatureHelp",
            at(&report, call_line, call_character + "label(".len()),
        ),
        changed(&report, 6, &import_text),
        request(
            28,
            "textDocument/completion",
            at(&report, import_line, import_character + "std.".len()),
        ),
        changed(&report, 7, REPORT),
        request(
            29,
            "textDocument/signatureHelp",
            at(&report, area_call_line, area_call_character),
        ),
        request(
            30,
            "textDocument/signatureHelp",
            at(&report, side_line, side_character),
        ),
        request(
            31,
            "textDocument/completion",
            at(&report, namespace_line, namespace_character),
        ),
        request(
            32,
            "textDocument/completion",
            at(&report, needs_line, needs_character),
        ),
        request(33, "workspace/symbol", obj(vec![("query", text("ar"))])),
        request(34, "workspace/symbol", obj(vec![("query", text(""))])),
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
    assert_eq!(report_published.len(), 8, "{report_published:?}");
    assert_eq!(report_published[0], &Json::Array(Vec::new()));
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
    // the buffers of stage 35c: a line the parser refuses in each of the
    // first three, an import without a module in the fourth, then clean
    // again; the repairs made for the requests publish nothing
    for (index, published) in report_published.iter().enumerate().take(7).skip(3) {
        assert_ne!(*published, &Json::Array(Vec::new()), "buffer {index}");
    }
    assert_eq!(report_published[7], &Json::Array(Vec::new()));

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
    assert_eq!(names, ["Square", "Named", "Named for Square", "area"]);
    assert_eq!(
        get(&symbols[0], &["kind"]),
        &number(23),
        "a record is a struct"
    );
    assert_eq!(
        text_of(get(&symbols[0], &["children", "0", "name"])),
        "side"
    );
    assert_eq!(
        get(&symbols[1], &["kind"]),
        &number(11),
        "an ability is an interface"
    );
    assert_eq!(
        text_of(get(&symbols[2], &["children", "0", "name"])),
        "label",
        "an implementation's methods are its children"
    );
    assert_eq!(get(&symbols[3], &["kind"]), &number(12), "a function");
    assert_eq!(
        number_of(get(&symbols[3], &["selectionRange", "start", "line"])),
        area_line
    );

    // nothing under the cursor, an unknown method
    assert_eq!(get(response(&messages, 6), &["result"]), &Json::Null);
    assert_eq!(
        get(response(&messages, 7), &["error", "code"]),
        &Json::Number("-32601".to_string())
    );

    // references of `area` from its call: its declaration and the call
    assert_eq!(
        locations_of(response(&messages, 9)),
        sorted(vec![
            (shapes.clone(), area_line, area_character),
            (report.clone(), hover_line, hover_character),
        ])
    );
    // references of `Square` from its declaration, the declaration left
    // out: the parameter's type and the implementation's target in
    // shapes.ry, the exposing import and the construction in report.ry
    assert_eq!(
        locations_of(response(&messages, 10)),
        sorted(vec![
            (shapes.clone(), param_line, param_character),
            (shapes.clone(), target_line, target_character),
            (report.clone(), exposed_line, exposed_character),
            (report.clone(), built_line, built_character),
        ])
    );
    // prepareRename on a call of `label`: the name's range and the name
    let prepared = get(response(&messages, 11), &["result"]);
    assert_eq!(text_of(get(prepared, &["placeholder"])), "label");
    assert_eq!(
        number_of(get(prepared, &["range", "start", "line"])),
        label_line
    );
    assert_eq!(
        number_of(get(prepared, &["range", "start", "character"])),
        label_character
    );
    assert_eq!(
        number_of(get(prepared, &["range", "end", "character"])),
        label_character + "label".len()
    );
    // rename `label` to `title`: the ability's method, the
    // implementation's and the call, one family
    let changes = get(response(&messages, 12), &["result", "changes"]);
    assert_eq!(
        edits_of(changes, &shapes),
        vec![
            (
                ability_label_line,
                ability_label_character,
                "title".to_string()
            ),
            (impl_label_line, impl_label_character, "title".to_string()),
        ]
    );
    assert_eq!(
        edits_of(changes, &report),
        vec![(label_line, label_character, "title".to_string())]
    );
    // renames refused: a name the parameter `square` would shadow (the
    // project checked with the rename applied), a reserved word, the
    // wrong class, a library function
    let refused =
        |id: usize| text_of(get(response(&messages, id), &["error", "message"])).to_string();
    assert!(
        refused(13).contains("`square` is a function of this module"),
        "{}",
        refused(13)
    );
    assert!(
        refused(13).contains("`shapes.ry` at line"),
        "{}",
        refused(13)
    );
    assert!(refused(14).contains("reserved word"), "{}", refused(14));
    assert!(
        refused(15).contains("not a function, method or constant name"),
        "{}",
        refused(15)
    );
    assert!(
        refused(16).contains("library module `std.console`"),
        "{}",
        refused(16)
    );
    // references of a library function: its calls in the workspace alone
    assert_eq!(
        locations_of(response(&messages, 17)),
        sorted(vec![
            (report.clone(), print_line, print_character),
            (report.clone(), print2_line, print2_character),
        ])
    );
    // hover on a type named in a signature
    let hover = text_of(get(
        response(&messages, 18),
        &["result", "contents", "value"],
    ));
    assert!(hover.contains("Square(side: Integer)"), "{hover}");

    // a local (decision AW39): its references from a use are its
    // declaration and its uses; definition from a use is the declaration
    assert_eq!(
        locations_of(response(&messages, 19)),
        sorted(vec![
            (report.clone(), local_line, local_character),
            (report.clone(), use1_line, use1_character),
            (report.clone(), use2_line, use2_character),
        ])
    );
    let definition = get(response(&messages, 20), &["result"]);
    assert_eq!(text_of(get(definition, &["uri"])), report);
    assert_eq!(
        number_of(get(definition, &["range", "start", "line"])),
        local_line
    );
    assert_eq!(
        number_of(get(definition, &["range", "start", "character"])),
        local_character
    );
    // rename `square` to `box` from its declaration: the three places
    let changes = get(response(&messages, 21), &["result", "changes"]);
    assert_eq!(
        edits_of(changes, &report),
        vec![
            (local_line, local_character, "box".to_string()),
            (use1_line, use1_character, "box".to_string()),
            (use2_line, use2_character, "box".to_string()),
        ]
    );
    // refused: `main` is a function of the module (the check after the
    // edits), and `self`
    assert!(
        refused(22).contains("`main` is a function of this module"),
        "{}",
        refused(22)
    );
    assert!(
        refused(23).contains("`self` cannot be renamed"),
        "{}",
        refused(23)
    );
    // prepareRename on a use of the local
    let prepared = get(response(&messages, 24), &["result"]);
    assert_eq!(text_of(get(prepared, &["placeholder"])), "square");
    assert_eq!(
        number_of(get(prepared, &["range", "start", "character"])),
        use2_character
    );

    // completion at a bare name (decision AW44), the line repaired for the
    // request: the local first, then the module's function, the imports'
    // namespaces and the exposed type, then the statement words
    let items_of = |id: usize| -> Vec<(String, String)> {
        let Json::Array(items) = get(response(&messages, id), &["result"]) else {
            panic!("not an array: {:?}", response(&messages, id));
        };
        items
            .iter()
            .map(|item| {
                (
                    text_of(get(item, &["label"])).to_string(),
                    text_of(get(item, &["sortText"])).to_string(),
                )
            })
            .collect()
    };
    let detail_of = |id: usize, label: &str| -> String {
        let Json::Array(items) = get(response(&messages, id), &["result"]) else {
            panic!("not an array: {:?}", response(&messages, id));
        };
        let item = items
            .iter()
            .find(|item| text_of(get(item, &["label"])) == label)
            .unwrap_or_else(|| panic!("no item `{label}` in the answer to {id}"));
        text_of(get(item, &["detail"])).to_string()
    };
    let has = |items: &[(String, String)], label: &str, sort: &str| {
        items
            .iter()
            .any(|(found, order)| found == label && order == sort)
    };
    let bare = items_of(25);
    assert!(has(&bare, "square", "0square"), "{bare:?}");
    assert!(has(&bare, "main", "1main"), "{bare:?}");
    assert!(has(&bare, "console", "2console"), "{bare:?}");
    assert!(has(&bare, "shapes", "2shapes"), "{bare:?}");
    assert!(has(&bare, "Square", "2Square"), "{bare:?}");
    assert!(has(&bare, "for each", "4for each"), "{bare:?}");
    assert!(has(&bare, "let", "4let"), "{bare:?}");
    assert!(
        !bare
            .iter()
            .any(|(label, _)| label == "area" || label == "self"),
        "{bare:?}"
    );
    assert_eq!(detail_of(25, "main"), "main() needs console");
    // after a dot: the record's field, the implementation's method and the
    // method every type has
    let member = items_of(26);
    assert!(has(&member, "side", "0side"), "{member:?}");
    assert!(has(&member, "label", "1label"), "{member:?}");
    assert!(has(&member, "equals", "2equals"), "{member:?}");
    assert!(
        !member.iter().any(|(label, _)| label == "area"),
        "{member:?}"
    );
    assert_eq!(detail_of(26, "side"), "side: Integer");
    assert_eq!(detail_of(26, "label"), "label(self) returns Text");
    // signature help of a method call on a line the parser refused
    let help = get(response(&messages, 27), &["result"]);
    assert_eq!(
        text_of(get(help, &["signatures", "0", "label"])),
        "label(self) returns Text"
    );
    assert_eq!(
        get(help, &["signatures", "0", "parameters"]),
        &Json::Array(Vec::new())
    );
    assert_eq!(get(help, &["activeParameter"]), &number(0));
    // an import being written: the library's modules, replacing the path
    // typed so far
    let imports = items_of(28);
    assert!(has(&imports, "std.console", "0std.console"), "{imports:?}");
    assert!(has(&imports, "std.json", "0std.json"), "{imports:?}");
    assert!(
        !imports.iter().any(|(label, _)| label == "shapes"),
        "{imports:?}"
    );
    let Json::Array(items) = get(response(&messages, 28), &["result"]) else {
        panic!("not an array");
    };
    let edit = get(&items[0], &["textEdit"]);
    assert_eq!(
        number_of(get(edit, &["range", "start", "line"])),
        import_line
    );
    assert_eq!(
        number_of(get(edit, &["range", "start", "character"])),
        import_character
    );
    assert_eq!(
        text_of(get(edit, &["newText"])),
        text_of(get(&items[0], &["label"]))
    );
    // the clean text: a function of an import called inside a hole, with
    // its purpose; a record built
    let help = get(response(&messages, 29), &["result"]);
    assert_eq!(
        text_of(get(help, &["signatures", "0", "label"])),
        "area(square: Square) returns Integer"
    );
    assert_eq!(
        text_of(get(help, &["signatures", "0", "parameters", "0", "label"])),
        "square: Square"
    );
    assert_eq!(
        text_of(get(help, &["signatures", "0", "documentation"])),
        "The area of the square."
    );
    let help = get(response(&messages, 30), &["result"]);
    assert_eq!(
        text_of(get(help, &["signatures", "0", "label"])),
        "Square(side: Integer)"
    );
    assert_eq!(
        text_of(get(help, &["signatures", "0", "parameters", "0", "label"])),
        "side: Integer"
    );
    // a namespace's members, the signature as the detail
    let members = items_of(31);
    assert!(has(&members, "print", "0print"), "{members:?}");
    assert!(
        !members.iter().any(|(label, _)| label == "label"),
        "{members:?}"
    );
    assert_eq!(detail_of(31, "print"), "print(text: Text) needs console");
    // the capabilities after `needs`
    let capabilities = items_of(32);
    assert!(
        has(&capabilities, "console", "0console"),
        "{capabilities:?}"
    );
    assert!(
        has(&capabilities, "filesystem.read", "0filesystem.read"),
        "{capabilities:?}"
    );
    // workspace symbols by a part of the name, and every one
    let symbols_of = |id: usize| -> Vec<(String, String)> {
        let Json::Array(items) = get(response(&messages, id), &["result"]) else {
            panic!("not an array: {:?}", response(&messages, id));
        };
        items
            .iter()
            .map(|item| {
                (
                    text_of(get(item, &["name"])).to_string(),
                    text_of(get(item, &["containerName"])).to_string(),
                )
            })
            .collect()
    };
    let named = symbols_of(33);
    assert!(
        named.contains(&("area".to_string(), "shapes".to_string())),
        "{named:?}"
    );
    assert!(!named.iter().any(|(name, _)| name == "main"), "{named:?}");
    let all = symbols_of(34);
    assert!(all.len() >= 7, "{all:?}");
    assert!(
        all.contains(&("side".to_string(), "shapes.Square".to_string())),
        "{all:?}"
    );
    assert!(
        all.contains(&("label".to_string(), "shapes.Named for Square".to_string())),
        "{all:?}"
    );
    assert!(
        all.contains(&("main".to_string(), "report".to_string())),
        "{all:?}"
    );

    // the shutdown
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
