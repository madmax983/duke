# OSS-jar smoke widening — findings (2026-07-09)

Widening the hermetic OSS-jar smoke harness (previously slf4j-only) to two new
libraries. This document records the vendored artifacts, each library's happy
path, the **observed** first blocker, a **ranked, statically-inferred** chain of
the next blockers, and a combined "missing natives to implement next" list.

All harness code lives in `crates/duke-interpreter/tests/oss_jar_smoke.rs`.
Blockers were surfaced by running each driver's `main` through
`execute_class_to_completion` with a JDK-modules + vendored-jar classpath, and
localized with the interpreter's `DUKE_TRACE_EXEC=1` instruction tracer.
`OBSERVED` entries were reproduced at runtime; `INFERRED` entries are predictions
from static bytecode call-graph analysis (`javap`) cross-referenced against the
native registration table in `crates/duke-interpreter/src/stdlib.rs` (read-only).
Inferred entries are **not** runtime-confirmed — no natives were implemented as
part of this work, so downstream blockers cannot be observed directly.

## Vendored artifacts

| JAR | Version | License | SHA-256 |
| --- | --- | --- | --- |
| `gson-2.11.0.jar` | 2.11.0 | Apache-2.0 | `57928d6e5a6edeb2abd3770a8f95ba44dce45f3b23b7a9dc2b309c581552a78b` |
| `commons-lang3-3.17.0.jar` | 3.17.0 | Apache-2.0 | `6ee731df5c8e5a2976a1ca023b6bb320ea8d3539fbe64c8a1d5cb765127c33b4` |

Both were downloaded from Maven Central and cryptographically verified against
each artifact's published `.jar.sha1` sidecar (gson `527175ca6d81050b53bdd4c457a6d6e017626b0e`,
commons-lang3 `b17d2136f0460dcc0d2016ceefca8723bdf4ee70`). Both are standalone at
runtime. Vendoring metadata is recorded in `tests/fixtures/oss-jars/LICENSES.md`.

---

## gson 2.11.0

**Driver:** `tests/fixtures/GsonSmoke.java`
**Happy path:** `new Gson().toJson(pojo)` on a tiny static-nested POJO
(`{int count; String name}`), then `gson.fromJson(json, Pojo.class)`, then print
the round-trip line. Under a real JDK this prints:
`gson round-trip: {"count":7,"name":"duke"} -> count=7 name=duke`.

### Observed first blocker

- **Rendered:** `InvalidRef { address: 240 }` (deterministic across runs)
- **Location:** `com/google/gson/stream/JsonWriter.<clinit>`, second iteration of
  the `REPLACEMENT_CHARS` fill loop.
- **Cause:** `<clinit>` builds the 0x00–0x1f escape table with
  `java/lang/String.format("\u%04x", java/lang/Integer.valueOf(i))`. Duke's
  `String.format` native *is* registered (`native.rs:16433`) but returns/derefs an
  invalid heap reference for the `%04x` case, so the blocker manifests as an
  internal `InvalidRef` heap error rather than an explicit missing-native. This is
  a **bug in an existing native**, not a missing one.

Note: unlike the slf4j trio, this first blocker is *not* an explicit
`MethodNotFound`/`ClassNotFound`/`Unimplemented`. The pinned test
`gson_smoke_surfaces_next_missing_capability_explicitly` asserts this exact
rendered string and documents that it is a runtime error.

### Ranked inferred chain (after the `String.format` bug is fixed)

1. **[INFERRED]** `java/lang/reflect/Field.getModifiers()I` → `MethodNotFound`
   ("Unsupported native"). During the first serialization, gson's
   `ReflectiveTypeAdapterFactory.getBoundFields` → `Excluder.excludeField` calls
   `field.getModifiers()` on each `Pojo` field. Duke registers a *synthetic*
   `java/lang/reflect/Field` (stdlib.rs ~3989) whose fields are
   `declaringClass/name/descriptor/publicFlag/staticFlag/accessibleFlag` and whose
   registered natives are `getName/getDeclaringClass/getType/get/set/setAccessible/
   getAnnotation(s)/getDeclaredAnnotations` — **no `getModifiers`**.
2. **[INFERRED]** `java/lang/reflect/Field.getGenericType()Ljava/lang/reflect/Type;`
   → `MethodNotFound`. `createBoundField`/`$Gson$Types.resolve` need each field's
   generic type; not registered on the synthetic `Field`.
3. **[INFERRED, lower confidence]** reflective `sun/misc/Unsafe.allocateInstance`
   on the `fromJson` half. `GsonSmoke$Pojo` has **no no-arg constructor** (only
   `Pojo(int,String)`), so gson's `ConstructorConstructor` falls through to
   `newUnsafeAllocator`, which reflectively does
   `Class.forName("sun.misc.Unsafe")` → read `theUnsafe` → obtain the
   `allocateInstance(Class)` `Method` → `Method.invoke(...)`. `sun/misc/Unsafe` is
   not modeled, so this reflective path is expected to fail (as
   `ClassNotFound`/`MethodNotFound` depending on how far the reflection gets).

---

## commons-lang3 3.17.0

**Driver:** `tests/fixtures/CommonsLang3Smoke.java`
**Happy path:** `StringUtils.join(new String[]{"duke","jvm","smoke"}, '-')`,
`StringUtils.capitalize("hello")`, `ArrayUtils.add(new int[]{1,2,3}, 4)`,
`ArrayUtils.contains(...)`, `ArrayUtils.toObject(...)`, then print. Under a real
JDK this prints:
`commons-lang3: join=duke-jvm-smoke capitalize=Hello added=1,2,3,4 contains4=true`.

### Observed first blocker

- **Rendered:** `JavaException { class_name: "java/util/regex/PatternSyntaxException" }`
- **Location:** `org/apache/commons/lang3/StringUtils.<clinit>` @3 (`invokestatic`).
- **Cause:** `<clinit>` initializes
  `STRIP_ACCENTS_PATTERN = Pattern.compile("\p{InCombiningDiacriticalMarks}+")`.
  Duke's regex engine does not support the `\p{InCombiningDiacriticalMarks}`
  Unicode block property, so `Pattern.compile` throws `PatternSyntaxException`,
  which propagates out of class initialization before the first `StringUtils`
  method runs. This is a **capability gap in the regex engine**, surfaced as a
  thrown Java exception rather than an explicit missing-native.

### Ranked inferred chain (after the regex gap is closed)

1. **[INFERRED]** `java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;`
   → `MethodNotFound`. `ArrayUtils.add(int[], int)` → `copyArrayGrow1` allocates
   the grown array via `Array.newInstance(Integer.TYPE, len+1)`. No
   `java/lang/reflect/Array` natives are registered in stdlib.rs. (`Integer.TYPE`
   *is* modeled; `System.arraycopy` *is* registered.)
2. **[INFERRED]** `java/lang/reflect/Array.getLength(Ljava/lang/Object;)I` and
   `java/lang/reflect/Array.set(Ljava/lang/Object;ILjava/lang/Object;)V` →
   `MethodNotFound`. Same `ArrayUtils` array-growth helpers; also unregistered.
3. **[INFERRED, likely OK]** `StringUtils.join(String[], char)`,
   `StringUtils.capitalize`, `ArrayUtils.toObject`, `ArrayUtils.contains` use only
   `StringBuilder`, `Integer.valueOf`, and index loops — expected to execute once
   the two gaps above are closed.

---

## Combined ranked "missing capabilities to implement next"

Ranked by how early they block the two happy paths:

1. **Fix `java/lang/String.format` for `%04x`** (gson) — existing native returns an
   `InvalidRef`; earliest blocker overall. *Bug, not missing.*
2. **Regex `\p{InCombiningDiacriticalMarks}` Unicode-block support in
   `java/util/regex/Pattern.compile`** (commons-lang3) — first blocker for commons.
3. **`java/lang/reflect/Field.getModifiers()I`** (gson) — first reflective blocker
   on `toJson`.
4. **`java/lang/reflect/Array.newInstance(Ljava/lang/Class;I)Ljava/lang/Object;`**
   (+ `Array.getLength`, `Array.set`) (commons-lang3) — `ArrayUtils.add`.
5. **`java/lang/reflect/Field.getGenericType()Ljava/lang/reflect/Type;`** (gson).
6. **Reflective `sun/misc/Unsafe.allocateInstance` path** (gson `fromJson`, for
   POJOs without a no-arg constructor) — lower confidence / deepest.

## Reproduction

```
# observe first blockers (both pass by pinning the exact rendered error)
cargo test -p duke-interpreter --test oss_jar_smoke smoke_surfaces_next -- --nocapture

# localize a blocker inside the jar's bytecode
DUKE_TRACE_EXEC=1 cargo test -p duke-interpreter --test oss_jar_smoke \
  gson_smoke_surfaces -- --nocapture 2>&1 | grep 'duke: trace' | tail

# end-to-end canaries (ignored until the chains are cleared)
cargo test -p duke-interpreter --test oss_jar_smoke -- --ignored
```
