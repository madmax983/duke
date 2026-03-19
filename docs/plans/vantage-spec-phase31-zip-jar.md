# 🔭 Vantage: Spec for ZIP and JAR File Support (`java.util.zip.ZipFile`, `java.util.jar.JarFile`)

## 👤 User Story
"As a Java Application Maintainer running my code on Duke, I want the VM to natively process and extract files from standard `.zip` and `.jar` archives, so that I can package, deploy, and execute multi-class applications using the standard Java ecosystem format rather than dealing with hundreds of loose `.class` files."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke can only load and execute loose `.class` files scattered across the filesystem. In the real world, Java applications—ranging from simple CLI tools to enterprise Spring Boot monoliths—are overwhelmingly distributed and deployed as `.jar` (Java ARchive) files. A `.jar` is simply a `.zip` file containing compiled classes and metadata (like the `META-INF/MANIFEST.MF`). Without support for reading compressed archives, Duke cannot run standard packaged applications without an external, manual extraction step. Adding native support for `java.util.zip.ZipFile` and `java.util.jar.JarFile` fundamentally bridges the gap between Duke and the massive existing ecosystem of compiled Java artifacts, vastly improving the developer experience and deployment story.

## 📈 Metric Definition
Success = A user can pass a `.jar` file to the Duke executable (e.g., `duke -jar app.jar`), and the VM will successfully locate the `Main-Class` in the `MANIFEST.MF`, dynamically read the required `.class` files directly from the compressed archive into memory, and execute the application without needing to extract the files to disk first.

## 🔍 Gap Analysis
- **Current State:** Duke's classloader (`duke-loader`) only knows how to search for and read uncompressed `.class` files from directories specified in the classpath.
- **Market/Standard Lib:** The standard JVM relies on native methods in `java.util.zip.ZipFile` to map, traverse, and decompress entries within `.jar`/`.zip` files efficiently. It also uses this capability internally to load classes from the `rt.jar` or application jars.
- **The Gap:** We need two things: (1) an enhancement to Duke's internal classloader to support `.jar` files as valid classpath entries using the existing `flate2` crate in the workspace, and (2) native method bridges for the `java.util.zip` package to expose this functionality back to Java space (so Java code itself can read `.zip` files).

## ✅ Acceptance Criteria
- Must extend the internal Duke classloader to accept `.jar` and `.zip` paths in the `CLASSPATH` environment variable or command-line arguments.
- Must implement native bridges for `java.util.zip.ZipFile.open()`, `getEntry()`, and `read()`.
- Must successfully decompress the `META-INF/MANIFEST.MF` to automatically discover the entry point when invoked with a `-jar` flag.
- Must correctly raise `java.util.zip.ZipException` for malformed or corrupted archives.
- Must correctly raise `java.io.FileNotFoundException` if the specified archive does not exist.
- Must lazily decompress class files into memory only when they are explicitly requested by the classloader (not eagerly unzipping the entire file at startup).

## 🚫 Out of Scope
- Writing or creating new `.zip` or `.jar` files (`ZipOutputStream`, `JarOutputStream`).
- Support for JAR signing and cryptographic signature verification.
- Support for advanced compression algorithms other than standard DEFLATE (the default for ZIP).
- Executing multi-release JAR files (Java 9+ MRJARs).