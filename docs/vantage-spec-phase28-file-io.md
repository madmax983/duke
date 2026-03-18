# 🔭 Vantage: Spec for Basic File I/O (`java.io.File`, `FileInputStream`, `FileOutputStream`)

## 👤 User Story
"As a Backend Developer running on Duke, I want to read from and write to local files using standard Java I/O streams, so that my applications can process external data, persist state, and interact with the host operating system."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke is isolated from the host file system. Real-world applications—whether they are data processors, web servers reading static assets, or simple scripts reading configurations—require file access. Without basic File I/O, Duke cannot run the vast majority of useful, stateful Java programs. Adding support for standard file reading and writing unlocks use cases like log analysis, configuration management, and data transformation, directly increasing the VM's utility for real-world workloads. Furthermore, this feature leverages the recently added `try-with-resources` support, providing a canonical use case for automatic resource management.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully open a local text file, read its contents into a `byte[]` using `FileInputStream`, write modified contents to a new file using `FileOutputStream`, and gracefully close both streams via `try-with-resources` without leaking file descriptors.

## 🔍 Gap Analysis
- **Current State:** Duke can print to standard output (via `System.out`) and execute complex logic, but it has no native bindings for standard file operations.
- **Market/Standard Lib:** Standard Java relies on `java.io.FileInputStream` and `java.io.FileOutputStream` for low-level byte I/O, typically wrapped by `InputStreamReader` or `BufferedReader`.
- **The Gap:** We need native bridge implementations for opening, reading, writing, and closing files, mapping Java's file descriptor concepts to the host operating system's file handles. We also need basic `java.io.File` support for path representation.

## ✅ Acceptance Criteria
- Must support creating a `java.io.File` instance from a string path.
- Must support `File.exists()`, `File.isFile()`, and `File.isDirectory()`.
- Must implement `FileInputStream.read()` (single byte) and `FileInputStream.read(byte[])` (block read).
- Must implement `FileOutputStream.write(int)` (single byte) and `FileOutputStream.write(byte[])` (block write).
- Must implement `close()` for both stream types, integrating with the `AutoCloseable` interface.
- Must correctly raise `java.io.FileNotFoundException` when attempting to read a non-existent file.
- Must correctly raise `java.io.IOException` for invalid read/write operations or closed streams.
- Must automatically release underlying host OS file handles when streams are garbage collected (or explicitly closed).

## 🚫 Out of Scope
- Advanced file attributes (permissions, symlinks, timestamps).
- Non-blocking I/O (`java.nio`).
- `RandomAccessFile`.
- Directory listing and recursive traversal (Phase 2).
