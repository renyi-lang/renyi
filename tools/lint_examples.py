#!/usr/bin/env python3
"""Pre-parser lint for the example corpus (M0).

The compiler does not exist yet, so this script enforces the parts of the
syntax sketch that a regular expression can see: forbidden symbols and foreign
keywords, reserved words used as identifiers, single-letter names, `then` on
conditions, line width, tabs and trailing whitespace, and a heuristic
block/`end` balance. It is a safety net for hand-written examples, not a
grammar check; M1 replaces it with the real parser.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
RESERVED = set("""ability all also and any as at be break by can check collect
concurrently continue count crash deprecated descending each end example expose
exposing fail fails failure false first for from function greater group has if
import in is lazy least less let match maybe module most mutable needs not
nothing of one or otherwise power public purpose remainder return returns run
see self set some sorted success sum tags test than then to tool true type when
where while with""".split())
assert len(RESERVED) == 81, len(RESERVED)

FORBIDDEN = [
    (r"(?<![=!<>])=(?![=>])", "'=' is not Renyi; use 'let x be', 'set x to', or 'is'"),
    (r"==|!=|<=|>=|&&|\|\||->|=>|\|>|::", "symbolic operator; use the English phrase"),
    (r"(?<![\w.\"'])[<>](?![\w.\"'])", "'<' or '>' comparison; use 'is less than' / 'is greater than'"),
    (r";\s*$", "semicolon"),
    (r"\b(else|elif|fn|def|null|None|lambda|var|const|elsif|unless|switch|case)\b",
     "keyword from another language"),
    (r"\t", "tab character"),
    (r"\s+$", "trailing whitespace"),
]
BINDING_PATTERNS = [
    re.compile(r"\blet (?:mutable )?([a-z_][a-z0-9_]*)"),
    re.compile(r"\bfor each ([a-z_][a-z0-9_]*)(?:, ([a-z_][a-z0-9_]*))?"),
    re.compile(r"\bhas ([a-z_][a-z0-9_]*):"),
    re.compile(r"[(,] *([a-z_][a-z0-9_]*):(?!:)"),
]
STARTERS = re.compile(
    r"^\s*(?:public )?(?:function |ability |test |if |match |while |run concurrently$|type \w+( is one of)?$|type \w+ of )")
QUERY_TAIL = re.compile(r"\b(collect|sum|count|first|any|all|group by)\b")


def strip_strings(line: str) -> str:
    """Blank out string literals so their contents are not linted."""
    return re.sub(r'"(?:[^"\\]|\\.)*"', lambda m: '"' + " " * (len(m.group(0)) - 2) + '"', line)


def lint_file(path: pathlib.Path) -> list[str]:
    problems = []
    opened = 0
    closed = 0
    previous = ""
    in_ability_declaration = False
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = strip_strings(raw)
        code = line.split("#", 1)[0] if " #" in line or line.lstrip().startswith("#") else line
        where = f"{path.name}:{number}"
        if len(raw) > 100:
            problems.append(f"{where}: line is {len(raw)} columns (limit 100)")
        for pattern, message in FORBIDDEN:
            if re.search(pattern, code):
                problems.append(f"{where}: {message}")
        for pattern in BINDING_PATTERNS:
            for match in pattern.finditer(code):
                for name in match.groups():
                    if not name:
                        continue
                    if name in RESERVED and name != "self":
                        problems.append(f"{where}: reserved word '{name}' used as an identifier")
                    if len(name) == 1:
                        problems.append(f"{where}: single-letter identifier '{name}'")
        stripped = code.strip()
        if re.match(r"^(if |otherwise if |when )", stripped) and " then" not in stripped:
            problems.append(f"{where}: condition without 'then'")
        expression_context = previous.endswith(" be") or previous == "return"
        if re.match(r"^(public )?ability \w+$", stripped):
            in_ability_declaration = True
        if stripped.startswith("for each"):
            if not expression_context and not QUERY_TAIL.search(stripped):
                opened += 1
        elif STARTERS.match(code):
            if not (in_ability_declaration and stripped.startswith("function ")):
                opened += 1
        if stripped == "end":
            closed += 1
            if in_ability_declaration and opened == closed:
                in_ability_declaration = False
        if stripped:
            previous = stripped
    if opened != closed:
        problems.append(f"{path.name}: block starters {opened} vs 'end' {closed} (heuristic)")
    return problems


def main() -> int:
    files = sorted((ROOT / "examples").glob("*.ry")) + sorted((ROOT / "examples").glob("*.renyi"))
    all_problems = []
    for path in files:
        all_problems.extend(lint_file(path))
    for problem in all_problems:
        print(problem)
    print(f"{len(files)} files, {len(all_problems)} problems")
    return 1 if all_problems else 0


if __name__ == "__main__":
    sys.exit(main())
