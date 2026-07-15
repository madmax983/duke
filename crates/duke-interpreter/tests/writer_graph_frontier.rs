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
//! This is an END-TO-END MILESTONE assertion: the real
//! `OutputStreamWriter -> StreamEncoder -> Charset` pipeline runs to COMPLETION on a
//! non-allowlisted sink. It fills the `CharBuffer`, runs `sun.nio.cs.UTF_8$Encoder.encode`,
//! encodes+drains all 6 bytes of `"hello\n"` — exercising our three stage-c natives on
//! the LIVE path (`ScopedMemoryAccess.registerNatives` no-op, `Unsafe.isBigEndian`→false
//! via `java/nio/ByteOrder.<clinit>`, and `JavaLangAccess.encodeASCII([CI[BII)I` with
//! `sp=0 dp=0 len=6`), releases the `StreamEncoder` `ReentrantLock`, and returns from
//! `main`. The test reads the sink bytes back off `WriterGraphProbe.BYTES` and asserts
//! they are the exact UTF-8 encoding of `"hello\n"`.
//!
//! HISTORY: this was formerly a frontier GUARD pinned at `StreamEncoder.write`'s
//! `ReentrantLock` release — `ReentrantLock$Sync.tryRelease@24` threw
//! `java/lang/IllegalMonitorStateException` because the exclusive-owner check
//! (`getExclusiveOwnerThread() == Thread.currentThread()`) failed: Duke's
//! `Thread.currentThread()` allocated a fresh throwaway identity per call, so the thread
//! that acquired the lock was not equal to the thread that released it. Wave-9 fix (C) —
//! a heap-scoped, GC-rooted, STABLE `Thread.currentThread()` (seeded once at bootstrap
//! into a static field on `java/lang/Thread`) — balances the owner check and clears the
//! wall. PR #1354 (inherited-static resolution + eager superclass `<clinit>`, JVMS
//! 5.4.3.2 / 5.5) had earlier cleared the prior wall at `java/nio/ByteBuffer.<clinit>@16`
//! `getstatic ByteBuffer.UNSAFE` (`InvalidFieldref { index: 0 }`). See
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

/// Outcome of driving the writer-graph probe to completion: the exact bytes drained
/// into the sink (`WriterGraphProbe.BYTES`) and the reported count
/// (`WriterGraphProbe.RESULT`).
struct ProbeCompletion {
    sink_bytes: Vec<u8>,
    reported_size: i32,
}

/// Drive the writer-graph probe under real-JDK shadow. Returns the completion
/// (sink bytes + reported size) if `main` returned, or the `{err:?}` rendering of
/// the first blocker.
fn run_writer_graph_probe_real_jdk_shadow() -> Result<ProbeCompletion, String> {
    std::thread::Builder::new()
        .name("writer-graph-probe-driver".to_string())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_writer_graph_probe_real_jdk_shadow_inner)
        .expect("spawn probe driver")
        .join()
        .expect("probe driver thread panicked")
}

fn run_writer_graph_probe_real_jdk_shadow_inner() -> Result<ProbeCompletion, String> {
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
    execute_class_to_completion(
        &mut registry,
        ArcLoader(Arc::clone(&loader)),
        &mut heap,
        &mut output,
        "WriterGraphProbe",
        "main",
        "([Ljava/lang/String;)V",
        &main_args,
    )
    .map_err(|err| format!("{err:?}"))?;

    // `main` returned. Read the two success markers back off the static fields.
    let ctx = registry
        .get("WriterGraphProbe")
        .expect("WriterGraphProbe registered after execution");
    let static_slot = |field: &str| -> Slot {
        let idx = ctx
            .fields
            .iter()
            .filter(|f| f.is_static)
            .position(|f| f.name == field)
            .unwrap_or_else(|| panic!("static field {field} present on WriterGraphProbe"));
        ctx.static_fields[idx]
    };

    let reported_size = match static_slot("RESULT") {
        Slot::Int(n) => n,
        other => panic!("WriterGraphProbe.RESULT is not an int: {other:?}"),
    };
    let bytes_ref = match static_slot("BYTES") {
        Slot::Reference(Some(r)) => r,
        other => panic!("WriterGraphProbe.BYTES is not a live array reference: {other:?}"),
    };
    let array = heap
        .get(bytes_ref)
        .expect("WriterGraphProbe.BYTES points to a live heap array");
    let sink_bytes = array
        .fields
        .iter()
        .map(|slot| match slot {
            // A `byte[]` element is a sign-extended i32; `as u8` takes the correct
            // two's-complement low byte (values here are all positive ASCII anyway).
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Slot::Int(byte) => *byte as u8,
            other => panic!("byte[] element is not an int: {other:?}"),
        })
        .collect();

    Ok(ProbeCompletion {
        sink_bytes,
        reported_size,
    })
}

/// END-TO-END MILESTONE: the real `OutputStreamWriter -> StreamEncoder -> Charset`
/// writer graph runs to COMPLETION on a non-allowlisted sink, WITHOUT
/// `FileOutputStream`. It fills the `CharBuffer`, runs `sun.nio.cs.UTF_8$Encoder`,
/// encodes+drains all 6 bytes of `"hello\n"` (exercising the three stage-c natives on
/// the live path — see the module doc), releases the `StreamEncoder` `ReentrantLock`,
/// and `main` returns. The exact bytes drained into the sink must be the UTF-8 encoding
/// of `"hello\n"`: `[104, 101, 108, 108, 111, 10]`.
///
/// HISTORY: this was formerly a frontier GUARD pinned at
/// `ReentrantLock$Sync.tryRelease@24 athrow` →
/// `JavaException { class_name: "java/lang/IllegalMonitorStateException" }`, because
/// `Thread.currentThread()` allocated a fresh identity per call and the lock's
/// exclusive-owner check (`getExclusiveOwnerThread() == currentThread`) failed on
/// release. Wave-9 fix (C) — a heap-scoped, GC-rooted, STABLE `Thread.currentThread()`
/// seeded once at bootstrap into a static field on `java/lang/Thread` — balances the
/// owner check and clears the wall. PR #1354 had earlier cleared the prior
/// `ByteBuffer.UNSAFE` wall (`InvalidFieldref { index: 0 }`).
///
/// Requires a real JDK jimage (`lib/modules`); without one it skips.
#[test]
fn writer_graph_probe_real_jdk_shadow_completes_with_hello_bytes() {
    // UTF-8 bytes of "hello\n".
    const EXPECTED_BYTES: [u8; 6] = [104, 101, 108, 108, 111, 10];

    let completion = match run_writer_graph_probe_real_jdk_shadow() {
        Ok(completion) => completion,
        Err(err) if err.starts_with("SKIP:") => {
            eprintln!("skipping: {err}");
            return;
        }
        Err(rendered) => {
            panic!(
                "Writer graph regressed: expected completion with sink bytes {EXPECTED_BYTES:?}, \
                 but hit a blocker: {rendered}"
            );
        }
    };

    assert_eq!(
        completion.sink_bytes, EXPECTED_BYTES,
        "sink did not receive the exact UTF-8 bytes of \"hello\\n\""
    );
    assert_eq!(
        completion.reported_size,
        i32::try_from(EXPECTED_BYTES.len()).unwrap(),
        "WriterGraphProbe.RESULT (sink size) should equal the number of encoded bytes"
    );
}
