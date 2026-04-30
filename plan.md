1. **Target**: Fuzz tests triggered a panic inside `write_field` in `duke-gc/src/lib.rs` (due to out-of-bounds indexing of `fields[field_idx] = value`).
2. **Additional Issues**:
   - `heap.young.get_mut(usize::try_from(r).unwrap())` can panic in `get_mut` and `get` if `r` does not fit in `usize`.
3. **Fix Strategy**:
   - Add a `VmError` variant with a custom message since `Error::VmError(String)` already exists. Or better, `Error::InvalidRef` already exists!
   - In `duke-runtime/src/error.rs`, add a new error variant `FieldOutOfBounds` that takes `index` and `length` to properly surface field indexing errors instead of panicking.
   - For `write_field`: Replace `obj.fields[field_idx] = value;` with safe indexing and return the new `FieldOutOfBounds` error.
   - Update `get` and `get_mut` to use `.unwrap_or(usize::MAX)` to prevent panics during conversion from `u64` to `usize`, and correctly return `Error::InvalidRef`.
