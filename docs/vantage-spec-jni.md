# 🔭 Vantage: Spec for Java Native Interface (JNI)

## 👤 User Story
"As a Systems Engineer integrating legacy C/C++ libraries with my Java application running on Duke, I want the VM to support the Java Native Interface (JNI), so that I can call native system APIs, hardware drivers, or highly optimized native code directly from my Java code."

## ❓ The "So What?"
What business problem does this solve?
While Java provides a vast standard library, enterprise applications frequently require access to platform-specific hardware, proprietary legacy codebases written in C/C++, or specialized performance-critical libraries (e.g., machine learning frameworks, graphics APIs). Without JNI, Duke applications are confined entirely to pure Java bytecode, making it impossible to migrate many real-world enterprise workloads that depend on these external native dependencies. Implementing JNI proves Duke can serve as a drop-in replacement for standard JVMs in complex, hybrid-language environments.

## 📈 Metric Definition
Success = A user can load a standard dynamic library (e.g., `.so`, `.dll`, or `.dylib`) containing a compiled JNI C/C++ function using `System.loadLibrary()`, declare the corresponding `native` method in a Java class, and successfully invoke that native method from Duke, correctly passing primitive arguments and receiving the expected primitive return value without the VM crashing.

## 🔍 Gap Analysis
- **Current State:** Duke has an internal mechanism for binding native methods (used for the standard library), but it does not support dynamically loading external shared libraries or the standardized JNI calling convention required by third-party native code.
- **Market/Standard Lib:** Standard JVMs fully implement the JNI specification, providing the environment pointer, function tables, and type bridging necessary for C/C++ code to interact with the JVM's heap and execution state.
- **The Gap:** We need to implement a dynamic library loader (via `System.loadLibrary`), generate or provide the environment context and function table compatible with the C ABI, and create a bridge that translates Duke's internal memory representations and object references into JNI-compatible types.

## ✅ Acceptance Criteria
- Must support loading external shared libraries via `System.loadLibrary()` and `System.load()`.
- Must implement the basic JNI invocation interface (`JNI_OnLoad`).
- Must provide a C-compatible environment context with at least the core functions for primitive type manipulation and basic object field/method access.
- Must correctly resolve and link Java `native` method declarations to their exported C/C++ counterparts (e.g., `Java_com_example_MyClass_myMethod`).

## 🚫 Out of Scope
- Support for the JVM Tool Interface (JVMTI) (handled in a separate debugging spec).
- Advanced JNI features like Weak Global References or monitor synchronization via JNI in the first iteration.
