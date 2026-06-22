1. **Increase search branch coverage (`cp_str` mapping on invalid index)**
   - Add a test in `duke/src/search.rs` to exercise the failure to cast or map strings in `cp_str`, pushing coverage for that function towards 100%. I've written the patch but it failed because `search_class_file` is not public in `lib.rs` scope or it was an issue with how `cf` was constructed.
   - We will write a specific unit test in `duke/src/search.rs` testing `search_class_file` indirectly via `dump_search_no_match` and also we will craft a test testing `cp_str` logic indirectly via testing `search_class_file` and constructing a ClassFile with non-UTF8 constant pool entries or calling `dump_search` with an empty mock file.
2. **Fix type inference errors on `jar_diff.rs` & `jar_search.rs`**
   - The type error when chaining combinators over `Option` will be fixed by properly adding type annotations.
3. **Write `failing_serializer` unit tests in `duke-telemetry`**
   - Implement `test_failing_serializer_methods` covering all dummy failing serialize implementations in `helpers.rs` to reach >80% coverage locally for that file.
4. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
