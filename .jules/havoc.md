## 2025-04-30 - Havoc's Fuzzing Journey
**Learning:** Found several areas to stress-test via proptest and manual OOM tests.
**Action:** When adding chaos tests that deal with temporary files, use unique dynamic file names to prevent flaky race-condition bugs caused by concurrent test execution. When passing huge numbers or strings, track GlobalAlloc limits to ensure the application panics gracefully instead of allowing OS to OOM the process.
