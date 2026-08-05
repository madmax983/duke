# 🔭 Vantage: Spec for Nest-Based Access Control (JEP 181)

## 👤 User Story
"As a Java Framework Developer running on Duke, I want the VM to support Nest-Based Access Control (JEP 181), so that inner and outer classes can seamlessly access each other's private members via reflection and bytecode without throwing `IllegalAccessException`."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke enforces strict access control rules where cross-class private invocations throw `IllegalAccessException`. The JVM ecosystem uses "nest-mates" to allow classes within the same nest (like an outer class and its inner classes) to access each other's private fields and methods. Modern enterprise frameworks like Spring Boot rely on this behavior during bootstrap (e.g., when reflectively invoking private methods on inner configuration classes). Without nest-mate support, these applications crash. Implementing this feature removes a critical blocker for running modern, reflection-heavy Java frameworks on Duke.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully use reflection from an outer class to call a private method on its inner class without throwing `IllegalAccessException`, and the Spring Boot ladder test proceeds past the `IllegalAccessException` wall.

## 🔍 Gap Analysis
- **Current State:** Duke's access control strictly verifies the exact class match for private access. `NestHost` and `NestMembers` attributes are ignored (not modelled in `ReflectedClassInfo` as noted in findings).
- **Market/Standard Lib:** Standard JVMs parse `NestHost` and `NestMembers` classfile attributes and relax access checks if the caller and callee share the same `NestHost`.
- **The Gap:** We need to parse the `NestHost` and `NestMembers` attributes. Then, we need to surface this data in `ReflectedClassInfo` and update the access control logic to grant access between nest-mates.

## ✅ Acceptance Criteria
- Must parse `NestHost` and `NestMembers` attributes.
- Must expose nest host data in class metadata (e.g., `ReflectedClassInfo`).
- Must update reflective access checks to grant access if the caller and target class share the same nest host.
- Must update normal bytecode access checks to permit nest-mate private access.
- Must not break existing strict access control for unrelated classes.

## 🚫 Out of Scope
- Modifying `duke-gc` memory management.
- Modifying unrelated reflection rules outside of nest-mates.
