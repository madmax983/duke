# 🔭 Vantage: Spec for Core Reflection (`java.lang.Class`, `java.lang.reflect.Method`)

## 👤 User Story
"As a Framework Developer running on Duke, I want to use Java Reflection (`java.lang.reflect` and `Class.forName`), so that I can dynamically inspect classes, instantiate objects, and invoke methods at runtime, which is a prerequisite for supporting modern Java frameworks and libraries (like dependency injection or JSON serialization)."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke can only execute code that is statically known at compile time. Modern Java development heavily relies on dynamic behavior—dependency injection (Spring), serialization (Jackson/Gson), and ORMs (Hibernate) all require Reflection. Without Reflection, Duke is limited to simple scripts and algorithms. Adding Reflection enables the ecosystem of standard Java libraries to function on Duke, massively increasing its utility and adoption potential.

## 📈 Metric Definition
Success = A Java program can dynamically load a class by name using `Class.forName()`, inspect its declared methods using `Class.getDeclaredMethods()`, and invoke a specific method dynamically using `Method.invoke()` without VM panics.

## 🔍 Gap Analysis
- **Current State:** Duke has no support for `java.lang.Class` natives like `forName0`, `getDeclaredMethods0`, or `java.lang.reflect.Method.invoke`. Classes are only loaded when explicitly referenced by bytecode instructions.
- **Market/Standard Lib:** The standard library uses native VM calls to traverse internal class metadata, allocate `Class` objects, and dynamically dispatch method calls.
- **The Gap:** We need native bridges to expose Duke's internal `ClassRegistry` and `ClassContext` metadata to Java space, and a mechanism to translate `Method.invoke` arguments into Duke's internal `exec_method` or `invoke_cb` functionality.

## ✅ Acceptance Criteria
- Must implement `Class.forName(String)` to dynamically load and initialize classes.
- Must implement basic class inspection: `Class.getName()`, `Class.getDeclaredMethods()`, `Class.getDeclaredFields()`.
- Must implement `Method.invoke(Object, Object...)` for dynamic method execution.
- Must appropriately handle boxing/unboxing of primitive types during reflective invocation.
- Must raise standard exceptions (`ClassNotFoundException`, `IllegalAccessException`, `InvocationTargetException`) for failure cases.

## 🚫 Out of Scope
- Modifying `final` fields via reflection (deep reflection).
- `java.lang.invoke` (MethodHandles / VarHandles - this is a separate, more complex system).
- Dynamic Proxy classes (`java.lang.reflect.Proxy`).
