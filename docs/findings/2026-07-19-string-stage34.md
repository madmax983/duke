# String real-layout migration — Stage 3 + Stage 4 findings (2026-07-19)

Branch: `swarm/string-stage34`. Start tip `906e059` (Stage 3a + 3b). This
document captures the Stage 3 surface that Stage 4 builds on and records the
**Stage 4 verdict**: run the real `java/lang/String.<clinit>` by dropping String
from `KEEP_SYNTHETIC`.

## Stage 3a — constructor-family coverage

Stage 3a (`9783f51`) added `native_string_init_from_string_builder`, handling
both:

- `String.<init>(Ljava/lang/StringBuilder;)V`
- `String.<init>(Ljava/lang/StringBuffer;)V`

Both builder types back their char storage on the `string_value` side-channel
(mirroring their `toString` natives). The handler reads those chars and mints the
receiver through `store_string_init_value` on the real 4-slot layout
(`value:[B` / `coder:B` chosen Latin1 vs UTF16LE from content, via
`set_string_layout`). A null builder arg throws `NullPointerException`. Registered
in `stdlib.rs` alongside the other String `<init>` natives.

This was the piece that moved the real-jdk bootstrap **past** the old
`String(StringBuilder)` `MethodNotFound` wall (see frontier movement below).

## Stage 3b — CharSequence-narrowing census

Stage 3b (`906e059`) routed every production reader that could receive a
real-layout `java/lang/String` off the `string_value` side-channel and onto
slot 0 (`value:[B`) via `read_string_bytes`. Two dispatch helpers were added in
`native/common.rs`:

- `charsequence_chars` (`common.rs:326`) — String → slot-0 decode; other char
  backings → their own `string_value` buffer.
- `heap_object_to_string_ref` (`common.rs:342`) — String → slot 0; everything
  else → `heap_object_to_string`.

Non-String metadata backings (StringBuilder/StringBuffer buffers, Throwable
messages, Class/Charset/Provider/Level/ZoneId/Pattern/Matcher names) continue to
use `string_value`.

Narrowed sites (census summary): `String.join` / `valueOf(Object)` / `format %s`,
`Object.toString`, `Objects.toString`, `print(ln)(Object)`,
StringBuilder/StringBuffer `append` overloads, `Arrays.toString`,
TreeMap/TreeSet/HashMap key handling, `Pattern.split`/`matches`, Matcher
`find`/`matches`/`replace*`/`group`/`append*`, StringJoiner helpers, Properties,
`Locale.toString`, `URLDecoder.decode`, ZipFile/ZipEntry names, InputStreamReader
charset, `AtomicReference.toString`, and the `String.coder` defensive fallback.

## `string_value` fate verdict — demote-to-debug-only is BLOCKED

The goal of demoting `string_value` to a debug-only field (so real-layout String
reads exclusively from slot 0) is **blocked** by four remaining production
readers that live in `native/duke_util.rs`, which is a **forbidden lane** for
this workstream:

- `duke_util.rs:178` and `:208` — the `Collectors.joining` collector drive path.
- `duke_util.rs:1583` — `native_collectors_joining(delim)`.
- `duke_util.rs:1614` — `native_collectors_joining_full(delim, prefix, suffix)`.

These read `obj.string_value` directly to fetch the delimiter/prefix/suffix
Strings. Until they are narrowed to `charsequence_chars` (the helper already
exists in `common.rs:326`), `string_value` must remain a live production field
for String. **Handoff:** this belongs to a `duke_util.rs`-permitted lane — once
those four readers move to slot-0 dispatch, `string_value` on String can be
demoted to debug-only.

## Real-jdk frontier movement

Stage 3a advanced the real-jdk bootstrap frontier:

- **Before:** `String(StringBuilder)` → `MethodNotFound`.
- **After:** with `String(StringBuilder)`/`(StringBuffer)` resolving, the
  bootstrap advances into `jdk/internal/util/StaticProperty.<clinit>`, which
  throws `InternalError` (the saved system-properties map lacks a required key).

Both frontier pins were updated to that new honest blocker
(`classloader_bootstrap_frontier.rs` and `spring_boot_real_jdk.rs` now pin
`java/lang/InternalError`). Stage 4 does not move this frontier further — the
`StaticProperty.<clinit>` `InternalError` remains the pinned wall.

## Stage 4 — drop String from `KEEP_SYNTHETIC`

### Change
Removed `"java/lang/String"` from `KEEP_SYNTHETIC` (`registry.rs:22`). Under
real-jdk shadow mode, `register(string_ctx)` now takes the
`should_shadow_synthetic` branch: the synthetic String ClassContext is skipped
and recorded as shadowed, so `ensure_loaded` fetches the real JDK String
classfile and the real `String.<clinit>` runs (setting `COMPACT_STRINGS` for
real). Default (flag-off) mode is untouched — `real_jdk_shadow` is false, so
`should_shadow_synthetic` always returns false and String stays synthetic with
its natives firing byte-identically.

### COMPACT_STRINGS shim
The synthetic `COMPACT_STRINGS:Z` shim lived inside `string_ctx` in
`stdlib.rs`. It **drops naturally** under shadow: when String is shadowed,
`string_ctx` is never registered, so the shim is simply not present and real
`String.<clinit>` supplies `COMPACT_STRINGS`. No shim code was removed — in
default (flag-off) mode `string_ctx` is still registered (harmless, and no real
bytecode reads `COMPACT_STRINGS` in that mode), and removing the `FieldEntry`
would have required re-indexing the `static_fields` slot order for
`CASE_INSENSITIVE_ORDER`, a needless risk to default mode for no shadow-mode
benefit. Net effect matches the intent: the shim is inert under shadow.

### Dispatch-preference finding (verified empirically)
`shadowed_bytecode_override` (`registry.rs`) prefers real classfile bytecode over
a registered synthetic native only when (a) shadow mode is on, (b) the owning
class is shadowed, and (c) the real class declares a **concrete** (non-native,
non-abstract) body. `javap -p java.lang.String` (JDK 21) shows the **only**
native method in real String is `intern()`. Therefore, after un-shadowing:

- **Kept (our native still fires):** `intern()` — real String declares it
  `native`, so `shadowed_bytecode_override` returns `None` and our synthetic
  native is preserved.
- **Overridden (real bytecode wins):** every other instance method and
  constructor — `length`, `coder`, `charAt`, `equals`, `equalsIgnoreCase`,
  `substring`, `indexOf`, `lastIndexOf`, `contains`, `hashCode`, `getBytes`,
  `getChars`, `isEmpty`, `isBlank`, the `<init>` family, etc. Our natives for
  these stay **registered** (they are the trigger — `apply_shadow_override` only
  fires when resolution first lands on `NativeOverride`) but are **never
  dispatched** for the shadowed String. No conflict: a registered-but-overridden
  native is exactly what the override path consumes.

Correctness of the overridden paths on our 4-slot layout is confirmed at runtime:
a String-heavy micro-workload (charAt/length/hashCode/indexOf/substring in a
loop) yields **byte-identical** results in default (native) and real-jdk
(real-bytecode) modes (`acc=-2437696252000` in both). The full canary/pin suite
(commons-lang3, gson, slf4j, spring-boot real-jdk) — all of which construct and
manipulate Strings heavily — passes with String shadowed.

### Stage 4 verdict: **LANDED**

The full gate is green with String shadowed:

- `cargo build --workspace` — clean.
- `cargo test --workspace` — **3177 passed / 0 failed / 3 ignored** (no net
  regression vs. the 3177/0/3 baseline; the exemplar-test edits keep the count).
- `cargo fmt --all --check` — clean.
- `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -W
  clippy::pedantic -W clippy::nursery` — zero warnings.
- `layout_coherence` (primary oracle) — 7 passed / 0 failed. The coherent real
  `<clinit>` guard does not trip.
- HelloWorld all 4 modes (default; `DUKE_LAYOUT_CHECK=fail`; `DUKE_REAL_JDK=1
  --real-jdk`; both) — all print `Hello, World!`, exit 0. Critically,
  `DUKE_LAYOUT_CHECK=fail` does **not** trip with String shadowed.
- Canaries `oss_jar_smoke` — slf4j/gson/commons-lang3 pass (5 passed / 1 ignored,
  the pre-existing `#663` ignore).
- Pins `classloader_bootstrap_frontier`, `spring_boot_real_jdk`,
  `real_jdk_shadow`, `module_model`, `streamencoder_writer_frontier`,
  `writer_graph_frontier` — all pass.

Perf impact is confined to real-jdk shadow mode (an experimental mode) and is
recorded in `RESULTS.md`: hot String loops run ~2x slower under shadow because
`charAt`/`length`/`hashCode`/`indexOf`/`substring` move from native Rust to
interpreted real JDK bytecode. Default mode is byte-identical (unchanged).

Verdict: **Stage 4 LANDED** as a single, cleanly-isolated commit
(`feat(string): run real String.<clinit> (drop from KEEP_SYNTHETIC)`).
