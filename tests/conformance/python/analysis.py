"""The Python side of the bridge's conformance fixture: what `analysis.ry`
declares. `mean` on the Renyi side is `average` here (the manifest's
`symbols`); the others keep their names."""


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
