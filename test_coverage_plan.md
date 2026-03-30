1. **Target**: `crates/duke-gc/src/lib.rs` - file handle operations.
   - Added multiple tests to cover file I/O error edge cases in `crates/duke-gc/tests/io_errors.rs` to simulate `read_host_file_byte`, `write_host_file_byte`, `open_host_input_file`, and `open_host_output_file` branch failures.
2. **Refactor**:
   - Refactored tests to properly mock file access failures, maintaining cross-platform compatibility by avoiding `std::os::unix::fs::PermissionsExt`.
3. **Review & Pre-commit Steps**:
   - Run `cargo fmt`, `cargo test`, `cargo clippy`, and `cargo doc` via the pre commit steps.
4. **Submit PR**:
   - Submit a PR with title "🛡️ Sentry: Add coverage for duke-gc host file IO edge cases".
