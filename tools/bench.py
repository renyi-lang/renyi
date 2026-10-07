"""The benchmarks of `bench/` (decision AG4), timed against the binary given.

Each program is run with the binary as it is (machine code for the hot
code objects, decision AG1), again with `--interpret`, and, where a Python
twin lies next to it, with CPython; the self-check of the compiler written
in Renyi and the start of `examples/hello.ry` complete the set. Every time
is the best of `--runs` wall-clock runs of the whole process. The table is
printed; the exit status is 0 whatever the numbers say, since CI prints
them and does not gate on them.

usage: python tools/bench.py [<renyi binary>] [--runs N] [--python <exe>]
"""

import os
import subprocess
import sys
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# name, the renyi command line after `run`, the Python twin (or None)
BENCHMARKS = [
    ("primes", ["bench/primes.ry"], "bench/primes.py"),
    ("strings", ["bench/strings.ry"], "bench/strings.py"),
    ("records", ["bench/records.ry"], "bench/records.py"),
    ("json_round_trip", ["bench/json_round_trip.ry"], "bench/json_round_trip.py"),
    ("checker on bodies.ry", ["compiler/checker.ry", "compiler/bodies.ry"], None),
    ("hello", ["examples/hello.ry", "Renyi"], None),
]


def best_of(command, runs):
    """The best wall-clock time of the command in seconds, and its output."""
    best = None
    output = b""
    for _ in range(runs):
        started = time.perf_counter()
        completed = subprocess.run(command, cwd=ROOT, capture_output=True)
        elapsed = time.perf_counter() - started
        if completed.returncode != 0:
            sys.stderr.write(completed.stdout.decode(errors="replace"))
            sys.stderr.write(completed.stderr.decode(errors="replace"))
            raise SystemExit(f"failed: {' '.join(command)}")
        output = completed.stdout
        best = elapsed if best is None else min(best, elapsed)
    return best, output


def main():
    args = sys.argv[1:]
    runs = 3
    python = sys.executable
    binary = None
    while args:
        arg = args.pop(0)
        if arg == "--runs":
            runs = int(args.pop(0))
        elif arg == "--python":
            python = args.pop(0)
        else:
            binary = arg
    if binary is None:
        binary = os.path.join("target", "release", "renyi")
    if not os.path.isabs(binary):
        binary = os.path.join(ROOT, binary)
    if os.name == "nt" and not os.path.exists(binary) and os.path.exists(binary + ".exe"):
        binary += ".exe"
    print(f"{'benchmark':22} {'renyi':>9} {'--interpret':>12} {'cpython':>9}   ratio")
    for name, arguments, twin in BENCHMARKS:
        native, output = best_of([binary, "run", *arguments], runs)
        interpreted, _ = best_of([binary, "run", "--interpret", *arguments], runs)
        line = f"{name:22} {native * 1000:7.0f}ms {interpreted * 1000:10.0f}ms"
        if twin:
            cpython, twin_output = best_of([python, os.path.join(ROOT, twin)], runs)
            same = twin_output.split() == output.split()
            line += f" {cpython * 1000:7.0f}ms   {cpython / native:4.1f}x"
            if not same:
                line += "   (the outputs differ!)"
        print(line)


if __name__ == "__main__":
    main()
