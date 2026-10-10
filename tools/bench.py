"""The benchmarks of `bench/` (decision AG4), timed against the binary given.

Each program is run with the binary as it is (machine code for the hot
code objects, decision AG1), again with `--interpret`, from its image
(`renyi build`, decision AS1: every code object as machine code, nothing
compiled at run time) and, where a Python twin lies next to it, with
CPython; the self-check of the compiler written in Renyi and the start of
`examples/hello.ry` complete the set. Every time is the best of `--runs`
wall-clock runs of the whole process. The table is printed; the exit
status is 0 whatever the numbers say, since CI prints them and does not
gate on them.

usage: python tools/bench.py [<renyi binary>] [--runs N] [--python <exe>]
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# name, the renyi command line after `run`, the Python twin (or None)
BENCHMARKS = [
    ("primes", ["bench/primes.ry"], "bench/primes.py"),
    ("strings", ["bench/strings.ry"], "bench/strings.py"),
    ("records", ["bench/records.ry"], "bench/records.py"),
    ("json_round_trip", ["bench/json_round_trip.ry"], "bench/json_round_trip.py"),
    # the self-check: the compiler written in Renyi as it stood at decision
    # AU39, frozen under bench/selfcheck/ (decision AU36 iv), checking itself
    ("checker on bodies.ry", ["bench/selfcheck/checker.ry", "bench/selfcheck/bodies.ry"], None),
    ("hello", ["examples/hello.ry", "Renyi"], None),
]


# the cold runs: nothing from the caches of `renyi run` (decisions AU10 and
# AU43): the front end runs and the hot code objects are compiled
COLD = dict(os.environ, RENYI_NO_CACHE="1")


def best_of(command, runs):
    """The best wall-clock time of the command in seconds, and its output."""
    best = None
    output = b""
    for _ in range(runs):
        started = time.perf_counter()
        completed = subprocess.run(command, cwd=ROOT, capture_output=True, env=COLD)
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
    images = tempfile.mkdtemp(prefix="renyi-bench-")
    print(f"{'benchmark':22} {'renyi':>9} {'--interpret':>12} {'image':>9} {'cpython':>9}   ratio")
    try:
        for name, arguments, twin in BENCHMARKS:
            native, output = best_of([binary, "run", *arguments], runs)
            interpreted, _ = best_of([binary, "run", "--interpret", *arguments], runs)
            line = f"{name:22} {native * 1000:7.0f}ms {interpreted * 1000:10.0f}ms"
            image = os.path.join(images, name.replace(" ", "_") + ".ryi")
            built = subprocess.run(
                [binary, "build", "--to", image, arguments[0]], cwd=ROOT, capture_output=True
            )
            if built.returncode == 0:
                from_image, _ = best_of([binary, "run", image, *arguments[1:]], runs)
                line += f" {from_image * 1000:7.0f}ms"
            else:
                # a machine that generates no code builds no image
                line += f" {'-':>9}"
            if twin:
                cpython, twin_output = best_of([python, os.path.join(ROOT, twin)], runs)
                same = twin_output.split() == output.split()
                line += f" {cpython * 1000:7.0f}ms   {cpython / native:4.1f}x"
                if not same:
                    line += "   (the outputs differ!)"
            print(line)
    finally:
        shutil.rmtree(images, ignore_errors=True)


if __name__ == "__main__":
    main()
