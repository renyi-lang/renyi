"""Merge the subagent graders' answers into a run's grades.json, under the
buckets `score` reads for `--grader agent:claude-sonnet-5-5
--second-grader agent:claude-opus-5-5` (decision U4).

A batch file `<label>.batch<n>.json` holds `{"items": [{"key", "reference",
"explanation"}, ...]}`, fifty samples of one model (`key` is
`explain/<program>.<index>`); a subagent given the rubric of
`EXPLAIN_GRADER` verbatim writes `<label>.batch<n>.agent-<suffix>.json`, a
JSON array of `{"key", "grade", "reason"}` in the same order. Every item is
checked against the run's own sample and reference before its digest is
computed, so that the cache entries match what `grade_of` would compute
and no API call is ever attempted. An item whose reference text is no
longer the run's reference (the file was corrected after the batch was
graded) is skipped: a later batch holds its fresh grade, and the harness
would otherwise find no cache entry and stop.

    python merge_agent_grades.py --run 2026-10-05-1623155 \
        --lane D:/Projects/.worktrees/Renyi/regrade-r1/grade \
        --labels claude-sonnet-5-5,claude-haiku-4-5-20251001,gpt-5.4-mini
"""
import argparse
import hashlib
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import run as harness  # noqa: E402  (imported after the path insert because the harness is a script, not a package)

GRADERS = {"sonnet": "agent:claude-sonnet-5-5", "opus": "agent:claude-opus-5-5"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--run", required=True, help="run directory name under tests/readability")
    parser.add_argument("--lane", required=True, help="directory holding the batch files and the subagents' answers")
    parser.add_argument("--labels", required=True, help="comma-separated model labels")
    args = parser.parse_args()
    lane = pathlib.Path(args.lane)
    run = harness.run_dir(args.run)

    missing, merged, stale = [], 0, []
    for label in args.labels.split(","):
        grades_file = run / "outputs" / label / "grades.json"
        grades = json.loads(harness.read(grades_file)) if grades_file.exists() else {}
        for source in sorted(lane.glob(f"{label}.batch*.json")):
            if ".agent-" in source.name:
                continue
            items = json.loads(source.read_text(encoding="utf-8"))["items"]
            for suffix, grader in GRADERS.items():
                answer_file = lane / f"{source.stem}.agent-{suffix}.json"
                if not answer_file.exists():
                    missing.append(answer_file.name)
                    continue
                answers = json.loads(answer_file.read_text(encoding="utf-8"))
                if [a["key"] for a in answers] != [i["key"] for i in items]:
                    sys.exit(f"{answer_file.name}: keys differ from the batch")
                bucket = grades.setdefault(f"{grader}#{harness.RUBRIC_ID}", {})
                for item, answer in zip(items, answers):
                    key = item["key"]
                    task, rest = key.split("/", 1)
                    name, index = rest.rsplit(".", 1)
                    record = json.loads(harness.read(run / "outputs" / label / task / f"{name}.json"))
                    sample = record["samples"][int(index)]
                    prompt = json.loads(harness.read(run / "prompts" / task / f"{name}.json"))
                    description = harness.reference_file(run, f"{name}.explain.txt")
                    reference = harness.read(description) if description.exists() else prompt["purposes"]
                    if item["explanation"] != sample:
                        sys.exit(f"{answer_file.name}: {key} does not match the run's sample")
                    if item["reference"] != reference:
                        stale.append(f"{answer_file.name}:{key}")
                        continue
                    grade = int(answer["grade"])
                    if not 1 <= grade <= 5:
                        sys.exit(f"{answer_file.name}: {key} has grade {grade}")
                    digest = hashlib.sha256((reference + "\n" + sample).encode("utf-8")).hexdigest()[:16]
                    bucket[key] = {"grade": grade, "sample": digest,
                                   "answer": str(answer.get("reason", "")).strip()}
                    merged += 1
        harness.write(grades_file, json.dumps(grades, indent=2, sort_keys=True))
        print(label, {b: len(v) for b, v in grades.items()})
    print(f"merged {merged} grades; stale (skipped) {len(stale)}; missing files: {missing or 'none'}")


if __name__ == "__main__":
    main()
