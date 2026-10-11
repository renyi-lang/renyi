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
# then the census of the image (tools/image_census.py, with the bytecode
# file for the ops): the bytes of machine code per code object and per op.
# The first three rows run with the caches off (decision AU10: the cold
# JIT run is the row that decides); a fourth, `cached run`, is the same
# command as the JIT run with the program's image (AU10) and, since
# decision AU43, the program itself in a cache of the script's own, the
# second run of a program as a user sees it (a binary without the cache
# prints `-`). Deterministic, so two binaries
# compare without repetition; about ten minutes a binary. The image is
# built under valgrind too, since valgrind hides some CPU features and an
# image built outside it is refused inside it.
#
#   tools/measure_size.sh <binary>          (from the repository root)
set -u
if [ $# -ne 1 ]; then echo "usage: tools/measure_size.sh <renyi binary>" >&2; exit 2; fi
export RENYI_NO_CACHE=1
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
echo "== $bin: the self-check (bench/selfcheck/checker.ry on bench/selfcheck/bodies.ry), cachegrind"
printf '%-14s %s\n' "JIT run" "$(counts "$bin" run bench/selfcheck/checker.ry bench/selfcheck/bodies.ry)"
# the same with every function compiled on the thread that runs it: the
# compile thread's share varies under valgrind, so this row is the one two
# binaries compare on (decision AU49's measure)
printf '%-14s %s\n' "JIT run, sync" "$(RENYI_NATIVE_SYNC=1 counts "$bin" run bench/selfcheck/checker.ry bench/selfcheck/bodies.ry)"
printf '%-14s %s\n' "interpreter" "$(counts "$bin" run --interpret bench/selfcheck/checker.ry bench/selfcheck/bodies.ry)"
valgrind --tool=none "$bin" build --to "$work/selfcheck.ryi" bench/selfcheck/checker.ry 2>/dev/null
printf '%-14s %s\n' "image (speed)" "$(counts "$bin" run "$work/selfcheck.ryi" bench/selfcheck/bodies.ry)"
# the cache filled under valgrind too, for the same reason as the image
cached=$(unset RENYI_NO_CACHE; export RENYI_CACHE_DIR="$work/cache"
  if valgrind --tool=none "$bin" build --cache bench/selfcheck/checker.ry >/dev/null 2>&1; then
    counts "$bin" run bench/selfcheck/checker.ry bench/selfcheck/bodies.ry
  else
    echo "-"
  fi)
printf '%-14s %s\n' "cached run" "$cached"
echo "== the image's census"
"$bin" compile --to "$work/selfcheck.ryc" bench/selfcheck/checker.ry 2>/dev/null
python3 "$(dirname "$0")/image_census.py" "$work/selfcheck.ryi" "$work/selfcheck.ryc" --summary
