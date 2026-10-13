#!/usr/bin/env python3
"""The art of the showcase game, taken from Freedoom (decision AW35,
stage 36b).

Freedoom (https://github.com/freedoom/freedoom) keeps its sprites, wall
patches, graphics and sounds as PNG and WAV files under the 3-clause BSD
license, with the lump names of the game it stands in for. This script
downloads the files the showcase uses, at one pinned commit, into a
directory; keeps the license beside them (`COPYING.adoc`, verbatim) with
a `README.md` of what came from where; and writes `art.ry`, the module
that lists every image with its size in pixels and its anchor (the
offsets of the `grAb` chunk a sprite carries) and every sound, which the
game draws from by name. The page serves the directory as it is: a
`Strip` or a `Picture` names an image by its file's stem, `play` a sound
by its.

Usage: python -I tools/freedoom_art.py <art directory> --ry <art.ry>
    [--commit <sha>] [--base <url>]

The directory is made when missing, and a file already there is not
downloaded again. The downloads are untrusted data: run the script from
the repository, not from the directory it fills.
"""

from __future__ import annotations

import argparse
import struct
import sys
import time
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPOSITORY = "https://github.com/freedoom/freedoom"
COMMIT = "3ac019628d70b812ffb722f7f2365771b76a9d29"
BASE = "https://raw.githubusercontent.com/freedoom/freedoom"


def demon_frames() -> list[tuple[str, str]]:
    """The demon's sprites: eight frames in five rotations (a rotation
    seen from the other side is the same file mirrored), then six of its
    death, as Freedoom names them."""
    frames = []
    for letter in "abcdefgh":
        frames.append((f"sarg{letter}1", f"frame {letter.upper()}, facing the player"))
        frames.append((f"sarg{letter}2{letter}8", f"frame {letter.upper()}, turned 45 degrees"))
        frames.append((f"sarg{letter}3{letter}7", f"frame {letter.upper()}, side on"))
        frames.append((f"sarg{letter}4{letter}6", f"frame {letter.upper()}, turned 135 degrees"))
        frames.append((f"sarg{letter}5", f"frame {letter.upper()}, from behind"))
    for letter in "ijklmn":
        frames.append((f"sarg{letter}0", f"dying, frame {letter.upper()}"))
    return frames


def face_frames() -> list[tuple[str, str]]:
    """The player's face: five levels of health in three looks, then
    hurt, grinning, killed, dead and invulnerable."""
    frames = []
    for level in range(5):
        for look in range(3):
            frames.append((f"stfst{level}{look}", f"health level {level}, look {look}"))
    for level in range(5):
        frames.append((f"stfouch{level}", f"hurt, health level {level}"))
    for level in range(5):
        frames.append((f"stfevl{level}", f"grinning, health level {level}"))
    for level in range(5):
        frames.append((f"stfkill{level}", f"killed, health level {level}"))
    frames.append(("stfdead0", "dead"))
    frames.append(("stfgod0", "invulnerable"))
    return frames


def font_glyphs() -> list[tuple[str, str]]:
    """The large font, one file per character by its ASCII code."""
    codes = list(range(33, 96)) + [121, 123, 124, 125]
    return [(f"stcfn{code:03}", f"the character {chr(code)!r}") for code in codes]


# The groups of files: the name of the group, the directory of Freedoom
# the files come from, what the group is for, and the files as (stem,
# what it is).
GROUPS: list[tuple[str, str, str, list[tuple[str, str]]]] = [
    (
        "walls",
        "patches",
        "The wall textures, 64 or 128 columns by 128 rows; a strip takes a column "
        "modulo the width. The names in capitals are the textures of the original "
        "game that Freedoom builds from these patches.",
        [
            ("sw19_2", "STARTAN3, the left half: grey panels on tan"),
            ("sw19_1", "STARTAN3, the right half"),
            ("rw10_1", "BRONZE1: riveted brown metal"),
            ("rw1_4", "BRICK2: brown brick"),
            ("wall47_1", "METAL: dark green metal"),
            ("wla128_1", "SLADWALL: green panels"),
            ("wall62_1", "BROWN96: brown metal, 128 wide"),
            ("mwall1_1", "MARBLE1: green marble, 128 wide"),
            ("w94_1", "TEKWALL4: circuitry, 128 wide"),
            ("door2_4", "BIGDOOR2: a grey door, 128 wide"),
            ("door9_2", "BIGDOOR3: a green door, 128 wide"),
        ],
    ),
    (
        "sky",
        "patches",
        "The sky, 256 columns by 128 rows, drawn stretched behind the walls.",
        [("rsky1", "SKY1: brown clouds")],
    ),
    (
        "demon",
        "sprites",
        "The demon: walking A to D, attacking E to G, hurt H, each in five rotations "
        "(1 facing the player, 2/8 turned 45 degrees, 3/7 side on, 4/6 turned 135 "
        "degrees, 5 from behind; 6 to 8 are 2 to 4 mirrored), dying I to N. The "
        "offsets are the anchor: the column of the sprite's middle and the row of "
        "its feet, from the top left.",
        demon_frames(),
    ),
    (
        "shotgun",
        "sprites",
        "The shotgun in the player's hands: still A, firing B to D; the muzzle flash "
        "A and B, drawn over it.",
        [
            ("shtga0", "the shotgun, still"),
            ("shtgb0", "firing, frame B: pulled back"),
            ("shtgc0", "firing, frame C: pumped"),
            ("shtgd0", "firing, frame D: the pump returning"),
            ("shtfa0", "the muzzle flash, frame A"),
            ("shtfb0", "the muzzle flash, frame B"),
        ],
    ),
    (
        "status bar",
        "graphics",
        "The status bar, 320 by 32, and what goes on it: the large red digits, the "
        "minus and the percent sign, the small yellow digits and the arms panel.",
        [("stbar", "the bar"), ("starms", "the arms panel")]
        + [(f"sttnum{digit}", f"the large digit {digit}") for digit in range(10)]
        + [("sttminus", "the large minus"), ("sttprcnt", "the large percent sign")]
        + [(f"stysnum{digit}", f"the small digit {digit}") for digit in range(10)],
    ),
    (
        "face",
        "graphics",
        "The player's face on the status bar, by health.",
        face_frames(),
    ),
    (
        "font",
        "graphics",
        "The large font, one file per character: STCFN followed by the ASCII code, "
        "33 (!) to 95 (_), then 121, 123, 124 and 125.",
        font_glyphs(),
    ),
    (
        "title",
        "graphics/titlepic",
        "The title picture, 320 by 200.",
        [("titlepic", "the title picture")],
    ),
    (
        "sounds",
        "sounds",
        "The sounds, WAV files.",
        [
            ("dsshotgn", "the shotgun fired"),
            ("dssgcock", "the shotgun pumped"),
            ("dssgtsit", "the demon sighting the player"),
            ("dssgtatk", "the demon biting"),
            ("dssgtdth", "the demon dying"),
            ("dsdmact", "the demon roaming"),
            ("dsplpain", "the player hurt"),
            ("dspldeth", "the player dying"),
            ("dsdoropn", "a door opening"),
            ("dsdorcls", "a door closing"),
            ("dsnoway", "a wall or a locked door bumped"),
            ("dsitemup", "an item picked up"),
        ],
    ),
]


def fetch(url: str, path: Path) -> int:
    """The file at the URL written to the path, in three tries; its
    size."""
    for attempt in range(3):
        try:
            with urllib.request.urlopen(url, timeout=60) as answer:
                data = answer.read()
            path.write_bytes(data)
            return len(data)
        except OSError as error:
            if attempt == 2:
                raise SystemExit(f"cannot download {url}: {error}") from error
            time.sleep(2)
    return 0


def png_head(data: bytes) -> tuple[int, int, int, int]:
    """A PNG's width and height from its IHDR chunk and the offsets of
    its grAb chunk, 0 and 0 without one."""
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit("not a PNG file")
    width = height = 0
    offset_x = offset_y = 0
    position = 8
    while position + 8 <= len(data):
        length, kind = struct.unpack(">I4s", data[position : position + 8])
        body = data[position + 8 : position + 8 + length]
        if kind == b"IHDR":
            width, height = struct.unpack(">II", body[:8])
        elif kind == b"grAb":
            offset_x, offset_y = struct.unpack(">ii", body[:8])
        elif kind == b"IEND":
            break
        position += 12 + length
    return width, height, offset_x, offset_y


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("directory", type=Path, help="where the art goes")
    parser.add_argument("--ry", type=Path, required=True, help="the module to write")
    parser.add_argument("--commit", default=COMMIT, help="the commit of Freedoom to take")
    parser.add_argument("--base", default=BASE, help="the raw files' URL before the commit")
    args = parser.parse_args()
    directory: Path = args.directory
    directory.mkdir(parents=True, exist_ok=True)
    root = f"{args.base}/{args.commit}"

    wanted: list[tuple[str, str, Path]] = []
    for group, source, _, files in GROUPS:
        extension = "wav" if group == "sounds" else "png"
        for stem, _ in files:
            wanted.append((group, f"{source}/{stem}.{extension}", directory / f"{stem}.{extension}"))
    wanted.append(("license", "COPYING.adoc", directory / "COPYING.adoc"))
    missing = [(path, item) for group, item, path in wanted if not path.is_file()]
    print(f"{len(wanted)} files, {len(missing)} to download", file=sys.stderr)
    with ThreadPoolExecutor(max_workers=8) as pool:
        list(pool.map(lambda pair: fetch(f"{root}/{pair[1]}", pair[0]), missing))

    heads: dict[str, tuple[int, int, int, int]] = {}
    sizes: dict[str, int] = {}
    for group, item, path in wanted:
        data = path.read_bytes()
        sizes[path.name] = len(data)
        if path.suffix == ".png":
            heads[path.stem] = png_head(data)
        elif path.suffix == ".wav" and data[:4] != b"RIFF":
            raise SystemExit(f"{path} is not a WAV file")

    readme = [
        "# The art of the showcase game\n",
        f"Taken from [Freedoom]({REPOSITORY}) at commit `{args.commit}` by",
        "`tools/freedoom_art.py`, which also writes `art.ry` beside this directory.",
        "Freedoom is published under the 3-clause BSD license, kept here verbatim as",
        "`COPYING.adoc`; the copyright is its authors'. The names are the lump names",
        "Freedoom uses; a `Strip` or a `Picture` of `std.screen` names an image by its",
        "file's stem, `play` a sound by its.\n",
    ]
    lines = []
    for group, source, about, files in GROUPS:
        readme.append(f"## {group}\n")
        readme.append(about + "\n")
        readme.append("| file | source | size | what |")
        readme.append("|------|--------|------|------|")
        for stem, what in files:
            if group == "sounds":
                name = f"{stem}.wav"
                size = f"{sizes[name]:,} bytes"
            else:
                name = f"{stem}.png"
                width, height, offset_x, offset_y = heads[stem]
                size = f"{width} by {height}"
                if (offset_x, offset_y) != (0, 0):
                    size += f", anchor {offset_x}, {offset_y}"
                lines.append(
                    f'  Image(name: "{stem}", width: {width}, height: {height}, '
                    f"offset_x: {offset_x}, offset_y: {offset_y})"
                )
            readme.append(f"| `{name}` | `{source}/{name}` | {size} | {what} |")
        readme.append("")
    (directory / "README.md").write_text("\n".join(readme), encoding="utf-8", newline="\n")

    sounds = [stem for group, _, _, files in GROUPS if group == "sounds" for stem, _ in files]
    first = lines[0].strip()
    module = [
        "module art",
        "  purpose: The images and the sounds of the art; written by tools/freedoom_art.py, not by hand.",
        "",
        "public type Image",
        "  purpose: An image of the art: its name, its size in pixels and its anchor, 0 and 0 when none.",
        "  has name: Text",
        "  has width: Integer",
        "  has height: Integer",
        "  has offset_x: Integer",
        "  has offset_y: Integer",
        "  can ToText",
        "end",
        "",
        "public let images: List of Image be [",
        ",\n".join(lines),
        "]",
        "  purpose: Every image of the art, in the order of the README's groups.",
        "",
        "public let sounds: List of Text be [",
        ",\n".join(f'  "{stem}"' for stem in sounds),
        "]",
        "  purpose: Every sound of the art, by name.",
        "",
        "public function image(name: Text) returns Image",
        "  purpose: The image of the name; a name that is not in the art is a crash.",
        f"  example: image(\"{lines[0].split(chr(34))[1]}\")",
        f"    is {first}",
        "",
        "  for each candidate in images",
        "    if candidate.name is name then return candidate end",
        "  end",
        '  crash with "no image named {name} in the art"',
        "end",
        "",
    ]
    args.ry.write_text("\n".join(module), encoding="utf-8", newline="\n")
    total = sum(sizes.values())
    print(
        f"{len(heads)} images, {len(sounds)} sounds, {total:,} bytes in {directory}; "
        f"{args.ry} written",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
