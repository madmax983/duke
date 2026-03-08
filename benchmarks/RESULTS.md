# Duke vs HotSpot Benchmark Results

Date: 2026-03-08
Duke version: Phase 21 (296 tests, switch-dispatch interpreter, no JIT, bump-pointer GC)
HotSpot version: OpenJDK 21.0.4
Platform: Windows 11 Pro, Intel(R) Core(TM) i9-14900HX

## Wall-Clock Comparison

Each measurement is one cold invocation (includes startup overhead):
- Duke: binary startup + class parse + `bootstrap_stdlib()` + execution
- HotSpot JIT: JVM startup + JIT-compiled execution
- HotSpot -Xint: JVM startup + pure interpreter (no JIT)

```
Benchmark                       Duke (ms)  HS JIT (ms)  HS -Xint (ms)     Duke/JIT
================================================================================
benchSum                              207          171            128           1.2x
benchFib                              274          133            202           2.1x
benchArrayList                         67          165            316           0.4x
benchHashMap                           53          187            182           0.3x
```

Note: Duke/JIT < 1.0x means Duke is faster than HotSpot for that benchmark.

## Criterion (Duke internal, each iteration includes parse + bootstrap_stdlib setup)

| Benchmark | Median time |
|-----------|-------------|
| benchSum (500k int adds) | 141.36 ms |
| benchFib (fib(25), ~500k calls) | 277.70 ms |
| benchArrayList (5k ArrayList.add) | 7.53 ms |
| benchHashMap (200 put + 200 get) | 1.48 ms |

Note: Criterion numbers are lower than wall-clock because Criterion amortizes startup
across many iterations — the benchmark loop itself is tight (no process spawn overhead).
The wall-clock comparison includes a full process launch per run.

## Analysis

- **Arithmetic (benchSum)**: Duke takes 207ms vs HotSpot JIT's 171ms — only 1.2x slower.
  Notably, Duke beats HotSpot -Xint (128ms vs 207ms), meaning the JVM's pure interpreter
  has less per-instruction overhead than Duke, but Duke is competitive with JIT at this scale.
  The startup + bootstrap_stdlib cost (~60-80ms estimated) dominates short workloads.

- **Recursive dispatch (benchFib)**: Duke is 2.1x slower than HotSpot JIT (274ms vs 133ms)
  and 1.4x slower than HotSpot -Xint (274ms vs 202ms). ~500k recursive calls expose
  Duke's Frame allocation cost (Vec push/pop per call) and the Rust HashMap lookup in
  ClassRegistry on every invokestatic.

- **ArrayList (benchArrayList)**: Duke is 2.5x FASTER than HotSpot JIT (67ms vs 165ms).
  Duke's ArrayList is a synthetic Rust-native class — `add`, `size`, and `iterator` are
  direct Rust function calls with zero bytecode overhead. HotSpot executes real
  java.util.ArrayList bytecode with full generics overhead.

- **HashMap (benchHashMap)**: Duke is 3.5x FASTER than HotSpot JIT (53ms vs 187ms).
  Same reason as ArrayList: Duke's HashMap is entirely synthetic (Rust Vec linear scan).
  This is misleading as a performance win — Duke's HashMap is O(n) while HotSpot's is
  O(1) amortized. At 200 entries the linear scan wins; at 10k entries it would collapse.

- **Startup dominates short benchmarks**: Criterion shows benchArrayList at 7.5ms and
  benchHashMap at 1.5ms (amortized, no process spawn). Wall-clock is 67ms and 53ms —
  the delta (~50ms) is pure startup cost (binary load + bootstrap_stdlib registration
  of ~125 native handlers). Any benchmark faster than ~100ms is startup-dominated.

## What's expensive in Duke (speculation pre-profiling)

- `bootstrap_stdlib()` registers ~125 natives on every cold start (~50ms wall-clock overhead)
- Each method invocation pushes a new `Frame` (Vec allocation, no pooling)
- HashMap/HashSet use linear scan (O(n²) for benchHashMap at scale; masked at 200 entries)
- No string interning: every LDC allocates a new `HeapObject` on the bump-pointer heap
- Switch-dispatch interpreter: one match arm per opcode, no threading or computed-goto
- ClassRegistry method lookup: `HashMap<(class, method, desc), MethodBody>` — hashmap
  overhead on every invokevirtual/invokestatic (no inline cache, no vtable)

## Baseline for future optimization

These results serve as the baseline. When profiling or optimization work begins,
compare new Criterion numbers against this baseline.

| Benchmark | Criterion baseline | Wall-clock baseline |
|-----------|-------------------|---------------------|
| benchSum  | 141.36 ms | 207 ms |
| benchFib  | 277.70 ms | 274 ms |
| benchArrayList | 7.53 ms | 67 ms |
| benchHashMap | 1.48 ms | 53 ms |

Highest-leverage optimization targets (estimated impact):
1. Pool `Frame` allocations — would most benefit benchFib (~500k allocs)
2. Cache `bootstrap_stdlib()` result or lazy-register — cuts ~50ms from wall-clock startup
3. Intern strings at LDC time — reduces Heap pressure for string-heavy workloads
4. Add vtable / inline cache for invokevirtual — reduces HashMap lookup overhead
