# 🔭 Vantage: Spec for Thread.getContextClassLoader()

## 👤 User Story
"As a Java Framework Developer running my code on Duke, I want the VM to support `Thread.getContextClassLoader()`, so that my application or library can dynamically load user-specific classes and resources using the appropriate classloader for the current execution context."

## ❓ The "So What?"
What business problem does this solve?
Modern Java frameworks (like Spring, SLF4J, and application servers) heavily rely on the Thread Context ClassLoader (TCCL) to break the standard classloading delegation hierarchy. When generic library code (loaded by a parent classloader) needs to load application-specific classes (loaded by a child classloader), it uses the TCCL. Without supporting `Thread.getContextClassLoader()`, Duke cannot run SLF4J or any containerized Java application, because these libraries will fail to locate their bindings or user code. According to our integration targets, `java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;` is a documented blocker for executing real-world JARs like `slf4j-simple`.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully call `Thread.currentThread().getContextClassLoader()`, retrieve the correct classloader instance, and use it to load a class or resource that is not visible to the system classloader.

## 🔍 Gap Analysis
- **Current State:** Duke can load classes statically and has basic threading, but lacks the native bridge to expose the Context ClassLoader for the current thread to Java space. The `slf4j-simple` OSS smoke test is explicitly blocked on this missing capability.
- **Market/Standard Lib:** The standard JVM stores a `ClassLoader` reference on the `java.lang.Thread` object itself, allowing it to be mutated via `setContextClassLoader` and retrieved via `getContextClassLoader`.
- **The Gap:** We need to implement the native bridge `native_thread_get_context_class_loader` (and its setter counterpart) to extract the classloader reference from the current thread's heap allocation and return it to the caller.

## ✅ Acceptance Criteria
- Must implement `Thread.getContextClassLoader()` to return the classloader associated with the current thread.
- Must implement `Thread.setContextClassLoader(ClassLoader)` to allow frameworks to update the context loader.
- Must correctly initialize the TCCL of the main thread to the application classloader during VM startup.
- Child threads must inherit the TCCL from their parent thread when created.

## 🚫 Out of Scope
- Implementing custom classloaders (e.g., `URLClassLoader`). This spec only covers the storage and retrieval of the classloader reference on the `Thread` object.
