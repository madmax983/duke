# jdk/internal/misc/Unsafe: array-intrinsics statics (2026-07-19)

Durable record of adding the array base-offset / index-scale constant surface to
the synthetic `jdk/internal/misc/Unsafe`, clearing the Unsafe-intrinsics wall on
the real-JDK bootstrap frontier.

---

## 1. Context / TL;DR

Under `DUKE_REAL_JDK` shadow, the String-encode bootstrap path advanced past
`java/lang/String` and reached
`getstatic jdk/internal/misc/Unsafe.ARRAY_BOOLEAN_INDEX_SCALE`. The synthetic
`Unsafe` (KEEP_SYNTHETIC; `registry.rs:22`) declared **no static fields at all**,
so `resolve_static_field` (`native/common.rs:12450`) walked its superclass chain,
found nothing, and returned the hardcoded `Err(InvalidFieldref { index: 0 })`.

**Baseline.** origin/trunk `595f0bc`, `cargo test --workspace` =
**3147 passed / 0 failed / 3 ignored**.

**Wave outcome.** The synthetic `Unsafe` now declares the full 9-kind array
intrinsics surface — 18 `public static final int` fields — seeded with constant
values. `getstatic Unsafe.ARRAY_*` now resolves, clearing the entire Unsafe
intrinsics lane in one move. The frontier advances to the `String(StringBuilder)`
constructor, which is String work owned elsewhere; we stop and pin there.

---

## 2. Mechanism (how a synthetic KEEP_SYNTHETIC class exposes a static field)

A static field's value is read by the `Getstatic` handler
(`execution.rs:1857`) as `registry.get(&decl_class)?.static_fields[sidx]`, where
`(decl_class, sidx)` come from `resolve_static_field` (`native/common.rs:12450`).
That resolver walks the superclass chain and, per class, computes
`ctx.fields.iter().filter(|f| f.is_static).position(|f| f.name == name)` — i.e.
the index is the **position among static-only `FieldEntry`s**, and the value lives
at that same index in the parallel `static_fields: Vec<Slot>`.

So exposing a static constant requires two parallel, order-matched additions on
the `ClassContext`:

1. a `FieldEntry { name, descriptor, is_static: true }` in `fields`, and
2. its value as a `Slot` at the matching static-position in `static_fields`.

This mirrors exactly how synthetic `java/lang/System` exposes `out`/`err`/`in`
(`stdlib.rs` ~:2214-2235). Since synthetic `Unsafe` has only static fields, the
`fields` and `static_fields` vectors are a direct 1:1 by construction.

---

## 3. Statics modeled + values + semantics

For each element kind `K` in
{BOOLEAN, BYTE, CHAR, SHORT, INT, LONG, FLOAT, DOUBLE, OBJECT}:

| Field                   | Descriptor | Value |
|-------------------------|-----------|-------|
| `ARRAY_<K>_BASE_OFFSET` | `I`       | `0`   |
| `ARRAY_<K>_INDEX_SCALE` | `I`       | `1`   |

18 fields total, seeded in `register_unsafe_stdlib` (`stdlib.rs`).

**Semantics justification.** Real bytecode addresses array element `i` as
`BASE_OFFSET + (i << log2(INDEX_SCALE))` — a byte offset into HotSpot's
byte-addressed heap. Duke's heap is **positional**: `HeapObject::fields` holds one
`Slot` per array element regardless of the element's primitive width
(`duke-gc`), and an Unsafe "offset" in Duke *is* the positional slot index.
Choosing `BASE_OFFSET == 0` and `INDEX_SCALE == 1` collapses the real formula to
`offset == i` exactly (`ASHIFT == log2(1) == 0`). This is the same contract the
existing `arrayBaseOffset()` (returns 0) and `arrayIndexScale()` (returns 1)
natives already honor (`stdlib.rs:1398,1403`; `native/jdk_internal.rs`), and the
offset-encoding contract documented at `native/common.rs:17690-17703`. Duke never
performs real pointer arithmetic with these values — offsets are slot indices — so
the collapsed formula is not an approximation but exact for Duke's model.

**`ADDRESS_SIZE` / `PAGE_SIZE`: intentionally NOT added.** Real `Unsafe` also
declares these platform constants (materialized by `UnsafeConstants` in the real
`<clinit>`, which Duke never runs). Empirically, the real-JDK frontier this surface
serves does **not** demand them: with only the 18 ARRAY_* constants seeded, the
chain advances straight past the Unsafe lane into `java/lang/String.<init>`. A probe
build that omitted ADDRESS_SIZE/PAGE_SIZE produced the identical downstream
frontier, confirming they are not on this path. Added only if a future frontier
issues `getstatic Unsafe.ADDRESS_SIZE`/`PAGE_SIZE` (seed with the real 64-bit
values 8 / 4096 — guard-plausible, since Duke does no real pointer/page arithmetic).
A code comment in `register_unsafe_stdlib` records this decision.

---

## 4. Frontier: before / after (verbatim)

### classloader_bootstrap_frontier :: slf4j_real_jdk_shadow_classloader_frontier_pin

BEFORE (verbatim harness output):

```
InvalidFieldref { index: 0 }
```

AFTER (verbatim harness output):

```
MethodNotFound { name: "java/lang/String.<init>", descriptor: "(Ljava/lang/StringBuilder;)V" }
```

The pin's `EXPECTED_FRONTIER` and documenting comment are updated to this new
verbatim string.

### writer_graph_frontier :: writer_graph_probe_real_jdk_shadow_completes_with_hello_bytes

UNCHANGED. This is not an Unsafe pin but an **end-to-end completion milestone**:
the real `OutputStreamWriter -> StreamEncoder -> Charset` writer graph already runs
to completion (its former `getstatic ByteBuffer.UNSAFE` wall was cleared long ago
by PR #1354, inherited-static resolution + eager superclass `<clinit>`). Adding the
ARRAY_* statics does not touch that path; the test still completes and emits the
exact UTF-8 bytes of `"hello\n"` (`[104, 101, 108, 108, 111, 10]`). Left as-is.

---

## 5. Where we stopped and why

Stopped at
`MethodNotFound { name: "java/lang/String.<init>", descriptor: "(Ljava/lang/StringBuilder;)V" }`.
The `String(StringBuilder)` constructor is **String work (String stages 3-4)**, a
distinct lane owned elsewhere — not an Unsafe intrinsic, not a jdk_internal static.
Per the ownership boundary, we pin honestly here rather than force past into
String territory. The Unsafe intrinsics lane is fully cleared: every
`getstatic Unsafe.ARRAY_*` the real bootstrap issues now resolves.
