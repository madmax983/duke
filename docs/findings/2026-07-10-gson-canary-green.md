# gson-2.11.0 canary GREEN — findings (2026-07-10)

The hermetic OSS-jar `gson-2.11.0` `toJson`/`fromJson` canary
(`crates/duke-interpreter/tests/oss_jar_smoke.rs::gson_smoke_runs_real_jar_bytecode`)
now runs unmodified `gson` bytecode end-to-end. This document records the
load-bearing GC root cause, the capability chain that was cleared on top of it,
and the known limitations that remain open.

**Driver:** `tests/fixtures/GsonSmoke.java`
**Happy path:** `new Gson().toJson(pojo)` on a tiny static-nested POJO
(`{int count; String name}`), then `gson.fromJson(json, Pojo.class)`, then print
the round-trip line:

```
gson round-trip: {"count":7,"name":"duke"} -> count=7 name=duke
```

The prior findings doc for this jar (`docs/findings/2026-07-09-oss-smoke-widening.md`)
recorded the observed first blocker (`String.format("%04x", ...)` returning an
`InvalidRef`) and a ranked, statically-inferred chain of downstream reflective
blockers. Clearing that chain is what this work did.

---

## Root cause: GC nested-`<clinit>` stale roots

The deepest and least obvious blocker was **not** a missing native — it was a
garbage-collection correctness bug. During `toJson`, gson triggers class
initialization (`<clinit>`) that itself allocates enough to trigger a collection
while an *outer* `<clinit>`/method is still mid-execution on the interpreter's
nested execution stack. Those nested, in-flight execution states were not being
registered as GC roots, so a collection during a nested initializer could free
still-live objects and leave stale/dangling roots, surfacing later as internal
heap errors (`InvalidRef`) that looked like unrelated native bugs.

The fix — **register all active nested execution states as GC roots** — landed
separately on branch `swarm/gc-nested-clinit-roots` (PR #1318), and the gson
canary branch (`swarm/gson-canary`) is stacked on top of it. With live roots
correctly retained across nested-`<clinit>` collections, the remainder of the
work was ordinary capability filling.

---

## Capability chain cleared (21 blockers)

On top of the GC fix, the reflective + boxing + stdlib capabilities gson needs
for a full `toJson`/`fromJson` round-trip were added, in execution order:

- **`java/lang/reflect/Type` marker interface** — the supertype that `Class`,
  `Field.getGenericType()`, and `TypeToken` erase to.
- **`Class` reflection natives** — `getModifiers`, `isInterface`, `isPrimitive`,
  `isRecord`, `isAnonymousClass`, `isLocalClass`, `getGenericSuperclass`,
  `isAssignableFrom`, and `cast`, so gson's `Excluder` / `ReflectiveTypeAdapterFactory`
  and `$Gson$Types` type walk can run.
- **`java/lang/reflect/Field` natives** — `getModifiers`, `getGenericType`,
  `getType`, `get`, plus `isSynthetic`, driving `getBoundFields`/`excludeField`.
- **Reflective `sun/misc/Unsafe.allocateInstance`** — `GsonSmoke$Pojo` has no
  no-arg constructor, so gson's `ConstructorConstructor` falls through to the
  Unsafe allocator on the `fromJson` half.
- **Boxed numerics extend `java/lang/Number`** — `Long`, `Short`, `Byte`,
  `Double`, and `Float` now declare `Number` as their super_class (mirroring the
  existing `Integer` change), so `instanceof`/`checkcast Number` is correct for
  every boxed numeric (gson's `Number` `TypeAdapter` bridge). `Character` and
  `Boolean` correctly remain on `Object`.
- **Misc `String` / `StringBuffer` / `Map` natives** — the remaining
  serialization-path helpers (`String.getChars`, `StringBuffer` sink methods,
  `Map` iteration) gson's `JsonWriter`/`JsonReader` rely on.

---

## Known limitations (still open)

These are tracked as `TODO(known-limitation)` markers in
`crates/duke-interpreter/src/native/reflect.rs`:

- **`Class.getModifiers()` / `Class.isInterface()` report ACC_PUBLIC-only.** They
  do not reflect the real `ACC_INTERFACE` / `ACC_ABSTRACT` / `ACC_FINAL` /
  non-`ACC_PUBLIC` bits, because `ReflectedClassInfo` does not carry the class's
  raw `ClassAccessFlags`. Surfacing the real flags needs plumbing through
  `registry.rs`. gson only reads these on concrete, non-interface raw types it is
  (de)serializing, so the current behavior is correct for the canary but wrong in
  general.
- **`Field.getModifiers()` omits the transient/final bits.** It reports only
  `ACC_PUBLIC`/`ACC_STATIC`; the transient and final bits are dropped when the
  `Field` mirror is built (`ReflectedFieldInfo` / `ReflectedFieldHandle` carry
  only public/static). gson uses the transient bit to skip fields, so a transient
  field would not be skipped. Retaining these bits also needs `registry.rs`.

Separately, `sun/misc/Unsafe` and the synthetic `ThreadLocal` are **not** on the
`KEEP_SYNTHETIC` allowlist. Under an eventual `real_jdk_shadow` run they would be
shadowed by real JDK bytecode that Duke does not yet fully support, so the canary
currently depends on the synthetic implementations.

---

## Reproduction

```
# end-to-end gson canary (now passing)
cargo test -p duke-interpreter --test oss_jar_smoke \
  gson_smoke_runs_real_jar_bytecode -- --nocapture

# slf4j-simple canary (still passing)
cargo test -p duke-interpreter --test oss_jar_smoke \
  slf4j_simple_smoke_runs_real_jar_bytecode -- --nocapture
```
