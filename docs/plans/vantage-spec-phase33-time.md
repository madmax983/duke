# 🔭 Vantage: Spec for Time and Date (`java.time.Instant`, `System.currentTimeMillis`)

## 👤 User Story
"As a Java Developer running on Duke, I want to access the current system time and perform date calculations, so that my applications can log events, measure execution duration, and schedule tasks accurately."

## ❓ The "So What?"
What business problem does this solve?
Currently, Duke lacks the ability to query the system clock. Time is a fundamental requirement for almost all non-trivial software—whether it's adding timestamps to log files, generating unique IDs based on time, measuring performance, or implementing timeouts. Without access to time, Duke applications cannot interact meaningfully with the real world or enforce temporal constraints. Adding basic time support enables a massive swath of standard library functionality and user code that relies on knowing "when" something happened.

## 📈 Metric Definition
Success = A Java program running on Duke can successfully call `System.currentTimeMillis()` multiple times, observe increasing values that match the host OS clock, and calculate a duration.

## 🔍 Gap Analysis
- **Current State:** Duke has no implementation for `System.currentTimeMillis()` or `System.nanoTime()`.
- **Market/Standard Lib:** Standard Java relies on native methods in `java.lang.System` (and underlying OS-specific bridges) to fetch the high-resolution system clock and the wall-clock time. Modern Java (8+) also relies heavily on these for the `java.time` API.
- **The Gap:** We need native bridge implementations that map Java's time requests to Rust's `std::time::SystemTime` and `std::time::Instant`.

## ✅ Acceptance Criteria
- Must implement `System.currentTimeMillis()` returning the number of milliseconds since the Unix epoch (using `SystemTime`).
- Must implement `System.nanoTime()` returning a high-resolution monotonic timer value suitable for duration measurement (using `Instant`).
- Must correctly handle potential system clock skew or backwards jumps gracefully (e.g., panicking or returning 0 if time went backwards before the epoch, though `SystemTime` in Rust handles this safely).

## 🚫 Out of Scope
- Full implementation of the `java.time` API (this is mostly Java code built on top of the native time primitives, but porting the entire package is out of scope for the native bridge phase).
- Timezone management (`java.util.TimeZone`, tzdata parsing).
- `java.util.Date` and `java.util.Calendar` legacy formatting natives (focus on the primitives first).
