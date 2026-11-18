# Manuscript

`memst-yifan.tex` is the paper; `memst-yifan.pdf` is its last build.

Every table and every number in the evaluation is generated from the result
files under `benchmarks/results/`, so the text cannot drift from the data:

```bash
python manuscript/memst/make_tables.py
```

writes `tables/*.tex` (the tables) and `tables/numbers.tex` (the `\num...`
commands the prose uses). Run it after a new evaluation run, then build:

```bash
cd manuscript/memst && pdflatex memst-yifan && bibtex memst-yifan && pdflatex memst-yifan && pdflatex memst-yifan
```

It needs TeX Live with the `latex-extra`, `fonts-extra`, `science` and
`pictures` collections.

`cite.bib` holds the references. Those added in November 2026 were checked
against arXiv metadata; older entries the paper does not cite have not been
checked. `cite-2026-candidates.bib` is a reading list, not a bibliography:
its author lists are truncated.
