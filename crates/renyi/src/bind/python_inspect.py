"""The inspection script of `renyi bind --python` (decision AM1).

Run as `python -c <this script> <package> <directory>`: the directory is
put first on the module path, the package is imported, and one JSON
object is written on the standard output: the interpreter's version, the
package's file and its functions, each with its parameters (name, kind,
whether it has a default, its annotation as a tree), its return
annotation, the first line of its docstring and its signature as Python
prints it, and the classes the annotations name that state their fields
(a dataclass, a `NamedTuple`, a `TypedDict`; decision AW49), each with
its kind, its bare name, whether it is generic, its docstring's first line
and its fields (name, annotation tree, whether required, whether the
constructor takes it). A function whose signature Python cannot give is
reported as skipped. An import that fails is `{"error": {"kind": ...,
"message": ...}}`. What the package prints while importing goes to the
standard error, so that it cannot mix with the report.
"""

import dataclasses
import importlib
import inspect
import json
import platform
import sys
import types
import typing

KINDS = {
    inspect.Parameter.POSITIONAL_ONLY: "positional",
    inspect.Parameter.POSITIONAL_OR_KEYWORD: "positional",
    inspect.Parameter.KEYWORD_ONLY: "keyword",
    inspect.Parameter.VAR_POSITIONAL: "var_positional",
    inspect.Parameter.VAR_KEYWORD: "var_keyword",
}

# `Optional[T]` and `Union[...]` have the origin `typing.Union`; `T | None`
# has `types.UnionType` (the same object as `typing.Union` since 3.14)
UNIONS = tuple(
    union
    for union in (typing.Union, getattr(types, "UnionType", None))
    if union is not None
)


# the classes met in the annotations whose fields Python states, by
# qualified name, in the order they were met (decision AW49)
CLASSES = {}


def main():
    package, directory = sys.argv[1], sys.argv[2]
    out = sys.stdout
    sys.stdout = sys.stderr
    sys.path.insert(0, directory)
    try:
        module = importlib.import_module(package)
        report = {
            "version": platform.python_version(),
            "file": getattr(module, "__file__", None),
            "functions": [
                function_of(name, function) for name, function in functions_of(module)
            ],
            "classes": list(CLASSES.values()),
        }
    except BaseException as error:
        # an import may raise anything, `SystemExit` included; every one of
        # them is the binder's answer, not a crash of the script
        report = {"error": {"kind": type(error).__name__, "message": str(error)}}
    out.write(json.dumps(report, ensure_ascii=False) + "\n")
    out.flush()


def functions_of(module):
    """The functions the file declares: the names of `__all__` that are
    functions, else the public functions defined in the module itself, in
    definition order."""
    listed = getattr(module, "__all__", None)
    if listed is None:
        return [
            (name, member)
            for name, member in list(vars(module).items())
            if not name.startswith("_")
            and inspect.isroutine(member)
            and getattr(member, "__module__", None) == module.__name__
        ]
    return [
        (name, getattr(module, name))
        for name in listed
        if inspect.isroutine(getattr(module, name, None))
    ]


def function_of(name, function):
    """One function of the report: its parameters, its result, its
    docstring's first line and its signature; or why it is skipped."""
    try:
        signature = inspect.signature(function, eval_str=True)
    except Exception:
        # the annotations do not evaluate (a forward reference to nothing,
        # say): the signature with them as text does, because the binder
        # reads such a text as a type it does not know
        try:
            signature = inspect.signature(function)
        except (TypeError, ValueError) as error:
            return {"name": name, "skipped": "no signature: " + str(error)}
    parameters = [
        {
            "name": parameter.name,
            "kind": KINDS[parameter.kind],
            "default": parameter.default is not parameter.empty,
            "annotation": (
                None if parameter.annotation is parameter.empty else tree(parameter.annotation)
            ),
        }
        for parameter in signature.parameters.values()
    ]
    returns = (
        None
        if signature.return_annotation is signature.empty
        else tree(signature.return_annotation)
    )
    doc = inspect.getdoc(function)
    first = None
    if doc:
        first = next((line.strip() for line in doc.splitlines() if line.strip()), None)
    return {
        "name": name,
        "signature": name + str(signature),
        "parameters": parameters,
        "returns": returns,
        "doc": first,
    }


def tree(annotation):
    """An annotation as the binder reads it: a class by its qualified name,
    a generic with its arguments, a union with its members, `None`, or the
    text of anything else."""
    if annotation is None or annotation is type(None):
        return {"none": True}
    if isinstance(annotation, str):
        return {"other": annotation}
    origin = typing.get_origin(annotation)
    if origin is not None and any(origin is union for union in UNIONS):
        return {"union": [tree(member) for member in typing.get_args(annotation)]}
    if origin is not None:
        return {
            "generic": qualified(origin),
            "args": [tree(argument) for argument in typing.get_args(annotation)],
        }
    if isinstance(annotation, type):
        note_class(annotation)
        return {"class": qualified(annotation)}
    return {"other": repr(annotation)}


def kind_of(cls):
    """`dataclass`, `namedtuple` or `typeddict` for a class that states its
    fields; None for any other class."""
    if dataclasses.is_dataclass(cls):
        return "dataclass"
    if issubclass(cls, tuple) and hasattr(cls, "_fields") and hasattr(cls, "__annotations__"):
        return "namedtuple"
    if getattr(typing, "is_typeddict", lambda candidate: False)(cls):
        return "typeddict"
    return None


def note_class(cls):
    """Add the class to the report's classes when its fields are stated:
    its fields' annotations may name further classes, noted in turn."""
    kind = kind_of(cls)
    name = qualified(cls)
    if kind is None or name in CLASSES:
        return
    entry = {
        "name": name,
        "short": cls.__name__,
        "kind": kind,
        "generic": bool(getattr(cls, "__parameters__", ())),
        "doc": class_doc(cls),
        "fields": [],
    }
    CLASSES[name] = entry
    try:
        hints = typing.get_type_hints(cls)
    except Exception:
        hints = dict(getattr(cls, "__annotations__", {}))
    if kind == "dataclass":
        members = [(field.name, True, field.init) for field in dataclasses.fields(cls)]
    elif kind == "namedtuple":
        members = [(field, True, True) for field in cls._fields]
    else:
        required = getattr(cls, "__required_keys__", frozenset(hints))
        members = [(field, field in required, True) for field in hints]
    entry["fields"] = [
        {
            "name": field,
            "annotation": None if field not in hints else tree(hints[field]),
            "required": required,
            "init": init,
        }
        for field, required, init in members
    ]


def class_doc(cls):
    """The first line of the class's own docstring; the one a dataclass or a
    `NamedTuple` writes for itself (`Point(across: int, down: int)`) does
    not count."""
    doc = cls.__dict__.get("__doc__")
    if not doc or doc.startswith(cls.__name__ + "("):
        return None
    return next((line.strip() for line in doc.splitlines() if line.strip()), None)


def qualified(kind):
    module = getattr(kind, "__module__", None)
    name = getattr(kind, "__qualname__", None) or getattr(kind, "__name__", None) or repr(kind)
    return name if module is None else module + "." + name


if __name__ == "__main__":
    main()
