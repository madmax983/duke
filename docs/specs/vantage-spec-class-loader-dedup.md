# 🔭 Vantage: Spec for Class Loader Deduplication

## 👤 User Story
"As a Java Framework Developer using Duke JVM, I want the class loading system to deduplicate classes loaded by the same logical classloader, so that my applications can perform reflective lookups (like Spring Boot's ApplicationListener) without failing due to ambiguous class name resolution."

## ❓ The "So What?"
What business problem does this solve?
Currently, when running complex applications like Spring Boot, the JVM attempts to load the same class (`ApplicationListener`) under multiple loader identities. This occurs because the system creates distinct, duplicate conceptual entries rather than deduplicating requests for the same class from the same loader. When the framework later uses reflection or instance checks against these classes, it triggers an "ambiguous class name" error or causes class cast exceptions. This fundamentally breaks class identity and application initialization for any large-scale Java application. Fixing this deduplication is required for Duke to run modern application servers.

## 📈 Metric Definition
Success = The Spring Boot fixture application passes the "ambiguous class name" failure point and the related class-identity casting issues during initialization without modifying the application code.

## 🔍 Gap Analysis
- **Current State:** The class management system does not correctly deduplicate classes loaded by the same loader, leading to multiple conceptual entries for the same class.
- **Market/Standard Lib:** Standard JVMs maintain strict class identity uniqueness for a given Fully Qualified Class Name (FQCN) within the namespace of a specific ClassLoader.
- **The Gap:** We need to implement loader-key deduplication in the system to ensure that a single class loaded by a specific classloader always resolves to the exact same conceptual representation.

## ✅ Acceptance Criteria
- Must deduplicate class loading requests so that the same FQCN requested by the same class loader results in a single, unique class identity.
- Must ensure that assignability and other reflection identity checks succeed when comparing a class to itself, validating that the underlying identities match.
- Must resolve the "ambiguous class name: org/springframework/context/ApplicationListener matches [..\0,..\0]" error in the Spring Boot initialization sequence.

## 🚫 Out of Scope
- Full implementation of custom user-defined network class loaders. This spec focuses on fixing the identity deduplication for the existing loader infrastructure.
