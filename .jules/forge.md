**[Option Iteration Simplification]**
**Learning:** `Option` implements `IntoIterator`. When conditionally pushing the value of an `Option` into a collection (e.g. `if let Some(v) = opt { vec.push(v); }`), it can be idiomatic and simpler to use `vec.extend(opt)`.
**Action:** Always prefer `collection.extend(option)` over `if let Some` blocks when appending optional values to reduce nesting and lines of code.
