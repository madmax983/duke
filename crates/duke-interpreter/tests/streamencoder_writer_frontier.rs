//! Stage-c discovery probe for the real-JDK `StreamEncoder`/`Charset` writer wall.
//!
//! Mirrors `classloader_bootstrap_frontier.rs`: builds a jimage-backed loader,
//! enables real-JDK shadow (`DUKE_REAL_JDK=1` in-process), loads the
//! `StreamEncoderWriterProbe` fixture, and drives its `main` to completion. The
//! fixture constructs `new OutputStreamWriter(new FileOutputStream(FileDescriptor.out),
//! "UTF-8")`, writes `"hello\n"`, and flushes — exercising the real
//! `OutputStreamWriter`/`StreamEncoder`/`Charset` graph one level above the landed
//! `FileOutputStream` floor.
//!
//! This is a REGRESSION GUARD for a DIFFERENT path than `writer_graph_frontier.rs`:
//! its sink is a real `FileOutputStream(FileDescriptor.out)`, which is walled behind
//! the `KEEP_SYNTHETIC` allowlist. Both the CLI (`DUKE_REAL_JDK=1`) and in-test
//! (`enable_real_jdk_shadow`) paths share `should_shadow_synthetic`/`KEEP_SYNTHETIC`,
//! so `FileOutputStream` stays synthetic and its real FD-constructor
//! `<init>(Ljava/io/FileDescriptor;)V` is never loaded — surfacing as `MethodNotFound`.
//! Dropping `FileOutputStream` from `KEEP_SYNTHETIC` is a forbidden allowlist edit;
//! see `docs/findings/2026-07-11-system-io-real-layout-blockers.md` §9–§10.
//!
//! Kept alongside `writer_graph_frontier.rs` (not folded away) because it guards a
//! genuinely distinct wall — the allowlist FD-constructor gate, versus that probe's
//! `ByteBuffer.UNSAFE` inherited-static gate.
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

/// Drive the writer probe under real-JDK shadow and return the rendered result
/// (`Ok(())` if it ran to completion, or the `{err:?}` rendering of the first blocker).
fn run_writer_probe_real_jdk_shadow() -> Result<(), String> {
    std::thread::Builder::new()
        .name("stream-encoder-probe-driver".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_writer_probe_real_jdk_shadow_inner)
        .expect("spawn probe driver")
        .join()
        .expect("probe driver thread panicked")
}

fn run_writer_probe_real_jdk_shadow_inner() -> Result<(), String> {
    let modules = jdk_modules_path().ok_or_else(|| "SKIP: no JDK jimage".to_string())?;

    let fixtures = repo_root().join("tests/fixtures");
    let classpath = vec![fixtures];
    let loader: Arc<dyn ClassLoader + Send + Sync> =
        Arc::new(BootstrapLoader::new(&modules, classpath).expect("build jimage+classpath loader"));

    let mut registry = ClassRegistry::new();
    let mut heap = Heap::new();
    registry.enable_real_jdk_shadow(Arc::clone(&loader));
    bootstrap_stdlib(&mut registry, &mut heap);

    registry
        .ensure_loaded("StreamEncoderWriterProbe", loader.as_ref())
        .expect("load StreamEncoderWriterProbe from the fixture classpath");

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = [Slot::Reference(Some(args_ref))];
    let mut output = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        ArcLoader(Arc::clone(&loader)),
        &mut heap,
        &mut output,
        "StreamEncoderWriterProbe",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    );

    if result.is_ok() {
        eprintln!(
            "=== PROBE STDOUT SINK ===\n{}\n=== END SINK ===",
            String::from_utf8_lossy(&output)
        );
    }
    result.map(|_| ()).map_err(|err| format!("{err:?}"))
}

/// REGRESSION GUARD: pin the `FileOutputStream` FD-constructor wall. Because
/// `FileOutputStream` stays on `KEEP_SYNTHETIC`, its real
/// `<init>(Ljava/io/FileDescriptor;)V` is never loaded and the probe walls with
/// `MethodNotFound` for that constructor — a distinct path from the sinkless
/// `writer_graph_frontier.rs` (which walls at `ByteBuffer.UNSAFE`).
///
/// Update `EXPECTED_FRONTIER` if the `KEEP_SYNTHETIC` allowlist edit lands and this
/// path advances. The `eprintln!` is retained for diagnostics on failure.
#[test]
fn stream_encoder_writer_probe_real_jdk_shadow_frontier() {
    // `FileOutputStream` is allowlisted synthetic, so its real FD constructor is
    // never loaded: `new FileOutputStream(FileDescriptor.out)` cannot resolve.
    const EXPECTED_FRONTIER: &str = r#"MethodNotFound { name: "java/io/FileOutputStream.<init>", descriptor: "(Ljava/io/FileDescriptor;)V" }"#;

    let rendered = match run_writer_probe_real_jdk_shadow() {
        Ok(()) => {
            eprintln!(
                "StreamEncoderWriterProbe ran to completion under real-JDK shadow (no frontier)"
            );
            return;
        }
        Err(err) if err.starts_with("SKIP:") => {
            eprintln!("skipping: {err}");
            return;
        }
        Err(rendered) => rendered,
    };

    eprintln!("=== VERBATIM STREAMENCODER WRITER FRONTIER ===\n{rendered}\n=== END ===");
    assert!(
        rendered.contains(EXPECTED_FRONTIER),
        "StreamEncoder writer frontier moved.\nExpected to contain: {EXPECTED_FRONTIER}\nActual: {rendered}"
    );
}
