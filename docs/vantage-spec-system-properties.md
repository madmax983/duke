# 🔭 Vantage: Spec for System Properties Bootstrap (`System.getProperties()`)

## 👤 User Story
"As a Backend Developer running my code on Duke, I want the VM to provide standard system properties (like file path separators, OS names, and Java version), so that my application and the standard library can correctly initialize path-based File I/O and classloaders without crashing during bootstrap."

## ❓ The "So What?"
What business problem does this solve?
System properties are foundational. The Java standard library relies heavily on them to determine environmental context (like case-sensitivity of the file system, line separators, or classpath). Currently, when running Duke with the real JDK shadow enabled, attempting to open a file via a path (`new FileOutputStream(path)`) or initializing the system classloader crashes because it cannot fetch these properties. Without this, we cannot read configuration files, load classes dynamically from the filesystem, or write application logs to disk. Resolving this cross-lane bootstrap issue unblocks the next major wave of real-world functionality, including File I/O and custom classloaders.

## 📈 Metric Definition
Success = A user can execute `System.getProperties()` or `new java.io.FileOutputStream("test.txt")` under `DUKE_REAL_JDK=1` without the VM crashing at `MethodNotFound: java/lang/System.getProperties()Ljava/util/Properties;`. The returned `Properties` object must be a live, well-formed shadowed `Properties`/`Hashtable` object graph.

## 🔍 Gap Analysis
- **Current State:** The ClassLoader bootstrap and `UnixFileSystem.<init>` (used by path-based `FileOutputStream`) are both blocked by a `MethodNotFound` error for `System.getProperties()`. When running under `--real-jdk` shadow, `java.util.Properties` is shadowed by real `Hashtable` bytecode, but Duke currently fails to materialize this complex object graph during the critical early boot phases.
- **Market/Standard Lib:** Standard JVMs materialize the system properties early in the boot sequence (`System.initPhase1`), creating a populated `java.util.Properties` instance backed by the real `Hashtable` implementation to expose OS and VM environment variables.
- **The Gap:** We need to implement a cross-lane bootstrap mechanism that safely instantiates and populates the real `java.util.Properties` (and its `Hashtable` superclass) object graph within Duke, bypassing or satisfying the cyclic dependencies that occur during early VM initialization.

## ✅ Acceptance Criteria
- Must successfully execute `java.lang.System.getProperties()` returning a `java.util.Properties` instance.
- Must ensure that the returned `Properties` object and its underlying `Hashtable` use the real JDK bytecode layouts when `DUKE_REAL_JDK=1` is enabled (no synthetic native shortcut for shadowed classes).
- Must unblock `UnixFileSystem.<init>` from crashing on `System.getProperties()`.
- Must unblock the ClassLoader bootstrap lane which depends on system properties.

## 🚫 Out of Scope
- Implementing the entirety of `System.initPhase1` or the complete JVM startup sequence (only the subset required to materialize the properties map).
- Populating every single standard Java property (focus on the critical ones needed by `UnixFileSystem` and ClassLoader, like `os.name`, `file.separator`, `path.separator`, `line.separator`).