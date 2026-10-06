//! The network and database primitives: a server answers a client over the
//! loopback, a redirect to another host is refused by the grant, and SQLite
//! rows decode into records.

use std::cell::RefCell;
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use renyi_syntax::SourceFile;
use renyi_vm::{compile_project, run_program, Options, Program, Run, RunOutcome};

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

fn run(program: &Program, stdout: &Capture, options: Options) -> Run {
    run_program(
        program,
        Options {
            stdout: Box::new(stdout.clone()),
            stderr: Box::new(Capture::default()),
            stdin: Box::new(std::io::Cursor::new(Vec::new())),
            ..options
        },
    )
}

fn scratch(name: &str) -> String {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf();
    let dir = workspace.join("target/vm-tests").join(name);
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir.display().to_string().replace('\\', "/")
}

#[test]
fn a_server_answers_a_client_over_the_loopback() {
    let port = 18000 + (std::process::id() % 2000) as u16;
    let server_source = format!(
        r#"module service
  purpose: A small JSON API that converts temperatures.

import std.console
import std.server exposing Request, Response, Port, StartError

type Conversion
  has celsius: Decimal
  has fahrenheit: Decimal
  can ToJson
end

function handle(request: Request) returns Response
  purpose: Route one request by its path.

  match request.path
    when "/health" then return server.ok("ok")
    when "/convert" then return convert(request)
    when "/moved" then return server.respond(status: 302, body: "").with_header(name: "location", value: "http://other.invalid/x")
    otherwise return server.not_found()
  end
end

function convert(request: Request) returns Response
  let celsius_text be request.query.get("celsius")
    otherwise return server.bad_request("celsius is required")
  let celsius be celsius_text.to_decimal()
    otherwise return server.bad_request("celsius must be a number")
  let conversion be Conversion(celsius: celsius, fahrenheit: celsius * 9 / 5 + 32)
  return server.ok_json(conversion)
end

public function main() or fails with StartError needs console, network.socket
  purpose: Serve the API until four requests were answered.

  console.print("listening")
  server.serve(port: Port({port}), handler: handle) otherwise fail
  console.print("done")
end
"#
    );
    let server = std::thread::spawn(move || {
        let program = compile("service.ry", &server_source);
        let stdout = Capture::default();
        let outcome = run(
            &program,
            &stdout,
            Options {
                serve_limit: Some(4),
                ..Options::default()
            },
        )
        .outcome;
        (outcome, stdout.text())
    });
    // wait for the port to open
    let started = Instant::now();
    loop {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the server did not start"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    let client_source = format!(
        r#"module client
  purpose: Call the service and print what comes back.

import std.console
import std.http exposing Url, HttpError

function show(url: Url) needs console, network.http("127.0.0.1")
  purpose: Print the status and body of one request, or its error.

  match http.get(url)
    when success(response) then console.print("{{response.status}} {{response.body}}")
    when failure(error) then console.print(error.to_text())
  end
end

public function main() needs console, network.http("127.0.0.1")
  purpose: Four requests to the local service.

  show(Url("http://127.0.0.1:{port}/health"))
  show(Url("http://127.0.0.1:{port}/convert?celsius=100"))
  show(Url("http://127.0.0.1:{port}/missing"))
  show(Url("http://127.0.0.1:{port}/moved"))
end
"#
    );
    let program = compile("client.ry", &client_source);
    let stdout = Capture::default();
    let outcome = run(&program, &stdout, Options::default()).outcome;
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(
        stdout.text(),
        format!(
            "200 ok\n200 {{\"celsius\":100,\"fahrenheit\":212}}\nStatus(url: \"http://127.0.0.1:{port}/missing\", status: 404, body: \"not found\")\nHostNotAllowed(host: \"other.invalid\")\n"
        )
    );
    let (outcome, printed) = server.join().expect("the server thread");
    assert_eq!(outcome, RunOutcome::Finished);
    assert_eq!(printed, "listening\ndone\n");
}

#[test]
fn sqlite_rows_decode_into_records() {
    let dir = scratch("sqlite");
    let _ = std::fs::remove_file(format!("{dir}/inventory.db"));
    let source = format!(
        r#"module inventory
  purpose: Keep a small stock inventory in SQLite.

import std.console
import std.filesystem exposing Path
import std.sqlite exposing Connection, DbError, FromRow

public type Item
  purpose: One product; field names match the column names.
  has sku: Text
  has name: Text
  has quantity: Integer where quantity is at least 0
  has price: Decimal
  has note: maybe Text
  can FromRow
end

type Wrong
  has sku: Text
  has quantity: Text
  can FromRow
end

type Positive
  has sku: Text
  has quantity: Integer where quantity is at least 1
  can FromRow
end

let create_table: Text be """
  CREATE TABLE IF NOT EXISTS items (
    sku TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    price NUMERIC NOT NULL,
    note TEXT
  )
  """

function report(connection: Connection) needs console, filesystem.read
  purpose: Print every item, lowest quantity first.

  let sql be "SELECT * FROM items ORDER BY quantity"
  let items: List of Item be connection.query(sql: sql, parameters: []) otherwise crash with "query"
  for each item in items
    let note be item.note otherwise "-"
    console.print("{{item.sku}} {{item.name}}: {{item.quantity}} at {{item.price}} ({{note}})")
  end
end

public function main() or fails with DbError needs console, filesystem
  purpose: Create the table, insert rows, update one, then query them.

  let connection be sqlite.open(Path("{dir}/inventory.db")) otherwise fail
  ignore connection.execute(sql: create_table, parameters: []) otherwise fail
  ignore connection.execute(sql: "DELETE FROM items", parameters: []) otherwise fail
  let insert be "INSERT INTO items VALUES (?, ?, ?, ?, ?)"
  ignore connection.execute(sql: insert, parameters: [sqlite.text("BOLT-10"), sqlite.text("Bolt"), sqlite.integer(3), sqlite.decimal(0.25), sqlite.absent()]) otherwise fail
  ignore connection.execute(sql: insert, parameters: [sqlite.text("NUT-10"), sqlite.text("Nut"), sqlite.integer(0), sqlite.decimal(12.50), sqlite.text("reorder")]) otherwise fail
  let changed be connection.execute(sql: "UPDATE items SET quantity = quantity + ? WHERE sku = ?", parameters: [sqlite.integer(5), sqlite.text("BOLT-10")]) otherwise fail
  console.print("changed {{changed}}")
  report(connection)
  let wrong: List of Wrong be connection.query(sql: "SELECT sku, quantity FROM items", parameters: [])
    otherwise fail
  console.print("wrong rows: {{wrong.length()}}")
  let positive: List of Positive be connection.query(sql: "SELECT sku, quantity FROM items", parameters: [])
    otherwise fail
  console.print("positive rows: {{positive.length()}}")
  connection.close()
end
"#
    );
    let program = compile("inventory.ry", &source);
    let stdout = Capture::default();
    let run_once = run(
        &program,
        &stdout,
        Options {
            record: true,
            ..Options::default()
        },
    );
    assert_eq!(
        run_once.outcome,
        RunOutcome::Failed(
            "Mismatch(column: \"quantity\", expected: \"Text\", found: \"INTEGER\")".to_string()
        )
    );
    assert_eq!(
        stdout.text(),
        "changed 1\nNUT-10 Nut: 0 at 12.5 (reorder)\nBOLT-10 Bolt: 8 at 0.25 (-)\n"
    );
    let recording = run_once.recording.expect("a recording");
    let primitives: Vec<&str> = recording
        .calls
        .iter()
        .map(|call| call.primitive.as_str())
        .collect();
    assert_eq!(
        primitives,
        [
            "std.sqlite.open",
            "std.sqlite.execute",
            "std.sqlite.execute",
            "std.sqlite.execute",
            "std.sqlite.execute",
            "std.sqlite.execute",
            "std.console.print",
            "std.sqlite.query",
            "std.console.print",
            "std.console.print",
            "std.sqlite.query",
        ]
    );
    assert_eq!(
        recording.calls[0].capability,
        format!("filesystem(\"{dir}/inventory.db\")")
    );
    assert_eq!(
        recording.calls[7].capability,
        format!("filesystem.read(\"{dir}/inventory.db\")")
    );

    // the same run from the recording: no file is touched
    std::fs::remove_file(format!("{dir}/inventory.db")).unwrap();
    let stdout = Capture::default();
    let replayed = run(
        &program,
        &stdout,
        Options {
            replay: Some(recording),
            ..Options::default()
        },
    );
    assert_eq!(
        replayed.outcome,
        RunOutcome::Failed(
            "Mismatch(column: \"quantity\", expected: \"Text\", found: \"INTEGER\")".to_string()
        )
    );
    assert!(replayed.unused.is_empty());
    assert_eq!(stdout.text(), "");
    assert!(!std::path::Path::new(&format!("{dir}/inventory.db")).exists());
}

#[test]
fn a_row_outside_a_refinement_is_a_mismatch() {
    let dir = scratch("sqlite-refined");
    let _ = std::fs::remove_file(format!("{dir}/counts.db"));
    let source = format!(
        r#"module counts
  purpose: Rows must satisfy the record's conditions.

import std.console
import std.filesystem exposing Path
import std.sqlite exposing DbError, FromRow

type Positive
  has name: Text
  has amount: Integer where amount is at least 1
  can FromRow
end

public function main() or fails with DbError needs console, filesystem
  purpose: Insert a zero and read it back as a Positive.

  let connection be sqlite.open(Path("{dir}/counts.db")) otherwise fail
  ignore connection.execute(sql: "CREATE TABLE t (name TEXT, amount INTEGER)", parameters: []) otherwise fail
  ignore connection.execute(sql: "INSERT INTO t VALUES ('a', 0)", parameters: []) otherwise fail
  let rows: List of Positive be connection.query(sql: "SELECT * FROM t", parameters: [])
    otherwise fail
  console.print("{{rows.length()}} rows")
  connection.close()
end
"#
    );
    let program = compile("counts.ry", &source);
    let stdout = Capture::default();
    let outcome = run(&program, &stdout, Options::default()).outcome;
    assert_eq!(
        outcome,
        RunOutcome::Failed(
            "Mismatch(column: \"Positive\", expected: \"a row where amount is at least 1\", found: \"a row outside the condition\")".to_string()
        )
    );
    assert_eq!(stdout.text(), "");
}
