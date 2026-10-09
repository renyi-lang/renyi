#!/bin/bash
# The measurements of the baseline JIT (decisions AR1 and AR4): the
# instruction counts of a binary, by cachegrind, on the compiler's
# self-check and on bench/records.ry on both tiers, then the micro
# programs of bench/micro/ per turn of their loop (the counts at two sizes,
# their difference over the difference of the sizes). Deterministic, so
# two binaries compare without repetition; about ten minutes a binary.
#
#   tools/measure_native.sh <binary>          (from the repository root)
#   RENYI_NATIVE_HOT=4000 tools/measure_native.sh <binary>   (a tiering)
set -u
if [ $# -ne 1 ]; then echo "usage: tools/measure_native.sh <renyi binary>" >&2; exit 2; fi
# the cold runs: no image from the cache (decision AU10)
export RENYI_NO_CACHE=1
bin=$1
irefs() {
  valgrind --tool=cachegrind --cache-sim=no --cachegrind-out-file=/dev/null "$@" 2>&1 >/dev/null \
    | grep "I refs" | sed 's/.*I refs: *//; s/,//g'
}
echo "== $bin: instructions"
for mode in "" "--interpret"; do
  n=$(irefs "$bin" run $mode compiler/checker.ry compiler/bodies.ry)
  printf '%-40s %-12s %16s\n' "compiler/checker.ry compiler/bodies.ry" "${mode:-native}" "$n"
done
for mode in "" "--interpret"; do
  n=$(irefs "$bin" run $mode bench/records.ry)
  printf '%-40s %-12s %16s\n' "bench/records.ry" "${mode:-native}" "$n"
done
echo "== bench/micro: instructions per turn"
for mode in "" "--interpret"; do
  a=$(irefs "$bin" run $mode bench/micro/call_integer_1000000.ry)
  b=$(irefs "$bin" run $mode bench/micro/call_integer_200000.ry)
  printf '%-40s %-12s %16s\n' "call_integer (literal bound)" "${mode:-native}" $(( (a - b) / 800000 ))
done
for prog in call_integer field_read call_record record_build; do
  for mode in "" "--interpret"; do
    a=$(irefs "$bin" run $mode bench/micro/$prog.ry 1000000)
    b=$(irefs "$bin" run $mode bench/micro/$prog.ry 200000)
    printf '%-40s %-12s %16s\n' "$prog" "${mode:-native}" $(( (a - b) / 800000 ))
  done
done
