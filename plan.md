1. **Analyze untested logic**: Based on `telemetry_missing.txt`, `crates/duke-telemetry/src/helpers.rs` and `crates/duke-telemetry/src/lib.rs` are missing coverage for serialization errors and missing fields in telemetry. I have reviewed the files and removed `.unwrap()` calls in `helpers.rs`, replacing them with tests ensuring serialization doesn't fail. I will also check missing lines in `lib.rs` according to `telemetry_missing.txt` (which has over 100 missing lines) by using `cargo tarpaulin` and removing `unwrap()` calls there.
2. **Implement Doc Tests**: Add documentation tests to `crates/duke-telemetry/src/helpers.rs` functions.
3. **Run tests**: Run `cargo test` and `cargo tarpaulin` to confirm coverage has increased.
4. **Complete pre-commit steps**: Complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.
5. **Submit PR**: Submit a PR titled "🛡️ Sentry: [test coverage improvement] added tests for duke-telemetry helpers".
