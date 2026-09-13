#!/usr/bin/env bash
# Java 21 parity compliance runner.
# For each tests/parity/java/P*.java: compile with javac 21, run on HotSpot
# (expected) and on Duke (actual), diff stdout + exit code.
# Usage: ./run_parity.sh [PNN...]   (optional filter, e.g. ./run_parity.sh P01 P06)
set -u
REPO="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$REPO/tests/parity/java"
BUILD="$REPO/tests/parity/build"
DUKE="$REPO/target/debug/duke"
TIMEOUT_S=60

export PATH="$HOME/.cargo/bin:$PATH"
mkdir -p "$BUILD"

if [ ! -x "$DUKE" ]; then
  echo "building duke..."
  (cd "$REPO" && cargo build 2>&1 | tail -2)
fi

pass=0; fail=0; failed_tests=()
for src in "$SRC"/P*.java; do
  name="$(basename "$src" .java)"
  if [ $# -gt 0 ]; then
    match=0
    for f in "$@"; do [[ "$name" == "$f"* ]] && match=1; done
    [[ $match == 0 ]] && continue
  fi
  dir="$BUILD/$name"
  rm -rf "$dir"; mkdir -p "$dir"
  if ! javac -d "$dir" "$src" 2>"$dir/javac.err"; then
    echo "COMPILE-FAIL $name"; cat "$dir/javac.err"; fail=$((fail+1)); failed_tests+=("$name"); continue
  fi
  # expected: HotSpot
  timeout $TIMEOUT_S java -cp "$dir" "$name" >"$dir/expected.out" 2>"$dir/expected.err"; exp_code=$?
  # actual: Duke
  timeout $TIMEOUT_S "$DUKE" run "$dir/$name.class" >"$dir/actual.out" 2>"$dir/actual.err"; act_code=$?
  ok=1
  [ $exp_code -ne $act_code ] && ok=0
  cmp -s "$dir/expected.out" "$dir/actual.out" || ok=0
  if [ $ok -eq 1 ]; then
    echo "PASS $name"; pass=$((pass+1))
  else
    echo "FAIL $name (hotspot exit=$exp_code duke exit=$act_code)"
    diff "$dir/expected.out" "$dir/actual.out" | head -20 | sed 's/^/    /'
    [ -s "$dir/actual.err" ] && { echo "    --- duke stderr ---"; head -5 "$dir/actual.err" | sed 's/^/    /'; }
    fail=$((fail+1)); failed_tests+=("$name")
  fi
done
echo "== parity: $pass passed, $fail failed =="
[ ${#failed_tests[@]} -gt 0 ] && echo "failed: ${failed_tests[*]}"
exit $fail
