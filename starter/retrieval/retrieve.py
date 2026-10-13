#!/usr/bin/env python3
"""Retrieval by embedding over the chunks `renyi index --chunks` writes
(the starter pack, decision AW43): the host's side of the loop, shown
once. The script keeps a store of vectors keyed by chunk id, embeds only
the chunks whose id it has not seen (the id is the hash of the text, so
it changes exactly when the text does), drops the ids that are gone, and
answers a question with the nearest chunks and their neighbours.

    renyi index --chunks . > project.jsonl
    renyi index --chunks --language > language.jsonl
    python retrieve.py project.jsonl language.jsonl --ask "where do we round money to cents"

Needs `pip install sentence-transformers` (a local model; the first run
downloads it). Nothing here is part of the toolchain: the toolchain
writes the chunks, this is the host that embeds them.
"""

import argparse
import json
import os


def read_chunks(paths):
    """Every chunk of the files, by id; the first line of a file is its
    header and carries no id."""
    chunks = {}
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                if line.strip():
                    record = json.loads(line)
                    if "id" in record:
                        chunks[record["id"]] = record
    return chunks


def load_store(path, model):
    """The vectors embedded so far, by id; empty when the file is missing
    or was made with another model."""
    if os.path.exists(path):
        with open(path, encoding="utf-8") as handle:
            store = json.load(handle)
        if store.get("model") == model:
            return store
    return {"model": model, "vectors": {}}


def embed(model_name, texts):
    from sentence_transformers import SentenceTransformer

    model = SentenceTransformer(model_name)
    return model.encode(texts, normalize_embeddings=True, show_progress_bar=False).tolist()


def neighbours_of(metadata, by_name):
    """The chunks a hit is read with: what it calls, what tests it and
    what it points to, where these are chunks of the index."""
    out = []
    for key in ("calls", "tested_by", "see_also"):
        for name in metadata.get(key) or []:
            qualified = name if name in by_name else f"{metadata['module']}.{name}"
            if qualified in by_name and qualified not in out:
                out.append(qualified)
    return out


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("files", nargs="+", help="chunk files `renyi index --chunks` wrote")
    parser.add_argument("--ask", help="a question to answer with the nearest chunks")
    parser.add_argument("--top", type=int, default=5, help="how many chunks to show")
    parser.add_argument("--store", default="retrieval-index.json", help="the vectors, by chunk id")
    parser.add_argument("--model", default="sentence-transformers/all-MiniLM-L6-v2")
    args = parser.parse_args()

    chunks = read_chunks(args.files)
    store = load_store(args.store, args.model)
    new = [id for id in chunks if id not in store["vectors"]]
    gone = [id for id in store["vectors"] if id not in chunks]
    if new:
        for id, vector in zip(new, embed(args.model, [chunks[id]["text"] for id in new])):
            store["vectors"][id] = vector
    for id in gone:
        del store["vectors"][id]
    with open(args.store, "w", encoding="utf-8") as handle:
        json.dump(store, handle)
    print(f"{len(chunks)} chunks: {len(new)} embedded, {len(gone)} dropped, {len(store['vectors'])} in {args.store}")

    if not args.ask:
        return
    question = embed(args.model, [args.ask])[0]
    scored = sorted(
        ((sum(a * b for a, b in zip(question, vector)), id) for id, vector in store["vectors"].items()),
        reverse=True,
    )
    by_name = {chunk["metadata"]["name"]: chunk for chunk in chunks.values()}
    for score, id in scored[: args.top]:
        metadata = chunks[id]["metadata"]
        location = metadata["location"]
        print(f"{score:.3f}  {metadata['kind']} {metadata['name']}  ({location['file']}:{location['line']})")
        if metadata.get("purpose"):
            print(f"       purpose: {metadata['purpose']}")
        neighbours = neighbours_of(metadata, by_name)
        if neighbours:
            print(f"       read with: {', '.join(neighbours)}")


if __name__ == "__main__":
    main()
