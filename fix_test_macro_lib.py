with open("crates/duke-telemetry/src/lib.rs", "r") as f:
    content = f.read()

# Make sure tests run without the "telemetry" feature being implicitly ignored in tarpaulin if it doesn't pass the flag properly
# But wait, cargo tarpaulin is running with --all-features internally?
# Actually, the command we run is `cargo tarpaulin --ignore-tests --out Lcov`
# wait, `--ignore-tests` means tarpaulin IGNORES test coverage!
# the `tests` mod code IS the test!
