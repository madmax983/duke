# Phase 3: Bootstrap Class Loader Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement `duke-loader` crate providing a bootstrap class loader that loads raw `.class` bytes from directory classpaths and OpenJDK 21's `lib/modules` jimage binary.

**Architecture:** A `ClassLoader` trait with `find_class(name) -> Result<Vec<u8>>`. Two implementations: `DirectoryLoader` for filesystem directories and `JImageReader` for the jimage binary format. `BootstrapLoader` composes both. Class names are internal-form slashes: `"java/lang/Object"`.

**Tech Stack:** Rust, `thiserror` (workspace), `flate2 = "1"` for jimage deflate decompression, `duke-classfile` (dev dep for integration tests).

**JDK Location:** `C:\Users\markm\Downloads\java-21-openjdk-21.0.4.0.7-1.win.jdk.x86_64\java-21-openjdk-21.0.4.0.7-1.win.jdk.x86_64\lib\modules`

**JImage Format (confirmed):**
- Header: 28 bytes LE — magic=0xCAFEDADA, version=0x00010000, flags, resource_count, table_length, locations_size, strings_size
- Index: redirect[i32; table_length], offsets[u32; table_length], locations[u8; locations_size], strings[u8; strings_size]
- Data section: starts at `28 + table_length*8 + locations_size + strings_size`
- Location attribute header byte: `(kind << 3) | (data_length - 1)`; data is big-endian
- Attribute kinds: 0=END, 1=MODULE, 2=PARENT, 3=BASE, 4=EXTENSION, 5=OFFSET, 6=COMPRESSED, 7=UNCOMPRESSED
- Strings: null-terminated, string table index 0 = empty string
- Path format for lookup: `/module/parent/base.extension` e.g. `/java.base/java/lang/Object.class`
- Data may be raw OR deflate-compressed (raw deflate, no zlib/gzip header); COMPRESSED field = 0 means uncompressed

---

### Task 1: Crate scaffolding and error type

**Files:**
- Create: `crates/duke-loader/Cargo.toml`
- Create: `crates/duke-loader/src/lib.rs`
- Create: `crates/duke-loader/src/error.rs`
- Modify: `Cargo.toml` (workspace root — add member and flate2 dep)

**Step 1: Add to workspace Cargo.toml**

In `Cargo.toml` (workspace root), add to `[workspace]` members:
```
"crates/duke-loader",
```
And to `[workspace.dependencies]`:
```toml
flate2 = "1"
```

**Step 2: Create `crates/duke-loader/Cargo.toml`**

```toml
[package]
name = "duke-loader"
version.workspace = true
edition.workspace = true
description = "Duke JVM - Bootstrap class loader (classpath + jimage)"

[dependencies]
duke-classfile = { path = "../duke-classfile" }
thiserror.workspace = true
flate2.workspace = true

[dev-dependencies]
duke-classfile = { path = "../duke-classfile" }
```

**Step 3: Create `crates/duke-loader/src/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("class not found: {name}")]
    NotFound { name: String },

    #[error("I/O error reading '{path}': {source}")]
    Io { path: String, #[source] source: std::io::Error },

    #[error("jimage format error: {msg}")]
    JImageFormat { msg: String },

    #[error("jimage decompression error for '{name}'")]
    Decompress { name: String },
}

pub type LoadResult<T> = Result<T, LoadError>;
```

**Step 4: Create `crates/duke-loader/src/lib.rs`**

```rust
pub mod error;

pub use error::{LoadError, LoadResult};

/// Abstraction over class file loading sources.
///
/// `name` is internal form: `"java/lang/Object"` (no `.class` suffix).
pub trait ClassLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>>;
}
```

**Step 5: Write the failing test**

At the bottom of `lib.rs`, add:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trait_is_object_safe() {
        // Verify ClassLoader can be used as a trait object.
        let _: Option<Box<dyn ClassLoader>> = None;
    }
}
```

**Step 6: Run test to verify it compiles and passes**

Run: `cargo test -p duke-loader`
Expected: PASS (1 test)

**Step 7: Commit**

```bash
git add crates/duke-loader/ Cargo.toml Cargo.lock
git commit -m "feat(loader): scaffold duke-loader crate with ClassLoader trait and error types"
```

---

### Task 2: DirectoryLoader

**Files:**
- Create: `crates/duke-loader/src/directory.rs`
- Modify: `crates/duke-loader/src/lib.rs`

**Step 1: Write failing tests first** (add to `lib.rs` test module)

```rust
#[test]
fn directory_loader_finds_hello_world() {
    use crate::directory::DirectoryLoader;
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");
    let loader = DirectoryLoader::new(fixtures);
    let bytes = loader.find_class("HelloWorld").expect("should find HelloWorld");
    assert!(!bytes.is_empty());
    // Verify it's a valid class file (magic bytes)
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
}

#[test]
fn directory_loader_returns_not_found() {
    use crate::directory::DirectoryLoader;
    let loader = DirectoryLoader::new("/nonexistent/path");
    let err = loader.find_class("NoSuchClass").unwrap_err();
    assert!(matches!(err, LoadError::NotFound { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p duke-loader`
Expected: FAIL — "directory" module not found

**Step 3: Create `crates/duke-loader/src/directory.rs`**

```rust
use std::path::{Path, PathBuf};

use crate::{ClassLoader, LoadError, LoadResult};

/// Loads `.class` files from a filesystem directory.
///
/// Given `find_class("java/lang/Object")`, looks for
/// `{root}/java/lang/Object.class`.
pub struct DirectoryLoader {
    root: PathBuf,
}

impl DirectoryLoader {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self { root: root.as_ref().to_path_buf() }
    }
}

impl ClassLoader for DirectoryLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        let mut path = self.root.clone();
        // name is "java/lang/Object" — convert to OS path + ".class"
        for component in name.split('/') {
            path.push(component);
        }
        path.set_extension("class");

        std::fs::read(&path).map_err(|_| LoadError::NotFound {
            name: name.to_string(),
        })
    }
}
```

**Step 4: Add to `lib.rs`**

```rust
pub mod directory;
pub use directory::DirectoryLoader;
```

**Step 5: Run tests to verify they pass**

Run: `cargo test -p duke-loader`
Expected: PASS (3 tests)

**Step 6: Commit**

```bash
git add crates/duke-loader/src/
git commit -m "feat(loader): add DirectoryLoader for classpath directory loading"
```

---

### Task 3: JImage header parsing

**Files:**
- Create: `crates/duke-loader/src/jimage.rs`
- Modify: `crates/duke-loader/src/lib.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn jimage_reader_opens_modules_file() {
    use crate::jimage::JImageReader;
    let path = jdk_modules_path();
    if !path.exists() {
        eprintln!("skipping — JDK modules not found at {path:?}");
        return;
    }
    let reader = JImageReader::open(&path).expect("should open jimage");
    assert!(reader.resource_count() > 0, "should have resources");
}

fn jdk_modules_path() -> std::path::PathBuf {
    std::path::PathBuf::from(
        r"C:\Users\markm\Downloads\java-21-openjdk-21.0.4.0.7-1.win.jdk.x86_64\java-21-openjdk-21.0.4.0.7-1.win.jdk.x86_64\lib\modules"
    )
}
```

**Step 2: Run test to verify it fails**

Expected: FAIL — "jimage" module not found

**Step 3: Create `crates/duke-loader/src/jimage.rs`** — header parsing only

```rust
use std::path::Path;

use crate::{LoadError, LoadResult};

const JIMAGE_MAGIC: u32 = 0xCAFE_DADA;
const JIMAGE_VERSION: u32 = 0x0001_0000;
const HEADER_SIZE: usize = 28;

#[derive(Debug, Clone)]
struct Header {
    resource_count: u32,
    table_length: u32,
    locations_size: u32,
    strings_size: u32,
}

/// Reader for OpenJDK 21 jimage format (`lib/modules`).
pub struct JImageReader {
    data: Vec<u8>,
    header: Header,
    // Pre-computed section offsets
    redirect_offset: usize,   // = HEADER_SIZE
    offsets_offset: usize,    // = HEADER_SIZE + table_length * 4
    locations_offset: usize,  // = HEADER_SIZE + table_length * 8
    strings_offset: usize,    // = HEADER_SIZE + table_length * 8 + locations_size
    data_offset: usize,       // = strings_offset + strings_size
}

impl JImageReader {
    pub fn open(path: &Path) -> LoadResult<Self> {
        let data = std::fs::read(path).map_err(|e| LoadError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

        let header = parse_header(&data)?;

        let tl = header.table_length as usize;
        let ls = header.locations_size as usize;
        let ss = header.strings_size as usize;

        let redirect_offset = HEADER_SIZE;
        let offsets_offset = redirect_offset + tl * 4;
        let locations_offset = offsets_offset + tl * 4;
        let strings_offset = locations_offset + ls;
        let data_offset = strings_offset + ss;

        if data.len() < data_offset {
            return Err(LoadError::JImageFormat {
                msg: format!(
                    "file too small: {} < required {}",
                    data.len(), data_offset
                ),
            });
        }

        Ok(Self {
            data,
            header,
            redirect_offset,
            offsets_offset,
            locations_offset,
            strings_offset,
            data_offset,
        })
    }

    pub fn resource_count(&self) -> u32 {
        self.header.resource_count
    }
}

fn parse_header(data: &[u8]) -> LoadResult<Header> {
    if data.len() < HEADER_SIZE {
        return Err(LoadError::JImageFormat {
            msg: format!("file too small for header: {} bytes", data.len()),
        });
    }

    let magic = read_u32_le(data, 0);
    if magic != JIMAGE_MAGIC {
        return Err(LoadError::JImageFormat {
            msg: format!("bad magic: {magic:#010x} (expected {JIMAGE_MAGIC:#010x})"),
        });
    }

    let version = read_u32_le(data, 4);
    if version != JIMAGE_VERSION {
        return Err(LoadError::JImageFormat {
            msg: format!("unsupported version: {version:#010x}"),
        });
    }

    Ok(Header {
        resource_count: read_u32_le(data, 12),
        table_length: read_u32_le(data, 16),
        locations_size: read_u32_le(data, 20),
        strings_size: read_u32_le(data, 24),
    })
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
```

**Step 4: Add to `lib.rs`**

```rust
pub mod jimage;
pub use jimage::JImageReader;
```

**Step 5: Run test**

Run: `cargo test -p duke-loader -- jimage`
Expected: PASS (or skip if JDK not found)

**Step 6: Commit**

```bash
git add crates/duke-loader/src/jimage.rs crates/duke-loader/src/lib.rs
git commit -m "feat(loader): add JImageReader with header parsing"
```

---

### Task 4: JImage location scanning and string resolution

This implements the O(n) index build: scan all location entries, decode their attributes, build a `HashMap<String, ResourceInfo>` keyed by jimage path.

**Files:** Modify `crates/duke-loader/src/jimage.rs`

**Step 1: Add the test**

```rust
#[test]
fn jimage_can_find_object_class_location() {
    use crate::jimage::JImageReader;
    let path = jdk_modules_path();
    if !path.exists() { return; }
    let reader = JImageReader::open(&path).expect("open");
    let info = reader.find_resource("/java.base/java/lang/Object.class")
        .expect("Object.class must exist in java.base");
    assert!(info.uncompressed > 0);
}
```

**Step 2: Run to verify it fails** — `find_resource` doesn't exist yet.

**Step 3: Implement attribute decoding and index building**

Add to `jimage.rs`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub offset: u64,        // offset into data section
    pub compressed: u64,    // 0 means not compressed
    pub uncompressed: u64,  // actual (decompressed) size in bytes
}

// Attribute kind constants
const ATTR_END: u8 = 0;
const ATTR_MODULE: u8 = 1;
const ATTR_PARENT: u8 = 2;
const ATTR_BASE: u8 = 3;
const ATTR_EXTENSION: u8 = 4;
const ATTR_OFFSET: u8 = 5;
const ATTR_COMPRESSED: u8 = 6;
const ATTR_UNCOMPRESSED: u8 = 7;
```

Add `index: HashMap<String, ResourceInfo>` field to `JImageReader`, and populate it from `build_index()` called in `open()`.

```rust
fn build_index(data: &[u8], locs_offset: usize, locs_size: usize,
               str_offset: usize) -> HashMap<String, ResourceInfo> {
    let mut index = HashMap::new();
    let mut pos = locs_offset;
    let locs_end = locs_offset + locs_size;

    while pos < locs_end {
        let start = pos;
        // Decode all attributes for this location
        let mut module: u64 = 0;
        let mut parent: u64 = 0;
        let mut base: u64 = 0;
        let mut extension: u64 = 0;
        let mut offset: u64 = 0;
        let mut compressed: u64 = 0;
        let mut uncompressed: u64 = 0;

        loop {
            if pos >= locs_end { break; }
            let hdr = data[pos];
            pos += 1;
            let kind = hdr >> 3;
            let len = ((hdr & 7) + 1) as usize;
            if kind == ATTR_END { break; }
            if pos + len > data.len() { break; }

            let val = read_be_u64(&data[pos..pos + len]);
            pos += len;

            match kind {
                ATTR_MODULE     => module = val,
                ATTR_PARENT     => parent = val,
                ATTR_BASE       => base = val,
                ATTR_EXTENSION  => extension = val,
                ATTR_OFFSET     => offset = val,
                ATTR_COMPRESSED => compressed = val,
                ATTR_UNCOMPRESSED => uncompressed = val,
                _ => {}  // unknown, skip
            }
        }

        if base == 0 && uncompressed == 0 { continue; }  // empty/padding slot

        // Build path from string table
        let mod_str = read_str(data, str_offset, module);
        let par_str = read_str(data, str_offset, parent);
        let base_str = read_str(data, str_offset, base);
        let ext_str = read_str(data, str_offset, extension);

        let path = build_path(mod_str, par_str, base_str, ext_str);
        if !path.is_empty() {
            index.insert(path, ResourceInfo { offset, compressed, uncompressed });
        }

        let _ = start; // suppress unused
    }

    index
}

fn read_be_u64(bytes: &[u8]) -> u64 {
    let mut val = 0u64;
    for &b in bytes {
        val = (val << 8) | b as u64;
    }
    val
}

fn read_str(data: &[u8], str_offset: usize, idx: u64) -> &str {
    let start = str_offset + idx as usize;
    if start >= data.len() { return ""; }
    let end = data[start..].iter().position(|&b| b == 0)
        .map(|p| start + p)
        .unwrap_or(data.len());
    std::str::from_utf8(&data[start..end]).unwrap_or("")
}

fn build_path(module: &str, parent: &str, base: &str, ext: &str) -> String {
    if module.is_empty() || base.is_empty() { return String::new(); }
    let mut path = format!("/{module}/");
    if !parent.is_empty() {
        path.push_str(parent);
        path.push('/');
    }
    path.push_str(base);
    if !ext.is_empty() {
        path.push('.');
        path.push_str(ext);
    }
    path
}
```

Add `find_resource` method:
```rust
pub fn find_resource(&self, path: &str) -> Option<&ResourceInfo> {
    self.index.get(path)
}
```

**Step 4: Run test**

Run: `cargo test -p duke-loader -- jimage`
Expected: PASS

**Step 5: Commit**

```bash
git add crates/duke-loader/src/jimage.rs
git commit -m "feat(loader): scan jimage locations table to build class index"
```

---

### Task 5: JImage data reading with decompression

**Files:** Modify `crates/duke-loader/src/jimage.rs`

**Step 1: Write the test**

```rust
#[test]
fn jimage_reads_object_class_bytes() {
    use crate::jimage::JImageReader;
    let path = jdk_modules_path();
    if !path.exists() { return; }
    let reader = JImageReader::open(&path).expect("open");
    let bytes = reader.read_class("/java.base/java/lang/Object.class")
        .expect("read Object.class");
    // Must be a valid class file
    assert_eq!(&bytes[..4], &[0xCA, 0xFE, 0xBA, 0xBE], "bad magic");
    assert_eq!(bytes.len(), 2487, "Object.class should be 2487 bytes (JDK 21.0.4)");
}
```

**Step 2: Run to verify it fails**

**Step 3: Implement `read_class`**

Add to `jimage.rs`:
```rust
use flate2::read::DeflateDecoder;
use std::io::Read;

impl JImageReader {
    pub fn read_class(&self, path: &str) -> LoadResult<Vec<u8>> {
        let info = self.index.get(path).ok_or_else(|| LoadError::NotFound {
            name: path.to_string(),
        })?;

        let start = self.data_offset + info.offset as usize;
        let (raw_len, decomp_len) = if info.compressed > 0 {
            (info.compressed as usize, info.uncompressed as usize)
        } else {
            (info.uncompressed as usize, 0)
        };

        if start + raw_len > self.data.len() {
            return Err(LoadError::JImageFormat {
                msg: format!("resource data out of bounds for {path}"),
            });
        }

        let raw = &self.data[start..start + raw_len];

        if info.compressed > 0 {
            // Raw deflate (no zlib header, negative window bits)
            let mut decoder = DeflateDecoder::new(raw);
            let mut out = Vec::with_capacity(decomp_len);
            decoder.read_to_end(&mut out).map_err(|_| LoadError::Decompress {
                name: path.to_string(),
            })?;
            Ok(out)
        } else {
            Ok(raw.to_vec())
        }
    }
}
```

**Step 4: Add `flate2` to workspace Cargo.toml** (if not already done in Task 1)

**Step 5: Run test**

Run: `cargo test -p duke-loader -- jimage`
Expected: PASS

**Step 6: Commit**

```bash
git add crates/duke-loader/src/jimage.rs Cargo.toml Cargo.lock
git commit -m "feat(loader): read and decompress jimage resources"
```

---

### Task 6: JImageLoader ClassLoader impl and BootstrapLoader

**Files:**
- Modify: `crates/duke-loader/src/jimage.rs`
- Create: `crates/duke-loader/src/bootstrap.rs`
- Modify: `crates/duke-loader/src/lib.rs`

**Step 1: Write the integration test**

```rust
#[test]
fn bootstrap_loader_loads_from_jimage_and_classpath() {
    use crate::{bootstrap::BootstrapLoader, ClassLoader};
    let jdk_modules = jdk_modules_path();
    if !jdk_modules.exists() { return; }
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");

    let loader = BootstrapLoader::new(&jdk_modules, vec![fixtures])
        .expect("create bootstrap loader");

    // Load from jimage
    let obj = loader.find_class("java/lang/Object").expect("Object from jimage");
    assert_eq!(&obj[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);

    // Load from classpath
    let hw = loader.find_class("HelloWorld").expect("HelloWorld from classpath");
    assert_eq!(&hw[..4], &[0xCA, 0xFE, 0xBA, 0xBE]);
}
```

**Step 2: Implement ClassLoader for JImageLoader**

Add to `jimage.rs`:
```rust
impl crate::ClassLoader for JImageReader {
    fn find_class(&self, name: &str) -> crate::LoadResult<Vec<u8>> {
        // Try common modules in order
        let candidate_modules = ["java.base", "java.logging", "java.sql",
                                  "java.desktop", "java.management"];
        let (parent, base) = split_class_name(name);
        for module in candidate_modules {
            let path = format!("/{module}/{parent}{base}.class");
            if self.index.contains_key(&path) {
                return self.read_class(&path);
            }
        }
        Err(crate::LoadError::NotFound { name: name.to_string() })
    }
}

fn split_class_name(name: &str) -> (&str, &str) {
    match name.rfind('/') {
        Some(pos) => (&name[..pos + 1], &name[pos + 1..]),
        None => ("", name),
    }
}
```

**Step 3: Create `crates/duke-loader/src/bootstrap.rs`**

```rust
use std::path::Path;

use crate::{ClassLoader, DirectoryLoader, JImageReader, LoadError, LoadResult};

/// Bootstrap class loader: tries jimage first, then classpath directories.
pub struct BootstrapLoader {
    jimage: JImageReader,
    classpath: Vec<DirectoryLoader>,
}

impl BootstrapLoader {
    pub fn new(
        modules_path: &Path,
        classpath_dirs: Vec<impl AsRef<Path>>,
    ) -> LoadResult<Self> {
        let jimage = JImageReader::open(modules_path)?;
        let classpath = classpath_dirs
            .into_iter()
            .map(|p| DirectoryLoader::new(p))
            .collect();
        Ok(Self { jimage, classpath })
    }
}

impl ClassLoader for BootstrapLoader {
    fn find_class(&self, name: &str) -> LoadResult<Vec<u8>> {
        // Standard library classes come from jimage
        if let Ok(bytes) = self.jimage.find_class(name) {
            return Ok(bytes);
        }
        // Application classes come from classpath
        for loader in &self.classpath {
            if let Ok(bytes) = loader.find_class(name) {
                return Ok(bytes);
            }
        }
        Err(LoadError::NotFound { name: name.to_string() })
    }
}
```

**Step 4: Add to `lib.rs`**

```rust
pub mod bootstrap;
pub use bootstrap::BootstrapLoader;
```

**Step 5: Run all tests**

Run: `cargo test -p duke-loader`
Expected: all pass

**Step 6: Commit**

```bash
git add crates/duke-loader/src/
git commit -m "feat(loader): add BootstrapLoader composing jimage and classpath loading"
```

---

### Task 7: Integration test — parse loaded classes with duke-classfile

**Files:** Modify `crates/duke-loader/src/lib.rs`

**Step 1: Write the test**

```rust
#[test]
fn loaded_object_class_parses_correctly() {
    use crate::{bootstrap::BootstrapLoader, ClassLoader};
    use duke_classfile::parse;
    let jdk_modules = jdk_modules_path();
    if !jdk_modules.exists() { return; }

    let loader = BootstrapLoader::new(&jdk_modules, vec![] as Vec<std::path::PathBuf>)
        .expect("create loader");
    let bytes = loader.find_class("java/lang/Object").expect("load Object");
    let cf = parse(&bytes).expect("parse Object.class");

    assert_eq!(cf.major_version, 65, "JDK 21 uses class version 65");
    // Object has no superclass
    assert_eq!(cf.super_class.0, 0, "java.lang.Object has no super");
}

#[test]
fn loaded_hello_world_parses_correctly() {
    use crate::{ClassLoader, DirectoryLoader};
    use duke_classfile::parse;
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures");
    let loader = DirectoryLoader::new(fixtures);
    let bytes = loader.find_class("HelloWorld").expect("load HelloWorld");
    let cf = parse(&bytes).expect("parse HelloWorld.class");
    assert_eq!(cf.major_version, 65);
}
```

**Step 2: Run test**

Run: `cargo test -p duke-loader`
Expected: PASS

**Step 3: Run full workspace test suite**

Run: `cargo test`
Expected: all tests pass, no warnings

**Step 4: Clippy**

Run: `cargo clippy -p duke-loader -- -W clippy::pedantic`
Fix any warnings.

**Step 5: Commit**

```bash
git add crates/duke-loader/src/lib.rs
git commit -m "test(loader): verify loaded class bytes parse correctly with duke-classfile"
```

---

### Task 8: Update workspace and binary, update memory

**Files:**
- Modify: `duke/src/main.rs` — add `load` subcommand
- Modify: workspace `Cargo.toml` — add duke-loader to duke binary
- Update: `MEMORY.md`

**Step 1: Add duke-loader dep to duke binary**

In `duke/Cargo.toml`, add:
```toml
duke-loader = { path = "../crates/duke-loader" }
```

**Step 2: Add `load` subcommand to `duke/src/main.rs`**

```rust
"load" => {
    use duke_loader::{ClassLoader, DirectoryLoader};
    let loader = DirectoryLoader::new(".");
    let bytes = loader.find_class(path).unwrap_or_else(|e| {
        eprintln!("duke: {e}");
        process::exit(1);
    });
    let cf = parse(&bytes).unwrap_or_else(|e| {
        eprintln!("duke: parse error: {e}");
        process::exit(1);
    });
    dump_class_file(&cf);
}
```

**Step 3: Run full test suite**

Run: `cargo test && cargo clippy`
Expected: all pass

**Step 4: Commit**

```bash
git add duke/ crates/duke-loader/ Cargo.toml Cargo.lock
git commit -m "feat: add 'duke load' subcommand using DirectoryLoader, complete Phase 3"
```

---

## Verification

Phase 3 is complete when:

1. `cargo test` — all tests pass across all crates (duke-classfile, duke-bytecode, duke-loader)
2. `cargo clippy -p duke-loader -- -W clippy::pedantic` — no warnings
3. `duke dump tests/fixtures/HelloWorld.class` — works as before (Phase 1/2 regression)
4. `duke load HelloWorld` — loads and dumps HelloWorld from cwd

**Total new tests in duke-loader:** ~8 unit/integration tests
