1. Use `run_in_bash_session` to create `docs/vantage-spec-collectors-advanced.md` using `cat << 'EOF' > docs/vantage-spec-collectors-advanced.md` with the full Vantage markdown content for advanced Java Collectors and Collections (based on Phase 56, 58, 59 test fixtures).
2. Use `run_in_bash_session` with `cat docs/vantage-spec-collectors-advanced.md` to verify the file was created and formatted correctly.
3. Use `run_in_bash_session` with `cargo test --workspace` to ensure no regressions were introduced.
4. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. Use the `submit` tool to present the roadmap PR titled "🔭 Vantage: Spec for Advanced Java Collectors and Collections Support".
