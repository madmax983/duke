**[JImage Index Map Preallocation]
**Learning:** [HashMap::new() for large, known-size collections causes severe reallocation overhead during startup, especially when parsing files with 30,000+ entries like the JDK jimage file.]
**Action:** [Always use `HashMap::with_capacity(capacity)` when the final size of the collection is already known from headers (e.g., `resource_count`).]

**[Repeated string formatting in Hot Paths]
**Learning:** [Using `format!` inside loops or hot paths (e.g. `ZipLoader::find_class`) causes repeated heap allocations which can severely degrade performance.]
**Action:** [Always pre-allocate a single mutable buffer with `String::with_capacity()` and reuse it across iterations using `.clear()` and `.push_str()` instead of repeated `format!` calls.]

**JImageReader String Buffer Reuse**
**Learning:** In hot loops like `JImageReader::find_class` where a dynamic string path is constructed multiple times per lookup attempt (e.g., across `PROBE_MODULES`), returning a new `String` directly causes a huge allocation bottleneck.
**Action:** Extract the `String` allocation to the outer scope using `String::with_capacity` and pass a mutable reference (`&mut String`) to path-building methods. Use `.clear()` in the loop prior to mutating the buffer, eliminating per-loop memory allocations while retaining safety.
