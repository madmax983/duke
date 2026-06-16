import re

with open('crates/duke-interpreter/src/native.rs', 'r') as f:
    content = f.read()

# For lines like `let s = heap.get(this_ref)?.string_value.clone().unwrap_or_default();`
# We can't just replace `clone()` with `as_deref()` if the variable is used directly as a string or needs to live beyond the heap borrow.
# Wait, if we do:
# let this_obj = heap.get(this_ref)?;
# let s = this_obj.string_value.as_deref().unwrap_or_default();
# If we do `heap.get(this_ref)?.string_value.as_deref().unwrap_or_default()`, it creates a temporary `&HeapObject` from `get`, gets the Option<&String> from as_deref, unwraps to `&str`, then the temporary `&HeapObject` is dropped, meaning the `&str` becomes a dangling pointer!
# Wait, does `unwrap_or_default` on Option<&str> return a `&str`? Yes, default for `&str` is `""`. But the `&str` returned by `as_deref()` borrows from the temporary `&HeapObject`. So it's a temporary value dropped while still in use.
# So we MUST store the `HeapObject` in a variable, or we can just replace `.string_value.clone().unwrap_or_default()` with `.string_value.as_deref().unwrap_or_default().to_string()` which is basically the same as cloning.

print("Testing the temporary borrow issue")
