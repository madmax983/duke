//! Frontier pin for the real-JDK `java/lang/ClassLoader` bootstrap wall.
//!
//! Under `real_jdk_shadow`, `java/lang/ClassLoader` is NOT on the `KEEP_SYNTHETIC`
//! allowlist, so the REAL JDK-21 `ClassLoader` classfile loads and its real
//! `<clinit>` runs. This test drives the same unmodified slf4j-simple smoke as
//! `module_model::slf4j_clears_module_wall_under_real_jdk_shadow`, but instead of
//! swallowing the error it ASSERTS the exact verbatim first-blocker string at the
//! current bootstrap frontier. It surfaces the wall (which the other test hides)
//! and regression-guards forward progress: as natives are added to push the wall,
//! the expected string is updated to the new honest frontier.
//!
//! Requires a real JDK jimage (`lib/modules`); without one it skips.

use std::path::PathBuf;
use std::sync::Arc;

use duke_gc::Heap;
use duke_interpreter::{ClassRegistry, bootstrap_stdlib, execute_class_to_completion};
use duke_loader::{BootstrapLoader, ClassLoader};
use duke_runtime::Slot;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonical workspace root")
}

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

/// Adapts a shared `Arc<dyn ClassLoader>` into an owned `'static` loader value for
/// `execute_class_to_completion`.
struct ArcLoader(Arc<dyn ClassLoader + Send + Sync>);

impl ClassLoader for ArcLoader {
    fn find_class(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        self.0.find_class(name)
    }
    fn find_resource(&self, name: &str) -> duke_loader::Result<Vec<u8>> {
        self.0.find_resource(name)
    }
    fn find_resources(&self, name: &str) -> duke_loader::Result<Vec<Vec<u8>>> {
        self.0.find_resources(name)
    }
    fn find_resource_entry(&self, name: &str) -> duke_loader::Result<duke_loader::LocatedResource> {
        self.0.find_resource_entry(name)
    }
    fn find_resource_entries(
        &self,
        name: &str,
    ) -> duke_loader::Result<Vec<duke_loader::LocatedResource>> {
        self.0.find_resource_entries(name)
    }
}

/// Drive the slf4j smoke under real-JDK shadow and return the rendered result
/// (`Ok(())` if it ran to completion, or the `{err:?}` rendering of the first blocker).
fn run_slf4j_real_jdk_shadow() -> Result<(), String> {
    // llvm-cov's `-C instrument-coverage` inflates every stack frame, and the deep
    // real-JDK bootstrap recursion this driver exercises overflows the default
    // test-harness thread stack only under that instrumentation. Run the driver on a
    // thread with explicit headroom so it survives coverage builds. The whole body is
    // built inside the inner fn, so there are no captured non-`Send` locals to move,
    // and a panic/assert inside still propagates via `join()`.
    std::thread::Builder::new()
        .name("real-jdk-frontier-driver".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_slf4j_real_jdk_shadow_inner)
        .expect("spawn frontier driver")
        .join()
        .expect("frontier driver thread panicked")
}

/// The actual driver body, run on a large-stack thread by `run_slf4j_real_jdk_shadow`.
fn run_slf4j_real_jdk_shadow_inner() -> Result<(), String> {
    let modules = jdk_modules_path().ok_or_else(|| "SKIP: no JDK jimage".to_string())?;

    let fixtures = repo_root().join("tests/fixtures");
    let oss = fixtures.join("oss-jars");
    let classpath = vec![
        fixtures,
        oss.join("slf4j-api-2.0.13.jar"),
        oss.join("slf4j-simple-2.0.13.jar"),
    ];
    let loader: Arc<dyn ClassLoader + Send + Sync> =
        Arc::new(BootstrapLoader::new(&modules, classpath).expect("build jimage+classpath loader"));

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    registry.enable_real_jdk_shadow(Arc::clone(&loader));
    bootstrap_stdlib(&mut registry, &mut heap);

    registry
        .ensure_loaded("Slf4jSimpleSmoke", loader.as_ref())
        .expect("load Slf4jSimpleSmoke from the fixture classpath");

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = [Slot::Reference(Some(args_ref))];
    let mut output = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        ArcLoader(Arc::clone(&loader)),
        &mut heap,
        &mut output,
        "Slf4jSimpleSmoke",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    );

    result.map(|_| ()).map_err(|err| format!("{err:?}"))
}

/// Pin the verbatim first-blocker at the current `java/lang/ClassLoader` bootstrap
/// frontier under real-JDK shadow. Update `EXPECTED_FRONTIER` whenever a native is
/// added that pushes the wall forward.
#[test]
fn slf4j_real_jdk_shadow_classloader_frontier_pin() {
    // The current honest frontier. The real `java/lang/ClassLoader.<clinit>` runs to
    // completion; execution proceeds through `ServiceLoader` ->
    // `jdk/internal/loader/BootLoader.<clinit>` ->
    // `jdk/internal/util/StaticProperty.<clinit>`, whose first act is
    // `System.getProperties()`. That method now materializes a live, real-layout
    // `java/util/Properties` object graph (allocate + real `<init>` + real
    // `setProperty` under the shadow flag), so `StaticProperty.<clinit>` advances
    // past it and reads the returned properties. With the synthetic String now
    // declaring the real-JDK `COMPACT_STRINGS:Z` static (init true, mirroring
    // String.<clinit> `iconst_1; putstatic COMPACT_STRINGS`), the former
    // `getstatic java/lang/String.COMPACT_STRINGS` fieldref wall is cleared and real
    // String bytecode advances into `String.coder()`. That instance method is now
    // provided as a native reading real-layout slot1 (0=Latin1/1=UTF16), so the coder
    // wall is cleared and real String bytecode advances into a `String.getBytes` copy
    // helper, one of the package-private `getBytes(byte[] dst, int dstBegin, byte coder)`
    // / `getBytes(byte[] dst, int dstBegin, int dstEnd, byte coder)` overloads the
    // synthetic KEEP_SYNTHETIC `java/lang/String` does not provide, which surfaces as a
    // `MethodNotFound { name: "java/lang/String.getBytes", descriptor: ... }`. That is a
    // separate lane (String real-layout / KEEP_SYNTHETIC allowlist), out of scope for the
    // system-properties lane.
    //
    // We pin the method NAME only and deliberately do NOT pin the exact descriptor: both
    // `([BIB)V` and `([BIIBI)V` are valid JDK-21 overloads of the package-private copy
    // helper, and which one is the *first-missing* method in the encode path is
    // JDK-build-specific (differs between local and CI runner JDKs). The frontier is the
    // getBytes copy-helper wall regardless of which overload surfaces first.
    //
    // When a native pushes the wall past this point, re-run with `-- --nocapture`,
    // read the new verbatim blocker above, and update this substring to lock it in.
    const EXPECTED_FRONTIER: &str = "MethodNotFound { name: \"java/lang/String.getBytes\",";

    let rendered = match run_slf4j_real_jdk_shadow() {
        Ok(()) => {
            eprintln!("slf4j smoke ran to completion under real-JDK shadow (no frontier)");
            return;
        }
        Err(err) if err.starts_with("SKIP:") => {
            eprintln!("skipping: {err}");
            return;
        }
        Err(rendered) => rendered,
    };

    eprintln!("=== VERBATIM CLASSLOADER BOOTSTRAP FRONTIER ===\n{rendered}\n=== END ===");

    assert!(
        rendered.contains(EXPECTED_FRONTIER),
        "ClassLoader bootstrap frontier moved.\nExpected to contain: {EXPECTED_FRONTIER}\nActual: {rendered}"
    );
}
