//! The foreign function interface (decision AF1): a function of a foreign
//! module is bound at its first call to a symbol of one of the libraries
//! the manifest names, loaded through `libloading`; a call marshals the
//! arguments into the words of the fixed signature family of
//! `foreign_abi`, calls, and reads the result back by the declared C type.
//! This module and `foreign_abi` hold the VM's only unsafe code: what the
//! callee does is outside every guarantee of the VM (decision Q3,
//! `07-system-design.md` section 4.3).

use std::ffi::{c_char, c_void, CStr, CString};

use num_bigint::BigInt;
use renyi_check::foreign::{CResult, CType};
use renyi_check::FunctionId;

use super::foreign_abi::{call, Returned, Returns, Slot, MAX_ARGS};
use super::{boolean, bytes, crash, float, int, text};
use crate::integer::Int;
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

/// A foreign function bound to its symbol: the address and the C
/// signature read off the bytecode's binding.
#[derive(Clone, Debug)]
pub struct Bound {
    address: usize,
    parameters: Vec<CType>,
    result: CResult,
}

/// Call the foreign function with the arguments; its binding is made at
/// the first call and kept.
pub fn run(
    vm: &mut Vm,
    function: FunctionId,
    origins: u64,
    args: &[Value],
) -> Result<Value, Interrupt> {
    let bound = match vm.foreign.get(function).cloned().flatten() {
        Some(bound) => bound,
        None => {
            let bound = bind(vm, function)?;
            vm.foreign[function] = Some(bound.clone());
            bound
        }
    };
    let value = invoke(vm, function, &bound, args)?;
    Ok(value.guarded(origins))
}

/// The symbol of the function in the first library of its list that
/// loads, and its signature.
fn bind(vm: &mut Vm, function: FunctionId) -> Result<Bound, Interrupt> {
    let qualified = vm.qualified(function);
    let meta = &vm.program.function_metas[function];
    let module = meta.module.clone();
    let binding = meta
        .foreign
        .clone()
        .ok_or_else(|| crash(format!("`{qualified}` is not a foreign function")))?;
    let unknown = |spelling: &str| {
        crash(format!(
            "`{qualified}`: the bytecode names the C type `{spelling}`, which this VM does not know"
        ))
    };
    let parameters = binding
        .parameters
        .iter()
        .map(|spelling| CType::parse(spelling).ok_or_else(|| unknown(spelling)))
        .collect::<Result<Vec<_>, _>>()?;
    let result = CResult::parse(&binding.result).ok_or_else(|| unknown(&binding.result))?;
    let library = load(vm, &module, &binding.libraries)?;
    // SAFETY: the symbol is read as an address only; it is called through
    // `foreign_abi::call` with the signature the binding declares, which is
    // the whole promise the FFI makes (decision AF1)
    let symbol: libloading::Symbol<unsafe extern "C" fn()> =
        unsafe { vm.libraries[library].1.get(binding.symbol.as_bytes()) }.map_err(|error| {
            crash(format!(
                "`{qualified}`: the symbol `{}` is not in the library: {error}",
                binding.symbol
            ))
        })?;
    let address = *symbol as *const c_void as usize;
    Ok(Bound {
        address,
        parameters,
        result,
    })
}

/// The index in the VM's libraries of the first of the names that loads;
/// a list already loaded is found again.
fn load(vm: &mut Vm, module: &str, libraries: &[String]) -> Result<usize, Interrupt> {
    if let Some(index) = vm
        .libraries
        .iter()
        .position(|(names, _)| names == libraries)
    {
        return Ok(index);
    }
    let mut tried = Vec::new();
    for name in libraries {
        // SAFETY: loading a library runs its initializers; what they do is
        // outside the VM's guarantees, as decision AF1 says of every foreign
        // call
        match unsafe { libloading::Library::new(name) } {
            Ok(library) => {
                vm.libraries.push((libraries.to_vec(), library));
                return Ok(vm.libraries.len() - 1);
            }
            Err(error) => tried.push(format!("`{name}`: {error}")),
        }
    }
    Err(crash(format!(
        "cannot load a library for the foreign module `{module}`: {}",
        tried.join("; ")
    )))
}

/// Marshal the arguments, call, and read the result back.
fn invoke(
    vm: &mut Vm,
    function: FunctionId,
    bound: &Bound,
    args: &[Value],
) -> Result<Value, Interrupt> {
    let qualified = vm.qualified(function);
    let names = &vm.program.function_metas[function].params;
    let mut slots: Vec<Slot> = Vec::with_capacity(MAX_ARGS);
    // the C strings live until the call has returned
    let mut texts: Vec<CString> = Vec::new();
    for (index, (ctype, value)) in bound.parameters.iter().zip(args).enumerate() {
        let name = names.get(index).map(String::as_str).unwrap_or("?");
        match ctype {
            CType::F64 => slots.push(Slot::Float(float(value)?)),
            CType::Bool => slots.push(Slot::Int(boolean(value)? as i64)),
            CType::Text => {
                let c_text = CString::new(text(value)?).map_err(|_| {
                    crash(format!(
                        "`{qualified}`: the argument `{name}` contains a NUL character, which a C string cannot hold"
                    ))
                })?;
                slots.push(Slot::Int(c_text.as_ptr() as i64));
                texts.push(c_text);
            }
            CType::Bytes => {
                let data = bytes(value)?;
                slots.push(Slot::Int(data.as_ptr() as i64));
                slots.push(Slot::Int(data.len() as i64));
            }
            width => {
                let word = word_of(int(value)?, *width).ok_or_else(|| {
                    crash(format!(
                        "`{qualified}`: the argument `{name}` does not fit a C {}",
                        width.spelling()
                    ))
                })?;
                slots.push(Slot::Int(word));
            }
        }
    }
    let returns = match bound.result {
        CResult::Void => Returns::Nothing,
        CResult::Value(CType::F64) => Returns::Float,
        _ => Returns::Int,
    };
    // SAFETY: the address came from a symbol of a loaded library and the
    // slots follow the binding's signature class for class; the callee is
    // outside the VM's guarantees (decision AF1)
    let returned =
        unsafe { call(bound.address as *const c_void, &slots, returns) }.ok_or_else(|| {
            crash(format!(
                "`{qualified}` takes {} words across the boundary; the VM calls at most {MAX_ARGS}",
                slots.len()
            ))
        })?;
    drop(texts);
    Ok(match (bound.result, returned) {
        (CResult::Void, _) => Value::Nothing,
        (CResult::Value(CType::F64), Returned::Float(value)) => Value::Float(value),
        (CResult::Value(CType::Bool), Returned::Int(word)) => Value::Boolean((word as u8) != 0),
        (CResult::Value(CType::Text), Returned::Int(word)) => {
            if word == 0 {
                return Err(crash(format!(
                    "`{qualified}` returned a null pointer where Text was declared"
                )));
            }
            text_at(word)
        }
        (CResult::TextOrNull, Returned::Int(word)) => {
            if word == 0 {
                Value::Nothing
            } else {
                text_at(word)
            }
        }
        (CResult::Value(width), Returned::Int(word)) => integer_of(word, width),
        _ => {
            return Err(crash(format!(
                "`{qualified}` returned a value of an unexpected class"
            )))
        }
    })
}

/// An integer as the word of its C width, sign- or zero-extended; `None`
/// when it does not fit.
fn word_of(value: &Int, width: CType) -> Option<i64> {
    let small = value.to_i64();
    match width {
        CType::I8 => small.filter(|v| i8::try_from(*v).is_ok()),
        CType::U8 => small.filter(|v| u8::try_from(*v).is_ok()),
        CType::I16 => small.filter(|v| i16::try_from(*v).is_ok()),
        CType::U16 => small.filter(|v| u16::try_from(*v).is_ok()),
        CType::I32 => small.filter(|v| i32::try_from(*v).is_ok()),
        CType::U32 => small.filter(|v| u32::try_from(*v).is_ok()),
        CType::I64 => small,
        CType::U64 | CType::Size => u64::try_from(value.to_big()).ok().map(|v| v as i64),
        _ => None,
    }
}

/// The word a C function returned, read by its width: the high bits of a
/// narrower result are undefined by the calling convention.
fn integer_of(word: i64, width: CType) -> Value {
    match width {
        CType::I8 => Value::integer(word as i8 as i64),
        CType::U8 => Value::integer(word as u8 as i64),
        CType::I16 => Value::integer(word as i16 as i64),
        CType::U16 => Value::integer(word as u16 as i64),
        CType::I32 => Value::integer(word as i32 as i64),
        CType::U32 => Value::integer(word as u32 as i64),
        CType::I64 => Value::integer(word),
        CType::U64 | CType::Size => Value::Integer(Int::from_big(BigInt::from(word as u64))),
        _ => Value::Nothing,
    }
}

/// The text a `char *` points at, decoded with replacement characters.
fn text_at(word: i64) -> Value {
    // SAFETY: the callee returned a pointer to a NUL-terminated string, as
    // the binding declares; the text is copied before the pointer is
    // dropped (decision AF1: outside the VM's guarantees)
    let c_text = unsafe { CStr::from_ptr(word as *const c_char) };
    Value::text(c_text.to_string_lossy())
}
