# 🔭 Vantage: Spec for java.net.URLConnection

## 👤 User Story
"As a Backend Developer running Spring Boot on Duke, I want to load configuration properties from classpath resources via `java.net.URL.openConnection()`, so that my application can bootstrap its configuration exactly as it does on a standard JVM without hitting `UnsatisfiedLinkError` or 'method not found' runtime errors."

## ❓ The "So What?"
What business problem does this solve?
The ability to read resources and properties from JARs and URLs is foundational to the JVM ecosystem. Frameworks like Spring Boot rely heavily on `java.net.URL.openConnection()` to return a `URLConnection` (and often an `HttpURLConnection`) to resolve and load these configuration resources (e.g., in `PropertiesLoaderUtils::fillProperties` mapping to `org/springframework/core/io/UrlResource::getInputStream`). Currently, Duke's lack of a synthetic `URLConnection` and the `openConnection` implementation blocks the initialization of real-world Spring Boot applications. Unblocking this capability enables Duke to boot industry-standard Java web applications.

## 📈 Metric Definition
Success = The Spring Boot real app fixture (`spring_boot_app_boots_end_to_end`) bypasses the current `java/net/URL.openConnection()Ljava/net/URLConnection;` blocker and progresses to the next missing capability.

## 🔍 Gap Analysis
- **Current State:** Duke has a synthetic `java/net/URL` and `native_url_open_stream`, but `java.net.URL.openConnection()` is missing entirely.
- **Market/Standard Lib:** Standard JVMs return subclasses of `java.net.URLConnection` (such as `HttpURLConnection` or `JarURLConnection`) to abstract reading resources over different protocols. Spring boot often expects `HttpURLConnection` for `instanceof` checks and `disconnect` capability.
- **The Gap:** We need a new synthetic abstract class `java.net.URLConnection`, and the native bridge for `URL.openConnection()` that returns an appropriate connection instance that supports `getInputStream()`, `setUseCaches()`, and potentially the `HttpURLConnection` hierarchy.

## ✅ Acceptance Criteria
- Must introduce a synthetic abstract class `java/net/URLConnection`.
- Must implement the native method `java/net/URL.openConnection()Ljava/net/URLConnection;`.
- The returned `URLConnection` must successfully support `getInputStream()` to read the underlying resource.
- The returned `URLConnection` must support `setUseCaches(boolean)`.
- If Spring's fallback mechanisms probe for `HttpURLConnection` `instanceof` or call `disconnect()`, these should be handled gracefully (even if as no-ops).

## 🚫 Out of Scope
- Full HTTP protocol implementation for external network calls (focus is on local resource/JAR/classpath resolution initially).
- Complex connection pooling or timeouts.
