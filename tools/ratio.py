"""The ratio of decision AU35 (ii) as AU36 (iii) records it, a development aid.

The compiler written in Renyi on `compiler/bodies.ry` (checking, parsing and
compiling, each run from an image the stage's binary builds of
`compiler/checker.ry`, `compiler/parse.ry` and `compiler/compile.ry`) over
the Rust front end's time on the same file (`renyi check`, `renyi parse
--json`, `renyi compile`), against a reference binary (AU34's, the gate of
AU36 iii) and against the stage's own binary, every time the best of
`--runs` wall-clock runs with the image cache off.

usage: python tools/ratio.py <stage binary> <reference binary> [--runs N]
"""

import os
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENV = dict(os.environ, RENYI_NO_CACHE="1")


def best(command, runs):
    """The best wall-clock time of the command in milliseconds."""
    times = []
    for _ in range(runs):
        started = time.perf_counter()
        subprocess.run(command, cwd=ROOT, capture_output=True, env=ENV)
        times.append((time.perf_counter() - started) * 1000)
    return min(times)


def main():
    args = sys.argv[1:]
    runs = 9
    if "--runs" in args:
        at = args.index("--runs")
        runs = int(args[at + 1])
        del args[at : at + 2]
    if len(args) != 2:
        raise SystemExit(__doc__)
    stage, reference = (os.path.abspath(arg) for arg in args)
    with tempfile.TemporaryDirectory() as work:
        images = {}
        for name in ["checker", "parse", "compile"]:
            image = os.path.join(work, f"{name}.ryi")
            built = subprocess.run(
                [stage, "build", "--to", image, f"compiler/{name}.ry"],
                cwd=ROOT,
                capture_output=True,
                env=ENV,
            )
            if built.returncode != 0:
                raise SystemExit(built.stderr.decode(errors="replace"))
            images[name] = image
        out_rust = os.path.join(work, "rust.ryc")
        out_renyi = os.path.join(work, "renyi.ryc")
        rows = [
            ("check", ["check", "compiler/bodies.ry"], ["run", images["checker"], "compiler/bodies.ry"]),
            ("parse", ["parse", "--json", "compiler/bodies.ry"], ["run", images["parse"], "compiler/bodies.ry"]),
            (
                "compile",
                ["compile", "--to", out_rust, "compiler/bodies.ry"],
                ["run", images["compile"], "--to", out_renyi, "compiler/bodies.ry"],
            ),
        ]
        print(f'{"":8} {"Rust, reference":>16} {"Rust, stage":>12} {"Renyi, stage":>13} {"gate":>7} {"same binary":>12}')
        for name, rust, renyi in rows:
            gate = best([reference] + rust, runs)
            own = best([stage] + rust, runs)
            ours = best([stage] + renyi, runs)
            print(f"{name:8} {gate:>14.1f}ms {own:>10.1f}ms {ours:>11.1f}ms {ours / gate:>6.1f}x {ours / own:>11.1f}x")


if __name__ == "__main__":
    main()
