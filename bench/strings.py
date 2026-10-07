"""The same text building and digit count as strings.ry, for CPython."""

pieces = []
for index in range(1, 100001):
    pieces.append(f"item {index}; ")
joined = "".join(pieces)
found = sum(1 for glyph in joined if glyph in "0123456789")
print(len(joined), found)
