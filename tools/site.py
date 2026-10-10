#!/usr/bin/env python3
"""Build the documentation site (decision AI2).

Every Markdown document under `docs/` but the session handoff, the grammar
file, the corpus index with each example's source, and the starter pack's
README become one HTML page each, with a shared header, and are written to
a directory GitHub Pages serves (`.github/workflows/pages.yml`). Links
between the documents are rewritten to the pages; every link is relative,
so the site works under a path such as `/renyi/`.

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
:root { color-scheme: light dark; }
body { margin: 0; font: 16px/1.55 system-ui, -apple-system, "Segoe UI", sans-serif;
       color: #1d1d1d; background: #fff; }
header { border-bottom: 1px solid #ccc; }
header nav { max-width: 48em; margin: 0 auto; padding: 0.6em 1em;
             display: flex; flex-wrap: wrap; gap: 0.3em 1.2em; }
header a { text-decoration: none; color: inherit; }
header a.current { font-weight: 600; }
main { max-width: 48em; margin: 0 auto; padding: 1em 1em 3em; }
footer { max-width: 48em; margin: 0 auto; padding: 1em; color: #666;
         border-top: 1px solid #ccc; font-size: 0.9em; }
h1, h2, h3 { line-height: 1.25; }
h1 { font-size: 1.8em; } h2 { font-size: 1.35em; margin-top: 1.8em; }
code, pre { font-family: ui-monospace, "Cascadia Mono", Consolas, Menlo, monospace;
            font-size: 0.92em; }
pre { padding: 0.8em 1em; overflow-x: auto; background: #f4f4f4; border-radius: 4px; }
code { background: #f4f4f4; padding: 0 0.2em; border-radius: 3px; }
pre code { padding: 0; background: none; }
table { border-collapse: collapse; margin: 1em 0; display: block; overflow-x: auto; }
th, td { border: 1px solid #ccc; padding: 0.3em 0.6em; text-align: left; vertical-align: top; }
th { background: #f4f4f4; }
blockquote { margin: 1em 0; padding: 0 1em; border-left: 3px solid #ccc; color: #444; }
a { color: #0a58a5; }
@media (prefers-color-scheme: dark) {
  body { color: #e6e6e6; background: #161616; }
  header, footer { border-color: #444; }
  pre, code, th { background: #242424; }
  th, td { border-color: #444; }
  blockquote { border-color: #555; color: #bbb; }
  a { color: #7cb7ff; }
  footer { color: #999; }
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
            text, extensions=["fenced_code", "tables", "toc", "sane_lists"]
        )
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


def wrap(page: str, title: str, body: str, known: dict[str, str], stamp: str) -> str:
    heading = title if title == "Renyi" else f"{title} · Renyi"
    links = []
    for label, target in NAVIGATION:
        current = ' class="current"' if known[target] == page else ""
        links.append(f'<a href="{relative(page, known[target])}"{current}>{label}</a>')
    links.append(f'<a href="{REPOSITORY}">GitHub</a>')
    return (
        "<!doctype html>\n"
        '<html lang="en">\n<head>\n<meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{html.escape(heading)}</title>\n"
        f"<style>{STYLE}</style>\n</head>\n<body>\n"
        f"<header><nav>{' '.join(links)}</nav></header>\n"
        f"<main>\n{body}\n</main>\n"
        f"<footer>{stamp}</footer>\n</body>\n</html>\n"
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
