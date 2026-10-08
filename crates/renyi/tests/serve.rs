//! `renyi serve --watch` (decisions Q4 and AO1) through the binary: a
//! service answers, its file is edited and the next request is answered
//! by the new version, a version with errors is reported while the last
//! good one keeps serving, a version with a definition added reloads
//! with the diff in the message; and what the command refuses.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .expect("the workspace root")
        .to_path_buf()
}

/// A scratch project under the workspace's `target`, clean.
fn project(name: &str) -> PathBuf {
    let directory = root().join("target/serve").join(name);
    if directory.exists() {
        std::fs::remove_dir_all(&directory).expect("a clean project directory");
    }
    std::fs::create_dir_all(&directory).expect("the project directory");
    directory
}

/// The service: `/` answers with the version, anything else with 404.
fn service(port: u16, answer: &str, extra: &str) -> String {
    format!(
        "module app
  purpose: A service for the test of the watch.

import std.server exposing Request, Response, Port, StartError
{extra}
function handle(request: Request) returns Response
  purpose: Answer the root with the version.

  match request.path
    when \"/\" then return {answer}
    otherwise return server.not_found()
  end
end

public function main() or fails with StartError needs network.socket
  purpose: Serve until the process stops.

  server.serve(port: Port({port}), handler: handle) otherwise fail
end
"
    )
}

/// The child is killed when the test ends, passed or failed.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// One request to the root; the body when the server answered 200.
fn get(port: u16) -> Option<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .ok()?;
    stream
        .write_all(b"GET / HTTP/1.1\r\nhost: localhost\r\nconnection: close\r\n\r\n")
        .ok()?;
    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;
    let text = String::from_utf8(response).ok()?;
    let (head, body) = text.split_once("\r\n\r\n")?;
    head.starts_with("HTTP/1.1 200").then(|| body.to_string())
}

fn wait_for(what: &str, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(
            started.elapsed() < Duration::from_secs(90),
            "timed out waiting for {what}"
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

#[test]
fn a_service_reloads_between_requests_and_keeps_serving_over_a_broken_version() {
    let port = 20000 + (std::process::id() % 2000) as u16;
    let directory = project("reload");
    let file = directory.join("app.ry");
    let log = directory.join("serve.log");
    std::fs::write(&file, service(port, "server.ok(\"v1\")", "")).expect("app.ry");
    let child = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(&directory)
        .args(["serve", "--watch", "app.ry"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(std::fs::File::create(&log).expect("the log"))
        .spawn()
        .expect("the renyi binary runs");
    let _server = Server(child);
    let logged = || std::fs::read_to_string(&log).unwrap_or_default();

    wait_for("the first version", || get(port).is_some());
    assert_eq!(get(port).as_deref(), Some("v1"));
    assert!(
        logged().contains("renyi serve: watching the current directory for changes"),
        "{}",
        logged()
    );

    // a body edited: the next request is answered by the new version
    std::fs::write(&file, service(port, "server.ok(\"v2\")", "")).expect("app.ry");
    wait_for("the second version", || get(port).as_deref() == Some("v2"));
    assert!(
        logged().contains(
            "renyi serve: reloading app.ry: 1 definition changed: app.handle (body changed)\n"
        ),
        "{}",
        logged()
    );

    // a version with an error: reported, the last good one keeps serving
    std::fs::write(&file, service(port, "server.ok(nothing_here)", "")).expect("app.ry");
    wait_for("the error report", || {
        logged()
            .contains("renyi serve: the new version has errors; still serving the last good one:\n")
    });
    assert!(logged().contains("unknown-name"), "{}", logged());
    assert_eq!(get(port).as_deref(), Some("v2"));

    // a definition added and the handler changed: both in the message
    let version = "
function version() returns Text
  purpose: The version served.

  return \"v3\"
end
";
    std::fs::write(&file, service(port, "server.ok(version())", version)).expect("app.ry");
    wait_for("the third version", || get(port).as_deref() == Some("v3"));
    assert!(
        logged().contains(
            "renyi serve: reloading app.ry: 2 definitions changed: app.handle (body changed); app.version (added)\n"
        ),
        "{}",
        logged()
    );
    // the error was reported once, and nothing crashed
    assert_eq!(logged().matches("still serving").count(), 1, "{}", logged());
    assert!(!logged().contains("crash"), "{}", logged());
}

#[test]
fn the_command_refuses_a_first_version_with_errors_and_run_refuses_the_flag() {
    let directory = project("refused");
    let file = directory.join("app.ry");
    std::fs::write(&file, service(1, "server.ok(nothing_here)", "")).expect("app.ry");
    let refused = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(&directory)
        .args(["serve", "--watch", "app.ry"])
        .output()
        .expect("the renyi binary runs");
    assert_eq!(refused.status.code(), Some(1));
    assert!(
        text(&refused.stdout).contains("unknown-name"),
        "{}",
        text(&refused.stdout)
    );

    let run = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(&directory)
        .args(["run", "--watch", "app.ry"])
        .output()
        .expect("the renyi binary runs");
    assert_eq!(run.status.code(), Some(1));
    assert_eq!(
        text(&run.stderr),
        "renyi: `--watch` is an option of `renyi serve`\n"
    );

    // without `--watch`, `serve` is `run`
    let plain = Command::new(env!("CARGO_BIN_EXE_renyi"))
        .current_dir(root())
        .args(["serve", "examples/hello.ry", "Renyi"])
        .output()
        .expect("the renyi binary runs");
    assert!(plain.status.success(), "{}", text(&plain.stderr));
    assert_eq!(text(&plain.stdout), "Hello, Renyi!\n");
}
