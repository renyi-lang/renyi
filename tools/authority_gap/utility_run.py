"""Utility of the model's programs: every clean program of the model-in-the-
loop run (`runs/b`) run with `renyi run` against AgentDojo's own
environment through the Python bridge, and the task's `utility` asked of
the environment after the run and the program's output.

Run it with the lane's venv (AgentDojo installed), which judges in process:
  venv/Scripts/python.exe utility_run.py --tasks data/tasks_v1.json --runs runs/b
      --tools <tests/authority/tools> --pkg scripts/utility_pkg/tools --out runs/utility
      --renyi <renyi binary> [--suites banking,slack] [--limit N] [--injection <id>]

For each suite a project is written under <out>/<suite>/project/: the
suite's declarations and manifest copied from <tools>, and the package
`tools/` (the `__init__.py` of <pkg> and a shim per tool). For each task
the final program is written there as `task.ry` and run with
`RENYI_PYTHON` naming this interpreter, `RENYI_AGENTDOJO_SUITE` the suite
and `RENYI_AGENTDOJO_POST` the file the package writes the environment to
after every call; the judge loads the suite's default environment as the
pre state, the file as the post state and asks the task's `utility`
(and, with `--injection`, the injection task's `security`).
"""
import argparse
import importlib
import json
import os
import pathlib
import pickle
import shutil
import subprocess
import sys
import time

from agentdojo.task_suite.load_suites import get_suite

SHIM = '"""The tool `{name}` over AgentDojo\'s environment; see the package."""\nfrom tools import call\n\n\ndef {name}(*args):\n    return call("{name}", args)\n'


def project_of(out, suite, tools_dir, pkg):
    project = out / suite / "project"
    if project.exists():
        return project
    project.mkdir(parents=True)
    for f in (tools_dir / suite).iterdir():
        if f.suffix == ".ry" or f.name == "renyi.json":
            shutil.copy(f, project / f.name)
    (project / "tools").mkdir()
    shutil.copy(pkg / "__init__.py", project / "tools" / "__init__.py")
    manifest = json.loads((tools_dir / suite / "renyi.json").read_text(encoding="utf-8"))
    for name in manifest["python"]["modules"]:
        (project / "tools" / f"{name}.py").write_text(SHIM.format(name=name), encoding="utf-8", newline="\n")
    return project


def run_program(renyi, project, program, env_extra, timeout):
    (project / "task.ry").write_text(program, encoding="utf-8", newline="\n")
    env = dict(os.environ, RENYI_NO_CACHE="1", **env_extra)
    started = time.time()
    try:
        r = subprocess.run([str(renyi), "run", "task.ry"], cwd=project, env=env, capture_output=True,
                           text=True, encoding="utf-8", errors="replace", timeout=timeout)
        return {"exit": r.returncode, "stdout": r.stdout, "stderr": r.stderr[-3000:], "seconds": round(time.time() - started, 1)}
    except subprocess.TimeoutExpired as error:
        return {"exit": None, "stdout": (error.stdout or b"").decode("utf-8", "replace") if isinstance(error.stdout, bytes) else (error.stdout or ""),
                "stderr": "timed out", "seconds": timeout}


def judge(suite, task, injection, post_file, output):
    pre = suite.load_and_inject_default_environment({})
    if post_file.exists():
        with open(post_file, "rb") as f:
            data = pickle.load(f)  # written by the run's own worker (tools/__init__.py)
        post = data["environment"]
        calls = data["calls"]
    else:
        post = suite.load_and_inject_default_environment({})
        calls = []
    verdict = {"calls": len(calls), "call_errors": sum(1 for c in calls if c.get("error")),
               "functions": sorted({c["function"] for c in calls})}
    try:
        verdict["utility"] = bool(task.utility(output, pre, post))
    except Exception as error:  # the judge's own failure is recorded as the verdict, not raised
        verdict["utility"] = None
        verdict["judge_error"] = f"{type(error).__name__}: {error}"
    if injection is not None:
        try:
            verdict["security"] = bool(injection.security(output, pre, post))
        except Exception as error:  # as above
            verdict["security"] = None
            verdict["security_error"] = f"{type(error).__name__}: {error}"
    return verdict


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--tasks", required=True)
    p.add_argument("--runs", required=True)
    p.add_argument("--tools", required=True)
    p.add_argument("--pkg", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--renyi", required=True)
    p.add_argument("--suites", default="banking,slack,travel,workspace")
    p.add_argument("--limit", type=int, default=0)
    p.add_argument("--timeout", type=int, default=300)
    a = p.parse_args()
    tasks = json.loads(pathlib.Path(a.tasks).read_text(encoding="utf-8"))
    runs, tools_dir, pkg = pathlib.Path(a.runs), pathlib.Path(a.tools).resolve(), pathlib.Path(a.pkg).resolve()
    out, renyi = pathlib.Path(a.out).resolve(), pathlib.Path(a.renyi).resolve()
    done = 0
    totals = {}
    for suite_name in a.suites.split(","):
        suite = get_suite("v1", suite_name)
        project = project_of(out, suite_name, tools_dir, pkg)
        counts = {"programs": 0, "utility": 0, "exit0": 0, "missing": 0}
        for task_id in tasks[suite_name]["user"]:
            record_file = runs / suite_name / task_id / "result.json"
            if not record_file.exists():
                counts["missing"] += 1
                continue
            record = json.loads(record_file.read_text(encoding="utf-8"))
            if not record["final"]:
                counts["missing"] += 1
                continue
            task_dir = out / suite_name / task_id
            result_file = task_dir / "utility.json"
            if result_file.exists():
                result = json.loads(result_file.read_text(encoding="utf-8"))
            else:
                task_dir.mkdir(parents=True, exist_ok=True)
                program = (runs / suite_name / task_id / f"attempt_{record['final']['attempt']}.ry").read_text(encoding="utf-8")
                post_file = task_dir / "post.pkl"
                if post_file.exists():
                    post_file.unlink()
                env_extra = {"RENYI_PYTHON": sys.executable, "RENYI_AGENTDOJO_SUITE": suite_name,
                             "RENYI_AGENTDOJO_POST": str(post_file)}
                run = run_program(renyi, project, program, env_extra, a.timeout)
                number = task_id.replace("UserTask", "user_task_")
                task = suite.user_tasks[number]
                verdict = judge(suite, task, None, post_file, run["stdout"])
                result = {"suite": suite_name, "task": task_id, "run": run, "verdict": verdict}
                result_file.write_text(json.dumps(result, indent=1, ensure_ascii=False), encoding="utf-8")
                done += 1
            counts["programs"] += 1
            counts["utility"] += result["verdict"].get("utility") is True
            counts["exit0"] += result["run"]["exit"] == 0
            v = result["verdict"]
            print(f"{suite_name}/{task_id}: exit {result['run']['exit']}, {v['calls']} calls ({v['call_errors']} errors), "
                  f"utility {v.get('utility')}{' ' + v['judge_error'] if v.get('judge_error') else ''}", flush=True)
            if a.limit and done >= a.limit:
                break
        totals[suite_name] = counts
        print(f"{suite_name}: {counts}", flush=True)
        if a.limit and done >= a.limit:
            break
    print("all:", {k: sum(c[k] for c in totals.values()) for k in ("programs", "utility", "exit0", "missing")})


if __name__ == "__main__":
    main()
