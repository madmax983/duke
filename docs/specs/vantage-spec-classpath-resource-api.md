# 🔭 Vantage: Spec for Classpath Resource API

## 👤 User Story
"As a Java Framework Developer building applications on Duke, I want the VM to provide a standard Classpath Resource Enumeration API, so that my applications can dynamically discover and load configuration files, service providers, and nested assets packaged within JARs or directory classpaths."

## ❓ The "So What?"
What business problem does this solve?
Currently, applications on Duke lack the ability to look up resources embedded inside the application's classpath programmatically. Modern enterprise Java architectures rely extensively on mechanisms like `ServiceLoader`, `spring.factories`, and classpath scanning to auto-configure themselves by reading embedded properties and descriptors. Without a standardized resource API (`getResource`, `getResources`, `getResourceAsStream`), complex frameworks like Spring Boot cannot initialize, reducing Duke's viability as a general-purpose runtime. Supporting this API transforms Duke from a simple bytecode executor into a platform capable of running full-scale, modular applications.

## 📈 Metric Definition
Success = A user application running on Duke can successfully invoke `ClassLoader.getResources("META-INF/services/...")` or `Class.getResourceAsStream("/application.properties")` across both uncompressed directory structures and nested JAR files, returning accurate, read-ready data streams and stable URL identifiers.

## 🔍 Gap Analysis
- **Current State:** Duke can execute bytecode but has no unified mechanism to expose arbitrary, non-class files from the classpath back to Java space.
- **Market/Standard Lib:** The standard JVM provides robust resource loading methods via `ClassLoader` and `Class`, generating standard `file:` or `jar:` URLs and providing input streams for reading contents.
- **The Gap:** We need a unified resource resolution substrate that bridges Duke's internal file and ZIP loaders to the Java-facing `URL` and `InputStream` standard libraries.

## ✅ Acceptance Criteria
- Must implement the native bridges required by `Class.getResource`, `Class.getResourceAsStream`, `ClassLoader.getResource`, and `ClassLoader.getResources`.
- Must support resolving resources from directory-based classpaths, exposing a stable `file:` URL.
- Must support resolving resources from JAR-based classpaths (including nested JARs), exposing a stable `jar:file:` URL.
- Must ensure that resource enumeration preserves the original order defined by the application classpath.
- Must provide functional input streams (`URL.openStream()`) allowing applications to read the resource content bytes.

## 🚫 Out of Scope
- Support for remote or custom URL protocol handlers (`http:`, `ftp:`). Focus solely on local directory and JAR resources.
- Advanced memory-mapped file resource loading optimizations; standard buffered stream reading is sufficient.
- Complete implementation of the entire Java networking stack; implement only what is strictly necessary for `URL.openStream()` to function for classpath resources.
