
**[Avoid `clippy::len_zero` false positives in test cases]**
**Learning:** `clippy::len_zero` complains about `assert!(events.len() >= 1)` and suggests `assert!(!events.is_empty())`. However, in tests checking for an exact count of events (like "this test threw exactly 1 exception" or "this test threw 2+ exceptions"), changing to `is_empty` obscures the semantic meaning of the test.
**Action:** Use `#[allow(clippy::len_zero)]` on the test function rather than changing the test's strictness just to appease clippy, to maintain the correct semantic meaning and strictness.
