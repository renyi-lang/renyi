# The article

`renyi.tex` and `renyi.bib` are the article on Renyi (decision AW45 iii):
the design, the implementation and the measures of release 0.2.0, for
arXiv under cs.PL, cross-listed to cs.AI.

Build:

```
cd paper && latexmk -pdf renyi
```

Every number in the text is read from the repository at the time of
writing and names its source in `docs/design/01-decisions.md` or a
measure's lane; the benchmark table is `tools/bench.py` on the release
build. When a number changes, the sentence that carries it changes in
the same commit.

For arXiv: submit `renyi.tex`, `renyi.bbl` (which `latexmk` writes; arXiv
does not run BibTeX) and nothing else; the first line `\pdfoutput=1` is
what arXiv's pdflatex path requires. The categories: cs.PL primary,
cs.AI cross-list. An account that has not published in cs.PL may be
asked for an endorsement.
