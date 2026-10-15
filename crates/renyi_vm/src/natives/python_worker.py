"""The worker of the Python bridge (decision AL2), started by the Renyi VM as
`python -c <this text> <project root>`: one JSON object per line on its
standard input, one per line on its standard output. A request is
{"call": N, "module": "analysis", "function": "average", "arguments": [...]}
and the answer {"call": N, "result": ...} or {"call": N, "error": {"where":
"import" | "call" | "result", "kind": "ValueError", "message": "..."}}. The
first line the worker writes is {"ready": true, "version": "3.13.3"}. What
the Python functions print goes to the standard error, so that it cannot mix
with the protocol. At the boundary (decision AW49) an argument whose
parameter is annotated with a dataclass or a `NamedTuple` is built from its
JSON object, through the fields' own annotations, and such a value in the
result becomes a JSON object of its fields; a `TypedDict` is a dict both
ways."""

import collections.abc
import dataclasses
import importlib
import inspect
import json
import os
import sys
import types
import typing


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
        arguments = converted_in(function, request.get("arguments", []))
        result = convert_out(function(*arguments))
    except Exception as error:
        return {"error": failure("call", error)}
    try:
        json.dumps(result, ensure_ascii=False, allow_nan=False)
    except (TypeError, ValueError) as error:
        return {"error": failure("result", error)}
    return {"result": result}


# the hints of a function or a class, read once
HINTS = {}

UNIONS = tuple(
    union
    for union in (typing.Union, getattr(types, "UnionType", None))
    if union is not None
)
LISTS = (list, set, frozenset, collections.abc.Sequence, collections.abc.Set, collections.abc.Iterable)
MAPS = (dict, collections.abc.Mapping)


def hints_of(target):
    """The evaluated annotations of a function or a class, {} when they do
    not evaluate."""
    found = HINTS.get(target)
    if found is None:
        try:
            found = typing.get_type_hints(target)
        except Exception:
            found = dict(getattr(target, "__annotations__", {}))
        HINTS[target] = found
    return found


def converted_in(function, arguments):
    """The positional arguments, each built by its parameter's annotation."""
    try:
        parameters = list(inspect.signature(function).parameters.values())
    except (TypeError, ValueError):
        return arguments
    hints = hints_of(function)
    positional = [
        parameter.name
        for parameter in parameters
        if parameter.kind in (parameter.POSITIONAL_ONLY, parameter.POSITIONAL_OR_KEYWORD)
    ]
    return [
        convert_in(value, hints.get(positional[index])) if index < len(positional) else value
        for index, value in enumerate(arguments)
    ]


def is_namedtuple(cls):
    return isinstance(cls, type) and issubclass(cls, tuple) and hasattr(cls, "_fields")


def convert_in(value, hint):
    """A value from JSON as the annotation asks for it: a dict into a
    dataclass or a NamedTuple, through a list, a map, an Optional; the
    value itself where nothing applies."""
    if hint is None or value is None:
        return value
    origin = typing.get_origin(hint)
    arguments = typing.get_args(hint)
    if origin is typing.Annotated:
        return convert_in(value, arguments[0])
    if origin in UNIONS:
        members = [member for member in arguments if member is not type(None)]
        return convert_in(value, members[0]) if len(members) == 1 else value
    if origin in LISTS and isinstance(value, list) and len(arguments) == 1:
        return [convert_in(item, arguments[0]) for item in value]
    if origin in MAPS and isinstance(value, dict) and len(arguments) == 2:
        return {key: convert_in(item, arguments[1]) for key, item in value.items()}
    if isinstance(hint, type) and isinstance(value, dict):
        fields = hints_of(hint)
        if dataclasses.is_dataclass(hint) or is_namedtuple(hint):
            return hint(**{key: convert_in(item, fields.get(key)) for key, item in value.items()})
        if getattr(typing, "is_typeddict", lambda candidate: False)(hint):
            # a key the TypedDict does not require comes as null for a
            # `maybe` field that is nothing: the key is left out, as Python
            # reads such a dict
            required = getattr(hint, "__required_keys__", frozenset(fields))
            return {
                key: convert_in(item, fields.get(key))
                for key, item in value.items()
                if item is not None or key in required
            }
    return value


def convert_out(value):
    """A result as JSON carries it: a dataclass or a NamedTuple as the
    object of its fields, through lists, tuples and dicts."""
    if dataclasses.is_dataclass(value) and not isinstance(value, type):
        return {
            field.name: convert_out(getattr(value, field.name))
            for field in dataclasses.fields(value)
        }
    if isinstance(value, tuple) and hasattr(value, "_fields"):
        return {name: convert_out(getattr(value, name)) for name in value._fields}
    if isinstance(value, (list, tuple)):
        return [convert_out(item) for item in value]
    if isinstance(value, dict):
        return {key: convert_out(item) for key, item in value.items()}
    return value


def failure(where, error):
    return {"where": where, "kind": type(error).__name__, "message": str(error)}


def write(out, message):
    out.write((json.dumps(message, ensure_ascii=False, allow_nan=False) + "\n").encode("utf-8"))
    out.flush()


main()
