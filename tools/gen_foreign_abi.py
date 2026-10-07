"""Write crates/renyi_vm/src/natives/foreign_abi.rs: the fixed family of C
signatures the VM can call (decision AF1). Every signature takes up to
MAX_ARGS arguments, each an integer-class word (an integer, a Boolean, a
pointer) or a double, and returns nothing, an integer-class word or a
double. Run `cargo fmt` after this script; the file is committed formatted.
"""
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
TARGET = ROOT / "crates/renyi_vm/src/natives/foreign_abi.rs"
MAX_ARGS = 6

HEAD = '''//! The fixed family of C signatures the VM can call (decision AF1), written
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
'''

TAIL = '''        _ => None,
    }
}
'''


def arms():
    out = []
    for count in range(MAX_ARGS + 1):
        for mask in range(1 << count):
            kinds = ["f64" if mask >> index & 1 else "i64" for index in range(count)]
            params = ", ".join(kinds)
            arguments = ", ".join(
                f"float(args, {index})" if kind == "f64" else f"int(args, {index})"
                for index, kind in enumerate(kinds)
            )
            for returns, rust in (("Nothing", ""), ("Int", " -> i64"), ("Float", " -> f64")):
                out.append(f"        ({count}, {mask}, Returns::{returns}) => {{\n")
                out.append(
                    f"            let f: unsafe extern \"C\" fn({params}){rust} = "
                    "std::mem::transmute(function);\n"
                )
                if returns == "Nothing":
                    out.append(f"            f({arguments});\n")
                    out.append("            Some(Returned::Nothing)\n")
                else:
                    out.append(f"            Some(Returned::{returns}(f({arguments})))\n")
                out.append("        }\n")
    return "".join(out)


def main():
    text = HEAD + arms() + TAIL
    TARGET.write_text(text, encoding="utf-8", newline="\n")
    print(f"wrote {TARGET.relative_to(ROOT)}: {text.count(chr(10))} lines")


if __name__ == "__main__":
    main()
