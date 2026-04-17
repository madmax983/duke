1. **Define Spec**: Use `run_in_bash_session` with a heredoc (`cat << 'EOF' > docs/vantage-spec-phase79-immutable-collections.md`) to write the exact Markdown string for the spec. The spec will be titled "🔭 Vantage: Spec for Phase 79 (Immutable Collections & Objects)" and will include User Story, "So What?", Metric Definition, Gap Analysis, Acceptance Criteria, and Out of Scope.
2. **Verify Spec**: Use `run_in_bash_session` with `cat docs/vantage-spec-phase79-immutable-collections.md` to confirm the specification was written correctly.
3. **Run Checks**: Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all` to ensure no regressions were introduced.
4. **Pre-commit**: Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Submit**: Create a commit with the title "🔭 Vantage: Spec for Phase 79 (Immutable Collections & Objects)".
