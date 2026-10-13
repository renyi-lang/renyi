# The retrieval measure

The query set of stage 37a (decision AW40): forty-five questions an
agent might ask of the corpus (`examples/`, twenty-five) and of the
compiler written in Renyi (`compiler/`, twenty), each with the
definition it targets, named as the map names it (`module.name`). The
questions paraphrase the purposes rather than copy them; a few quote a
line of code (`{:?}`, `next_cursor`).

The measure, by hand (no model and no network in CI):

```
python tools/chunks_prototype.py examples --text header --renyi target/debug/renyi.exe --out <lane>/examples_header.jsonl
python tools/chunks_prototype.py compiler --text header --renyi target/debug/renyi.exe --out <lane>/compiler_header.jsonl
HF_HUB_OFFLINE=1 python tools/chunks_measure.py --queries tests/retrieval/queries.json --bm25 [--model BAAI/bge-m3] <lane>/examples_header.jsonl <lane>/compiler_header.jsonl
```

for each of the three texts (`source`, `header`, `hops`), the two
projects' chunks in one index; hit@1, hit@5 and the mean reciprocal
rank in all and per project, with an embedding model (the default a
small one, `--model` a larger one from the local cache) and a BM25
baseline. The numbers are in the decision entry of the stage (AW41).
