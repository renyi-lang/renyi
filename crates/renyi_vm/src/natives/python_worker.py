"""The worker of the Python bridge (decision AL2), started by the Renyi VM as
`python -c <this text> <project root>`: one JSON object per line on its
standard input, one per line on its standard output. A request is
{"call": N, "module": "analysis", "function": "average", "arguments": [...]}
and the answer {"call": N, "result": ...} or {"call": N, "error": {"where":
"import" | "call" | "result", "kind": "ValueError", "message": "..."}}. The
first line the worker writes is {"ready": true, "version": "3.13.3"}. What
the Python functions print goes to the standard error, so that it cannot mix
with the protocol."""

import importlib
import json
import os
import sys


def main():
    out = sys.stdout.buffer
    sys.stdout = sys.stderr
    root = sys.argv[1] if len(sys.argv) > 1 else ""
    sys.path.insert(0, root or os.getcwd())
    modules = {}
    write(out, {"ready": True, "version": "%d.%d.%d" % sys.version_info[:3]})
    for line in sys.stdin.buffer:
        try:
            request = json.loads(line.decode("utf-8"))
            call = request["call"]
        except (ValueError, KeyError, TypeError) as error:
            write(out, {"call": None, "error": failure("request", error)})
            continue
        answer = reply(modules, request)
        answer["call"] = call
        write(out, answer)


def reply(modules, request):
    """The result of the call, or the error and where it happened."""
    name = request.get("module")
    try:
        module = modules.get(name)
        if module is None:
            module = importlib.import_module(name)
            modules[name] = module
        function = getattr(module, request.get("function"))
    except Exception as error:
        return {"error": failure("import", error)}
    try:
        result = function(*request.get("arguments", []))
    except Exception as error:
        return {"error": failure("call", error)}
    try:
        json.dumps(result, ensure_ascii=False, allow_nan=False)
    except (TypeError, ValueError) as error:
        return {"error": failure("result", error)}
    return {"result": result}


def failure(where, error):
    return {"where": where, "kind": type(error).__name__, "message": str(error)}


def write(out, message):
    out.write((json.dumps(message, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8"))
    out.flush()


main()
