//! `std.console`, `std.environment` and `std.random`.

use std::rc::Rc;

use num_bigint::BigInt;

use super::{arg, crash, list, small, text, NativeFn};
use crate::decimal::Decimal;
use crate::value::Value;
use crate::vm::{Interrupt, Vm};

pub fn lookup(module: &str, name: &str) -> Option<NativeFn> {
    Some(match (module, name) {
        ("std.console", "print") => console_print,
        ("std.console", "print_error") => console_print_error,
        ("std.console", "read_line") => console_read_line,
        ("std.environment", "arguments") => environment_arguments,
        ("std.environment", "get") => environment_get,
        ("std.environment", "current_directory") => environment_current_directory,
        ("std.environment", "exit") => environment_exit,
        ("std.random", "integer") => random_integer,
        ("std.random", "decimal") => random_decimal,
        ("std.random", "choice") => random_choice,
        ("std.random", "shuffled") => random_shuffled,
        _ => return None,
    })
}

// ----------------------------------------------------------------- console

fn console_print(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let line = text(arg(args, 0))?;
    writeln!(vm.stdout, "{line}").map_err(|error| crash(format!("cannot write: {error}")))?;
    Ok(Value::Nothing)
}

fn console_print_error(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let line = text(arg(args, 0))?;
    writeln!(vm.stderr, "{line}").map_err(|error| crash(format!("cannot write: {error}")))?;
    Ok(Value::Nothing)
}

fn console_read_line(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    let mut line = String::new();
    match vm.stdin.read_line(&mut line) {
        Ok(0) => Ok(Value::Nothing),
        Ok(_) => {
            let trimmed = line.strip_suffix('\n').unwrap_or(&line);
            let trimmed = trimmed.strip_suffix('\r').unwrap_or(trimmed);
            Ok(Value::text(trimmed))
        }
        Err(error) => Err(crash(format!("cannot read: {error}"))),
    }
}

// ------------------------------------------------------------- environment

fn environment_arguments(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::list(vm.arguments.iter().map(Value::text).collect()))
}

fn environment_get(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(match std::env::var(text(arg(args, 0))?) {
        Ok(value) => Value::text(value),
        Err(_) => Value::Nothing,
    })
}

fn environment_current_directory(_: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    std::env::current_dir()
        .map(|path| Value::text(path.display().to_string()))
        .map_err(|error| crash(format!("cannot find the current directory: {error}")))
}

fn environment_exit(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Err(Interrupt::Exit(small(arg(args, 0))? as i32))
}

// ------------------------------------------------------------------ random

fn next_random(vm: &mut Vm) -> u64 {
    // xorshift64*: enough for choices and shuffles, replaced by a seeded
    // generator when recorded runs arrive
    let mut x = vm.random_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    vm.random_state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

fn random_integer(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let lowest = small(arg(args, 0))?;
    let highest = small(arg(args, 1))?;
    if highest < lowest {
        return Err(crash("`random.integer` needs lowest at most highest"));
    }
    let span = (highest as i128 - lowest as i128 + 1) as u128;
    let offset = (next_random(vm) as u128) % span;
    Ok(Value::integer((lowest as i128 + offset as i128) as i64))
}

fn random_decimal(vm: &mut Vm, _: &mut [Value]) -> Result<Value, Interrupt> {
    let digits = next_random(vm) % 1_000_000_000;
    Decimal::new(BigInt::from(digits), -9)
        .map(Value::decimal)
        .map_err(|_| crash("cannot build a random Decimal"))
}

fn random_choice(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let items = list(arg(args, 0))?.clone();
    if items.is_empty() {
        return Ok(Value::Nothing);
    }
    let index = (next_random(vm) % items.len() as u64) as usize;
    Ok(items[index].clone())
}

fn random_shuffled(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let mut items: Vec<Value> = (**list(arg(args, 0))?).clone();
    for index in (1..items.len()).rev() {
        let other = (next_random(vm) % (index as u64 + 1)) as usize;
        items.swap(index, other);
    }
    Ok(Value::List(Rc::new(items)))
}
