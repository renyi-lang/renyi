//! `std.filesystem`: paths are text; errors map to the `FileError` variants.

use std::rc::Rc;

use super::{arg, bytes, text};
use crate::extension::Native;
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// The natives of `std.filesystem` (decision AK2): the declared function
/// each implements, by module, name and the type of its first parameter.
pub(crate) const NATIVES: &[Native] = &[
    Native::method("std.filesystem", "name", "Path", path_name),
    Native::method("std.filesystem", "parent", "Path", path_parent),
    Native::method("std.filesystem", "join", "Path", path_join),
    Native::method("std.filesystem", "extension", "Path", path_extension),
    Native::function("std.filesystem", "read_text", read_text),
    Native::function("std.filesystem", "read_bytes", read_bytes),
    Native::function("std.filesystem", "write_text", write_text),
    Native::function("std.filesystem", "write_bytes", write_bytes),
    Native::function("std.filesystem", "append_text", append_text),
    Native::function("std.filesystem", "exists", exists),
    Native::function("std.filesystem", "inspect", inspect),
    Native::function("std.filesystem", "list", list_directory),
    Native::function("std.filesystem", "create_directory", create_directory),
    Native::function("std.filesystem", "remove", remove),
    Native::function("std.filesystem", "copy", copy),
    Native::function("std.filesystem", "move", move_path),
];

fn segments(path: &str) -> Vec<&str> {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn path_name(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    Ok(Value::text(segments(path).last().copied().unwrap_or("")))
}

fn path_parent(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    let trimmed = path.trim_end_matches(['/', '\\']);
    Ok(match trimmed.rfind(['/', '\\']) {
        Some(0) => Value::text(&trimmed[..1]),
        Some(index) => Value::text(&trimmed[..index]),
        None => Value::Nothing,
    })
}

fn path_join(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    let segment = text(arg(args, 1))?;
    if path.is_empty() {
        return Ok(Value::text(segment));
    }
    let base = path.trim_end_matches(['/', '\\']);
    Ok(Value::text(format!("{base}/{segment}")))
}

fn path_extension(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    let name = segments(path).last().copied().unwrap_or("");
    Ok(match name.rfind('.') {
        Some(index) if index > 0 && index + 1 < name.len() => Value::text(&name[index + 1..]),
        _ => Value::Nothing,
    })
}

fn file_error(vm: &Vm, path: &str, error: std::io::Error) -> Result<Value, Interrupt> {
    use std::io::ErrorKind;
    let (variant, fields) = match error.kind() {
        ErrorKind::NotFound => ("NotFound", vec![Value::text(path)]),
        ErrorKind::PermissionDenied => ("PermissionDenied", vec![Value::text(path)]),
        ErrorKind::AlreadyExists => ("AlreadyExists", vec![Value::text(path)]),
        ErrorKind::InvalidData => ("InvalidEncoding", vec![Value::text(path)]),
        _ => (
            "Io",
            vec![Value::text(path), Value::text(error.to_string())],
        ),
    };
    vm.fail_variant("std.filesystem", "FileError", variant, fields)
}

fn read_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::text(content)),
        Err(error) => file_error(vm, path, error),
    }
}

fn read_bytes(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::read(path) {
        Ok(content) => Ok(Value::Bytes(Rc::from(content))),
        Err(error) => file_error(vm, path, error),
    }
}

fn write_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::write(path, text(arg(args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn write_bytes(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::write(path, bytes(arg(args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn append_text(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    use std::io::Write;
    let path = text(arg(args, 0))?;
    let content = text(arg(args, 1))?;
    let result = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .and_then(|mut file| file.write_all(content.as_bytes()));
    match result {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn exists(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        std::path::Path::new(text(arg(args, 0))?).exists(),
    ))
}

fn inspect(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::metadata(path) {
        Ok(metadata) => {
            if metadata.is_file() {
                vm.library_variant(
                    "std.filesystem",
                    "Entry",
                    "File",
                    vec![Value::integer(metadata.len() as i64)],
                )
            } else if metadata.is_dir() {
                vm.library_variant("std.filesystem", "Entry", "Directory", Vec::new())
            } else {
                vm.library_variant("std.filesystem", "Entry", "Other", Vec::new())
            }
        }
        Err(error) => file_error(vm, path, error),
    }
}

fn list_directory(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) => return file_error(vm, path, error),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => names.push(entry.file_name().to_string_lossy().to_string()),
            Err(error) => return file_error(vm, path, error),
        }
    }
    names.sort();
    let base = path.trim_end_matches(['/', '\\']);
    Ok(Value::list(
        names
            .into_iter()
            .map(|name| {
                if base.is_empty() {
                    Value::text(name)
                } else {
                    Value::text(format!("{base}/{name}"))
                }
            })
            .collect(),
    ))
}

fn create_directory(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    match std::fs::create_dir_all(path) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn remove(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?;
    let result = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => std::fs::remove_dir(path),
        Ok(_) => std::fs::remove_file(path),
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn copy(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let source = text(arg(args, 0))?;
    match std::fs::copy(source, text(arg(args, 1))?) {
        Ok(_) => Ok(Value::Nothing),
        Err(error) => file_error(vm, source, error),
    }
}

fn move_path(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let source = text(arg(args, 0))?;
    match std::fs::rename(source, text(arg(args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, source, error),
    }
}
