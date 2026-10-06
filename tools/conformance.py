#!/usr/bin/env python3
"""Run the conformance suite against a Renyi implementation (decision V8).

    python tools/conformance.py <renyi-binary> [--only TEXT]

The suite is `tests/conformance/manifest.json`, described in the README
beside it: every case names a program, the command to run it with (`run` or
`check`) and what the implementation must do. Nothing here depends on the
Rust crates: the runner only starts the binary it is given, from the
repository root, and compares. Exit status is 1 when any case fails.
"""
import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "tests" / "conformance" / "manifest.json"


def normalised(text: str) -> str:
    """Line endings as the expectations spell them."""
    return text.replace("\r\n", "\n")


def first_difference(actual: str, expected: str) -> str:
    """The first line where two outputs differ, for the report."""
    for number, (got, wanted) in enumerate(zip(actual.splitlines(), expected.splitlines()), 1):
        if got != wanted:
            return f"line {number}: got {got!r}, expected {wanted!r}"
    got, wanted = actual.count("\n"), expected.count("\n")
    return f"got {got} lines, expected {wanted}"


def problems_of(binary: str, case: dict) -> list[str]:
    """What the implementation got wrong on one case; empty when it passed."""
    command = [binary, case["command"], *case.get("options", []), case["program"],
               *case.get("arguments", [])]
    completed = subprocess.run(command, cwd=ROOT, capture_output=True, encoding="utf-8",
                               errors="replace")
    stdout, stderr = normalised(completed.stdout), normalised(completed.stderr)
    problems = []
    expected_code = case.get("exit_code", 0)
    if completed.returncode != expected_code:
        problems.append(f"exit code {completed.returncode}, expected {expected_code}")
    if "stdout" in case:
        expected = normalised((ROOT / case["stdout"]).read_text(encoding="utf-8"))
        if stdout != expected:
            problems.append("standard output differs: " + first_difference(stdout, expected))
    for code in case.get("diagnostics", []):
        if f"[{code}]" not in stdout + stderr:
            problems.append(f"no diagnostic [{code}] reported")
    for text in case.get("stderr_contains", []):
        if text not in stderr:
            problems.append(f"standard error lacks {text!r}")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) < 2 or argv[1].startswith("-"):
        print(__doc__.strip())
        return 2
    binary = str(pathlib.Path(argv[1]).resolve())
    only = argv[argv.index("--only") + 1] if "--only" in argv else None
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    cases = [case for case in manifest["cases"] if only is None or only in case["name"]]
    failed = 0
    for case in cases:
        problems = problems_of(binary, case)
        if problems:
            failed += 1
            print(f"FAIL  {case['name']} ({case['program']})")
            for problem in problems:
                print(f"      {problem}")
        else:
            print(f"ok    {case['name']}")
    print(f"{len(cases) - failed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
