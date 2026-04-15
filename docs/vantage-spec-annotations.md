# 🔭 Vantage: Spec for Java Annotations

## 👤 User Story
"As a Java Framework Developer running my code on Duke, I want the VM to parse and support Java Annotations (e.g., `@Override`, `@Deprecated`, or custom runtime annotations), so that my frameworks can utilize reflection to discover metadata and wire up application logic (like Dependency Injection)."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke parses the raw JVM `.class` files but ignores annotation attributes (`RuntimeVisibleAnnotations`, `RuntimeInvisibleAnnotations`, etc.). Modern Java enterprise development (especially frameworks like Spring, Hibernate, and JUnit) relies almost entirely on annotations for configuration, routing, and lifecycle management rather than XML files. Without the ability to read and reflect upon annotations at runtime, Duke cannot support any modern dependency injection framework or web routing system, severely limiting its viability for hosting standard enterprise microservices.

## 📈 Metric Definition
Success = A user can define a custom runtime annotation (`@Retention(RetentionPolicy.RUNTIME)`), apply it to a class or method, load that class in Duke, and successfully retrieve the annotation instance using standard Reflection APIs (`Class.getAnnotation(MyAnnotation.class)`), reading its default and configured values without the VM crashing or silently dropping the metadata.

## 🔍 Gap Analysis
- **Current State:** The `duke-classfile` crate does not fully parse the `RuntimeVisibleAnnotations` and related attributes. The VM does not expose these annotations to the Java space via the `java.lang.reflect` API.
- **Market/Standard Lib:** Standard JVMs parse annotation attributes during class loading and lazily construct dynamic proxy instances (or internal representations) implementing the annotation interface when queried via Reflection.
- **The Gap:** We need to update `duke-classfile` to parse the complex annotation structures (including element-value pairs). We then need to bridge this parsed data in `duke-interpreter` to expose it via the native `getAnnotation` and `getAnnotations` methods in `java.lang.Class`, `java.lang.reflect.Method`, and `java.lang.reflect.Field`.

## ✅ Acceptance Criteria
- `duke-classfile` must parse `RuntimeVisibleAnnotations` for Classes, Methods, and Fields.
- Must implement native bridges for `Class.getAnnotations()`, `Method.getAnnotations()`, and `Field.getAnnotations()`.
- Must allow retrieving specific annotations by type (e.g., `Class.getAnnotation(Class)`).
- Must synthesize a Java-space proxy or instance object that implements the requested Annotation interface and correctly returns the parsed element values when its methods are invoked.
- Must support all standard annotation element types (primitives, Strings, Enums, Class references, Arrays, and nested Annotations).

## 🚫 Out of Scope
- `RuntimeInvisibleAnnotations` (compile-time only annotations).
- Type Annotations (`RuntimeVisibleTypeAnnotations`, Java 8+).
- Annotation Processing (APT) during compilation (Duke is a runtime, not `javac`).
