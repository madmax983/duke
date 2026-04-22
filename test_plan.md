1. **Identify the Weak Point**: In `crates/duke-loader/src/zip.rs` around line 210, `read_entry_info` parses `uncompressed_size` and immediately allocates a `Vec::with_capacity(cap.min(1024 * 1024 * 32))` without fully validating if the capacity matches the actual compressed payload. If an attacker crafts a zip file with a large uncompressed size but a tiny compressed size, `flate2::read::DeflateDecoder::new(compressed).read_to_end(&mut buf)` will allocate up to 32MB of RAM per entry. While this is bounded by 32MB, an attacker could trigger OOM with a "zip bomb". Alternatively, if the file contains invalid DEFLATE blocks, `read_to_end` will fail, but the 32MB allocation has already occurred.
Wait, 32MB is `1024 * 1024 * 32`. A file with thousands of such entries could exhaust memory quickly. But `with_capacity` is relatively safe.
Instead, let's look at `String::from_utf8(out).unwrap()` in `crates/duke-interpreter/src/tests.rs` or `duke-telemetry/src/lib.rs`.
Actually, Havoc's boundaries mandate creating a test that fails, then writing the *minimal* code to make it pass.

Another idea: `crates/duke-loader/src/directory.rs` has a vulnerability.
```rust
        // Prevent Windows absolute paths and directory traversal
        if name.contains("..")
            || name.contains('.')
            || name.starts_with('/')
            || name.starts_with('\\')
            || name.contains(':')
        {
```
Wait, `name.contains('.')` prevents loading classes with inner classes (e.g. `java/lang/Map$Entry.class`? No, `.` is not `$`). It does prevent `name.contains("..")`, but what if the user asks for `name = "com/example/Main"` and the `root` is `/var/www/html`?

Let's look at `crates/duke-loader/src/zip.rs`:
```rust
        let extra_len = usize::from(read_u16_le(&self.data, offset + 28));
        let data_start = offset
            .checked_add(30)
            .and_then(|v| v.checked_add(filename_len))
            .and_then(|v| v.checked_add(extra_len))
            .ok_or_else(|| LoadError::ZipFormat {
                msg: format!("entry '{}' local header offset overflow", info.name),
            })?;
```

Is there an integer overflow we can trigger?
In `crates/duke-bytecode/src/decoder.rs`:
```rust
fn decode_lookupswitch(c: &mut Cursor<'_>, pc: usize) -> DecodeResult<Instruction> {
...
    let npairs_usize = usize::try_from(npairs).unwrap_or(0);
    let mut pairs = Vec::with_capacity(npairs_usize.min(c.data.len().saturating_sub(c.pos) / 8));
```
Wait, there's `npairs_usize.min(...)`. This bounds the capacity by the remaining bytes.

What about `crates/duke-interpreter/src/native.rs`?
```rust
fn zip_open(path: &std::path::Path) -> VmResult<i32> {
...
    let id = NEXT_ZIP_ID.fetch_add(1, Ordering::Relaxed);
    zip_files().write().unwrap().insert(id, reader);
```
If a zip file is opened repeatedly, the `HashMap` grows infinitely, leading to an OOM! But wait, `zip_close` removes it.

Let's look at `crates/duke-gc/src/host.rs`:
```rust
    pub fn bind_server_socket(&mut self, addr: &str) -> VmResult<i32> {
        let listener = std::net::TcpListener::bind(addr).map_err(...)
```
Wait, `addr` is an `&str` directly from the Java `String` object.

How about `System.setProperty` TOCTOU race condition?
```rust
use loom::sync::Mutex;
```
Wait, in `crates/duke-interpreter/src/native.rs`:
```rust
            let is_self_join = runtime.handles.get(&thread_id).is_some_and(|h| h.thread().id() == std::thread::current().id());
            if is_self_join {
                // Java semantics dictate that a thread joining itself blocks forever.
                // Instead of panicking or returning immediately, we park the thread.
                drop(runtime);
                loop {
                    std::thread::park();
                }
            }
```
If `join_java_thread` is called with `thread_id` equal to the current thread, it parks the thread in a loop! Is that correct? Yes, Java threads joining themselves block forever.

Let's fuzz `crates/duke-bytecode/src/decoder.rs`. Wait, I'm supposed to find *one* weakness and fix it.
