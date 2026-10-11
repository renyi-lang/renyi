#!/usr/bin/env python3
"""The mark and the wordmark (decision AV1), and the files made from them.

The mark is a war hammer on a tile of the accent colour, drawn upright on a
grid of 24 and turned 40 degrees: the full drawing, with a grip band and a
pommel, from 48 pixels up, and the plain one, a thicker haft without them,
below. The wordmark is the name signed in Ms Madi (Robert Leuschke, SIL Open
Font License 1.1), shaped by HarfBuzz and kept here as one path, so that no
page loads a font for it. `tools/site.py` takes the mark, the tab's icon and
the wordmark from here; run, this script writes the files the README, the VS
Code extension and the repository's social preview use:

  docs/assets/renyi-mark.svg            the mark, the full drawing
  docs/assets/renyi-logo-light.svg      the mark and the wordmark, light
  docs/assets/renyi-logo-dark.svg       the same for a dark background
  editors/vscode/images/file-light.svg  the icon of a .ry file, light themes
  editors/vscode/images/file-dark.svg   the same for dark themes
  editors/vscode/images/icon.png        the extension's icon     (--png)
  docs/assets/social-preview.png        the social preview       (--png)

The PNG files are screenshots by a headless Chromium (`headless_shell` or
`chrome`); the social preview sets its text in Inter where it is installed,
else in the system's sans-serif. GitHub takes the social preview from the
repository's settings, where the owner uploads the file.

Usage: python tools/brand.py [--png <headless Chromium>]
"""

from __future__ import annotations

import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The site's accent and the colour on it, the text and the page, in both
# themes (`LIGHT` and `DARK` in tools/site.py, which checks they agree).
LIGHT = {"tile": "#2f4fd8", "ink": "#ffffff", "text": "#111318", "muted": "#50565f",
         "bg": "#ffffff"}
DARK = {"tile": "#8ea2ff", "ink": "#0a0b0e", "text": "#ecedf0", "muted": "#a3a9b3",
        "bg": "#0a0b0e"}

# The hammer's parts, upright: the socket on the haft, the hammer with its
# flared face to the left, the beak to the right, the spike on top.
HEAD = (
    '<rect x="10.35" y="3.1" width="3.3" height="6.9" rx=".65"/>'
    '<rect x="6.2" y="4.1" width="4.6" height="4.7"/>'
    '<rect x="4.7" y="3.45" width="2.1" height="6" rx=".6"/>'
    '<path d="M13.4 4.6C15.6 5.15 17.4 6.15 18.9 7.85C17.1 7.6 15.3 7.6 13.4 7.8Z" '
    'stroke="currentColor" stroke-width=".35" stroke-linejoin="round"/>'
    '<path d="M10.95 3.3L12 .95L13.05 3.3Z" stroke="currentColor" stroke-width=".35" '
    'stroke-linejoin="round"/>'
)
HAFT_FULL = (
    '<rect x="11.05" y="8.25" width="1.9" height="13.4" rx=".95"/>'
    '<rect x="10.55" y="17.4" width="2.9" height="2.1" rx=".5"/>'
    '<circle cx="12" cy="21.15" r="1.2"/>'
)
HAFT_PLAIN = '<rect x="10.8" y="8" width="2.4" height="13.9" rx="1.2"/>'

# The signature, Ms Madi at 100 pixels to the em, its bounds from the origin.
SIGNATURE_BOX = (214.4, 107.7)
SIGNATURE = (
    "M15.5 74.6Q13.7 74.6 12.6 72.9Q11.4 71.2 10.8 69.1Q10.3 67 10.3 65.9Q10.3 64.2 11.2 61.4"
    "Q12 58.6 13.2 55.6Q14.4 52.5 15.7 49.8Q16.9 47 17.8 45.4Q15.7 43.9 14.4 42.3"
    "Q13.1 40.6 13.1 38.1Q13.1 36.2 14.2 35.1Q15.3 34 17.5 34Q19 34 20.5 34.5"
    "Q22 34.9 23.3 35.6Q26.5 30.7 30.6 25.2Q34.8 19.6 38.8 15.3Q39.3 14.8 40 14.2"
    "Q40.8 13.6 41.7 13.6Q42.6 13.6 42.6 14.6Q42.6 15.3 42.2 15.9Q41.8 16.5 41.4 17"
    "Q40.1 18.6 38.8 20.1Q37.5 21.6 36.2 23.2Q33.4 26.7 30.9 30.4Q28.3 34.1 26 37.9"
    "Q28.2 39.9 30.2 42.1Q32.3 44.2 34.3 46.4H37.3Q43.5 46.4 50.4 45.5Q57.3 44.6 63.9 42.5"
    "Q70.4 40.3 75.5 36.6Q80.5 32.9 82.9 27.2Q84.2 23.9 84.2 20.9Q84.2 16 81.4 12.7"
    "Q78.6 9.4 74.1 7.5Q69.6 5.5 64.5 4.7Q59.4 3.8 54.8 3.8H53Q47.5 3.8 41.1 4.5"
    "Q34.8 5.1 28.5 6.7Q22.2 8.2 16.8 11Q11.4 13.7 7.9 17.9Q6.3 19.7 5.3 21.9"
    "Q4.3 24.1 4.3 26.4Q4.3 27.1 4.5 27.8Q4.8 28.5 4.8 29.2Q4.8 29.8 4.3 30.2"
    "Q3.8 30.6 3.1 30.6Q1.6 30.6 1.3 29.2Q1 27.7 1 26.9Q1 24.3 2 21.8Q3.1 19.3 4.7 17.2"
    "Q8.4 12.3 14.1 9.2Q19.7 6 26.4 4.2Q33.1 2.4 40 1.7Q46.9 1 53 1H54.7Q60 1 65.8 2"
    "Q71.5 3 76.5 5.3Q81.4 7.6 84.5 11.5Q87.5 15.3 87.5 20.9Q87.5 24.6 86 28.1"
    "Q83.3 34.2 77.9 38.3Q72.5 42.4 65.5 44.8Q58.6 47.2 51.1 48.3Q43.7 49.3 36.9 49.4"
    "Q38 50.5 39 51.7Q40.1 52.8 41.1 54Q42.3 55.4 44.6 58Q47 60.6 49.9 63.3"
    "Q52.7 65.9 55.5 67.8Q58.4 69.6 60.7 69.6Q63 69.6 64.3 68.6Q65.7 67.6 66.7 67.6"
    "Q68.3 67.6 68.3 69.1Q68.3 70 66.8 70.8Q65.2 71.6 63.4 72.1Q61.5 72.5 60.6 72.5"
    "Q58.2 72.5 55.7 71.4Q53.8 70.6 51.4 68.6Q49 66.6 46.5 64.1Q44.1 61.6 42 59.3"
    "Q39.8 57 38.5 55.6L32.7 49.3Q30.1 49.1 28.4 49Q26.6 48.9 24.9 48.5Q23.3 48.1 20.8 47.1"
    "Q20 48.7 18.8 51.3Q17.5 53.9 16.3 56.8Q15.1 59.7 14.3 62.3Q13.5 64.9 13.5 66.5"
    "Q13.5 69.3 14.4 70.4Q15.4 71.4 16.3 71.8Q17.2 72.1 17.2 73Q17.2 73.6 16.8 74.1"
    "Q16.3 74.6 15.5 74.6ZM19.4 42.8Q20.2 41.4 20.9 40.1Q21.5 38.8 22.3 37.5"
    "Q20.6 36.1 18.6 36.1Q16.4 36.1 16.4 37.9Q16.4 39.3 17.4 40.6Q18.4 41.8 19.4 42.8Z"
    "M30.2 46.2Q28.8 44.7 27.4 43.2Q26.1 41.6 24.7 40Q24 41.1 23.4 42.3Q22.8 43.5 22.2 44.6"
    "Q24.3 45.6 26.1 45.8Q27.9 46 30.2 46.2ZM88.5 70.4Q83.3 70.4 81.2 67.4"
    "Q79.2 64.3 79.2 60.2Q79.2 57.2 80.7 53.1Q82.2 48.9 84.8 44.9Q87.5 40.9 91 38.3"
    "Q94.5 35.6 98.5 35.6Q101.1 35.6 102.7 37.1Q104.2 38.5 104.2 40.7Q104.2 44.9 101.5 48.8"
    "Q98.8 52.7 94.5 55.2Q90.2 57.7 85.3 57.7Q84.7 57.7 84.1 57.7Q83.5 57.6 82.9 57.5"
    "Q82.6 58.8 82.6 60.5Q82.6 63.1 83.8 65.3Q85.1 67.5 88.7 67.5Q91.7 67.5 94.5 66"
    "Q97.3 64.5 99.2 62.6Q101.1 60.7 102.4 59.2Q103.7 57.7 105.1 55.8Q106.5 53.8 108.6 50.6"
    "Q109 49.9 109.8 49.9Q111.2 49.9 111.2 51.4Q111.2 51.9 111 52.2Q108.3 56.8 105 61"
    "Q101.7 65.1 97.7 67.8Q93.6 70.4 88.5 70.4ZM86 54.6Q89.8 54.6 93.2 52.6"
    "Q96.5 50.5 98.7 47.4Q100.8 44.3 100.8 41.1Q100.8 40 100.2 39.3Q99.7 38.6 98.4 38.6"
    "Q95.8 38.6 93.4 40.4Q91 42.1 89 44.8Q87.1 47.4 85.8 50Q84.4 52.6 83.8 54.4"
    "Q84.3 54.5 84.8 54.6Q85.4 54.6 86 54.6ZM113.9 69.7Q113.4 69.7 112.8 69.4"
    "Q112.2 69.1 112.2 68.6Q112.2 67.8 112.5 67.2Q113.6 64.8 115.3 61.3Q117.1 57.8 118.8 54.1"
    "Q120.6 50.3 121.8 47Q123.1 43.7 123.1 41.7Q123.1 41.1 122.8 40.6Q122.6 40 121.8 40"
    "Q120.6 40 119.5 41Q118.3 42 117.4 43.3Q116.5 44.5 115.9 45.3Q113.9 48 112.5 50.3"
    "Q111.1 52.5 109.7 54.7Q108.2 56.8 106.1 59.4Q105.8 59.8 105.5 60.1Q105.1 60.3 104.5 60.3"
    "Q103.9 60.3 103.5 59.7Q103 59.1 103.3 58.6Q105.5 55.1 107.6 52Q109.7 48.8 111.3 46.5"
    "Q112.9 44.2 113.5 43.4Q115.2 41.2 117 39.2Q118.9 37.2 122.1 37.2Q124.8 37.2 125.8 38.7"
    "Q126.9 40.2 126.9 42.2Q126.9 45.8 123.9 51.5Q127.6 47.6 131.8 44.4Q136.1 41.2 139.8 39.1"
    "Q143.5 36.9 145.4 36.4Q146.3 36.1 147.2 35.9Q148.2 35.7 149 35.7Q150 35.7 150.7 36.2"
    "Q151.3 36.7 151.3 38.1Q151.3 39.5 149.5 42.1Q147.7 44.6 145.2 47.9Q142.6 51.1 140.3 54.4"
    "Q138 57.7 137 60.6Q136.7 61.3 136.6 62Q136.5 62.6 136.5 63.1Q136.5 65.8 139 65.8"
    "Q140.6 65.8 143 64.2Q144.4 63.3 145.8 62.2Q147.1 61 147.8 60.1Q149.3 58.4 150.2 57.2"
    "Q151.2 56 152.2 54.6Q153.1 53.1 154.6 50.9Q154.9 50.4 155.2 50.2Q155.5 50 156.2 50"
    "Q157.3 50 157.3 51.4Q157.3 52 156.4 53.5Q155.5 54.9 154.2 56.7Q153 58.4 151.8 59.9"
    "Q150.7 61.3 150.2 61.9Q149.1 63.2 147.4 64.6Q145.8 66 144.1 67Q142.5 67.9 140.8 68.3"
    "Q139.1 68.7 137.5 68.7Q134.9 68.7 133.8 67.2Q132.6 65.7 132.6 64Q132.6 61.4 134.3 58.1"
    "Q136.1 54.7 138.6 51.4Q141 48 143.2 45.3Q145.3 42.6 146.1 41.4Q146.3 41.1 146.4 40.9"
    "Q146.5 40.6 146.5 40.4Q146.5 39.6 145.3 39.6Q144.7 39.6 143.9 39.9Q139.8 41.6 135.3 44.9"
    "Q130.9 48.2 127.2 52.1Q123.4 55.9 121.2 59.3L117.2 65.6Q116.7 66.4 116.3 67.2"
    "Q115.9 68.1 115.4 68.9Q114.8 69.7 113.9 69.7ZM139.6 106.7Q136.2 106.7 134.2 105"
    "Q132.3 103.2 132.3 100.5Q132.3 98.1 134.4 95.2Q136.5 92.3 141 88.8Q142.5 87.6 145.8 85.5"
    "Q149.2 83.3 153.4 80.7Q157.7 78.1 161.8 75.6Q166 73.1 169.3 71.2Q172.6 69.2 174 68.3"
    "Q174.9 65.9 175.3 63.1Q175.7 60.2 175.8 57.9Q175.8 55.5 175.7 54.4Q174.5 57.2 172.1 60.5"
    "Q169.6 63.7 166.4 66.9Q164.5 68.8 162.1 70.1Q159.6 71.3 157.3 71.3Q155.1 71.3 153.7 69.8"
    "Q152.2 68.3 152.2 64.8Q152.2 62.5 152.7 60.2Q153.2 57.9 153.8 56.2Q154.5 54.5 154.8 54.1"
    "Q154.3 53.4 154.3 52.6Q154.3 50.8 155.7 48.4Q157 46 158.6 44Q160.1 42 160.5 41.3"
    "Q160.7 40.9 161.3 40.7Q162 40.4 162.5 40.4Q163.2 40.4 163.8 40.8Q164.3 41.2 164.3 41.8"
    "Q164.3 42 164.1 42.4Q160.7 48.2 158.1 54.1Q155.4 59.9 155.4 64.8Q155.4 66.7 156.1 67.5"
    "Q156.8 68.2 157.8 68.2Q159.5 68.2 161.5 66.9Q163.5 65.6 164.6 64.4Q168.3 60.6 171.1 56.1"
    "Q173.9 51.6 175.1 49.1Q175.4 48.3 176.2 47.3Q176.9 46.2 178.1 46.2Q179.2 46.2 179.4 47.2"
    "Q179.7 48.1 179.7 50.1Q179.7 54.7 179.3 58.5Q179 62.3 178.5 64.9Q181.9 62.1 184.8 58.2"
    "Q187.6 54.2 189.9 50.7Q190.4 49.9 191.2 49.9Q191.7 49.9 192.2 50.3Q192.6 50.7 192.6 51.3"
    "Q192.6 51.7 192.3 52.3Q189.2 57.3 185.7 61.9Q182.2 66.4 176.9 69.9Q174.1 77.9 169.2 85.1"
    "Q164.4 92.2 158.6 97.5Q152.7 102.7 146.8 105.1Q143 106.7 139.6 106.7ZM139.6 104.2"
    "Q140.8 104.2 142.2 103.8Q143.6 103.4 145.1 102.8Q151 100.2 155.9 95.8"
    "Q160.9 91.4 164.6 86.3Q167.1 82.8 169 79.1Q170.9 75.3 172 73.3Q170.2 74.3 167.2 76.1"
    "Q164.1 77.9 160.4 80.1Q156.8 82.3 153.2 84.5Q149.7 86.7 146.8 88.6Q144 90.4 142.6 91.5"
    "Q139.4 94 137.6 96.2Q135.7 98.5 135.7 100.5Q135.7 102.1 136.7 103.2"
    "Q137.7 104.2 139.6 104.2ZM205.3 28.6Q204.8 28.6 204.2 28.4Q203.6 28.2 203.6 27.5"
    "Q203.6 27 203.9 26.6Q204.7 25.1 206.8 24Q209 22.9 210.1 22.9Q210.6 22.9 211.2 23.2"
    "Q211.8 23.4 211.8 24.1Q211.8 25.8 209.8 27.2Q207.7 28.6 205.3 28.6ZM194.1 69.5"
    "Q190.7 69.5 188.7 67.6Q186.7 65.7 186.7 62.3Q186.7 60.3 187.1 58.4Q187.5 56.4 188.2 54.5"
    "Q188.6 53.4 189.8 50.9Q191.1 48.3 192.8 45.5Q194.5 42.6 196.1 40.6Q197.7 38.6 198.9 38.6"
    "Q199.4 38.6 200.2 39Q201.1 39.3 201.1 40Q201.1 40.4 200.7 41Q198.4 43.6 195.9 47.2"
    "Q193.4 50.8 191.7 54.7Q190 58.6 190 62.2Q190 66.8 194.4 66.8Q197.4 66.8 200.4 63.9"
    "Q203.5 61 207.2 56.1Q208.3 54.7 209.2 53.4Q210 52 210.7 51Q211.4 50.3 212.2 50.3"
    "Q213.4 50.3 213.4 51.2Q213.4 52.3 211.8 54.7Q210.2 57.1 208.2 59.7Q204.3 64.5 201.4 67"
    "Q198.5 69.5 194.1 69.5Z"
)


def hammer(full: bool) -> str:
    """The hammer, turned and placed in the tile, drawn in `currentColor`."""
    haft = HAFT_FULL if full else HAFT_PLAIN
    return (
        '<g transform="translate(12.3 12.5) rotate(40) translate(-12 -11)" fill="currentColor">'
        + haft + HEAD + "</g>"
    )


def mark(full: bool, colours: dict[str, str], size: int | None = None) -> str:
    """The mark as an SVG document in the colours given."""
    dims = f' width="{size}" height="{size}"' if size else ""
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"{dims}>'
        f'<rect width="24" height="24" rx="6.5" fill="{colours["tile"]}"/>'
        f'<g color="{colours["ink"]}">{hammer(full)}</g></svg>'
    )


def mark_inline(full: bool = False) -> str:
    """The mark for a page whose stylesheet colours `.tile` and `.ink`."""
    return (
        '<svg class="mark" viewBox="0 0 24 24" aria-hidden="true">'
        '<rect class="tile" width="24" height="24" rx="6.5"/>'
        f'<g class="ink">{hammer(full)}</g></svg>'
    )


def wordmark(height: float, colour: str = "currentColor", css_class: str = "") -> str:
    """The signature, `height` pixels high, as an inline SVG element."""
    w, h = SIGNATURE_BOX
    attribute = f' class="{css_class}"' if css_class else ""
    return (
        f'<svg{attribute} viewBox="0 0 {w} {h}" width="{height * w / h:.1f}" height="{height}" '
        f'aria-hidden="true"><path d="{SIGNATURE}" fill="{colour}"/></svg>'
    )


def logo(colours: dict[str, str]) -> str:
    """The mark and the wordmark side by side, the signature's capitals centred on the mark."""
    size, gap = 72, 18
    w, h = SIGNATURE_BOX
    height = size * 1.3
    width = height * w / h
    # the middle of the capitals, between the top of the R and the baseline, lies a third
    # of the signature's height from its top; the descender of the y hangs below the mark
    top = size / 2 - height * 0.33
    canvas_w, canvas_h = size + gap + width, top + height
    s = height / h
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canvas_w:.1f} {canvas_h:.1f}" '
        f'width="{canvas_w:.0f}" height="{canvas_h:.0f}" role="img" aria-label="Renyi">'
        f'<g transform="scale(3)"><rect width="24" height="24" rx="6.5" fill="{colours["tile"]}"/>'
        f'<g color="{colours["ink"]}">{hammer(True)}</g></g>'
        f'<path transform="translate({size + gap} {top:.2f}) scale({s:.5f})" d="{SIGNATURE}" '
        f'fill="{colours["text"]}"/></svg>'
    )


SOCIAL = """<!doctype html><html><head><meta charset="utf-8"><style>
html, body { margin: 0; width: 1280px; height: 640px; background: @BG@; }
body { box-sizing: border-box; padding: 0 120px; display: flex; flex-direction: column;
       justify-content: center; position: relative; color: @TEXT@;
       font-family: Inter, "Helvetica Neue", Arial, sans-serif; }
.top { display: flex; align-items: center; gap: 48px; }
.top .name { transform: translateY(17%); }
.lead { margin: 76px 0 0; font-size: 48px; font-weight: 650; letter-spacing: -0.02em; }
.sub { margin: 16px 0 0; font-size: 28px; line-height: 1.4; color: @MUTED@; max-width: 1000px; }
.url { position: absolute; left: 120px; bottom: 52px; font-size: 24px; font-weight: 600;
       color: @TILE@; }
</style></head><body>
<div class="top">@MARK@ @WORDMARK@</div>
<p class="lead">The scripting language of AI agents</p>
<p class="sub">The agent writes it, you review it at a glance, and the program can only do
what it declares.</p>
<div class="url">renyi-lang.org</div>
</body></html>
"""


def social_preview() -> str:
    """The page whose screenshot is the social preview, 1280 by 640."""
    c = LIGHT
    fills = {"@BG@": c["bg"], "@TEXT@": c["text"], "@MUTED@": c["muted"], "@TILE@": c["tile"],
             "@MARK@": mark(True, c, 168), "@WORDMARK@": wordmark(204, c["text"], "name")}
    page = SOCIAL
    for key, value in fills.items():
        page = page.replace(key, value)
    return page


def icon_page(size: int) -> str:
    """The page whose screenshot is the extension's icon: the full mark, transparent around it."""
    return (
        "<!doctype html><html><head><style>html, body { margin: 0; background: transparent; }"
        f"</style></head><body>{mark(True, LIGHT, size)}</body></html>"
    )


def screenshot(browser: str, page: str, width: int, height: int, out: Path) -> None:
    """A screenshot of the page at its size, transparent where the page is."""
    with tempfile.TemporaryDirectory() as work:
        source = Path(work) / "page.html"
        source.write_text(page, encoding="utf-8")
        subprocess.run(
            [browser, "--headless", "--no-sandbox", "--disable-gpu", "--hide-scrollbars",
             "--force-device-scale-factor=1", "--default-background-color=00000000",
             f"--window-size={width},{height}", f"--screenshot={out}", source.as_uri()],
            check=True, capture_output=True,
        )


def main() -> int:
    files = {
        "docs/assets/renyi-mark.svg": mark(True, LIGHT),
        "docs/assets/renyi-logo-light.svg": logo(LIGHT),
        "docs/assets/renyi-logo-dark.svg": logo(DARK),
        "editors/vscode/images/file-light.svg": mark(False, LIGHT),
        "editors/vscode/images/file-dark.svg": mark(False, DARK),
    }
    for name, text in files.items():
        target = ROOT / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text + "\n", encoding="utf-8", newline="\n")
        print(f"wrote {name}")
    if "--png" in sys.argv:
        browser = sys.argv[sys.argv.index("--png") + 1]
        screenshot(browser, icon_page(128), 128, 128, ROOT / "editors/vscode/images/icon.png")
        print("wrote editors/vscode/images/icon.png")
        screenshot(browser, social_preview(), 1280, 640, ROOT / "docs/assets/social-preview.png")
        print("wrote docs/assets/social-preview.png")
    return 0


if __name__ == "__main__":
    sys.exit(main())
