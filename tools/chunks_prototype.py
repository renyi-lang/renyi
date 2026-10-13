#!/usr/bin/env python3
"""The prototype of the chunks for retrieval by embedding (stage 37a,
decision AW40): the map's records and the sources as JSON Lines, one
chunk per definition, in one of three texts, so that the measure
(`tools/chunks_measure.py`) can settle which text the export of 37b
writes.

    python tools/chunks_prototype.py <path> --text source|header|hops
        [--renyi <binary>] [--out <file.jsonl>]

`<path>` is what `renyi index` takes (a directory, or a file with its
imports). The texts: `source`, the definition's canonical source alone;
`header`, a header from the record (the kind and the qualified name, the
signature, the purpose, the tags, the transitive effects, the module's
purpose) and then the source; `hops`, the header, the source and the
purposes of the project definitions it calls. Every chunk is one line:
`{"id": "sha256:<hash of the text>", "text": ..., "metadata": {...}}`,
after a first line naming the project, the revision, the toolchain, the
text and the count. The sources are read as they are on disk, which the
corpus and `compiler/` keep canonical; the export of 37b takes the
canonical text from the workspace. A development aid, as decision H1
allows; the export itself is `renyi index --chunks` (37b).
"""

import argparse
import hashlib
import json
import os
import subprocess
import sys

TEXTS = ("source", "header", "hops")


def read_map(renyi: str, path: str) -> dict:
    """The map of the project as `renyi index --json` prints it."""
    done = subprocess.run(
        [renyi, "index", "--json", path], capture_output=True, text=True, encoding="utf-8"
    )
    if done.returncode != 0 or not done.stdout.strip():
        sys.exit(f"renyi index failed on {path}:\n{done.stderr}")
    return json.loads(done.stdout)


def source_lines(cache: dict, root: str, location: dict) -> str:
    """The definition's lines, by the location the map gives (1-based,
    inclusive), from the file under the current directory or under the
    directory of the path indexed."""
    file = location["file"]
    if file not in cache:
        candidate = file if os.path.exists(file) else os.path.join(root, file)
        with open(candidate, encoding="utf-8") as handle:
            cache[file] = handle.read().split("\n")
    lines = cache[file]
    return "\n".join(lines[location["line"] - 1 : location["end_line"]]).rstrip("\n") + "\n"


def qualified(definition: dict) -> str:
    return f"{definition['module']}.{definition['name']}"


def header_of(definition: dict, module_purpose: str) -> str:
    """The lines an embedding should see before the source: what the
    definition is called, what it is for, what it needs."""
    lines = [f"{definition['kind']} {qualified(definition)}"]
    if definition["signature"]:
        lines.append(definition["signature"])
    if definition.get("purpose"):
        lines.append(f"purpose: {definition['purpose']}")
    if definition.get("tags"):
        lines.append("tags: " + ", ".join(definition["tags"]))
    effects = definition["effects"]["transitive"]
    if effects:
        lines.append("needs: " + ", ".join(effects))
    fails = definition["fails"]["transitive"]
    if fails:
        lines.append("fails with: " + ", ".join(fails))
    if module_purpose:
        lines.append(f"module {definition['module']}: {module_purpose}")
    return "\n".join(lines) + "\n"


def hops_of(definition: dict, by_name: dict) -> str:
    """The purposes of the project definitions the body calls, one line
    each; library calls have no record in the map and are left out."""
    lines = []
    for callee in definition["calls"]:
        target = by_name.get(callee)
        if target is not None and target.get("purpose"):
            lines.append(f"calls {callee}: {target['purpose']}")
    return ("\n".join(lines) + "\n") if lines else ""


def chunk_text(kind: str, definition: dict, source: str, module_purpose: str, by_name: dict) -> str:
    if kind == "source":
        return source
    text = header_of(definition, module_purpose) + "\n" + source
    if kind == "hops":
        hops = hops_of(definition, by_name)
        if hops:
            text = text + "\n" + hops
    return text


def metadata_of(definition: dict) -> dict:
    """What a host filters by and walks from: the map's identities, the
    kind and the module, the effects, the edges and the location."""
    return {
        "name": qualified(definition),
        "map_id": definition["id"],
        "text_hash": definition["text_hash"],
        "module": definition["module"],
        "package": definition.get("package"),
        "kind": definition["kind"],
        "public": definition["public"],
        "signature": definition["signature"],
        "purpose": definition.get("purpose"),
        "effects": definition["effects"]["transitive"],
        "fails": definition["fails"]["transitive"],
        "calls": definition["calls"],
        "uses": definition["uses"],
        "tested_by": definition["tested_by"],
        "see_also": definition.get("see_also", []),
        "tested": definition["coverage"]["tests"] > 0 or definition["coverage"]["examples"] > 0,
        "location": definition["location"],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("path", help="what `renyi index` takes: a directory, or a file with its imports")
    parser.add_argument("--text", choices=TEXTS, default="header", help="the text of a chunk")
    parser.add_argument("--renyi", default="renyi", help="the renyi binary (default: renyi on the PATH)")
    parser.add_argument("--out", help="the JSON Lines file to write (default: standard output)")
    args = parser.parse_args()

    project = read_map(args.renyi, args.path)
    root = args.path if os.path.isdir(args.path) else os.path.dirname(args.path) or "."
    module_purposes = {module["name"]: module.get("purpose") or "" for module in project["modules"]}
    by_name = {qualified(definition): definition for definition in project["definitions"]}
    cache: dict = {}
    lines = [
        json.dumps(
            {
                "project": project.get("project"),
                "revision": project.get("revision"),
                "toolchain": project.get("toolchain"),
                "text": args.text,
                "chunks": len(project["definitions"]),
            }
        )
    ]
    for definition in project["definitions"]:
        source = source_lines(cache, root, definition["location"])
        text = chunk_text(args.text, definition, source, module_purposes[definition["module"]], by_name)
        digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
        lines.append(
            json.dumps({"id": f"sha256:{digest}", "text": text, "metadata": metadata_of(definition)})
        )
    output = "\n".join(lines) + "\n"
    if args.out:
        with open(args.out, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(output)
        print(f"{len(project['definitions'])} chunks ({args.text}) to {args.out}", file=sys.stderr)
    else:
        sys.stdout.write(output)


if __name__ == "__main__":
    main()
