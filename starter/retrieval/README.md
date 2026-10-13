# Retrieval by embedding, the host's side

`renyi index --chunks` writes a project as chunks for retrieval by
embedding (one per module and per definition, each with an id that is the
hash of its text, the text and the metadata; `docs/design/05-agent-tooling.md`,
section 8), and `renyi index --chunks --language` writes the language's own
documents the same way (the cheat sheet's and the reference's sections, the
library's modules and public declarations). The toolchain writes; the host
embeds. `retrieve.py` is that host, in eighty lines, so that the loop is
shown once:

```sh
pip install sentence-transformers                      # a local model; the first run downloads it
renyi index --chunks ../workflows > project.jsonl      # the project's chunks
renyi index --chunks --language > language.jsonl      # the language's
python retrieve.py project.jsonl language.jsonl --ask "which program reads the log and groups its errors"
```

The script keeps `retrieval-index.json`, the vectors by chunk id: on every
run it embeds only the ids it has not seen and drops the ids that are gone,
which is the id contract (a chunk's id changes exactly when its text does).
A question is embedded with the same model and answered with the nearest
chunks, each with its kind, its name, its file and line and its purpose,
and the chunks to read it with: what it calls, what tests it and what its
`see also` names, taken from the metadata without a tool call. Change a
definition, export again, run again: one chunk is embedded, the rest are
kept.

`--model` names another model from the local cache (the measure of decision
AW41 used `BAAI/bge-m3` beside the default); `--top` how many chunks to
show; `--store` where the vectors go. CI checks that the script compiles
and does not run it: a model is outside the toolchain.
