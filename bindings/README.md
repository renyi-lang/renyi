# The Renyi C API

This archive is the embedding API of Renyi for C hosts and the languages
that call C (decision AW46; the guide is `docs/embedding.md` section 7
of the repository, https://github.com/renyi-lang/renyi). A host loads a
Renyi module with a grant and calls its public functions; the module
reaches nothing the grant does not name, because there is no ambient
I/O, no reflection and no native call without `foreign`. Values cross
as JSON texts in the codec of `std.json`.

| File | What it is |
|------|------------|
| `librenyi_c.so`, `librenyi_c.dylib` or `renyi_c.dll` (with `renyi_c.dll.lib`) | the shared library |
| `librenyi_c.a` or `renyi_c.lib` | the static library |
| `renyi.h` | the header: every function, the statuses, the ownership rules |
| `renyi.py` | the Python binding, one file over `ctypes`, no dependency |
| `host.c`, `calc.ry` | a C host of forty lines and the module it loads |
| `example.py` | the same host in Python |

## From C

```sh
cc host.c -I. -L. -lrenyi_c -Wl,-rpath,"$PWD" -o host   # Linux, macOS
./host calc.ry
```

On Windows link `renyi_c.dll.lib` and keep `renyi_c.dll` on the `PATH`
or beside the executable. The library declares its own counting
allocator, so a grant with `memory` works with no declaration of the
host's.

A module is loaded with `renyi_load(path, len, grant, len)` or
`renyi_load_source(name, len, text, len, grant, len)`, the grant the
JSON object `{"grant": "console", "memory": "16 megabytes"}` as a text
or the empty text for a grant of nothing; a load that fails answers
null with the reason in `renyi_last_error`. A call,
`renyi_call(sandbox, name, len, arguments, len)`, takes the arguments
as one JSON array text and answers a status: `RENYI_OK` with the value
as JSON in `renyi_text`, `RENYI_FAILED` with the failure rendered,
`RENYI_CRASHED`, `RENYI_EXITED` (the code in `renyi_exit_code`),
`RENYI_REFUSED` (nothing ran; the reason in the text) or
`RENYI_OVER_MEMORY` (the budget in `renyi_memory_limit`, the peak in
`renyi_memory_used`). Every text crosses as a pointer and a length,
UTF-8, not NUL-terminated; a text the library answers belongs to the
handle and is valid until the next call on that handle.

## From Python

```python
import renyi   # renyi.py beside your script, or on sys.path

with renyi.Sandbox.load("calc.ry", {"grant": "console"}) as calc:
    print(calc.call("add", 2, 3))                       # 5
    print(calc.call("shift", {"across": 1, "down": 2}, 10))
```

`renyi.py` finds the library beside itself, or through `RENYI_C_LIB`.
A call answers the value `json.loads` reads, or raises `Failed`,
`Crashed`, `Exited`, `Refused` or `OverMemory`; `functions()` lists the
public functions with their signatures. `python example.py calc.ry`
runs the example.
