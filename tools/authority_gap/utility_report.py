"""The report of the utility runs (decision AX5): the model's programs run
against AgentDojo's environments through the bridge, judged by each task's
own `utility`.

Usage:
  python -I utility_report.py --tasks tests/authority/tasks_v1.json --runs <utility runs dir> --loop <model loop runs dir> [--save <bundle.json>]
  python -I utility_report.py --tasks tests/authority/tasks_v1.json --results tests/authority/utility.json

`--runs` reads the records `utility_run.py` wrote (and `--loop` the model
loop's, for the attempt each program took and its call set against the
ground truth), `--save` bundles them into one file, which `--results`
reads back; CI runs the second line and compares the output with
`tests/authority/utility_expected.txt`.
"""
import argparse
import collections
import json
import pathlib


def bundle_of(runs, loop, tasks):
    out = {}
    for suite, data in tasks.items():
        for task_id in data["user"]:
            f = runs / suite / task_id / "utility.json"
            if not f.exists():
                continue
            r = json.loads(f.read_text(encoding="utf-8"))
            record = {"suite": suite, "task": task_id, "exit": r["run"]["exit"], "seconds": r["run"]["seconds"],
                      "output": r["run"]["stdout"], "calls": r["verdict"]["calls"],
                      "call_errors": r["verdict"]["call_errors"], "functions": r["verdict"]["functions"],
                      "utility": r["verdict"].get("utility"), "judge_error": r["verdict"].get("judge_error")}
            lf = loop / suite / task_id / "result.json" if loop else None
            if lf and lf.exists():
                lr = json.loads(lf.read_text(encoding="utf-8"))
                record["attempt"] = lr["final"]["attempt"] if lr["final"] else None
                record["tools_called"] = lr["final"]["tools_called"] if lr["final"] else []
                record["ground_truth_tools"] = lr["ground_truth_tools"]
            out[f"{suite}/{task_id}"] = record
    return out


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--tasks", required=True)
    p.add_argument("--runs")
    p.add_argument("--loop")
    p.add_argument("--results")
    p.add_argument("--save")
    a = p.parse_args()
    tasks = json.loads(pathlib.Path(a.tasks).read_text(encoding="utf-8"))
    if a.runs:
        results = bundle_of(pathlib.Path(a.runs), pathlib.Path(a.loop) if a.loop else None, tasks)
        if a.save:
            pathlib.Path(a.save).write_text(json.dumps(results, indent=1, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n")
    elif a.results:
        results = json.loads(pathlib.Path(a.results).read_text(encoding="utf-8"))
    else:
        p.error("--runs or --results")
    totals = collections.Counter()
    by_attempt = collections.defaultdict(collections.Counter)
    by_match = collections.defaultdict(collections.Counter)
    seconds = 0.0
    for suite, data in tasks.items():
        row = collections.Counter()
        for task_id in data["user"]:
            r = results.get(f"{suite}/{task_id}")
            if r is None:
                row["without program"] += 1
                continue
            row["programs"] += 1
            seconds += r.get("seconds") or 0
            row["exit 0"] += r["exit"] == 0
            row["no call"] += r["calls"] == 0
            row["call errors"] += r["call_errors"] > 0
            ok = r.get("utility") is True
            row["utility"] += ok
            if r.get("attempt"):
                by_attempt[r["attempt"]]["programs"] += 1
                by_attempt[r["attempt"]]["utility"] += ok
            if "tools_called" in r:
                match = "exact" if set(r["tools_called"]) == set(r["ground_truth_tools"]) else "differs"
                by_match[match]["programs"] += 1
                by_match[match]["utility"] += ok
            if not ok:
                reason = r.get("judge_error") or ("exit %s" % r["exit"] if r["exit"] != 0 else "exit 0")
                print(f"  {suite}/{task_id}: utility {r.get('utility')}, {reason}, {r['calls']} calls ({r['call_errors']} errors), functions {r['functions']}")
        totals.update(row)
        print_row(suite, row)
    print_row("all", totals)
    for attempt in sorted(by_attempt):
        c = by_attempt[attempt]
        print(f"by attempt {attempt}: {c['utility']} of {c['programs']} ({100 * c['utility'] / c['programs']:.1f}%)")
    for match in ("exact", "differs"):
        c = by_match[match]
        if c["programs"]:
            print(f"call set {match} from the ground truth: {c['utility']} of {c['programs']} ({100 * c['utility'] / c['programs']:.1f}%)")
    print(f"{seconds:.0f} s of running")


def print_row(name, c):
    n = c.get("programs", 0)
    if not n:
        print(f"{name}: no programs")
        return
    print(f"{name}: {n} programs ({c.get('without program', 0)} tasks without), exit 0 {c.get('exit 0', 0)}, "
          f"no call {c.get('no call', 0)}, call errors {c.get('call errors', 0)}, "
          f"utility {c.get('utility', 0)} ({100 * c.get('utility', 0) / n:.1f}%)")


if __name__ == "__main__":
    main()
