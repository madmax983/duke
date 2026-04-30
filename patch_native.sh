#!/bin/bash
cat << 'INNER_EOF' > /tmp/native_patch
--- crates/duke-interpreter/src/native.rs
+++ crates/duke-interpreter/src/native.rs
@@ -1,3 +1,11 @@
+//! JVM Native method implementations and JNI bridging.
+//!
+//! This module contains implementations for the `native` methods of standard
+//! library classes (e.g., `java/util/zip/ZipFile`, `java/lang/System`) and handles
+//! the boundary transitions between interpreted JVM bytecode and native Rust code.
+//!
+//! When `invoke_virtual` or `invoke_static` encounters a method marked `ACC_NATIVE`,
+//! control flow routes to the handlers registered here rather than decoding bytecode.

 use std::sync::{RwLock, OnceLock};
 use std::sync::atomic::{AtomicI32, Ordering};
INNER_EOF
patch crates/duke-interpreter/src/native.rs /tmp/native_patch
