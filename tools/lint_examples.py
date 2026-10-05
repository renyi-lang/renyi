#!/usr/bin/env python3
"""Pre-parser lint for the example corpus (M0).

The compiler does not exist yet, so this script enforces the parts of the
syntax sketch that a regular expression can see: forbidden symbols and foreign
keywords, reserved words used as identifiers, single-letter names, `then` on
conditions, line width, tabs and trailing whitespace, and a heuristic
block/`end` balance, unused bindings, and every called method or module
function against the declarations in `docs/design/04-stdlib-sketch.md` and the
corpus itself. It is a safety net for hand-written examples, not a
grammar check; M1 replaces it with the real parser.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
RESERVED = set("""ability all also and any as at be break by can check collect
concurrently continue count crash deprecated descending each end example expose
exposing fail fails failure false first for from function greater group has if
ignore import in is lazy least less let match maybe module most mutable needs
not nothing of one or otherwise power public purpose raw remainder repeat return
returns run see self set some sorted success sum tags test than then to tool
true type until when where with within""".split())
assert len(RESERVED) == 85, len(RESERVED)

FORBIDDEN = [
    (r"(?<![=!<>])=(?![=>])", "'=' is not Renyi; use 'let x be', 'set x to', or 'is'"),
    (r"==|!=|<=|>=|&&|\|\||->|=>|\|>|::", "symbolic operator; use the English phrase"),
    (r"(?<![\w.\"'])[<>](?![\w.\"'])", "'<' or '>' comparison; use 'is less than' / 'is greater than'"),
    (r";\s*$", "semicolon"),
    (r"\b(else|elif|fn|def|null|None|lambda|var|const|elsif|unless|switch|case|while)\b",
     "keyword from another language"),
]
BINDING_PATTERNS = [
    re.compile(r"\blet (?:mutable )?([a-z_][a-z0-9_]*)"),
    re.compile(r"\bfor each ([a-z_][a-z0-9_]*)(?:, ([a-z_][a-z0-9_]*))?"),
    re.compile(r"\bhas ([a-z_][a-z0-9_]*):"),
    re.compile(r"[(,] *([a-z_][a-z0-9_]*):(?!:)"),
]
STARTERS = re.compile(
    r"^\s*(?:public )?(?:function |ability |test |if |match |repeat until |run concurrently( within .*)?$|type \w+( is one of)?$|type \w+ of )")
QUERY_TAIL = re.compile(r"\b(collect|sum|count|first|any|all|group by)\b")
STDLIB_SKETCH = ROOT / "docs" / "design" / "04-stdlib-sketch.md"
CALL = re.compile(r"\.([a-z][a-z0-9_]*)\(")
DECLARATION = re.compile(r"\bfunction ([a-z][a-z0-9_]*)\(")


def declared_functions(paths) -> set[str]:
    """Function names declared in the standard library sketch and in the corpus."""
    names = set()
    for path in paths:
        names.update(DECLARATION.findall(path.read_text(encoding="utf-8")))
    return names


PATTERN_WORDS = {"nothing", "some", "success", "failure", "true", "false", "and", "or",
                 "not", "is"}


def bound_names(code: str) -> list[str]:
    """Names a line binds: let, loop variables, and pattern variables after `when`."""
    names = []
    for match in re.finditer(r"\blet (?:mutable )?([a-z][a-z0-9_]*)", code):
        names.append(match.group(1))
    for match in re.finditer(r"\bfor each ([a-z][a-z0-9_]*)(?:, ([a-z][a-z0-9_]*))?", code):
        names.extend(name for name in match.groups() if name)
    when = re.match(r"^\s*when (.*?)(?: where .*)?(?: then\b.*)?$", code)
    if when:
        pattern = when.group(1)
        for match in re.finditer(r"\b([a-z][a-z0-9_]*)(?:: *([A-Za-z][A-Za-z0-9_]*))?", pattern):
            label, after = match.group(1), match.group(2)
            if label in PATTERN_WORDS:
                continue
            if after and after[0].islower():
                names.append(after)      # renamed field: the new name is the binding
            else:
                names.append(label)      # punned field or typed binding
    return names


def strip_strings(line: str) -> str:
    """Blank out string literals so their contents are not linted."""
    return re.sub(r'"(?:[^"\\]|\\.)*"', lambda m: '"' + " " * (len(m.group(0)) - 2) + '"', line)


CLAUSE_TEXT = re.compile(r"^(\s*(?:purpose|tags|see also|deprecated):)(.*)$")


def strip_clause_text(line: str) -> str:
    """Blank out the free text of a documentation clause, which is not code."""
    return CLAUSE_TEXT.sub(lambda m: m.group(1), line)


def lint_file(path: pathlib.Path, known: set[str]) -> list[str]:
    problems = []
    opened = 0
    closed = 0
    previous = ""
    in_ability_declaration = False
    in_block_string = False
    full_text = path.read_text(encoding="utf-8")
    bindings: list[tuple[str, str]] = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        where = f"{path.name}:{number}"
        if in_block_string:
            if len(raw) > 100:
                problems.append(f"{where}: line is {len(raw)} columns (limit 100)")
            if "\t" in raw:
                problems.append(f"{where}: tab character")
            if raw.count('"""') % 2 == 1:
                in_block_string = False
            continue
        if raw.count('"""') % 2 == 1:
            in_block_string = True
            raw = raw.split('"""', 1)[0].rstrip()
        line = strip_clause_text(strip_strings(raw))
        code = line.split("#", 1)[0] if " #" in line or line.lstrip().startswith("#") else line
        if len(raw) > 100:
            problems.append(f"{where}: line is {len(raw)} columns (limit 100)")
        if "\t" in raw:
            problems.append(f"{where}: tab character")
        if raw != raw.rstrip():
            problems.append(f"{where}: trailing whitespace")
        for pattern, message in FORBIDDEN:
            if re.search(pattern, code):
                problems.append(f"{where}: {message}")
        for call in CALL.findall(re.sub(r"\bneeds .*$", "", code)):
            if call not in known:
                problems.append(f"{where}: '{call}' is not declared in the standard library sketch or the corpus")
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
        bindings.extend((name, where) for name in bound_names(code))
        if re.match(r"^(if |otherwise if |when )", stripped) and " then" not in stripped:
            problems.append(f"{where}: condition without 'then'")
        expression_context = previous.endswith(" be") or previous == "return"
        if re.match(r"^(public )?ability \w+( of [A-Z]\w*(, [A-Z]\w*)*)?$", stripped):
            in_ability_declaration = True
        if stripped.startswith("for each"):
            if not expression_context and not QUERY_TAIL.search(stripped):
                opened += 1
        elif STARTERS.match(code):
            if not (in_ability_declaration and stripped.startswith("function ")):
                opened += 1
        if stripped == "end" or (stripped.startswith("if ") and stripped.endswith(" end")):
            closed += 1
            if in_ability_declaration and opened == closed:
                in_ability_declaration = False
        if stripped:
            previous = stripped
    if opened != closed:
        problems.append(f"{path.name}: block starters {opened} vs 'end' {closed} (heuristic)")
    for name, where in bindings:
        if len(re.findall(rf"\b{name}\b", full_text)) < 2:
            problems.append(f"{where}: binding '{name}' is never used (heuristic)")
    return problems


def main() -> int:
    files = sorted((ROOT / "examples").glob("*.ry")) + sorted((ROOT / "examples").glob("*.renyi"))
    known = declared_functions([STDLIB_SKETCH, *files])
    all_problems = []
    for path in files:
        all_problems.extend(lint_file(path, known))
    for problem in all_problems:
        print(problem)
    print(f"{len(files)} files, {len(all_problems)} problems")
    return 1 if all_problems else 0


if __name__ == "__main__":
    sys.exit(main())
