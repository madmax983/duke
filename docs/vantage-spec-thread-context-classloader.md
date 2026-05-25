# 🔭 Vantage: Spec for Thread Context ClassLoader

## 👤 User Story
As a Java Application Developer using the Duke JVM, I want the Thread.getContextClassLoader() method to function correctly, so that standard library and third-party frameworks (like slf4j) can dynamically load resources and classes appropriate to the execution context.

## 💡 So What? (Business Problem)
Without the Context ClassLoader (TCCL), enterprise frameworks fail to boot on Duke. Java relies heavily on TCCL to load service providers and resources. The immediate business value is unblocking slf4j-simple smoke tests, allowing Duke to run standard OSS logging out of the box.

## 📏 Success Metrics
- Success = slf4j-simple-2.0.13 OSS JAR smoke test progresses past the Thread.getContextClassLoader() call.
- Overhead < 1us.

## ✅ Acceptance Criteria
- Must support java/lang/Thread.getContextClassLoader()Ljava/lang/ClassLoader;.
- By default, a newly created thread must inherit the Context ClassLoader of its parent thread.
- If called by the primordial/main thread, it should return the System ClassLoader.

## 🚫 Out of Scope
- Complete OSGi or Java 9 Module System classloader isolation layers.
- SecurityManager checks for fetching/setting the TCCL.
