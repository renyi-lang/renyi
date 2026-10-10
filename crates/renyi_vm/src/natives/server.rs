//! `std.server`: an HTTP/1.1 server over `std::net` (decision S1), one
//! request at a time, each answered by the handler function; no TLS, which
//! a local service or one behind a proxy does not need.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use renyi_check::effects::Capability;

use super::{arg, crash, small, text};
use crate::extension::Native;
use crate::natives::json::{self, Naming};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// The natives of `std.server` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Native] = &[
    Native::function("std.server", "serve", serve),
    Native::function("std.server", "ok", ok),
    Native::function("std.server", "ok_json", ok_json),
    Native::function("std.server", "not_found", not_found),
    Native::function("std.server", "bad_request", bad_request),
    Native::function("std.server", "respond", respond),
    Native::method("std.server", "with_header", "Response", with_header),
];

/// Headers and bodies larger than these are refused with 400.
const MAX_HEAD: usize = 64 * 1024;
const MAX_BODY: usize = 16 * 1024 * 1024;

/// Under a watch (decision AO1): how often `serve` asks it between
/// requests, and how long it sleeps between looks at the socket.
const WATCH_INTERVAL: Duration = Duration::from_millis(500);
const WATCH_SLEEP: Duration = Duration::from_millis(20);

fn response(
    vm: &Vm,
    status: i64,
    headers: Vec<(&str, String)>,
    body: String,
) -> Result<Value, Interrupt> {
    let mut map = IndexMap::new();
    for (name, value) in headers {
        map.insert(Value::text(name), Value::text(value));
    }
    vm.library_record(
        "std.server",
        "Response",
        vec![
            Value::integer(status),
            Value::Map(Rc::new(map)),
            Value::text(body),
        ],
    )
}

fn ok(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let body = text(arg(args, 0))?.to_string();
    response(
        vm,
        200,
        vec![("content-type", "text/plain; charset=utf-8".to_string())],
        body,
    )
}

fn ok_json(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let value = arg(args, 0).clone();
    let encoded = json::encode(vm, &value, Naming::Exact)?;
    let mut body = String::new();
    json::write_json(&encoded, &mut body, None, 0);
    response(
        vm,
        200,
        vec![("content-type", "application/json".to_string())],
        body,
    )
}

fn not_found(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    response(
        vm,
        404,
        vec![("content-type", "text/plain; charset=utf-8".to_string())],
        "not found".to_string(),
    )
}

fn bad_request(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let detail = text(arg(args, 0))?.to_string();
    response(
        vm,
        400,
        vec![("content-type", "text/plain; charset=utf-8".to_string())],
        detail,
    )
}

fn respond(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let status = small(arg(args, 0))?;
    let body = text(arg(args, 1))?.to_string();
    response(vm, status, Vec::new(), body)
}

fn with_header(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let Value::Record(record) = arg(args, 0) else {
        return Err(crash("`with_header` needs a Response"));
    };
    let name = text(arg(args, 1))?.to_string();
    let value = text(arg(args, 2))?.to_string();
    let mut record = record.clone();
    let Some(index) = vm.program.types.field_index(record.ty, "headers") else {
        return Err(crash("a Response has no headers field"));
    };
    let mut headers = match record.fields.get(index) {
        Some(Value::Map(map)) => (**map).clone(),
        _ => IndexMap::new(),
    };
    headers.insert(Value::text(name), Value::text(value));
    record.make_mut().fields[index] = Value::Map(Rc::new(headers));
    Ok(Value::Record(record))
}

/// What one connection sent.
struct Incoming {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Listen on the port and answer every request with the handler until the
/// process stops, or until the VM's `serve_limit` requests were answered;
/// under a watch (decision AO1), until the watch says a new version is
/// ready between two requests: the listener is left in the VM for the
/// next version and the run stops with `Interrupt::Reload`.
fn serve(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let port = small(arg(args, 0))?;
    let Value::Function(handler) = arg(args, 1).clone() else {
        return Err(crash("`server.serve` needs a handler function"));
    };
    // the socket the previous version handed over, when it is this port's
    let handed = vm.listener.take().filter(|listener| {
        listener
            .local_addr()
            .is_ok_and(|address| address.port() == port as u16)
    });
    let listener = match handed {
        Some(listener) => listener,
        None => match TcpListener::bind(("0.0.0.0", port as u16)) {
            Ok(listener) => listener,
            Err(error) => {
                let variant = match error.kind() {
                    ErrorKind::AddrInUse => "PortInUse",
                    ErrorKind::PermissionDenied => "PermissionDenied",
                    _ => return Err(crash(format!("cannot listen on port {port}: {error}"))),
                };
                return vm.fail_variant(
                    "std.server",
                    "StartError",
                    variant,
                    vec![Value::integer(port)],
                );
            }
        },
    };
    // under a watch the loop looks away from the socket every half second
    // to ask the watch, because a blocking accept would hold a new
    // version back until the next request came in
    let watching = vm.watch.is_some();
    if watching && listener.set_nonblocking(true).is_err() {
        return Err(crash(format!("cannot poll the socket of port {port}")));
    }
    let mut asked = Instant::now();
    let mut served = 0usize;
    while vm.serve_limit.is_none_or(|limit| served < limit) {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if asked.elapsed() >= WATCH_INTERVAL {
                    asked = Instant::now();
                    if vm.watch.as_mut().is_some_and(|ready| ready()) {
                        vm.listener = Some(listener);
                        return Err(Interrupt::Reload);
                    }
                }
                // a short sleep between looks, because a spin on a
                // non-blocking accept would burn a core for nothing
                std::thread::sleep(WATCH_SLEEP);
                continue;
            }
            Err(_) => continue,
        };
        if watching {
            // the request itself is read and answered as without a watch
            let _ = stream.set_nonblocking(false);
        }
        let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
        let (status, headers, body) = match read_request(&mut stream) {
            Ok(Some(incoming)) => {
                let request = request_record(vm, incoming)?;
                let answer = vm.call_function(handler, vec![request])?;
                // a response built from guarded data leaves through the
                // socket (decision P3): refused with the guard's message
                // in the body, which names no data
                match vm.refusal(answer.origins(), &socket_effect(), "the response") {
                    Some(message) => (500, Vec::new(), message),
                    None => response_parts(vm, answer.plain()),
                }
            }
            Ok(None) => continue,
            Err(detail) => (400, Vec::new(), detail),
        };
        let _ = write_response(&mut stream, status, &headers, &body);
        served += 1;
    }
    Ok(Value::Nothing)
}

/// What sending a response exercises: `network.socket`, the server's own
/// capability.
fn socket_effect() -> Capability {
    Capability {
        path: vec!["network".to_string(), "socket".to_string()],
        scope: None,
        budget: None,
        only_to: Vec::new(),
    }
}

/// The `Request` record of what came in: the method, the decoded path, the
/// query parameters, the headers (names lower-cased) and the body as text.
fn request_record(vm: &Vm, incoming: Incoming) -> Result<Value, Interrupt> {
    let (path, query_text) = match incoming.target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (incoming.target.as_str(), ""),
    };
    let mut query = IndexMap::new();
    for pair in query_text.split('&').filter(|pair| !pair.is_empty()) {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        query.insert(
            Value::text(percent_decode(name)),
            Value::text(percent_decode(value)),
        );
    }
    let mut headers = IndexMap::new();
    for (name, value) in &incoming.headers {
        headers.insert(Value::text(name), Value::text(value));
    }
    vm.library_record(
        "std.server",
        "Request",
        vec![
            Value::text(incoming.method),
            Value::text(percent_decode(path)),
            Value::Map(Rc::new(query)),
            Value::Map(Rc::new(headers)),
            Value::text(String::from_utf8_lossy(&incoming.body)),
        ],
    )
}

/// The status, headers and body of the handler's `Response`.
fn response_parts(vm: &Vm, answer: &Value) -> (i64, Vec<(String, String)>, String) {
    let Value::Record(record) = answer else {
        return (
            500,
            Vec::new(),
            "the handler produced no response".to_string(),
        );
    };
    let types = &vm.program.types;
    let field = |name: &str| {
        types
            .field_index(record.ty, name)
            .and_then(|index| record.fields.get(index))
    };
    let status = field("status").and_then(Value::as_i64).unwrap_or(500);
    let mut headers = Vec::new();
    if let Some(Value::Map(map)) = field("headers") {
        for (name, value) in map.iter() {
            if let (Some(name), Some(value)) = (name.as_text(), value.as_text()) {
                headers.push((name.to_string(), value.to_string()));
            }
        }
    }
    let body = field("body")
        .and_then(Value::as_text)
        .unwrap_or("")
        .to_string();
    (status, headers, body)
}

/// Read one request: `None` when the connection closed without sending
/// one; an error names what was wrong with it.
fn read_request(stream: &mut TcpStream) -> Result<Option<Incoming>, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(index) = find(&buffer, b"\r\n\r\n") {
            break index;
        }
        if buffer.len() > MAX_HEAD {
            return Err("the request headers are too long".to_string());
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                if buffer.is_empty() {
                    return Ok(None);
                }
                return Err("the request ended inside its headers".to_string());
            }
            Ok(count) => buffer.extend_from_slice(&chunk[..count]),
            Err(error) => return Err(format!("cannot read the request: {error}")),
        }
    };
    let head = String::from_utf8_lossy(&buffer[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err("malformed request line".to_string());
    };
    if !version.starts_with("HTTP/1.") {
        return Err(format!("unsupported protocol {version}"));
    }
    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            return Err(format!("malformed header line `{line}`"));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let length: usize = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    if length > MAX_BODY {
        return Err("the request body is too large".to_string());
    }
    let mut body = buffer[head_end + 4..].to_vec();
    while body.len() < length {
        match stream.read(&mut chunk) {
            Ok(0) => return Err("the request ended inside its body".to_string()),
            Ok(count) => body.extend_from_slice(&chunk[..count]),
            Err(error) => return Err(format!("cannot read the request: {error}")),
        }
    }
    body.truncate(length);
    Ok(Some(Incoming {
        method: method.to_string(),
        target: target.to_string(),
        headers,
        body,
    }))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn write_response(
    stream: &mut TcpStream,
    status: i64,
    headers: &[(String, String)],
    body: &str,
) -> std::io::Result<()> {
    let mut out = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    let mut has_type = false;
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("content-type") {
            has_type = true;
        }
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    if !has_type && !body.is_empty() {
        out.push_str("content-type: text/plain; charset=utf-8\r\n");
    }
    out.push_str(&format!("content-length: {}\r\n", body.len()));
    out.push_str("connection: close\r\n\r\n");
    stream.write_all(out.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()
}

fn reason(status: i64) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

/// `%XX` escapes and `+` as a space.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        index += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::percent_decode;

    #[test]
    fn escapes_decode() {
        assert_eq!(percent_decode("a%20b+c%2Fd"), "a b c/d");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }
}
