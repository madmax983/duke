1. **Understand the problem**:
    *   The `duke-telemetry` crate is currently an optional dependency of `duke-interpreter`, enabled by the `telemetry` feature flag.
    *   This leads to scattering `#[cfg(feature = "telemetry")]` attributes everywhere in `duke-interpreter` to conditionally use telemetry types and record telemetry metrics.
    *   This is a structural problem ("The Tangle") because `duke-interpreter`'s execution logic is highly coupled with whether telemetry is enabled at compile time or not via cargo features. This pollutes `duke-interpreter`'s code.

2. **Blueprint (The Structural Fix)**:
    *   Make `duke-telemetry` a regular, non-optional dependency of `duke-interpreter`.
    *   Instead of `#[cfg(feature = "telemetry")]`, we will provide a `NoopTelemetry` or conditionally compile out the *internals* of telemetry storage within `duke-telemetry`, or just always collect the telemetry data but perhaps hide the serialization logic behind the feature flag in `duke-telemetry`.
    *   Wait, the prompt asks to look for an architectural change. Let's see how `duke-telemetry` is designed. It has `#[cfg_attr(feature = "telemetry", derive(serde::Serialize))]` and a `#[cfg(feature = "telemetry")]` block for `to_json` and `print_report`.
    *   Let's check if there is a `telemetry` feature in `duke-telemetry` already. Yes, `telemetry = ["dep:serde", "dep:serde_json"]`.
    *   So `duke-telemetry` is *always* capable of recording telemetry metrics if we include it without the `telemetry` feature, it just won't be able to serialize it to JSON! Wait, if we always record it, it might add overhead.
    *   The problem is: `duke-interpreter` has `#[cfg(feature = "telemetry")]` all over `lib.rs` (about 60 lines). This is bad architecture (Feature Flag that complicates the build and code).
    *   Instead of optional compilation in the `duke-interpreter`, we can define a trait or simply let `TelemetryStore` have no-op implementations if `telemetry` is disabled, OR we could keep `TelemetryStore` always available but its `record_xxx` methods do nothing unless a feature is enabled.
    *   Even better: In `duke-telemetry`, we can have the `telemetry` feature flag conditionally add fields to the stores, and the `record_...` methods are empty when the feature is disabled.
    *   Let's look at `duke-telemetry/src/lib.rs`.

3. **Alternative Blueprint**:
    *   In `duke-telemetry`, change the `record_*` methods to be no-ops when the `telemetry` feature is not enabled.
    *   Wait, the `telemetry` feature in `duke-telemetry` is currently only for JSON serialization (`dep:serde`, `dep:serde_json`). It doesn't gate the *collection* of telemetry data, just the *reporting*.
    *   If `duke-telemetry` *always* collects data, maybe the overhead is too high?
    *   Actually, let's look at `duke-telemetry`'s Cargo.toml again.
    *   Wait, the "Atlas" persona says:
        *   "Using `#[cfg(feature = "x")]` to allow users to opt-out of heavy parts of the system" is a favorite tactic. But `duke-interpreter` is using `#[cfg(feature = "telemetry")]` *everywhere*.
        *   "The Facade: A lib.rs that re-exports only what the user needs"
        *   If we remove `#[cfg(feature = "telemetry")]` from `duke-interpreter`, we can let `duke-interpreter` *always* call `registry.telemetry.record_...()` and have `duke-telemetry` itself handle the `#[cfg(feature = "telemetry")]` internal feature flag. If `telemetry` is disabled, `TelemetryStore` is essentially an empty struct, and its methods are empty inline functions that compile away to nothing!

4. **Implementation Plan**:
    *   In `crates/duke-telemetry/src/lib.rs`:
        *   Gate the actual storage fields inside `TelemetryStore` and the specific stores behind `#[cfg(feature = "telemetry")]`.
        *   Provide empty implementations (no-ops) for all `record_*` methods when `#[cfg(not(feature = "telemetry"))]` is active.
        *   When `#[cfg(feature = "telemetry")]` is active, the stores have their fields and do the actual recording.
    *   Wait, `duke-interpreter` uses `duke-telemetry/telemetry` feature? Let's check `duke-interpreter/Cargo.toml`: `telemetry = ["dep:duke-telemetry", "duke-telemetry/telemetry"]`. So it entirely omits `duke-telemetry` if not enabled.
    *   Change `duke-interpreter/Cargo.toml` to always depend on `duke-telemetry`, but pass the feature flag through:
        ```toml
        [dependencies]
        duke-telemetry = { path = "../duke-telemetry" }
        [features]
        telemetry = ["duke-telemetry/telemetry"]
        ```
    *   In `duke-interpreter/src/lib.rs`:
        *   Remove all `#[cfg(feature = "telemetry")]` annotations on telemetry-related field accesses and method calls.
        *   `ClassRegistry` will always have `pub telemetry: duke_telemetry::TelemetryStore`.
    *   In `crates/duke-telemetry/src/lib.rs`:
        *   Update to compile efficiently as a no-op when `telemetry` is disabled.
        *   `TelemetryStore` is always defined.
        *   If `telemetry` is disabled, the structs like `BytecodeCostStore` etc. will be empty (or have a PhantomData, or just be unit structs).
        *   Or better, we can define a macro or conditional compilation in `duke-telemetry`.

Let's double check `duke-telemetry/Cargo.toml` again.
