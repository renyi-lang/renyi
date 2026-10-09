# Extending Renyi with Rust

An extension adds functions to the toolchain that run as Rust: a
declaration file that states each function's signature, its capability
and its failures, and a table of natives that implement them, built into
a `renyi` binary (decisions AJ1 and AK1 to AK4 in
`design/01-decisions.md`). A program imports the extension's module like
a library module, and the toolchain treats its functions as library
primitives: the checker checks the calls against the declarations, the
runtime admits only what the program's grant allows, and a run through
them is recorded, replayed and narrated like any other. The standard
library is the first extension, so everything below is what
`crates/renyi_vm/src/natives/` does.

## 1. The declaration file

One file per module, as the standard library declares its own under
`library/std/`; `renyi parse --declarations demo.ry` reads it. Every
function states what it needs and how it fails:

```
module demo
  purpose: Three natives: a pure function, a console one, a file one.

import std.filesystem exposing Path

public type PeekError is one of
  purpose: What peek reports.
  PermissionDenied(path: Path)
  Missing(path: Path)
end

public function twice(amount: Integer) returns Integer
  purpose: Twice the amount.
public function shout(text: Text) needs console
  purpose: Print the text in capitals with a bang.
public function peek(path: Path) returns Text or fails with PeekError needs filesystem.read
  purpose: The first line of the file.
```

The capabilities are those of reference section 11 and no others
(decision AK4): `console`, `filesystem`, `network`, `environment`,
`time`, `random`, `process`, `foreign` and `python` (the bridge's,
decision AJ3; the guide is `python.md`). A function declared under a kind
that takes a scope (`filesystem`, `network`, `environment`, `process`)
names its scope through a parameter: a `Path` for `filesystem`, a `Url`
for `network`, the variable's name for `environment.get`, the program's
name for `process`. The boundary reads the scope off that argument and
checks it against the grant before the native runs, so `peek` above is
refused under `needs filesystem.read("data")` when it is given
`Path("elsewhere/secret.txt")`, and the refusal is reported through the
declared failure type: the variant named `PermissionDenied` with one
field gets the scope (`HostNotAllowed` and `ProgramNotAllowed` are read
the same way, and `OverBudget` for a call past a budget). A function
without such a variant crashes when refused, as `environment.get` does.

## 2. The natives

A native has one signature, the standard library's:

```rust
use std::io::Write;

use renyi_vm::natives::{arg, crash, small, text};
use renyi_vm::{Interrupt, Value, Vm};

fn twice(_: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    Ok(Value::integer(small(arg(args, 0))? * 2))
}

fn shout(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let line = text(arg(args, 0))?.to_uppercase();
    writeln!(vm.stdout, "{line}!").map_err(|error| crash(format!("cannot write: {error}")))?;
    Ok(Value::Nothing)
}

fn peek(vm: &mut Vm, args: &mut [Value]) -> Result<Value, Interrupt> {
    let path = text(arg(args, 0))?.to_string();
    match std::fs::read_to_string(&path) {
        Ok(content) => Ok(Value::text(content.lines().next().unwrap_or(""))),
        Err(_) => vm.fail_variant("demo", "PeekError", "Missing", vec![Value::text(path)]),
    }
}
```

`args` holds the arguments in declaration order as plain values (the
checker has verified their number and types); `renyi_vm::natives` reads
them: `arg`, `take`, `text`, `int`, `small`, `decimal`, `float`,
`boolean`, `list`, `map`, `set`, `range`, `bytes`, `duration`,
`instant`. The result is a `Value`: `Value::integer`, `Value::text`,
`Value::list`, `Value::Nothing` for a function without `returns`; a
record or a variant of a declared type is built by name,
`vm.library_record("demo", "Summary", fields)` and
`vm.library_variant(...)`, and a failure by `vm.fail_record` and
`vm.fail_variant`. A bug is `Err(crash("..."))`, which ends the run as a
crash. The console is `vm.stdout` and `vm.stderr`, not the process's,
so that a run under `renyi test` or the MCP server captures it.

## 3. The extension

```rust
use renyi_vm::{Extension, Native};

pub const DEMO: Extension = Extension {
    name: "demo",
    version: env!("CARGO_PKG_VERSION"),
    modules: &[("demo", include_str!("demo.ry"))],
    natives: &[
        Native::function("demo", "twice", twice),
        Native::function("demo", "shout", shout),
        Native::function("demo", "peek", peek),
    ],
};
```

A `Native` names the module, the function and, for a method or an
overloaded name, the type of the first parameter as the checker spells
it: `Native::method("demo", "length", "Text", text_length)` by the head
of the type, or `Native::method("demo", "sum", "List of Integer", ...)`
in full. A lookup takes the entry whose receiver is the full spelling,
then the head, then the entry registered with `Native::function`
(decision AK2).

A native may carry a typed entry (decision AU1), a second function from
a fixed family of signatures that the generated code calls without the
boundary, its arguments borrowed where they lie and its answer in a
register:

```rust
use renyi_vm::natives::plain_small;
use renyi_vm::Typed;

fn twice_typed(args: &[Value]) -> Option<i64> {
    plain_small(&args[0])?.checked_mul(2)
}

Native::function("demo", "twice", twice).with_typed(Typed::Int(twice_typed))
```

The entry sees the arguments plain and borrowed (`&[Value]`) and answers
`Option<bool>`, `Option<i64>`, `Option<f64>` or `Option<Value>`
(`Typed::Bool`, `Int`, `Float`, `Value`); its kind must be the declared
result's (`Boolean`, `Integer`, `Float`, anything else), and the function
must need no capability, since the entry skips the boundary. It must
answer exactly what the native answers whenever it answers, and it
declines with `None` wherever it cannot: on a guarded argument (the
native's answer carries the guard's origins), on an Integer past the
machine word, and wherever the native would fail or crash; the native
then runs as if the entry were not there. The helpers `plain_text`,
`plain_small`, `plain_float`, `plain_list`, `plain_map` and `plain_set`
of `renyi_vm::natives` read a plain argument of their kind and answer
`None` for anything else, a guarded value included. A `Value` entry
never answers a `Failure`. The standard library's hottest primitives
carry such entries (`length`, `contains`, `get`, `at`, ...).

The table and the declaration file are held equal both ways: every
declared function must have a native and every native a declaration,
every file must parse and check, and a module is declared by one
extension. The check is a test of the extension's crate:

```rust
#[test]
fn registered() {
    assert_eq!(renyi_vm::Registry::standard().with(DEMO).verify(), Ok(()));
}
```

It names what is wrong, one problem per line: ``extension `demo`:
`demo.peek` on `Path` is declared but has no native``, ``extension
`demo`: the native `demo.whisper` on `Text` implements no declared
function``, ``extension `demo`: `demo`: unknown capability `gpu` ...``.

## 4. The binary

A binary with the extension is a crate of a few lines on top of the
`renyi` crate (decision AK1):

```toml
[package]
name = "renyi-with-demo"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "renyi"
path = "src/main.rs"

[dependencies]
renyi = "0.1"
demo = { path = "../demo" }
```

```rust
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;

fn main() -> std::process::ExitCode {
    renyi::main_with(vec![demo::DEMO])
}
```

The allocator line is decision AP1's: mimalloc under the count that the
memory budget of a sandbox needs (`embedding.md`, section 4); a binary
without it runs on the system allocator and refuses a grant with
`memory`. `cargo install --path .` puts that `renyi` in place of the
official one.
It runs the same check as the test at start and refuses to start when
it fails; `renyi version` prints the extensions after the toolchain's
line (`extension demo 0.1.0`). Every command then knows the extension's
modules: `renyi check`, `renyi index`, `renyi tools`, `renyi mcp` and
the rest check programs against its declarations, and `renyi run`,
`record`, `test` and `reproduce` run its natives.

## 5. What the boundary does for a native

Nothing in the extension speaks to the grant, the recording or the
narration; the boundary of the VM does, as it does for the standard
library:

- the call is checked against the effective grant of the call chain, the
  budgets (`at most`) and the guards (`only to`), with the scope read off
  the `Path` or `Url` argument;
- under `renyi record` the call is written to the recording with its
  arguments and its result, under `--replay` and `reproduce` it is
  answered from the recording without running the native, and under
  `--explain` it is narrated with the function's `purpose:`;
- the run manifest names the extensions beyond the standard library
  (`"extensions": ["demo 0.1.0"]`), and `reproduce` warns when the
  recording's differ from the binary's.

A program compiled to a bytecode file with an extension's functions runs
on a binary with that extension; on another, the call crashes as a
library function this build does not implement.

## 6. What an extension is trusted with

An extension is Rust in the process, trusted as the standard library is:
whoever builds the binary chooses what goes in. The guarantees of the
reference hold for a program to the extent that the natives do what
their declarations say: a native declared pure that writes a file is a
bug of the extension, not something the toolchain can catch. The
declaration file is the contract a reviewer reads; keep it exact.
