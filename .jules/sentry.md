## 2024-06-23 - Handle invalid JAR analysis cases gracefully
**Learning:** Functions that parse files shouldn't panic using `unwrap_or_else(|| process::exit(1))` as it prevents them from being easily tested with `#[should_panic]`.
**Action:** Replace `unwrap_or_else` process exits with graceful `return` statements and print error messages to standard error. This makes them fully testable.
