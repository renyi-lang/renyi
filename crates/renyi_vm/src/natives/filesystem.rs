//! `std.filesystem`: paths are text; errors map to the `FileError` variants.

use std::rc::Rc;

use super::{arg, bytes, text, NativeFn};
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(name: &str, head: Option<&str>) -> Option<NativeFn> {
    Some(match (head, name) {
        (Some("Path"), "name") => path_name,
        (Some("Path"), "parent") => path_parent,
        (Some("Path"), "join") => path_join,
        (Some("Path"), "extension") => path_extension,
        (_, "read_text") => read_text,
        (_, "read_bytes") => read_bytes,
        (_, "write_text") => write_text,
        (_, "write_bytes") => write_bytes,
        (_, "append_text") => append_text,
        (_, "exists") => exists,
        (_, "inspect") => inspect,
        (_, "list") => list_directory,
        (_, "create_directory") => create_directory,
        (_, "remove") => remove,
        (_, "copy") => copy,
        (_, "move") => move_path,
        _ => return None,
    })
}

fn segments(path: &str) -> Vec<&str> {
    path.split(['/', '\\'])
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn path_name(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    Ok(Value::text(segments(path).last().copied().unwrap_or("")))
}

fn path_parent(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    let trimmed = path.trim_end_matches(['/', '\\']);
    Ok(match trimmed.rfind(['/', '\\']) {
        Some(0) => Value::text(&trimmed[..1]),
        Some(index) => Value::text(&trimmed[..index]),
        None => Value::Nothing,
    })
}

fn path_join(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    let segment = text(arg(&args, 1))?;
    if path.is_empty() {
        return Ok(Value::text(segment));
    }
    let base = path.trim_end_matches(['/', '\\']);
    Ok(Value::text(format!("{base}/{segment}")))
}

fn path_extension(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
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

fn read_text(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    match std::fs::read_to_string(path) {
        Ok(content) => Ok(Value::text(content)),
        Err(error) => file_error(vm, path, error),
    }
}

fn read_bytes(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    match std::fs::read(path) {
        Ok(content) => Ok(Value::Bytes(Rc::from(content))),
        Err(error) => file_error(vm, path, error),
    }
}

fn write_text(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    match std::fs::write(path, text(arg(&args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn write_bytes(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    match std::fs::write(path, bytes(arg(&args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn append_text(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    use std::io::Write;
    let path = text(arg(&args, 0))?;
    let content = text(arg(&args, 1))?;
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

fn exists(_: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    Ok(Value::Boolean(
        std::path::Path::new(text(arg(&args, 0))?).exists(),
    ))
}

fn inspect(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
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

fn list_directory(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
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

fn create_directory(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
    match std::fs::create_dir_all(path) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, path, error),
    }
}

fn remove(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let path = text(arg(&args, 0))?;
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

fn copy(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let source = text(arg(&args, 0))?;
    match std::fs::copy(source, text(arg(&args, 1))?) {
        Ok(_) => Ok(Value::Nothing),
        Err(error) => file_error(vm, source, error),
    }
}

fn move_path(vm: &mut Vm, args: Vec<Value>) -> Result<Value, Interrupt> {
    let source = text(arg(&args, 0))?;
    match std::fs::rename(source, text(arg(&args, 1))?) {
        Ok(()) => Ok(Value::Nothing),
        Err(error) => file_error(vm, source, error),
    }
}
