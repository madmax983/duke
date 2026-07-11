//! Real-JDK integration guard for the minimal `java.lang.Module` model.
//!
//! Under `real_jdk_shadow`, the synthetic `java/util/ServiceLoader` is replaced by
//! the real JDK bytecode, which — unlike Duke's intrinsic `ServiceLoader` — walks the
//! module system: `caller.getModule()`, `Module.isNamed()`, `Reflection.verify*Access`
//! and `Reflection.getClassAccessFlags`. This test drives the unmodified slf4j-simple
//! smoke through that path and asserts the module-system wall is cleared: no module
//! native is left unimplemented.
//!
//! It requires a real JDK jimage (`lib/modules`); without one it skips (there is no
//! real `ServiceLoader` to shadow in). Flag-off behavior is unaffected — this test is
//! the only place that enables the shadow for the slf4j smoke.

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

/// Adapts a shared `Arc<dyn ClassLoader>` (needed for `enable_real_jdk_shadow`) into
/// an owned, `'static` loader value for `execute_class_to_completion`.
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

#[test]
fn slf4j_clears_module_wall_under_real_jdk_shadow() {
    let Some(modules) = jdk_modules_path() else {
        eprintln!("skipping: no JDK jimage (lib/modules) available");
        return;
    };

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

    // The module-system wall is cleared: no Module / module-reflection native is
    // left unimplemented. Before this change, execution failed with
    // `MethodNotFound java/lang/Class.getModule`; it now flows through
    // getModule -> Module.isNamed -> Reflection.verifyModuleAccess ->
    // getClassAccessFlags -> ServiceLoader.checkCaller. If any of those natives
    // regresses, the corresponding method name resurfaces in the error here.
    // (Execution stops later at the separate ClassLoader system-bootstrap
    // frontier, which is out of scope for the module model. `Ok` is also
    // acceptable — it would mean the smoke ran fully to completion.)
    if let Err(err) = &result {
        let rendered = format!("{err:?}");
        for forbidden in [
            "getModule",
            "verifyModuleAccess",
            "getClassAccessFlags",
            "isNamed",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "module-system wall regressed: execution failed on {forbidden}: {rendered}"
            );
        }
    }
}
