1. **Explore & Identify coverage gap**: Found that `native_server_socket_accept` in `crates/duke-interpreter/src/native/java_net.rs` has no test coverage (`FNDA:0`).
2. **Implement tests**: Add a `#[cfg(test)] mod java_net_tests` to the end of `crates/duke-interpreter/src/native/java_net.rs` to cover both invalid fd case (where `native_server_socket_accept` throws an IOException) and the valid fd case (where it connects to a thread mimicking a client and succeeds).
3. **Pre-commit**: Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
4. **Request code review**: Submit a PR titled "🛡️ Sentry: [test coverage improvement]" detailing Target, Risk, Strategy, and Verification.
