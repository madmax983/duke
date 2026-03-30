# 🔭 Vantage: Spec for Spring Boot Fat JAR Support

## 👤 User Story
"As a Backend Developer running my code on Duke, I want the VM to natively execute Spring Boot Fat JARs, so that I can deploy standard enterprise microservices without extracting or repackaging them."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke only understands standard Java JARs with a `Main-Class`. However, the vast majority of modern enterprise Java applications are packaged as Spring Boot "Fat" or "Uber" JARs. These JARs contain nested JARs (`BOOT-INF/lib/`) and use a custom classloader (`org.springframework.boot.loader.launch.JarLauncher`). Without the ability to parse these nested structures and recognize Spring Boot's launch mechanics, Duke is locked out of the massive Spring Boot ecosystem. Supporting Fat JARs proves Duke is viable for modern, real-world enterprise deployments.

## 📈 Metric Definition
Success = A user can pass a Spring Boot Fat JAR to the Duke executable (e.g., `duke -jar spring-boot-app.jar`), and the VM successfully boots the application by delegating to the internal Spring Boot loader, parsing nested JARs, and starting the embedded web server or application context without panicking.

## 🔍 Gap Analysis
- **Current State:** Duke can execute standard JARs if they have a `Main-Class` in the `MANIFEST.MF`. However, attempting to run a Spring Boot loader JAR (like `spring-boot-loader-3.5.12.jar`) fails or requires missing custom classloader capabilities (like `java.net.URLClassLoader` or nested ZIP reading).
- **Market/Standard Lib:** Standard JVMs rely on Java-level classloaders (like Spring's `JarLauncher`) to handle nested JARs. This requires Duke to support the specific native methods and networking/URL APIs that Spring's classloader uses to mount nested `jar:file:` URLs.
- **The Gap:** We need to support nested JAR reading and the prerequisite URL/ClassLoader native bridges required by `org.springframework.boot.loader`.

## ✅ Acceptance Criteria
- Must support reading classes from nested JARs (`BOOT-INF/lib/*.jar` and `BOOT-INF/classes/`).
- Must support the `java.net.URL` and `java.net.URLClassLoader` natively (or sufficiently to allow Spring's loader to function).
- Must successfully invoke `org.springframework.boot.loader.launch.JarLauncher` (or the respective launcher) as the entry point.
- Must not require external extraction of the Fat JAR prior to execution.

## 🚫 Out of Scope
- Writing/creating Spring Boot Fat JARs.
- Supporting older Spring Boot 1.x/2.x loader formats (focus on Spring Boot 3.x+).
