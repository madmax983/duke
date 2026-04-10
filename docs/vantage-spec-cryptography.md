# 🔭 Vantage: Spec for Java Cryptography Architecture (JCA) Support

## 👤 User Story
"As a Security-Conscious Developer building modern applications on Duke, I want the VM to provide standard Java Cryptography Architecture (JCA) providers, so that my applications can hash passwords, generate secure tokens, and encrypt sensitive data without relying on insecure workarounds."

## ❓ The "So What?"
What business problem does this solve?
Security is not an optional feature for modern software. Almost every enterprise application needs to generate secure random numbers for session IDs, hash passwords before storing them, or verify digital signatures. Without support for the standard `java.security` and `javax.crypto` packages, Duke applications cannot perform these basic security operations. Providing robust JCA support proves Duke can safely run modern backend services that handle sensitive user data and authentication.

## 📈 Metric Definition
Success = A user can instantiate a `java.security.SecureRandom` to generate cryptographically secure bytes, and compute a SHA-256 hash using `java.security.MessageDigest.getInstance("SHA-256")` without throwing a `NoSuchAlgorithmException` or VM panic.

## 🔍 Gap Analysis
- **Current State:** Duke lacks support for native cryptography operations. Attempting to use standard `java.security` classes fails because the underlying native methods connecting to OS-level secure random generators (like `/dev/urandom` or CryptGenRandom) and optimized hashing algorithms are missing.
- **Market/Standard Lib:** Standard JVMs bundle a default security provider (like `SUN` or `SunJCE`) that bridges Java APIs to native OS cryptography libraries or pure-Java implementations of standard algorithms.
- **The Gap:** Duke needs to implement or delegate the core native methods for `SecureRandom` and provide a basic set of standard algorithms (at least SHA-256, SHA-1, MD5) for `MessageDigest`.

## ✅ Acceptance Criteria
- Must support `java.security.SecureRandom` backed by a native OS CSPRNG.
- Must support `java.security.MessageDigest` for at least SHA-256 and MD5.
- Must handle provider registration and algorithm lookup properly so standard `.getInstance()` calls work.

## 🚫 Out of Scope
- Custom HSM (Hardware Security Module) integration.
- Full suite of obscure/legacy algorithms (focus on the modern standards).
- Proprietary third-party JCE providers.
