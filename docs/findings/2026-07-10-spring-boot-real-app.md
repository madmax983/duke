# Spring Boot real-app fixtures — loader-lane walk (2026-07-10)

Branch: `swarm/spring-boot-real-app`

Fixtures (committed under `tests/fixtures/oss-jars/spring-boot/`):
- `duke-spring-boot-app-3.5.12.jar` — real Spring Boot app, `Start-Class com.example.duke.DukeApplication`.
- `duke-spring-boot-ladder-3.5.12.jar` — commons-logging ladder, `Start-Class com.example.duke.ladder.LadderApplication`.

Driver: `duke -jar <fixture>` (default synthetic-stdlib path).

## Fix applied (loader lane)

`crates/duke-interpreter/src/registry.rs`, `ensure_loaded_inner` (~line 1210).

The `self.classes.contains_key(&class_key)` fast-path branch recorded
per-loader / per-code-source provenance (`class_code_sources`,
`class_runtime_loaders`) onto **plain synthetic bootstrap classes**
(`java/lang/System`, `java/lang/ClassLoader`, ...). That flipped
`is_plain_bootstrap_class(name)` from `true` -> `false`, so a later
`class_key_from_provenance` call started computing a **loader-suffixed** key
(`java/lang/System\0loader:113`) for the same singleton — a key nothing was
stored under -> `ClassNotFound`.

Symptoms before the fix:
- app fixture: `class not found: java/lang/System loader:113`
- ladder fixture: `class not found: java/lang/ClassLoader loader:77`

HelloWorld survived only because it referenced `System` exactly once (the key
was computed before the poison was recorded).

Fix: guard the fast-path recording branch with
`if !self.is_plain_bootstrap_class(&internal_name)` so plain bootstrap
singletons never acquire provenance, keeping `is_plain_bootstrap_class` and
`class_key_from_provenance` in sync.

## Post-fix first blocker (both fixtures) — OTHER LANE, reported not fixed

Both fixtures now boot much further (into the JarLauncher / Spring Boot
classpath-scanning path) and land on the SAME blocker:

```
duke: runtime error: method not found: java/lang/ClassLoader.getSystemResources(Ljava/lang/String;)Ljava/util/Enumeration;
```

Classification: **other lane (stdlib/native)**. `java/lang/ClassLoader` and its
resource methods (`getResource`, `getResources`, `getResourceAsStream`) are
synthetic bootstrap methods defined in
`crates/duke-interpreter/src/stdlib.rs` (off-limits). `getSystemResources` is a
missing synthetic method; adding it requires editing `stdlib.rs`/`native/`,
which is an explicit STOP condition for the loader-lane walk. No further
loader-lane rungs remain — the chain terminates here.

Owning lane: synthetic-stdlib / native (`ClassLoader` resource API surface).
