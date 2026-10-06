"""Write the batch files the subagent graders read (decision U4): for every
label, the run's Explain samples in batches of fifty, each item carrying the
sample's key, the reference the grader judges against (the run's own
snapshot of `reference/<program>.explain.txt`, or the prompt's `purposes`
when there is none) and the explanation.

    python grade_batches.py --run 2026-10-06-3e7c45a \\
        --lane D:/Projects/.worktrees/Renyi/round3/grade \\
        --labels claude-sonnet-5-5,gpt-5.5

A subagent given `<label>.batch<n>.json` and the rubric of `EXPLAIN_GRADER`
verbatim writes `<label>.batch<n>.agent-<suffix>.json`, a JSON array of
`{"key", "grade", "reason"}` in the same order; `merge_agent_grades.py` then
puts the grades into the run's `grades.json`.
"""
import argparse
import json
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import run as harness  # noqa: E402  (imported after the path insert because the harness is a script, not a package)

BATCH = 50


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--run", required=True, help="run directory name under tests/readability")
    parser.add_argument("--lane", required=True, help="directory to write the batch files into")
    parser.add_argument("--labels", required=True, help="comma-separated model labels")
    args = parser.parse_args()
    run = harness.run_dir(args.run)
    lane = pathlib.Path(args.lane)
    lane.mkdir(parents=True, exist_ok=True)

    for label in args.labels.split(","):
        items = []
        for record_file in sorted((run / "outputs" / label / "explain").glob("*.json")):
            name = record_file.stem
            record = json.loads(harness.read(record_file))
            prompt = json.loads(harness.read(run / "prompts" / "explain" / f"{name}.json"))
            description = harness.reference_file(run, f"{name}.explain.txt")
            reference = harness.read(description) if description.exists() else prompt["purposes"]
            for index, sample in enumerate(record["samples"]):
                items.append({"key": f"explain/{name}.{index}", "reference": reference, "explanation": sample})
        for number, start in enumerate(range(0, len(items), BATCH), 1):
            batch = lane / f"{label}.batch{number}.json"
            batch.write_text(json.dumps({"items": items[start:start + BATCH]}, indent=1, ensure_ascii=False) + "\n",
                             encoding="utf-8", newline="\n")
        print(f"{label}: {len(items)} items in {(len(items) + BATCH - 1) // BATCH} batches under {lane}")


if __name__ == "__main__":
    main()
