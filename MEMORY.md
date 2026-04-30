# MEMORY

- 2026-04-29: Before issue #637, ServiceLoader providers whose static initialization or constructors touched `java.util.concurrent.atomic` failed before useful work. The `service-loader-atomic.jar` fixture now exercises a provider backed by `AtomicLong`; `AtomicServiceLoaderInteropTest.providerClinitUsesAtomicLong()` passes after the atomic synthetic classes and natives were added.
