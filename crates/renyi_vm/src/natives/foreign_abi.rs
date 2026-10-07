//! The fixed family of C signatures the VM can call (decision AF1), written
//! by `tools/gen_foreign_abi.py`: up to six arguments, each an integer-class
//! word (an integer, a Boolean, a pointer) or a double, returning nothing,
//! an integer-class word or a double. Each arm casts the symbol to the
//! Rust function type of that exact shape, so that both the System V and
//! the Windows x64 conventions place every argument where the C function
//! reads it. Do not edit by hand: change the script and run it, then
//! `cargo fmt`.

use std::ffi::c_void;

/// One argument as the C calling convention classes it.
#[derive(Clone, Copy, Debug)]
pub enum Slot {
    Int(i64),
    Float(f64),
}

/// What a signature returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returns {
    Nothing,
    Int,
    Float,
}

/// What a call returned.
#[derive(Clone, Copy, Debug)]
pub enum Returned {
    Nothing,
    Int(i64),
    Float(f64),
}

/// The most arguments a signature of the family takes.
pub const MAX_ARGS: usize = 6;

fn int(args: &[Slot], index: usize) -> i64 {
    match args[index] {
        Slot::Int(value) => value,
        Slot::Float(_) => 0,
    }
}

fn float(args: &[Slot], index: usize) -> f64 {
    match args[index] {
        Slot::Float(value) => value,
        Slot::Int(_) => 0.0,
    }
}

/// Call the C function at `function` with the arguments, each in its
/// class; `None` when the family has no signature of that shape (more
/// than `MAX_ARGS` arguments).
///
/// # Safety
///
/// `function` must point to a C function whose parameters match `args`
/// class for class and in number, and whose result matches `returns`; the
/// callee runs with no guarantee of the VM's.
pub unsafe fn call(function: *const c_void, args: &[Slot], returns: Returns) -> Option<Returned> {
    let mask: usize = args
        .iter()
        .enumerate()
        .map(|(index, slot)| match slot {
            Slot::Float(_) => 1 << index,
            Slot::Int(_) => 0,
        })
        .sum();
    match (args.len(), mask, returns) {
        (0, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn() = std::mem::transmute(function);
            f();
            Some(Returned::Nothing)
        }
        (0, 0, Returns::Int) => {
            let f: unsafe extern "C" fn() -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f()))
        }
        (0, 0, Returns::Float) => {
            let f: unsafe extern "C" fn() -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f()))
        }
        (1, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64) = std::mem::transmute(function);
            f(int(args, 0));
            Some(Returned::Nothing)
        }
        (1, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0))))
        }
        (1, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(int(args, 0))))
        }
        (1, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64) = std::mem::transmute(function);
            f(float(args, 0));
            Some(Returned::Nothing)
        }
        (1, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(float(args, 0))))
        }
        (1, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(float(args, 0))))
        }
        (2, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1));
            Some(Returned::Nothing)
        }
        (2, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0), int(args, 1))))
        }
        (2, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(int(args, 0), int(args, 1))))
        }
        (2, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1));
            Some(Returned::Nothing)
        }
        (2, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(float(args, 0), int(args, 1))))
        }
        (2, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(float(args, 0), int(args, 1))))
        }
        (2, 2, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1));
            Some(Returned::Nothing)
        }
        (2, 2, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0), float(args, 1))))
        }
        (2, 2, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(int(args, 0), float(args, 1))))
        }
        (2, 3, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1));
            Some(Returned::Nothing)
        }
        (2, 3, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(float(args, 0), float(args, 1))))
        }
        (2, 3, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(float(args, 0), float(args, 1))))
        }
        (3, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), int(args, 2));
            Some(Returned::Nothing)
        }
        (3, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0), int(args, 1), int(args, 2))))
        }
        (3, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(int(args, 0), int(args, 1), int(args, 2))))
        }
        (3, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), int(args, 2));
            Some(Returned::Nothing)
        }
        (3, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(float(args, 0), int(args, 1), int(args, 2))))
        }
        (3, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
            )))
        }
        (3, 2, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), int(args, 2));
            Some(Returned::Nothing)
        }
        (3, 2, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0), float(args, 1), int(args, 2))))
        }
        (3, 2, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
            )))
        }
        (3, 3, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1), int(args, 2));
            Some(Returned::Nothing)
        }
        (3, 3, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
            )))
        }
        (3, 3, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
            )))
        }
        (3, 4, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), float(args, 2));
            Some(Returned::Nothing)
        }
        (3, 4, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(int(args, 0), int(args, 1), float(args, 2))))
        }
        (3, 4, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
            )))
        }
        (3, 5, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), float(args, 2));
            Some(Returned::Nothing)
        }
        (3, 5, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
            )))
        }
        (3, 5, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
            )))
        }
        (3, 6, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), float(args, 2));
            Some(Returned::Nothing)
        }
        (3, 6, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
            )))
        }
        (3, 6, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
            )))
        }
        (3, 7, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1), float(args, 2));
            Some(Returned::Nothing)
        }
        (3, 7, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
            )))
        }
        (3, 7, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
            )))
        }
        (4, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), int(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), int(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 2, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), int(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 2, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 2, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 3, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1), int(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 3, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 3, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
            )))
        }
        (4, 4, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), float(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 4, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 4, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 5, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), float(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 5, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 5, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 6, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), float(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 6, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 6, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 7, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1), float(args, 2), int(args, 3));
            Some(Returned::Nothing)
        }
        (4, 7, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 7, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
            )))
        }
        (4, 8, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), int(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 8, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 8, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 9, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), int(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 9, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 9, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 10, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), int(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 10, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 10, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 11, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64) = std::mem::transmute(function);
            f(float(args, 0), float(args, 1), int(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 11, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 11, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
            )))
        }
        (4, 12, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64) = std::mem::transmute(function);
            f(int(args, 0), int(args, 1), float(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 12, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 12, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 13, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64) = std::mem::transmute(function);
            f(float(args, 0), int(args, 1), float(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 13, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 13, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 14, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64) = std::mem::transmute(function);
            f(int(args, 0), float(args, 1), float(args, 2), float(args, 3));
            Some(Returned::Nothing)
        }
        (4, 14, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 14, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 15, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
            );
            Some(Returned::Nothing)
        }
        (4, 15, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64) -> i64 = std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (4, 15, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64) -> f64 = std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
            )))
        }
        (5, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 2, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 2, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 2, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 3, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 3, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 3, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 4, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 4, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 4, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 5, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 5, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 5, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 6, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 6, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 6, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 7, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 7, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 7, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
            )))
        }
        (5, 8, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 8, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 8, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 9, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 9, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 9, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 10, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 10, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 10, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 11, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 11, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 11, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 12, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 12, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 12, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 13, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 13, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 13, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 14, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 14, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 14, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 15, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 15, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 15, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
            )))
        }
        (5, 16, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 16, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 16, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 17, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 17, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 17, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 18, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 18, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 18, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 19, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 19, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 19, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 20, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 20, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 20, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 21, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 21, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 21, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 22, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 22, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 22, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 23, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 23, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 23, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
            )))
        }
        (5, 24, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 24, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 24, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 25, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 25, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 25, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 26, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 26, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 26, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 27, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 27, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 27, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 28, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 28, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 28, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 29, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 29, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 29, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 30, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64) = std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 30, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 30, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 31, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64) = std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            );
            Some(Returned::Nothing)
        }
        (5, 31, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (5, 31, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
            )))
        }
        (6, 0, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 0, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 0, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 1, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 1, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 1, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 2, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 2, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 2, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 3, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 3, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 3, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 4, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 4, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 4, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 5, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 5, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 5, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 6, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 6, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 6, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 7, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 7, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 7, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 8, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 8, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 8, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 9, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 9, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 9, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 10, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 10, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 10, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 11, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 11, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 11, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 12, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 12, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 12, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 13, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 13, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 13, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 14, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 14, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 14, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 15, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 15, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 15, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                int(args, 5),
            )))
        }
        (6, 16, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 16, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 16, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 17, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 17, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 17, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 18, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 18, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 18, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 19, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 19, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 19, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 20, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 20, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 20, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 21, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 21, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 21, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 22, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 22, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 22, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 23, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 23, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 23, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 24, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 24, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 24, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 25, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 25, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 25, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 26, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 26, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 26, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 27, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 27, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 27, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 28, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 28, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 28, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 29, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 29, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 29, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 30, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 30, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 30, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 31, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, i64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 31, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, i64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 31, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, i64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                int(args, 5),
            )))
        }
        (6, 32, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 32, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 32, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 33, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 33, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 33, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 34, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 34, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 34, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 35, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 35, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 35, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 36, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 36, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 36, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 37, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 37, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 37, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 38, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 38, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 38, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 39, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 39, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 39, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 40, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 40, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 40, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 41, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 41, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 41, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 42, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 42, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 42, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 43, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 43, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 43, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 44, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 44, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 44, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 45, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 45, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 45, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 46, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 46, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 46, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 47, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 47, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 47, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, i64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                int(args, 4),
                float(args, 5),
            )))
        }
        (6, 48, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 48, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 48, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 49, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 49, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 49, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 50, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 50, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 50, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 51, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 51, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 51, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 52, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 52, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 52, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 53, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 53, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 53, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 54, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 54, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 54, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 55, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 55, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 55, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, i64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                int(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 56, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 56, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 56, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 57, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 57, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 57, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 58, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 58, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 58, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 59, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 59, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 59, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, i64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                int(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 60, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 60, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 60, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, i64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 61, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 61, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 61, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, i64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                int(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 62, Returns::Nothing) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 62, Returns::Int) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 62, Returns::Float) => {
            let f: unsafe extern "C" fn(i64, f64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                int(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 63, Returns::Nothing) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, f64) =
                std::mem::transmute(function);
            f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            );
            Some(Returned::Nothing)
        }
        (6, 63, Returns::Int) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, f64) -> i64 =
                std::mem::transmute(function);
            Some(Returned::Int(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        (6, 63, Returns::Float) => {
            let f: unsafe extern "C" fn(f64, f64, f64, f64, f64, f64) -> f64 =
                std::mem::transmute(function);
            Some(Returned::Float(f(
                float(args, 0),
                float(args, 1),
                float(args, 2),
                float(args, 3),
                float(args, 4),
                float(args, 5),
            )))
        }
        _ => None,
    }
}
