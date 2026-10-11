#!/usr/bin/env python3
"""Build the documentation site (decision AI2).

Every Markdown document under `docs/` but the session handoff, the grammar
file, the corpus index with each example's source, and the starter pack's
README become one HTML page each, with a shared header and footer, and are
written to a directory GitHub Pages serves (`.github/workflows/pages.yml`).
Links between the documents are rewritten to the pages; every link is
relative, so the site works under a path such as `/renyi/`.

A page makes no request beyond itself: no font, no stylesheet and no script
from elsewhere. The colours of both themes are custom properties (`LIGHT`,
`DARK`). Code is highlighted here: Renyi by the token classes of reference
section 1 with the reserved words of its section 17, the grammar as EBNF,
and the other languages the documents quote by a small lexicon each. A
short inline script (`SCRIPT`) adds a theme switch, copy buttons, the
section being read in the contents, and plays the terminal once it comes
into view; every page works without it.

The front page, `docs/index.md`, and `docs/how-it-works.md` are HTML blocks
with inline SVG drawn in the theme's colours, each with a layout of its own
(`LAYOUTS`). Every other page gets the name of its group above its title, a
one-sentence lead under it (`LEADS`), and a long one a table of its
sections built from its headings, beside it at wide widths and in a
disclosure on narrow ones; the sections of the decision record fold, and
the corpus index shows its table as a card for each program. A page of the
navigation links the pages before and after it, an example the examples
before and after it in the corpus index. The terminal demo (`SESSION`) is a
real session with the release binary, as text whose lines appear in turn;
it replaces `<div class="terminal"></div>`.

Usage: python tools/site.py [<output directory>]      (default: _site)
Requires the `markdown` package (`pip install markdown`).
"""

from __future__ import annotations

import functools
import html
import posixpath
import re
import subprocess
import sys
from collections import Counter
from pathlib import Path
from urllib.parse import quote

import markdown

ROOT = Path(__file__).resolve().parent.parent

# The session handoff is the state of the work, not documentation.
SKIPPED = {"docs/HANDOFF.md"}

HOME = "docs/index.md"
CORPUS = "examples/README.md"
REPOSITORY = "https://github.com/renyi-lang/renyi"

# The navigation, grouped; a target is a repository path or an address.
GROUPS = [
    ("Learn", [
        ("Cheat sheet", "docs/cheatsheet.md"),
        ("Examples", CORPUS),
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

# The group a page outside the navigation is shown under, by path prefix.
SECTIONS = [("docs/design/", "Design record"), ("examples/", "Examples"), ("docs/", "Project")]

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
    CORPUS: "Complete programs, one per file, that fixed the surface of the language; the test "
            "suite parses, checks and formats every one.",
    "starter/README.md": "What an agent needs in its first hour: the skill, the MCP "
                         "configuration and five workflows that do real work.",
}

# Pages with a layout of their own: the class of their `main`.
LAYOUTS = {HOME: "home", "docs/how-it-works.md": "feature"}

# The decision record folds its lettered sections; the last stays open.
FOLDED = {"docs/design/01-decisions.md"}

# A page lists its sections beside it when it has at least this many.
CONTENTS_FROM = 4

# An inline code span longer than this may break across lines; a shorter one
# stays whole, so that `--explain` never breaks at its hyphens.
WRAP_FROM = 24

DESCRIPTION = (
    "Renyi is the scripting language of AI agents: the agent writes it, you "
    "review it at a glance, and the program can only do what it declares."
)

# The two themes as CSS custom properties. Every text colour keeps a contrast
# of at least 4.5:1 (WCAG AA) with each background it stands on, the accent and
# the colours of highlighted code included; `text-3` is the faintest text.
LIGHT = {
    "bg": "#ffffff",
    "bg-soft": "#f8f9fb",
    "raised": "#ffffff",
    "surface": "#f4f5f8",
    "surface-strong": "#eceef2",
    "line": "#e4e6eb",
    "line-strong": "#d0d4db",
    "text": "#111318",
    "text-2": "#50565f",
    "text-3": "#656b75",
    "accent": "#2f4fd8",
    "accent-strong": "#2440b8",
    "on-accent": "#ffffff",
    "danger": "#c4321c",
    "tint": "color-mix(in srgb, #2f4fd8 8%, transparent)",
    "tint-strong": "color-mix(in srgb, #2f4fd8 15%, transparent)",
    "selection": "color-mix(in srgb, #2f4fd8 20%, transparent)",
    "syn-keyword": "#2f4fd8",
    "syn-type": "#7b3fbf",
    "syn-string": "#0f7a55",
    "syn-number": "#a24b0c",
    "syn-comment": "#656b75",
    "syn-doc": "#50565f",
    "grid": "rgb(17 19 24 / 0.055)",
    "glow": "color-mix(in srgb, #2f4fd8 14%, transparent)",
    "shadow-sm": "0 1px 2px rgb(17 19 24 / 0.06)",
    "shadow-md": "0 1px 2px rgb(17 19 24 / 0.04), 0 8px 20px -8px rgb(17 19 24 / 0.14)",
    "shadow-lg": "0 1px 2px rgb(17 19 24 / 0.04), 0 28px 56px -24px rgb(17 19 24 / 0.26)",
}
DARK = {
    "bg": "#0a0b0e",
    "bg-soft": "#0e1014",
    "raised": "#121419",
    "surface": "#13151a",
    "surface-strong": "#1a1d24",
    "line": "#23262e",
    "line-strong": "#323640",
    "text": "#ecedf0",
    "text-2": "#a3a9b3",
    "text-3": "#868d98",
    "accent": "#8ea2ff",
    "accent-strong": "#aebcff",
    "on-accent": "#0a0b0e",
    "danger": "#ff8f80",
    "tint": "color-mix(in srgb, #8ea2ff 12%, transparent)",
    "tint-strong": "color-mix(in srgb, #8ea2ff 21%, transparent)",
    "selection": "color-mix(in srgb, #8ea2ff 32%, transparent)",
    "syn-keyword": "#8ea2ff",
    "syn-type": "#c9a5ff",
    "syn-string": "#6cd3a5",
    "syn-number": "#f2b27a",
    "syn-comment": "#868d98",
    "syn-doc": "#a3a9b3",
    "grid": "rgb(236 237 240 / 0.05)",
    "glow": "color-mix(in srgb, #8ea2ff 13%, transparent)",
    "shadow-sm": "0 1px 2px rgb(0 0 0 / 0.5)",
    "shadow-md": "0 1px 2px rgb(0 0 0 / 0.5), 0 8px 20px -8px rgb(0 0 0 / 0.6)",
    "shadow-lg": "0 1px 2px rgb(0 0 0 / 0.5), 0 28px 56px -24px rgb(0 0 0 / 0.75)",
    # the front page's band, dark in both themes (`.invert` takes this palette)
    "band": "#0f1115",
}

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

# The pace of the terminal, in seconds: a command appears, its output after
# a pause, one line after another, and the next command after a longer one.
COMMAND_PAUSE, LINE_PAUSE, TURN_PAUSE = 0.55, 0.12, 0.65

# The mark: an R on the accent, drawn on a grid of 24. The tab's icon is the
# same drawing with the light theme's colours written in.
MARK_PATH = "M8.5 17.5V6.5h4.25a3.25 3.25 0 0 1 0 6.5H8.5M12.4 13l3.6 4.5"
MARK = (
    '<svg class="mark" viewBox="0 0 24 24" aria-hidden="true"><rect width="24" height="24" '
    f'rx="6.5"/><path d="{MARK_PATH}"/></svg>'
)
ICON = "data:image/svg+xml," + quote(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" '
    f'rx="6.5" fill="{LIGHT["accent"]}"/><path d="{MARK_PATH}" fill="none" stroke="#fff" '
    'stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round"/></svg>'
)

# The theme switch; the script shows it and names what it does.
THEME_BUTTON = (
    '<button class="icon theme" type="button" hidden aria-label="Switch the theme">'
    '<svg class="moon" viewBox="0 0 24 24" aria-hidden="true">'
    '<path d="M20 14.6A8.2 8.2 0 1 1 9.4 4a6.6 6.6 0 0 0 10.6 10.6z"/></svg>'
    '<svg class="sun" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4"/>'
    '<path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4'
    'M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"/></svg></button>'
)

# Before the first paint: the theme the reader chose on this device, if any.
THEME_SCRIPT = (
    'try { const theme = localStorage.getItem("renyi-theme"); '
    'if (theme === "light" || theme === "dark") document.documentElement.dataset.theme = theme; '
    "} catch (error) {}"
)

# After the page: each part improves a page that works without it.
SCRIPT = r"""
(() => {
  const root = document.documentElement;

  // The theme switch: light or dark, remembered on this device.
  const theme = document.querySelector(".theme");
  if (theme) {
    const system = matchMedia("(prefers-color-scheme: dark)");
    const dark = () => (root.dataset.theme || (system.matches ? "dark" : "light")) === "dark";
    const name = () => theme.setAttribute("aria-label",
      dark() ? "Use the light theme" : "Use the dark theme");
    theme.hidden = false;
    name();
    system.addEventListener("change", name);
    theme.addEventListener("click", () => {
      root.dataset.theme = dark() ? "light" : "dark";
      try { localStorage.setItem("renyi-theme", root.dataset.theme); } catch (error) {}
      name();
    });
  }

  // The menu closes on Escape and on a click outside it.
  const menu = document.querySelector(".menu");
  if (menu) {
    document.addEventListener("keydown", (event) => {
      if (event.key === "Escape" && menu.open) {
        menu.open = false;
        menu.querySelector("summary").focus();
      }
    });
    document.addEventListener("click", (event) => {
      if (menu.open && !menu.contains(event.target)) menu.open = false;
    });
  }

  // A copy button on every code block but the front page's sample.
  if (navigator.clipboard) {
    for (const block of document.querySelectorAll(".code")) {
      if (block.closest(".sample")) continue;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "copy";
      button.textContent = "Copy";
      button.addEventListener("click", async () => {
        const text = block.querySelector("code").textContent.replace(/\n$/, "");
        try {
          await navigator.clipboard.writeText(text);
          button.textContent = "Copied";
        } catch (error) {
          button.textContent = "Not copied";
        }
        setTimeout(() => { button.textContent = "Copy"; }, 1600);
      });
      block.append(button);
    }
  }

  // The contents beside a long page mark the section being read.
  const toc = document.querySelector(".toc");
  if (toc) {
    const entries = [...toc.querySelectorAll('a[href^="#"]')]
      .map((link) => [link, document.getElementById(decodeURIComponent(link.hash.slice(1)))])
      .filter(([, heading]) => heading);
    let current = null;
    let queued = false;
    const mark = () => {
      queued = false;
      let found = entries[0];
      for (const entry of entries) {
        const box = entry[1].getBoundingClientRect();
        if (box.height === 0) continue;
        if (box.top > innerHeight * 0.3) break;
        found = entry;
      }
      if (!found || found[0] === current) return;
      if (current) current.classList.remove("here");
      current = found[0];
      current.classList.add("here");
      const frame = toc.getBoundingClientRect();
      const item = current.getBoundingClientRect();
      if (item.top < frame.top || item.bottom > frame.bottom) {
        toc.scrollTop += item.top - frame.top - frame.height / 2;
      }
    };
    addEventListener("scroll", () => {
      if (!queued) {
        queued = true;
        requestAnimationFrame(mark);
      }
    }, { passive: true });
    mark();
  }

  // The terminal plays its session once it comes into view; the button plays it again.
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  if (!still && "IntersectionObserver" in window) {
    for (const term of document.querySelectorAll(".term")) {
      const play = () => {
        term.classList.remove("play");
        void term.offsetWidth;
        term.classList.add("play");
      };
      term.classList.add("armed");
      new IntersectionObserver((seen, observer) => {
        if (seen.some((entry) => entry.isIntersecting)) {
          play();
          observer.disconnect();
        }
      }, { threshold: 0.35 }).observe(term);
      const again = term.querySelector(".replay");
      again.hidden = false;
      again.addEventListener("click", play);
    }
  }
})();
"""

STYLE = r"""
:root {
  --sans: "Segoe UI Variable Text", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont,
          "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  --display: "Segoe UI Variable Display", ui-sans-serif, system-ui, -apple-system,
             BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  --mono: ui-monospace, "SF Mono", SFMono-Regular, "Cascadia Mono", "JetBrains Mono",
          "Roboto Mono", Menlo, Consolas, "Liberation Mono", "DejaVu Sans Mono", monospace;
  --measure: 44rem; --wide: 72rem; --bar: 76rem; --aside: 13.5rem; --gutter: 20px;
  --header: 3.75rem;
  --radius: 12px;
}
@media (min-width: 640px) { :root { --gutter: 32px; } }

/* The base: one sans for text, one mono for code, the accent for focus and selection. */
*, *::before, *::after { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; text-size-adjust: 100%;
       scroll-padding-top: calc(var(--header) + 1.25rem); }
@media (prefers-reduced-motion: no-preference) { html { scroll-behavior: smooth; } }
body { display: flex; flex-direction: column; min-height: 100vh; margin: 0;
       background: var(--bg); color: var(--text); font: 1rem/1.7 var(--sans);
       -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale;
       text-rendering: optimizeLegibility; overflow-wrap: break-word; }
@media (min-width: 640px) { body { font-size: 1.0625rem; } }
main { flex: 1 0 auto; }
::selection { background: var(--selection); }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
button { font: inherit; }
[hidden] { display: none !important; }
.invert { color: var(--text); }
.sr { position: absolute; width: 1px; height: 1px; margin: -1px; overflow: hidden;
      clip: rect(0 0 0 0); clip-path: inset(50%); white-space: nowrap; }
.skip { position: absolute; top: 0; left: var(--gutter); z-index: 40; padding: .65rem 1rem;
        border-radius: 10px; background: var(--accent); color: var(--on-accent);
        font-size: .9375rem; font-weight: 600; text-decoration: none;
        transform: translateY(-120%); transition: transform .15s; }
.skip:focus { transform: translateY(.75rem); }
a { color: var(--accent); text-decoration-line: underline; text-decoration-thickness: 1px;
    text-underline-offset: .22em;
    text-decoration-color: color-mix(in srgb, currentColor 35%, transparent);
    transition: color .15s, text-decoration-color .15s; }
a:hover { text-decoration-color: currentColor; }
h1, h2, h3, h4, h5, h6 { color: var(--text); font-family: var(--display); }
.in { max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto; padding: 0 var(--gutter); }

/* The header: the mark and the name, the four groups in one row from 1280px
   and a menu below that, the theme switch. */
.top { position: sticky; top: 0; z-index: 30; border-bottom: 1px solid var(--line);
       background: var(--bg); background: color-mix(in srgb, var(--bg) 82%, transparent);
       -webkit-backdrop-filter: saturate(1.8) blur(16px);
       backdrop-filter: saturate(1.8) blur(16px); }
.bar { display: flex; align-items: center; gap: 1rem; height: var(--header);
       max-width: calc(var(--bar) + 2 * var(--gutter)); margin: 0 auto; padding: 0 var(--gutter); }
.brand { display: inline-flex; flex: none; align-items: center; gap: .6rem; color: var(--text);
         font: 650 1.0625rem/1 var(--display); letter-spacing: -0.02em; text-decoration: none; }
.mark { flex: none; width: 1.625rem; height: 1.625rem; }
.mark rect { fill: var(--accent); }
.mark path { fill: none; stroke: var(--on-accent); stroke-width: 2.1; stroke-linecap: round;
             stroke-linejoin: round; }
.row { display: none; }
.tools { display: flex; align-items: center; gap: .25rem; margin-left: auto; }
.icon { display: inline-grid; place-items: center; width: 2.25rem; height: 2.25rem; padding: 0;
        border: 0; border-radius: 10px; background: none; color: var(--text-2); cursor: pointer;
        transition: color .15s, background-color .15s; }
.icon:hover { background: var(--surface); color: var(--text); }
.icon svg { width: 1.125rem; height: 1.125rem; fill: none; stroke: currentColor;
            stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; }
.theme .sun, :root[data-theme="dark"] .theme .moon { display: none; }
:root[data-theme="dark"] .theme .sun { display: block; }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) .theme .sun { display: block; }
  :root:not([data-theme="light"]) .theme .moon { display: none; }
}
.menu > summary { display: inline-flex; align-items: center; gap: .55rem; height: 2.25rem;
                  padding: 0 .8rem; border-radius: 10px; color: var(--text-2); cursor: pointer;
                  font-size: .875rem; font-weight: 550; list-style: none;
                  transition: color .15s, background-color .15s; }
.menu > summary::-webkit-details-marker { display: none; }
.menu > summary:hover, .menu[open] > summary { background: var(--surface); color: var(--text); }
.lines { width: 1rem; height: 1rem; fill: none; stroke: currentColor; stroke-width: 1.8;
         stroke-linecap: round; }
.lines path { transform-box: fill-box; transform-origin: center; transition: transform .2s; }
.menu[open] .lines path:first-child { transform: translateY(3px) rotate(45deg); }
.menu[open] .lines path:last-child { transform: translateY(-3px) rotate(-45deg); }
.menu .panel { position: absolute; top: 100%; right: 0; left: 0;
               max-height: calc(100vh - var(--header)); overflow-y: auto;
               overscroll-behavior: contain; padding: 1.75rem var(--gutter) 2.25rem;
               border-bottom: 1px solid var(--line); background: var(--bg);
               box-shadow: var(--shadow-lg); }
.menu .groups { max-width: var(--wide); margin: 0 auto; }
.menu .groups a { padding: .5rem 0; font-size: 1rem; }
@media (min-width: 1280px) {
  .menu { display: none; }
  .row { display: block; margin-left: .5rem; }
  .row > ul { display: flex; align-items: center; margin: 0; padding: 0; list-style: none; }
  .row li { display: flex; align-items: center; }
  .row li + li::before { content: ""; width: 1px; height: .875rem; margin: 0 .375rem;
                         background: var(--line-strong); }
  .row a { position: relative; display: flex; align-items: center; height: 2.25rem;
           padding: 0 .375rem; border-radius: 8px; color: var(--text-2); font-size: .8125rem;
           font-weight: 500; text-decoration: none; white-space: nowrap; transition: color .15s; }
  .row a:hover, .row a[aria-current="page"] { color: var(--text); }
  .row a[aria-current="page"]::after { content: ""; position: absolute; right: .375rem;
                                       bottom: calc((2.25rem - var(--header)) / 2 - 1px);
                                       left: .375rem; height: 2px; border-radius: 2px 2px 0 0;
                                       background: var(--accent); }
}
.ext { margin-left: .2rem; color: var(--text-3); font-size: .8em; }

/* The groups of the navigation, in the menu and in the footer. */
.groups { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1.75rem 1.5rem; }
@media (min-width: 640px) { .groups { grid-template-columns: repeat(4, minmax(0, 1fr)); } }
.groups p { margin: 0 0 .6rem; color: var(--text-3); font-size: .75rem; font-weight: 650;
            letter-spacing: .08em; text-transform: uppercase; }
.groups ul { margin: 0; padding: 0; list-style: none; }
.groups li { margin: 0; }
.groups a { display: inline-block; padding: .3rem 0; color: var(--text-2); font-size: .9375rem;
            text-decoration: none; transition: color .15s; }
.groups a:hover { color: var(--text); }
.groups a[aria-current="page"] { color: var(--accent); font-weight: 550; }

/* The documents: one measure, the group above the title, a lead under it. */
.doc { width: 100%; max-width: calc(var(--measure) + 2 * var(--gutter)); margin: 0 auto;
       padding: clamp(2.5rem, 1.5rem + 3vw, 4rem) var(--gutter) 5rem; }
.doc.wide { max-width: calc(55rem + 2 * var(--gutter)); }
.doc > :first-child, .doc article > :first-child { margin-top: 0; }
.eyebrow { margin: 0 0 .85rem; color: var(--accent); font-size: .75rem; font-weight: 650;
           letter-spacing: .09em; line-height: 1.4; text-transform: uppercase; }
.eyebrow a { color: inherit; text-decoration: none; }
.eyebrow a:hover { text-decoration: underline; }
.doc h1, .feature h1 { margin: 0 0 1rem; font-size: clamp(2rem, 1.55rem + 1.6vw, 2.75rem);
                       font-weight: 700; letter-spacing: -0.03em; line-height: 1.12;
                       text-wrap: balance; }
h1.file { font-family: var(--mono); font-weight: 650; letter-spacing: -0.035em; }
.doc h2 { margin: 3.5rem 0 1rem; padding-top: 2.25rem; border-top: 1px solid var(--line);
          font-size: clamp(1.375rem, 1.25rem + .45vw, 1.5625rem); font-weight: 650;
          letter-spacing: -0.022em; line-height: 1.25; }
.doc h3 { margin: 2.5rem 0 .75rem; font-size: 1.1875rem; font-weight: 620;
          letter-spacing: -0.014em; line-height: 1.35; }
.doc :is(h4, h5, h6) { margin: 2rem 0 .5rem; font-size: 1.0625rem; font-weight: 620;
                       letter-spacing: -0.01em; }
.doc :is(h1, .lead, .inline) + h2 { margin-top: 2.5rem; padding-top: 0; border-top: 0; }
.lead { margin: 0 0 2.5rem; color: var(--text-2);
        font-size: clamp(1.125rem, 1.05rem + .4vw, 1.3125rem); letter-spacing: -0.012em;
        line-height: 1.5; text-wrap: pretty; }
.demonstrates { margin: -1.25rem 0 2.25rem; color: var(--text-2); font-size: .9375rem;
                line-height: 1.65; }
.demonstrates span { display: block; margin-bottom: .2rem; color: var(--text-3);
                     font-size: .75rem; font-weight: 650; letter-spacing: .08em;
                     text-transform: uppercase; }
.doc :is(p, ul, ol, dl) { margin: 0 0 1.15rem; }
.doc :is(ul, ol) { padding-left: 1.4rem; }
.doc li { margin: .35rem 0; padding-left: .15rem; }
.doc li > p { margin-bottom: .5rem; }
.doc li::marker { color: var(--text-3); }
.doc ol > li::marker { font-variant-numeric: tabular-nums; font-weight: 550; }
.doc strong { color: var(--text); font-weight: 650; }
.doc hr { margin: 3rem 0; border: 0; border-top: 1px solid var(--line); }
.doc hr:has(+ :is(h2, .fold)), .fold > hr:last-child { display: none; }
.doc blockquote { margin: 1.5rem 0; padding: .85rem 1.25rem; border-left: 3px solid var(--accent);
                  border-radius: 0 10px 10px 0; background: var(--bg-soft); color: var(--text-2); }
.doc blockquote > :last-child { margin-bottom: 0; }
.doc dt { font-weight: 600; }
.doc dd { margin: 0 0 .75rem 1.25rem; }
.headerlink { margin-left: .4rem; padding: 0 .25rem; border-radius: 4px; color: var(--text-3);
              font-weight: 400; text-decoration: none; opacity: 0;
              transition: opacity .15s, color .15s; }
:is(h1, h2, h3, h4, h5, h6):hover > .headerlink, .headerlink:focus-visible { opacity: 1; }
.headerlink:hover { color: var(--accent); }

/* Code: inline on a tint of grey; a block on its own surface, highlighted, with
   its own scroll, never the page's. */
code, kbd, pre { font-family: var(--mono); font-variant-ligatures: none; }
:not(pre) > code { padding: .14em .38em; border-radius: 6px; background: var(--surface-strong);
                   color: var(--text); font-size: .86em; white-space: nowrap; }
:not(pre) > code.wrap { white-space: normal; }
.nobr { white-space: nowrap; }
a > code { background: var(--tint); color: inherit; }
td > a > code { padding: 0; background: none; }
.code { position: relative; margin: 1.5rem 0 1.75rem; border: 1px solid var(--line);
        border-radius: var(--radius); background: var(--surface); }
.code pre { margin: 0; padding: 1rem 1.25rem; overflow-x: auto; overscroll-behavior-x: contain;
            font-size: .8125rem; line-height: 1.7; tab-size: 4; scrollbar-width: thin;
            -webkit-overflow-scrolling: touch; }
.code code { font-size: inherit; white-space: pre; }
.code .lang { position: absolute; top: 0; right: .9rem; padding: .25rem .5rem;
              border: 1px solid var(--line); border-radius: 6px; background: var(--bg);
              color: var(--text-3); font: 650 .625rem/1 var(--sans); letter-spacing: .08em;
              text-transform: uppercase; pointer-events: none; transform: translateY(-50%);
              transition: opacity .15s; }
.copy { position: absolute; top: .45rem; right: .45rem; padding: .4rem .65rem;
        border: 1px solid var(--line-strong); border-radius: 8px; background: var(--bg);
        color: var(--text-2); font: 550 .75rem/1 var(--sans); cursor: pointer; opacity: 0;
        box-shadow: var(--shadow-sm); transition: opacity .15s, color .15s, border-color .15s; }
.code:hover .copy, .copy:focus-visible { opacity: 1; }
.code:is(:hover, :focus-within) .lang { opacity: 0; }
.copy:hover { border-color: var(--text-3); color: var(--text); }
@media (hover: none) {
  .copy { display: none; }
  .commands .copy { display: block; }
}
pre .k { color: var(--syn-keyword); }
pre .t { color: var(--syn-type); }
pre .s { color: var(--syn-string); }
pre .h { color: var(--text); }
pre .n { color: var(--syn-number); }
pre .c { color: var(--syn-comment); }
pre .d { color: var(--syn-doc); }
pre .f { font-weight: 650; }
pre .anchor { color: var(--accent); font-weight: 650; }
.code-head { display: flex; align-items: center; min-height: 2.6rem;
             padding: .5rem 5.5rem .5rem 1.25rem; border-bottom: 1px solid var(--line);
             color: var(--text-2); font: 550 .8125rem/1.4 var(--mono); }
.numbered code { counter-reset: line; }
.numbered .line::before { content: counter(line); counter-increment: line; display: inline-block;
                          width: 3ch; margin-right: 2.5ch; color: var(--text-3);
                          text-align: right; }
@media (max-width: 639px) {
  .code pre { padding-inline: 1rem; font-size: .78125rem; }
  .numbered .line::before { width: 2ch; margin-right: 1.5ch; }
}

/* Tables: inside a container that scrolls on a narrow screen. */
.table { margin: 1.75rem 0 2rem; overflow-x: auto; border: 1px solid var(--line);
         border-radius: var(--radius); scrollbar-width: thin; -webkit-overflow-scrolling: touch; }
table { width: 100%; border-collapse: collapse; font-size: .90625rem; line-height: 1.55; }
td.num { font-variant-numeric: tabular-nums; white-space: nowrap; }
th, td { padding: .7rem 1rem; border-bottom: 1px solid var(--line); text-align: left;
         vertical-align: top; }
tbody tr:last-child > * { border-bottom: 0; }
th { background: var(--bg-soft); color: var(--text-2); font-size: .8125rem; font-weight: 600;
     white-space: nowrap; }
>>>>

@media (max-width: 639px) { th, td { min-width: 9rem; } }

/* The corpus index: a card for each program, its use case and what it shows. */
.corpus { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 20rem), 1fr));
          gap: .75rem; margin: 2rem 0 2.5rem; padding: 0; list-style: none; }
.doc .corpus { padding: 0; }
.doc .corpus li { margin: 0; padding: 0; }
.corpus a { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-content: start;
            align-items: center; gap: .5rem .75rem; height: 100%; padding: 1rem 1.15rem 1.1rem;
            border: 1px solid var(--line); border-radius: var(--radius);
            background: var(--raised); color: var(--text-2); text-decoration: none;
            transition: border-color .15s, box-shadow .15s; }
.corpus a:hover { border-color: var(--line-strong); box-shadow: var(--shadow-md); }
.corpus .name { overflow-wrap: anywhere; color: var(--accent);
                font: 600 .9375rem/1.4 var(--mono); }
.corpus .use { padding: .15rem .55rem; border-radius: 999px; background: var(--surface-strong);
               color: var(--text-2); font-size: .75rem; font-weight: 550; line-height: 1.5;
               white-space: nowrap; }
.corpus .what { grid-column: 1 / -1; font-size: .875rem; line-height: 1.6; }

/* The sections of a long page: a disclosure under the lead, a sticky column
   from 1100px, where the section being read is marked. */
.contents ol { margin: 0; padding: 0; list-style: none; }
.contents li { margin: 0; padding: 0; }
.contents a { display: block; margin-left: -1px; padding: .3rem 0 .3rem 1rem;
              border-left: 1px solid transparent; color: var(--text-2); line-height: 1.4;
              text-decoration: none; transition: color .15s, border-color .15s; }
.contents ol ol a { padding-left: 1.75rem; }
.contents a:hover { color: var(--text); }
.contents a.here { border-left-color: var(--accent); color: var(--accent); }
.contents code { padding: 0; background: none; font-size: .9em; }
.inline { margin: 0 0 2.75rem; border: 1px solid var(--line); border-radius: var(--radius);
          background: var(--bg-soft); font-size: .9375rem; }
.inline > summary { display: flex; align-items: center; justify-content: space-between; gap: 1rem;
                    padding: .85rem 1.1rem; color: var(--text); cursor: pointer;
                    font-size: .875rem; font-weight: 600; list-style: none; }
.inline > summary::-webkit-details-marker { display: none; }
.inline > summary::after { content: ""; width: .45rem; height: .45rem; margin-top: -.2rem;
                           border-right: 1.75px solid var(--text-3);
                           border-bottom: 1.75px solid var(--text-3); transform: rotate(45deg);
                           transition: transform .2s; }
.inline[open] > summary::after { margin-top: .2rem; transform: rotate(-135deg); }
.inline > ol { max-height: 60vh; overflow-y: auto; padding: 0 1.1rem 1rem; }
.inline a { margin-left: 0; padding-left: 0; border-left: 0; }
.inline ol ol a { padding-left: .9rem; }
.toc { display: none; }
.toc p { margin: 0 0 .75rem; padding-left: 1rem; color: var(--text-3); font-size: .75rem;
         font-weight: 650; letter-spacing: .08em; text-transform: uppercase; }
@media (min-width: 1100px) {
  .doc.paged { display: grid; grid-template-columns: minmax(0, var(--measure)) var(--aside);
               column-gap: 4.5rem; align-items: start;
               max-width: calc(var(--measure) + var(--aside) + 4.5rem + 2 * var(--gutter)); }
  .inline { display: none; }
  .toc { display: block; position: sticky; top: calc(var(--header) + 2rem);
         max-height: calc(100vh - var(--header) - 4rem); overflow-y: auto;
         overscroll-behavior: contain; font-size: .8125rem; scrollbar-width: thin; }
  .toc > ol { border-left: 1px solid var(--line); }
}

/* The way on: the pages before and after this one, and its source. */
.pager { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1rem;
         margin: 4.5rem 0 0; }
.pager a { display: block; padding: 1rem 1.25rem; border: 1px solid var(--line);
           border-radius: var(--radius); color: var(--text); text-decoration: none;
           transition: border-color .15s, background-color .15s; }
.pager a:hover { border-color: var(--line-strong); background: var(--bg-soft); }
.pager span { display: block; color: var(--text-3); font-size: .75rem; font-weight: 650;
              letter-spacing: .08em; text-transform: uppercase; }
.pager strong { display: block; margin-top: .25rem; font-weight: 600; overflow-wrap: anywhere; }
.pager .file { font-family: var(--mono); font-size: .9em; }
.pager .prev span::before { content: "\2190\00a0"; }
.pager .next { grid-column: 2; text-align: right; }
.pager .next span::after { content: "\00a0\2192"; }
.source { margin: 1.5rem 0 0; color: var(--text-3); font-size: .875rem; }
.source a { color: var(--text-2); }

/* The decision record: each lettered section folds; find-in-page opens it. */
.fold { border-top: 1px solid var(--line); }
.fold:last-of-type { margin-bottom: 2rem; border-bottom: 1px solid var(--line); }
.fold > summary { display: flex; align-items: center; gap: .85rem; margin: 0 -.75rem;
                  padding: .95rem .75rem; border-radius: 10px; cursor: pointer; list-style: none;
                  transition: background-color .15s; }
.fold > summary::-webkit-details-marker { display: none; }
.fold > summary::before { content: ""; flex: none; width: .45rem; height: .45rem;
                          border-right: 1.75px solid var(--text-3);
                          border-bottom: 1.75px solid var(--text-3); transform: rotate(-45deg);
                          transition: transform .2s; }
.fold[open] > summary::before { transform: rotate(45deg); }
.fold > summary:hover { background: var(--bg-soft); }
.fold > summary h2 { margin: 0; padding: 0; border: 0; font-size: 1.125rem; font-weight: 620;
                     letter-spacing: -0.014em; }
.fold[open] { padding-bottom: 1.5rem; }

/* Diagrams: inline SVG in the theme's colours, one type scale, one stroke
   weight and one family of radii across every drawing. */
figure { margin: 2rem 0 2.5rem; }
figcaption { margin-top: 1rem; color: var(--text-2); font-size: .875rem; line-height: 1.55;
             text-align: center; text-wrap: balance; }
.dg { display: block; width: 100%; height: auto; overflow: visible; }
.dg text { fill: var(--text); font-family: var(--sans); font-size: 13px; }
.dg .mono { font-family: var(--mono); font-size: 12.5px; }
.dg .small { font-size: 12px; }
.dg .tiny { font-size: 11px; }
.dg .label { fill: var(--text-3); font-size: 10.5px; font-weight: 650; letter-spacing: .08em; }
.dg .strong { font-weight: 650; }
.dg .muted { fill: var(--text-2); }
.dg .accent { fill: var(--accent); }
.dg .box { fill: var(--surface); stroke: var(--line-strong); }
.dg .chip { fill: var(--bg); stroke: var(--line-strong); }
.dg .frame { fill: none; stroke: var(--line-strong); }
.dg .dash { fill: none; stroke: var(--text-3); stroke-dasharray: 3 4; }
.dg .tint { fill: var(--tint); }
.dg .tint-strong { fill: var(--tint-strong); }
.dg .hot { fill: var(--tint); stroke: color-mix(in srgb, var(--accent) 50%, transparent); }
.dg .rail { fill: var(--accent); }
.dg .flow { fill: none; stroke: var(--accent); stroke-width: 1.5; stroke-linecap: round;
            stroke-linejoin: round; }
.dg .seq { fill: none; stroke: var(--text-3); stroke-width: 1.25; stroke-linecap: round;
           stroke-linejoin: round; }
.dg .head { fill: var(--accent); }
.dg .head-seq { fill: var(--text-3); }
.dg .ok { fill: none; stroke: var(--accent); stroke-width: 2; stroke-linecap: round;
          stroke-linejoin: round; }
.dg .no { fill: none; stroke: var(--text); stroke-width: 2; stroke-linecap: round; }
.dg.mini text, .dg.mini .mono { font-size: 11px; }
.dg.mini .tiny { font-size: 10px; }
.dg.mini .label { font-size: 9.5px; }
@keyframes dg-in { from { opacity: 0; } }
@supports (animation-timeline: view()) {
  .dg { view-timeline: --dg block; }
  .dg :is(.p1, .p2, .p3) { animation: dg-in linear both; animation-timeline: --dg; }
  .dg .p1 { animation-range: entry 10% entry 55%; }
  .dg .p2 { animation-range: entry 35% entry 80%; }
  .dg .p3 { animation-range: entry 60% entry 100%; }
}

/* The terminal: a real session, dark in both themes; its lines appear in turn. */
.term { margin: 0; overflow: hidden; border: 1px solid var(--line-strong); border-radius: 14px;
        background: var(--surface); box-shadow: var(--shadow-lg); }
.term-bar { display: grid; grid-template-columns: 1fr auto 1fr; align-items: center;
            height: 2.5rem; padding: 0 .75rem 0 .95rem; border-bottom: 1px solid var(--line);
            background: var(--bg-soft); }
.dots { display: flex; gap: .45rem; }
.dots i { width: .65rem; height: .65rem; border-radius: 50%; background: var(--line-strong); }
.term-title { color: var(--text-3); font: 550 .75rem/1 var(--sans); }
.replay { justify-self: end; padding: .35rem .6rem; border: 1px solid var(--line-strong);
          border-radius: 7px; background: none; color: var(--text-2); cursor: pointer;
          font: 550 .75rem/1 var(--sans); transition: color .15s, border-color .15s; }
.replay:hover { border-color: var(--text-3); color: var(--text); }
.term pre { margin: 0; padding: 1.1rem 1.2rem 1.3rem; color: var(--text-2); font-size: .8125rem;
            line-height: 1.7; white-space: pre-wrap; overflow-wrap: anywhere; }
.term code { font-size: inherit; }
.term .ln { display: block; padding-left: 2ch; text-indent: -2ch; }
.term .ln.i2 { padding-left: 4ch; text-indent: -4ch; }
.term .cmd { color: var(--text); }
.term .ps { color: var(--accent); user-select: none; }
.term .fix { color: var(--accent); }
.term .err { color: var(--danger); font-weight: 600; }
.cursor { display: inline-block; width: .55em; height: 1.15em; margin-left: .1em;
          background: var(--text-2); vertical-align: -.22em; }
@media (prefers-reduced-motion: no-preference) {
  .term.armed:not(.play) .ln { opacity: 0; }
  .term.play .ln { animation: term-in .35s ease-out var(--at, 0s) both; }
  .cursor { animation: blink 1.1s steps(1) infinite; }
}
@keyframes term-in { from { opacity: 0; transform: translateY(3px); } }
@keyframes blink { 50% { opacity: 0; } }

/* The front page: sections across the whole width, their content in `.in`. */
.home section { padding: clamp(4.5rem, 2.5rem + 6vw, 8rem) 0; }
.home h2 { margin: 0 0 1.25rem; font-size: clamp(1.875rem, 1.25rem + 2.4vw, 3rem);
           font-weight: 700; letter-spacing: -0.038em; line-height: 1.08; text-wrap: balance; }
.home .promises, .home .read { padding-top: 0; }
.promises h2 { max-width: 20ch; }
.home .hero { position: relative; isolation: isolate; overflow: clip;
              padding: clamp(3.5rem, 1.5rem + 6vw, 7rem) 0 clamp(4rem, 2rem + 6vw, 7.5rem); }
.hero::before { content: ""; position: absolute; inset: 0; z-index: -1;
                background-image: linear-gradient(var(--grid) 1px, transparent 1px),
                                  linear-gradient(90deg, var(--grid) 1px, transparent 1px);
                background-position: center -1px; background-size: 48px 48px;
                -webkit-mask-image: radial-gradient(ellipse 80% 70% at 72% 32%, #000 8%,
                                                    transparent 72%);
                mask-image: radial-gradient(ellipse 80% 70% at 72% 32%, #000 8%, transparent 72%); }
.hero::after { content: ""; position: absolute; top: -14rem; right: -12rem; z-index: -1;
               width: 56rem; height: 44rem;
               background: radial-gradient(closest-side, var(--glow), transparent); }
.hero .in { display: grid; grid-template-columns: minmax(0, 1fr); gap: 3.5rem;
            align-items: center; }
.hero h1 { margin: 0 0 1.5rem; font-size: clamp(2.75rem, 1.5rem + 4.4vw, 4.75rem);
           font-weight: 700; letter-spacing: -0.045em; line-height: 1.02; text-wrap: balance; }
.tagline { max-width: 32rem; margin: 0 0 2.25rem; color: var(--text-2);
           font-size: clamp(1.125rem, 1rem + .5vw, 1.3125rem); letter-spacing: -0.012em;
           line-height: 1.55; }
.actions { display: flex; flex-wrap: wrap; gap: .75rem; margin: 0; }
.button { display: inline-flex; align-items: center; justify-content: center; height: 2.75rem;
          padding: 0 1.25rem; border: 1px solid var(--line-strong); border-radius: 10px;
          background: var(--bg); box-shadow: var(--shadow-sm); color: var(--text);
          font-size: .9375rem; font-weight: 600; text-decoration: none;
          transition: background-color .15s, border-color .15s, color .15s; }
.button:hover { border-color: var(--text-3); }
.button.primary { border-color: var(--accent); background: var(--accent); color: var(--on-accent); }
.button.primary:hover { border-color: var(--accent-strong); background: var(--accent-strong); }

/* The sample: the three lines of the signature, each with what it says. */
.sample { margin: 0; }
.sample h2 { margin: 0 0 1rem; font-size: clamp(1.0625rem, 1rem + .3vw, 1.1875rem);
             font-weight: 600; letter-spacing: -0.012em; line-height: 1.4; }
.card { position: relative; }
.sample .code { margin: 0; border-radius: 16px; background: var(--raised);
                box-shadow: var(--shadow-lg); }
.sample pre { padding: 1.35rem 0; overflow: visible; font-size: .75rem; line-height: 1.75; }
.sample code { white-space: pre-wrap; }
.sample .line { display: block; padding: 0 1.1rem 0 calc(1.1rem + 4ch); text-indent: -4ch; }
.sample .line:is(:nth-child(2), :nth-child(3), :nth-child(4)) {
  background: var(--tint); box-shadow: inset 3px 0 0 var(--accent);
  animation: sig-in .7s ease-out .35s both;
}
.sample .line:nth-child(3) { animation-delay: .85s; }
.sample .line:nth-child(4) { background: var(--tint-strong); animation-delay: 1.35s; }
@keyframes sig-in { from { background-color: transparent; box-shadow: inset 0 0 0 var(--accent); } }
.notes { display: none; }
@media (min-width: 640px) {
  .sample pre { font-size: .8125rem; }
  .sample .line { padding: 0 1.4rem 0 calc(1.4rem + 4ch); }
  .notes { display: block; position: absolute; top: calc(1.35rem + .8125rem * 1.75); right: 1.25rem;
           margin: 0; padding: 0; list-style: none; text-align: right; pointer-events: none; }
  .notes li { height: calc(.8125rem * 1.75); margin: 0; color: var(--text-2);
              font: 500 .75rem/calc(.8125rem * 1.75) var(--sans);
              animation: notes-in .7s ease-out .35s both; }
  .notes li:nth-child(2) { animation-delay: .85s; }
  .notes li:last-child { color: var(--accent); font-weight: 650; animation-delay: 1.35s; }
}
@keyframes notes-in { from { opacity: 0; } }

/* The four points: a card each, its number, its title, one line and a drawing. */
.points { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1rem; margin: 3rem 0 0; }
.point { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1.5rem; align-content: start;
         padding: 1.75rem; border: 1px solid var(--line); border-radius: 18px;
         background: var(--bg-soft); }
.point .num { display: block; margin: 0 0 1rem; color: var(--accent);
              font: 600 .8125rem/1 var(--mono); }
.point h3 { margin: 0 0 .5rem; font-size: 1.25rem; font-weight: 650; letter-spacing: -0.02em;
            line-height: 1.3; }
.point p { margin: 0; color: var(--text-2); font-size: .96875rem; line-height: 1.6; }
.point .dg { max-width: 20rem; }
.more { margin: 2.5rem 0 0; font-weight: 600; }
.more a { text-decoration: none; }
.more a::after { content: "\00a0\2192"; display: inline-block; transition: transform .2s; }
.more a:hover::after { transform: translateX(3px); }

/* The gap and the session that closes it, on a band dark in both themes. */
.band { position: relative; isolation: isolate; overflow: clip; border-block: 1px solid var(--line);
        background: var(--band); }
.band::before { content: ""; position: absolute; top: 0; right: -20%; bottom: 0; z-index: -1;
                width: 80%; background: radial-gradient(closest-side, var(--glow), transparent); }
.band .in { display: grid; grid-template-columns: minmax(0, 1fr); gap: 3rem; align-items: center; }
.band h2 { margin: 0; }
.then { margin: 1.5rem 0 0; color: var(--text-2); font-size: clamp(1.25rem, 1rem + 1vw, 1.75rem);
        font-weight: 550; letter-spacing: -0.025em; line-height: 1.3; text-wrap: balance; }

/* Install: three commands, a label each, a copy button each. */
.install .in { display: grid; grid-template-columns: minmax(0, 1fr); gap: 2rem; }
.install p { max-width: 26rem; margin: 0; color: var(--text-2); }
.install p + p { margin-top: 1rem; }
.commands { display: grid; grid-template-columns: minmax(0, 1fr); gap: 1.25rem; margin: 0; }
.commands dt { margin: 0 0 .5rem; color: var(--text-2); font-size: .8125rem; font-weight: 600; }
.commands dd { margin: 0; }
.commands .code { margin: 0; }
.commands .lang { display: none; }
.commands pre { padding-right: 5rem; }
.commands code { white-space: pre-wrap; overflow-wrap: anywhere; }
.commands .copy { top: 50%; opacity: 1; transform: translateY(-50%); }
@media (max-width: 639px) {
  .commands pre { padding-right: 1.25rem; font-size: .75rem; }
  .commands .copy { top: -2rem; right: 0; transform: none; }
}

/* The links: a card each, its name and what it holds. */
.links { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 18rem), 1fr));
         gap: 1rem; margin: 0; padding: 0; list-style: none; }
.links li { margin: 0; }
.links a { display: flex; flex-direction: column; gap: .4rem; height: 100%; padding: 1.35rem 1.4rem;
           border: 1px solid var(--line); border-radius: 16px; background: var(--raised);
           color: var(--text); text-decoration: none;
           transition: border-color .2s, box-shadow .2s, transform .2s; }
.links a:hover { border-color: var(--line-strong); box-shadow: var(--shadow-md);
                 transform: translateY(-2px); }
.links strong { display: flex; justify-content: space-between; gap: 1rem; font-size: 1.0625rem;
                font-weight: 650; letter-spacing: -0.015em; }
.links strong::after { content: "\2192"; color: var(--text-3);
                       transition: color .2s, transform .2s; }
.links a:hover strong::after { color: var(--accent); transform: translateX(3px); }
.links span { color: var(--text-2); font-size: .9375rem; line-height: 1.55; }

/* A card sets its drawing beside its text where it is wide: alone on its row
   from 640px, two to a row from 1280px. */
@media (min-width: 640px) and (max-width: 1023px), (min-width: 1280px) {
  .point { grid-template-columns: minmax(0, 1fr) 16.25rem; align-items: center; gap: 2rem; }
}
@media (min-width: 1024px) {
  .points { grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 1.25rem; }
  .install .in { grid-template-columns: minmax(0, 3fr) minmax(0, 7fr); gap: 3rem;
                 align-items: start; }
}
@media (min-width: 1100px) {
  .hero .in, .band .in { grid-template-columns: minmax(0, 5fr) minmax(0, 6fr); gap: 4.5rem; }
}

/* How it works: each mechanism beside its drawing from 1100px, one session above. */
.feature { width: 100%; max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto;
           padding: clamp(2.5rem, 1.5rem + 3vw, 4rem) var(--gutter) 5rem; }
.feature .lead { max-width: 44rem; }
.chapters ol { display: flex; flex-wrap: wrap; gap: .5rem; margin: 0 0 1rem; padding: 0;
               list-style: none; }
.chapters a { display: inline-flex; align-items: center; gap: .5rem; padding: .45rem .85rem;
              border: 1px solid var(--line); border-radius: 999px; background: var(--bg);
              color: var(--text-2); font-size: .875rem; font-weight: 550; text-decoration: none;
              transition: color .15s, border-color .15s; }
.chapters a:hover { border-color: var(--line-strong); color: var(--text); }
.chapters span { color: var(--accent); font: 600 .75rem/1 var(--mono); }
.feature section { display: grid; grid-template-columns: minmax(0, 1fr); gap: 2rem 4rem;
                   padding: clamp(3.5rem, 2rem + 4vw, 5.5rem) 0;
                   border-top: 1px solid var(--line); }
.feature h2 { margin: 0 0 1rem; font-size: clamp(1.5rem, 1.2rem + 1.2vw, 2.125rem);
              font-weight: 700; letter-spacing: -0.032em; line-height: 1.15; text-wrap: balance; }
.feature .num { display: block; margin: 0 0 .9rem; color: var(--accent);
                font: 600 .8125rem/1 var(--mono); }
.feature .num + h2 { scroll-margin-top: calc(var(--header) + 3.25rem); }
.feature .prose { max-width: 40rem; }
.feature .prose p { margin: 0 0 1.15rem; }
.feature figure { margin: 0; }
.feature .code { margin: 0; }
.feature .dg { max-width: 33rem; margin: 0 auto; }
@media (max-width: 639px) {
  .feature .dg { width: calc(100% + var(--gutter)); margin-inline: calc(-.5 * var(--gutter)); }
}
@media (min-width: 1100px) {
  .feature .mechanism { grid-template-columns: minmax(0, 1fr) minmax(0, 1.08fr);
                        align-items: start; }
  .mechanism .prose { position: sticky; top: calc(var(--header) + 2.5rem); }
  .feature .session { grid-template-columns: minmax(0, 6fr) minmax(0, 5fr); column-gap: 2.5rem;
                      align-items: start; }
  .session .prose { grid-column: 1 / -1; }
}

/* The footer: the mark and the one line, the groups, the build. */
.foot { border-top: 1px solid var(--line); background: var(--bg-soft); color: var(--text-2);
        font-size: .875rem; }
.foot-main { display: grid; grid-template-columns: minmax(0, 1fr); gap: 2.5rem;
             padding: 3.5rem 0 3rem; }
.foot-brand p { max-width: 22rem; margin: 1rem 0 0; line-height: 1.6; }
.meta { display: flex; flex-wrap: wrap; justify-content: space-between; gap: .5rem 1.5rem;
        padding: 1.5rem 0 2.5rem; border-top: 1px solid var(--line); color: var(--text-3);
        font-size: .8125rem; }
.meta a { color: var(--text-2); }
@media (min-width: 960px) {
  .foot-main { grid-template-columns: minmax(0, 4fr) minmax(0, 8fr); gap: 4rem; }
}

/* Motion is a courtesy: without it everything is shown at once, highlighted. */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after { animation: none !important; transition: none !important; }
}

/* Print: the text, without the navigation. */
@media print {
  .top, .toc, .inline, .pager, .foot, .skip, .copy, .headerlink, .replay {
    display: none !important;
  }
  body { display: block; background: #fff; color: #000; }
  .doc.paged { display: block; }
  .code, .table { break-inside: avoid; }
  .code code { white-space: pre-wrap; }
}
"""


def custom_properties(palette: dict[str, str]) -> str:
    return "".join(f"\n  --{name}: {value};" for name, value in palette.items())


def theme_css() -> str:
    """The two themes: the light one by default; the dark one when the system
    asks for it and the reader has not chosen, or when the reader chose it.
    An `.invert` element takes the dark one in both."""
    light, dark = custom_properties(LIGHT), custom_properties(DARK)
    return (
        f":root {{ color-scheme: light;{light}\n}}\n"
        "@media (prefers-color-scheme: dark) {\n"
        f':root:not([data-theme="light"]) {{ color-scheme: dark;{dark}\n}}\n'
        "}\n"
        f':root[data-theme="dark"], .invert {{ color-scheme: dark;{dark}\n}}\n'
    )


# Highlighting. Each highlighter turns source text into its lines as HTML,
# every token in a span whose class names its kind: k a reserved word, t a
# type, s text, h a hole in text, n a number, c a comment, d the text of a
# documentation clause, f the name a definition introduces.


@functools.cache
def reserved_words() -> frozenset[str]:
    """The reserved words of Renyi: the first block of reference section 17."""
    text = (ROOT / "docs/reference.md").read_text(encoding="utf-8")
    found = re.search(r"^## 17\..*?^```\n(.*?)^```", text, re.S | re.M)
    if not found:
        raise SystemExit("site.py: no list of reserved words in section 17 of the reference")
    return frozenset(found.group(1).split())


def span(kind: str, text: str) -> str:
    escaped = html.escape(text, quote=False)
    return f'<span class="{kind}">{escaped}</span>' if text else ""


# A Renyi line, token by token (reference section 1).
RENYI_TOKEN = re.compile(
    r"(?P<comment>#.*)"
    r'|(?P<raw>\braw "[^"]*"?)'
    r'|(?P<text>"(?:\\.|[^"\\])*"?)'
    r"|(?P<number>\b[0-9][0-9_]*(?:\.[0-9]+)?\b)"
    r"|(?P<word>[A-Za-z_][A-Za-z0-9_]*)"
    r'|(?P<other>[^#"A-Za-z0-9_]+|.)'
)
# A documentation clause: the rest of its line is prose (reference 1.6).
CLAUSE = re.compile(r"( *)(purpose|tags|see also|deprecated):(.*)")
# A hole in a text literal; a brace after a backslash is a literal brace.
HOLE = re.compile(r"(?<!\\)\{[^{}]*\}")


def renyi_text(literal: str) -> str:
    """A text literal: its holes in the text colour, the rest as text."""
    out, last = [], 0
    for found in HOLE.finditer(literal):
        out += [span("s", literal[last:found.start()]), span("h", found.group())]
        last = found.end()
    return "".join(out) + span("s", literal[last:])


def renyi_line(line: str) -> str:
    out, previous = [], ""
    for found in RENYI_TOKEN.finditer(line):
        kind, token = found.lastgroup, found.group()
        if kind == "comment":
            out.append(span("c", token))
        elif kind == "raw":
            out.append(span("k", "raw") + span("s", token[3:]))
        elif kind == "text":
            out.append(renyi_text(token))
        elif kind == "number":
            out.append(span("n", token))
        elif kind == "word":
            if line[found.start() - 1:found.start()] == ".":
                out.append(token)  # a member: any word directly after a dot
            elif previous == "function":
                out.append(span("f", token))
            elif token in reserved_words():
                out.append(span("k", token))
            elif token[0].isupper():
                out.append(span("t", token))
            else:
                out.append(token)
        else:
            out.append(html.escape(token, quote=False))
        if token.strip():
            previous = token.strip()
    return "".join(out)


def highlight_renyi(source: str) -> list[str]:
    lines, in_block = [], False
    for line in source.split("\n"):
        if in_block:  # block text, up to the line that is its closing quotes
            lines.append(span("s", line))
            in_block = line.strip() != '"""'
        elif clause := CLAUSE.fullmatch(line):
            indent, word, rest = clause.groups()
            lines.append(indent + span("k", word + ":") + span("d", rest))
        elif line.rstrip().endswith('"""'):
            head = line.rstrip()[:-3]
            lines.append(renyi_line(head) + span("s", '"""'))
            in_block = True
        else:
            lines.append(renyi_line(line))
    return lines


# Blocks the documents leave untagged are Renyi when their first line of code
# starts like a Renyi item or statement, or when reserved words are a quarter
# of their words outside text and comments (the cheat sheet's expressions);
# the rest (a table of capabilities, a narrated run) is shown plain.
RENYI_STARTS = frozenset(
    "ability change for function if import is let match module public repeat return run test "
    "type".split()
)


def looks_like_renyi(source: str) -> bool:
    code = [line for line in source.split("\n")
            if line.strip() and not line.lstrip().startswith("#")]
    if not code:
        return False
    if code[0].split()[0] in RENYI_STARTS:
        return True
    bare = re.sub(r'"[^"\n]*"|#.*', " ", "\n".join(code))
    words = re.findall(r"(?<![.\w])[a-z_]+\b(?!\()", bare)
    reserved = sum(word in reserved_words() for word in words)
    return reserved >= 3 and reserved * 4 >= len(words)


# The grammar, token by token: comments, quoted terminals, the definition
# sign, names and the operators of W3C EBNF.
EBNF_TOKEN = re.compile(
    r"(?P<comment>/\*.*?(?:\*/|$))"
    r"|(?P<terminal>'[^']*'?)"
    r"|(?P<define>::=)"
    r"|(?P<name>[A-Za-z][A-Za-z0-9]*)"
    r"|(?P<operator>[|?*+()\-])"
    r"|(?P<other>[^/'A-Za-z:|?*+()\-]+|.)"
)


def highlight_ebnf(source: str) -> list[str]:
    lines, in_comment = [], False
    for line in source.split("\n"):
        if in_comment:  # a comment that spans lines, up to its end
            end = line.find("*/")
            if end < 0:
                lines.append(span("c", line))
                continue
            head, line, in_comment = line[:end + 2], line[end + 2:], False
        else:
            head = ""
        out = [span("c", head)]
        for found in EBNF_TOKEN.finditer(line):
            kind, token = found.lastgroup, found.group()
            if kind == "comment":
                out.append(span("c", token))
                in_comment = not token.endswith("*/")
            elif kind == "terminal":
                out.append(span("s", token))
            elif kind in ("define", "operator"):
                out.append(span("k", token))
            elif kind == "name" and found.start() == 0:
                out.append(span("f", token))  # the rule this line defines
            else:
                out.append(html.escape(token, quote=False))
        lines.append("".join(out))
    return lines


# The other languages the documents quote: the marker of a comment, whether
# single quotes make text, the reserved words.
LEXICONS = {
    "rust": ("//", False, frozenset(
        "as async await break const continue crate dyn else enum extern false fn for if impl in "
        "let loop match mod move mut pub ref return self Self static struct super trait true type "
        "unsafe use where while".split())),
    "python": ("#", True, frozenset(
        "and as assert async await break class continue def del elif else except False finally "
        "for from global if import in is lambda None nonlocal not or pass raise return True try "
        "while with yield".split())),
    "json": ("", False, frozenset({"true", "false", "null"})),
    "toml": ("#", True, frozenset({"true", "false"})),
    "sh": ("#", True, frozenset()),
    "powershell": ("#", True, frozenset()),
}
LANGUAGE_NAMES = {"ebnf": "EBNF", "json": "JSON", "powershell": "PowerShell", "python": "Python",
                  "rust": "Rust", "sh": "Shell", "toml": "TOML"}


@functools.cache
def other_token(language: str) -> re.Pattern[str]:
    comment, single_quotes, _ = LEXICONS[language]
    parts = [rf"(?P<comment>(?<!\S){re.escape(comment)}.*)"] if comment else []
    parts.append(r'(?P<text>"(?:\\.|[^"\\])*"?)')
    if single_quotes:
        parts.append(r"(?P<quoted>'[^'\n]*'?)")
    parts += [r"(?P<number>\b[0-9][0-9_.]*\b)", r"(?P<word>[A-Za-z_][A-Za-z0-9_]*)",
              r"(?P<other>.)"]
    return re.compile("|".join(parts))


def highlight_other(language: str, source: str) -> list[str]:
    keywords, lines = LEXICONS[language][2], []
    for line in source.split("\n"):
        out, first = [], True
        for found in other_token(language).finditer(line):
            kind, token = found.lastgroup, found.group()
            if kind in ("text", "quoted"):
                key = language == "json" and line[found.end():].lstrip().startswith(":")
                out.append(span("t" if key else "s", token))
            elif kind == "comment":
                out.append(span("c", token))
            elif kind == "number":
                out.append(span("n", token))
            elif kind == "word" and token in keywords:
                out.append(span("k", token))
            elif kind == "word" and first and language in ("sh", "powershell"):
                out.append(span("f", token))  # the command
            else:
                out.append(html.escape(token, quote=False))
            first = first and not token.strip()
        lines.append("".join(out))
    return lines


def highlight(language: str, source: str) -> list[str]:
    """The lines of a block as HTML, each token in a span of its kind."""
    if language == "renyi":
        return highlight_renyi(source)
    if language == "ebnf":
        return highlight_ebnf(source)
    if language in LEXICONS:
        return highlight_other(language, source)
    return [html.escape(line, quote=False) for line in source.split("\n")]


def code_block(lines: list[str], language: str, *, numbered: bool = False,
               marks: dict[int, str] | None = None, head: str = "") -> str:
    """A block of code: each line in a span of its own that ends with its line
    break, the language named unless it is Renyi, the site's own; a marked
    line is an anchor (the grammar's sections)."""
    marks = marks or {}
    rows = []
    for number, line in enumerate(lines):
        if number in marks:
            rows.append(f'<span class="line anchor" id="{marks[number]}">{line}\n</span>')
        else:
            rows.append(f'<span class="line">{line}\n</span>')
    kind = "code numbered" if numbered else "code"
    name = LANGUAGE_NAMES.get(language)
    label = f'<span class="lang">{name}</span>' if name else ""
    return f'<div class="{kind}">{head}{label}<pre><code>{"".join(rows)}</code></pre></div>'


CODE_BLOCK = re.compile(r'<pre><code(?: class="language-([\w-]+)")?>(.*?)</code></pre>', re.S)


def highlight_blocks(body: str) -> str:
    """Every code block of a rendered page, highlighted in its language."""

    def dressed(found: re.Match[str]) -> str:
        language = found.group(1) or ""
        source = html.unescape(found.group(2)).rstrip("\n")
        if not language and looks_like_renyi(source):
            language = "renyi"
        return code_block(highlight(language, source), language)

    return CODE_BLOCK.sub(dressed, body)


# A date the documents write, which never breaks at its hyphens.
DATE = re.compile(r"\b([0-9]{4}-[0-9]{2}-[0-9]{2})\b")


def keep_dates_whole(body: str) -> str:
    """Every date in the text of a page (never in a tag) kept on one line."""
    parts = re.split(r"(<[^>]+>)", body)
    return "".join(part if part.startswith("<") else DATE.sub(r'<span class="nobr">\1</span>', part)
                   for part in parts)


def mark_long_code(body: str) -> str:
    """An inline code span longer than WRAP_FROM characters may break."""

    def marked(found: re.Match[str]) -> str:
        if len(html.unescape(found.group(1))) <= WRAP_FROM:
            return found.group(0)
        return f'<code class="wrap">{found.group(1)}</code>'

    return re.sub(r"<code>([^<]*)</code>", marked, body)


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
    found[CORPUS] = "examples/index.html"
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


@functools.cache
def corpus() -> dict[str, tuple[str, str]]:
    """The corpus index's table, in its order: each program's file name with its
    use case and, as HTML, what it demonstrates."""
    text = (ROOT / CORPUS).read_text(encoding="utf-8")
    rows = re.findall(r"^\| `([a-z_]+\.ry)` \| ([^|]+) \| (.+) \|$", text, re.M)
    return {name: (use.strip(), inline_markdown(shows.strip())) for name, use, shows in rows}


def inline_markdown(text: str) -> str:
    """One line of Markdown as HTML, without its paragraph."""
    return re.sub(r"^<p>|</p>$", "", markdown.markdown(text).strip())


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
    """The grammar file in one highlighted block, its section comments marked
    as anchors for the contents."""
    lines = text.rstrip("\n").split("\n")
    shown = highlight_ebnf("\n".join(lines))
    marks, contents = {}, []
    for number, line in enumerate(lines):
        found = re.fullmatch(r"/\* (Sections? ([^*]+?)) \*/", line)
        if found:
            ident = "section-" + re.sub(r"[^a-z0-9]+", "-", found.group(2).lower()).strip("-")
            marks[number] = ident
            shown[number] = html.escape(line, quote=False)
            contents.append((ident, html.escape(found.group(1), quote=False), []))
    block = code_block(shown, "ebnf", marks=marks)
    return f'<h1 class="file">grammar.ebnf</h1>\n{block}\n', contents


def render_program(source: str, text: str) -> str:
    """An example: its name as the title, its source with line numbers."""
    name = html.escape(Path(source).name)
    head = f'<div class="code-head">{html.escape(source)}</div>'
    block = code_block(highlight_renyi(text.rstrip("\n")), "renyi", numbered=True, head=head)
    return f'<h1 class="file">{name}</h1>\n{block}\n'


def render(source: str, text: str, known: dict[str, str]) -> tuple[str, Contents]:
    """The body of a page with its sections: Markdown rendered, or a source file."""
    if source.endswith(".ebnf"):
        return render_grammar(text)
    if source.endswith(".ry"):
        return render_program(source, text), []
    converter = markdown.Markdown(
        extensions=["fenced_code", "tables", "toc", "sane_lists"],
        extension_configs={"toc": {"permalink": "#", "permalink_title": "Link to this section"}},
    )
    body = converter.convert(text)
    contents = sections_of(converter.toc_tokens)  # type: ignore[attr-defined]
    # a table scrolls inside its own container, never the page; a cell that
    # holds only numbers sets them in tabular figures
    body = body.replace("<table>", '<div class="table"><table>')
    body = body.replace("</table>", "</table></div>")
    body = re.sub(r"<td>([0-9][0-9.,:% ]*)</td>", r'<td class="num">\1</td>', body)
    if source == CORPUS:
        body = re.sub(r'<div class="table">.*?</div>', corpus_cards(known), body, count=1,
                      flags=re.S)
    if source in FOLDED:
        body = fold(body)
    return highlight_blocks(body), contents


def corpus_cards(known: dict[str, str]) -> str:
    """The corpus index's table as a card for each program, in its order: the
    name, the use case and what it demonstrates, the whole card a link to the
    program's page; the table's headings stay as labels a screen reader reads."""
    cards = []
    for name, (use, shows) in corpus().items():
        page = posixpath.basename(known[f"examples/{name}"])
        cards.append(f'<li><a href="{page}"><span class="name"><span class="sr">File: </span>'
                     f'{name}</span><span class="use"><span class="sr">Use case: </span>'
                     f'{html.escape(use)}</span><span class="what"><span class="sr">'
                     f"Demonstrates: </span>{shows}</span></a></li>")
    return f'<ul class="corpus">{"".join(cards)}</ul>'


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


def terminal() -> str:
    """The session as lines of text that appear in turn (STYLE): each command,
    then its output; a wrapped line hangs under its first characters."""
    rows, at = [], 0.3

    def row(kind: str, text: str) -> None:
        rows.append(f'<span class="ln {kind}" style="--at: {at:.2f}s">{text}\n</span>')

    for command, output in SESSION:
        row("cmd", f'<span class="ps">$</span> {html.escape(command, quote=False)}')
        at += COMMAND_PAUSE
        for line in output:
            kind = "i2" if line.startswith("  ") else ""
            text = html.escape(line, quote=False)
            if line.startswith("  fix:"):
                kind += " fix"
            text = re.sub(r"\berror\b", '<span class="err">error</span>', text, count=1)
            row(f"out {kind}".strip(), text)
            at += LINE_PAUSE
        at += TURN_PAUSE
    row("cmd", '<span class="ps">$</span> <span class="cursor"></span>')
    return (
        '<figure class="term invert">'
        '<figcaption class="sr">A session at the terminal</figcaption>'
        '<div class="term-bar"><span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>'
        '<span class="term-title">todo</span>'
        '<button class="replay" type="button" hidden>Replay</button></div>'
        f'<pre><code>{"".join(rows)}</code></pre></figure>'
    )


def link(page: str, label: str, target: str, known: dict[str, str]) -> str:
    if "://" in target:
        return f'<a href="{target}">{label}<span class="ext" aria-hidden="true">↗</span></a>'
    here = known[target] == page or (target == CORPUS and page.startswith("examples/"))
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


def brand(page: str, known: dict[str, str]) -> str:
    home = known[HOME]
    current = ' aria-current="page"' if home == page else ""
    return f'<a class="brand" href="{relative(page, home)}"{current}>{MARK}<span>Renyi</span></a>'


def header(page: str, known: dict[str, str]) -> str:
    row = "".join(
        f'<li><span class="sr">{name}: </span>'
        + "".join(link(page, label, target, known) for label, target in entries) + "</li>"
        for name, entries in GROUPS
    )
    return (
        '<header class="top"><div class="bar">'
        f"{brand(page, known)}\n"
        f'<nav class="row" aria-label="Site"><ul>{row}</ul></nav>\n'
        f'<div class="tools">{THEME_BUTTON}'
        '<details class="menu"><summary>Menu<svg class="lines" viewBox="0 0 24 24" '
        'aria-hidden="true"><path d="M5 9h14"/><path d="M5 15h14"/></svg></summary>'
        f'<nav class="panel" aria-label="Site">{groups(page, known)}</nav></details>'
        "</div></div></header>\n"
    )


def footer(page: str, known: dict[str, str], stamp: str) -> str:
    return (
        '<footer class="foot"><div class="in"><div class="foot-main">'
        f'<div class="foot-brand">{brand(page, known)}<p>{html.escape(DESCRIPTION)}</p></div>'
        f'<nav aria-label="All pages">{groups(page, known)}</nav></div>'
        f'<div class="meta"><span>{stamp}</span><span>Apache-2.0 · <a href="{REPOSITORY}">'
        "github.com/renyi-lang/renyi</a></span></div></div></footer>\n"
    )


def group_of(source: str) -> str:
    """The name shown above a page's title: its group in the navigation, or the
    section its path belongs to."""
    for name, entries in GROUPS:
        if any(target == source for _, target in entries):
            return name
    return next(name for prefix, name in SECTIONS if source.startswith(prefix))


def sequence_of(source: str) -> list[tuple[str, str]]:
    """The pages a page is read among, as (source, label): an example among the
    corpus index's programs, any other page among the navigation's."""
    if source.endswith(".ry"):
        return [(f"examples/{name}", name) for name in corpus()]
    return [(target, label) for _, entries in GROUPS for label, target in entries
            if "://" not in target]


def pager(source: str, page: str, known: dict[str, str]) -> str:
    """Links to the pages before and after this one, when it has a place among them."""
    order = sequence_of(source)
    places = [target for target, _ in order]
    if source not in places:
        return ""
    index = places.index(source)
    style = ' class="file"' if source.endswith(".ry") else ""
    parts = []
    for side, word, step in (("prev", "Previous", -1), ("next", "Next", 1)):
        if 0 <= index + step < len(order):
            target, label = order[index + step]
            parts.append(f'<a class="{side}" href="{relative(page, known[target])}">'
                         f"<span>{word}</span><strong{style}>{label}</strong></a>")
    return f'<nav class="pager" aria-label="Previous and next">{"".join(parts)}</nav>\n'


def layout(source: str, text: str, body: str, contents: Contents, page: str,
           known: dict[str, str]) -> str:
    """The `main` of a page: the group above the title, the lead under it and,
    for a long page, its sections in a disclosure there and in a sticky column
    beside it; at its end the pages before and after it and its source."""
    style = LAYOUTS.get(source)
    if style == "home":
        return f'<main id="content" class="home">\n{body}\n</main>'
    above = group_of(source)
    if source.endswith(".ry"):
        index = relative(page, known[CORPUS])
        use = corpus().get(Path(source).name, ("", ""))[0]
        above = f'<a href="{index}">Examples</a>' + (f" · {html.escape(use)}" if use else "")
    lead = lead_of(source, text)
    after = f'\n<p class="lead">{lead}</p>' if lead else ""
    shows = corpus().get(Path(source).name, ("", ""))[1] if source.endswith(".ry") else ""
    if shows:
        after += f'\n<p class="demonstrates"><span>Demonstrates</span>{shows}</p>'
    long = len(contents) >= CONTENTS_FROM
    if long:
        after += ('\n<details class="contents inline"><summary>On this page</summary>'
                  f"{contents_list(contents)}</details>")
    head, _, rest = body.partition("<h1")
    end = rest.index("</h1>") + len("</h1>")
    body = f'{head}<p class="eyebrow">{above}</p>\n<h1{rest[:end]}{after}{rest[end:]}'
    tail = pager(source, page, known)
    tail += (f'<p class="source"><a href="{REPOSITORY}/blob/main/{source}">'
             "View the source of this page</a></p>\n")
    if style == "feature":
        return f'<main id="content" class="feature">\n{body}\n{tail}</main>'
    kind = "doc wide" if source.endswith(".ry") or source == CORPUS else "doc"
    if not long:
        return f'<main id="content" class="{kind}">\n{body}\n{tail}</main>'
    return (
        f'<main id="content" class="{kind} paged">\n<article>\n'
        f"{body}\n{tail}</article>\n"
        '<nav class="contents toc" aria-label="On this page"><p>On this page</p>'
        f"{contents_list(contents)}</nav>\n</main>"
    )


def wrap(page: str, title: str, main: str, known: dict[str, str], stamp: str, css: str) -> str:
    """The page around its `main`: the head, the header, the footer, the script."""
    heading = title if title == "Renyi" else f"{title} · Renyi"
    return (
        "<!doctype html>\n"
        '<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{html.escape(heading)}</title>\n"
        f'<meta name="description" content="{html.escape(DESCRIPTION)}">\n'
        '<meta name="color-scheme" content="light dark">\n'
        f'<meta name="theme-color" content="{LIGHT["bg"]}" '
        'media="(prefers-color-scheme: light)">\n'
        f'<meta name="theme-color" content="{DARK["bg"]}" media="(prefers-color-scheme: dark)">\n'
        f'<link rel="icon" type="image/svg+xml" href="{ICON}">\n'
        f"<script>{THEME_SCRIPT}</script>\n"
        f"<style>\n{css}{STYLE}</style>\n</head>\n<body>\n"
        '<a class="skip" href="#content">Skip to the content</a>\n'
        f"{header(page, known)}"
        f"{main}\n"
        f"{footer(page, known, stamp)}"
        f"<script>{SCRIPT}</script>\n</body>\n</html>\n"
    )


def build(out: Path) -> int:
    known = pages()
    stamp = f"Renyi {version()}, built from commit {commit()}."
    css = theme_css()
    demo = terminal()
    for source, page in known.items():
        text = (ROOT / source).read_text(encoding="utf-8")
        body, contents = render(source, text, known)
        body = body.replace('<div class="terminal"></div>', demo)
        main = layout(source, text, body, contents, page, known)
        main = mark_long_code(rewrite_links(main, source, page, known))
        if source not in LAYOUTS:
            main = keep_dates_whole(main)
        target = out / page
        target.parent.mkdir(parents=True, exist_ok=True)
        page_html = wrap(page, title_of(source, text), main, known, stamp, css)
        target.write_text(page_html, encoding="utf-8", newline="\n")
    return len(known)


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "_site"
    count = build(out)
    print(f"{count} pages written to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
