#!/usr/bin/env python3
"""Build the documentation site (decision AI2).

Every Markdown document under `docs/` but the session handoff, the grammar
file, the corpus index with each example's source, and the starter pack's
README become one HTML page each, with a shared header, and are written to
a directory GitHub Pages serves (`.github/workflows/pages.yml`). Links
between the documents are rewritten to the pages; every link is relative,
so the site works under a path such as `/renyi/`.

The pages are static: no script, no font, no request beyond the page. The
front page, `docs/index.md`, and `docs/how-it-works.md` are HTML blocks
with inline SVG drawn in the text colour and the accent; the front page
has a layout of its own (`main.home`). Every other page gets a one-sentence
lead under its title (`LEADS`), and a long one a table of its sections
built from its headings, beside it at wide widths and in a disclosure on
narrow ones; the sections of the decision record fold. The terminal demo
(`SESSION`) is a real session with the release binary, drawn as an SVG
whose lines appear in turn; it replaces `<div class="terminal"></div>`.

Usage: python tools/site.py [<output directory>]      (default: _site)
Requires the `markdown` package (`pip install markdown`).
"""

from __future__ import annotations

import html
import posixpath
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path

import markdown

ROOT = Path(__file__).resolve().parent.parent

# The session handoff is the state of the work, not documentation.
SKIPPED = {"docs/HANDOFF.md"}

HOME = "docs/index.md"
REPOSITORY = "https://github.com/renyi-lang/renyi"

# The navigation, grouped; a target is a repository path or an address.
GROUPS = [
    ("Learn", [
        ("Cheat sheet", "docs/cheatsheet.md"),
        ("Examples", "examples/README.md"),
        ("Starter pack", "starter/README.md"),
        ("How it works", "docs/how-it-works.md"),
    ]),
    ("Reference", [
        ("Reference", "docs/reference.md"),
        ("Grammar", "docs/grammar.ebnf"),
    ]),
    ("Extend", [
        ("Extensions", "docs/extensions.md"),
        ("Python", "docs/python.md"),
        ("Embedding", "docs/embedding.md"),
    ]),
    ("Project", [
        ("Positioning", "docs/design/08-positioning.md"),
        ("Roadmap", "docs/ROADMAP.md"),
        ("Decisions", "docs/design/01-decisions.md"),
        ("GitHub", REPOSITORY),
    ]),
]

# One sentence under each page's title; an example's lead is its module's purpose.
LEADS = {
    "docs/cheatsheet.md": "The whole language on one page: every construct once, in the form a "
                          "program writes it.",
    "docs/how-it-works.md": "Each of the four promises drawn as the mechanism that keeps it, and "
                            "one real session at the terminal.",
    "docs/reference.md": "The normative description of Renyi: for each construct its grammar, "
                         "its meaning, its static rules with their codes and its behaviour at "
                         "run time.",
    "docs/grammar.ebnf": "The formal grammar of the syntax level in W3C EBNF, which a test holds "
                         "equal to what the parser accepts.",
    "docs/extensions.md": "How to add functions written in Rust to the toolchain: a declaration "
                          "file, a table of natives and a binary of a few lines.",
    "docs/python.md": "How a program calls a Python module through the bridge, under the same "
                      "grant, recording and replay as every other effect.",
    "docs/embedding.md": "How a Rust host loads a Renyi module under a grant and calls its "
                         "functions, with the effect system as the sandbox.",
    "docs/design/08-positioning.md": "What Renyi is to its first users, the four points it leads "
                                     "with and the mechanism behind each.",
    "docs/ROADMAP.md": "Every stage of the project from the first design document to today, in "
                       "order and by track, and what comes next.",
    "docs/design/01-decisions.md": "Every design decision with the options weighed and the reason, "
                                   "appended round by round; a reversal is a new entry.",
    "docs/design/02-syntax-sketch.md": "The design record of the concrete syntax, from which the "
                                       "reference and the grammar were derived.",
    "docs/design/03-readability-test.md": "The protocol that measured how well language models "
                                          "read and write the grammar before it was frozen.",
    "docs/design/04-stdlib-sketch.md": "The names and signatures of the prelude, the core modules "
                                       "and the extension packages.",
    "docs/design/05-agent-tooling.md": "The project map, the semantic diff, content hashes, "
                                       "budgets and the MCP server an agent works through.",
    "docs/design/06-runtime-guarantees.md": "Recorded and replayed runs, narration, budgets in "
                                            "grants and provenance guards.",
    "docs/design/07-system-design.md": "The trade-offs Renyi claims to resolve, and the design of "
                                       "capability-safe packages, reproducibility, in-process "
                                       "sandboxing and checked live update.",
    "docs/GAPS.md": "The audit of 2026-10-06: what the design promised and the implementation did "
                    "not yet deliver, with the order of work it proposed.",
    "docs/RELEASE.md": "How a release is made: what a version tag automates and what the owner "
                       "does by hand.",
    "examples/README.md": "Complete programs, one per file, that fixed the surface of the "
                          "language; the test suite parses, checks and formats every one.",
    "starter/README.md": "What an agent needs in its first hour: the skill, the MCP "
                         "configuration and five workflows that do real work.",
}

# The decision record folds its lettered sections; the last stays open.
FOLDED = {"docs/design/01-decisions.md"}

# A page lists its sections beside it when it has at least this many.
CONTENTS_FROM = 4

DESCRIPTION = (
    "Renyi is the scripting language of AI agents: the agent writes it, you "
    "review it at a glance, and the program can only do what it declares."
)

# The terminal demo: a session with the release binary (`target/renyi-au49`,
# RENYI_NO_CACHE=1) in a directory holding `todo.ry` and `todo.txt`; every
# line of output is what it printed, standard error and output interleaved.
SESSION = [
    ("renyi check todo.ry", [
        "todo.ry:10:15: error [capability-missing]: `read_text` needs `filesystem.read`, "
        "which `main` does not declare",
        "  fix: add `needs filesystem.read` to the signature of `main`",
    ]),
    ("sed -i 's/needs console/&, filesystem.read/' todo.ry", []),
    ("renyi check todo.ry", []),
    ("renyi record --to run.json todo.ry", [
        "3 items",
        "renyi: recorded 2 calls to run.json",
    ]),
    ("rm todo.txt", []),
    ("renyi run --replay run.json --explain todo.ry", [
        "Print how many items the list holds. (main)",
        '  filesystem.read read_text(path: "todo.txt") -> "milk\\neggs\\nbread\\n"',
        '  console "3 items"',
    ]),
]
COLUMNS = 54  # a longer line wraps, as a terminal of that width wraps it

STYLE = """
:root {
  color-scheme: light dark;
  --text: #16171a; --muted: #62656d; --faint: #8b8e95;
  --bg: #ffffff; --surface: #f6f6f4; --line: #e6e6e3; --line-strong: #d4d4d0;
  --accent: #2f4fd8; --tint: 9%; --tint-strong: 16%;
  --sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
          Roboto, "Helvetica Neue", Arial, sans-serif;
  --mono: ui-monospace, "SF Mono", "Cascadia Mono", "JetBrains Mono", Menlo,
          Consolas, "DejaVu Sans Mono", monospace;
  --measure: 44rem; --wide: 68rem; --gutter: 16px; --aside: 14rem;
}
@media (prefers-color-scheme: dark) {
  :root {
    --text: #ebebe8; --muted: #a2a4aa; --faint: #74777e;
    --bg: #0e0f11; --surface: #16171a; --line: #24262a; --line-strong: #34363b;
    --accent: #93a6ff; --tint: 14%; --tint-strong: 24%;
  }
}
@media (min-width: 640px) { :root { --gutter: 28px; } }
*, *::before, *::after { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; text-size-adjust: 100%; }
body { margin: 0; background: var(--bg); color: var(--text);
       font: 1rem/1.7 var(--sans); -webkit-font-smoothing: antialiased;
       text-rendering: optimizeLegibility; overflow-wrap: break-word; }
@media (min-width: 640px) { body { font-size: 1.0625rem; } }
::selection { background: color-mix(in srgb, var(--accent) 22%, transparent); }
.sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0);
      white-space: nowrap; }

/* The header: the name, then the four groups as one row from 1200px, a menu below. */
.top { position: sticky; top: 0; z-index: 10; border-bottom: 1px solid var(--line);
       background: var(--bg); background: color-mix(in srgb, var(--bg) 86%, transparent);
       -webkit-backdrop-filter: saturate(1.6) blur(14px);
       backdrop-filter: saturate(1.6) blur(14px); }
.bar { max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto; padding: 0 var(--gutter);
       display: flex; align-items: center; column-gap: 1.5rem; min-height: 3.3rem; }
.brand { padding: 0.8rem 0; font-weight: 650; font-size: 1.0625rem;
         letter-spacing: -0.015em; color: var(--text); text-decoration: none; }
.row { display: none; }
.menu { margin-left: auto; }
.menu > summary { list-style: none; cursor: pointer; padding: 0.8rem 0; font-size: 0.875rem;
                  color: var(--muted); display: flex; align-items: center; gap: 0.55rem; }
.menu > summary::-webkit-details-marker { display: none; }
.menu > summary::after { content: ""; width: 0.42rem; height: 0.42rem; margin-top: -0.25rem;
                         border-right: 1.5px solid currentColor;
                         border-bottom: 1.5px solid currentColor; transform: rotate(45deg);
                         transition: transform 0.15s; }
.menu[open] > summary { color: var(--text); }
.menu[open] > summary::after { transform: rotate(-135deg); margin-top: 0.2rem; }
.menu .panel { position: absolute; left: 0; right: 0; top: 100%; background: var(--bg);
               border-bottom: 1px solid var(--line); padding: 1.5rem var(--gutter) 2rem;
               max-height: calc(100vh - 3.5rem); overflow-y: auto;
               box-shadow: 0 18px 30px -24px rgb(0 0 0 / 0.35); }
.groups { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1.5rem 2rem;
          max-width: var(--wide); margin: 0 auto; }
.groups p { margin: 0 0 0.45rem; font-size: 0.75rem; font-weight: 600; letter-spacing: 0.07em;
            text-transform: uppercase; color: var(--faint); }
.groups ul { list-style: none; margin: 0; padding: 0; }
.groups li { margin: 0; }
.groups a { display: block; padding: 0.28rem 0; color: var(--muted); text-decoration: none;
            font-size: 0.9375rem; }
.groups a:hover, .groups a[aria-current="page"] { color: var(--text); }
@media (min-width: 640px) { .groups { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
@media (min-width: 1200px) {
  .menu { display: none; }
  .row { display: block; margin-left: auto; }
  .row > ul { display: flex; align-items: center; list-style: none; margin: 0; padding: 0; }
  .row li { display: flex; align-items: center; gap: 0.75rem; margin: 0; white-space: nowrap; }
  .row li + li { margin-left: 0.8rem; padding-left: 0.8rem;
                 border-left: 1px solid var(--line-strong); }
  .row a { padding: 0.8rem 0; font-size: 0.8125rem; color: var(--muted);
           text-decoration: none; transition: color 0.15s; }
  .row a:hover { color: var(--text); }
  .row a[aria-current="page"] { color: var(--text); font-weight: 550; }
}

/* The documents: one measure, a quiet scale, a lead under the title. */
main { max-width: calc(var(--measure) + 2 * var(--gutter)); margin: 0 auto;
       padding: 3rem var(--gutter) 5rem; }
main > :first-child, article > :first-child { margin-top: 0; }
h1, h2, h3, h4, h5, h6 { color: var(--text); line-height: 1.25; scroll-margin-top: 5.5rem;
                         letter-spacing: -0.015em; }
h1 { font-size: clamp(1.9rem, 1.4rem + 2vw, 2.6rem); font-weight: 680;
     letter-spacing: -0.03em; margin: 0 0 1.25rem; }
h2 { font-size: 1.45rem; font-weight: 650; margin: 3.25rem 0 1rem; padding-top: 1.75rem;
     border-top: 1px solid var(--line); }
h3 { font-size: 1.15rem; font-weight: 640; margin: 2.25rem 0 0.75rem; }
h4, h5, h6 { font-size: 1rem; font-weight: 640; margin: 1.75rem 0 0.5rem; }
h1 + h2 { border-top: 0; padding-top: 0; }
.lead { margin: -0.35rem 0 2.25rem; font-size: 1.1875rem; line-height: 1.5; color: var(--muted);
        letter-spacing: -0.01em; text-wrap: pretty; }
@media (min-width: 640px) { .lead { font-size: 1.3125rem; } }
p, ul, ol, dl, blockquote { margin: 0 0 1.1rem; }
ul, ol { padding-left: 1.4rem; }
li { margin: 0.3rem 0; }
li::marker { color: var(--faint); }
a { color: var(--accent); text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--accent) 35%, transparent);
    text-decoration-thickness: 1px; text-underline-offset: 0.2em; }
a:hover { text-decoration-color: currentColor; }
strong { font-weight: 640; }
hr { border: 0; border-top: 1px solid var(--line); margin: 3rem 0; }
blockquote { padding: 0.1rem 0 0.1rem 1.1rem; border-left: 2px solid var(--line-strong);
             color: var(--muted); }
.headerlink { margin-left: 0.4rem; color: var(--faint); text-decoration: none;
              font-weight: 400; opacity: 0; transition: opacity 0.15s; }
:is(h1, h2, h3, h4, h5, h6):hover .headerlink, .headerlink:focus { opacity: 1; }

/* Code: a quiet surface, its own scroll, never the page's. */
code, pre, kbd { font-family: var(--mono); font-size: 0.875em; font-variant-ligatures: none; }
:not(pre) > code { padding: 0.12em 0.36em; border-radius: 5px; background: var(--surface);
                   border: 1px solid var(--line); }
pre { margin: 0 0 1.4rem; padding: 1rem 1.15rem; overflow-x: auto; line-height: 1.6;
      background: var(--surface); border: 1px solid var(--line); border-radius: 10px;
      -webkit-overflow-scrolling: touch; tab-size: 4; }
pre code { font-size: 0.85rem; white-space: pre; overflow-wrap: normal; }
pre .mark { color: var(--accent); font-weight: 600; scroll-margin-top: 5.5rem; }
a > code { color: inherit; }
section > p > code { white-space: nowrap; }

/* Tables: inside a container that scrolls on a narrow screen. */
.table { margin: 1.5rem 0 1.75rem; overflow-x: auto; -webkit-overflow-scrolling: touch;
         border-top: 1px solid var(--line-strong); }
table { border-collapse: collapse; width: 100%; font-size: 0.9rem; line-height: 1.55; }
th, td { padding: 0.6rem 1rem 0.6rem 0; text-align: left; vertical-align: top;
         border-bottom: 1px solid var(--line); }
th:last-child, td:last-child { padding-right: 0; }
th { font-weight: 600; color: var(--muted); font-size: 0.8125rem; letter-spacing: 0.01em; }
@media (max-width: 639px) { th, td { min-width: 9rem; } }

/* The sections of a long page: a disclosure under the lead, a sticky column from 1100px. */
.contents ol { list-style: none; margin: 0; padding: 0; }
.contents ol ol { padding-left: 0.85rem; margin: 0.1rem 0 0.35rem; }
.contents li { margin: 0; }
.contents a { display: block; padding: 0.22rem 0; line-height: 1.4; color: var(--muted);
              text-decoration: none; }
.contents a:hover { color: var(--text); }
.contents code { font-size: 0.85em; padding: 0; border: 0; background: none; }
.inline { margin: 0 0 2.75rem; border: 1px solid var(--line); border-radius: 10px;
          background: var(--surface); font-size: 0.9375rem; }
.inline > summary { padding: 0.75rem 1rem; cursor: pointer; font-size: 0.875rem;
                    font-weight: 600; color: var(--muted); }
.inline > ol { padding: 0 1rem 1rem; max-height: 60vh; overflow-y: auto; }
.toc { display: none; }
.toc p { margin: 0 0 0.6rem; font-size: 0.75rem; font-weight: 600; letter-spacing: 0.07em;
         text-transform: uppercase; color: var(--faint); }
@media (min-width: 1100px) {
  main.paged { max-width: calc(var(--measure) + var(--aside) + 4rem + 2 * var(--gutter));
               display: grid; grid-template-columns: minmax(0, var(--measure)) var(--aside);
               column-gap: 4rem; align-items: start; }
  .inline { display: none; }
  .toc { display: block; position: sticky; top: 5rem; max-height: calc(100vh - 6.5rem);
         overflow-y: auto; overscroll-behavior: contain; padding: 0.2rem 0 1rem 1.1rem;
         border-left: 1px solid var(--line); font-size: 0.8125rem; scrollbar-width: thin; }
}

/* The decision record: each lettered section folds; find-in-page opens it. */
.fold { border-top: 1px solid var(--line); }
.fold:last-of-type { border-bottom: 1px solid var(--line); margin-bottom: 2rem; }
.fold > summary { list-style: none; cursor: pointer; display: flex; align-items: baseline;
                  gap: 0.85rem; padding: 0.95rem 0; }
.fold > summary::-webkit-details-marker { display: none; }
.fold > summary::before { content: ""; flex: none; width: 0.45rem; height: 0.45rem;
                          border-right: 1.5px solid var(--faint);
                          border-bottom: 1.5px solid var(--faint); transform: rotate(-45deg);
                          transition: transform 0.15s; position: relative; top: -0.12rem; }
.fold[open] > summary::before { transform: rotate(45deg); }
.fold > summary h2 { margin: 0; padding: 0; border: 0; font-size: 1.1875rem; font-weight: 640; }
.fold > summary:hover h2 { color: var(--accent); }
.fold[open] { padding-bottom: 1.5rem; }
.fold > hr:last-child, hr:has(+ .fold) { display: none; }

/* Diagrams: inline SVG in the text colour and the accent, so they follow the theme. */
figure { margin: 2rem 0 2.5rem; }
figcaption { margin-top: 0.85rem; font-size: 0.875rem; color: var(--muted); }
.dg { display: block; width: 100%; height: auto; color: var(--text);
      view-timeline: --dg block; }
.dg text { fill: currentColor; font-family: var(--sans); font-size: 13px; }
.dg .mono { font-family: var(--mono); font-size: 12px; }
.dg .sm { font-size: 11px; }
.dg .xs { font-size: 10px; }
.dg .cap { font-size: 10px; font-weight: 600; letter-spacing: 0.08em; fill-opacity: 0.55; }
.dg .b { font-weight: 640; }
.dg .mut { fill-opacity: 0.62; }
.dg .acc { fill: var(--accent); fill-opacity: 1; }
.dg .box { fill: var(--surface); stroke: currentColor; stroke-opacity: 0.16; }
.dg .frame { fill: none; stroke: currentColor; stroke-opacity: 0.22; }
.dg .dash { fill: none; stroke: currentColor; stroke-opacity: 0.35; stroke-dasharray: 3 4; }
.dg .tint { fill: color-mix(in srgb, var(--accent) var(--tint), transparent); }
.dg .tint2 { fill: color-mix(in srgb, var(--accent) var(--tint-strong), transparent); }
.dg .rail { fill: var(--accent); }
.dg .wire { fill: none; stroke: var(--accent); stroke-width: 1.5; stroke-linecap: round;
            stroke-linejoin: round; }
.dg .thin { fill: none; stroke: currentColor; stroke-opacity: 0.4; stroke-width: 1.2;
            stroke-linecap: round; stroke-linejoin: round; }
.dg .tip { fill: var(--accent); }
.dg .tip2 { fill: currentColor; fill-opacity: 0.55; }
.dg .ok { fill: none; stroke: var(--accent); stroke-width: 2; stroke-linecap: round;
          stroke-linejoin: round; }
.dg .no { fill: none; stroke: currentColor; stroke-width: 2; stroke-linecap: round; }
.full { max-width: 34rem; }
@keyframes dg-in { from { opacity: 0; } }
@supports (animation-timeline: view()) {
  .dg .p1, .dg .p2, .dg .p3 { animation: dg-in linear both; animation-timeline: --dg; }
  .dg .p1 { animation-range: entry 10% entry 55%; }
  .dg .p2 { animation-range: entry 35% entry 80%; }
  .dg .p3 { animation-range: entry 60% entry 100%; }
}

/* The terminal: a real session; its lines appear in turn and the loop starts again. */
.term { display: block; width: 100%; height: auto; max-width: 40rem; color: var(--text); }
.term .pane { fill: var(--surface); stroke: var(--line-strong); }
.term .rule { stroke: var(--line); }
.term .dot { fill: var(--line-strong); }
.term text { font-family: var(--mono); font-size: 13px; fill: currentColor; white-space: pre; }
.term .out { fill-opacity: 0.66; }
.term .fix { fill: var(--accent); fill-opacity: 1; }
.term .ps { fill: var(--accent); }
.term .name { font-family: var(--sans); font-size: 11px; fill-opacity: 0.5; }
.term .c0 { animation: c0 22s infinite; } .term .o0 { animation: o0 22s infinite; }
.term .c1 { animation: c1 22s infinite; } .term .c2 { animation: c2 22s infinite; }
.term .c3 { animation: c3 22s infinite; } .term .o3 { animation: o3 22s infinite; }
.term .c4 { animation: c4 22s infinite; } .term .c5 { animation: c5 22s infinite; }
.term .o5 { animation: o5 22s infinite; }
@keyframes c0 { 0%, 2% { opacity: 0; } 3%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes o0 { 0%, 7% { opacity: 0; } 8%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes c1 { 0%, 19% { opacity: 0; } 20%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes c2 { 0%, 28% { opacity: 0; } 29%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes c3 { 0%, 35% { opacity: 0; } 36%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes o3 { 0%, 40% { opacity: 0; } 41%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes c4 { 0%, 50% { opacity: 0; } 51%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes c5 { 0%, 58% { opacity: 0; } 59%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }
@keyframes o5 { 0%, 64% { opacity: 0; } 65%, 93% { opacity: 1; } 97%, 100% { opacity: 0; } }

footer { max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto;
         padding: 2.75rem var(--gutter) 3rem; border-top: 1px solid var(--line);
         color: var(--muted); font-size: 0.8125rem; }
footer .groups { margin: 0 0 2.75rem; }
footer .groups a { font-size: 0.875rem; }
.meta { display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; justify-content: space-between; }
.meta a { color: inherit; }

/* The front page: one idea per section, the code as its picture. */
main.home { max-width: calc(var(--wide) + 2 * var(--gutter)); padding-top: 0;
            padding-bottom: 2rem; }
.home section { padding: 5rem 0; border-top: 1px solid var(--line); scroll-margin-top: 3rem; }
.home h1 { font-size: clamp(2.9rem, 1.1rem + 6.6vw, 6.75rem); line-height: 0.98;
           letter-spacing: -0.055em; font-weight: 720; max-width: 11ch; margin: 0 0 2rem;
           text-wrap: balance; }
.home h2 { border: 0; padding: 0; margin: 0 0 2.5rem;
           font-size: clamp(1.9rem, 1.2rem + 2.6vw, 3.25rem); line-height: 1.06;
           letter-spacing: -0.04em; font-weight: 700; max-width: 18ch; text-wrap: balance; }
.home h3 { margin: 0 0 0.5rem; font-size: 1.125rem; font-weight: 660; letter-spacing: -0.02em; }
.home .hero { border-top: 0; padding: 4.5rem 0 4.5rem; }
.tagline { font-size: clamp(1.2rem, 0.95rem + 1.2vw, 1.85rem); line-height: 1.35;
           letter-spacing: -0.025em; font-weight: 450; color: var(--muted); max-width: 30ch;
           margin: 0 0 2.75rem; text-wrap: balance; }
.actions { display: flex; flex-wrap: wrap; gap: 0.75rem; margin: 0; }
.button { display: inline-block; padding: 0.7rem 1.35rem; border-radius: 999px;
          font-size: 0.9375rem; font-weight: 560; text-decoration: none;
          border: 1px solid var(--line-strong); color: var(--text); }
.button:hover { border-color: var(--text); }
.button.primary { background: var(--text); border-color: var(--text); color: var(--bg); }
.button.primary:hover { opacity: 0.88; }

/* The sample: the signature lines take the accent one after another. */
.sample pre { margin: 0; max-width: 54rem; padding: 1.5rem 1.25rem; border-radius: 16px;
              --pad: 1.25rem; padding-inline: var(--pad); overflow-x: hidden; }
.sample pre code { display: block; font-size: 0.8125rem; line-height: 1.75;
                   white-space: pre-wrap; overflow-wrap: anywhere; }
.sample .k { color: var(--text); font-weight: 600; }
.sample .c { color: var(--muted); }
.sample .sig { display: block; margin: 0 calc(-1 * var(--pad)); padding: 0 var(--pad);
               background: color-mix(in srgb, var(--accent) var(--tint), transparent);
               box-shadow: inset 3px 0 0 var(--accent);
               animation: sig-on 0.7s ease-out both; }
.sample .sig .k { color: var(--accent); animation: sig-k 0.7s ease-out both; }
.sample .sig.needs { background: color-mix(in srgb, var(--accent) var(--tint-strong),
                                            transparent); }
.sample .d1, .sample .d1 .k { animation-delay: 0.35s; }
.sample .d2, .sample .d2 .k { animation-delay: 0.85s; }
.sample .d3, .sample .d3 .k { animation-delay: 1.35s; }
@keyframes sig-on { from { background-color: transparent; box-shadow: inset 0 0 0 transparent; } }
@keyframes sig-k { from { color: var(--text); } }

/* The four points: muted numerals, a small drawing, a title, one line. */
.points { display: grid; grid-template-columns: minmax(0, 1fr); gap: 0 2rem; margin: 0; }
.points > div { padding: 1.75rem 0 2rem; border-top: 1px solid var(--line-strong); }
.points .n { display: block; margin: 0 0 1.25rem; font-size: clamp(2.5rem, 2rem + 1.5vw, 3.5rem);
             line-height: 1; font-weight: 300; letter-spacing: -0.04em; color: var(--faint);
             font-variant-numeric: tabular-nums; }
.points .dg { max-width: 20rem; margin: 0 0 1.5rem; }
.points p { margin: 0; color: var(--muted); font-size: 0.96875rem; line-height: 1.6; }
.more { margin: 2.5rem 0 0; font-weight: 560; }
.more a { text-decoration: none; }
.more a::after { content: " \\2192"; }

/* The gap and the session that closes it. */
.gap p { font-size: clamp(1.6rem, 1rem + 2.4vw, 3rem); line-height: 1.12; letter-spacing: -0.04em;
         font-weight: 650; max-width: 22ch; margin: 0; text-wrap: balance; }
.gap p + p { margin-top: 1.25rem; color: var(--muted); font-weight: 500; }
.gap figure { margin: 3.5rem 0 0; }
.demo figure { margin: 2rem 0 2.5rem; }
@media (max-width: 639px) {
  .gap figure, .demo figure { margin-inline: calc(-0.5 * var(--gutter)); }
}

/* Install: three commands, a label each. */
.install dl { display: grid; grid-template-columns: minmax(0, 1fr); gap: 0.6rem 2rem;
              margin: 0; }
.install dt { margin: 0.9rem 0 0; font-size: 0.8125rem; color: var(--muted); }
.install dd { margin: 0; }
.install pre { margin: 0; }
.install pre code { white-space: pre-wrap; overflow-wrap: anywhere; }

/* The links: labels alone, large, between hairlines. */
.links { list-style: none; padding: 0; margin: 0; display: grid;
         grid-template-columns: minmax(0, 1fr); column-gap: 3rem; }
.links li { margin: 0; border-top: 1px solid var(--line); }
.links a { display: flex; align-items: baseline; justify-content: space-between; gap: 1rem;
           padding: 1.1rem 0; color: var(--text); text-decoration: none;
           font-size: clamp(1.15rem, 1rem + 0.6vw, 1.4rem); font-weight: 600;
           letter-spacing: -0.02em; }
.links a::after { content: "\\2192"; color: var(--faint);
                  font-weight: 400; }
.links a:hover::after { color: var(--accent); }

@media (min-width: 640px) {
  .home section { padding: 7rem 0; }
  .home .hero { padding: 7.5rem 0 7rem; }
  .sample pre { --pad: 2rem; padding-block: 2rem; }
  .sample pre code { font-size: 0.9375rem; }
  .points { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .install dl { grid-template-columns: 10rem minmax(0, 1fr); align-items: center; }
  .install dt { margin: 0; }
  .links { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (min-width: 1100px) {
  .home section { padding: 8.5rem 0; }
  .home .hero { padding: 9.5rem 0 8.5rem; }
  .sample pre code { font-size: 1.0625rem; }
  .links { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .gap { display: grid; grid-template-columns: minmax(0, 5fr) minmax(0, 6fr); gap: 0 4rem;
         align-items: center; }
  .gap figure { margin: 0; grid-column: 2; grid-row: 1 / span 2; }
}
@media (min-width: 1200px) {
  .points { grid-template-columns: repeat(4, minmax(0, 1fr)); }
}

/* Motion is a courtesy: without it everything is shown at once, highlighted. */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; }
}
"""


def pages() -> dict[str, str]:
    """Every source file of the site, as a repository path, with its page path."""
    found: dict[str, str] = {}
    for path in sorted((ROOT / "docs").rglob("*")):
        if not path.is_file():
            continue
        source = path.relative_to(ROOT).as_posix()
        if source in SKIPPED or path.suffix not in {".md", ".ebnf"}:
            continue
        page = posixpath.relpath(source, "docs")
        found[source] = str(Path(page).with_suffix(".html").as_posix())
    found["examples/README.md"] = "examples/index.html"
    for path in sorted((ROOT / "examples").glob("*.ry")):
        found[f"examples/{path.name}"] = f"examples/{path.stem}.html"
    found["starter/README.md"] = "starter/index.html"
    return found


def title_of(source: str, text: str) -> str:
    if source == HOME:
        return "Renyi"  # the front page's heading is its one line, not its name
    if not source.endswith(".md"):
        return Path(source).name
    for line in text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    heading = re.search(r"<h1[^>]*>(.*?)</h1>", text, re.S)  # a page written as HTML
    if heading:
        return html.unescape(re.sub(r"<[^>]+>", "", heading.group(1))).strip()
    return Path(source).name


def lead_of(source: str, text: str) -> str:
    """The sentence under the title: from LEADS, or an example's module purpose."""
    if source in LEADS:
        return html.escape(LEADS[source], quote=False)
    found = re.match(r"module [^\n]*\n\s+purpose: ([^\n]+)", text)
    if source.endswith(".ry") and found:
        return html.escape(found.group(1).strip(), quote=False)
    return ""


def version() -> str:
    manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    found = re.search(r'^version = "([^"]+)"', manifest, re.M)
    return found.group(1) if found else "unknown"


def commit() -> str:
    try:
        done = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=True,
        )
        return done.stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def relative(from_page: str, to_page: str) -> str:
    """The relative link from one page to another, as the browser resolves it."""
    return posixpath.relpath(to_page, posixpath.dirname(from_page) or ".")


def rewrite_links(body: str, source: str, page: str, known: dict[str, str]) -> str:
    """Links to other source files become links to their pages; others stay."""

    def replacement(found: re.Match[str]) -> str:
        target = found.group(2)
        if "://" in target or target.startswith("#") or target.startswith("mailto:"):
            return found.group(0)
        path, _, fragment = target.partition("#")
        resolved = posixpath.normpath(posixpath.join(posixpath.dirname(source), path))
        if resolved not in known:
            return found.group(0)
        link = relative(page, known[resolved])
        if fragment:
            link = f"{link}#{fragment}"
        return f'{found.group(1)}="{link}"'

    return re.sub(r'\b(href|src)="([^"]+)"', replacement, body)


# A page's sections: (id, label as HTML, its subsections as (id, label)).
Contents = list[tuple[str, str, list[tuple[str, str]]]]


def sections_of(tokens: list[dict]) -> Contents:
    """The h2 headings and, under each, the h3 headings whose names the page does
    not repeat (the reference's "Grammar" or "Meaning" under every section)."""
    found: list[dict] = []

    def walk(items: list[dict]) -> None:
        for item in items:
            if item["level"] == 2:
                found.append(item)
            elif item["level"] < 2:
                walk(item["children"])

    walk(tokens)
    names = Counter(child["name"] for item in found for child in item["children"])
    return [
        (item["id"], item["html"],
         [(child["id"], child["html"]) for child in item["children"]
          if child["level"] == 3 and names[child["name"]] == 1])
        for item in found
    ]


def render_grammar(text: str) -> tuple[str, Contents]:
    """The grammar file in one block, its section comments marked as anchors."""
    lines, contents = [], []
    for line in html.escape(text, quote=False).split("\n"):
        found = re.fullmatch(r"/\* (Sections? ([^*]+?)) \*/", line)
        if found:
            ident = "section-" + re.sub(r"[^a-z0-9]+", "-", found.group(2).lower()).strip("-")
            contents.append((ident, found.group(1), []))
            line = f'<span class="mark" id="{ident}">{line}</span>'
        lines.append(line)
    body = "\n".join(lines)
    return f"<h1><code>grammar.ebnf</code></h1>\n<pre><code>{body}</code></pre>\n", contents


def render(source: str, text: str, known: dict[str, str]) -> tuple[str, Contents]:
    """The body of a page with its sections: Markdown rendered, or a source file."""
    if source.endswith(".ebnf"):
        return render_grammar(text)
    if not source.endswith(".md"):
        name = Path(source).name
        return (f"<h1><code>{html.escape(name)}</code></h1>\n"
                f"<pre><code>{html.escape(text)}</code></pre>\n"), []
    converter = markdown.Markdown(
        extensions=["fenced_code", "tables", "toc", "sane_lists"],
        extension_configs={"toc": {"permalink": "#", "permalink_title": "Link to this section"}},
    )
    body = converter.convert(text)
    contents = sections_of(converter.toc_tokens)  # type: ignore[attr-defined]
    # a table scrolls inside its own container, never the page
    body = body.replace("<table>", '<div class="table"><table>')
    body = body.replace("</table>", "</table></div>")
    if source == "examples/README.md":
        # the corpus index names its programs in code spans; each becomes a link
        def linked(found: re.Match[str]) -> str:
            name = found.group(1)
            if f"examples/{name}" not in known:
                return found.group(0)
            return f'<a href="{name}"><code>{name}</code></a>'

        body = re.sub(r"<code>([a-z_]+\.ry)</code>", linked, body)
    if source in FOLDED:
        body = fold(body)
    return body, contents


def fold(body: str) -> str:
    """Each h2 section in a details element whose summary is the heading; the last
    stays open. A closed section's text is still found by find-in-page, which
    opens it (Chromium, Firefox, Safari)."""
    first, *sections = re.split(r"(?=<h2[ >])", body)
    out = [first]
    for number, section in enumerate(sections, 1):
        end = section.index("</h2>") + len("</h2>")
        state = " open" if number == len(sections) else ""
        out.append(f'<details class="fold"{state}><summary>{section[:end]}</summary>\n'
                   f"{section[end:].strip()}\n</details>\n")
    return "".join(out)


def contents_list(contents: Contents) -> str:
    items = []
    for ident, label, children in contents:
        nested = "".join(f'<li><a href="#{child}">{text}</a></li>' for child, text in children)
        nested = f"<ol>{nested}</ol>" if nested else ""
        items.append(f'<li><a href="#{ident}">{label}</a>{nested}</li>')
    return f"<ol>{''.join(items)}</ol>"


def wrap_rows(line: str) -> list[str]:
    """A line as the rows a terminal COLUMNS wide shows; joined, they are the line."""
    rows: list[str] = []
    while len(line) > COLUMNS:
        cut = line.rfind(" ", 0, COLUMNS) + 1 or COLUMNS
        rows.append(line[:cut])
        line = line[cut:]
    return rows + [line]


def terminal() -> str:
    """The session as an SVG; each command and its output appear in turn (STYLE)."""
    rows = []  # (text, class, continued)
    for beat, (command, output) in enumerate(SESSION):
        for index, row in enumerate(wrap_rows("$ " + command)):
            rows.append((row, f"c{beat}", index > 0))
        for line in output:
            kind = "out fix" if line.startswith("  fix:") else "out"
            for index, row in enumerate(wrap_rows(line)):
                rows.append((row, f"{kind} o{beat}", index > 0))
    char, step, left, top = 7.9, 20, 18, 52
    width = round(left * 2 + COLUMNS * char)
    height = top + step * (len(rows) - 1) + 24
    lines = []
    for number, (row, kind, continued) in enumerate(rows):
        x = left + (2 * char if continued else 0)
        text = html.escape(row, quote=False)
        if row.startswith("$ ") and not continued:
            text = f'<tspan class="ps">$</tspan>{text[1:]}'
        lines.append(f'<text class="{kind}" x="{x:g}" y="{top + step * number}">{text}</text>')
    said = " ".join(command for command, _ in SESSION)
    return (
        f'<svg class="term" viewBox="0 0 {width} {height}" role="img" '
        'aria-labelledby="term-title term-desc">\n'
        '<title id="term-title">A session at the terminal</title>\n'
        '<desc id="term-desc">renyi check refuses a program that reads a file without '
        "declaring filesystem.read and prints the fix; after the fix the check passes, "
        "renyi record runs it and writes run.json, and with the input file deleted renyi run "
        f"--replay answers every effect from the recording. The commands: {html.escape(said)}"
        "</desc>\n"
        f'<rect class="pane" x="0.5" y="0.5" width="{width - 1}" height="{height - 1}" '
        'rx="12"/>\n'
        f'<line class="rule" x1="0" y1="28.5" x2="{width}" y2="28.5"/>\n'
        '<circle class="dot" cx="18" cy="14.5" r="4.5"/>'
        '<circle class="dot" cx="33" cy="14.5" r="4.5"/>'
        '<circle class="dot" cx="48" cy="14.5" r="4.5"/>\n'
        f'<text class="name" x="{width / 2:g}" y="18.5" text-anchor="middle">todo</text>\n'
        + "\n".join(lines) + "\n</svg>"
    )


def link(page: str, label: str, target: str, known: dict[str, str]) -> str:
    if "://" in target:
        return f'<a href="{target}">{label}</a>'
    here = known[target] == page or (target == "examples/README.md"
                                     and page.startswith("examples/"))
    current = ' aria-current="page"' if here else ""
    return f'<a href="{relative(page, known[target])}"{current}>{label}</a>'


def groups(page: str, known: dict[str, str]) -> str:
    """The navigation's groups, each a label and its links (the menu and the footer)."""
    blocks = []
    for name, entries in GROUPS:
        items = "".join(f"<li>{link(page, label, target, known)}</li>"
                        for label, target in entries)
        blocks.append(f"<div><p>{name}</p><ul>{items}</ul></div>")
    return f'<div class="groups">{"".join(blocks)}</div>'


def header(page: str, known: dict[str, str]) -> str:
    home = known[HOME]
    current = ' aria-current="page"' if home == page else ""
    row = "".join(
        f'<li><span class="sr">{name}: </span>'
        + "".join(link(page, label, target, known) for label, target in entries) + "</li>"
        for name, entries in GROUPS
    )
    return (
        '<header class="top"><div class="bar">'
        f'<a class="brand" href="{relative(page, home)}"{current}>Renyi</a>\n'
        f'<nav class="row" aria-label="Site"><ul>{row}</ul></nav>\n'
        '<details class="menu"><summary>Menu</summary>'
        f'<nav class="panel" aria-label="Site">{groups(page, known)}</nav></details>'
        "</div></header>\n"
    )


def wrap(page: str, title: str, body: str, known: dict[str, str], stamp: str) -> str:
    """The page around a body; the front page's `main` has a layout of its own."""
    heading = title if title == "Renyi" else f"{title} · Renyi"
    return (
        "<!doctype html>\n"
        '<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{html.escape(heading)}</title>\n"
        f'<meta name="description" content="{html.escape(DESCRIPTION)}">\n'
        '<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">\n'
        '<meta name="theme-color" content="#0e0f11" media="(prefers-color-scheme: dark)">\n'
        f"<style>{STYLE}</style>\n</head>\n<body>\n"
        f"{header(page, known)}"
        f"{body}\n"
        f"<footer>{groups(page, known)}"
        f'<div class="meta"><span>{stamp}</span><span>Apache-2.0 · <a href="{REPOSITORY}">'
        "github.com/renyi-lang/renyi</a></span></div></footer>\n</body>\n</html>\n"
    )


def layout(source: str, text: str, body: str, contents: Contents) -> str:
    """The `main` of a page: the lead under the title and, for a long page, its
    sections in a disclosure there and in a sticky column beside it."""
    if source == HOME:
        return f'<main class="home">\n{body}\n</main>'
    lead = lead_of(source, text)
    long = len(contents) >= CONTENTS_FROM
    after = f'\n<p class="lead">{lead}</p>' if lead else ""
    if long:
        after += ('\n<details class="contents inline"><summary>On this page</summary>'
                  f"{contents_list(contents)}</details>")
    if "</h1>" in body:
        head, _, rest = body.partition("</h1>")
        body = f"{head}</h1>{after}{rest}"
    if not long:
        return f"<main>\n{body}\n</main>"
    return (
        '<main class="paged">\n<article>\n'
        f"{body}\n</article>\n"
        '<nav class="contents toc" aria-label="On this page"><p>On this page</p>'
        f"{contents_list(contents)}</nav>\n</main>"
    )


def build(out: Path) -> int:
    known = pages()
    stamp = f"Renyi {version()}, built from commit {commit()}."
    demo = terminal()
    for source, page in known.items():
        text = (ROOT / source).read_text(encoding="utf-8")
        body, contents = render(source, text, known)
        body = body.replace('<div class="terminal"></div>', demo)
        body = rewrite_links(layout(source, text, body, contents), source, page, known)
        target = out / page
        target.parent.mkdir(parents=True, exist_ok=True)
        page_html = wrap(page, title_of(source, text), body, known, stamp)
        target.write_text(page_html, encoding="utf-8", newline="\n")
    return len(known)


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "_site"
    count = build(out)
    print(f"{count} pages written to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
