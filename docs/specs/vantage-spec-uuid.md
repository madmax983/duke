# 🔭 Vantage: Spec for java.util.UUID Support

## 👤 User Story
"As a Backend Developer running my services on Duke, I want the VM to support `java.util.UUID.randomUUID()`, so that my applications can generate universally unique identifiers for database records and distributed tracing without relying on external libraries."

## ❓ The "So What?"
What business problem does this solve?
Unique identifiers are fundamental to modern distributed systems, databases, and message queues. Standard Java applications heavily rely on `java.util.UUID` for generating transaction IDs, entity primary keys, and idempotency tokens. Without native support for `UUID.randomUUID()` (which typically relies on `SecureRandom`), developers cannot run standard backend frameworks on Duke. Supporting this standard library feature unlocks compatibility with data-intensive applications and basic web services.

## 📈 Metric Definition
Success = A Java program can execute `java.util.UUID.randomUUID().toString()` to produce a valid, formatted UUID string (e.g., `123e4567-e89b-12d3-a456-426614174000`) without crashing due to missing native method implementations.

## 🔍 Gap Analysis
- **Current State:** Duke lacks the native bindings required by `java.security.SecureRandom` or the direct entropy sources that `java.util.UUID` uses under the hood to generate random bytes for Type 4 UUIDs.
- **Market/Standard Lib:** The standard JVM provides `SecureRandom` backed by the host OS's cryptographic entropy pool (e.g., `/dev/urandom` or CryptGenRandom).
- **The Gap:** We need to implement a native bridge that provides cryptographically secure random bytes to the Java environment, enabling `UUID.randomUUID()` to function.

## ✅ Acceptance Criteria
- Must successfully generate valid Type 4 (pseudo-random) UUIDs.
- Must implement the underlying native method required for random byte generation (e.g., hooking into Rust's `rand` or `getrandom` crate).
- Must correctly parse existing UUID strings via `UUID.fromString()`.

## 🚫 Out of Scope
- Type 1 (time-based) UUID generation using host MAC addresses.
- Full implementation of the `java.security` cryptographic provider framework (only provide enough for `SecureRandom` to work for UUIDs).