# 🔭 Vantage: Spec for `java.net.URLClassLoader` Implementation

## 👤 User Story
"As a Java Framework Developer running my code on Duke, I want the VM to support `java.net.URLClassLoader`, so that my applications can dynamically load classes from JARs and directories at runtime without them being on the initial classpath."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke's classloader is static, initialized entirely at VM startup from the `CLASSPATH` or `-jar` flag. Modern Java applications, particularly application servers (like Tomcat), plugin architectures, and build tools (like Maven), rely heavily on dynamic class loading. They create custom classloaders, almost always extending or delegating to `java.net.URLClassLoader`, to isolate different applications or fetch code over the network/filesystem dynamically. Without `URLClassLoader`, these dynamic systems cannot run, drastically limiting Duke's use cases. Supporting it is a prerequisite for running complex, modular frameworks like Spring Boot (which uses custom loaders to read Fat JARs).

## 📈 Metric Definition
Success = A user can dynamically instantiate a `java.net.URLClassLoader` in Java code, point it to a `.jar` file not on the original classpath, and successfully invoke `loadClass("com.example.MyClass")` to retrieve and instantiate the class.

## 🔍 Gap Analysis
- **Current State:** Duke has an internal classloader (`duke-loader`) but lacks the Java-space native bridge for `java.net.URLClassLoader` and its underlying dependencies.
- **Market/Standard Lib:** The standard JVM provides `URLClassLoader` which handles fetching classes and resources from `http:`, `file:`, and `jar:` URLs.
- **The Gap:** We need to implement the native bridge logic that allows Java's `URLClassLoader` (or its internal delegate, like `java.net.URLClassLoader$1` or `sun.misc.URLClassPath` depending on JDK version) to communicate with Duke's internal `duke-loader`. We also need basic support for parsing and handling `java.net.URL` objects.

## ✅ Acceptance Criteria
- Must implement the native bridges required by `java.net.URLClassLoader` (and related classes like `sun.misc.URLClassPath` if applicable to the supported JDK version).
- Must support loading classes dynamically from a `file://` URL pointing to an uncompressed directory.
- Must support loading classes dynamically from a `file://` URL pointing to a `.jar` or `.zip` file (delegating to the ZIP support implemented in Phase 31).
- Must properly isolate classes loaded by different `URLClassLoader` instances, preventing them from interfering with each other while still delegating to the parent classloader (usually the AppClassLoader) correctly.
- Must throw `java.lang.ClassNotFoundException` if the class cannot be found in the provided URLs.

## 🚫 Out of Scope
- Fetching classes over the network via HTTP/HTTPS (`http://` URLs). Focus on local files and JARs first.
- Complete implementation of the entire `java.net.URL` specification (focus only on what is strictly necessary for `URLClassLoader` to function with local files).
- Custom URL stream handlers (`URLStreamHandlerFactory`).
