# 🔭 Vantage: Spec for Customizable Security Scan Rules

👤 **User Story:**
As a Security Engineer, I want to define custom rules for the `jar-scan` tool using an external configuration file, so that I can flag organization-specific dangerous APIs without modifying the JVM source code.

**The "So What?" ask:**
Organizations have proprietary APIs or specific banned libraries (e.g., legacy crypto) that aren't globally considered malicious. The current hardcoded `DEFAULT_RULES` array is too rigid. Allowing custom rule definitions transforms `duke jar-scan` from a toy scanner into an enterprise-ready compliance tool, increasing adoption by enterprise security teams.

**Metric Definition:**
- **Success:** Users can supply a custom rule file and the scanner correctly identifies matching APIs with 0% false positives for exact class/method matches.
- **Performance:** Parsing the custom rules file and merging it with (or overriding) the default rules adds < 50ms overhead to the total execution time of `jar-scan`.

**Gap Analysis:**
Existing Java SAST tools (like SpotBugs or Semgrep) allow extensive custom rules, but they are often heavyweight and require complex setup. Duke is fast and hermetic. By adding simple customizable rules (like JSON/YAML), we bridge the gap between "hardcoded basics" and "complex enterprise SAST" while maintaining our performance edge.

✅ **Acceptance Criteria:**
- The CLI must accept a new optional argument (e.g., `--rules <file>`) to specify the custom rules file.
- The configuration file format must support specifying the target class name, method name, severity, and description for each rule.
- If the custom rules file is provided, its rules must be evaluated during `scan` and `jar-scan`.
- If the file is malformed, the tool must fail gracefully with a clear error message.
- The output report must display custom rule matches in the same format as default rule matches.

🚫 **Out of Scope:**
- Regex matching or wildcards for class/method names (Phase 2).
- Dynamic analysis or data flow analysis to trace malicious input (we only flag static API presence).
- Auto-remediation or rewriting of the dangerous bytecode.