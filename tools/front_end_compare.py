"""Two binaries on the front end's commands, a development aid of the front
end's round (decision AU35).

For each command, the instructions under callgrind (when valgrind is on the
path) and the best of `--runs` wall-clock runs, the two binaries
alternating, with the image cache off: an empty program's check, `check`
and `run` of `examples/hello.ry`, `check` of `compiler/bodies.ry` and of
`compiler/checker.ry`, `compile` of `compiler/checker.ry`, and every example
checked one at a time, summed.

usage: python tools/front_end_compare.py <binary A> <binary B> [--runs N]
"""

import glob
import os
import shutil
import subprocess
import sys
import tempfile
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENV = dict(os.environ, RENYI_NO_CACHE="1")
EMPTY = """module empty
  purpose: Nothing at all.

public function main()
  purpose: Do nothing.

  return
end
"""


def instructions(binary, args):
    """The instructions the command runs, or 0 without valgrind."""
    if shutil.which("valgrind") is None:
        return 0
    completed = subprocess.run(
        ["valgrind", "--tool=callgrind", "--callgrind-out-file=/dev/null", binary] + args,
        cwd=ROOT,
        capture_output=True,
        text=True,
        env=ENV,
    )
    for line in completed.stderr.splitlines():
        if "refs:" in line:
            return int(line.split()[-1].replace(",", ""))
    return 0


def wall(binary, args):
    started = time.perf_counter()
    subprocess.run([binary] + args, cwd=ROOT, capture_output=True, env=ENV)
    return (time.perf_counter() - started) * 1000


def change(a, b):
    return 100 * (b - a) / a if a else 0.0


def main():
    args = sys.argv[1:]
    runs = 9
    if "--runs" in args:
        at = args.index("--runs")
        runs = int(args[at + 1])
        del args[at : at + 2]
    if len(args) != 2:
        raise SystemExit(__doc__)
    first, second = (os.path.abspath(arg) for arg in args)
    with tempfile.TemporaryDirectory() as work:
        empty = os.path.join(work, "empty.ry")
        with open(empty, "w") as out:
            out.write(EMPTY)
        rows = [
            ("check empty", ["check", empty]),
            ("check hello", ["check", "examples/hello.ry"]),
            ("run hello", ["run", "examples/hello.ry"]),
            ("check bodies.ry", ["check", "compiler/bodies.ry"]),
            ("check checker.ry", ["check", "compiler/checker.ry"]),
            ("compile checker.ry", ["compile", "--to", os.path.join(work, "c.ryc"), "compiler/checker.ry"]),
        ]
        print(f'{"":20} {"instructions A":>15} {"B":>15} {"":>7}   {"ms A":>8} {"B":>8} {"":>7}')
        for name, command in rows:
            ia, ib = instructions(first, command), instructions(second, command)
            ta, tb = [], []
            for _ in range(runs):
                ta.append(wall(first, command))
                tb.append(wall(second, command))
            a, b = min(ta), min(tb)
            print(f"{name:20} {ia:>15,} {ib:>15,} {change(ia, ib):>+6.1f}%   {a:>8.1f} {b:>8.1f} {change(a, b):>+6.1f}%")
        examples = sorted(glob.glob(os.path.join(ROOT, "examples", "*.ry")))
        sa = sb = 0.0
        for example in examples:
            xa, xb = [], []
            for _ in range(3):
                xa.append(wall(first, ["check", example]))
                xb.append(wall(second, ["check", example]))
            sa += min(xa)
            sb += min(xb)
        print(f'{"check each example":20} {"":>15} {"":>15} {"":>7}   {sa:>8.1f} {sb:>8.1f} {change(sa, sb):>+6.1f}%  ({len(examples)} files, summed)')


if __name__ == "__main__":
    main()
