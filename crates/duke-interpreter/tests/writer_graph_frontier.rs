//! Stage-c discovery probe for the real-JDK `OutputStreamWriter`/`StreamEncoder`/
//! `Charset` writer graph, driven WITHOUT `java.io.FileOutputStream`.
//!
//! Mirrors `streamencoder_writer_frontier.rs`, but the sink is a USER-DEFINED
//! `OutputStream` subclass (`WriterGraphProbe$Sink`) wrapping a real
//! `ByteArrayOutputStream`. Under real-JDK shadow the subclass, the
//! `ByteArrayOutputStream`, and the `OutputStreamWriter`/`StreamEncoder`/`Charset`
//! graph are all non-allowlisted real bytecode, so the real writer pipeline runs
//! WITHOUT touching the `FileOutputStream` floor (which is gated behind a forbidden
//! `KEEP_SYNTHETIC` allowlist edit).
//!
//! This is a REGRESSION GUARD: it asserts the honest stage-c writer-graph frontier.
//! The real `OutputStreamWriter -> StreamEncoder -> Charset` pipeline runs clean on a
//! non-allowlisted sink all the way THROUGH the encode: it fills the `CharBuffer`,
//! runs `sun.nio.cs.UTF_8$Encoder.encode`, and encodes+drains all 6 bytes of
//! `"hello\n"` — exercising our three stage-c natives on the LIVE path
//! (`ScopedMemoryAccess.registerNatives` no-op, `Unsafe.isBigEndian`→false via
//! `java/nio/ByteOrder.<clinit>`, and `JavaLangAccess.encodeASCII([CI[BII)I`
//! with `sp=0 dp=0 len=6`). It walls only when `StreamEncoder.write` releases its
//! `ReentrantLock`: `ReentrantLock$Sync.tryRelease@24` throws
//! `java/lang/IllegalMonitorStateException` because the lock's exclusive-owner check
//! (`getExclusiveOwnerThread() != Thread.currentThread()`) fails — Duke's
//! `Thread.currentThread()` allocates a fresh throwaway identity per call, so the
//! thread that acquired the lock is not equal to the thread that releases it.
//!
//! HISTORY: PR #1354 (inherited-static resolution + eager superclass `<clinit>`,
//! JVMS 5.4.3.2 / 5.5) cleared the previous wall at `java/nio/ByteBuffer.<clinit>@16`
//! `getstatic ByteBuffer.UNSAFE` (inherited from superclass `java/nio/Buffer`), which
//! surfaced as `InvalidFieldref { index: 0 }`. Crossing THIS wall needs the deferred
//! wave-9 fix (C): a heap-scoped, stable `Thread.currentThread()` identity — see
//! `docs/findings/2026-07-11-system-io-real-layout-blockers.md` §10.
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

/// Drive the writer-graph probe under real-JDK shadow and return the rendered result
/// (`Ok(())` if it ran to completion, or the `{err:?}` rendering of the first blocker).
fn run_writer_graph_probe_real_jdk_shadow() -> Result<(), String> {
    std::thread::Builder::new()
        .name("writer-graph-probe-driver".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_writer_graph_probe_real_jdk_shadow_inner)
        .expect("spawn probe driver")
        .join()
        .expect("probe driver thread panicked")
}

fn run_writer_graph_probe_real_jdk_shadow_inner() -> Result<(), String> {
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
        .ensure_loaded("WriterGraphProbe", loader.as_ref())
        .expect("load WriterGraphProbe from the fixture classpath");

    let args_ref = heap.allocate("[Ljava/lang/String;".to_string(), 0);
    let main_args = [Slot::Reference(Some(args_ref))];
    let mut output = Vec::new();
    let result = execute_class_to_completion(
        &mut registry,
        ArcLoader(Arc::clone(&loader)),
        &mut heap,
        &mut output,
        "WriterGraphProbe",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    );

    result.map(|_| ()).map_err(|err| format!("{err:?}"))
}

/// REGRESSION GUARD: pin the honest stage-c writer-graph frontier reached WITHOUT
/// `FileOutputStream`. The real `OutputStreamWriter -> StreamEncoder -> Charset` graph
/// runs on a non-allowlisted sink THROUGH the encode of `"hello\n"` (all three stage-c
/// natives exercised live — see the module doc), then walls when `StreamEncoder.write`
/// releases its `ReentrantLock`: `ReentrantLock$Sync.tryRelease@24` throws
/// `java/lang/IllegalMonitorStateException` because `Thread.currentThread()` returns a
/// fresh identity per call, so the lock's exclusive-owner check fails on release. That
/// surfaces as `JavaException { class_name: "java/lang/IllegalMonitorStateException" }`.
///
/// PR #1354 (inherited-static resolution + eager superclass `<clinit>`) cleared the
/// prior `ByteBuffer.UNSAFE` wall (`InvalidFieldref { index: 0 }`); the remaining wall
/// is the deferred wave-9 fix (C): a heap-scoped, stable `Thread.currentThread()`.
///
/// Update `EXPECTED_FRONTIER` whenever the wave-9 `Thread.currentThread()` fix (see the
/// module doc / findings §10) advances the wall — a moved frontier is a real signal,
/// not a flake. The `eprintln!` is retained for diagnostics on failure.
#[test]
fn writer_graph_probe_real_jdk_shadow_frontier() {
    // Real `OutputStreamWriter -> StreamEncoder -> Charset` encodes `"hello\n"`, then
    // walls at `ReentrantLock$Sync.tryRelease@24 athrow` (unbalanced lock owner because
    // `Thread.currentThread()` identity is unstable).
    const EXPECTED_FRONTIER: &str =
        "JavaException { class_name: \"java/lang/IllegalMonitorStateException\" }";

    let rendered = match run_writer_graph_probe_real_jdk_shadow() {
        Ok(()) => {
            eprintln!("WriterGraphProbe ran to completion under real-JDK shadow (no frontier)");
            return;
        }
        Err(err) if err.starts_with("SKIP:") => {
            eprintln!("skipping: {err}");
            return;
        }
        Err(rendered) => rendered,
    };

    eprintln!("=== VERBATIM WRITER-GRAPH FRONTIER ===\n{rendered}\n=== END ===");
    assert!(
        rendered.contains(EXPECTED_FRONTIER),
        "Writer-graph frontier moved.\nExpected to contain: {EXPECTED_FRONTIER}\nActual: {rendered}"
    );
}
