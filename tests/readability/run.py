#!/usr/bin/env python3
"""Harness for the LLM readability test (docs/design/03-readability-test.md).

Subcommands, run from the repository root:

  prepare   build the prompts for one grammar revision into
            tests/readability/<date>-<revision>/prompts/; a prompt file
            never holds the answer key (reference output, original body)
  run       send every prompt to one model and store the raw samples
  score     score the stored samples (Predict exactly, Complete and Write with
            the lint plus a judgement file, Explain with a grading model);
            `renyi format` runs before the lint unless --no-format is given,
            and --scores names the output file so that both tallies can be kept
  report    print pass rates per model and task

Only the standard library is used; vendors are reached over HTTPS with the
credentials in ANTHROPIC_API_KEY and OPENAI_API_KEY (or any OpenAI-compatible
endpoint through --base-url). The `file` provider reads completions from a
directory so that a run can also be done by hand.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import pathlib
import re
import subprocess
import sys
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
    "A program's documentation clauses say what it does:\n\n{purposes}\n\n"
    "A reader who saw only the program wrote this explanation:\n\n{explanation}\n\n"
    "Grade the explanation from 1 to 5: 5 means it states the program's purpose and "
    "effects correctly and completely, 3 means it is right but misses an important "
    "part, 1 means it is wrong. Answer with the digit only."
)


# ----------------------------------------------------------------- helpers

def read(path: pathlib.Path) -> str:
    return path.read_text(encoding="utf-8")


def git_revision() -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                                       text=True).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def run_dir(name: str | None) -> pathlib.Path:
    if name:
        return HERE / name
    runs = sorted(p for p in HERE.iterdir() if p.is_dir() and re.match(r"\d{4}-\d{2}-\d{2}-", p.name))
    if not runs:
        sys.exit("no run directory; use `prepare` first")
    return runs[-1]


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


def lint_text(code: str, format_first: bool = False) -> list[str]:
    scratch = HERE / ".scratch.ry"
    scratch.write_text(code, encoding="utf-8")
    try:
        if format_first:
            # layout is the formatter's job; a program that does not parse is left as it is
            subprocess.run([str(renyi_binary()), "format", str(scratch)], capture_output=True)
        known = lint_examples.declared_functions(
            [lint_examples.STDLIB_SKETCH, *sorted(EXAMPLES.glob("*.ry")), scratch])
        return lint_examples.lint_file(scratch, known)
    finally:
        scratch.unlink(missing_ok=True)


def renyi_binary() -> pathlib.Path:
    """The `renyi` binary, built with `cargo build` when it is missing."""
    binary = ROOT / "target" / "debug" / "renyi"
    if not binary.exists():
        subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
    return binary


# ----------------------------------------------------------------- providers

def call_anthropic(model: str, system: str, user: str, temperature: float) -> str:
    key = os.environ.get("ANTHROPIC_API_KEY")
    if not key:
        sys.exit("ANTHROPIC_API_KEY is not set")
    base = os.environ.get("ANTHROPIC_BASE_URL", "https://api.anthropic.com").rstrip("/")
    body = {"model": model, "max_tokens": 4000, "temperature": temperature, "system": system,
            "messages": [{"role": "user", "content": user}]}
    data = post_json(f"{base}/v1/messages", body,
                     {"x-api-key": key, "anthropic-version": "2023-06-01"})
    return "".join(block.get("text", "") for block in data["content"])


def call_openai(model: str, system: str, user: str, temperature: float, base_url: str) -> str:
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        sys.exit("OPENAI_API_KEY is not set")
    body = {"model": model, "temperature": temperature,
            "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}]}
    data = post_json(f"{base_url.rstrip('/')}/chat/completions", body,
                     {"Authorization": f"Bearer {key}"})
    return data["choices"][0]["message"]["content"]


def post_json(url: str, body: dict, headers: dict) -> dict:
    request = urllib.request.Request(url, data=json.dumps(body).encode("utf-8"), method="POST",
                                     headers={"Content-Type": "application/json", **headers})
    try:
        with urllib.request.urlopen(request, timeout=300) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        sys.exit(f"{url}: HTTP {error.code}: {error.read().decode('utf-8', 'replace')[:500]}")


def complete(provider: str, model: str, system: str, user: str, temperature: float,
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
                         PROMPTS["predict"].format(arguments=json.dumps(arguments), program=program),
                         {"arguments": arguments})
            count += 1
        write_prompt(prompts / "explain" / f"{name}.json", "explain", name,
                     PROMPTS["explain"].format(program=program), {"purposes": purposes(program)})
        count += 1
        if "complete" in entry:
            function = entry["complete"]
            write_prompt(prompts / "complete" / f"{name}.json", "complete", name,
                         PROMPTS["complete"].format(target=function, program=remove_body(program, function)),
                         {"target": function})
            count += 1
    for task_id, description in write_tasks.items():
        write_prompt(prompts / "write" / f"{task_id}.json", "write", task_id,
                     PROMPTS["write"].format(task=description), {"description": description})
        count += 1
    (target / "system.txt").write_text(SYSTEM.format(cheatsheet=read(CHEATSHEET)), encoding="utf-8")
    (target / "meta.json").write_text(json.dumps({
        "date": dt.date.today().isoformat(), "revision": revision,
        "cheatsheet_bytes": CHEATSHEET.stat().st_size, "programs": len(programs)}, indent=2))
    print(f"{count} prompts written to {target.relative_to(ROOT)}")


def write_prompt(path: pathlib.Path, task: str, name: str, prompt: str, extra: dict) -> None:
    path.write_text(json.dumps({"task": task, "name": name, "prompt": prompt, **extra},
                               indent=2, ensure_ascii=False), encoding="utf-8")


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
                out_file.write_text(json.dumps(record, indent=2, ensure_ascii=False), encoding="utf-8")
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
        for task in TASKS:
            for out_file in sorted((label_dir / task).glob("*.json")):
                prompt = json.loads(read(target / "prompts" / task / out_file.name))
                record = json.loads(read(out_file))
                results = []
                for index, sample in enumerate(record["samples"]):
                    key = f"{task}/{out_file.stem}.{index}"
                    results.append(score_sample(task, prompt, sample, judgement.get(key), args, target))
                scores[f"{task}/{out_file.stem}"] = results
        (label_dir / args.scores).write_text(json.dumps(scores, indent=2), encoding="utf-8")
        pending = sum(1 for results in scores.values() for r in results if r.get("pending"))
        print(f"{label_dir.name}: {len(scores)} items scored, {pending} samples await judgement")


def score_sample(task: str, prompt: dict, sample: str, judged, args, target) -> dict:
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
        problems = lint_text(code, format_first=args.format)
        result = {"lint_problems": problems}
        if problems:
            result["pass"] = False
            result["rules"] = sorted({rule_of(p) for p in problems})
        elif judged is None:
            result["pending"] = True
        else:
            result["pass"] = bool(judged)
        return result
    if task == "explain":
        if judged is not None:
            return {"grade": judged, "pass": judged >= 4}
        if not args.grader:
            return {"pending": True}
        provider, model = args.grader.split(":", 1)
        answer = complete(provider, model, "You grade explanations of programs.",
                          EXPLAIN_GRADER.format(purposes=prompt["purposes"], explanation=sample),
                          0.0, args.base_url, None, "")
        digits = re.findall(r"[1-5]", answer)
        grade = int(digits[0]) if digits else 0
        return {"grade": grade, "pass": grade >= 4}
    return {"pending": True}


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


def rule_of(problem: str) -> str:
    """Map a lint message to a stable rule name for the per-rule breakdown."""
    for needle, rule in RULES:
        if needle in problem:
            return rule
    return "other"


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
    r.add_argument("--temperature", type=float, default=0.0)
    r.add_argument("--base-url", default="https://api.openai.com/v1",
                   help="OpenAI-compatible endpoint for --provider openai")
    r.add_argument("--answers", type=pathlib.Path, help="directory of <task>/<name>.<index>.txt for --provider file")
    r.add_argument("--tasks", help="comma-separated subset of predict,explain,complete,write")
    r.add_argument("--run", help="run directory name; defaults to the latest")
    r.set_defaults(func=cmd_run)
    s = sub.add_parser("score")
    s.add_argument("--run"); s.add_argument("--label")
    s.add_argument("--grader", help="provider:model used to grade Explain, for example anthropic:claude-sonnet-5-5")
    s.add_argument("--base-url", default="https://api.openai.com/v1")
    s.add_argument("--no-format", dest="format", action="store_false",
                   help="skip `renyi format` before the lint (the strict tally; decision M5 formats first)")
    s.add_argument("--scores", default="scores.json", help="file name for the scores under outputs/<label>/")
    s.set_defaults(func=cmd_score)
    t = sub.add_parser("report"); t.add_argument("--run")
    t.add_argument("--scores", default="scores.json", help="file name of the scores under outputs/<label>/")
    t.set_defaults(func=cmd_report)
    args = parser.parse_args()
    if args.command == "run" and not args.label:
        args.label = re.sub(r"[^A-Za-z0-9_.-]", "_", args.model)
    args.func(args)


if __name__ == "__main__":
    main()
