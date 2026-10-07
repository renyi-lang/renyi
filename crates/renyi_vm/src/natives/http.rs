//! `std.http`: a client over the `ureq` crate (decision S1). Redirects are
//! followed here one hop at a time, so that every host a request reaches
//! is checked against the grant: a redirect to a host outside it fails
//! with `HostNotAllowed`, as a direct request would.

use std::rc::Rc;
use std::time::Duration;

use indexmap::IndexMap;
use renyi_check::effects::{self, Capability};

use super::{arg, map, text, NativeFn};
use crate::grant::host_of;
use crate::natives::json::{self, Naming};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str) -> Option<NativeFn> {
    Some(match name {
        "get" => get,
        "get_with" => get_with,
        "post" => post,
        "post_json" => post_json,
        "put" => put,
        "delete" => delete,
        "request" => request,
        _ => return None,
    })
}

/// Each request has a 30-second limit (library sketch, section 8).
const LIMIT: Duration = Duration::from_secs(30);
const MAX_REDIRECTS: usize = 10;

fn get(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    perform(vm, "GET", &url, Vec::new(), None)
}

fn get_with(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    let headers = header_list(arg(args, 1))?;
    perform(vm, "GET", &url, headers, None)
}

fn post(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    let body = text(arg(args, 1))?.to_string();
    let headers = header_list(arg(args, 2))?;
    perform(vm, "POST", &url, headers, Some(body))
}

fn post_json(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    let value = arg(args, 1).clone();
    let encoded = json::encode(vm, &value, Naming::Exact)?;
    let mut body = String::new();
    json::write_json(&encoded, &mut body, None, 0);
    let headers = vec![("content-type".to_string(), "application/json".to_string())];
    perform(vm, "POST", &url, headers, Some(body))
}

fn put(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    let body = text(arg(args, 1))?.to_string();
    let headers = header_list(arg(args, 2))?;
    perform(vm, "PUT", &url, headers, Some(body))
}

fn delete(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let url = text(arg(args, 0))?.to_string();
    perform(vm, "DELETE", &url, Vec::new(), None)
}

fn request(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let method = text(arg(args, 0))?.to_ascii_uppercase();
    let url = text(arg(args, 1))?.to_string();
    let body = text(arg(args, 2))?.to_string();
    let headers = header_list(arg(args, 3))?;
    let body =
        (!body.is_empty() || !matches!(method.as_str(), "GET" | "HEAD" | "DELETE")).then_some(body);
    perform(vm, &method, &url, headers, body)
}

fn header_list(value: &Value) -> Result<Vec<(String, String)>, Interrupt> {
    let mut out = Vec::new();
    for (name, value) in map(value)?.iter() {
        out.push((text(name)?.to_string(), text(value)?.to_string()));
    }
    Ok(out)
}

fn error(vm: &Vm, variant: &str, fields: Vec<Value>) -> Result<Value, Interrupt> {
    vm.fail_variant("std.http", "HttpError", variant, fields)
}

/// One request and its redirects; the result is the `Response` record of
/// a 2xx answer, or the failure the sketch names.
fn perform(
    vm: &mut Vm,
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Option<String>,
) -> Result<Value, Interrupt> {
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(LIMIT))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut current_url = url.to_string();
    let mut current_method = method.to_string();
    let mut current_body = body;
    for _ in 0..=MAX_REDIRECTS {
        if let Some(host) = host_of(&current_url) {
            let needed = Capability {
                path: vec!["network".to_string(), "http".to_string()],
                scope: Some(host.clone()),
                budget: None,
                only_to: Vec::new(),
            };
            if !effects::covered(vm.effective_grant(), &needed, true) {
                return error(vm, "HostNotAllowed", vec![Value::text(host)]);
            }
        }
        let mut builder = ureq::http::Request::builder()
            .method(current_method.as_str())
            .uri(current_url.as_str());
        for (name, value) in &headers {
            builder = builder.header(name.as_str(), value.as_str());
        }
        let outgoing = match builder.body(current_body.clone().unwrap_or_default()) {
            Ok(request) => request,
            Err(detail) => {
                return error(
                    vm,
                    "Unreachable",
                    vec![Value::text(url), Value::text(detail.to_string())],
                )
            }
        };
        let response = match agent.run(outgoing) {
            Ok(response) => response,
            Err(ureq::Error::Timeout(_)) => return error(vm, "Timeout", vec![Value::text(url)]),
            Err(detail) => {
                return error(
                    vm,
                    "Unreachable",
                    vec![Value::text(url), Value::text(detail.to_string())],
                )
            }
        };
        let status = response.status().as_u16();
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            if let Some(location) = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
            {
                current_url = resolve(&current_url, location);
                if status == 303 || (matches!(status, 301 | 302) && current_method == "POST") {
                    current_method = "GET".to_string();
                    current_body = None;
                }
                continue;
            }
        }
        let (parts, mut body) = response.into_parts();
        let bytes = match body.read_to_vec() {
            Ok(bytes) => bytes,
            Err(detail) => {
                return error(
                    vm,
                    "Unreachable",
                    vec![Value::text(url), Value::text(detail.to_string())],
                )
            }
        };
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if !(200..300).contains(&status) {
            return error(
                vm,
                "Status",
                vec![
                    Value::text(url),
                    Value::integer(status as i64),
                    Value::text(text),
                ],
            );
        }
        let mut headers_map: IndexMap<Value, Value> = IndexMap::new();
        for (name, value) in parts.headers.iter() {
            let value = String::from_utf8_lossy(value.as_bytes()).into_owned();
            let key = Value::text(name.as_str());
            match headers_map.get_mut(&key) {
                Some(Value::Text(existing)) => {
                    *existing = Rc::from(format!("{existing}, {value}"));
                }
                _ => {
                    headers_map.insert(key, Value::text(value));
                }
            }
        }
        return vm.library_record(
            "std.http",
            "Response",
            vec![
                Value::integer(status as i64),
                Value::Map(Rc::new(headers_map)),
                Value::text(text),
                Value::Bytes(Rc::from(bytes)),
            ],
        );
    }
    error(
        vm,
        "Unreachable",
        vec![Value::text(url), Value::text("too many redirects")],
    )
}

/// A `Location` header against the URL it answered.
fn resolve(base: &str, location: &str) -> String {
    if location.contains("://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if location.starts_with('/') {
        return format!("{scheme}://{authority}{location}");
    }
    let path = rest.get(authority.len()..).unwrap_or("/");
    let path = path.split(['?', '#']).next().unwrap_or("/");
    let directory = match path.rfind('/') {
        Some(index) => &path[..=index],
        None => "/",
    };
    format!("{scheme}://{authority}{directory}{location}")
}

#[cfg(test)]
mod tests {
    use super::resolve;

    #[test]
    fn locations_resolve_against_the_request() {
        assert_eq!(
            resolve("https://a.example/v1/x?q=1", "/v2/y"),
            "https://a.example/v2/y"
        );
        assert_eq!(
            resolve("https://a.example/v1/x", "y?z=2"),
            "https://a.example/v1/y?z=2"
        );
        assert_eq!(
            resolve("https://a.example", "http://b.example/"),
            "http://b.example/"
        );
    }
}
