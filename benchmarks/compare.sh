#!/usr/bin/env bash
# Duke vs HotSpot wall-clock comparison.
# Usage: bash benchmarks/compare.sh
# Run from workspace root.
set -e

BENCHMARKS_DIR="benchmarks"
CLASS="$BENCHMARKS_DIR/BenchmarkSuite.class"

echo "Building Duke (release)..."
cargo build --release --quiet 2>&1
DUKE="./target/release/duke"

echo ""
printf "%-30s %10s %12s %14s %12s\n" "Benchmark" "Duke (ms)" "HS JIT (ms)" "HS -Xint (ms)" "Duke/JIT"
printf "%s\n" "$(printf '=%.0s' {1..80})"

for ENTRY in "sum:benchSum" "fib:benchFib" "arraylist:benchArrayList" "hashmap:benchHashMap"; do
    ARG="${ENTRY%%:*}"
    METHOD="${ENTRY##*:}"

    # Time Duke
    DUKE_SECS=$( { TIMEFORMAT='%R'; time "$DUKE" exec "$CLASS" "$METHOD" > /dev/null 2>/dev/null; } 2>&1 )
    DUKE_MS=$(echo "$DUKE_SECS" | LC_NUMERIC=C awk '{printf "%.0f", $1 * 1000}')

    # Time HotSpot JIT
    HS_SECS=$( { TIMEFORMAT='%R'; time java -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG" > /dev/null 2>/dev/null; } 2>&1 )
    HS_MS=$(echo "$HS_SECS" | LC_NUMERIC=C awk '{printf "%.0f", $1 * 1000}')

    # Time HotSpot -Xint (interpreter only, no JIT)
    XI_SECS=$( { TIMEFORMAT='%R'; time java -Xint -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG" > /dev/null 2>/dev/null; } 2>&1 )
    XI_MS=$(echo "$XI_SECS" | LC_NUMERIC=C awk '{printf "%.0f", $1 * 1000}')

    # Ratio
    RATIO=$(echo "$DUKE_MS $HS_MS" | awk '{if ($2 > 0) printf "%.0fx", $1/$2; else print "N/A"}')

    printf "%-30s %10s %12s %14s %12s\n" "$METHOD" "${DUKE_MS}" "${HS_MS}" "${XI_MS}" "$RATIO"
done

echo ""
echo "Notes:"
echo "  Duke includes: cargo binary startup + class parse + bootstrap_stdlib + execution"
echo "  HotSpot includes: JVM startup + execution (JIT or interpreter)"
echo "  HotSpot -Xint disables JIT compilation (pure interpreter comparison)"
