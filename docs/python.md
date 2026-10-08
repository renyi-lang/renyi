# Calling Python from Renyi

A Renyi program calls a Python module through the bridge (decisions AJ2,
AJ3 and AL1 to AL4 in `design/01-decisions.md`): a declaration file of
the project states each function's signature, the manifest binds the
file to the Python module, and the VM runs the calls in one Python
process per run. A call is one primitive at the boundary, like a call
into the standard library: the checker checks it against the
declaration, the grant admits it only under `python("<package>")`, and
a run through it is recorded, replayed and narrated like any other.
What the Python side does is outside every guarantee of the VM, and the
program says so in its grant.

## 1. The manifest

`renyi.json` names the Python modules of the project under `python`:

```json
{
  "name": "bridge",
  "version": "0.1.0",
  "python": {
    "interpreter": "python3",
    "modules": {
      "analysis": {
        "package": "analysis",
        "symbols": {
          "mean": "average"
        }
      }
    }
  }
}
```

Each key of `modules` is a Renyi module name: the declaration file
`analysis.ry` in the project root (or `tools/stats.ry` for
`tools.stats`). Its `package` is the name the Python side imports the
module by, `analysis` for `analysis.py` beside the manifest or an
installed package such as `scipy.stats`; it is the module's own name
when left out. `symbols` renames functions whose Python name differs
from their Renyi name; left out, every function keeps its name.
`interpreter` names the Python to run, the one of a virtual
environment, say; left out, the VM uses the environment variable
`RENYI_PYTHON` when it is set, else `python3` and then `python` on the
PATH (`python` first on Windows, where `python3` is usually the Store's
stub). The first interpreter that starts and answers the bridge's
greeting is kept for the run.

## 2. The declaration file

A declaration file has no bodies, as the standard library's files under
`library/std/` have none; `renyi parse --declarations analysis.ry`
reads it. Every function needs `python("<package>")` with the package
of its binding and nothing else, fails with `PythonError` of
`std.python` and nothing else, and takes no type parameters
(`python-signature`):

```
module analysis
  purpose: The functions of `analysis.py`, declared for the bridge.

import std.python exposing PythonError

public type Sample
  purpose: What `describe` takes.
  has name: Text
  has labels: List of Text
  can ToJson
end

public type Description
  purpose: What `describe` gives back.
  has name: Text
  has total: Integer
  has labels: List of Text
  can FromJson
end

public function mean(values: List of Float)
  returns Float
  or fails with PythonError
  needs python("analysis")
  purpose: The arithmetic mean of the values.

public function describe(sample: Sample)
  returns Description
  or fails with PythonError
  needs python("analysis")
  purpose: The sample described.
```

A call crosses as JSON both ways, so a parameter is a type that can
`ToJson` and a result a type that can `FromJson`, or nothing
(`python-type`): `Integer`, `Decimal`, `Float`, `Boolean`, `Text` and
`Bytes` (as base64), a subtype of one, `maybe` one, a `List` or a `Set`
of such, a `Map of Text to` such, `JsonValue` of `std.json`, and a
record or a sum type that derives the ability. The arguments reach the
Python function by position, in the order the declaration lists them,
as the values `json.dumps` would give: a record is a `dict`, a list a
`list`, `nothing` is `None`. The result comes back the same way and is
read by the declared type: a `dict` for a record, a number for
`Integer` or `Float`, `None` for `maybe`. A result the declaration does
not mention is dropped.

The Python side of the example is `analysis.py` beside the manifest:

```python
def average(values):
    return sum(values) / len(values)


def describe(sample):
    return {
        "name": sample["name"].upper(),
        "total": len(sample["labels"]),
        "labels": sorted(sample["labels"]),
    }
```

## 3. The program

A program imports the module like any other and grants the capability in
`main`:

```
module stats
  purpose: Call a Python module of the project through the bridge.

import std.console
import std.python exposing PythonError
import analysis exposing Sample

function main() or fails with PythonError needs console, python("analysis")
  purpose: Print what Python computes.

  let mean be analysis.mean([1.0, 2.0, 4.5]) otherwise fail
  console.print("{mean}")
  let described be analysis.describe(Sample(name: "renyi", labels: ["b", "a"])) otherwise fail
  console.print("{described.name} {described.total}")
end
```

`renyi run stats.ry` says "this program can run Python through
`analysis`" on its standard error, as it says of native code, and
prints `2.5` and `RENYI 2`. The worker starts at the first call, with
the project root on its module path so that the project's own `.py`
files import, and ends with the VM. What the Python functions print
goes to the standard error; it cannot mix with the protocol, which is
one JSON object per line on the worker's standard input and output.

Every call is recorded with its arguments and its result, so `renyi
record` and `renyi run --replay` work as they do for the library: a
replay answers the calls from the recording and never starts Python,
`renyi reproduce` compares the outcome and the output, `--explain`
narrates each call with its duration. The capability takes a scope
and no budget: `python` in `main` covers every package,
`python("analysis")` this one; `--deny python` refuses the program.

## 4. What fails

A call fails with `PythonError` (`library/std/python.ry`), never with
an exception of its own:

- `Raised(exception, message)`: the function let an exception escape;
  the class name and its text, to match on
  (`when failure(Raised(exception, message))`).
- `NotCarried(detail)`: the result does not fit the declared type (a
  text where an `Integer` was declared) or JSON does not carry it (a
  `set`, `NaN`).
- `Unavailable(detail)`: no interpreter answers, the module does not
  import, the function is not there, or the worker ended (a function
  called `sys.exit`); the next call starts a worker again.
- `PermissionDenied(package)`: the package is outside the effective
  grant.

A project with Python modules cannot be published as a package (`renyi
publish` refuses it): a package is written in Renyi, and Python stays in
the program that grants it. The conformance fixtures
`tests/conformance/python/` and `python_bad/` are the example above and
a file that breaks every rule; `crates/renyi/tests/python.rs` drives the
whole of it through the binary.
