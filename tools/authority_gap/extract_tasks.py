"""Extract AgentDojo's user and injection tasks as data, without running its code.

Usage: python -I extract_tasks.py <agentdojo-src>/src/agentdojo/default_suites/v1 <out.json>

For every suite directory with user_tasks.py and injection_tasks.py, every
task class gives: id, prompt (or goal), and its ground-truth calls as
FunctionCall(function=..., args={...}) found by walking the AST of
`ground_truth`. Each argument is classified: ("literal", value) when it is
a constant or resolves through class/module constants (f-strings included),
("derived", source) when it comes from the environment or a computation.
A `ground_truth` that splices another task's ground truth is recorded as a
reference and resolved after the walk.
"""
import ast
import json
import sys
from pathlib import Path


class Unresolved(Exception):
    pass


def resolve(node, consts):
    """A literal value for `node` through `consts`, or Unresolved."""
    if isinstance(node, ast.Constant):
        return node.value
    if isinstance(node, ast.Name):
        if node.id in consts:
            return consts[node.id]
        raise Unresolved(ast.unparse(node))
    if isinstance(node, ast.Attribute) and isinstance(node.value, ast.Name) and node.value.id == "self":
        if node.attr in consts:
            return consts[node.attr]
        raise Unresolved(ast.unparse(node))
    if isinstance(node, ast.JoinedStr):
        parts = []
        for v in node.values:
            if isinstance(v, ast.Constant):
                parts.append(str(v.value))
            elif isinstance(v, ast.FormattedValue):
                parts.append(str(resolve(v.value, consts)))
            else:
                raise Unresolved(ast.unparse(node))
        return "".join(parts)
    if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
        return resolve(node.left, consts) + resolve(node.right, consts)
    if isinstance(node, (ast.List, ast.Tuple)):
        return [resolve(e, consts) for e in node.elts]
    if isinstance(node, ast.Dict):
        return {resolve(k, consts): resolve(v, consts) for k, v in zip(node.keys, node.values)}
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
        return -resolve(node.operand, consts)
    raise Unresolved(ast.unparse(node))


def classify(node, consts):
    try:
        return {"kind": "literal", "value": resolve(node, consts)}
    except Unresolved:
        return {"kind": "derived", "source": ast.unparse(node)}


def constant_or_skip(name, value, consts):
    """Record `name` when `value` resolves to a literal; a binding that does
    not (an import, a call, a comprehension) is not a constant, which is the
    reason it is left out: an argument that names it is then classified as
    derived by `classify`, which is the wanted outcome."""
    try:
        consts[name] = resolve(value, consts)
    except Unresolved:
        return


def module_consts(tree):
    consts = {}
    for node in tree.body:
        if isinstance(node, ast.Assign) and len(node.targets) == 1 and isinstance(node.targets[0], ast.Name):
            constant_or_skip(node.targets[0].id, node.value, consts)
    return consts


def class_consts(cls, consts):
    out = dict(consts)
    for node in cls.body:
        if isinstance(node, ast.Assign):
            targets = [t for t in node.targets if isinstance(t, ast.Name)]
            value = node.value
        elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and node.value is not None:
            targets = [node.target]
            value = node.value
        else:
            continue
        for t in targets:
            constant_or_skip(t.id, value, out)
    return out


def ground_truth_calls(cls, consts):
    fn = next((n for n in cls.body if isinstance(n, ast.FunctionDef) and n.name == "ground_truth"), None)
    if fn is None:
        return [], []
    calls, refs = [], []
    for node in ast.walk(fn):
        if not isinstance(node, ast.Call):
            continue
        if isinstance(node.func, ast.Name) and node.func.id == "FunctionCall":
            kw = {k.arg: k.value for k in node.keywords}
            function = resolve(kw["function"], consts)
            args = {}
            if "args" in kw and isinstance(kw["args"], ast.Dict):
                for k, v in zip(kw["args"].keys, kw["args"].values):
                    args[resolve(k, consts)] = classify(v, consts)
            calls.append({"function": function, "args": args})
        elif isinstance(node.func, ast.Attribute) and node.func.attr == "ground_truth":
            refs.append(ast.unparse(node.func.value))
    return calls, refs


def extract_file(path, kind):
    tree = ast.parse(path.read_text(encoding="utf-8"))
    consts = module_consts(tree)
    tasks = {}
    for cls in tree.body:
        if not isinstance(cls, ast.ClassDef) or not cls.name.startswith(kind):
            continue
        cc = class_consts(cls, consts)
        text_key = "PROMPT" if kind == "UserTask" else "GOAL"
        calls, refs = ground_truth_calls(cls, cc)
        tasks[cls.name] = {
            "id": cls.name,
            "text": cc.get(text_key, ""),
            "calls": calls,
            "refs": refs,
            "consts": {k: v for k, v in cc.items() if k not in consts and isinstance(v, (str, int, float, list))},
        }
    # a task that splices another's ground truth: `self._TASK.ground_truth(...)` with
    # `_TASK = UserTaskN()` as a class constant, or `UserTaskN().ground_truth(...)`
    for name, t in tasks.items():
        for ref in t["refs"]:
            target = None
            m = ref.replace("self.", "")
            if m.endswith("()"):
                target = m[:-2]
            else:
                cls = next(c for c in tree.body if isinstance(c, ast.ClassDef) and c.name == name)
                for node in cls.body:
                    if isinstance(node, ast.Assign) and any(isinstance(x, ast.Name) and x.id == m for x in node.targets):
                        if isinstance(node.value, ast.Call) and isinstance(node.value.func, ast.Name):
                            target = node.value.func.id
            if target in tasks:
                t["calls"] = tasks[target]["calls"] + t["calls"]
                t.setdefault("spliced", []).append(target)
            else:
                t.setdefault("unresolved_refs", []).append(ref)
    # a combined task (TaskCombinator.create_combined_task, the eleven of slack and
    # workspace): its ground truth is the two tasks' in order, its prompt the one
    # given or the combinator's "Perform the following tasks in the order they are
    # given: 1. <first> 2. <second>"
    for node in tree.body:
        call = node.value if isinstance(node, ast.Expr) else None
        if not (isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute)
                and call.func.attr == "create_combined_task"):
            continue
        name = resolve(call.args[0], consts)
        parts = []
        for arg in call.args[1:3]:
            # task_suite.user_tasks["user_task_N"] names the class UserTaskN
            key = resolve(arg.slice, consts)
            parts.append("UserTask" + key.rsplit("_", 1)[1])
        prompt = next((resolve(k.value, consts) for k in call.keywords if k.arg == "prompt"), None)
        first, second = (tasks[part] for part in parts)
        if prompt is None:
            prompt = f"Perform the following tasks in the order they are given: 1. {first['text']} 2. {second['text']}"
        tasks[name] = {
            "id": name,
            "text": prompt,
            "calls": first["calls"] + second["calls"],
            "refs": [],
            "consts": {**first["consts"], **second["consts"], "PROMPT": prompt},
            "combined": parts,
        }
    return tasks


def main():
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
    suites = {}
    for suite in sorted(p for p in root.iterdir() if (p / "user_tasks.py").exists()):
        suites[suite.name] = {
            "user": extract_file(suite / "user_tasks.py", "UserTask"),
            "injection": extract_file(suite / "injection_tasks.py", "InjectionTask"),
        }
    out.write_text(json.dumps(suites, indent=1, ensure_ascii=False), encoding="utf-8")
    for name, s in suites.items():
        u, i = s["user"], s["injection"]
        derived = sum(1 for t in u.values() for c in t["calls"] for a in c["args"].values() if a["kind"] == "derived")
        unres = sum(len(t.get("unresolved_refs", [])) for t in u.values())
        print(f"{name}: user {len(u)} tasks, {sum(len(t['calls']) for t in u.values())} calls ({derived} derived args, {unres} unresolved refs); "
              f"injection {len(i)} tasks, {sum(len(t['calls']) for t in i.values())} calls")


main()
