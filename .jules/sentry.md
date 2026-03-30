## 2026-03-20 - Cross-platform File IO Testing
**Learning:** Testing file IO errors (like write failures due to read-only permissions) using `std::os::unix::fs::PermissionsExt` breaks cross-platform compatibility on Windows.
**Action:** Always use the standard library's `Permissions::set_readonly(true)` when simulating permission-based IO errors in tests to ensure they compile and run correctly on all targets.
