"""The census of an image `renyi build` writes (decision AT1): the bytes
of machine code per code object and, with the program's bytecode file
beside it (`renyi compile` writes it; the image holds the program in a
binary encoding, decision AT3), per op, the op kinds, and, when numpy is
at hand, a least-squares attribution of the bytes to the op kinds.

usage: python tools/image_census.py <file.ryi> [<file.ryc>] [--summary]
"""

import json
import struct
import sys
from collections import Counter

IMAGE_FORMAT = 8
SECTION_ALIGN = 16384


class Reader:
    def __init__(self, data):
        self.data = data
        self.at = 0

    def take(self, count):
        taken = self.data[self.at : self.at + count]
        if len(taken) != count:
            raise SystemExit("the image is truncated")
        self.at += count
        return taken

    def u8(self):
        return self.take(1)[0]

    def u16(self):
        return struct.unpack("<H", self.take(2))[0]

    def u32(self):
        return struct.unpack("<I", self.take(4))[0]

    def block(self):
        return self.take(self.u32())


def read_image(path):
    """The file's size, the size of the program's encoding and, per code
    object, None or (body bytes, trampoline bytes, loop headers, deopt
    points); the code section's offset is checked to be aligned."""
    reader = Reader(open(path, "rb").read())
    if reader.take(4) != b"RYI\0":
        raise SystemExit(f"{path}: not an image")
    image_format = reader.u32()
    if image_format != IMAGE_FORMAT:
        raise SystemExit(f"{path}: image format {image_format}; this script reads format {IMAGE_FORMAT}")
    reader.block()  # the renyi version
    reader.u32()  # the code format
    reader.block()  # the target
    reader.block()  # the optimisation level
    reader.block()  # the code hash
    program = len(reader.block())
    codes = []
    for _ in range(reader.u32()):
        if reader.u8() == 0:
            codes.append(None)
            continue
        reader.u32()  # the body's offset in the section
        body = reader.u32()
        reader.u32()  # the trampoline's offset
        trampoline = reader.u32()
        headers = [reader.u32() for _ in range(reader.u32())]
        deopts = 0
        for _ in range(reader.u32()):
            reader.u32()  # pc
            reader.u16()  # locals
            for _ in range(reader.u32()):
                reader.u8()
            for _ in range(reader.u32()):
                reader.u8()
            for _ in range(reader.u32()):
                if reader.u8() == 1:
                    reader.u32()
            for _ in range(reader.u32()):
                reader.u32()  # a handled region's target
                reader.u32()  # and the depth below it
            deopts += 1
        # the callees expanded into the code object (decision AU50,
        # format 8): the callee, the call's pc, the region's start, its
        # body and its end, its first slot and its parent region
        for _ in range(reader.u32()):
            for _ in range(5):
                reader.u32()
            reader.u16()
            reader.u32()
        codes.append((body, trampoline, headers, deopts))
    section_offset = reader.u32()
    section_len = reader.u32()
    if section_offset % SECTION_ALIGN != 0 or section_offset < reader.at:
        raise SystemExit(f"{path}: the code section is misplaced")
    if section_offset + section_len != len(reader.data):
        raise SystemExit(f"{path}: the image is truncated")
    return len(reader.data), program, codes


def main():
    args = sys.argv[1:]
    summary = "--summary" in args
    args = [arg for arg in args if arg != "--summary"]
    if len(args) not in (1, 2):
        raise SystemExit(__doc__)
    size, program_size, codes = read_image(args[0])
    metas = None
    if len(args) == 2:
        metas = json.load(open(args[1], encoding="utf-8"))["codes"]
        if len(metas) != len(codes):
            raise SystemExit(f"{args[1]} has {len(metas)} code objects, the image {len(codes)}")
    compiled = [code for code in codes if code is not None]
    total_body = sum(body for body, _, _, _ in compiled)
    total_trampoline = sum(trampoline for _, trampoline, _, _ in compiled)
    total_deopts = sum(deopts for _, _, _, deopts in compiled)
    print(
        f"image {size} bytes: the program {program_size}, machine code {total_body} in bodies"
        f" and {total_trampoline} in trampolines; {len(compiled)} of {len(codes)} code objects compiled"
    )
    if metas is None:
        print(f"{total_body / max(len(compiled), 1):.0f} bytes of body per code object; {total_deopts} deopt points (the bytecode file would give the ops)")
        return
    rows = []
    kinds_total = Counter()
    total_ops = 0
    for meta, code in zip(metas, codes):
        if code is None:
            continue
        body, trampoline, headers, deopts = code
        kinds = Counter(op["kind"] for op in meta["ops"])
        kinds_total.update(kinds)
        rows.append((meta["name"], body, trampoline, len(meta["ops"]), deopts, meta["locals"], len(headers), kinds))
        total_ops += len(meta["ops"])
    print(
        f"{total_ops} ops, {total_body / max(total_ops, 1):.1f} bytes of body per op,"
        f" {total_body / max(len(rows), 1):.0f} per code object; {total_deopts} deopt points"
    )
    if summary:
        return
    print("\nthe largest bodies:")
    for name, body, _, ops, deopts, locals_, headers, _ in sorted(rows, key=lambda row: -row[1])[:15]:
        print(f"  {body:8} B {ops:5} ops {body / ops:6.1f} B/op  deopts {deopts:4} locals {locals_:3} headers {headers:2}  {name}")
    print("\nthe op kinds:")
    for kind, count in kinds_total.most_common():
        print(f"  {kind:22} {count:7} {100 * count / total_ops:5.1f}%")
    try:
        import numpy as np
    except ImportError:
        print("\n(numpy is not installed: no attribution of the bytes to the op kinds)")
        return
    kinds = [kind for kind, _ in kinds_total.most_common()]
    names = kinds + ["deopt point", "loop header", "per code object"]
    matrix = np.array(
        [[kc.get(kind, 0) for kind in kinds] + [deopts, headers, 1] for _, _, _, _, deopts, _, headers, kc in rows],
        float,
    )
    bodies = np.array([row[1] for row in rows], float)
    coefficients, *_ = np.linalg.lstsq(matrix, bodies, rcond=None)
    predicted = matrix @ coefficients
    r2 = 1 - ((bodies - predicted) ** 2).sum() / ((bodies - bodies.mean()) ** 2).sum()
    print(f"\nthe bytes attributed to the op kinds by least squares over {len(rows)} code objects (r2 {r2:.3f}):")
    print("  kind                   bytes each    count   total bytes   share")
    contributions = [(name, coefficients[i], matrix[:, i].sum()) for i, name in enumerate(names)]
    for name, each, count in sorted(contributions, key=lambda item: -abs(item[1] * item[2])):
        total = each * count
        print(f"  {name:22} {each:9.1f} {int(count):8} {total:12.0f} {100 * total / total_body:6.1f}%")


if __name__ == "__main__":
    main()
