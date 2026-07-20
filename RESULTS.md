# Stage 4 perf results — real String bytecode vs. synthetic natives

## Workload
`StringLoop` micro-benchmark: 4000 outer iterations, each doing, on the fixed
String `"The quick brown fox jumps over the lazy dog"`:
- `length()` + `charAt(j)` over every character
- `equals(...)`
- `hashCode()`
- `indexOf("fox")`
- `substring(4, 9).length()`

Run via the `duke` binary (`/workspace/target/debug/duke run ...`), wall-clock
via `time`, two samples each. All runs produce the identical result
`acc=-2437696252000`, confirming real String bytecode on the 4-slot layout is
byte-identical to the native path.

## Numbers

| Mode | String dispatch | Sample 1 | Sample 2 | ~mean |
|------|-----------------|----------|----------|-------|
| default (flag off) | synthetic natives | 3.02s | — | ~3.0s |
| real-jdk **BEFORE** (String in KEEP_SYNTHETIC) | synthetic natives | 4.10s | 4.02s | ~4.06s |
| real-jdk **AFTER** (String shadowed) | real JDK bytecode | 8.18s | 8.15s | ~8.16s |

## Delta

- **Stage 4 real-jdk delta:** ~4.06s → ~8.16s ≈ **2.0x slower** for a
  String-hot loop under real-jdk shadow mode. Cause: `charAt`/`length`/
  `hashCode`/`indexOf`/`substring` move from native Rust to interpreted real JDK
  String bytecode (which itself decodes `value:[B`/`coder:B`, walks
  `StringLatin1`/`StringUTF16` helpers, etc.).
- **Default mode:** byte-identical before/after — Stage 4 changes behavior only
  under real-jdk shadow mode (`should_shadow_synthetic` is always false when the
  flag is off, so String stays synthetic and its natives fire unchanged). The
  ~3.0s default figure is the untouched native baseline.

## Interpretation
The slowdown is confined to the experimental real-jdk shadow mode and is the
expected cost of trading hand-written native intrinsics for real interpreted
bytecode (the whole point of the real-layout migration: correctness/fidelity over
raw speed in shadow mode). No perf regression reaches the default execution path.
If real-jdk String throughput later matters, the overridden hot methods
(`charAt`/`length`/`hashCode`/…) are candidates to re-register as real-layout
intrinsics that win over bytecode — but that is out of Stage 4 scope.
