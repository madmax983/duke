# 🔭 Vantage: Spec for Array Reflection (`java.lang.reflect.Array`)

## 👤 User Story
"As a Library Developer running my code on Duke, I want to use Java's Array reflection capabilities to dynamically create arrays, so that I can implement generic utility methods without hardcoding specific types."

## ❓ The "So What?"
What business problem does this solve?
Many foundational Java libraries rely on dynamic array creation to provide generic utility functions. Without `java.lang.reflect.Array` support, these ubiquitous library methods fail with `MethodNotFound` errors, blocking the adoption of Duke for a wide swath of enterprise Java software.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully execute array growth functions which rely on `java.lang.reflect.Array.newInstance(java.lang.Class, int)`.

## 🔍 Gap Analysis
- **Current State:** Duke has basic support for arrays, but `java.lang.reflect.Array.newInstance` is not supported.
- **Market/Standard Lib:** Standard JDKs provide native methods to instantiate arrays dynamically using a `Class` object and a length.
- **The Gap:** We lack the execution capability for reflective array instantiation. We need to implement the native method for `java.lang.reflect.Array.newInstance(java.lang.Class, int)`.

## ✅ Acceptance Criteria
- Must support dynamically allocating new arrays via `java.lang.reflect.Array.newInstance(java.lang.Class, int)`.

## 🚫 Out of Scope
- Support for multi-dimensional array creation (unless immediately required by a core library).
- Deep structural validation of the `Class` parameter beyond what is required to allocate the array.
