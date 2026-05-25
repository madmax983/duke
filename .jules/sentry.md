## 2026-05-25 - Markdown Fallback Formatting Tested
**Learning:** Adding fallback `| (none) | 0 | 0 |` output for markdown reports when the underlying maps in `TelemetryStore` are empty prevents non-descriptive table sections and broken markup formatting. The edge case where `to_markdown_report` acts on an empty `TelemetryStore::default()` initially had zero coverage.
**Action:** When working on reporting, rendering, or printing methods, immediately look for an implicit `is_empty()` state gap that needs default/fallback formatting verification tests.
