# MEMORY

- 2026-04-29: Before issue #637, ServiceLoader providers whose static initialization or constructors touched `java.util.concurrent.atomic` failed before useful work. The `service-loader-atomic.jar` fixture now exercises a provider backed by `AtomicLong`; `AtomicServiceLoaderInteropTest.providerClinitUsesAtomicLong()` passes after the atomic synthetic classes and natives were added.
- 2026-04-30: Before issue #640, classes whose static initialization touched `java.util.concurrent.ConcurrentHashMap` could fail before `main()` because Duke had no CHM synthetic class or natives. `ConcurrentHashMapStaticInitTest` now covers a static cache that calls `put` and `computeIfAbsent` during `<clinit>` and then reads/prints `42 84`.
