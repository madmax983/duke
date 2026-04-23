1. **Smell identified:** Deep nesting and duplicated logic when checking for subroutine branches (`Jsr` and `JsrW`).
2. **Action:**
   - Extract a new helper method `is_subroutine_call()` on the `Instruction` enum in `crates/duke-bytecode/src/instruction.rs`.
   - Implement it using `matches!(self, Self::Jsr(_) | Self::JsrW(_))`.
   - Update `crates/duke-bytecode/src/cfg.rs` to use `is_subroutine_call()` instead of inline `matches!`.
   - Ensure a test for `is_subroutine_call()` is added to `instruction.rs`.
3. **Pre-commit:**
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo fmt --all`.
   - Run `cargo test`.
4. **Submit PR:**
   - Add journal entry to `.jules/forge.md`.
   - Commit with title `⚒️ Forge: Extract subroutine branch matchers`.
