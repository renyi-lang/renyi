"""The Python side of the bridge's conformance fixture: what `analysis.ry`
declares. `mean` on the Renyi side is `average` here (the manifest's
`symbols`); the others keep their names. `stall` sleeps past the
manifest's deadline, and `shift` takes and gives back a dataclass, which
the worker builds from the JSON object and renders back (decision AW49)."""

import time
from dataclasses import dataclass


@dataclass
class Point:
    across: int
    down: int


def average(values):
    return sum(values) / len(values)


def describe(sample):
    return {
        "name": sample["name"].upper(),
        "total": len(sample["labels"]),
        "labels": sorted(sample["labels"]),
    }


def shout(text):
    return text.upper() + "!"


def explode(message):
    raise ValueError(message)


def oddly(times):
    return "not a number"


def stall(seconds):
    time.sleep(seconds)
    return seconds


def shift(point: Point, by: int) -> Point:
    return Point(point.across + by, point.down + by)
