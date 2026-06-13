## 2024-05-18 - Avoid over-eager refactoring when iterating collections
**Learning:** Removing a `.collect::<Vec<T>>()` allocation from an iterator pipeline may introduce borrow checker conflicts if the collected list was acting as a lifetime boundary, allowing a mutable borrow later in the loop (e.g. `loader.find_class()`).
**Action:** When removing `.collect()` to improve performance by directly iterating, carefully verify that doing so does not extend an immutable borrow across a scope that requires a mutable borrow on the same underlying structure.

## 2024-05-18 - Replacing format! with String::with_capacity
**Learning:** In hot loops, replacing `format!` with `String::with_capacity()` and a series of `push_str` calls is a safe and effective way to reduce string allocation overhead and macro parsing cost without fighting the borrow checker.
**Action:** Use `String::with_capacity()` when concatenating variables in tight loops.
