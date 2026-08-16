# 🔭 Vantage: Spec for Classloader Identity Issue

## 👤 User Story
"As an Enterprise Java Developer running modular applications (like Spring Boot) on Duke, I want the JVM to correctly recognize when two components are using the exact same class from a shared classloader, so that my application doesn't crash with spurious `ClassCastException` or ambiguous class errors."

## ❓ The "So What?"
What business problem does this solve?
Modern Java applications, especially application servers and frameworks like Spring Boot, use hierarchies of classloaders to isolate components and load code dynamically from Fat JARs. When an application asks different classloaders for the same framework class, the JVM must recognize that they resolve to the exact same underlying class definition. Currently, Duke treats these requests as distinct, creating duplicates. This breaks core Java guarantees: passing an object between components causes a `ClassCastException` because Duke thinks they are different types, and looking up classes becomes ambiguous. Fixing this is a non-negotiable prerequisite for running any enterprise framework that uses custom classloaders.

## 📈 Metric Definition
Success = An application using multiple delegating classloaders can instantiate a class from a child loader, pass it to a method expecting the parent loader's version of the class, and pass the `instanceof` and type-casting checks with 0 failures.

## 🔍 Gap Analysis
- **Current State:** Duke naively ties class identity to the specific classloader that requested the class (the initiating loader), rather than the one that actually provided it (the defining loader), leading to duplicated, conflicting classes.
- **Market/Standard Lib:** The standard JVM explicitly separates "initiating" classloaders from "defining" classloaders to maintain a consistent runtime type system across a hierarchy of loaders.
- **The Gap:** We need to implement proper classloader delegation semantics, ensuring that class identity is resolved and deduplicated based on the defining classloader.

## ✅ Acceptance Criteria
- Must accurately determine class identity across classloader hierarchies (Defining vs. Initiating loaders).
- Must prevent duplicate registrations of the same class when requested by a child classloader but defined by a parent.
- Must ensure that type checking (`instanceof`, casting, and assignability) succeeds when comparing instances of a class loaded via different initiating classloaders that share the same defining classloader.
- Must preserve class isolation when the same class name is loaded by completely unrelated classloaders.

## 🚫 Out of Scope
- Dynamic unloading of ClassLoaders or classes (Garbage Collection of classes).
- Java 9+ Module System (JPMS) strict encapsulation rules.
