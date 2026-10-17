"""The `tools` package of a utility run: the Python side of the Renyi
programs' tool modules, over AgentDojo's own environment and tools.

The Renyi VM's worker imports `tools.<tool>` (one shim per tool, written by
`utility_run.py`) and calls `<tool>(*args)` by position; the shim calls
`call(name, args)` here. The package loads the suite named by
`RENYI_AGENTDOJO_SUITE` with the injections of `RENYI_AGENTDOJO_INJECTIONS`
(a JSON object, empty for the defaults) when first imported, runs every
call through AgentDojo's `FunctionsRuntime` with the environment's
dependencies injected as the benchmark does, and after every call pickles
the environment and the calls so far to `RENYI_AGENTDOJO_POST`, for the
judge to read when the program is over (a worker may be ended without
notice, so the state is kept current, not written at exit).

Results cross the bridge as JSON: a pydantic model as its fields with
`id_` renamed `id` (the Renyi declarations' spelling), datetimes and enums
as their JSON forms.
"""
import json
import os
import pickle

from agentdojo.functions_runtime import FunctionsRuntime
from agentdojo.task_suite.load_suites import get_suite
from pydantic import BaseModel

SUITE = os.environ["RENYI_AGENTDOJO_SUITE"]
suite = get_suite("v1", SUITE)
INJECTIONS = json.loads(os.environ.get("RENYI_AGENTDOJO_INJECTIONS") or "{}")
env = suite.load_and_inject_default_environment(INJECTIONS)
runtime = FunctionsRuntime(suite.tools)
POST = os.environ.get("RENYI_AGENTDOJO_POST")
calls = []


def convert(value):
    if isinstance(value, BaseModel):
        return convert(value.model_dump(mode="json"))
    if isinstance(value, dict):
        return {("id" if key == "id_" else key): convert(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [convert(item) for item in value]
    return value


def dump():
    if not POST:
        return
    # pickled, not JSON: the models rebuild derived fields (an inbox's `emails`
    # from `initial_emails`) when validated again, and a JSON round trip would
    # lose what the run added
    with open(POST, "wb") as out:
        pickle.dump({"environment": env, "calls": calls}, out)


def call(name, args):
    f = runtime.functions[name]
    names = list(f.parameters.model_fields)
    kwargs = dict(zip(names, args))
    result, error = runtime.run_function(env, name, kwargs, raise_on_error=False)
    calls.append({"function": name, "args": kwargs, "error": error})
    dump()
    if error is not None:
        raise RuntimeError(error)
    return convert(result)
