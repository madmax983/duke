//! Integration tests for the opt-in "real JDK shadow" mode on [`ClassRegistry`].
//!
//! When the flag is OFF (the default), `bootstrap_stdlib` must register every
//! synthetic stdlib class exactly as before. When the flag is ON, synthetic
//! classes that are NOT on the keep-synthetic allowlist and whose real classfile
//! is resolvable from the JDK jimage are skipped, so a later `ensure_loaded`
//! loads their real JDK bytecode instead.

use std::path::PathBuf;
use std::sync::Arc;

use duke_gc::Heap;
use duke_interpreter::{ClassLoadSource, ClassRegistry, bootstrap_stdlib};
use duke_loader::{BootstrapLoader, ClassLoader};

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

/// Build a jimage-backed loader, or `None` when no JDK jimage is present.
fn jimage_loader() -> Option<Arc<dyn ClassLoader + Send + Sync>> {
    let modules = jdk_modules_path()?;
    let loader = BootstrapLoader::new(&modules, Vec::<PathBuf>::new()).ok()?;
    Some(Arc::new(loader))
}

/// Allowlisted roots that MUST stay synthetic regardless of the flag.
const KEEP_SYNTHETIC_SAMPLE: &[&str] = &[
    "java/lang/String",
    "java/lang/System",
    "java/lang/Class",
    "java/lang/Thread",
];

/// Classes that were empirically migrated OUT of `KEEP_SYNTHETIC` (zero-layout-risk: their
/// synthetic instance-field count matched the real jimage layout, and removing them from the
/// allowlist regressed no previously-passing fixture). Under the flag they MUST now be
/// shadowed by real JDK bytecode. This locks in the migration delta.
const MIGRATED_TO_REAL_SAMPLE: &[&str] = &[
    "java/lang/Object",
    "java/lang/Integer",
    "java/lang/Long",
    "java/lang/Short",
    "java/lang/Byte",
    "java/lang/Boolean",
    "java/lang/Character",
    "java/lang/Float",
    "java/lang/Double",
    "java/lang/Number",
    "java/io/OutputStream",
    "java/io/InputStream",
];

/// Flag OFF (default): full synthetic stdlib registered, nothing shadowed.
#[test]
fn flag_off_registers_synthetic_stdlib_and_shadows_nothing() {
    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    bootstrap_stdlib(&mut registry, &mut heap);

    assert!(
        !registry.real_jdk_shadow_enabled(),
        "real-jdk shadow must be off by default"
    );
    assert!(
        registry.shadowed_classes().is_empty(),
        "nothing should be shadowed when the flag is off, got {:?}",
        registry.shadowed_classes()
    );

    // A core synthetic root is present and synthetic.
    let string = registry
        .get("java/lang/String")
        .expect("java/lang/String synthetic stub should be registered");
    assert_eq!(string.load_source, ClassLoadSource::Synthetic);
}

/// Flag ON: non-allowlisted synthetics backed by a real classfile are skipped,
/// the allowlist is protected, and a shadowed class loads as real bytecode.
#[test]
fn flag_on_shadows_non_allowlisted_synthetics_but_protects_allowlist() {
    let Some(loader) = jimage_loader() else {
        eprintln!("skipping flag_on test: no JDK jimage (lib/modules) available");
        return;
    };

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    registry.enable_real_jdk_shadow(Arc::clone(&loader));
    bootstrap_stdlib(&mut registry, &mut heap);

    assert!(registry.real_jdk_shadow_enabled());

    // Allowlisted roots MUST remain synthetic even with the flag on, and must
    // never be recorded as shadowed.
    for keep in KEEP_SYNTHETIC_SAMPLE {
        let ctx = registry
            .get(keep)
            .unwrap_or_else(|_| panic!("{keep} should remain registered synthetically"));
        assert_eq!(
            ctx.load_source,
            ClassLoadSource::Synthetic,
            "{keep} must stay synthetic under real-jdk shadow"
        );
        assert!(
            !registry.shadowed_classes().iter().any(|c| c == keep),
            "{keep} must not be shadowed"
        );
    }

    // Classes migrated out of KEEP_SYNTHETIC MUST now be shadowed by real bytecode:
    // recorded as shadowed and absent from the synthetic registry (so ensure_loaded
    // fetches the real classfile). This locks in the migration delta.
    for migrated in MIGRATED_TO_REAL_SAMPLE {
        assert!(
            registry.is_shadowed(migrated),
            "{migrated} was migrated out of KEEP_SYNTHETIC and must be shadowed under the flag"
        );
        assert!(
            registry.shadowed_classes().iter().any(|c| c == migrated),
            "{migrated} must be recorded among the shadowed classes"
        );
        assert!(
            registry.get(migrated).is_err(),
            "migrated class {migrated} should not be pre-registered synthetically"
        );
    }

    // With a real JDK jimage, at least one non-allowlisted synthetic should have
    // been shadowed (its real classfile exists in java.base).
    assert!(
        !registry.shadowed_classes().is_empty(),
        "expected some synthetic stdlib classes to be shadowed by real JDK bytecode"
    );

    // Every shadowed class: absent from the registry (so ensure_loaded will fetch
    // real bytecode) and resolvable from the jimage.
    for name in registry.shadowed_classes() {
        assert!(
            registry.get(name).is_err(),
            "shadowed class {name} should not be pre-registered synthetically"
        );
        assert!(
            loader.find_class(name).is_ok(),
            "shadowed class {name} must be resolvable from the jimage"
        );
        assert!(
            !KEEP_SYNTHETIC_SAMPLE.contains(&name.as_str()),
            "allowlisted class {name} must never be shadowed"
        );
    }

    // A shadowed class loads as real Classfile bytecode via ensure_loaded.
    let name = registry
        .shadowed_classes()
        .first()
        .cloned()
        .expect("at least one shadowed class");
    let loaded = registry
        .ensure_loaded(&name, loader.as_ref())
        .expect("ensure_loaded should succeed for a jimage class");
    assert!(loaded, "ensure_loaded should load {name} from the jimage");
    let ctx = registry
        .get(&name)
        .expect("shadowed class should be registered after ensure_loaded");
    assert_eq!(
        ctx.load_source,
        ClassLoadSource::Classfile,
        "{name} should now be real classfile bytecode"
    );
}
