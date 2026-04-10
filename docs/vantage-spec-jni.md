# 🔭 Vantage: Spec for Java Native Interface (JNI)

## 👤 User Story
"As a Java Developer running on Duke, I want to execute standard Java Native Interface (JNI) libraries, so that I can utilize existing high-performance native code (C/C++/Rust), hardware drivers, or specialized libraries without rewriting them in Java."

## ❓ The "So What?"
What business problem does this solve?
While Duke can execute Java bytecode, the real world is not purely Java. Countless critical enterprise libraries—ranging from machine learning frameworks (TensorFlow/PyTorch Java wrappers), cryptography accelerators, graphics libraries (LWJGL), to database drivers (SQLite)—rely on native shared libraries (`.so`, `.dll`, `.dylib`) via JNI. Without the ability to dynamically load these libraries and bridge calls to them, Duke is locked out of applications that require system-level or high-performance native extensions. Supporting standard JNI proves Duke is a serious, standard-compliant JVM capable of running complex, native-backed workloads.

## 📈 Metric Definition
Success = A user can run a Java application on Duke that calls `System.loadLibrary("mylib")`, and when the application invokes a `native` method, Duke successfully locates the symbol in the shared library, executes the C/C++ function passing primitive arguments via the standard `JNIEnv` interface, and retrieves the correct return value without panicking or corrupting memory.

## 🔍 Gap Analysis
- **Current State:** Duke only supports internal "intrinsic" native methods hardcoded directly into the VM in Rust. It has no capability to dynamically load external shared libraries or conform to the C ABI required by standard JNI libraries.
- **Market/Standard Lib:** The standard JVM provides a fully specified C API (`JNIEnv`). When `System.loadLibrary` is called, it loads the shared object using OS-level dynamic linking (`dlopen`/`LoadLibrary`), resolves the symbol (e.g., `Java_com_example_MyClass_myMethod`), and calls it passing a pointer to the JVM environment.
- **The Gap:** We need to implement dynamic linking (likely using the `libloading` crate), build out the C-ABI compliant `JNIEnv` interface that external libraries expect to interact with the JVM, and marshal data (arguments and return values) between Duke's internal Rust representation and the native C ABI.

## ✅ Acceptance Criteria
- Must implement `System.loadLibrary(String)` and `System.load(String)` to dynamically load shared objects (`.so`, `.dylib`, `.dll`) from the filesystem.
- Must implement dynamic symbol resolution mapping Java `native` method declarations to the expected C function names (e.g., `Java_pkg_Class_method`).
- Must define a basic, C-ABI compliant `JNIEnv` struct to pass as the first argument to native functions.
- Must support passing primitive types (int, float, etc.) across the JNI boundary safely.
- Must properly throw `java.lang.UnsatisfiedLinkError` if a required library or method symbol cannot be found.

## 🚫 Out of Scope
- The JNI Invocation API (embedding the Duke VM inside a C/C++ application).
- Advanced JNI capabilities like direct memory buffers (`GetDirectBufferAddress`) or complex reflection-based object manipulation via `JNIEnv` (Phase 1 should focus on primitives and simple object handles).
- Supporting legacy JNI features like `MonitorEnter`/`MonitorExit` via the native interface.
