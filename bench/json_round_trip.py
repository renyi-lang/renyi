"""The same JSON round trips as json_round_trip.ry, for CPython."""

import json

entries = [
    {"name": f"entry {index}", "score": index, "labels": ["a", "b"]}
    for index in range(1, 2001)
]
total = 0
for _ in range(50):
    back = json.loads(json.dumps(entries))
    total += sum(entry["score"] for entry in back)
print(total)
