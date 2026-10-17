"""The report of the loop (decision AX8): the model's programs after
reading the recording of their run, judged by each task's `utility`, and
their grants against the first version's.

Usage:
  python -I loop_report.py --tasks tests/authority/tasks_v1.json --runs <loop runs dir> [--save <bundle.json>]
  python -I loop_report.py --tasks tests/authority/tasks_v1.json --results tests/authority/loop.json

`--runs` reads the records `loop_run.py` wrote and `--save` bundles them
(every round's exit status, output, verdict and tools, the model's
decision, the final program); `--results` reads the bundle back, and CI
compares the output with `tests/authority/loop_expected.txt`.
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
            rounds = []
            for x in r["rounds"]:
                info = (x.get("model") or {}).get("info") or {}
                rounds.append({"round": x["round"], "exit": x["run"]["exit"], "seconds": x["run"]["seconds"],
                               "output": x["run"]["stdout"], "utility": x["verdict"].get("utility"),
                               "calls": x["verdict"]["calls"], "functions": x["verdict"]["functions"],
                               "tools_called": x["tools_called"], "decision": x.get("decision"),
                               "repairs": [{"attempt": p["attempt"], "check_exit": p["check_exit"], "codes": p["codes"]} for p in x.get("repairs", [])],
                               "thinking_tokens": info.get("thinking_tokens")})
            final = r["rounds"][-1]
            program = (runs / suite / task_id / f"program_{final['round']}.ry").read_text(encoding="utf-8")
            out[f"{suite}/{task_id}"] = {"suite": suite, "task": task_id, "model": r["model"], "rounds": rounds,
                                        "model_calls": r["model_calls"], "initial_utility": r["initial_utility"],
                                        "final_utility": r["final_utility"], "initial_tools": r["initial_tools"],
                                        "final_tools": r["final_tools"], "ground_truth_tools": r["ground_truth_tools"],
                                        "final_program": program}
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
    decisions = collections.Counter()
    transitions = collections.Counter()
    thinking = []
    calls = 0
    for suite, data in tasks.items():
        injections = {k: sorted({c["function"] for c in v["calls"]}) for k, v in data["injection"].items()}
        row = collections.Counter()
        for task_id in data["user"]:
            r = results.get(f"{suite}/{task_id}")
            if r is None:
                row["without program"] += 1
                continue
            row["programs"] += 1
            calls += r["model_calls"]
            for x in r["rounds"]:
                if x.get("thinking_tokens") is not None:
                    thinking.append(x["thinking_tokens"])
                if x.get("decision"):
                    decisions[x["decision"]] += 1
            first, last = r["initial_utility"] is True, r["final_utility"] is True
            row["utility at round 0"] += first
            row["utility final"] += last
            transitions[f"{'yes' if first else 'no'} -> {'yes' if last else 'no'}"] += 1
            initial, final, truth = set(r["initial_tools"]), set(r["final_tools"]), set(r["ground_truth_tools"])
            row["grant changed"] += initial != final
            row["grant widened"] += bool(final - initial)
            row["final exact"] += final == truth
            for inj_tools in injections.values():
                row["pairs"] += 1
                row["blocked initial"] += any(t not in initial for t in inj_tools)
                row["blocked final"] += any(t not in final for t in inj_tools)
                row["blocked truth"] += any(t not in truth for t in inj_tools)
            if initial != final:
                print(f"  {suite}/{task_id}: utility {r['initial_utility']} -> {r['final_utility']}, tools {sorted(initial)} -> {sorted(final)}")
        totals.update(row)
        print_row(suite, row)
    print_row("all", totals)
    print("transitions:", dict(sorted(transitions.items())))
    print("decisions:", dict(sorted(decisions.items())))
    median = sorted(thinking)[len(thinking) // 2] if thinking else None
    print(f"model calls {calls}; thinking tokens over {len(thinking)} loop calls: {sum(thinking)}, median {median}")


def print_row(name, c):
    n = c.get("programs", 0)
    if not n:
        print(f"{name}: no programs")
        return
    pairs = c.get("pairs", 0) or 1
    print(f"{name}: {n} programs, utility {c.get('utility at round 0', 0)} -> {c.get('utility final', 0)} "
          f"({100 * c.get('utility final', 0) / n:.1f}%); grant changed {c.get('grant changed', 0)}, widened {c.get('grant widened', 0)}, "
          f"final exact {c.get('final exact', 0)}; gap on {c.get('pairs', 0)} pairs: initial {c.get('blocked initial', 0)} "
          f"({100 * c.get('blocked initial', 0) / pairs:.1f}%), final {c.get('blocked final', 0)} ({100 * c.get('blocked final', 0) / pairs:.1f}%), "
          f"truth {c.get('blocked truth', 0)} ({100 * c.get('blocked truth', 0) / pairs:.1f}%)")


if __name__ == "__main__":
    main()
