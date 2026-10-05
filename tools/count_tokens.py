#!/usr/bin/env python3
"""Count tokens of the cheat sheet (decision D1: hard budget of 3000 tokens).

Renyi's design budget is defined against general-purpose LLM tokenizers. Two
public encodings are measured and the larger count is the gate, so the budget
is conservative. Exit status is 1 when the gate is exceeded.
"""
import sys
import pathlib
import tiktoken

BUDGET = 3000
BYTES_PER_TOKEN = 4.0
DEFAULT = pathlib.Path(__file__).resolve().parent.parent / "docs" / "cheatsheet.md"


def main() -> int:
    path = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT
    text = path.read_text(encoding="utf-8")
    counts = {}
    for name in ("o200k_base", "cl100k_base"):
        try:
            counts[name] = len(tiktoken.get_encoding(name).encode(text))
        except Exception as error:  # the encodings are fetched from the network on first use
            print(f"{name:12s} unavailable ({type(error).__name__}); estimating from bytes")
    if not counts:
        # 4.0 bytes per token was measured on the cheat sheet with o200k_base and cl100k_base
        counts["estimate"] = round(len(text.encode("utf-8")) / BYTES_PER_TOKEN)
    worst = max(counts.values())
    for name, count in counts.items():
        print(f"{name:12s} {count:5d} tokens")
    print(f"{'gate (max)':12s} {worst:5d} / {BUDGET}  {'OK' if worst <= BUDGET else 'OVER BUDGET'}")
    print(f"{'bytes':12s} {len(text.encode('utf-8')):5d}   lines {text.count(chr(10))}")
    return 0 if worst <= BUDGET else 1


if __name__ == "__main__":
    sys.exit(main())
