//! Integration tests for the real-JDK layout-coherence guard and the static layout
//! audit added alongside real-JDK shadow mode.
//!
//! The guard (`DUKE_LAYOUT_CHECK=warn|fail`) validates `getfield`/`putfield` accesses
//! under real-JDK shadow mode; its decision is the pure predicate
//! [`ClassRegistry::is_layout_incoherent`], which these tests exercise directly. The audit
//! (`DUKE_LAYOUT_AUDIT=1`) reports zero-layout-risk migration candidates via
//! [`ClassRegistry::layout_audit_candidates`].
//!
//! Flag-OFF behavior (the default) must be byte-for-byte unchanged: the guard is provably
//! a no-op because its gate (`real_jdk_shadow_enabled() && layout_check_mode() != Off`) is
//! false, and a normal program (`HelloWorld`) runs identically.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use duke_gc::Heap;
use duke_interpreter::{
    ClassRegistry, LayoutCheckMode, bootstrap_stdlib, build_class_context,
    execute_class_to_completion,
};
use duke_loader::{BootstrapLoader, ClassLoader, DirectoryLoader};

/// Locate a JDK jimage (`lib/modules`), preferring `JAVA_HOME`.
fn jdk_modules_path() -> Option<PathBuf> {
    let from_java_home = std::env::var_os("JAVA_HOME")
        .map(PathBuf::from)
        .map(|path| path.join("lib/modules"));
    let fallbacks = [
        PathBuf::from("/usr/lib/jvm/java-21-openjdk-amd64/lib/modules"),
        PathBuf::from("/usr/lib/jvm/default-java/lib/modules"),
    ];
    from_java_home
        .into_iter()
        .chain(fallbacks)
        .find(|path| path.exists())
}

/// Build a jimage-backed shadow loader, or `None` when no JDK jimage is present.
fn jimage_loader() -> Option<Arc<dyn ClassLoader + Send + Sync>> {
    let modules = jdk_modules_path()?;
    let loader = BootstrapLoader::new(&modules, Vec::<PathBuf>::new()).ok()?;
    Some(Arc::new(loader))
}

/// Absolute path to `tests/fixtures` at the repository root.
fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

// ---------------------------------------------------------------------------
// (a) Guard decision: the incoherence predicate flags corruption
// ---------------------------------------------------------------------------

/// (a1) Deterministic, no-jimage: the hard slot-bounds arm of the predicate. An access
/// whose resolved slot is out of bounds for the target object is always incoherent; an
/// in-bounds access between two same-regime (non-shadowed) classes is coherent.
#[test]
fn predicate_flags_out_of_bounds_slot() {
    let registry = ClassRegistry::new();

    // No shadow mode: neither class is shadowed, so regime always matches and only the
    // bounds arm can fire.
    assert!(
        registry.is_layout_incoherent("Foo", "Bar", 3, 2),
        "slot 3 into a 2-slot object must be flagged incoherent (silent-corruption case)"
    );
    assert!(
        registry.is_layout_incoherent("Foo", "Bar", 2, 2),
        "slot 2 into a 2-slot object (== len) must be flagged incoherent"
    );
    assert!(
        !registry.is_layout_incoherent("Foo", "Bar", 1, 2),
        "in-bounds access between two non-shadowed classes must be coherent"
    );
    assert!(
        !registry.is_layout_incoherent("Foo", "Foo", 0, 1),
        "in-bounds self access must be coherent"
    );
}

/// (a2) jimage-gated: the shadow-regime arm. A real (shadowed) resolving class accessing
/// a synthetic (`KEEP_SYNTHETIC`) object — an in-bounds slot, but the two disagree on layout
/// regime — is flagged incoherent, which is exactly the half-migrated corruption the guard
/// exists to catch. A same-regime access is coherent even at the same slot.
#[test]
fn predicate_flags_shadow_regime_mismatch() {
    let Some(loader) = jimage_loader() else {
        eprintln!("skipping predicate_flags_shadow_regime_mismatch: no JDK jimage available");
        return;
    };

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    registry.enable_real_jdk_shadow(Arc::clone(&loader));
    bootstrap_stdlib(&mut registry, &mut heap);

    let shadowed = registry
        .shadowed_classes()
        .first()
        .cloned()
        .expect("expected at least one shadowed class under real-jdk shadow mode");
    assert!(registry.is_shadowed(&shadowed));
    // A KEEP_SYNTHETIC root is never shadowed (synthetic layout).
    let synthetic = "java/lang/String";
    assert!(!registry.is_shadowed(synthetic));

    // In bounds (large slot budget) but the resolving class is real while the object is
    // synthetic → regime mismatch → incoherent.
    assert!(
        registry.is_layout_incoherent(&shadowed, synthetic, 0, 1000),
        "real resolving-class ({shadowed}) vs synthetic object-class ({synthetic}) must be \
         flagged incoherent even at an in-bounds slot"
    );
    // Same regime on both sides → coherent (only bounds could fire, and slot is in range).
    assert!(
        !registry.is_layout_incoherent(&shadowed, &shadowed, 0, 1000),
        "same-regime in-bounds access must be coherent"
    );
    assert!(
        !registry.is_layout_incoherent(synthetic, synthetic, 0, 1000),
        "same-regime synthetic in-bounds access must be coherent"
    );
}

// ---------------------------------------------------------------------------
// (b) Flag-OFF regression: default behavior is unchanged
// ---------------------------------------------------------------------------

/// A default registry has the guard provably disabled: shadow mode is off and the layout
/// check mode is `Off`, so the guard's gate can never be true and the getfield/putfield
/// fast path is byte-for-byte the original.
#[test]
fn flag_off_guard_is_provably_disabled() {
    let registry = ClassRegistry::new();
    assert!(!registry.real_jdk_shadow_enabled());
    assert_eq!(registry.layout_check_mode(), LayoutCheckMode::Off);
    // The audit likewise reports no candidates when shadow mode is off.
    assert!(registry.layout_audit_candidates().is_empty());
}

/// `HelloWorld` runs identically with the guard off (the default configuration): it executes
/// to completion and prints "Hello, World!". No shadow mode, no layout check.
#[test]
fn flag_off_helloworld_runs_identically() {
    let class_path = fixtures_dir().join("HelloWorld.class");
    let bytes = std::fs::read(&class_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", class_path.display()));
    let cf = duke_classfile::parse(&bytes).expect("HelloWorld.class should parse");
    let ctx = build_class_context(&cf);
    let entry_class = ctx.class_name.clone();

    let mut registry = ClassRegistry::new();
    registry.register(ctx);
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);
    assert_eq!(registry.layout_check_mode(), LayoutCheckMode::Off);

    let loader = DirectoryLoader::new(fixtures_dir());
    let mut out: Vec<u8> = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        loader,
        &mut heap,
        &mut out,
        &entry_class,
        "main",
        "([Ljava/lang/String;)V",
        &[duke_runtime::Slot::Reference(None)],
    );
    assert!(
        result.is_ok(),
        "HelloWorld must run to completion: {result:?}"
    );
    let printed = String::from_utf8_lossy(&out);
    assert!(
        printed.contains("Hello, World!"),
        "expected HelloWorld output, got: {printed:?}"
    );
}

// ---------------------------------------------------------------------------
// (c) Audit produces a non-empty candidate report without panicking
// ---------------------------------------------------------------------------

/// jimage-gated: with shadow mode on, the audit runs without panicking and identifies a
/// non-empty set of zero-layout-risk migration candidates (e.g. the boxed number types,
/// whose synthetic layout already matches the real jimage layout).
#[test]
fn audit_reports_nonempty_candidates() {
    let Some(loader) = jimage_loader() else {
        eprintln!("skipping audit_reports_nonempty_candidates: no JDK jimage available");
        return;
    };

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    registry.enable_real_jdk_shadow(loader);
    bootstrap_stdlib(&mut registry, &mut heap);

    // Must not panic.
    registry.run_layout_audit();

    let candidates = registry.layout_audit_candidates();
    assert!(
        !candidates.is_empty(),
        "expected a non-empty zero-layout-risk candidate list from the audit"
    );
    // java/lang/System is a fieldless synthetic root whose layout trivially matches the real
    // class, so it remains a zero-layout-risk candidate. (The boxed number types were migrated
    // out of KEEP_SYNTHETIC — see `real_jdk_shadow.rs` — so they are now shadowed, not
    // candidates.)
    assert!(
        candidates.iter().any(|c| c == "java/lang/System"),
        "expected java/lang/System among zero-layout-risk candidates, got: {candidates:?}"
    );
}
