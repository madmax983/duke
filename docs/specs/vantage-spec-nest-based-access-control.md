# 🔭 Vantage: Spec for Nest-Based Access Control (JEP 181)

## 👤 User Story
"As a Java Developer using modern Java features (like lambdas and inner classes), I want the VM to natively support Nest-Based Access Control (JEP 181), so that my nested classes can securely and efficiently access each other's private members without the compiler generating synthetic bridge methods, and reflection APIs behave correctly across nest-mates."

## ❓ The "So What?"
What business problem does this solve?
Since Java 11, the JVM introduced the concept of "nests" to formalize the relationship between outer and inner classes. Prior to JEP 181, inner classes accessing private members of their outer classes (or vice versa) required the compiler to inject synthetic package-private bridge methods (`access$000`). This added overhead, bloated the constant pool, and broke encapsulation (the synthetic methods could be accessed by other classes in the same package). Furthermore, modern reflection APIs and `MethodHandles.Lookup` rely on nest-mate relationships to grant appropriate access. Currently, Duke only supports strict same-class access for private members via reflection. Because modern bytecode (compiled with Java 11+) omits these bridge methods and relies on native JVM nest-mate access checks, running modern JARs on Duke will eventually result in `IllegalAccessError` or `IllegalAccessException` when inner classes try to reflect or directly access private members of their nest-mates. Implementing this bridges a critical compatibility gap for running any Java 11+ codebase.

## 📈 Metric Definition
Success = A user can execute bytecode compiled for Java 11+ where an inner class invokes a private method of its outer class (via standard bytecode instructions like `invokespecial` or `invokevirtual`) and successfully executes without throwing `IllegalAccessError`. Furthermore, reflective access (e.g., `java.lang.reflect.Method.invoke`) across nest-mates must succeed without requiring `setAccessible(true)`.

## 🔍 Gap Analysis
- **Current State:** Duke's class parser (`duke-classfile`) parses classfiles but the runtime/reflection subsystem does not model `NestHost` or `NestMembers` attributes. The reflection access check (`caller_is_same_class` in `native_reflect_method_invoke`) only permits strict same-class access; cross-nest private invoke throws `IllegalAccessException`.
- **Market/Standard Lib:** Standard Java 11+ JVMs correctly parse `NestHost` and `NestMembers` class attributes, build a runtime representation of the nest, and relax access control checks (both for bytecode execution and reflection) to permit private access among nest-mates. They also provide standard library APIs like `Class.getNestHost()` and `Class.getNestMembers()`.
- **The Gap:** We need to parse the `NestHost` and `NestMembers` attributes, model nest membership on `ReflectedClassInfo` or a similar runtime structure, and update the access control logic in both bytecode resolution (e.g., `resolve_method`) and reflection (`native_reflect_method_invoke`) to permit access if two classes share the same nest host. We also need to implement the corresponding `java.lang.Class` natives.

## ✅ Acceptance Criteria
- Must update the bytecode access control logic to permit private member access between any two classes in the same nest.
- Must update reflective access checks (`Method.invoke`, `Field.get/set`, etc.) to permit private access among nest-mates.
- Must implement the native methods for `java.lang.Class.getNestHost()` and `java.lang.Class.getNestMembers()`.

## 🚫 Out of Scope
- Support for dynamic addition of nest-mates (e.g., `MethodHandles.Lookup.defineHiddenClass` with `NESTMATE` option) if dynamic hidden classes are not yet supported.
- Modifying bytecode generation or Java compilers (this is purely a JVM runtime feature).
