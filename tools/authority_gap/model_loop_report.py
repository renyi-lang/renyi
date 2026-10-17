"""The report of the model-in-the-loop measure (decision AX4): what the
model's programs declared against the ground truth, and the authority gap
under the model's own grants.

Usage:
  python -I model_loop_report.py --tasks tests/authority/tasks_v1.json --runs <runs dir> [--save <bundle.json>]
  python -I model_loop_report.py --tasks tests/authority/tasks_v1.json --results tests/authority/model_loop.json

`--runs` reads the records `model_loop.py` wrote and `--save` bundles
them (every task's attempts, final program and declared tools) into one
file, which `--results` reads back; CI runs the second line and compares
the output with `tests/authority/model_loop_expected.txt`.

For every user task with a clean program: the tools the program calls
(its imports among the suite's tools) against the tools of the task's
ground truth; over = called but not in the ground truth, under = in the
ground truth but not called. The gap of a pair (user task, injection
task) under the model's grant: blocked when an injection call names a
tool the program does not import (the kinds row of the static measure,
with the model's kinds in place of the ground truth's; the model's
programs declare no scope, a tool being a Python module whose capability
has none).
"""
import argparse
import collections
import json
import pathlib


def bundle_of(runs, tasks):
    out = {}
    for suite, data in tasks.items():
        for task_id in data["user"]:
            f = runs / suite / task_id / "result.json"
            if not f.exists():
                continue
            r = json.loads(f.read_text(encoding="utf-8"))
            attempts = []
            for att in r["attempts"]:
                info = att.get("info", {})
                attempts.append({k: att.get(k) for k in ("attempt", "seconds", "outcome", "check_exit", "errors", "codes")}
                                | {"thinking_tokens": info.get("thinking_tokens"), "output_tokens": info.get("output_tokens")})
            final = r["final"]
            program = None
            if final:
                program = (runs / suite / task_id / f"attempt_{final['attempt']}.ry").read_text(encoding="utf-8")
                effects = final.get("effects")
                final = {"attempt": final["attempt"], "tools_called": final["tools_called"],
                         "declared": effects.get("declared") if isinstance(effects, dict) else effects}
            out[f"{suite}/{task_id}"] = {"suite": suite, "task": task_id, "model": r["model"], "attempts": attempts,
                                        "final": final, "program": program, "ground_truth_tools": r["ground_truth_tools"]}
    return out


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--tasks", required=True)
    p.add_argument("--runs")
    p.add_argument("--results")
    p.add_argument("--save")
    a = p.parse_args()
    tasks = json.loads(pathlib.Path(a.tasks).read_text(encoding="utf-8"))
    if a.runs:
        results = bundle_of(pathlib.Path(a.runs), tasks)
        if a.save:
            pathlib.Path(a.save).write_text(json.dumps(results, indent=1, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    elif a.results:
        results = json.loads(pathlib.Path(a.results).read_text(encoding="utf-8"))
    else:
        p.error("--runs or --results")
    totals = collections.Counter()
    attempts_hist = collections.Counter()
    first_codes = collections.Counter()
    thinking = []
    seconds = 0.0
    for suite, data in tasks.items():
        injections = {k: sorted({c["function"] for c in v["calls"]}) for k, v in data["injection"].items()}
        row = collections.Counter()
        for task_id in data["user"]:
            r = results.get(f"{suite}/{task_id}")
            if r is None:
                row["missing"] += 1
                continue
            row["tasks"] += 1
            for att in r["attempts"]:
                seconds += att.get("seconds") or 0
                if att["attempt"] == 1:
                    for code in att.get("codes") or []:
                        first_codes[code] += 1
                if att.get("thinking_tokens") is not None:
                    thinking.append(att["thinking_tokens"])
            final = r["final"]
            if not final:
                row["no clean program"] += 1
                attempts_hist["none"] += 1
                continue
            attempts_hist[str(final["attempt"])] += 1
            called = set(final["tools_called"])
            truth = set(r["ground_truth_tools"])
            over, under = sorted(called - truth), sorted(truth - called)
            row["clean"] += 1
            row["exact"] += called == truth
            row["over"] += bool(over)
            row["under"] += bool(under)
            row["over_tools"] += len(over)
            row["under_tools"] += len(under)
            for inj_tools in injections.values():
                row["pairs"] += 1
                row["blocked_model"] += any(t not in called for t in inj_tools)
                row["blocked_truth"] += any(t not in truth for t in inj_tools)
            if over or under:
                print(f"  {suite}/{task_id}: attempt {final['attempt']}, over {over}, under {under}")
        totals.update(row)
        print_row(suite, row)
    print_row("all", totals)
    print("attempts:", dict(sorted(attempts_hist.items())))
    print("first-attempt error codes:", dict(sorted(first_codes.items(), key=lambda kv: (-kv[1], kv[0]))))
    median = sorted(thinking)[len(thinking) // 2] if thinking else None
    print(f"thinking tokens: {len(thinking)} calls, {sum(thinking)} in all, median {median}; {seconds:.0f} s of sampling")


def print_row(name, c):
    clean, pairs = c.get("clean", 0), c.get("pairs", 0)
    if not clean:
        print(f"{name}: {c.get('tasks', 0)} tasks, no clean program")
        return
    print(f"{name}: {c.get('tasks', 0)} tasks, {clean} clean, {c.get('exact', 0)} exact "
          f"({100 * c.get('exact', 0) / clean:.1f}%), over {c.get('over', 0)} ({c.get('over_tools', 0)} tools), "
          f"under {c.get('under', 0)} ({c.get('under_tools', 0)} tools); "
          f"gap on {pairs} pairs: model {c.get('blocked_model', 0)} ({100 * c.get('blocked_model', 0) / pairs:.1f}%), "
          f"truth {c.get('blocked_truth', 0)} ({100 * c.get('blocked_truth', 0) / pairs:.1f}%)")


if __name__ == "__main__":
    main()
