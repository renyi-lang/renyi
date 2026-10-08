#!/bin/bash
# The measurement of the round on the size of the generated code
# (decision AT1): cachegrind with the cache and branch simulation on the
# compiler's self-check, on the JIT run, on the interpreter and on the
# image `renyi build` writes, and from its counts KCachegrind's estimate
# of the cycles, which is the round's rule:
#
#   CEst = Ir + 10 * (I1 misses + D1 misses) + 10 * conditional mispredicts
#             + 20 * indirect mispredicts + 100 * LL misses
#
# then the census of the image (tools/image_census.py): the bytes of
# machine code per code object and per op. Deterministic, so two binaries
# compare without repetition; about ten minutes a binary. The image is
# built under valgrind too, since valgrind hides some CPU features and an
# image built outside it is refused inside it.
#
#   tools/measure_size.sh <binary>          (from the repository root)
set -u
if [ $# -ne 1 ]; then echo "usage: tools/measure_size.sh <renyi binary>" >&2; exit 2; fi
bin=$1
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
counts() {
  # the summary lines of cachegrind, as "name value" pairs
  valgrind --tool=cachegrind --cache-sim=yes --branch-sim=yes --cachegrind-out-file=/dev/null "$@" 2>&1 >/dev/null \
    | sed -n 's/^==[0-9]*== *//p' | tr -d ',' | awk '
      /^I +refs:/ {ir=$3}
      /^I1 +misses:/ {i1=$3}
      /^D1 +misses:/ {d1=$3}
      /^LL misses:/ {ll=$3}
      /^Mispredicts:/ {bm=$4; bi=$7}
      END {
        est = ir + 10*(i1+d1) + 10*bm + 20*bi + 100*ll
        printf "Ir %d I1m %d D1m %d LLm %d Bm %d Bi %d CEst %d\n", ir, i1, d1, ll, bm, bi, est
      }'
}
echo "== $bin: the self-check (compiler/checker.ry on compiler/bodies.ry), cachegrind"
printf '%-14s %s\n' "JIT run" "$(counts "$bin" run compiler/checker.ry compiler/bodies.ry)"
printf '%-14s %s\n' "interpreter" "$(counts "$bin" run --interpret compiler/checker.ry compiler/bodies.ry)"
valgrind --tool=none "$bin" build --to "$work/selfcheck.ryi" compiler/checker.ry 2>/dev/null
printf '%-14s %s\n' "image (speed)" "$(counts "$bin" run "$work/selfcheck.ryi" compiler/bodies.ry)"
echo "== the image's census"
python3 "$(dirname "$0")/image_census.py" "$work/selfcheck.ryi" --summary
