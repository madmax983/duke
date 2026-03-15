## 2024-03-15 - [u64 to usize Conversion Panics and Truncation in GC]
**Learning:** `usize::try_from(u64_ref).unwrap()` will panic on 32-bit platforms for large references, and `(u64_ref & !OLD_BIT) as usize` will silently truncate and access the wrong object.
**Action:** Replace direct casting and unwraps with `.try_from().unwrap_or(usize::MAX)` when using `u64` IDs to index into `Vec`s. This ensures out-of-bounds references safely fail the `Vec::get()` call and return a proper `Error` instead of panicking or truncating.
