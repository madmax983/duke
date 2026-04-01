**[execute_class `collect` chain removal]**
**Learning:** Rust's standard library `.collect::<Result<Vec<_>, _>>()` on an iterator forces the lower bound of `size_hint` to `0` to handle early errors safely. This means that pre-allocation fails, causing multiple heap allocations for the returned `Vec`.
**Action:** In hot loops, such as JVM native method dispatch and lambda argument fetching where `arg_count` is known, explicitly use `Vec::with_capacity` and a `for` loop with `.push()` instead of `.map().collect()`.
