# 🔭 Vantage: Spec for Thread Context ClassLoader

## 👤 User Story
"As an Enterprise Framework Maintainer running my code on Duke, I want my threads to support Thread Context ClassLoaders (TCCL), so that my dynamic framework can load user application classes that are not visible to the system classloader."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks support for `java.lang.Thread.getContextClassLoader()`. Frameworks like SLF4J, Spring, and application servers rely heavily on the TCCL to break out of the standard classloader delegation hierarchy and dynamically load plugins, logging implementations, or application code that is supplied at runtime. Without TCCL support, Duke is blocked from running these common enterprise libraries, as demonstrated by the `slf4j-simple` OSS JAR smoke test failing on this missing capability. Implementing TCCL bridges this gap and moves Duke closer to real-world framework compatibility.

## 📈 Metric Definition
Success = The `slf4j-simple` OSS JAR smoke test progresses past the `java.lang.Thread.getContextClassLoader()` failure, and a user can successfully set and get a context classloader on a thread in Java code, with child threads properly inheriting the classloader from their parent at creation time.

## 🔍 Gap Analysis
- **Current State:** Duke has basic threading support and classloaders, but lacks the specific native methods `getContextClassLoader` and `setContextClassLoader` on `java.lang.Thread`, as well as the internal VM state to track this per-thread.
- **Market/Standard Lib:** The standard JVM provides `getContextClassLoader()` to allow code to find classes on behalf of the thread executing the code.
- **The Gap:** We need to implement the native bridge logic for `java.lang.Thread.getContextClassLoader()` and `setContextClassLoader()`. We also need to add a `contextClassLoader` field to Duke's internal thread representation and ensure it is inherited during thread creation.

## ✅ Acceptance Criteria
- Must implement native `java.lang.Thread.getContextClassLoader()`.
- Must implement native `java.lang.Thread.setContextClassLoader(ClassLoader cl)`.
- Must ensure newly created threads inherit the context classloader from their parent thread.
- Must correctly return the system classloader (or a suitable default) if no TCCL has been explicitly set or inherited.

## 🚫 Out of Scope
- Full SecurityManager checks for `getContextClassLoader` (Duke currently uses a no-security-manager approach).
- Complex custom ClassLoader implementations beyond what is needed to verify TCCL storage and retrieval.
