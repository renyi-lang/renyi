# Conformance suite

Decision V8 (and H6 before it): Renyi programs with what an implementation
must do with them, in files that a compiler written in any language, one
day in Renyi itself, can be tested against without the Rust crates. The
suite is the contract; `crates/` is one implementation of it.

## Files

| Path | Content |
|------|---------|
| `manifest.json` | the cases, one object each (fields below) |
| `expected/<name>.out` | the exact standard output a `run` case must print; the ten Predict references of the readability test are the first members |
| `programs/<name>.ry` | programs written for one diagnostic or one runtime guarantee; the corpus programs under `examples/` are referred to in place |
| `packages/` | the package fixture of decision AC1: a directory registry with one package (`registry/greeting/1.0.0/`, its `package.json` as `renyi publish` renders it) and the projects that use it, each with its `renyi.json` (`project/` with the lockfile that matches, `stale/` with one that does not, `unlocked/` without one, `broken/` with a manifest the format refuses) |
| `foreign/`, `foreign_bad/` | the foreign fixtures of decision AF1: a project whose `renyi.json` binds the foreign module `libc` to the C library (`length.ry` calls `strlen` and `abs`), and one whose declarations break every rule of the boundary (`bad.ry`) |
| `python/`, `python_bad/` | the Python fixtures of decision AL1: a project whose `renyi.json` binds the module `analysis` to `analysis.py` beside it (`stats.ry` calls it through the bridge, an exception and a result of the wrong type included; the runner needs a Python 3 on the PATH), and one whose declarations break every rule of the bridge (`bad.ry`) |
| `../../tools/conformance.py` | the runner: `python tools/conformance.py <renyi-binary>` from the repository root |
| `../../crates/renyi/tests/conformance.rs` | the same manifest run by `cargo test` against the Rust binary |

## A case

```json
{
  "name": "a deprecated call is an error under --strict",
  "command": "check",
  "options": ["--strict"],
  "program": "tests/conformance/programs/deprecated_call.ry",
  "diagnostics": ["deprecated"],
  "exit_code": 1
}
```

- `command`: `run` or `check`. The runner starts `<binary> <command>
  [options] <program> [arguments]` from the repository root, so a program
  that reads a file names it relative to the root.
- `program`: the file, relative to the repository root.
- `options`: what comes before the program on the command line (`--strict`).
- `arguments`: the program's own arguments, after it (`run` only).
- `stdout`: a file whose content the standard output must equal exactly,
  after `\r\n` is read as `\n` on both sides.
- `diagnostics`: codes that must each appear as `[code]` somewhere in the
  standard output or the standard error.
- `stderr_contains`: texts that must each appear in the standard error.
- `exit_code`: what the process must exit with; `0` by default.

## What the implementation promises

- `check` exits with `0` when the program has no errors, warnings included,
  and with `1` when it has any; every diagnostic names its code in square
  brackets, and the line after it starts with `fix:` and suggests a fix
  (decision D3).
- `run` exits with `0` when `main` returns, `1` when it fails, `2` when the
  program crashes, and with the code `environment.exit` was given. A
  failure or a crash is reported on the standard error; the standard
  output holds only what the program printed.

## Adding a case

Put the program under `programs/` (or refer to one under `examples/`),
write its expected output under `expected/` when it has one, add the
object to `manifest.json`, and run both runners. A new diagnostic of the
checker gets a case here in the same commit.
