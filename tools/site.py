#!/usr/bin/env python3
"""Build the documentation site (decision AI2).

Every Markdown document under `docs/` but the session handoff, the grammar
file, the corpus index with each example's source, and the starter pack's
README become one HTML page each, with a shared header, and are written to
a directory GitHub Pages serves (`.github/workflows/pages.yml`). Links
between the documents are rewritten to the pages; every link is relative,
so the site works under a path such as `/renyi/`.
The front page, `docs/index.md`, is HTML blocks with a layout of its own
(`main.home` in the stylesheet); every table scrolls in its own container.

Usage: python tools/site.py [<output directory>]      (default: _site)
Requires the `markdown` package (`pip install markdown`).
"""

from __future__ import annotations

import html
import posixpath
import re
import subprocess
import sys
from pathlib import Path

import markdown

ROOT = Path(__file__).resolve().parent.parent

# The session handoff is the state of the work, not documentation.
SKIPPED = {"docs/HANDOFF.md"}

# The header's links, as repository paths; each page links to them relatively.
NAVIGATION = [
    ("Renyi", "docs/index.md"),
    ("Cheat sheet", "docs/cheatsheet.md"),
    ("Reference", "docs/reference.md"),
    ("Grammar", "docs/grammar.ebnf"),
    ("Examples", "examples/README.md"),
    ("Starter pack", "starter/README.md"),
    ("Extensions", "docs/extensions.md"),
    ("Python", "docs/python.md"),
    ("Embedding", "docs/embedding.md"),
    ("Positioning", "docs/design/08-positioning.md"),
    ("Roadmap", "docs/ROADMAP.md"),
    ("Decisions", "docs/design/01-decisions.md"),
]

REPOSITORY = "https://github.com/renyi-lang/renyi"

STYLE = """
:root {
  color-scheme: light dark;
  --text: #16171a; --muted: #62656d; --faint: #8b8e95;
  --bg: #ffffff; --surface: #f6f6f4; --line: #e6e6e3; --line-strong: #d4d4d0;
  --accent: #2f4fd8;
  --sans: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
          Roboto, "Helvetica Neue", Arial, sans-serif;
  --mono: ui-monospace, "SF Mono", "Cascadia Mono", "JetBrains Mono", Menlo,
          Consolas, monospace;
  --measure: 44rem; --wide: 68rem; --gutter: 16px;
}
@media (prefers-color-scheme: dark) {
  :root {
    --text: #ebebe8; --muted: #a2a4aa; --faint: #74777e;
    --bg: #0e0f11; --surface: #16171a; --line: #24262a; --line-strong: #34363b;
    --accent: #93a6ff;
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

/* The header: the name, the documents, the repository. */
.top { position: sticky; top: 0; z-index: 10; border-bottom: 1px solid var(--line);
       background: var(--bg); background: color-mix(in srgb, var(--bg) 86%, transparent);
       -webkit-backdrop-filter: saturate(1.6) blur(14px);
       backdrop-filter: saturate(1.6) blur(14px); }
.bar { max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto; padding: 0 var(--gutter);
       display: flex; flex-wrap: wrap; align-items: center; column-gap: 2rem; }
.brand { order: 1; padding: 0.85rem 0; font-weight: 650; font-size: 1.0625rem;
         letter-spacing: -0.015em; color: var(--text); text-decoration: none; }
.repo { order: 2; margin-left: auto; padding: 0.85rem 0; font-size: 0.875rem;
        color: var(--muted); text-decoration: none; }
.top nav { order: 3; flex: 1 0 100%; display: flex; gap: 1.35rem; overflow-x: auto;
           white-space: nowrap; scrollbar-width: none; padding: 0 0 0.7rem;
           margin: 0 calc(-1 * var(--gutter)); padding-inline: var(--gutter); font-size: 0.875rem; }
.top nav::-webkit-scrollbar { display: none; }
.top nav a, .repo { color: var(--muted); text-decoration: none; transition: color 0.15s; }
.top nav a:hover, .repo:hover { color: var(--text); }
.top nav a[aria-current="page"] { color: var(--text); font-weight: 550; }
@media (min-width: 1200px) {
  .bar { flex-wrap: nowrap; }
  .top nav { order: 2; flex: 0 1 auto; min-width: 0; margin: 0 0 0 auto; padding: 0;
             gap: 0.9rem; justify-content: flex-start; }
  .repo { order: 3; flex: 0 0 auto; margin-left: 0; padding-left: 1.5rem;
          border-left: 1px solid var(--line); }
}

/* The documents: one measure, a quiet scale. */
main { max-width: calc(var(--measure) + 2 * var(--gutter)); margin: 0 auto;
       padding: 3rem var(--gutter) 5rem; }
main > :first-child { margin-top: 0; }
h1, h2, h3, h4, h5, h6 { color: var(--text); line-height: 1.25; scroll-margin-top: 7rem;
                         letter-spacing: -0.015em; }
h1 { font-size: clamp(1.9rem, 1.4rem + 2vw, 2.6rem); font-weight: 680;
     letter-spacing: -0.03em; margin: 0 0 1.25rem; }
h2 { font-size: 1.45rem; font-weight: 650; margin: 3.25rem 0 1rem; padding-top: 1.75rem;
     border-top: 1px solid var(--line); }
h3 { font-size: 1.15rem; font-weight: 640; margin: 2.25rem 0 0.75rem; }
h4, h5, h6 { font-size: 1rem; font-weight: 640; margin: 1.75rem 0 0.5rem; }
h1 + h2 { border-top: 0; padding-top: 0; }
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
a > code { color: inherit; }

/* Tables: inside a container that scrolls on a narrow screen. */
.table { margin: 1.5rem 0 1.75rem; overflow-x: auto; -webkit-overflow-scrolling: touch;
         border-top: 1px solid var(--line-strong); }
table { border-collapse: collapse; width: 100%; font-size: 0.9rem; line-height: 1.55; }
th, td { padding: 0.6rem 1rem 0.6rem 0; text-align: left; vertical-align: top;
         border-bottom: 1px solid var(--line); }
th:last-child, td:last-child { padding-right: 0; }
th { font-weight: 600; color: var(--muted); font-size: 0.8125rem; letter-spacing: 0.01em; }
@media (max-width: 639px) { th, td { min-width: 9rem; } }

footer { max-width: calc(var(--wide) + 2 * var(--gutter)); margin: 0 auto;
         padding: 2rem var(--gutter) 3rem;
         border-top: 1px solid var(--line); color: var(--muted); font-size: 0.8125rem;
         display: flex; flex-wrap: wrap; gap: 0.5rem 1.5rem; justify-content: space-between; }
footer a { color: inherit; }

/* The front page. */
main.home { max-width: calc(var(--wide) + 2 * var(--gutter)); padding-top: 0; }
.home section { padding: 4.5rem 0; border-top: 1px solid var(--line); scroll-margin-top: 2rem; }
.home section:first-child { border-top: 0; padding: 5.5rem 0 4rem; }
.home h1 { font-size: clamp(2.4rem, 1.5rem + 4.2vw, 4.4rem); line-height: 1.04;
           letter-spacing: -0.045em; font-weight: 700; max-width: 13ch; margin: 0 0 1.5rem;
           text-wrap: balance; }
.home h2 { border: 0; padding: 0; margin: 0 0 1.75rem; font-size: 0.8125rem; font-weight: 600;
           letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
.home h3 { margin: 0 0 0.5rem; font-size: 1.0625rem; letter-spacing: -0.01em; }
.tagline { font-size: clamp(1.2rem, 1rem + 0.9vw, 1.6rem); line-height: 1.4;
           letter-spacing: -0.02em; font-weight: 500; max-width: 32ch; margin: 0 0 1.25rem; }
.lead { color: var(--muted); max-width: 38rem; margin: 0 0 2.25rem; }
.actions { display: flex; flex-wrap: wrap; gap: 0.75rem; margin: 0; }
.button { display: inline-block; padding: 0.6rem 1.15rem; border-radius: 999px;
          font-size: 0.9375rem; font-weight: 550; text-decoration: none;
          border: 1px solid var(--line-strong); color: var(--text); }
.button:hover { border-color: var(--text); }
.button.primary { background: var(--text); border-color: var(--text); color: var(--bg); }
.button.primary:hover { opacity: 0.88; }
.sample, .points, .install, .links { grid-template-columns: minmax(0, 1fr); }
.sample { display: grid; gap: 1.5rem 3.5rem; align-items: start; }
.sample p { color: var(--muted); max-width: 30rem; }
.sample .statement { color: var(--text); font-size: 1.35rem; line-height: 1.35;
                     letter-spacing: -0.02em; font-weight: 600; }
.sample pre { margin: 0; padding: 1.4rem 1.5rem; border-radius: 14px; }
.sample pre code { font-size: 0.8125rem; line-height: 1.7; white-space: pre-wrap; }
.k { color: var(--accent); }
.c { color: var(--muted); }
.points { display: grid; gap: 2.5rem 3rem; }
.points p { margin: 0; color: var(--muted); font-size: 0.96875rem; }
.points .n { display: block; margin-bottom: 0.9rem; font: 500 0.8125rem var(--mono);
             color: var(--faint); }
.gap p { font-size: clamp(1.25rem, 1rem + 1vw, 1.75rem); line-height: 1.4;
         letter-spacing: -0.02em; font-weight: 500; max-width: 36ch; margin: 0 0 1rem; }
.gap p + p { color: var(--muted); }
.install { display: grid; gap: 1.5rem 3.5rem; }
.install pre { margin: 0 0 1rem; }
.install .label { margin: 0 0 0.4rem; font-size: 0.8125rem; color: var(--muted); }
.install .note { color: var(--muted); font-size: 0.9375rem; }
.links { list-style: none; padding: 0; margin: 0; display: grid;
         border-top: 1px solid var(--line); }
.links li { margin: 0; border-bottom: 1px solid var(--line); }
.links a { display: flex; flex-wrap: wrap; justify-content: space-between; gap: 0.25rem 1rem;
           padding: 1.05rem 0; color: var(--text); text-decoration: none; }
.links a span { color: var(--muted); font-size: 0.9375rem; }
.links a:hover span { color: var(--text); }
@media (min-width: 760px) {
  .points { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .install { grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr); }
  .links { grid-template-columns: repeat(2, minmax(0, 1fr)); column-gap: 3.5rem; }
}
@media (min-width: 1000px) {
  .sample { grid-template-columns: minmax(0, 0.8fr) minmax(0, 1.2fr); }
  .points { grid-template-columns: repeat(4, minmax(0, 1fr)); }
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
    if source == NAVIGATION[0][1]:
        return NAVIGATION[0][0]  # the front page's heading is its one line, not its name
    for line in text.splitlines():
        if line.startswith("# "):
            return line[2:].strip()
    return Path(source).name


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


def render(source: str, text: str, known: dict[str, str]) -> str:
    """The body of a page: Markdown rendered, or a source file in a block."""
    if source.endswith(".md"):
        body = markdown.markdown(
            text,
            extensions=["fenced_code", "tables", "toc", "sane_lists"],
            extension_configs={
                "toc": {"permalink": "#", "permalink_title": "Link to this section"}
            },
        )
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
        return body
    name = Path(source).name
    return f"<h1><code>{html.escape(name)}</code></h1>\n<pre><code>{html.escape(text)}</code></pre>\n"


DESCRIPTION = (
    "Renyi is the scripting language of AI agents: the agent writes it, you "
    "review it at a glance, and the program can only do what it declares."
)


def wrap(page: str, title: str, body: str, known: dict[str, str], stamp: str) -> str:
    """The page around a body; the front page's `main` has a layout of its own."""
    heading = title if title == "Renyi" else f"{title} · Renyi"
    (brand, home), *entries = NAVIGATION
    links = []
    for label, target in entries:
        current = ' aria-current="page"' if known[target] == page else ""
        links.append(f'<a href="{relative(page, known[target])}"{current}>{label}</a>')
    home_page = known[home]
    brand_current = ' aria-current="page"' if home_page == page else ""
    kind = ' class="home"' if home_page == page else ""
    return (
        "<!doctype html>\n"
        '<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{html.escape(heading)}</title>\n"
        f'<meta name="description" content="{html.escape(DESCRIPTION)}">\n'
        '<meta name="theme-color" content="#ffffff" media="(prefers-color-scheme: light)">\n'
        '<meta name="theme-color" content="#0e0f11" media="(prefers-color-scheme: dark)">\n'
        f"<style>{STYLE}</style>\n</head>\n<body>\n"
        '<header class="top"><div class="bar">'
        f'<a class="brand" href="{relative(page, home_page)}"{brand_current}>{brand}</a>'
        f'<nav aria-label="Documents">{"".join(links)}</nav>'
        f'<a class="repo" href="{REPOSITORY}">GitHub</a>'
        "</div></header>\n"
        f"<main{kind}>\n{body}\n</main>\n"
        f'<footer><span>{stamp}</span><span>Apache-2.0 · <a href="{REPOSITORY}">'
        "github.com/renyi-lang/renyi</a></span></footer>\n</body>\n</html>\n"
    )


def build(out: Path) -> int:
    known = pages()
    stamp = f"Renyi {version()}, built from commit {commit()}."
    for source, page in known.items():
        text = (ROOT / source).read_text(encoding="utf-8")
        body = rewrite_links(render(source, text, known), source, page, known)
        title = title_of(source, text) if source.endswith(".md") else Path(source).name
        target = out / page
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(wrap(page, title, body, known, stamp), encoding="utf-8", newline="\n")
    return len(known)


def main() -> int:
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "_site"
    count = build(out)
    print(f"{count} pages written to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
