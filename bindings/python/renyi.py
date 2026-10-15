"""Renyi from Python, through the C API (decision AW46, stage 40b).

One file over ``ctypes`` and the standard library: a module loaded with
a grant answers calls to its public functions and nothing else. Values
cross as JSON, so a call takes Python values that ``json.dumps`` can
write (numbers, texts, booleans, ``None``, lists, dicts) and answers
what ``json.loads`` reads, in the codec of ``std.json``: a record a
dict by its field names, a variant a dict with ``kind``, a list a list,
a map a dict, Bytes base64, an Instant ISO 8601. Each argument is
decoded against the declared type of its parameter before anything
runs; one that does not fit is refused with the parameter named.

    import renyi
    with renyi.Sandbox.load("calc.ry", {"grant": "console"}) as calc:
        print(calc.call("add", 2, 3))

The library is found through ``RENYI_C_LIB`` (a path to ``renyi_c.dll``,
``librenyi_c.so`` or ``librenyi_c.dylib``), else beside this file (the
release archive's layout), else under ``target/release`` and
``target/debug`` of a checkout this file sits in, else by the system's
loader. The guide is ``docs/embedding.md`` section 7.
"""

import ctypes
import ctypes.util
import json
import os
import sys
from pathlib import Path

__all__ = [
    "Sandbox",
    "RenyiError",
    "LoadError",
    "Failed",
    "Crashed",
    "Exited",
    "Refused",
    "OverMemory",
    "version",
    "api_version",
    "library",
]

API_VERSION = 1

OK, FAILED, CRASHED, EXITED, REFUSED, OVER_MEMORY = range(6)


class RenyiError(Exception):
    """What a load or a call answered instead of a value."""


class LoadError(RenyiError):
    """The module did not load: the diagnostics or the grant's problem."""


class Failed(RenyiError):
    """The function failed; ``str(error)`` is the failure as the console shows it."""


class Crashed(RenyiError):
    """The call crashed; ``str(error)`` is the message and the location."""


class Exited(RenyiError):
    """``environment.exit``; ``code`` is the exit code."""

    def __init__(self, code):
        super().__init__(f"exited with {code}")
        self.code = code


class Refused(RenyiError):
    """Nothing ran: no such function, the wrong arguments, or an uncovered need."""


class OverMemory(RenyiError):
    """The call exceeded the memory budget; ``limit`` and ``used`` are bytes."""

    def __init__(self, limit, used):
        super().__init__(f"over the memory budget: {used} of {limit} bytes")
        self.limit = limit
        self.used = used


def _library_name():
    if sys.platform == "win32":
        return "renyi_c.dll"
    if sys.platform == "darwin":
        return "librenyi_c.dylib"
    return "librenyi_c.so"


def _candidates():
    named = os.environ.get("RENYI_C_LIB")
    if named:
        yield Path(named)
    here = Path(__file__).resolve().parent
    name = _library_name()
    yield here / name
    # a checkout: bindings/python/renyi.py beside target/
    root = here.parent.parent
    for profile in ("release", "debug"):
        yield root / "target" / profile / name
    found = ctypes.util.find_library("renyi_c")
    if found:
        yield Path(found)


_LIBRARY = None


def library(path=None):
    """The loaded C library, found as the module's docstring says, or at ``path``."""
    global _LIBRARY
    if _LIBRARY is not None and path is None:
        return _LIBRARY
    candidates = [Path(path)] if path else [c for c in _candidates() if c.exists()]
    if not candidates:
        raise LoadError(
            f"the library {_library_name()} was not found: set RENYI_C_LIB to its path "
            "or build it with `cargo build --release -p renyi_c`"
        )
    lib = ctypes.CDLL(str(candidates[0]))
    _declare(lib)
    got = lib.renyi_api_version()
    if got != API_VERSION:
        raise LoadError(f"the library speaks API {got}, this binding API {API_VERSION}")
    _LIBRARY = lib
    return lib


def _declare(lib):
    handle = ctypes.c_void_p
    text = ctypes.c_char_p
    size = ctypes.c_size_t
    signatures = {
        "renyi_api_version": ([], ctypes.c_int),
        "renyi_version": ([], ctypes.c_void_p),
        "renyi_version_len": ([], size),
        "renyi_load": ([text, size, text, size], handle),
        "renyi_load_source": ([text, size, text, size, text, size], handle),
        "renyi_free_sandbox": ([handle], None),
        "renyi_last_error": ([], ctypes.c_void_p),
        "renyi_last_error_len": ([], size),
        "renyi_text": ([handle], ctypes.c_void_p),
        "renyi_text_len": ([handle], size),
        "renyi_call": ([handle, text, size, text, size], ctypes.c_int),
        "renyi_call_guarded": ([handle, text, size, text, size, text, size], ctypes.c_int),
        "renyi_exit_code": ([handle], ctypes.c_int),
        "renyi_memory_limit": ([handle], ctypes.c_uint64),
        "renyi_memory_used": ([handle], ctypes.c_uint64),
        "renyi_module": ([handle], None),
        "renyi_grant": ([handle], None),
        "renyi_warnings": ([handle], None),
        "renyi_functions": ([handle], None),
    }
    for name, (argtypes, restype) in signatures.items():
        function = getattr(lib, name)
        function.argtypes = argtypes
        function.restype = restype


def _read(pointer, length):
    if not pointer or length == 0:
        return ""
    return ctypes.string_at(pointer, length).decode("utf-8")


def _bytes(value):
    return value.encode("utf-8") if isinstance(value, str) else bytes(value)


def _grant_text(grant):
    if grant is None:
        return b""
    if isinstance(grant, (dict, list)):
        return json.dumps(grant).encode("utf-8")
    return _bytes(grant)


def version():
    """The toolchain's version, as ``renyi --version`` prints it."""
    lib = library()
    return _read(lib.renyi_version(), lib.renyi_version_len())


def api_version():
    """The C API's version; this binding speaks ``API_VERSION``."""
    return library().renyi_api_version()


class Sandbox:
    """A loaded module with its grant: one handle, freed by ``close`` or ``with``."""

    def __init__(self, handle):
        self._lib = library()
        self._handle = handle

    @classmethod
    def load(cls, path, grant=None):
        """Load the module at ``path`` with the grant: a dict such as
        ``{"grant": "console", "memory": "16 megabytes"}``, the same as a
        JSON text, or ``None`` for a grant of nothing."""
        lib = library()
        path = _bytes(os.fspath(path))
        grant = _grant_text(grant)
        handle = lib.renyi_load(path, len(path), grant, len(grant))
        return cls._loaded(lib, handle)

    @classmethod
    def load_source(cls, name, text, grant=None):
        """Load the module from ``text``, its imports resolved from ``name``."""
        lib = library()
        name, text, grant = _bytes(name), _bytes(text), _grant_text(grant)
        handle = lib.renyi_load_source(name, len(name), text, len(text), grant, len(grant))
        return cls._loaded(lib, handle)

    @classmethod
    def _loaded(cls, lib, handle):
        if not handle:
            raise LoadError(_read(lib.renyi_last_error(), lib.renyi_last_error_len()))
        return cls(handle)

    def close(self):
        """Free the handle; the sandbox answers nothing after it."""
        if self._handle:
            self._lib.renyi_free_sandbox(self._handle)
            self._handle = None

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def __del__(self):
        try:
            self.close()
        except Exception:
            # the interpreter is shutting down and the library is gone;
            # nothing is left to free, because the process ends with it
            pass

    def _text(self):
        return _read(self._lib.renyi_text(self._handle), self._lib.renyi_text_len(self._handle))

    def _described(self, function):
        self._open()
        function(self._handle)
        return self._text()

    def _open(self):
        if not self._handle:
            raise RenyiError("the sandbox is closed")

    @property
    def module(self):
        """The module's name."""
        return self._described(self._lib.renyi_module)

    @property
    def grant(self):
        """The grant's capabilities, as a ``needs`` clause spells them."""
        return self._described(self._lib.renyi_grant)

    @property
    def warnings(self):
        """The checker's warnings, one per entry."""
        text = self._described(self._lib.renyi_warnings)
        return text.splitlines() if text else []

    def functions(self):
        """The public functions: dicts with ``name``, ``signature``, ``purpose``,
        ``parameters``, ``needs``, ``exposed_as_tool``, ``input_schema`` and
        ``callable``."""
        return json.loads(self._described(self._lib.renyi_functions))

    def call(self, name, *arguments, guards=None):
        """Call the public function ``name`` with the arguments, each a
        value ``json.dumps`` writes, and answer the value ``json.loads``
        reads; ``guards`` gives, per argument, the capability of the grant
        whose ``only to`` guard the argument carries, or ``None``. A call
        that ends otherwise raises ``Failed``, ``Crashed``, ``Exited``,
        ``Refused`` or ``OverMemory``."""
        self._open()
        name_bytes = _bytes(name)
        arguments_bytes = json.dumps(list(arguments)).encode("utf-8")
        if guards is None:
            status = self._lib.renyi_call(
                self._handle, name_bytes, len(name_bytes), arguments_bytes, len(arguments_bytes)
            )
        else:
            guards_bytes = json.dumps(list(guards)).encode("utf-8")
            status = self._lib.renyi_call_guarded(
                self._handle,
                name_bytes,
                len(name_bytes),
                arguments_bytes,
                len(arguments_bytes),
                guards_bytes,
                len(guards_bytes),
            )
        text = self._text()
        if status == OK:
            return json.loads(text)
        if status == FAILED:
            raise Failed(text)
        if status == CRASHED:
            raise Crashed(text)
        if status == EXITED:
            raise Exited(self._lib.renyi_exit_code(self._handle))
        if status == REFUSED:
            raise Refused(text)
        if status == OVER_MEMORY:
            raise OverMemory(
                self._lib.renyi_memory_limit(self._handle), self._lib.renyi_memory_used(self._handle)
            )
        raise RenyiError(f"status {status}: {text}")
