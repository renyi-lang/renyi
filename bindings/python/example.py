"""A Python host of a Renyi module (decision AW46, stage 40b): the twin of
crates/renyi_c/examples/host.c. Loads the module given on the command
line (calc.ry of the C example by default) with a grant, prints its
public functions, calls four of them and shows a refusal and a failure.

    python bindings/python/example.py crates/renyi_c/examples/calc.ry

The output is example.expected beside this file, which CI compares.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import renyi  # noqa: E402  (the import follows the path insert above, by design)


def show(calc, name, *arguments):
    sys.stdout.flush()  # what the module prints to the console comes in order
    try:
        value = calc.call(name, *arguments)
    except renyi.RenyiError as error:
        print(f"{name}{json.dumps(list(arguments))} -> {type(error).__name__}: {error}")
    else:
        print(f"{name}{json.dumps(list(arguments))} -> {json.dumps(value)}")


def main():
    path = sys.argv[1] if len(sys.argv) > 1 else "calc.ry"
    grant = {"grant": "console", "memory": "16 megabytes"}
    with renyi.Sandbox.load(path, grant) as calc:
        print(f"{renyi.version()}, API {renyi.api_version()}")
        print(f"module: {calc.module}")
        print(f"grant: {calc.grant}")
        for function in calc.functions():
            print(f"  {function['signature']}")
        show(calc, "add", 2, 3)
        show(calc, "shout", "hello")
        show(calc, "shift", {"across": 1, "down": 2}, 10)
        show(calc, "add", "two", 3)
        show(calc, "risky", -1)


if __name__ == "__main__":
    main()
