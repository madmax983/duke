#!/usr/bin/env bash
# Duke vs HotSpot wall-clock comparison.
# Usage: bash benchmarks/compare.sh
# Run from workspace root.
set -e

BENCHMARKS_DIR="benchmarks"
CLASS="$BENCHMARKS_DIR/BenchmarkSuite.class"

# Number of runs per measurement; reported value is the median (odd N recommended).
RUNS="${RUNS:-7}"

echo "Building Duke (release)..."
cargo build --release --quiet 2>&1
DUKE="./target/release/duke"

# median <ms...> : print the median of the integer millisecond samples passed as args.
median() {
    printf '%s\n' "$@" | sort -n | awk '{a[NR]=$0} END{print a[int((NR+1)/2)]}'
}

# median_ms <command...> : time the command RUNS times, return the median wall-clock ms.
median_ms() {
    local samples=()
    local i secs
    for ((i = 0; i < RUNS; i++)); do
        secs=$( { TIMEFORMAT='%R'; time "$@" > /dev/null 2>/dev/null; } 2>&1 )
        samples+=("$(echo "$secs" | LC_NUMERIC=C awk '{printf "%.0f", $1 * 1000}')")
    done
    median "${samples[@]}"
}

echo ""
echo "Reporting median of ${RUNS} runs per cell (override with RUNS=N)."
printf "%-30s %10s %12s %14s %12s\n" "Benchmark" "Duke (ms)" "HS JIT (ms)" "HS -Xint (ms)" "Duke/JIT"
printf "%s\n" "$(printf '=%.0s' {1..80})"

for ENTRY in "sum:benchSum" "fib:benchFib" "arraylist:benchArrayList" "hashmap:benchHashMap"; do
    ARG="${ENTRY%%:*}"
    METHOD="${ENTRY##*:}"

    # Median wall-clock for Duke, HotSpot JIT, and HotSpot -Xint (interpreter only).
    DUKE_MS=$(median_ms "$DUKE" exec "$CLASS" "$METHOD")
    HS_MS=$(median_ms java -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG")
    XI_MS=$(median_ms java -Xint -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG")

    # Ratio
    RATIO=$(echo "$DUKE_MS $HS_MS" | awk '{if ($2 > 0) printf "%.0fx", $1/$2; else print "N/A"}')

    printf "%-30s %10s %12s %14s %12s\n" "$METHOD" "${DUKE_MS}" "${HS_MS}" "${XI_MS}" "$RATIO"
done

echo ""
echo "Notes:"
echo "  Duke includes: cargo binary startup + class parse + bootstrap_stdlib + execution"
echo "  HotSpot includes: JVM startup + execution (JIT or interpreter)"
echo "  HotSpot -Xint disables JIT compilation (pure interpreter comparison)"
