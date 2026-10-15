//! The embedding API for C hosts and the languages that call C
//! (decision AW46, stage 40a): a shared and a static library over
//! `renyi::Sandbox`, with the header `include/renyi.h`.
//!
//! A host loads a module with a grant (`renyi_load`, `renyi_load_source`)
//! and gets an opaque handle, one per module, freed by
//! `renyi_free_sandbox`; it calls a public function with the arguments as
//! one JSON array text (`renyi_call`, `renyi_call_guarded`) and reads the
//! value as one JSON text, both in the codec of `std.json`
//! (`Sandbox::call_json`); the outcome of a call is a status
//! (`RENYI_OK` to `RENYI_OVER_MEMORY`) and the handle's text
//! (`renyi_text`, `renyi_text_len`), valid until the next call on the
//! handle. A load that fails answers a null handle with the reason in
//! `renyi_last_error`. Texts cross as a pointer and a length, not
//! NUL-terminated, UTF-8. The library declares the counting allocator,
//! so a grant with `memory` works with no declaration of the host's; a
//! handle is for one thread at a time.

use std::cell::RefCell;

use renyi::{CallError, Function, Grant, Sandbox};
use renyi_json::{read_json, write_json, Json};

/// The counting allocator the memory budget needs (decision AP1): the
/// library's own, so the count sees the VM's allocations and not the
/// host's.
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;

/// The version of this API: additive changes keep it, a change to a
/// function or a status raises it.
pub const API_VERSION: i32 = 1;

/// The call answered its value; the value as JSON is the text.
pub const OK: i32 = 0;
/// The function failed; the failure rendered is the text.
pub const FAILED: i32 = 1;
/// The call crashed; the message and the location are the text.
pub const CRASHED: i32 = 2;
/// `environment.exit`; the code through `renyi_exit_code`.
pub const EXITED: i32 = 3;
/// Nothing ran: no such public function, the arity, a `needs` the grant
/// does not cover, an argument the codec could not decode; the reason
/// is the text.
pub const REFUSED: i32 = 4;
/// The memory budget was exceeded; the limit and the peak through
/// `renyi_memory_limit` and `renyi_memory_used`.
pub const OVER_MEMORY: i32 = 5;

/// A loaded module, and what its last call answered.
pub struct Handle {
    sandbox: Sandbox,
    text: Vec<u8>,
    exit_code: i32,
    memory_limit: u64,
    memory_used: u64,
}

thread_local! {
    /// Why the last load on this thread answered no handle.
    static LAST_ERROR: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

const VERSION: &str = concat!("renyi ", env!("CARGO_PKG_VERSION"));

/// The text at a pointer the host gave, copied out; an empty text for a
/// length of zero, whatever the pointer.
unsafe fn text_at(pointer: *const u8, len: usize) -> String {
    if len == 0 {
        return String::new();
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(pointer, len)).into_owned()
}

fn set_last_error(message: String) {
    LAST_ERROR.with(|error| *error.borrow_mut() = message.into_bytes());
}

/// The grant from its JSON text (`embedding.md` section 1); the empty
/// text grants nothing.
fn grant_of(text: &str) -> Result<Grant, String> {
    if text.trim().is_empty() {
        Ok(Grant::default())
    } else {
        Grant::from_json(text)
    }
}

fn opened(loaded: Result<Sandbox, String>) -> *mut Handle {
    match loaded {
        Ok(sandbox) => Box::into_raw(Box::new(Handle {
            sandbox,
            text: Vec::new(),
            exit_code: 0,
            memory_limit: 0,
            memory_used: 0,
        })),
        Err(message) => {
            set_last_error(message);
            std::ptr::null_mut()
        }
    }
}

impl Handle {
    /// A call's outcome kept for the host: the status, with the text,
    /// the exit code and the memory figures it carries.
    fn answer(&mut self, outcome: Result<String, CallError>) -> i32 {
        self.exit_code = 0;
        self.memory_limit = 0;
        self.memory_used = 0;
        let (status, text) = match outcome {
            Ok(value) => (OK, value),
            Err(CallError::Refused(message)) => (REFUSED, message),
            Err(CallError::Failed(message)) => (FAILED, message),
            Err(CallError::Crashed { message, location }) => (
                CRASHED,
                match location {
                    Some(location) => format!("{message} (at {location})"),
                    None => message,
                },
            ),
            Err(CallError::Exited(code)) => {
                self.exit_code = code;
                (EXITED, format!("exited with code {code}"))
            }
            Err(error @ CallError::OverMemory { limit, used }) => {
                self.memory_limit = limit;
                self.memory_used = used;
                (OVER_MEMORY, error.to_string())
            }
        };
        self.text = text.into_bytes();
        status
    }

    /// The public functions as a JSON array, one object each.
    fn functions_json(&self) -> String {
        let functions = self.sandbox.functions().iter().map(function_json).collect();
        let mut text = String::new();
        write_json(&Json::Array(functions), &mut text, None, 0);
        text
    }
}

fn function_json(function: &Function) -> Json {
    let text = |value: &str| Json::Text(value.to_string());
    let optional = |value: &Option<String>| value.as_deref().map_or(Json::Null, text);
    Json::Object(vec![
        ("name".to_string(), text(&function.name)),
        ("signature".to_string(), text(&function.signature)),
        ("purpose".to_string(), optional(&function.purpose)),
        (
            "parameters".to_string(),
            Json::Number(function.parameters.to_string()),
        ),
        (
            "needs".to_string(),
            Json::Array(function.needs.iter().map(|need| text(need)).collect()),
        ),
        (
            "exposed_as_tool".to_string(),
            Json::Boolean(function.exposed_as_tool),
        ),
        (
            "input_schema".to_string(),
            function
                .input_schema
                .as_deref()
                .and_then(|schema| read_json(schema).ok())
                .unwrap_or(Json::Null),
        ),
        ("callable".to_string(), Json::Boolean(function.callable)),
    ])
}

/// The guards of a call from their JSON text: an array with a
/// capability text or null per argument; the empty text for none.
fn guards_of(text: &str) -> Result<Vec<Option<String>>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let json = read_json(text)
        .map_err(|(detail, at)| format!("the guards do not parse as JSON: {detail} (at {at})"))?;
    let Json::Array(items) = json else {
        return Err(
            "the guards are a JSON array, a capability text or null per argument".to_string(),
        );
    };
    items
        .into_iter()
        .map(|item| match item {
            Json::Null => Ok(None),
            Json::Text(capability) => Ok(Some(capability)),
            other => Err(format!(
                "a guard is a capability text or null, not {}",
                other.kind()
            )),
        })
        .collect()
}

// ------------------------------------------------------------ the C API

/// This API's version, `RENYI_API_VERSION` of the header.
#[no_mangle]
pub extern "C" fn renyi_api_version() -> i32 {
    API_VERSION
}

/// The toolchain's version as a text, `renyi 0.2.1`: its bytes, valid
/// for the process's life.
#[no_mangle]
pub extern "C" fn renyi_version() -> *const u8 {
    VERSION.as_ptr()
}

#[no_mangle]
pub extern "C" fn renyi_version_len() -> usize {
    VERSION.len()
}

/// A module loaded from its source file with a grant, the grant as the
/// JSON text of `embedding.md` section 1 (the empty text grants
/// nothing): a handle, or null with the reason in `renyi_last_error`.
///
/// # Safety
/// The two texts lie at the pointers for their lengths.
#[no_mangle]
pub unsafe extern "C" fn renyi_load(
    path: *const u8,
    path_len: usize,
    grant: *const u8,
    grant_len: usize,
) -> *mut Handle {
    let (path, grant) = (text_at(path, path_len), text_at(grant, grant_len));
    opened(grant_of(&grant).and_then(|grant| Sandbox::load(&path, grant)))
}

/// A module loaded from a text, `name` the file name its imports resolve
/// from; otherwise as `renyi_load`.
///
/// # Safety
/// The three texts lie at the pointers for their lengths.
#[no_mangle]
pub unsafe extern "C" fn renyi_load_source(
    name: *const u8,
    name_len: usize,
    text: *const u8,
    text_len: usize,
    grant: *const u8,
    grant_len: usize,
) -> *mut Handle {
    let (name, text, grant) = (
        text_at(name, name_len),
        text_at(text, text_len),
        text_at(grant, grant_len),
    );
    opened(grant_of(&grant).and_then(|grant| Sandbox::load_source(&name, &text, grant)))
}

/// The handle released with its module; null is ignored.
///
/// # Safety
/// `handle` is what a load answered and was not freed before.
#[no_mangle]
pub unsafe extern "C" fn renyi_free_sandbox(handle: *mut Handle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Why the last load on this thread answered null: its bytes, valid
/// until the next load on the thread.
#[no_mangle]
pub extern "C" fn renyi_last_error() -> *const u8 {
    LAST_ERROR.with(|error| error.borrow().as_ptr())
}

#[no_mangle]
pub extern "C" fn renyi_last_error_len() -> usize {
    LAST_ERROR.with(|error| error.borrow().len())
}

/// The handle's text, what the last call on it answered: its bytes,
/// valid until the next call on the handle.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_text(handle: *const Handle) -> *const u8 {
    (*handle).text.as_ptr()
}

/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_text_len(handle: *const Handle) -> usize {
    (*handle).text.len()
}

/// A public function called with its arguments as one JSON array text:
/// the status, and the value as JSON or the reason in the text.
///
/// # Safety
/// `handle` is a live handle; the texts lie at the pointers for their
/// lengths.
#[no_mangle]
pub unsafe extern "C" fn renyi_call(
    handle: *mut Handle,
    name: *const u8,
    name_len: usize,
    arguments: *const u8,
    arguments_len: usize,
) -> i32 {
    renyi_call_guarded(
        handle,
        name,
        name_len,
        arguments,
        arguments_len,
        std::ptr::null(),
        0,
    )
}

/// `renyi_call` with guards (decision P3): a JSON array with, per
/// argument, the capability of the grant whose `only to` guard the
/// argument carries, or null; the empty text for no guards.
///
/// # Safety
/// `handle` is a live handle; the texts lie at the pointers for their
/// lengths.
#[no_mangle]
pub unsafe extern "C" fn renyi_call_guarded(
    handle: *mut Handle,
    name: *const u8,
    name_len: usize,
    arguments: *const u8,
    arguments_len: usize,
    guards: *const u8,
    guards_len: usize,
) -> i32 {
    let (name, arguments, guards) = (
        text_at(name, name_len),
        text_at(arguments, arguments_len),
        text_at(guards, guards_len),
    );
    let handle = &mut *handle;
    let outcome = match guards_of(&guards) {
        Ok(guards) => {
            let guards: Vec<Option<&str>> = guards.iter().map(Option::as_deref).collect();
            handle.sandbox.call_json(&name, &arguments, &guards)
        }
        Err(message) => Err(CallError::Refused(message)),
    };
    handle.answer(outcome)
}

/// The code of the last call on the handle that exited; zero otherwise.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_exit_code(handle: *const Handle) -> i32 {
    (*handle).exit_code
}

/// The memory budget the last call on the handle exceeded, in bytes;
/// zero otherwise.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_memory_limit(handle: *const Handle) -> u64 {
    (*handle).memory_limit
}

/// The most bytes that call held above the level at its start; zero
/// otherwise.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_memory_used(handle: *const Handle) -> u64 {
    (*handle).memory_used
}

/// The module's name, into the handle's text.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_module(handle: *mut Handle) {
    let handle = &mut *handle;
    handle.text = handle.sandbox.module().as_bytes().to_vec();
}

/// The grant's capabilities as a `needs` clause spells them, into the
/// handle's text.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_grant(handle: *mut Handle) {
    let handle = &mut *handle;
    handle.text = handle.sandbox.grant().spelling().into_bytes();
}

/// The checker's warnings on the module as `renyi check` prints them,
/// into the handle's text; empty when there are none.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_warnings(handle: *mut Handle) {
    let handle = &mut *handle;
    handle.text = handle.sandbox.warnings().as_bytes().to_vec();
}

/// The public functions of the module as a JSON array, one object each
/// (`name`, `signature`, `purpose`, `parameters`, `needs`,
/// `exposed_as_tool`, `input_schema`, `callable`), into the handle's
/// text.
///
/// # Safety
/// `handle` is a live handle.
#[no_mangle]
pub unsafe extern "C" fn renyi_functions(handle: *mut Handle) {
    let handle = &mut *handle;
    handle.text = handle.functions_json().into_bytes();
}
