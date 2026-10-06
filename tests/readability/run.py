#!/usr/bin/env python3
"""Harness for the LLM readability test (docs/design/03-readability-test.md).

Subcommands, run from the repository root:

  prepare   build the prompts for one grammar revision into
            tests/readability/<date>-<revision>/prompts/; a prompt file
            never holds the answer key (reference output, original body)
  run       send every prompt to one model and store the raw samples
  score     score the stored samples (Predict exactly, Complete and Write with
            the lint, then `renyi check`, then the VM running the program's own
            examples and tests, then a judgement file for a program with
            nothing to run, Explain with a grading model whose grades are
            cached in grades.json); `renyi format` runs before the lint unless
            --no-format is given, and --scores names the output file so that
            both tallies can be kept
  report    print pass rates per model and task
  compare   decision L2: the VM's verdict on every judged Complete and Write
            sample against the judgement file's

Only the standard library is used; vendors are reached over HTTPS with the
credentials in ANTHROPIC_API_KEY and OPENAI_API_KEY (or any OpenAI-compatible
endpoint through --base-url). The `file` provider reads completions from a
directory so that a run can also be done by hand.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[2]
HERE = pathlib.Path(__file__).resolve().parent
EXAMPLES = ROOT / "examples"
CHEATSHEET = ROOT / "docs" / "cheatsheet.md"
MANIFEST = HERE / "manifest.json"
WRITE_TASKS = HERE / "write_tasks.json"
sys.path.insert(0, str(ROOT / "tools"))
import lint_examples  # noqa: E402

TASKS = ("predict", "explain", "complete", "write")
SYSTEM = (
    "You are reading programs in Renyi, a small statically typed language whose "
    "syntax is regular English. Everything you know about the language is in the "
    "cheat sheet below. Do not assume features it does not list.\n\n"
    "=== RENYI CHEAT SHEET ===\n{cheatsheet}\n=== END OF CHEAT SHEET ==="
)
PROMPTS = {
    "predict": (
        "Here is a Renyi program. It is run with the command-line arguments {arguments} "
        "and no other input. Output exactly what the program prints to standard output, "
        "and nothing else: no explanation, no code fences.\n\n{program}"
    ),
    "explain": (
        "Explain what this Renyi program does in at most three sentences, for a reader "
        "who will decide whether to use it. Do not quote code.\n\n{program}"
    ),
    "complete": (
        "The body of the function `{target}` was removed from this Renyi program; its "
        "signature and documentation clauses are kept. Write the complete function "
        "`{target}`, from its head line to its `end`, so that every `example:` line and "
        "every `test` block in the program holds. Answer with the function only, in one "
        "code fence.\n\n{program}"
    ),
    "write": (
        "Write a complete Renyi program for the task below. Follow the cheat sheet "
        "exactly: no features it does not list. Give every public definition a "
        "`purpose:` clause and give pure functions `example:` lines. Answer with the "
        "program only, in one code fence.\n\nTask: {task}"
    ),
}
EXPLAIN_GRADER = (
    "The author of a program describes what it does:\n\n{reference}\n\n"
    "A reader who saw only the program wrote this explanation:\n\n{explanation}\n\n"
    "Grade the explanation from 1 to 5 by these rules, judging only against the "
    "author's description. A statement the description contradicts is wrong; a "
    "statement it neither confirms nor contradicts is ignored; style, length, opinions "
    "about usefulness and cautions or doubts the reader adds are ignored.\n"
    "5: nothing is wrong, and the overall purpose, the effects (what the program needs: "
    "console, files, network, environment, clock) and the main behaviours are stated.\n"
    "4: nothing is wrong; one of those is missing.\n"
    "3: nothing is wrong, but the overall purpose is vague or several of those are "
    "missing.\n"
    "2: one statement is wrong.\n"
    "1: the explanation describes a different program or is mostly wrong.\n"
    "Answer with the digit only."
)
# grades are cached per grader and per wording of the rubric, so that a change
# to the rubric grades everything afresh without discarding the old grades
RUBRIC_ID = hashlib.sha256(EXPLAIN_GRADER.encode("utf-8")).hexdigest()[:8]


# ----------------------------------------------------------------- helpers

def read(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8")


def write(path: pathlib.Path, text: str) -> None:
    """Write UTF-8 with LF line endings on every platform (Python would write
    CRLF on Windows, and the repository pins LF)."""
    path.write_text(text, encoding="utf-8", newline="\n")


def git_revision() -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                                       text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def run_dir(name: str | None) -> pathlib.Path:
    """The run directory named, or else the one `prepare` wrote most recently.

    Two runs prepared on one day differ only in the revision hash, which sorts
    arbitrarily, so the choice goes by the time `prepare` wrote `meta.json`.
    """
    if name:
        target = HERE / name
        if not target.is_dir():
            sys.exit(f"no run directory {target.relative_to(ROOT)}")
        return target
    runs = [p for p in HERE.iterdir() if p.is_dir() and re.match(r"\d{4}-\d{2}-\d{2}-", p.name)]
    if not runs:
        sys.exit("no run directory; use `prepare` first")
    return max(runs, key=lambda p: ((p / "meta.json").stat().st_mtime
                                     if (p / "meta.json").exists() else 0.0, p.name))


def remove_body(program: str, target: str) -> str:
    """Replace the body of `target` with a placeholder line, keeping head and clauses."""
    lines = program.splitlines()
    head = next((i for i, line in enumerate(lines)
                 if re.match(rf"^(public )?function {re.escape(target)}\(", line)), None)
    if head is None:
        raise ValueError(f"no function {target}")
    blank = next(i for i in range(head, len(lines)) if lines[i].strip() == "")
    end = next(i for i in range(blank, len(lines)) if lines[i] == "end")
    return "\n".join(lines[:blank + 1] + ["  # ... body removed ..."] + lines[end:]) + "\n"


def purposes(program: str) -> str:
    return "\n".join(line.strip() for line in program.splitlines()
                     if line.strip().startswith(("purpose:", "example:")))


def extract_code(answer: str) -> str:
    fenced = re.findall(r"```[a-zA-Z]*\n(.*?)```", answer, flags=re.S)
    return (fenced[0] if fenced else answer).strip("\n") + "\n"


def lint_text(code: str, format_first: bool = False) -> tuple[list[str], list[str], dict | None]:
    """The lint's problems; when the lint is clean, the checker's errors; and
    when the checker is clean too, the VM's verdict on the program's own
    `example:` lines and `test` blocks (decision L2), or None when it has
    none to run.

    The scratch file lives in the examples directory so that a program's
    imports of other corpus modules resolve (the checker reads imports from
    the file's own directory). Its name carries the process id so that
    several `score` processes can run at once: a shared name let one process
    read, format or delete another's sample.
    """
    scratch = EXAMPLES / f".scratch-{os.getpid()}.ry"
    # LF on every platform: Python would otherwise write CRLF on Windows, and
    # the formatter and the checker reject carriage returns
    write(scratch, code.replace("\r\n", "\n"))
    try:
        if format_first:
            # layout is the formatter's job; a program that does not parse is left as it is
            subprocess.run([str(renyi_binary()), "format", str(scratch)], capture_output=True)
        # the corpus without any scratch file (another process's sample must
        # not lend its declarations to this one)
        corpus = sorted(p for p in EXAMPLES.glob("*.ry") if not p.name.startswith("."))
        known = lint_examples.declared_functions([lint_examples.STDLIB_SKETCH, *corpus, scratch])
        problems = [unnamed(p, scratch.name) for p in lint_examples.lint_file(scratch, known)]
        if problems:
            return problems, [], None
        errors = check_errors(scratch)
        if errors:
            return [], errors, None
        return [], [], vm_verdict(scratch)
    finally:
        scratch.unlink(missing_ok=True)


def vm_verdict(path: pathlib.Path) -> dict | None:
    """What `renyi test` says of the program's own `example:` lines and `test`
    blocks: the summary line, whether every item passed, and the failing
    items; None when the program has nothing to run, which leaves the
    verdict to the judgement file."""
    result = subprocess.run([str(renyi_binary()), "test", str(path)],
                            capture_output=True, text=True, encoding="utf-8")
    lines = result.stdout.strip().splitlines()
    summary = lines[-1] if lines else ""
    found = re.match(r"(\d+) passed, (\d+) failed", summary)
    if not found:
        return {"summary": (result.stderr.strip() or summary)[:200], "pass": False}
    passed, failed = int(found.group(1)), int(found.group(2))
    if passed + failed == 0:
        return None
    verdict = {"summary": summary, "pass": failed == 0}
    failures = [line.strip() for line in lines if line.startswith(("FAIL", "      "))]
    if failures:
        verdict["failures"] = failures[:8]
    return verdict


def unnamed(problem: str, name: str) -> str:
    """A lint message without the scratch file's name, which varies per process:
    `line 54: ...` for a located problem, the bare message otherwise."""
    rest = problem.removeprefix(name).lstrip(":").strip()
    return f"line {rest}" if rest[:1].isdigit() else rest


def check_errors(path: pathlib.Path) -> list[str]:
    """The errors `renyi check --json` reports, as `[code] message`."""
    result = subprocess.run([str(renyi_binary()), "check", "--json", str(path)],
                            capture_output=True, text=True, encoding="utf-8")
    try:
        diagnostics = json.loads(result.stdout)
    except json.JSONDecodeError:
        return [f"[check] renyi check produced no diagnostics: {result.stderr.strip()[:200]}"]
    return [f"[{d['code']}] {d['message']}" for d in diagnostics if d["severity"] == "error"]


def renyi_binary() -> pathlib.Path:
    """The `renyi` binary, built with `cargo build` when it is missing."""
    binary = ROOT / "target" / "debug" / ("renyi.exe" if os.name == "nt" else "renyi")
    if not binary.exists():
        subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
    return binary


# ----------------------------------------------------------------- providers

def call_anthropic(model: str, system: str, user: str, temperature: float | None) -> str:
    key = os.environ.get("ANTHROPIC_API_KEY")
    if not key:
        sys.exit("ANTHROPIC_API_KEY is not set")
    base = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com").rstrip("/")
    body = {"model": model, "max_tokens": 4000, "system": system,
            "messages": [{"role": "user", "content": user}]}
    if temperature is not None:  # some models reject the field outright
        body["temperature"] = temperature
    data = post_json(f"{base}/v1/messages", body,
                     {"x-api-key": key, "anthropic-version": "2023-06-01"})
    return "".join(block.get("text", "") for block in data["content"])


def call_openai(model: str, system: str, user: str, temperature: float | None,
                base_url: str) -> str:
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        sys.exit("OPENAI_API_KEY is not set")
    body = {"model": model,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    if temperature is not None:
        body["temperature"] = temperature
    data = post_json(f"{base_url.rstrip('/')}/chat/completions", body,
                     {"Authorization": f"Bearer {key}"})
    return data["choices"][0]["message"]["content"]


TRANSIENT_STATUS = {408, 409, 429, 500, 502, 503, 504, 529}
ATTEMPTS = 6


def post_json(url: str, body: dict, headers: dict) -> dict:
    """POST and decode JSON; waits and retries on transient failures.

    A round is several hundred sequential calls, so a rate limit or an overloaded
    vendor would otherwise end the run on its first occurrence. Only transient
    statuses and connection errors are retried, with exponential backoff; any
    other error still stops the run.
    """
    data = json.dumps(body).encode("utf-8")
    for attempt in range(1, ATTEMPTS + 1):
        request = urllib.request.Request(url, data=data, method="POST",
                                         headers={"Content-Type": "application/json", **headers})
        delay = 5.0 * 2 ** (attempt - 1)
        try:
            with urllib.request.urlopen(request, timeout=300) as response:
                return json.loads(response.read().decode("utf-8"))
        except urllib.error.HTTPError as error:
            detail = error.read().decode("utf-8", "replace")[:500]
            if error.code not in TRANSIENT_STATUS or attempt == ATTEMPTS:
                sys.exit(f"{url}: HTTP {error.code}: {detail}")
            try:
                delay = max(delay, float(error.headers.get("retry-after") or 0))
            except ValueError:
                pass  # a date-valued retry-after is rare; the backoff delay stands
            print(f"HTTP {error.code}, retry {attempt}/{ATTEMPTS} in {delay:.0f}s: {detail[:120]}",
                  file=sys.stderr)
        except OSError as error:  # connection reset, DNS failure, read timeout
            if attempt == ATTEMPTS:
                sys.exit(f"{url}: {error}")
            print(f"{error}, retry {attempt}/{ATTEMPTS} in {delay:.0f}s", file=sys.stderr)
        time.sleep(delay)  # because the failure was transient, waiting is the fix
    raise AssertionError("unreachable: every path above returns or exits")


def complete(provider: str, model: str, system: str, user: str, temperature: float | None,
             base_url: str, answers_dir: pathlib.Path | None, key: str) -> str:
    if provider == "anthropic":
        return call_anthropic(model, system, user, temperature)
    if provider == "openai":
        return call_openai(model, system, user, temperature, base_url)
    if provider == "file":
        path = (answers_dir or HERE) / f"{key}.txt"
        if not path.exists():
            raise FileNotFoundError(path)
        return read(path)
    sys.exit(f"unknown provider {provider}")


# ----------------------------------------------------------------- commands

def cmd_prepare(args: argparse.Namespace) -> None:
    manifest = json.loads(read(MANIFEST))
    write_tasks = json.loads(read(WRITE_TASKS))
    revision = args.revision or git_revision()
    target = HERE / f"{dt.date.today().isoformat()}-{revision}"
    prompts = target / "prompts"
    for task in TASKS:
        (prompts / task).mkdir(parents=True, exist_ok=True)
    programs = sorted(EXAMPLES.glob("*.ry"))
    count = 0
    for path in programs:
        name, program = path.stem, read(path)
        entry = manifest["programs"].get(name, {})
        if "predict" in entry:
            assert (HERE / "reference" / f"{name}.out").exists(), f"no reference output for {name}"
            arguments = entry["predict"].get("arguments", [])
            write_prompt(prompts / "predict" / f"{name}.json", "predict", name,
                         PROMPTS["predict"].format(arguments=json.dumps(arguments),
                                                   program=with_imports(program)),
                         {"arguments": arguments})
            count += 1
        write_prompt(prompts / "explain" / f"{name}.json", "explain", name,
                     PROMPTS["explain"].format(program=with_imports(program)),
                     {"purposes": purposes(program)})
        count += 1
        if "complete" in entry:
            function = entry["complete"]
            write_prompt(prompts / "complete" / f"{name}.json", "complete", name,
                         PROMPTS["complete"].format(
                             target=function, program=with_imports(remove_body(program, function))),
                         {"target": function})
            count += 1
    for task_id, description in write_tasks.items():
        write_prompt(prompts / "write" / f"{task_id}.json", "write", task_id,
                     PROMPTS["write"].format(task=description), {"description": description})
        count += 1
    write(target / "system.txt", SYSTEM.format(cheatsheet=read(CHEATSHEET)))
    write(target / "meta.json", json.dumps({
        "date": dt.date.today().isoformat(), "revision": revision,
        "cheatsheet_bytes": CHEATSHEET.stat().st_size, "programs": len(programs)}, indent=2))
    print(f"{count} prompts written to {target.relative_to(ROOT)}")


def with_imports(program: str) -> str:
    """The program followed by every corpus module it imports.

    A program's behaviour depends on the modules it imports, so a reader who
    is asked to predict, explain or complete it must see them too. Library
    modules (`std.*`) are described by the cheat sheet and are not repeated.
    """
    text = program
    for module in re.findall(r"^import ([a-z_][a-z0-9_]*)(?: exposing [^\n]*)?$", program, re.M):
        path = EXAMPLES / f"{module}.ry"
        if path.exists():
            text += f"\nThe program imports the module `{module}`, which is this file:\n\n{read(path)}"
    return text


def write_prompt(path: pathlib.Path, task: str, name: str, prompt: str, extra: dict) -> None:
    write(path, json.dumps({"task": task, "name": name, "prompt": prompt, **extra},
                           indent=2, ensure_ascii=False))


def cmd_run(args: argparse.Namespace) -> None:
    target = run_dir(args.run)
    system = read(target / "system.txt")
    outputs = target / "outputs" / args.label
    tasks = args.tasks.split(",") if args.tasks else TASKS
    done = 0
    for task in tasks:
        (outputs / task).mkdir(parents=True, exist_ok=True)
        for prompt_file in sorted((target / "prompts" / task).glob("*.json")):
            out_file = outputs / task / prompt_file.name
            record = json.loads(read(out_file)) if out_file.exists() else {"samples": []}
            prompt = json.loads(read(prompt_file))
            while len(record["samples"]) < args.samples:
                index = len(record["samples"])
                key = f"{task}/{prompt_file.stem}.{index}"
                try:
                    answer = complete(args.provider, args.model, system, prompt["prompt"],
                                      args.temperature, args.base_url, args.answers, key)
                except FileNotFoundError as missing:
                    print(f"skipping {key}: {missing}", file=sys.stderr)
                    break
                record["samples"].append(answer)
                record["model"] = args.model
                record["temperature"] = args.temperature
                write(out_file, json.dumps(record, indent=2, ensure_ascii=False))
                done += 1
    print(f"{done} samples stored under {outputs.relative_to(ROOT)}")


def output_dirs(target: pathlib.Path) -> list[pathlib.Path]:
    outputs = target / "outputs"
    if not outputs.exists():
        sys.exit(f"no outputs under {target.relative_to(ROOT)}; use `run` first")
    return sorted(p for p in outputs.iterdir() if p.is_dir())


def cmd_score(args: argparse.Namespace) -> None:
    target = run_dir(args.run)
    for label_dir in output_dirs(target):
        if args.label and label_dir.name != args.label:
            continue
        scores = {}
        judgement_file = label_dir / "judgement.json"
        judgement = json.loads(read(judgement_file)) if judgement_file.exists() else {}
        # grades already obtained from a grading model, by grader, so that a
        # re-score does not pay for them again
        grades_file = label_dir / "grades.json"
        grades = json.loads(read(grades_file)) if grades_file.exists() else {}
        for task in TASKS:
            for out_file in sorted((label_dir / task).glob("*.json")):
                prompt = json.loads(read(target / "prompts" / task / out_file.name))
                record = json.loads(read(out_file))
                results = []
                before = json.dumps(grades, sort_keys=True)
                for index, sample in enumerate(record["samples"]):
                    key = f"{task}/{out_file.stem}.{index}"
                    results.append(score_sample(task, prompt, sample, judgement.get(key), args,
                                                target, key, grades))
                scores[f"{task}/{out_file.stem}"] = results
                if json.dumps(grades, sort_keys=True) != before:
                    write(grades_file, json.dumps(grades, indent=2, sort_keys=True))
        write(label_dir / args.scores, json.dumps(scores, indent=2))
        pending = sum(1 for results in scores.values() for r in results if r.get("pending"))
        print(f"{label_dir.name}: {len(scores)} items scored, {pending} samples await judgement")


def score_sample(task: str, prompt: dict, sample: str, judged, args, target,
                 key: str, grades: dict) -> dict:
    if isinstance(judged, dict):
        judged = judged.get("verdict")  # {"verdict": ..., "reason": "..."} form
    if task == "predict":
        expected = read(HERE / "reference" / f"{prompt['name']}.out").rstrip()
        actual = sample.strip().strip("`").rstrip()
        return {"pass": actual == expected}
    if task in ("complete", "write"):
        code = extract_code(sample)
        if task == "complete":
            original = read(EXAMPLES / f"{prompt['name']}.ry")
            code = splice(original, prompt["target"], code)
        problems, errors, vm = lint_text(code, format_first=args.format)
        result = {"lint_problems": problems}
        if problems:
            result["pass"] = False
            result["rules"] = sorted({rule_of(p) for p in problems})
        elif errors:
            # the type and effect checker decides before any judgement does
            result["check_errors"] = errors
            result["pass"] = False
            result["rules"] = sorted({"check:" + e[1:e.index("]")] for e in errors})
        elif vm is not None:
            # the VM decides from the program's own examples and tests
            # (decision L2); a judge's verdict is kept beside it for `compare`
            result["vm"] = vm["summary"]
            if "failures" in vm:
                result["failures"] = vm["failures"]
            result["pass"] = vm["pass"]
            if judged is not None:
                result["judged"] = bool(judged)
        elif judged is None:
            result["pending"] = True
        else:
            result["pass"] = bool(judged)
        return result
    if task == "explain":
        if judged is not None:
            return {"grade": judged, "pass": judged >= 4, "grader": "judgement"}
        if not args.grader:
            return {"pending": True}
        # both graders grade every explanation (protocol: two graders where they
        # disagree by more than one); the first grader's grade stands when the
        # second is within one of it, a wider disagreement awaits adjudication
        # in judgement.json, and a grader that answers without a digit (a
        # refusal) decides nothing
        # the author's description of the program (reference/<name>.explain.txt);
        # the purpose and example lines stand in when there is none
        description = HERE / "reference" / f"{prompt['name']}.explain.txt"
        reference = read(description) if description.exists() else prompt["purposes"]
        first = grade_of(args.grader, key, sample, reference, args, grades)
        second = (grade_of(args.second_grader, key, sample, reference, args, grades)
                  if args.second_grader else 0)
        if first and second and abs(first - second) > 1:
            return {"pending": True, "grades": {args.grader: first, args.second_grader: second},
                    "note": "the graders disagree by more than one"}
        grade = first or second
        if not grade:
            return {"pending": True, "note": "no grader gave a grade"}
        result = {"grade": grade, "pass": grade >= 4,
                  "grader": args.grader if first else args.second_grader}
        if first and second:
            result["second_grade"] = second
        return result
    return {"pending": True}


def grade_of(grader: str, key: str, sample: str, reference: str, args, grades: dict) -> int:
    """The grader's grade for the sample, from the cache or from a call; 0 when it gave none.

    The cache is kept by grader and rubric; an entry names the answer and the
    reference it graded, so that a regenerated answer or a changed reference
    is graded afresh, and an answer without a digit is not cached.
    """
    digest = hashlib.sha256((reference + "\n" + sample).encode("utf-8")).hexdigest()[:16]
    bucket = f"{grader}#{RUBRIC_ID}"
    cached = grades.setdefault(bucket, {}).get(key)
    if isinstance(cached, dict) and cached.get("sample") == digest:
        return cached["grade"]
    provider, model = grader.split(":", 1)
    answer = complete(provider, model, "You grade explanations of programs.",
                      EXPLAIN_GRADER.format(reference=reference, explanation=sample),
                      args.temperature, args.base_url, None, "")
    grade = parse_grade(answer)
    if not grade:
        print(f"{key}: {grader} gave no grade: {answer[:80]!r}", file=sys.stderr)
        return 0
    grades[bucket][key] = {"grade": grade, "sample": digest, "answer": answer.strip()[-2000:]}
    return grade


def parse_grade(answer: str) -> int:
    """The grade in a grader's answer: the last digit from 1 to 5 that stands alone.

    A grader asked for the digit only sometimes reasons first and ends with
    the digit; taking the first digit then returned the 1 of "18" or "10%"
    and failed correct explanations. A digit next to another digit or a dot
    ("18", "1.4.2", "0.50") is never the grade.
    """
    found = re.findall(r"(?<![\d.])[1-5](?![\d.])", answer)
    return int(found[-1]) if found else 0


def splice(original: str, target: str, function_text: str) -> str:
    lines = original.splitlines()
    head = next(i for i, line in enumerate(lines)
                if re.match(rf"^(public )?function {re.escape(target)}\(", line))
    end = next(i for i in range(head, len(lines)) if lines[i] == "end")
    return "\n".join(lines[:head] + function_text.rstrip("\n").splitlines() + lines[end + 1:]) + "\n"


RULES = [
    ("is not Renyi", "equals-sign"),
    ("symbolic operator", "symbolic-operator"),
    ("'<' or '>'", "angle-comparison"),
    ("semicolon", "semicolon"),
    ("keyword from another language", "foreign-keyword"),
    ("tab character", "whitespace"),
    ("trailing whitespace", "whitespace"),
    ("reserved word", "reserved-word-identifier"),
    ("single-letter identifier", "single-letter-identifier"),
    ("condition without 'then'", "missing-then"),
    ("columns (limit 100)", "line-width"),
    ("block starters", "block-balance"),
    ("is never used", "unused-binding"),
    ("not declared in the standard library sketch", "unknown-function"),
]
# a checker failure is reported under its diagnostic code, `check:<code>`


def rule_of(problem: str) -> str:
    """Map a lint message to a stable rule name for the per-rule breakdown."""
    for needle, rule in RULES:
        if needle in problem:
            return rule
    return "other"


def cmd_compare(args: argparse.Namespace) -> None:
    """Decision L2: the VM's verdict on every judged Complete and Write sample
    against the judgement file's, per model; the samples the lint or the
    checker now reject are listed apart, since neither judge saw them."""
    target = run_dir(args.run)
    for label_dir in output_dirs(target):
        if args.label and label_dir.name != args.label:
            continue
        judgement_file = label_dir / "judgement.json"
        if not judgement_file.exists():
            continue
        judgement = json.loads(read(judgement_file))
        agree = disagree = undecided = rejected = 0
        lines = []
        for key, entry in sorted(judgement.items()):
            task, rest = key.split("/", 1)
            if task not in ("complete", "write"):
                continue
            name, index = rest.rsplit(".", 1)
            prompt = json.loads(read(target / "prompts" / task / f"{name}.json"))
            record = json.loads(read(label_dir / task / f"{name}.json"))
            sample = record["samples"][int(index)]
            verdict = entry.get("verdict") if isinstance(entry, dict) else entry
            reason = entry.get("reason", "") if isinstance(entry, dict) else ""
            code = extract_code(sample)
            if task == "complete":
                code = splice(read(EXAMPLES / f"{prompt['name']}.ry"), prompt["target"], code)
            problems, errors, vm = lint_text(code, format_first=args.format)
            if problems or errors:
                rejected += 1
                lines.append(f"  -  {key}: now rejected before any judge ({(problems or errors)[0][:90]}); judged {verdict}")
            elif vm is None:
                undecided += 1
                lines.append(f"  ?  {key}: nothing for the VM to run; judged {verdict}")
            elif vm["pass"] == bool(verdict):
                agree += 1
            else:
                disagree += 1
                lines.append(f"  !  {key}: VM {vm['summary']}, judged {verdict}: {reason[:200]}")
                for failure in vm.get("failures", []):
                    lines.append(f"       {failure}")
        print(f"{label_dir.name}: {agree} agree, {disagree} disagree, {undecided} undecided by the VM, "
              f"{rejected} now rejected before any judge")
        for line in lines:
            print(line)


def cmd_report(args: argparse.Namespace) -> None:
    target = run_dir(args.run)
    print(f"run {target.name}")
    print(f"{'model':30} {'task':9} {'items':>5} {'pass':>5} {'pending':>7} {'rate':>6}")
    for label_dir in output_dirs(target):
        scores_file = label_dir / args.scores
        if not scores_file.exists():
            continue
        scores = json.loads(read(scores_file))
        for task in TASKS:
            items = {k: v for k, v in scores.items() if k.startswith(task + "/")}
            if not items:
                continue
            passed = sum(1 for results in items.values()
                         if sum(1 for r in results if r.get("pass")) >= max(1, (len(results) * 4 + 4) // 5))
            pending = sum(1 for results in items.values() if any(r.get("pending") for r in results))
            rate = 100 * passed / len(items)
            print(f"{label_dir.name:30} {task:9} {len(items):5} {passed:5} {pending:7} {rate:5.0f}%")
        rules: dict[str, int] = {}
        for key, results in scores.items():
            if key.startswith("write/"):
                for r in results:
                    for rule in r.get("rules", []):
                        rules[rule] = rules.get(rule, 0) + 1
        if rules:
            total = sum(rules.values())
            print(f"  lint rules violated in Write ({total} total):")
            for rule, n in sorted(rules.items(), key=lambda kv: -kv[1]):
                print(f"    {n:4}  {100 * n / total:3.0f}%  {rule}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare"); p.add_argument("--revision")
    p.set_defaults(func=cmd_prepare)
    r = sub.add_parser("run")
    r.add_argument("--provider", choices=["anthropic", "openai", "file"], required=True)
    r.add_argument("--model", required=True)
    r.add_argument("--label", help="directory name for the outputs; defaults to the model id")
    r.add_argument("--samples", type=int, default=5)
    r.add_argument("--temperature", default="0",
                   help="sampling temperature; `none` omits the field for models that reject it")
    r.add_argument("--base-url", default="https://api.openai.com/v1",
                   help="OpenAI-compatible endpoint for --provider openai")
    r.add_argument("--answers", type=pathlib.Path, help="directory of <task>/<name>.<index>.txt for --provider file")
    r.add_argument("--tasks", help="comma-separated subset of predict,explain,complete,write")
    r.add_argument("--run", help="run directory name; defaults to the latest")
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("score")
    s.add_argument("--run"); s.add_argument("--label")
    s.add_argument("--grader", help="provider:model used to grade Explain, for example anthropic:claude-sonnet-5-5")
    s.add_argument("--second-grader",
                   help="provider:model that grades every explanation as well; a grade more than one "
                        "away from the grader's leaves the sample to adjudication")
    s.add_argument("--base-url", default="https://api.openai.com/v1")
    s.add_argument("--temperature", default="0", help="the grader's temperature; `none` omits it")
    s.add_argument("--no-format", dest="format", action="store_false",
                   help="skip `renyi format` before the lint (the strict tally; decision M5 formats first)")
    s.add_argument("--scores", default="scores.json", help="file name for the scores under outputs/<label>/")
    s.set_defaults(func=cmd_score)
    t = sub.add_parser("report"); t.add_argument("--run")
    t.add_argument("--scores", default="scores.json", help="file name of the scores under outputs/<label>/")
    t.set_defaults(func=cmd_report)
    c = sub.add_parser("compare", help="decision L2: the VM's verdicts against judgement.json")
    c.add_argument("--run"); c.add_argument("--label")
    c.add_argument("--no-format", dest="format", action="store_false",
                   help="skip `renyi format` before the lint, as `score --no-format` does")
    c.set_defaults(func=cmd_compare)
    args = parser.parse_args()
    if args.command == "run" and not args.label:
        args.label = re.sub(r"[^A-Za-z0-9_.-]", "_", args.model)
    if args.command in ("run", "score"):
        args.temperature = None if args.temperature.lower() == "none" else float(args.temperature)
    args.func(args)


if __name__ == "__main__":
    main()
