# java/lang/String: real-layout migration blockers & staged plan (2026-07-13)

This document is the durable record of the scoping investigation into promoting
`java/lang/String` from its synthetic stub to real JDK layout under the real-jdk
shadow, removing it from the `KEEP_SYNTHETIC` allowlist
(`crates/duke-interpreter/src/registry.rs:23`). The short version: full migration
is a single coherent, multi-file cluster that touches the interpreter core files
this wave does not own; what landed instead is a cheap, honest, in-lane
`COMPACT_STRINGS` static shim that advances the real-jdk frontier one honest
bytecode step (`getstatic COMPACT_STRINGS` → `String.coder()B`).

---

## 1. Context / TL;DR

`java/lang/String` is the last-and-most-entangled member of `KEEP_SYNTHETIC`
(`registry.rs:23`; **11 entries total**). Every real-JDK bootstrap path now
converges on the same fieldref: real `jdk/internal/util/StaticProperty.<clinit>`
(reached after the `System.getProperties()` wall was cleared by #1339) executes
`getstatic java/lang/String.COMPACT_STRINGS:Z` — a real static that the synthetic
String does not declare.

**Baseline.** origin/trunk `03f753b`, `cargo test --workspace` =
**3095 passed / 0 failed / 4 ignored**.

**Wave outcome.** A cheap, honest, in-lane shim (this PR) declares
`COMPACT_STRINGS:Z = true` on the synthetic String, advancing the frontier one
honest step to `String.coder()B`. **Full migration is deferred**: it is a coherent
4-field-cluster refactor (mint site + all `string_value` natives + `<clinit>` deps
+ intern cache) that must own the interpreter-core files (`registry.rs`,
`native/common.rs`, `execution.rs`) — the files this wave is forbidden to touch —
and must be sequenced with the in-flight `Arc<str>` interning refactor. This PR
does **not** commit the VM to real String layout.

---

## 2. The wall (empirical)

The synthetic `java/lang/String` `ClassContext` is built in `bootstrap_stdlib`
(`crates/duke-interpreter/src/stdlib.rs` ~:3006-3023) with `fields: Vec::new()`,
`static_fields: Vec::new()`, `instance_field_count: 0` — **no static fields and no
instance fields at all**. Real bootstrap bytecode running
`getstatic java/lang/String.COMPACT_STRINGS:Z` cannot resolve the fieldref: the
named-static lookup (`static_field_idx`, `native/common.rs:12345`) returns
`Err(InvalidFieldref { index: 0 })`.

This surfaces as the same frontier in two pinned tests (same wall, reached after
the `System.getProperties()` wall cleared):

- `crates/duke-interpreter/tests/classloader_bootstrap_frontier.rs:146` —
  `InvalidFieldref { index: 0 }`.
- `duke/tests/spring_boot_real_jdk.rs:69` — CLI rendering
  `constant pool index 0 is not a valid Fieldref`.

**KEEP_SYNTHETIC consult point.** The synthetic-vs-real branch is
`should_shadow_synthetic` (`registry.rs:1093-1101`), whose
`!KEEP_SYNTHETIC.contains(&ctx.class_name.as_str())` guard at `registry.rs:1096`
keeps String synthetic; it is called from `register` (`registry.rs:1013`). Because
String is on the list, its synthetic stub is always kept even under the shadow
flag, so the real static never gets a home.

---

## 3. What real `String.<clinit>` needs (javap, JDK 21)

Real `java/lang/String.<clinit>` (`javap -p -c java.lang.String`, JDK 21):

```
static {};
   0: iconst_1
   1: putstatic  COMPACT_STRINGS:Z                 // COMPACT_STRINGS = true
   4: iconst_0
   5: anewarray  java/io/ObjectStreamField
   8: putstatic  serialPersistentFields:[L...;
  11: new        java/lang/String$CaseInsensitiveComparator
  14: dup
  15: invokespecial String$CaseInsensitiveComparator."<init>":()V
  18: putstatic  CASE_INSENSITIVE_ORDER:Ljava/util/Comparator;
  21: return
```

So `<clinit>` requires: `putstatic COMPACT_STRINGS = true` (the wall);
`serialPersistentFields`; `CASE_INSENSITIVE_ORDER`, which needs the inner class
`java/lang/String$CaseInsensitiveComparator`.

**Real instance fields = 4 slots** (JDK9+ compact strings):
`private final byte[] value` (slot 0); `private final byte coder` (slot 1);
`private int hash` (slot 2); `private boolean hashIsZero` (slot 3). Note
`value` is **`byte[]`, not `char[]`**. Contrast the synthetic String: **0 fields**.

---

## 4. Synthetic-string minting audit

The real string-minting primitive is `heap.allocate_string`
(`crates/duke-gc/src/lib.rs:755`): it mints a `HeapObject` of class
`java/lang/String` with an **empty `fields` Vec** and carries the character data
in a Rust side-channel `string_value: Option<String>` — **not** in JVM
`value`/`coder`/`hash` fields.

**Call-site surface.** `heap.allocate_string(` occurs **264 times** across
`crates/duke-interpreter/src` (118 in `tests.rs`; **~146 in production**).
Highest concentrations: `native/java_lang.rs` (53), `native/common.rs` (37),
`native/java_util.rs` (26), `execution.rs` (2 — the `ldc`/`ldc_w` intern paths),
plus java_util_regex/java_time/stdlib/java_io/etc.

**CORRECTION to prior scoping.** `allocate_string_backed_object`
(`native/common.rs:2928`) does **NOT** mint a `java/lang/String`. It builds a
generic 1-slot object (slot 0 = reference to a heap String) for URL/URI/Path
(6 call sites). Exclude it from the String-mint migration surface.

**Interning.**
- Interpreter constant cache: `string_intern: HashMap<(u8, String), u64>` keyed
  `(0, literal)` for String constants, `(1, class_name)` for Class constants,
  populated on the `ldc` path in `execution.rs`.
- `String.intern()` native is **identity** (`native/java_lang.rs:4516`); there is
  no separate interned-String pool — value-equality + the ldc cache stand in.
- **#1312 GC fix** (must be preserved by any layout change): the `string_intern`
  values are treated as GC **roots** (`native/common.rs:15378`) and **forwarded**
  after a minor collection (`native/common.rs:15436-15442`). A moved literal
  otherwise leaves a dangling ref (the gson `JsonWriter.<clinit>`
  `String.format("\\u%04x", …)` bug).

---

## 5. Layout guard interaction (#1311)

Two-tier getfield/putfield guard in `crates/duke-interpreter/src/execution.rs`:

- **Tier 1 — unconditional bounds guard** (fires in EVERY mode, even Off):
  getfield `execution.rs:1734-1756`, putfield `execution.rs:1795-1817`. Returns a
  **graceful** `Error::FieldOutOfBounds` (not a raw panic) plus a loud
  `[layout-coherence] INCOHERENT …` diagnostic.
- **Tier 2 — mode-gated regime check** via `is_layout_incoherent`
  (`registry.rs:864-881`): incoherent when `slot >= object_slot_count` OR resolving
  and object class disagree on layout regime (one real, one synthetic).

**CRITICAL.** The Tier-1 guard checks the **concrete `HeapObject.fields.len()`**,
NOT the declared `ClassContext.instance_field_count`. Bumping the declared count
alone is insufficient — the mint site (`heap.allocate_string`) must actually
allocate 4 slots. Empirically (Exp B — throwaway removal of String from
`KEEP_SYNTHETIC`), the guard trips immediately on the first real `getfield`:

```
[layout-coherence] INCOHERENT getfield java/lang/String.value in java/lang/String.length:
  resolving-class java/lang/String (real, expects 4 slots) vs
  object-class java/lang/String (real, has 0 slots); slot 0 out of bounds
FieldOutOfBounds { index: 1, length: 0 }
```

Both sides are "real" (regime matches), so this is purely the **bounds** arm: the
heap object has 0 slots. The error stays graceful (the spring RAW_PANIC_MARKERS
check still passes).

---

## 6. The cheap unblocker landed in THIS PR

Added a read-only `COMPACT_STRINGS:Z = true` (`Slot::Int(1)`) static to the
synthetic String `ClassContext` in `stdlib.rs` (mirroring the
`java/lang/System` static-field construction):

```rust
fields: vec![FieldEntry {
    name: "COMPACT_STRINGS".to_string(),
    descriptor: "Z".to_string(),
    is_static: true,
}],
static_fields: vec![Slot::Int(1)],
instance_field_count: 0,
```

**Why honest.** Real `String.<clinit>` sets `COMPACT_STRINGS = true`
unconditionally (`iconst_1; putstatic`), and the field is `static final` — read as
a Latin1-vs-UTF16 branch selector, **never written by app code**. A hardcoded
`true` mirrors reality exactly and cannot drift. The synthetic String never runs a
real `<clinit>`, so the value is seeded directly rather than by executing the
`putstatic` — same end-state, no fake path.

**Why in-lane.** Touches only `stdlib.rs` + two test pins — none of the forbidden
interpreter-core files (`registry.rs`, `native/common.rs`, `execution.rs`).

**Result (Exp A).** `getstatic java/lang/String.COMPACT_STRINGS:Z` now resolves
(returns `true`) and the frontier advances one honest bytecode step to a new
graceful wall — real String bytecode invokes the private instance accessor
`String.coder()B`, which the synthetic String does not provide as a native:

- `classloader_bootstrap_frontier.rs` —
  `MethodNotFound { name: "java/lang/String.coder", descriptor: "()B" }`.
- `spring_boot_real_jdk.rs` CLI — `method not found: java/lang/String.coder()B`.

**Zero collateral.** Only the 3 frontier pins flip; the full suite stays
**3095 / 0 / 4**. No native / layout-guard / interning / `allocate_string`
breakage. This does **NOT** commit the VM to real String layout — String stays
synthetic, so `getfield value/coder` on the side-channel object is never reached;
the very next real call (`coder()`) misses first.

---

## 7. Dependency DAG

Full real-layout String migration is one coupled unit whose foundation is the
mint site; everything reading `string_value` must move with it. Nodes and what
each drags in:

```
heap.allocate_string  (duke-gc/src/lib.rs:755 — mint REAL 4 slots: value byte[]/coder/hash/hashIsZero)
        │  [load-bearing change; ~146 production mint sites]
        ├─► String natives reading string_value  (length/equals/charAt/substring/…)
        │       must read value/coder instead, OR be dropped so real bytecode runs
        │       (java_lang.rs 53, common.rs 37, java_util.rs 26 mints)
        │
        ├─► <clinit> deps
        │       COMPACT_STRINGS (real <clinit> sets it — retires the Stage-0 shim),
        │       CASE_INSENSITIVE_ORDER, serialPersistentFields,
        │       + inner class java/lang/String$CaseInsensitiveComparator
        │
        ├─► String.coder()B + private accessors (isLatin1 etc.)
        │       as INTRINSICS on the real 4-field layout
        │
        ├─► intern cache #1312  (string_intern roots + post-GC forwarding)
        │       key must move to the new byte[] value; forwarding preserved
        │
        └─► Arc<str> interning refactor  [IN FLIGHT — owns execution.rs ldc + intern table]
```

**Must move TOGETHER (the coherent cluster):** `heap.allocate_string` (4-slot
mint) + **all** `string_value`-reading String natives + the `<clinit>` deps + the
two exemplar tests (`crates/duke-interpreter/tests/real_jdk_shadow.rs:39`
`KEEP_SYNTHETIC_SAMPLE`, `layout_coherence.rs:105`) — because a native reading
`string_value` on a real-layout object (or vice versa) is the #1307
half-migration/silent-corruption trap.

**Can be INTRINSICS (not full String.java):** String's hot natives
(`length`/`charAt`/`substring`/`coder`/`isLatin1`) can stay as Duke natives that
read the **real** 4-field layout, rather than running all of `String.java`. Only
the `<clinit>` deps genuinely need real bytecode.

---

## 8. Staged plan

For each stage, the interpreter-core files it needs (forbidden this wave) are
noted so scheduling is explicit.

- **Stage 0 — THIS PR (done).** Synthetic `COMPACT_STRINGS:Z = true` shim in
  `stdlib.rs`; frontier advances `getstatic COMPACT_STRINGS` → `coder()B`.
  Forbidden-file needs: **none** (stdlib.rs + test pins only).

- **Stage 1 — real 4-slot mint.** `heap.allocate_string` mints real
  `value: byte[]`, `coder`, `hash`, `hashIsZero`; update all ~146 mint sites and
  every `string_value` reader **atomically** (the Tier-1 guard checks the concrete
  `fields.len()`, so declared-count changes alone corrupt). Owns
  `crates/duke-gc` + the String natives in `java_lang`/`java_util`/**`common.rs`**.
  MUST coordinate with the `Arc<str>` interning refactor (owns
  `registry.rs`/`common.rs`/**`execution.rs`** incl. the ldc intern table) so the
  intern-cache key + #1312 forwarding stay coherent with the new `byte[]` value.
  Forbidden-file needs: **`native/common.rs`, `execution.rs`**.

- **Stage 2 — `<clinit>` deps + accessors.** Provide `CASE_INSENSITIVE_ORDER`,
  `serialPersistentFields`, and `String$CaseInsensitiveComparator`; implement
  `String.coder()B` and the private accessors (`isLatin1` etc.) as intrinsics on
  the real layout; drop the Stage-0 synthetic `COMPACT_STRINGS` shim (real
  `<clinit>` now sets it). Forbidden-file needs: likely **`registry.rs`** (dispatch
  wiring) + the String natives.

- **Stage 3 — remaining natives.** Rewrite all remaining String natives onto
  `value`+`coder`; `StringBuilder`/`StringBuffer.toString`, char access, intern
  identity. Forbidden-file needs: **`native/common.rs`** (String natives).

- **Stage 4 — allowlist drop.** Remove `"java/lang/String"` from `KEEP_SYNTHETIC`
  (`registry.rs:23`); update the two exemplar tests
  (`real_jdk_shadow.rs:39`, `layout_coherence.rs:105`) to a different synthetic
  exemplar; frontier fully real. Forbidden-file needs: **`registry.rs`**.

---

## 9. Sequencing vs the Arc<str> interning refactor

An `Arc<str>` interning refactor is in flight. It changes interning to `Arc<str>`
and **owns** `execution.rs`'s `ldc` path, the `string_intern` table, and the #1312
GC roots/forwarding (`common.rs:15378`, `common.rs:15436-15442`).

**Recommendation:** that lane should land — or freeze its interface — **before
Stage 1**. Stage 1 changes the String `value` to `byte[]`, which changes what the
intern table must key on and what the #1312 forwarding relocates. Doing Stage 1
first would force a second rewrite of the intern path. Read its PR when it opens
and sequence Stage 1 behind it.

---

## 10. New pins / frontier

This PR moves the pinned real-jdk frontier from the fieldref-resolution wall to a
method-dispatch wall. The two frontier tests now pin `String.coder()B`:

- `crates/duke-interpreter/tests/classloader_bootstrap_frontier.rs:146` —
  `MethodNotFound { name: "java/lang/String.coder", descriptor: "()B" }`.
- `duke/tests/spring_boot_real_jdk.rs:69` —
  `method not found: java/lang/String.coder()B`.

The new wall is real String's private byte-coder accessor `coder()B`. The **next
target** (Stage 2) is to implement `String.coder()B` on the real 4-field layout.
The spring RAW_PANIC_MARKERS check (`"index out of bounds"`, `"execution.rs"`,
`"panicked at"`) still passes — the new CLI text contains none of them.
