1. **Understand the Goal**: The persona "Vantage" is requesting a PM-style specification for the `jar-scan` and `jar-analyze` tools.
2. **Review Existing Implementation**: Both `jar-scan` and `jar-analyze` are currently implemented in `duke/src/scan.rs` and `duke/src/jar_analyze.rs` respectively. They perform static analysis on JAR files to report potential vulnerabilities (`jar-scan`) and code complexity metrics (`jar-analyze`).
3. **Formulate the Spec**: Based on Vantage's persona and boundaries, create a PM specification encompassing the User Story, Acceptance Criteria, Out of Scope, and Gap Analysis for the `JAR Analysis & Security Tools` feature.
4. **Draft the Issue/PR (Documentation Only)**: Create a new markdown file (e.g., `docs/vantage-spec-jar-tools.md`) with the required Vantage spec format.
5. **Verify and Commit**: Make sure pre-commit checks pass and the new documentation file is successfully created and correctly formatted.
