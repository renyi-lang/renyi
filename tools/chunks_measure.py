#!/usr/bin/env python3
"""The measure of the chunks' texts (stage 37a, decision AW40): a query
set against one index of chunks, with an embedding model and, beside it,
a lexical baseline.

    python tools/chunks_measure.py --queries tests/retrieval/queries.json
        [--model sentence-transformers/all-MiniLM-L6-v2] [--bm25] <chunks.jsonl>...

Every chunks file (written by `tools/chunks_prototype.py`, one text per
file) joins one index; the files given should be one text of every
project the queries name. For each query the chunks are ranked by cosine
similarity of normalized embeddings, and the rank of the chunk whose
metadata name is the query's target gives hit@1, hit@5 and the mean
reciprocal rank, printed in all and per project. `--bm25` ranks by BM25
over lower-cased alphanumeric tokens as well (`rank_bm25`). The model is
loaded by `sentence_transformers`; set `HF_HUB_OFFLINE=1` to use a model
already in the cache without the network. A development aid (decision
H1); its numbers go to the decision entry of the stage.
"""

import argparse
import json
import re
import sys


def load_chunks(paths: list[str]) -> list[dict]:
    chunks = []
    for path in paths:
        with open(path, encoding="utf-8") as handle:
            first = True
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                record = json.loads(line)
                if first:
                    first = False
                    if "text" in record and "chunks" in record:
                        continue  # the header line
                chunks.append(record)
    return chunks


def ranks_of(scores, chunks: list[dict], queries: list[dict]) -> list[int | None]:
    """For each query, the 1-based rank of its target, or None when the
    target is in no chunk."""
    names = [chunk["metadata"]["name"] for chunk in chunks]
    ranks = []
    for query, row in zip(queries, scores):
        order = sorted(range(len(chunks)), key=lambda i: -row[i])
        rank = None
        for position, index in enumerate(order, start=1):
            if names[index] == query["target"]:
                rank = position
                break
        ranks.append(rank)
    return ranks


def report(label: str, ranks: list[int | None], queries: list[dict]) -> None:
    def line(name: str, selected: list[int]) -> str:
        found = [ranks[i] for i in selected if ranks[i] is not None]
        missing = len(selected) - len(found)
        hit1 = sum(1 for r in found if r == 1)
        hit5 = sum(1 for r in found if r <= 5)
        mrr = sum(1 / r for r in found) / len(selected) if selected else 0.0
        return (
            f"  {name:10} queries {len(selected):3}  hit@1 {hit1:3} ({hit1 / len(selected):5.1%})"
            f"  hit@5 {hit5:3} ({hit5 / len(selected):5.1%})  MRR {mrr:.3f}"
            + (f"  target missing {missing}" if missing else "")
        )

    print(label)
    print(line("all", list(range(len(queries)))))
    for project in sorted({query["project"] for query in queries}):
        print(line(project, [i for i, query in enumerate(queries) if query["project"] == project]))


def tokens(text: str) -> list[str]:
    return re.findall(r"[a-z0-9]+", text.lower())


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("chunks", nargs="+", help="chunks files, one text of every project the queries name")
    parser.add_argument("--queries", required=True, help="the query set (JSON: {\"queries\": [{query, target, project}]})")
    parser.add_argument("--model", default="sentence-transformers/all-MiniLM-L6-v2", help="the embedding model")
    parser.add_argument("--bm25", action="store_true", help="rank by BM25 over the texts as well")
    parser.add_argument("--no-embedding", action="store_true", help="skip the embedding model")
    args = parser.parse_args()

    with open(args.queries, encoding="utf-8") as handle:
        queries = json.load(handle)["queries"]
    chunks = load_chunks(args.chunks)
    texts = [chunk["text"] for chunk in chunks]
    print(f"{len(chunks)} chunks from {len(args.chunks)} file(s), {len(queries)} queries")

    if args.bm25:
        from rank_bm25 import BM25Okapi

        index = BM25Okapi([tokens(text) for text in texts])
        scores = [index.get_scores(tokens(query["query"])) for query in queries]
        report("BM25 (lexical baseline)", ranks_of(scores, chunks, queries), queries)

    if not args.no_embedding:
        import numpy as np
        from sentence_transformers import SentenceTransformer

        model = SentenceTransformer(args.model)
        embedded = model.encode(texts, batch_size=32, normalize_embeddings=True, show_progress_bar=False)
        asked = model.encode(
            [query["query"] for query in queries], batch_size=32, normalize_embeddings=True, show_progress_bar=False
        )
        scores = np.asarray(asked) @ np.asarray(embedded).T
        report(f"embedding ({args.model})", ranks_of(scores, chunks, queries), queries)


if __name__ == "__main__":
    main()
