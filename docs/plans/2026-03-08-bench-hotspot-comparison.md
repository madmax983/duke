# Duke vs HotSpot Benchmark — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add Criterion microbenchmarks for Duke's interpreter and a shell comparison script that shows Duke vs HotSpot JIT vs HotSpot -Xint wall-clock times.

**Architecture:** Four benchmark programs in a new `benchmarks/` directory (Java source + compiled .class files). Criterion benchmarks in `crates/duke-interpreter/benches/interpreter.rs` call `execute_class` directly (same pattern as `run_bootstrap_int`). A bash comparison script times Duke and HotSpot for each program and prints a table. Results are recorded in `benchmarks/RESULTS.md`.

**Tech Stack:** Rust, criterion 0.5, bash (Git Bash on Windows), javac 21, java 21

---

## Background

### What we're measuring

| Dimension | Duke | HotSpot JIT | HotSpot -Xint |
|-----------|------|-------------|----------------|
| JIT | No | Yes (tiered C1+C2) | No |
| GC | Bump-pointer, no collection | G1 | G1 |
| Expected speed | baseline | ~100-1000x faster | ~10-50x faster |

### Benchmark programs

Four programs cover different aspects of the interpreter:

| Program | What it stresses |
|---------|-----------------|
| `benchSum` | Arithmetic loop (iconst, iadd, iinc, if_icmplt) |
| `benchFib` | Recursive method dispatch (invokestatic, ireturn) |
| `benchArrayList` | Heap allocation + ArrayList natives |
| `benchHashMap` | HashMap linear scan + String allocation via invokedynamic |

### Criterion setup pattern (mirrors `run_bootstrap_int`)

```rust
use duke_interpreter::{bootstrap_stdlib, build_class_context, execute_class, ClassRegistry};
use duke_loader::DirectoryLoader;

fn bench_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap()   // crates/
        .parent().unwrap()   // workspace root
        .join("benchmarks")
}

fn run_bench(class: &str, method: &str, desc: &str) -> Option<duke_runtime::Slot> {
    let dir = bench_dir();
    let bytes = std::fs::read(dir.join(class)).unwrap();
    let cf = duke_classfile::ClassFile::parse(&bytes).unwrap();
    let ctx = build_class_context(&cf);
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = DirectoryLoader::new(&dir);
    execute_class(&mut registry, &loader, &mut heap, &mut std::io::sink(),
        "BenchmarkSuite", method, desc, &[]).unwrap()
}
```

Each Criterion iteration includes full setup (parse + bootstrap_stdlib). This is intentional: it measures the same "cold start" cost that the wall-clock comparison measures, keeping both apples-to-apples.

### Wall-clock comparison method

Bash script uses `TIMEFORMAT='%R'` with bash's built-in `time` to capture seconds, then converts to milliseconds with `awk`. Includes Duke startup (cargo run) + HotSpot startup (JVM launch). For large-N benchmarks the computation dominates.

### `BenchmarkSuite.java` design

- Static methods `benchSum()`, `benchFib()`, `benchArrayList()`, `benchHashMap()` — return `int`, usable by `duke exec`
- `main(String[])` — takes benchmark name as arg[0], calls the method once (HotSpot comparison via shell timing)
- Compiled to `benchmarks/BenchmarkSuite.class`

---

## Task 1: Java Benchmark Programs

**Files:**
- Create: `benchmarks/BenchmarkSuite.java`
- Create: `benchmarks/BenchmarkSuite.class` (compiled)

### Step 1: Create `benchmarks/` directory and Java source

Create `benchmarks/BenchmarkSuite.java`:

```java
import java.util.ArrayList;
import java.util.HashMap;

public class BenchmarkSuite {

    // Arithmetic loop: sum 0..N-1 (wraps silently in int)
    static int benchSum() {
        int sum = 0;
        for (int i = 0; i < 500_000; i++) sum += i;
        return sum;
    }

    // Recursive fibonacci — exercises method dispatch
    static int fib(int n) {
        if (n <= 1) return n;
        return fib(n - 1) + fib(n - 2);
    }
    static int benchFib() {
        return fib(25);  // 75025; ~500k recursive calls
    }

    // ArrayList: add N boxed ints, return size
    static int benchArrayList() {
        ArrayList<Integer> list = new ArrayList<>();
        for (int i = 0; i < 5_000; i++) {
            list.add(Integer.valueOf(i));
        }
        return list.size();  // 5000
    }

    // HashMap: put 200 string-keyed entries, then get all, return checksum
    static int benchHashMap() {
        HashMap<String, Integer> map = new HashMap<>();
        for (int i = 0; i < 200; i++) {
            map.put("key" + i, Integer.valueOf(i));
        }
        int sum = 0;
        for (int i = 0; i < 200; i++) {
            Integer v = (Integer) map.get("key" + i);
            if (v != null) sum += v.intValue();
        }
        return sum;  // 200*199/2 = 19900
    }

    // main: HotSpot comparison — takes benchmark name as arg, runs it once
    public static void main(String[] args) {
        String name = args.length > 0 ? args[0] : "";
        int result;
        if ("sum".equals(name)) {
            result = benchSum();
        } else if ("fib".equals(name)) {
            result = benchFib();
        } else if ("arraylist".equals(name)) {
            result = benchArrayList();
        } else if ("hashmap".equals(name)) {
            result = benchHashMap();
        } else {
            // default: run all so HotSpot can warm up
            benchSum(); benchFib(); benchArrayList();
            result = benchHashMap();
        }
        // Print result as sanity check
        System.out.println(result);
    }
}
```

### Step 2: Compile

```bash
cd /c/Users/markm/duke
mkdir -p benchmarks
javac --release 21 -d benchmarks benchmarks/BenchmarkSuite.java
```

Expected: `benchmarks/BenchmarkSuite.class` created.

### Step 3: Smoke-test with Duke

```bash
cargo run --release -- exec benchmarks/BenchmarkSuite.class benchSum
# Expected: Int(<some int>)

cargo run --release -- exec benchmarks/BenchmarkSuite.class benchFib
# Expected: Int(75025)

cargo run --release -- exec benchmarks/BenchmarkSuite.class benchArrayList
# Expected: Int(5000)

cargo run --release -- exec benchmarks/BenchmarkSuite.class benchHashMap
# Expected: Int(19900)
```

### Step 4: Smoke-test with HotSpot

```bash
java -cp benchmarks BenchmarkSuite sum
# Expected: prints the sum value

java -cp benchmarks BenchmarkSuite fib
# Expected: 75025

java -cp benchmarks BenchmarkSuite arraylist
# Expected: 5000

java -cp benchmarks BenchmarkSuite hashmap
# Expected: 19900
```

### Step 5: Commit

```bash
git add benchmarks/BenchmarkSuite.java benchmarks/BenchmarkSuite.class
git commit -m "bench: add BenchmarkSuite Java programs for Duke vs HotSpot comparison"
```

---

## Task 2: Criterion Benchmarks

**Files:**
- Modify: `crates/duke-interpreter/Cargo.toml` — add criterion, [[bench]] section
- Create: `crates/duke-interpreter/benches/interpreter.rs`

### Step 1: Add criterion to `crates/duke-interpreter/Cargo.toml`

Open `crates/duke-interpreter/Cargo.toml`. After the `[dependencies]` section, add:

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "interpreter"
harness = false
```

### Step 2: Create `crates/duke-interpreter/benches/interpreter.rs`

```rust
use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use duke_interpreter::{bootstrap_stdlib, build_class_context, execute_class, ClassRegistry};
use duke_loader::DirectoryLoader;
use std::path::PathBuf;

/// Path to the `benchmarks/` directory at the workspace root.
fn bench_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // workspace root
        .join("benchmarks")
}

/// Set up a fresh registry + heap + loader for one benchmark run.
/// Includes: parse .class, register class context, bootstrap stdlib.
/// This mirrors `run_bootstrap_int` from the test module.
fn make_env() -> (ClassRegistry, duke_gc::Heap, DirectoryLoader) {
    let dir = bench_dir();
    let bytes = std::fs::read(dir.join("BenchmarkSuite.class"))
        .expect("benchmarks/BenchmarkSuite.class not found — run Task 1 first");
    let cf = duke_classfile::ClassFile::parse(&bytes).unwrap();
    let ctx = build_class_context(&cf);
    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = duke_gc::Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    let loader = DirectoryLoader::new(&dir);
    (registry, heap, loader)
}

fn bench_sum(c: &mut Criterion) {
    c.bench_function("benchSum (500k int adds)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchSum",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_fib(c: &mut Criterion) {
    c.bench_function("benchFib (fib(25), ~500k calls)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchFib",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_arraylist(c: &mut Criterion) {
    c.bench_function("benchArrayList (5k ArrayList.add)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchArrayList",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_hashmap(c: &mut Criterion) {
    c.bench_function("benchHashMap (200 put + 200 get)", |b| {
        b.iter_batched(
            make_env,
            |(mut registry, mut heap, loader)| {
                black_box(execute_class(
                    &mut registry,
                    &loader,
                    &mut heap,
                    &mut std::io::sink(),
                    "BenchmarkSuite",
                    "benchHashMap",
                    "()I",
                    &[],
                ))
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_sum, bench_fib, bench_arraylist, bench_hashmap);
criterion_main!(benches);
```

### Step 3: Run Criterion benchmarks

```bash
cargo bench -p duke-interpreter 2>&1 | tee benchmarks/criterion-output.txt
```

Expected: Criterion runs each benchmark, prints median time. This will take a few minutes. Criterion auto-calibrates iteration count.

Sample expected output (very rough, actual numbers will vary):
```
benchSum (500k int adds)    time:   [XXX ms  XXX ms  XXX ms]
benchFib (fib(25), ...)     time:   [XXX ms  XXX ms  XXX ms]
benchArrayList (5k ...)     time:   [XXX ms  XXX ms  XXX ms]
benchHashMap (200 ...)      time:   [XXX ms  XXX ms  XXX ms]
```

### Step 4: Commit

```bash
git add crates/duke-interpreter/Cargo.toml crates/duke-interpreter/benches/interpreter.rs benchmarks/criterion-output.txt
git commit -m "bench: add Criterion microbenchmarks for Duke interpreter"
```

---

## Task 3: Wall-Clock Comparison Script + Results

**Files:**
- Create: `benchmarks/compare.sh`
- Create: `benchmarks/RESULTS.md`

### Step 1: Create `benchmarks/compare.sh`

```bash
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
    DUKE_SECS=$( { TIMEFORMAT='%R'; time "$DUKE" exec "$CLASS" "$METHOD" > /dev/null; } 2>&1 )
    DUKE_MS=$(echo "$DUKE_SECS" | awk '{printf "%.0f", $1 * 1000}')

    # Time HotSpot JIT
    HS_SECS=$( { TIMEFORMAT='%R'; time java -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG" > /dev/null; } 2>&1 )
    HS_MS=$(echo "$HS_SECS" | awk '{printf "%.0f", $1 * 1000}')

    # Time HotSpot -Xint (interpreter only, no JIT)
    XI_SECS=$( { TIMEFORMAT='%R'; time java -Xint -cp "$BENCHMARKS_DIR" BenchmarkSuite "$ARG" > /dev/null; } 2>&1 )
    XI_MS=$(echo "$XI_SECS" | awk '{printf "%.0f", $1 * 1000}')

    # Ratio
    RATIO=$(echo "$DUKE_MS $HS_MS" | awk '{if ($2 > 0) printf "%.0fx", $1/$2; else print "N/A"}')

    printf "%-30s %10s %12s %14s %12s\n" "$METHOD" "${DUKE_MS}" "${HS_MS}" "${XI_MS}" "$RATIO"
done

echo ""
echo "Notes:"
echo "  Duke includes: cargo binary startup + class parse + bootstrap_stdlib + execution"
echo "  HotSpot includes: JVM startup + execution (JIT or interpreter)"
echo "  HotSpot -Xint disables JIT compilation (pure interpreter comparison)"
```

### Step 2: Run the comparison

```bash
bash benchmarks/compare.sh 2>&1 | tee /tmp/comparison-run.txt
cat /tmp/comparison-run.txt
```

This will take a few minutes (Duke's fib(25) is the slowest). Record the actual output.

### Step 3: Create `benchmarks/RESULTS.md`

After running, create `benchmarks/RESULTS.md` with the actual results. Template:

```markdown
# Duke vs HotSpot Benchmark Results

Date: 2026-03-08
Duke version: Phase 23 (327 tests, switch-dispatch interpreter, no JIT, bump-pointer GC)
HotSpot version: OpenJDK 21.0.4
Platform: Windows 11 Pro, [CPU info from `wmic cpu get name`]

## Wall-Clock Comparison

[Paste actual table output from compare.sh here]

## Criterion (Duke internal, includes bootstrap_stdlib setup)

[Paste key lines from criterion output here]

## Analysis

[3-5 bullet points on what the numbers show]

## What's expensive in Duke (speculation pre-profiling)

- `bootstrap_stdlib()` registers ~125 natives on every cold start
- Each method invocation pushes a new `Frame` (Vec allocation)
- HashMap/HashSet use linear scan (O(n²) for benchHashMap)
- No string interning: every LDC allocates a new HeapObject
- Switch-dispatch interpreter: no branch prediction benefit, no JIT

## Baseline for future optimization

These results serve as the baseline. When profiling or optimization work begins,
compare new Criterion numbers against this baseline.
```

### Step 4: Commit

```bash
git add benchmarks/compare.sh benchmarks/RESULTS.md benchmarks/criterion-output.txt
git commit -m "bench: add compare.sh and record baseline results vs HotSpot"
```

---

## Summary

| Task | What | Deliverable |
|------|------|-------------|
| 1 | `BenchmarkSuite.java` — 4 benchmark methods | `benchmarks/*.java/.class` |
| 2 | Criterion microbenchmarks | `benches/interpreter.rs`, median ns/iter |
| 3 | Shell comparison + recorded results | `compare.sh`, `RESULTS.md` |

**Expected outcome:** Numbers showing Duke is 50-500x slower than HotSpot JIT for arithmetic, and 5-50x slower than HotSpot -Xint (pure interpreter comparison). This establishes a baseline for future profiling and optimization work.

**Note on what "benchmark" measures:** Criterion + wall-clock both include `bootstrap_stdlib` setup (registering ~125 natives). This is intentional — it reflects actual cold-start cost. Future benchmarks can separate setup from execution using `iter_with_large_drop` or by persisting a registry across runs.
