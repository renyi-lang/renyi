# Embedding Renyi in a host

A host program loads a Renyi module with a grant and calls its public
functions. The module reaches nothing the grant does not name: there is
no ambient I/O, no reflection and no native call without `foreign`, so
the effect system is the sandbox, with no process, container or Wasm
runtime around it (decisions Q3, AP1 and AP2; `design/07-system-design.md`
section 4). The API is Rust, in the `renyi` crate; the C API of decision
A5, for Python, JS and C hosts, is a layer over it and follows later.

## 1. The grant

A grant is spelled as a `needs` clause is, with the same scopes, budgets
(`at most`, decision P2) and guards (`only to`, decision P3):

```
console, network.http("api.example.com") at most 60 per minute, filesystem.read("data")
```

In a file it is a JSON object with the memory budget beside it; both
fields are optional, and there are no others:

```json
{
  "grant": "console, network.http(\"api.example.com\") at most 60 per minute",
  "memory": "256 megabytes"
}
```

`Grant::parse` reads the clause, `Grant::from_json` and `Grant::from_file`
the object; the memory is `bytes`, `kilobytes`, `megabytes` or
`gigabytes`. An empty clause grants nothing: a module loaded with it can
compute and nothing else.

## 2. Loading a module

```rust
use renyi::{Grant, Sandbox, Value};

let grant = Grant::from_file("grant.json")?;
let mut module = Sandbox::load("tools/summarize.ry", grant)?;
for function in module.functions() {
    println!("{}: {}", function.name, function.signature);
}
```

`Sandbox::load` reads the file, resolves its imports from its directory
and its project (`renyi.json`, decision AC1) and checks it as `renyi
check` does; a module with errors is refused with the diagnostics, and
the warnings are kept (`warnings()`). `Sandbox::load_source(name, text,
grant)` takes the text from memory, with `name` the file name its imports
resolve from. A bytecode file (`.ryc`) is refused: it carries no
visibility.

`functions()` lists the public functions of the module in source order:
the name, the signature as the project map spells it, the purpose, the
number of parameters, the `needs` clause, and for a function with
`expose as tool` the JSON Schema of its parameters as a text, as `renyi
tools` writes it (decision D6).

## 3. Calling

```rust
match module.call("summarize", vec![Value::text(report), Value::integer(3)]) {
    Ok(value) => println!("{value:?}"),
    Err(error) => eprintln!("summarize {error}"),
}
```

A call names a public function and gives its arguments in order, as
`Value`s (`Value::text`, `Value::integer`, `Value::Boolean`, lists, maps
and the rest of `renyi_vm::Value`). The value comes back, or a
`CallError`:

- `Refused(message)`: nothing ran. The function is not public or does not
  exist, the arguments are not as many as its parameters, or the function
  needs a capability the grant does not cover (`` `fetch` needs
  `network.http`, which the grant does not cover ``). The check is on the
  function's own `needs`; inside the call, the grant stack narrows as
  under `renyi run`, and a primitive outside what the function was given
  fails at the boundary.
- `Failed(error)`: the function failed, with the failure rendered as the
  console would show it (`NotFound(path: "a.txt")`, `OverBudget(path:
  "a.txt")`, `Guarded(origin: ..., sink: ...)`).
- `Crashed { message, location }`: `crash with`, an unhandled failure, an
  arithmetic domain error.
- `Exited(code)`: `environment.exit`.
- `OverMemory { limit, used }`: the memory budget, section 4.

The grant's budgets count across the calls of one sandbox as one run:
`filesystem.read("data") at most 2 per run` admits two reads in the
sandbox's life, whichever calls make them. Each call runs on a VM of its
own, and nothing survives from one call to the next but the budgets: a
Renyi module has no mutable state at the top level, so there is nothing
else to keep.

A value the host hands in may carry a guard of the grant (decision P3):

```rust
let secret = module.guarded(Value::text(key), "filesystem.read(\"secrets\")")?;
module.call("sign", vec![secret, Value::text(payload)])?;
```

`guarded` names a capability of the grant that has `only to` sinks; the
value, and whatever the module computes from it, may leave the module
only through those sinks, and any other outgoing primitive fails with
`Guarded`.

## 4. The memory budget

`memory` in the grant bounds the bytes a call holds above the level at
its start. The VM reads the count at every call, every primitive and
every loop turning, and a call over the budget ends as a whole with
`OverMemory { limit, used }`; the next call starts afresh. A budgeted
call runs on the interpreter, as a narrated or profiled one does
(decision AG3), so that every safe point is the interpreter's.

The count comes from the process's allocator, which must be the counting
one. The official `renyi` binary declares it; a host declares it in one
line:

```rust
#[global_allocator]
static ALLOCATOR: renyi::Allocator = renyi::Allocator;
```

`renyi::Allocator` is mimalloc under the count. A host that keeps an
allocator of its own wraps it instead:

```rust
#[global_allocator]
static ALLOCATOR: renyi::Counting<std::alloc::System> = renyi::Counting(std::alloc::System);
```

Without either, a grant with `memory` is refused at load with the line
to add; a grant without `memory` needs nothing. The count costs one load
per allocation while no budget is in force. One budget is in force at a
time in a process, and it counts the process's allocations, so a host
that allocates on other threads during a budgeted call sees those bytes
counted too.

## 5. The same grant from the command line

```sh
renyi run --sandbox grant.json program.ry
renyi record --sandbox grant.json program.ry
renyi serve --watch --sandbox grant.json service.ry
```

`main` runs under its declared grant intersected with the file's, the
file's budgets and guards added, and its memory budget in force; `main`
must need nothing the grant does not cover, or the command refuses to
start naming the capability (`the sandbox does not grant
`network.http`, which `main` needs`), as `--deny` does (decision E3).
The run manifest and a recording name the effective grant. A run over
the memory budget exits with status 2, as a crash does.

## 6. What it promises

The sandbox isolates effects (the grant), calls (the budgets), where
data may go (the guards), time (`within` deadlines inside the module)
and memory (the budget). It does not hide timing or scheduling from the
module; `foreign` code and Python, once granted, are outside every
guarantee, as `extensions.md` and `python.md` say; and the module's
console output goes to the process's standard streams in this version.
A module's calls run one after the other (decision S2), each on its own
VM.
