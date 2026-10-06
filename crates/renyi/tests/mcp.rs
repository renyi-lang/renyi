//! `renyi mcp`: the toolchain over standard input and output, in both eras
//! of the Model Context Protocol (decision T1), serving the corpus.

use std::io::Write;
use std::process::{Command, Stdio};

use renyi_vm::natives::json::{read_json, Json};

fn examples() -> String {
    format!("{}/../../examples", env!("CARGO_MANIFEST_DIR"))
}

/// Send the lines to a fresh server and collect its messages, one per line.
fn exchange(lines: &[String]) -> Vec<Json> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .arg("mcp")
        .arg(examples())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the renyi binary starts");
    {
        let mut stdin = child.stdin.take().expect("a piped stdin");
        for line in lines {
            writeln!(stdin, "{line}").expect("the line is written");
        }
    }
    let output = child
        .wait_with_output()
        .expect("the server exits when its input closes");
    assert!(
        output.status.success(),
        "the server exited with {}",
        output.status
    );
    let text = String::from_utf8(output.stdout).expect("utf-8");
    text.lines()
        .map(|line| {
            read_json(line)
                .unwrap_or_else(|(detail, _)| panic!("not a JSON message: {detail}: {line}"))
        })
        .collect()
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

fn content(message: &Json) -> &str {
    text_of(get(message, &["result", "content", "0", "text"]))
}

fn request(id: u32, method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
}

/// A request of revision 2026-07-28: its version and capabilities travel in
/// `_meta`.
fn modern(id: u32, method: &str, params: &str) -> String {
    let meta = r#""_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28","io.modelcontextprotocol/clientCapabilities":{}}"#;
    let body = if params.is_empty() {
        meta.to_string()
    } else {
        format!("{meta},{params}")
    };
    request(id, method, &format!("{{{body}}}"))
}

fn call(id: u32, tool: &str, arguments: &str) -> String {
    request(
        id,
        "tools/call",
        &format!(r#"{{"name":"{tool}","arguments":{arguments}}}"#),
    )
}

#[test]
fn a_legacy_client_initializes_lists_and_calls() {
    let messages = exchange(&[
        request(
            1,
            "initialize",
            r#"{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}"#,
        ),
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#.to_string(),
        request(2, "tools/list", "{}"),
        call(3, "cheat_sheet", "{}"),
        call(4, "run", r#"{"path":"hello.ry","arguments":["Ada"]}"#),
        call(
            5,
            "check",
            r#"{"source":"module demo\n  purpose: A demo.\n\npublic function answer() returns Integer\n  return 42\nend\n"}"#,
        ),
        call(6, "nothing", "{}"),
        request(7, "ping", "{}"),
    ]);
    assert_eq!(messages.len(), 7, "the notification has no answer");
    let initialized = get(&messages[0], &["result"]);
    assert_eq!(
        text_of(get(initialized, &["protocolVersion"])),
        "2025-06-18"
    );
    assert_eq!(text_of(get(initialized, &["serverInfo", "name"])), "renyi");
    assert!(
        matches!(initialized, Json::Object(fields) if !fields.iter().any(|(k, _)| k == "resultType")),
        "a legacy result carries no resultType"
    );
    let tools: Vec<&str> = match get(&messages[1], &["result", "tools"]) {
        Json::Array(items) => items
            .iter()
            .map(|tool| text_of(get(tool, &["name"])))
            .collect(),
        other => panic!("not a list: {other:?}"),
    };
    assert_eq!(
        tools,
        [
            "cheat_sheet",
            "library_lookup",
            "project_map",
            "definition",
            "effects",
            "check",
            "format",
            "run",
            "run_tests",
            "diff"
        ]
    );
    assert!(content(&messages[2]).starts_with("# Renyi Cheat Sheet"));
    assert_eq!(
        get(&messages[3], &["result", "isError"]),
        &Json::Boolean(false)
    );
    assert!(
        content(&messages[3]).starts_with("Hello, Ada!\n"),
        "{}",
        content(&messages[3])
    );
    assert!(content(&messages[3]).ends_with("--- finished ---\n"));
    assert!(
        content(&messages[4]).contains("purpose-missing"),
        "{}",
        content(&messages[4])
    );
    assert_eq!(
        get(&messages[5], &["error", "code"]),
        &Json::Number("-32602".to_string())
    );
    assert_eq!(get(&messages[6], &["result"]), &Json::Object(Vec::new()));
}

#[test]
fn a_modern_client_discovers_and_calls_with_its_version() {
    let messages = exchange(&[
        modern(1, "server/discover", ""),
        modern(
            2,
            "tools/call",
            r#""name":"definition","arguments":{"name":"invoice.total"}"#,
        ),
        request(
            3,
            "tools/list",
            r#"{"_meta":{"io.modelcontextprotocol/protocolVersion":"2025-11-25","io.modelcontextprotocol/clientCapabilities":{}}}"#,
        ),
        request(
            4,
            "tools/list",
            r#"{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}"#,
        ),
        modern(5, "resources/list", ""),
        "{not json".to_string(),
        modern(6, "tools/list", ""),
    ]);
    assert_eq!(messages.len(), 7);
    let discovered = get(&messages[0], &["result"]);
    assert_eq!(text_of(get(discovered, &["resultType"])), "complete");
    assert_eq!(
        get(discovered, &["supportedVersions"]),
        &Json::Array(vec![Json::Text("2026-07-28".to_string())])
    );
    assert_eq!(
        text_of(get(
            discovered,
            &["_meta", "io.modelcontextprotocol/serverInfo", "name"]
        )),
        "renyi"
    );
    assert_eq!(
        text_of(get(&messages[1], &["result", "resultType"])),
        "complete"
    );
    let definition = content(&messages[1]);
    assert!(
        definition.contains("\"signature\": \"total(invoice: Invoice) returns Decimal\""),
        "{definition}"
    );
    assert!(definition.contains("source (./invoice.ry:"), "{definition}");
    assert!(
        definition.contains("public function total(invoice: Invoice) returns Decimal"),
        "{definition}"
    );
    let unsupported = get(&messages[2], &["error"]);
    assert_eq!(
        get(unsupported, &["code"]),
        &Json::Number("-32022".to_string())
    );
    assert_eq!(
        text_of(get(unsupported, &["data", "requested"])),
        "2025-11-25"
    );
    assert_eq!(
        get(unsupported, &["data", "supported"]),
        &Json::Array(vec![Json::Text("2026-07-28".to_string())])
    );
    assert_eq!(
        get(&messages[3], &["error", "code"]),
        &Json::Number("-32602".to_string())
    );
    assert_eq!(
        get(&messages[4], &["error", "code"]),
        &Json::Number("-32601".to_string())
    );
    assert_eq!(
        get(&messages[5], &["error", "code"]),
        &Json::Number("-32700".to_string())
    );
    assert_eq!(get(&messages[5], &["id"]), &Json::Null);
    let listed = get(&messages[6], &["result"]);
    assert_eq!(text_of(get(listed, &["cacheScope"])), "public");
    assert!(matches!(get(listed, &["ttlMs"]), Json::Number(_)));
}

#[test]
fn the_tools_answer_from_the_served_project() {
    let messages = exchange(&[
        call(1, "library_lookup", r#"{"query":"get_with"}"#),
        call(2, "project_map", r#"{"module":"invoice"}"#),
        call(3, "effects", r#"{"name":"sales_report.main"}"#),
        call(
            4,
            "format",
            r#"{"source":"module demo\n  purpose: A demo.\n\npublic function answer() returns Integer\n  purpose: The answer.\n\n   return 42\nend\n"}"#,
        ),
        call(5, "run_tests", r#"{"path":"invoice.ry"}"#),
        call(6, "run", r#"{"path":"hello.ry","deny":["console"]}"#),
        call(7, "project_map", r#"{"json":true}"#),
        call(8, "definition", r#"{"name":"no_such_thing"}"#),
        call(9, "diff", r#"{"base":"HEAD"}"#),
    ]);
    assert_eq!(messages.len(), 9);
    let lookup = content(&messages[0]);
    assert!(
        lookup.contains("std.http: public function get_with(url: Url, headers: Map of Text to Text) returns Response or fails with HttpError needs network.http"),
        "{lookup}"
    );
    let map = content(&messages[1]);
    assert!(map.contains("module invoice "), "{map}");
    assert!(!map.contains("module hello "), "{map}");
    let effects = content(&messages[2]);
    assert!(
        effects.starts_with("function sales_report.main  effects:"),
        "{effects}"
    );
    assert!(effects.contains("filesystem"), "{effects}");
    assert!(effects.contains("library:"), "{effects}");
    assert_eq!(
        get(&messages[3], &["result", "isError"]),
        &Json::Boolean(false)
    );
    assert!(
        content(&messages[3]).contains("\n  return 42\n"),
        "{}",
        content(&messages[3])
    );
    assert_eq!(
        get(&messages[4], &["result", "isError"]),
        &Json::Boolean(false)
    );
    assert!(
        content(&messages[4]).contains(" passed, 0 failed\n"),
        "{}",
        content(&messages[4])
    );
    assert_eq!(
        get(&messages[5], &["result", "isError"]),
        &Json::Boolean(true)
    );
    assert!(
        content(&messages[5]).contains("`console` is denied"),
        "{}",
        content(&messages[5])
    );
    assert!(
        content(&messages[6]).contains("\"definitions\": ["),
        "{}",
        content(&messages[6])
    );
    assert_eq!(
        get(&messages[7], &["result", "isError"]),
        &Json::Boolean(true)
    );
    assert!(content(&messages[7]).contains("no definition named `no_such_thing`"));
}
