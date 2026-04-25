# 🔭 Vantage: Spec for Reflection Fields (`java.lang.reflect.Field`)

## 👤 User Story
"As a Framework Developer, I want to use `java.lang.reflect.Field`, so that my dependency injection library or serialization tool can inspect and modify object fields at runtime without needing direct getter/setter methods."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks the native bridges for inspecting and modifying fields via reflection. Modern, enterprise-grade Java frameworks like Spring, Hibernate, and Jackson rely heavily on reflection to automate dependency injection, database ORM mapping, and JSON serialization. Without `java.lang.reflect.Field`, Duke cannot run these standard libraries, making it unusable for most enterprise workloads. Supporting field reflection unlocks a massive ecosystem of dynamic libraries.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully execute `Class.getDeclaredFields()` to enumerate fields, and use `Field.get(Object)` and `Field.set(Object, Object)` to read and modify field values on instances without crashing or throwing a `NoSuchMethodError` for native implementations.

## 🔍 Gap Analysis
- **Current State:** Duke has no native method implementations for `Class.getDeclaredFields()`, `Field.get()`, and `Field.set()`.
- **Market/Standard Lib:** The standard JVM provides complete reflection capabilities in `java.lang.reflect`, backed by complex native methods that interface with the internal object representation.
- **The Gap:** We need to implement the native JVM bridges for these methods, translating the Java reflection API into Duke's internal memory model for field access.

## ✅ Acceptance Criteria
- Must implement `Class.getDeclaredFields()` returning an array of `Field` objects.
- Must implement `Field.get(Object)` returning the correct object or boxed primitive value for the field.
- Must implement `Field.set(Object, Object)` correctly updating the target object's field (with appropriate unboxing if necessary).
- Must respect access control boundaries, or implement `setAccessible(true)`.

## 🚫 Out of Scope
- Reflection for Methods and Constructors (`Method.invoke()`, `Constructor.newInstance()`) - to be handled in a separate phase.
- Deep performance optimization of reflective access (e.g. inflation to bytecode accessors).
