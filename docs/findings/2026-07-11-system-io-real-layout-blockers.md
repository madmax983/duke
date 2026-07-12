# System/IO real-layout migration — phase 1 findings & blocker map (2026-07-11)

This document is the durable record of the phase-1 investigation into making
`System.out`/`System.err`/`System.in` real-layout `PrintStream` instances under
`DUKE_REAL_JDK=1`, and (ideally) removing `System`, `PrintStream`,
`FileOutputStream`, and `FileInputStream` from the `KEEP_SYNTHETIC` allowlist
(`crates/duke-interpreter/src/registry.rs:22-45`, currently 10 members). The
short version: none of the three IO migrations is safely landable this wave, each
is independently multi-wave, and the load-bearing reasons are captured below so
the next wave does not re-derive them.

---

## 1. Goal & outcome

**Goal.** Promote the stdio/file streams to real JDK layout under
`DUKE_REAL_JDK=1` and shrink `KEEP_SYNTHETIC`
(`crates/duke-interpreter/src/registry.rs:22-45`).

**Outcome.** Exhaustive investigation shows all three IO migrations
(`PrintStream`, `System`, `FileOutputStream`/`FileInputStream`) are
**independently multi-wave**. **No allowlist reduction is safely landable this
wave.** Forcing one is the classic #1307 half-migration / silent-corruption trap:
real-layout bytecode running against a synthetically-shaped instance (or vice
versa) corrupts fields without an obvious crash.

**What landed instead:** the `setOut0`/`setErr0`/`setIn0` System-init seeding
infrastructure (commit `df33dcd`) — a shared `set_system_stream` helper plus
three static callback natives on `java/lang/System`, with the existing bootstrap
seeding routed through the same helper (byte-identical), plus 5 unit tests. The
allowlist is **unchanged (10 members)**. Tests 3042 → 3047 (+5), fmt/clippy clean,
flag-off byte-identical, HelloWorld exit 0 flag-on and flag-off.

---

## 2. The oracle & real slot counts

Two environment-gated tools drive this work:

- **`DUKE_LAYOUT_CHECK=warn|fail`** (`registry.rs` ~58-79). A runtime
  layout-coherence guard that fires on `getfield`/`putfield`
  (`execution.rs` ~1587-1646) via `is_layout_incoherent`
  (`registry.rs` ~822-840). `warn` logs; `fail` returns
  `Error::FieldOutOfBounds` — it is a recoverable error return, **not** a panic.
- **`DUKE_LAYOUT_AUDIT=1`** (`registry.rs` ~888-961). Emits the real-vs-synthetic
  instance-field-count audit table used below.

Measured audit table (real vs synthetic **instance**-field counts):

| Class                     | Synthetic slots | Real slots           | Verdict         |
|---------------------------|-----------------|----------------------|-----------------|
| `java/lang/System`        | 0               | 0                    | MATCH candidate |
| `java/io/PrintStream`     | 1               | 8 own / 11 total     | MISMATCH        |
| `java/io/FileOutputStream`| 1               | 5                    | MISMATCH        |
| `java/io/FileInputStream` | 1               | 5                    | MISMATCH        |

**Audit summary:** 2 of 10 allowlist members are slot-count candidates —
`System` and `jdk/internal/misc/Unsafe`.

Two caveats:

- **`System`'s candidacy is misleading.** Its slot count matches (0/0) but its
  blocker is *behavioral*, not slots (see §4).
- **Correct a prior assumption:** real `PrintStream` is **8 own / 11 total**
  slots (own + super chain), **not** "11 own". Any writer-graph work must size
  against 8 own fields.

---

## 3. PIVOTAL ARCHITECTURAL FINDING — "bytecode wins" for shadowed classes

**When a shadowed (real-layout) class's method has both a registered Duke native
AND a real bytecode body, Duke dispatches the REAL BYTECODE in preference to the
native.**

Evidence:

- `apply_shadow_override` converts `NativeOverride` → `Bytecode`
  (`execution.rs:40-46`).
- This runs on the `invokestatic` path (`execution.rs:542-566`) and the sibling
  invoke paths.
- The gate `shadowed_bytecode_override` returns `Some` only when there is a
  concrete method body and the class `is_shadowed`
  (`registry.rs:1038-1041`).

**Consequence.** The hoped-for shortcut — "allocate a real-layout `PrintStream`
but keep the native `println` at the bottom of the stack as a shim" — is
**infeasible**. Dropping `PrintStream` from the allowlist loads the real 11-slot
bytecode, which runs *instead of* the native and dies with
`type mismatch: expected reference, got int`: the real `println` executes
`getfield charOut` / `textOut` / `lock` and finds the `Int(0)` placeholder left
by synthetic allocation — and it does this **before** control ever reaches the
layout guard. A real `System.out`/`System.err` `PrintStream` therefore requires
**actually constructing a valid writer graph**, not a native shim.

This finding generalizes: no real-layout migration on the allowlist can be done
by "keep the native as a fallback." Once shadowed, the real bytecode wins, so the
real object graph must be genuinely well-formed.

---

## 4. Three independent multi-wave blocker chains

| Chain                        | Immediate blocker                                                             | Drags in                                                          | Waves |
|------------------------------|-------------------------------------------------------------------------------|------------------------------------------------------------------|-------|
| `PrintStream`                | real `println` `getfield charOut/textOut/lock` hits `Int(0)` placeholder      | nio charset, `StreamEncoder`, full writer graph                  | ≥1    |
| `System`                     | needs runnable real `<clinit>`/`initPhase1` + a real `PrintStream`            | the whole `PrintStream` chain; also 2 locked tests               | ≥1    |
| `FileOutputStream`/`In`      | first insn of real `FileOutputStream.<clinit>@0` → `SharedSecrets` → `invoke` | `java.lang.invoke` engine, `Reflection.getClassAccessFlags`, Blocker, Cleaner | ≥2 |

### PrintStream

Real `PrintStream` needs a real writer graph populated **before** any real
bytecode touches it: `charOut`/`textOut` must be
`BufferedWriter → OutputStreamWriter → StreamEncoder + Charset`, terminating in a
native-backed `OutputStream`, and `lock` must be non-null. This drags in the nio
charset machinery and `StreamEncoder`. Evidence: the type-mismatch crash in §3.

### System

Real `System.<clinit>` / `initPhase1` calls `setOut0` / `setErr0` / `setIn0`
(now scaffolded — commit `df33dcd`, see §6), but it also needs a runnable real
`<clinit>` **and** the real `PrintStream` from the chain above. Two tests LOCK
`System` synthetic and must be updated when it migrates:

- `crates/duke-interpreter/tests/real_jdk_shadow.rs` — `KEEP_SYNTHETIC_SAMPLE`
  (~line 110).
- `crates/duke-interpreter/tests/layout_coherence.rs` —
  `audit_reports_nonempty_candidates` (~line 213), which currently
  **hard-asserts** that `System` is an audit candidate.

### FileOutputStream / FileInputStream — a VERIFIED REGRESSION if dropped

Real-JDK file I/O works **while the streams stay synthetic**: a write+read
fixture prints `READBACK=DUKEIO` and the host file is written. **Dropping**
`FileOutputStream`/`FileInputStream` from the allowlist dies at the **first
instruction** of real `FileOutputStream.<clinit>@0`:

1. `SharedSecrets.getJavaIOFileDescriptorAccess()` finds `FD_ACCESS == null`.
2. That forces `MethodHandles.Lookup.ensureInitialized(FileDescriptor.class)`.
3. Which spins up the whole `java.lang.invoke` engine (~50 classes loaded).
4. Which reaches the **unregistered** native
   `jdk/internal/reflect/Reflection.getClassAccessFlags(Ljava/lang/Class;)I`.
5. Crash: `local variable index 1 out of bounds (max_locals=0)`.

The leaf file natives (`open0`, `writeBytes`, `readBytes`, `read0`, `close0`,
`initIDs`) are **never reached** — the class initializer dies long before any
actual I/O.

Additional deep dependencies on this path:

- **`jdk/internal/misc/Blocker.begin`/`end`** on *every* open/read/write. These
  need `SharedSecrets.getJavaLangAccess()`. This is made **worse** because Duke
  seeds `VM.isBooted() == true` (`jdk_internal.rs:231`), so `Blocker` takes its
  full path instead of the pre-boot `-1` short-circuit.
- **`FileCleanable.register` → `CleanerFactory.cleaner()`** (a daemon `Cleaner`
  thread) plus `PhantomCleanable`.

**None** of `Cleaner`, `CleanerFactory`, `SharedSecrets`,
`JavaIOFileDescriptorAccess`, `PhantomCleanable`, `FileCleanable`, or `Blocker`
is implemented in Duke (grep: zero hits). The file natives that DO exist are only
the synthetic abstract-API handlers keyed by public method names
(`stdlib.rs` ~2752-2818, `java_io.rs`), and they model `fd` as an **`int`**,
which is incompatible with the real layout where field 0 is a `FileDescriptor`
**reference**.

---

## 5. Dependency DAG + staged plan for remaining waves

The three chains are not equal-depth; they share one foundational prerequisite.

```
(a) reflection / java.lang.invoke foundation        [SHARED PREREQUISITE]
        Reflection.getClassAccessFlags
        + enough MethodHandles bootstrap to run
              │
              ▼
(b) SharedSecrets / JavaLangAccess / JavaIOFileDescriptorAccess
        + Blocker + minimal Cleaner/FileCleanable
        (or: bootstrap shortcut pre-initting FileDescriptor.<clinit>)
              │
              ├─► wire leaf file natives to host-file plumbing
              │     (open_host_output_file / open_host_input_file),
              │     storing file_id inside real FileDescriptor.fd
              │           │
              │           ▼
              │     drop File*Stream (10 → 8)
              │
(c) charset / StreamEncoder + writer graph + System.initPhase1
        (using now-landed setOut0/setErr0)
              │
              ▼
        real PrintStream + real System
        (drop both → update the two locked tests in §4)
```

**Stage (a) — reflection / `java.lang.invoke` foundation (Module/reflection
lane).** Register `Reflection.getClassAccessFlags` and enough of the
`MethodHandles` bootstrap to run. This is a **shared prerequisite** that unblocks
the `SharedSecrets` accessors used by both the file and stream chains.

**Stage (b) — file streams.** Implement `SharedSecrets` / `JavaLangAccess` /
`JavaIOFileDescriptorAccess` + `Blocker` + a minimal `Cleaner` / `FileCleanable`.
Alternatively, a **bootstrap shortcut**: pre-initialize `FileDescriptor.<clinit>`
so `FD_ACCESS` is non-null and `FileOutputStream.<clinit>` skips the
`MethodHandles` path via its `ifnonnull`. Then wire the leaf file natives
(`open0`/`writeBytes`/`readBytes`/`read0`/`close0`) to Duke's existing host-file
plumbing (`open_host_output_file` / `open_host_input_file`), storing the
`file_id` inside the real `FileDescriptor.fd`. Then drop `FileOutputStream` and
`FileInputStream` (10 → 8). **Caveat:** the "pre-init `FileDescriptor.<clinit>`"
shortcut is **unverified** and still leaves the `Blocker`/`JLA` and Cleaner paths
to handle on every open/read/write.

**Stage (c) — streams + System.** Build the charset / `StreamEncoder` layer and
the writer graph, run `System.initPhase1` (using the now-landed
`setOut0`/`setErr0`), producing a real `PrintStream` and real `System`. Drop both
from the allowlist and update the two locked tests (§4).

---

## 6. What landed this wave (`df33dcd`)

The **System-side down-payment**, inert until the eventual migration but
exercised on every startup:

- A shared `set_system_stream(ops, field, ref)` helper.
- Three static callback natives on `java/lang/System` — `setOut0`, `setErr0`,
  `setIn0` — using `CallbackOps::write_static_field`.
- The existing bootstrap stdio seeding routed through the **same** helper (live,
  byte-identical to before).
- A null-seeded `System.in` static field so `setIn0` has a target slot.
- 5 unit tests.

`System` stays on `KEEP_SYNTHETIC`. This path runs on every startup (the seeding
path) and is unit-tested, so it will not bit-rot before the migration wave picks
it up.

---

## 7. Measurements

| Metric                 | Before  | After   | Note                                                |
|------------------------|---------|---------|-----------------------------------------------------|
| `KEEP_SYNTHETIC` size  | 10      | 10      | No safe reduction this wave                          |
| Tests                  | 3042    | 3047    | +5, 0 failed, 4 ignored                              |
| fmt / clippy           | clean   | clean   | 1.97 pedantic + nursery, `-D warnings`               |
| Flag-off output        | —       | byte-identical | bootstrap routes the same refs to the same slots |
| HelloWorld (flag-off)  | —       | exit 0  | prints `Hello, World!`                               |
| HelloWorld (flag-on)   | —       | exit 0  | `DUKE_REAL_JDK=1 DUKE_LAYOUT_CHECK=fail`             |
| `oss_jar_smoke`        | —       | 5 passed / 1 ignored | unchanged                                |

**Known flaky test (unrelated to this work):** the JDWP test
`test_jdwp_stackframe_getvalues_oom` (`havoc_jdwp_oom.rs`) failed once then passed
on rerun during baseline. It is unrelated to this migration but is worth flagging
for CI stability.

---

## 8. Edge-1 update (2026-07-12) — `java.lang.invoke` / `SharedSecrets` foundation

Stage (a)/(b) down-payment: walking the FIRST edge of the file chain until the
real `java/io/FileOutputStream.<clinit>` runs to completion under
`DUKE_REAL_JDK=1`. Probed empirically with a throwaway removal of
`FileOutputStream` from `KEEP_SYNTHETIC` under `DUKE_LAYOUT_CHECK=fail`, driving a
fixture that constructs a `FileOutputStream`. **The probe was reverted** — the
allowlist is unchanged (still 10 members). The half-migration would regress
because full FOS *construction* is not yet wired (see the next-blocker wall
below); only `<clinit>` completes.

### Chain progress — before vs. after

**Before this wave** (post-#1330, `getClassAccessFlags` already landed): dropping
`FileOutputStream` died at `Unsafe.ensureClassInitialized` — `method not found`.

**After this wave:** real `FileOutputStream.<clinit>` runs to completion
(`FileOutputStream::<clinit>@9 return` observed under `DUKE_TRACE_EXEC=1`). The
full cleared chain is:

1. `FileOutputStream.<clinit>@0` → `SharedSecrets.getJavaIOFileDescriptorAccess()`
   (FD_ACCESS null) → `MethodHandles.Lookup.ensureInitialized(FileDescriptor)`
   (real invoke bytecode: `VerifyAccess.isClassAccessible`, `checkSecurityManager`)
   → `Unsafe.ensureClassInitialized(FileDescriptor.class)`. **[native added]**
2. That forces real `FileDescriptor.<clinit>`, which opens with native
   `initIDs()V` **[native added, no-op]**, installs the `JavaIOFileDescriptorAccess`
   via `SharedSecrets.setJavaIOFileDescriptorAccess` (real bytecode), and builds
   `in`/`out`/`err` via `FileDescriptor(int)` — whose body calls native
   `getHandle(I)J` **[native added, −1 on unix]** and `getAppend(I)Z`
   **[native added, false]**.
3. Control returns; `SharedSecrets.getJavaIOFileDescriptorAccess()` now returns
   non-null FD_ACCESS.
4. `FileOutputStream.<clinit>@3` stores FD_ACCESS, `@6` runs native
   `FileOutputStream.initIDs()V` **[native added, no-op]**, `@9 return`. **DONE.**

### Natives added (all honest, dormant while FOS stays synthetic)

Registration in `stdlib.rs` (`bootstrap_stdlib` tail, grouped block
`// java.lang.invoke / SharedSecrets foundation`); handlers in
`native/jdk_internal.rs`:

| Native | Descriptor | Behavior |
|--------|-----------|----------|
| `jdk/internal/misc/Unsafe.ensureClassInitialized` | `(Ljava/lang/Class;)V` | drives arg-1 `Class` through `CallbackOps::ensure_class_initialized` (real `<clinit>`) |
| `java/io/FileDescriptor.initIDs` | `()V` | no-op (nothing to cache under positional fields) |
| `java/io/FileOutputStream.initIDs` | `()V` | no-op (same) |
| `java/io/FileDescriptor.getHandle` | `(I)J` | `-1` (Windows-only concept; matches unix native) |
| `java/io/FileDescriptor.getAppend` | `(I)Z` | `false` (std descriptors built in `<clinit>` are non-append) |

4 direct-dispatch unit tests pin these contracts (`tests.rs`), since the natives
are dormant in the default (FOS-synthetic) config.

### VERBATIM next blocker (the next wall — a NEW, deeper chain)

Past `<clinit>`, FOS *construction* (`new FileOutputStream(path)`) allocates a
`File`, whose `File.<clinit>` reaches `UnixFileSystem.<clinit>@0 invokestatic`:

```
duke: trace java/io/UnixFileSystem::<clinit>@0 invokestatic stack=0
duke: runtime error: fell off end of bytecode without a return instruction
```

`UnixFileSystem.<clinit>` is `0: invokestatic initIDs:()V; 3: return` — i.e. the
native `java/io/UnixFileSystem.initIDs()V`. This is **Stage (b) file-stream
territory**, not the invoke foundation: it drags in the whole `File` /
`FileSystem` / `UnixFileSystem` layer plus the leaf file natives
(`open0`/`writeBytes`/`readBytes`/`read0`/`close0`), which still model `fd` as an
`int` incompatible with the real `FileDescriptor` reference field. That is a
separate wave and was intentionally NOT walked here.

**Allowlist delta: none.** `KEEP_SYNTHETIC` remains 10 members. Tests
3059 → 3063 (+4). fmt/clippy clean, HelloWorld exit 0 flag-on and flag-off,
real-JDK gate tests green.

---

## Summary

Phase 1 is an investigation plus a scaffolding down-payment. The single most
load-bearing takeaway is §3: **bytecode wins over registered natives for shadowed
classes**, so no allowlist member can be migrated with a native-shim shortcut —
the real object graph must be genuinely well-formed. The staged plan in §5
sequences the remaining waves behind one shared `java.lang.invoke` /
`Reflection.getClassAccessFlags` prerequisite. §8 lands the first slice of that
prerequisite: real `FileOutputStream.<clinit>` now completes under
`DUKE_REAL_JDK=1`; the next wall is the `UnixFileSystem`/leaf-file-native layer
(Stage b), which the allowlist still correctly guards.
